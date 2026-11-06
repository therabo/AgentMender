//! Fail-closed Chromium local-storage LevelDB observation, entirely read-only.
//! Format references: google/leveldb doc/{log,table}_format.md and Chromium
//! components/services/storage/dom_storage/leveldb/local_storage_leveldb.h.
use agentmender_domain::{CleanupFileIdentity, DesktopLegacyCommitment, DesktopLegacyFile};
use agentmender_persistence::EffectError;
use rustix::fs::{self, Dir, Mode, OFlags};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs::File,
    io::Read,
    os::{fd::OwnedFd, unix::fs::MetadataExt},
    path::{Component, Path},
};

type LegacyInventory = (Vec<DesktopLegacyFile>, BTreeMap<String, Vec<u8>>);
type BlockEntry = (Vec<u8>, Vec<u8>);

const MAX_BYTES: usize = 128 * 1024 * 1024;
const MAX_FILES: usize = 4096;
const MAX_RECORDS: usize = 1_000_000;
fn bad<T>() -> Result<T, EffectError> {
    Err(EffectError::InvalidEvidence)
}
fn hash(b: &[u8]) -> String {
    Sha256::digest(b)
        .iter()
        .map(|x| format!("{x:02x}"))
        .collect()
}
fn dir(parent: &impl std::os::fd::AsFd, name: &std::ffi::OsStr) -> Result<OwnedFd, EffectError> {
    fs::openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| EffectError::InvalidEvidence)
}
fn root(path: &Path) -> Result<OwnedFd, EffectError> {
    if !path.is_absolute() {
        return bad();
    }
    let mut d = fs::open(
        "/",
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|_| EffectError::InvalidEvidence)?;
    for c in path.components() {
        match c {
            Component::RootDir => (),
            Component::Normal(n) => d = dir(&d, n)?,
            _ => return bad(),
        }
    }
    Ok(d)
}
fn list(d: &OwnedFd) -> Result<Vec<String>, EffectError> {
    let mut names = Vec::new();
    let fresh = dir(d, ".".as_ref())?;
    for e in Dir::read_from(&fresh).map_err(|_| EffectError::InvalidEvidence)? {
        let e = e.map_err(|_| EffectError::InvalidEvidence)?;
        let b = e.file_name().to_bytes();
        if b == b"." || b == b".." {
            continue;
        }
        if names.len() >= MAX_FILES {
            return bad();
        }
        names.push(
            std::str::from_utf8(b)
                .map_err(|_| EffectError::InvalidEvidence)?
                .to_owned(),
        );
    }
    names.sort();
    Ok(names)
}
fn same(m: &std::fs::Metadata, i: &CleanupFileIdentity) -> bool {
    m.dev() == i.device
        && m.ino() == i.inode
        && m.nlink() == i.links
        && m.len() == i.size_bytes
        && m.mtime() == i.mtime_seconds
        && m.mtime_nsec() == i.mtime_nanoseconds as i64
}
fn read_files(d: &OwnedFd, path: &Path, names: &[String]) -> Result<LegacyInventory, EffectError> {
    let mut inventory = Vec::new();
    let mut all = BTreeMap::new();
    let mut total = 0usize;
    for n in names {
        let fd = fs::openat(
            d,
            n.as_str(),
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC | OFlags::NONBLOCK,
            Mode::empty(),
        )
        .map_err(|_| EffectError::InvalidEvidence)?;
        let mut f = File::from(fd);
        let m = f.metadata().map_err(|_| EffectError::InvalidEvidence)?;
        if !m.is_file() || m.nlink() != 1 {
            return bad();
        }
        let size = usize::try_from(m.len()).map_err(|_| EffectError::InvalidEvidence)?;
        total = total
            .checked_add(size)
            .ok_or(EffectError::InvalidEvidence)?;
        if total > MAX_BYTES {
            return bad();
        }
        let identity = CleanupFileIdentity {
            path: path.join(n),
            device: m.dev(),
            inode: m.ino(),
            links: m.nlink(),
            size_bytes: m.len(),
            mtime_seconds: m.mtime(),
            mtime_nanoseconds: m.mtime_nsec() as u32,
        };
        let mut bytes = Vec::with_capacity(size);
        (&mut f)
            .take((size as u64) + 1)
            .read_to_end(&mut bytes)
            .map_err(|_| EffectError::InvalidEvidence)?;
        if bytes.len() != size
            || !same(
                &f.metadata().map_err(|_| EffectError::InvalidEvidence)?,
                &identity,
            )
        {
            return bad();
        }
        inventory.push(DesktopLegacyFile {
            identity,
            sha256: hash(&bytes),
        });
        all.insert(n.clone(), bytes);
    }
    Ok((inventory, all))
}
fn crc(data: &[u8]) -> u32 {
    let mut c = !0u32;
    for &v in data {
        c ^= v as u32;
        for _ in 0..8 {
            c = (c >> 1) ^ (0x82f63b78 & (0u32.wrapping_sub(c & 1)));
        }
    }
    !c
}
fn masked(data: &[u8]) -> u32 {
    crc(data).rotate_right(15).wrapping_add(0xa282ead8)
}
fn u32le(b: &[u8]) -> Result<u32, EffectError> {
    Ok(u32::from_le_bytes(
        b.try_into().map_err(|_| EffectError::InvalidEvidence)?,
    ))
}
fn u64le(b: &[u8]) -> Result<u64, EffectError> {
    Ok(u64::from_le_bytes(
        b.try_into().map_err(|_| EffectError::InvalidEvidence)?,
    ))
}
fn var(data: &[u8], p: &mut usize) -> Result<u64, EffectError> {
    let mut n = 0u64;
    for i in 0..10 {
        let x = *data.get(*p).ok_or(EffectError::InvalidEvidence)?;
        *p += 1;
        if i == 9 && x > 1 {
            return bad();
        }
        n |= ((x & 127) as u64) << (7 * i);
        if x & 128 == 0 {
            return Ok(n);
        }
    }
    bad()
}
fn take<'a>(data: &'a [u8], p: &mut usize, len: usize) -> Result<&'a [u8], EffectError> {
    let e = p.checked_add(len).ok_or(EffectError::InvalidEvidence)?;
    let b = data.get(*p..e).ok_or(EffectError::InvalidEvidence)?;
    *p = e;
    Ok(b)
}
fn slice<'a>(data: &'a [u8], p: &mut usize) -> Result<&'a [u8], EffectError> {
    let n = usize::try_from(var(data, p)?).map_err(|_| EffectError::InvalidEvidence)?;
    take(data, p, n)
}
fn records(data: &[u8]) -> Result<Vec<Vec<u8>>, EffectError> {
    let mut out = Vec::new();
    let mut p = 0;
    let mut fragment: Option<Vec<u8>> = None;
    while p < data.len() {
        let remain = 32768 - (p % 32768);
        if remain < 7 {
            let n = remain.min(data.len() - p);
            if data[p..p + n].iter().any(|&x| x != 0) {
                return bad();
            }
            p += n;
            continue;
        }
        if data.len() - p < 7 {
            return bad();
        }
        let sum = u32le(&data[p..p + 4])?;
        let len = u16::from_le_bytes([data[p + 4], data[p + 5]]) as usize;
        let kind = data[p + 6];
        if len > remain - 7 || p + 7 + len > data.len() {
            return bad();
        }
        if kind == 0 && sum == 0 && len == 0 {
            if data[p..p + remain.min(data.len() - p)]
                .iter()
                .any(|&x| x != 0)
            {
                return bad();
            }
            p += remain.min(data.len() - p);
            continue;
        }
        if masked(&data[p + 6..p + 7 + len]) != sum {
            return bad();
        }
        let body = &data[p + 7..p + 7 + len];
        p += 7 + len;
        match kind {
            1 if fragment.is_none() => out.push(body.to_vec()),
            2 if fragment.is_none() => fragment = Some(body.to_vec()),
            3 if fragment.is_some() => fragment.as_mut().unwrap().extend_from_slice(body),
            4 if fragment.is_some() => {
                let mut x = fragment.take().unwrap();
                x.extend_from_slice(body);
                out.push(x)
            }
            _ => return bad(),
        }
        if out.len() > MAX_RECORDS {
            return bad();
        }
    }
    if fragment.is_some() {
        return bad();
    }
    Ok(out)
}
#[derive(Default)]
struct Manifest {
    tables: BTreeMap<u64, (u64, u64)>,
    log: Option<u64>,
    prev: u64,
    next: Option<u64>,
    last: Option<u64>,
    comparator: bool,
}
fn manifest(data: &[u8]) -> Result<Manifest, EffectError> {
    let mut m = Manifest::default();
    let rs = records(data)?;
    if rs.is_empty() {
        return bad();
    }
    for r in rs {
        let mut p = 0;
        while p < r.len() {
            match var(&r, &mut p)? {
                1 => {
                    if slice(&r, &mut p)? != b"leveldb.BytewiseComparator" {
                        return bad();
                    }
                    m.comparator = true
                }
                2 => m.log = Some(var(&r, &mut p)?),
                3 => m.next = Some(var(&r, &mut p)?),
                4 => m.last = Some(var(&r, &mut p)?),
                5 => {
                    if var(&r, &mut p)? > 6 {
                        return bad();
                    }
                    let _ = slice(&r, &mut p)?;
                }
                6 => {
                    let level = var(&r, &mut p)?;
                    if level > 6 {
                        return bad();
                    }
                    let n = var(&r, &mut p)?;
                    if m.tables
                        .remove(&n)
                        .is_none_or(|(old_level, _)| old_level != level)
                    {
                        return bad();
                    }
                }
                7 => {
                    let level = var(&r, &mut p)?;
                    if level > 6 {
                        return bad();
                    }
                    let n = var(&r, &mut p)?;
                    let size = var(&r, &mut p)?;
                    let a = slice(&r, &mut p)?;
                    let b = slice(&r, &mut p)?;
                    if a.len() < 8 || b.len() < 8 || m.tables.insert(n, (level, size)).is_some() {
                        return bad();
                    }
                }
                9 => m.prev = var(&r, &mut p)?,
                _ => return bad(),
            }
        }
    }
    if !m.comparator || m.log.is_none() || m.next.is_none() || m.last.is_none() {
        return bad();
    }
    Ok(m)
}
#[derive(Default)]
struct Rows {
    data: BTreeMap<Vec<u8>, (u64, u8, Vec<u8>)>,
    bytes: usize,
}
fn merge(rows: &mut Rows, key: &[u8], seq: u64, kind: u8, value: &[u8]) -> Result<(), EffectError> {
    if seq > 0x00ff_ffff_ffff_ffff || kind > 1 || (kind == 0 && !value.is_empty()) {
        return bad();
    }
    if let Some(old) = rows.data.get(key) {
        if old.0 == seq && (old.1 != kind || old.2 != value) {
            return bad();
        }
        if old.0 >= seq {
            return Ok(());
        }
    }
    if rows.data.len() >= MAX_RECORDS && !rows.data.contains_key(key) {
        return bad();
    }
    let old_size = rows.data.get(key).map_or(0, |old| key.len() + old.2.len());
    rows.bytes = rows
        .bytes
        .checked_sub(old_size)
        .and_then(|n| n.checked_add(key.len()))
        .and_then(|n| n.checked_add(value.len()))
        .ok_or(EffectError::InvalidEvidence)?;
    if rows.bytes > MAX_BYTES {
        return bad();
    }
    rows.data.insert(key.to_vec(), (seq, kind, value.to_vec()));
    Ok(())
}
fn wal(data: &[u8], rows: &mut Rows) -> Result<(), EffectError> {
    for r in records(data)? {
        if r.len() < 12 {
            return bad();
        }
        let seq = u64le(&r[..8])?;
        let count = u32le(&r[8..12])? as usize;
        let mut p = 12;
        for i in 0..count {
            let kind = *take(&r, &mut p, 1)?.first().unwrap();
            let key = slice(&r, &mut p)?;
            let val = match kind {
                0 => &[][..],
                1 => slice(&r, &mut p)?,
                _ => return bad(),
            };
            merge(
                rows,
                key,
                seq.checked_add(i as u64)
                    .ok_or(EffectError::InvalidEvidence)?,
                kind,
                val,
            )?;
        }
        if p != r.len() {
            return bad();
        }
    }
    Ok(())
}
fn handle(b: &[u8], p: &mut usize) -> Result<(usize, usize), EffectError> {
    let off = usize::try_from(var(b, p)?).map_err(|_| EffectError::InvalidEvidence)?;
    let len = usize::try_from(var(b, p)?).map_err(|_| EffectError::InvalidEvidence)?;
    Ok((off, len))
}
fn block(file: &[u8], h: (usize, usize), limit: usize) -> Result<Vec<u8>, EffectError> {
    let (o, n) = h;
    let end = o.checked_add(n).ok_or(EffectError::InvalidEvidence)?;
    if end.checked_add(5).is_none_or(|e| e > limit) {
        return bad();
    }
    let payload = &file[o..end];
    let ty = file[end];
    let stored = u32le(&file[end + 1..end + 5])?;
    if masked(&file[o..end + 1]) != stored {
        return bad();
    }
    match ty {
        0 => Ok(payload.to_vec()),
        1 => {
            let size =
                snap::raw::decompress_len(payload).map_err(|_| EffectError::InvalidEvidence)?;
            if size > MAX_BYTES {
                return bad();
            }
            snap::raw::Decoder::new()
                .decompress_vec(payload)
                .map_err(|_| EffectError::InvalidEvidence)
        }
        _ => bad(),
    }
}
fn entries(b: &[u8]) -> Result<Vec<BlockEntry>, EffectError> {
    if b.len() < 4 {
        return bad();
    }
    let count = u32le(&b[b.len() - 4..])? as usize;
    if count == 0 || count > (b.len() - 4) / 4 {
        return bad();
    }
    let start = b.len() - 4 - count * 4;
    let mut restarts = Vec::new();
    for i in 0..count {
        let x = u32le(&b[start + i * 4..start + i * 4 + 4])? as usize;
        if (i == 0 && x != 0) || (i > 0 && x <= restarts[i - 1]) || x > start {
            return bad();
        }
        restarts.push(x);
    }
    let mut result = Vec::new();
    let mut p = 0;
    let mut prior = Vec::new();
    let mut seen = BTreeSet::new();
    while p < start {
        let entry_start = p;
        let shared = usize::try_from(var(b, &mut p)?).map_err(|_| EffectError::InvalidEvidence)?;
        let unshared =
            usize::try_from(var(b, &mut p)?).map_err(|_| EffectError::InvalidEvidence)?;
        let value = usize::try_from(var(b, &mut p)?).map_err(|_| EffectError::InvalidEvidence)?;
        if p > start || shared > prior.len() {
            return bad();
        }
        if restarts.binary_search(&entry_start).is_ok() {
            if shared != 0 {
                return bad();
            }
            seen.insert(entry_start);
        }
        let suffix = take(&b[..start], &mut p, unshared)?;
        let val = take(&b[..start], &mut p, value)?;
        let mut key = prior[..shared].to_vec();
        key.extend_from_slice(suffix);
        prior = key.clone();
        result.push((key, val.to_vec()));
        if result.len() > MAX_RECORDS {
            return bad();
        }
    }
    if (p != start || seen.len() != restarts.len()) && !(start == 0 && restarts == [0]) {
        return bad();
    }
    Ok(result)
}
fn internal_cmp(a: &[u8], b: &[u8]) -> Result<std::cmp::Ordering, EffectError> {
    if a.len() < 8 || b.len() < 8 {
        return bad();
    }
    let (ak, av) = a.split_at(a.len() - 8);
    let (bk, bv) = b.split_at(b.len() - 8);
    Ok(ak
        .cmp(bk)
        .then_with(|| u64le(bv).unwrap_or(0).cmp(&u64le(av).unwrap_or(0))))
}
fn table(file: &[u8], rows: &mut Rows) -> Result<(), EffectError> {
    if file.len() < 48 || file[file.len() - 8..] != [0x57, 0xfb, 0x80, 0x8b, 0x24, 0x75, 0x47, 0xdb]
    {
        return bad();
    }
    let limit = file.len() - 48;
    let footer = &file[limit..limit + 40];
    let mut p = 0;
    let meta = handle(footer, &mut p)?;
    let index = handle(footer, &mut p)?;
    if footer[p..].iter().any(|&x| x != 0) {
        return bad();
    }
    let mut ranges = Vec::new();
    let mut include = |h: (usize, usize)| -> Result<(), EffectError> {
        let end =
            h.0.checked_add(h.1)
                .and_then(|x| x.checked_add(5))
                .ok_or(EffectError::InvalidEvidence)?;
        if end > limit || ranges.iter().any(|&(a, b)| h.0 < b && a < end) {
            return bad();
        }
        ranges.push((h.0, end));
        Ok(())
    };
    include(meta)?;
    include(index)?;
    let meta_entries = entries(&block(file, meta, limit)?)?;
    for (_, v) in meta_entries {
        let mut p = 0;
        let h = handle(&v, &mut p)?;
        if p != v.len() {
            return bad();
        }
        include(h)?;
        let _ = block(file, h, limit)?;
    }
    let index_entries = entries(&block(file, index, limit)?)?;
    let mut prev_key: Option<Vec<u8>> = None;
    let mut previous_index: Option<Vec<u8>> = None;
    for (idx, v) in index_entries {
        if idx.len() < 8 {
            return bad();
        }
        if let Some(x) = &previous_index
            && internal_cmp(x, &idx)? != std::cmp::Ordering::Less
        {
            return bad();
        }
        previous_index = Some(idx.clone());
        let mut p = 0;
        let h = handle(&v, &mut p)?;
        if p != v.len() {
            return bad();
        }
        include(h)?;
        let data = entries(&block(file, h, limit)?)?;
        if data.is_empty() {
            return bad();
        }
        for (k, v) in data {
            if k.len() < 8 {
                return bad();
            }
            if let Some(prev) = &prev_key
                && internal_cmp(prev, &k)? != std::cmp::Ordering::Less
            {
                return bad();
            }
            if internal_cmp(&k, &idx)? == std::cmp::Ordering::Greater {
                return bad();
            }
            let split = k.len() - 8;
            let tag = u64le(&k[split..])?;
            merge(rows, &k[..split], tag >> 8, (tag & 255) as u8, &v)?;
            prev_key = Some(k);
        }
    }
    ranges.sort();
    let mut end = 0;
    for (start, next) in ranges {
        if start != end {
            return bad();
        }
        end = next;
    }
    if end != limit {
        return bad();
    }
    Ok(())
}
fn atom(key: &[u8]) -> Result<bool, EffectError> {
    let Some(k) = key.strip_prefix(b"_app://-\0") else {
        return Ok(false);
    };
    let Some((&encoding, body)) = k.split_first() else {
        return bad();
    };
    let prefix = b"codex:persisted-atom:";
    match encoding {
        1 => Ok(body.starts_with(prefix)),
        0 => {
            if body.len() % 2 != 0 {
                return bad();
            }
            let utf: Vec<u16> = body
                .chunks_exact(2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
                .collect();
            Ok(utf.starts_with(&prefix.iter().map(|&b| b as u16).collect::<Vec<_>>()))
        }
        _ => bad(),
    }
}
fn number(name: &str, suffix: &str) -> Option<u64> {
    let s = name.strip_suffix(suffix)?;
    if s.len() < 6 || !s.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    s.parse().ok()
}
fn classify(files: &BTreeMap<String, Vec<u8>>) -> Result<(), EffectError> {
    let current = files.get("CURRENT").ok_or(EffectError::InvalidEvidence)?;
    let name = std::str::from_utf8(current)
        .map_err(|_| EffectError::InvalidEvidence)?
        .strip_suffix('\n')
        .ok_or(EffectError::InvalidEvidence)?;
    let num = name
        .strip_prefix("MANIFEST-")
        .and_then(|n| n.parse::<u64>().ok())
        .ok_or(EffectError::InvalidEvidence)?;
    if format!("MANIFEST-{num:06}") != name {
        return bad();
    }
    let m = manifest(files.get(name).ok_or(EffectError::InvalidEvidence)?)?;
    let mut rows = Rows::default();
    for (&n, &(_, size)) in &m.tables {
        let a = format!("{n:06}.ldb");
        let b = format!("{n:06}.sst");
        let data = match (files.get(&a), files.get(&b)) {
            (Some(x), None) | (None, Some(x)) => x,
            _ => return bad(),
        };
        if data.len() as u64 != size {
            return bad();
        }
        table(data, &mut rows)?;
    }
    let log = m.log.ok_or(EffectError::InvalidEvidence)?;
    let mut found = false;
    let mut prev_found = m.prev == 0;
    for (n, b) in files {
        if let Some(no) = number(n, ".log") {
            if no >= log || no == m.prev {
                if no == log {
                    found = true;
                }
                if no == m.prev {
                    prev_found = true;
                }
                wal(b, &mut rows)?;
            }
        } else if n == "CURRENT"
            || n == "LOCK"
            || n == "LOG"
            || n == "LOG.old"
            || n.strip_prefix("MANIFEST-")
                .is_some_and(|s| s.len() >= 6 && s.bytes().all(|b| b.is_ascii_digit()))
            || number(n, ".ldb").is_some()
            || number(n, ".sst").is_some()
            || number(n, ".dbtmp").is_some()
        {
        } else {
            return bad();
        }
    }
    if (log != 0 && !found) || !prev_found {
        return bad();
    }
    match rows.data.get(b"VERSION".as_slice()) {
        Some((_, 1, v)) if v == b"1" => (),
        _ => return bad(),
    };
    for (k, (_, kind, _)) in rows.data {
        if kind == 1 && atom(&k)? {
            return bad();
        }
    }
    Ok(())
}
fn verify_directories(dirs: &[(std::path::PathBuf, std::fs::Metadata)]) -> Result<(), EffectError> {
    for (path, before) in dirs {
        let fd = root(path)?;
        let after = File::from(fd)
            .metadata()
            .map_err(|_| EffectError::InvalidEvidence)?;
        if !after.is_dir()
            || before.dev() != after.dev()
            || before.ino() != after.ino()
            || before.mtime() != after.mtime()
            || before.mtime_nsec() != after.mtime_nsec()
        {
            return bad();
        }
    }
    Ok(())
}
/// Inspect only the selected profile's fixed Local Storage directory. The caller
/// holds closed-writer authority; files are opened without following symlinks.
pub(super) fn observe(
    user_data: &CleanupFileIdentity,
) -> Result<DesktopLegacyCommitment, EffectError> {
    let base = root(&user_data.path)?;
    let md = File::from(dir(&base, ".".as_ref())?)
        .metadata()
        .map_err(|_| EffectError::InvalidEvidence)?;
    if !same(&md, user_data) {
        return bad();
    }
    let mut d = base;
    let mut path = user_data.path.clone();
    let mut directories = vec![(path.clone(), md)];
    for n in ["Local Storage", "leveldb"] {
        match fs::openat(
            &d,
            n,
            OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(next) => {
                d = next;
                path.push(n);
                let md = File::from(dir(&d, ".".as_ref())?)
                    .metadata()
                    .map_err(|_| EffectError::InvalidEvidence)?;
                directories.push((path.clone(), md));
            }
            Err(rustix::io::Errno::NOENT) => {
                verify_directories(&directories)?;
                if !matches!(
                    fs::openat(
                        &d,
                        n,
                        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                        Mode::empty()
                    ),
                    Err(rustix::io::Errno::NOENT)
                ) {
                    return bad();
                }
                return Ok(DesktopLegacyCommitment {
                    files: Vec::new(),
                    layout_id: "absent".into(),
                    origin: "app://-".into(),
                    logical_legacy_entries: 0,
                });
            }
            Err(_) => return bad(),
        }
    }
    let names = list(&d)?;
    let (inventory, files) = read_files(&d, &path, &names)?;
    classify(&files)?;
    let names2 = list(&d)?;
    if names != names2 {
        return bad();
    }
    let (again, _) = read_files(&d, &path, &names2)?;
    if inventory != again {
        return bad();
    }
    verify_directories(&directories)?;
    Ok(DesktopLegacyCommitment {
        files: inventory,
        layout_id: "chromium-local-storage-leveldb-v1".into(),
        origin: "app://-".into(),
        logical_legacy_entries: 0,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusty_leveldb::{DB, in_memory};
    fn files(env: &dyn rusty_leveldb::env::Env) -> BTreeMap<String, Vec<u8>> {
        let mut out = BTreeMap::new();
        for p in env.children(Path::new("fixture")).unwrap() {
            let mut bytes = Vec::new();
            env.open_sequential_file(&Path::new("fixture").join(&p))
                .unwrap()
                .read_to_end(&mut bytes)
                .unwrap();
            out.insert(p.file_name().unwrap().to_str().unwrap().to_owned(), bytes);
        }
        out
    }
    #[test]
    fn ram_oracle_wal_and_tombstone() {
        let opt = in_memory();
        let env = opt.env.clone();
        let mut db = DB::open("fixture", opt).unwrap();
        db.put(b"VERSION", b"1").unwrap();
        db.put(b"_app://-\0\x01unrelated", b"secret").unwrap();
        db.flush().unwrap();
        assert!(classify(&files(env.as_ref().as_ref())).is_ok());
        let key = b"_app://-\0\x01codex:persisted-atom:thread";
        db.put(key, b"x").unwrap();
        db.flush().unwrap();
        assert!(classify(&files(env.as_ref().as_ref())).is_err());
        db.delete(key).unwrap();
        db.flush().unwrap();
        let clean = files(env.as_ref().as_ref());
        assert!(classify(&clean).is_ok());
        let mut corrupt = clean;
        let wal_name = corrupt
            .keys()
            .find(|n| n.ends_with(".log"))
            .unwrap()
            .clone();
        corrupt.get_mut(&wal_name).unwrap()[0] ^= 1;
        assert!(classify(&corrupt).is_err());
        let key16: Vec<u8> = b"_app://-\0\0"
            .iter()
            .copied()
            .chain(
                "codex:persisted-atom:abc"
                    .encode_utf16()
                    .flat_map(u16::to_le_bytes),
            )
            .collect();
        db.put(&key16, b"x").unwrap();
        db.flush().unwrap();
        assert!(classify(&files(env.as_ref().as_ref())).is_err());
    }
    #[test]
    fn ram_oracle_sst_and_corruption() {
        let opt = in_memory();
        let env = opt.env.clone();
        let mut db = DB::open("fixture", opt).unwrap();
        db.put(b"VERSION", b"1").unwrap();
        db.put(b"_app://-\0\x01codex:persisted-atom:old", b"x")
            .unwrap();
        db.delete(b"_app://-\0\x01codex:persisted-atom:old")
            .unwrap();
        db.compact_range(b"", b"\xff").unwrap();
        db.flush().unwrap();
        let snapshot = files(env.as_ref().as_ref());
        assert!(
            snapshot
                .keys()
                .any(|n| n.ends_with(".ldb") || n.ends_with(".sst"))
        );
        assert!(classify(&snapshot).is_ok());
        let mut obsolete = snapshot.clone();
        obsolete.insert("999999.ldb".into(), vec![0, 1, 2]);
        assert!(classify(&obsolete).is_ok());
        let mut broken = snapshot.clone();
        let name = broken
            .keys()
            .find(|n| n.ends_with(".ldb") || n.ends_with(".sst"))
            .unwrap()
            .clone();
        broken.get_mut(&name).unwrap()[0] ^= 1;
        assert!(classify(&broken).is_err());
        let mut broken = snapshot;
        let name = broken
            .keys()
            .find(|n| n.starts_with("MANIFEST-"))
            .unwrap()
            .clone();
        broken.get_mut(&name).unwrap().pop();
        assert!(classify(&broken).is_err());
    }
    #[test]
    fn sst_then_wal_delete_and_owned_readonly_inventory() {
        let opt = in_memory();
        let env = opt.env.clone();
        let mut db = DB::open("fixture", opt).unwrap();
        let target = b"_app://-\0\x01codex:persisted-atom:live";
        db.put(b"VERSION", b"1").unwrap();
        db.put(target, b"stored").unwrap();
        db.compact_range(b"", b"\xff").unwrap();
        db.flush().unwrap();
        let live = files(env.as_ref().as_ref());
        assert!(classify(&live).is_err());
        db.delete(target).unwrap();
        db.flush().unwrap();
        let snapshot = files(env.as_ref().as_ref());
        assert!(classify(&snapshot).is_ok());
        let dir = tempfile::tempdir().unwrap();
        let store = dir.path().join("Local Storage").join("leveldb");
        std::fs::create_dir_all(&store).unwrap();
        for (name, bytes) in &snapshot {
            std::fs::write(store.join(name), bytes).unwrap();
        }
        let m = std::fs::metadata(dir.path()).unwrap();
        let root = CleanupFileIdentity {
            path: std::fs::canonicalize(dir.path()).unwrap(),
            device: m.dev(),
            inode: m.ino(),
            links: m.nlink(),
            size_bytes: m.len(),
            mtime_seconds: m.mtime(),
            mtime_nanoseconds: m.mtime_nsec() as u32,
        };
        let observed = observe(&root).unwrap();
        assert_eq!(observed.files.len(), snapshot.len());
        for (name, bytes) in &snapshot {
            assert_eq!(&std::fs::read(store.join(name)).unwrap(), bytes);
        }
        std::fs::remove_file(store.join("CURRENT")).unwrap();
        std::os::unix::fs::symlink("MANIFEST-000001", store.join("CURRENT")).unwrap();
        assert!(observe(&root).is_err());
    }
}
