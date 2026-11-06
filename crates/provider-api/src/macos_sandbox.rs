//! The sole audited C-ABI boundary. See ADR-0004 and WebKit's SandboxSPI.h.
#![allow(unsafe_code)]

use crate::ProviderError;
use std::{
    ffi::{CString, OsStr},
    os::unix::ffi::OsStrExt,
    path::{Path, PathBuf},
};

#[link(name = "sandbox")]
unsafe extern "C" {
    fn sandbox_check(
        pid: libc::pid_t,
        operation: *const libc::c_char,
        filter: libc::c_int,
        ...
    ) -> libc::c_int;
    static SANDBOX_CHECK_NO_REPORT: libc::c_int;
}

unsafe extern "C" {
    fn csops(
        pid: libc::pid_t,
        operation: libc::c_uint,
        data: *mut libc::c_void,
        size: usize,
    ) -> libc::c_int;
}

#[derive(PartialEq, Eq)]
struct ProcessIdentity {
    pid: libc::pid_t,
    ppid: u32,
    uid: libc::uid_t,
    ruid: libc::uid_t,
    started_at: (u64, u64),
    zombie: bool,
}

#[derive(PartialEq, Eq)]
pub(crate) struct DesktopProcess {
    identity: ProcessIdentity,
    pub(crate) executable: PathBuf,
    pub(crate) signing_identifier: Vec<u8>,
}

#[derive(PartialEq, Eq)]
pub(crate) struct DesktopProcessCensus {
    pub(crate) processes: Vec<DesktopProcess>,
    pub(crate) complete: bool,
}

pub(crate) fn current_unprivileged_uid() -> Option<libc::uid_t> {
    // SAFETY: These credential queries have no pointer parameters or side effects.
    let (uid, euid) = unsafe { (libc::getuid(), libc::geteuid()) };
    (uid != 0 && uid == euid).then_some(uid)
}

/// Snapshot unprivileged current-user process metadata. No argv, environment,
/// process memory, open files, or provider state are queried.
pub(crate) fn desktop_processes() -> Option<DesktopProcessCensus> {
    use std::collections::BTreeSet;

    let uid = current_unprivileged_uid()?;
    let own_pid = libc::pid_t::try_from(std::process::id()).ok()?;
    // SDK sys/proc_info.h. A process can have either user credential.
    const PROC_UID_ONLY: u32 = 4;
    const PROC_RUID_ONLY: u32 = 5;
    let mut pids = BTreeSet::new();
    for filter in [PROC_UID_ONLY, PROC_RUID_ONLY] {
        let mut buffer = vec![0; 16_384];
        let capacity = libc::c_int::try_from(std::mem::size_of_val(buffer.as_slice())).ok()?;
        // SAFETY: buffer is an initialized, writable array of pid_t-sized values
        // and capacity is its exact byte length. No buffer outlives the call.
        let used =
            unsafe { libc::proc_listpids(filter, uid, buffer.as_mut_ptr().cast(), capacity) };
        if used <= 0
            || used >= capacity
            || !(used as usize).is_multiple_of(size_of::<libc::pid_t>())
        {
            return None;
        }
        let listed = &buffer[..used as usize / size_of::<libc::pid_t>()];
        let unique: BTreeSet<_> = listed.iter().copied().collect();
        if !unique.contains(&own_pid) || unique.len() != listed.len() || unique.first()? <= &0 {
            return None;
        }
        pids.extend(unique);
    }
    let mut processes = Vec::with_capacity(pids.len());
    let mut complete = true;
    for pid in pids {
        match observe_desktop_process(pid, uid) {
            Ok(Some(process)) => processes.push(process),
            Ok(None) => {}
            Err(()) => complete = false,
        }
    }
    Some(DesktopProcessCensus {
        processes,
        complete,
    })
}

fn observe_desktop_process(
    pid: libc::pid_t,
    uid: libc::uid_t,
) -> Result<Option<DesktopProcess>, ()> {
    let identity = process_identity(pid).ok_or(())?;
    if identity.uid != uid && identity.ruid != uid {
        return Err(());
    }
    if identity.zombie {
        return Ok(None);
    }
    let executable = process_executable(pid).ok_or(())?;
    let signing_identifier = process_signing_identifier(pid).ok_or(())?;
    if identity != process_identity(pid).ok_or(())? {
        return Err(());
    }
    Ok(Some(DesktopProcess {
        identity,
        executable,
        signing_identifier,
    }))
}

fn process_identity(pid: libc::pid_t) -> Option<ProcessIdentity> {
    let mut info = std::mem::MaybeUninit::<libc::proc_bsdinfo>::uninit();
    let size = libc::c_int::try_from(size_of::<libc::proc_bsdinfo>()).ok()?;
    // SAFETY: PROC_PIDTBSDINFO writes a proc_bsdinfo to the supplied writable
    // storage. Only assume initialization after the exact ABI byte count.
    let used = unsafe {
        libc::proc_pidinfo(
            pid,
            libc::PROC_PIDTBSDINFO,
            0,
            info.as_mut_ptr().cast(),
            size,
        )
    };
    if used != size {
        return None;
    }
    // SAFETY: The whole proc_bsdinfo was written by the kernel.
    let info = unsafe { info.assume_init() };
    if info.pbi_pid != pid as u32 || info.pbi_start_tvsec == 0 || info.pbi_start_tvusec >= 1_000_000
    {
        return None;
    }
    Some(ProcessIdentity {
        pid,
        ppid: info.pbi_ppid,
        uid: info.pbi_uid,
        ruid: info.pbi_ruid,
        started_at: (info.pbi_start_tvsec, info.pbi_start_tvusec),
        zombie: info.pbi_status == libc::SZOMB,
    })
}

fn process_executable(pid: libc::pid_t) -> Option<PathBuf> {
    let mut bytes = [0_u8; libc::PROC_PIDPATHINFO_MAXSIZE as usize];
    // SAFETY: the buffer has the exact maximum size specified by sys/proc_info.h
    // and is writable for the duration of this kernel query.
    let used = unsafe { libc::proc_pidpath(pid, bytes.as_mut_ptr().cast(), bytes.len() as u32) };
    let length = usize::try_from(used).ok()?;
    let path = bytes.get(..length)?;
    if length == 0 || bytes.get(length) != Some(&0) || path.contains(&0) {
        return None;
    }
    let path = Path::new(OsStr::from_bytes(path));
    path.is_absolute().then(|| path.to_path_buf())
}

fn process_signing_identifier(pid: libc::pid_t) -> Option<Vec<u8>> {
    // XNU sys/codesign.h and kern/kern_proc.c, CS_OPS_IDENTITY only.
    const CS_OPS_IDENTITY: libc::c_uint = 11;
    let mut bytes = [0_u8; 1024];
    // SAFETY: the pointer names writable storage of exactly the supplied length.
    // This fixed csops operation queries only the executable's signing identifier.
    if unsafe { csops(pid, CS_OPS_IDENTITY, bytes.as_mut_ptr().cast(), bytes.len()) } != 0 {
        return None;
    }
    signing_identifier_from_blob(&bytes).map(<[u8]>::to_vec)
}

fn signing_identifier_from_blob(bytes: &[u8]) -> Option<&[u8]> {
    // XNU returns four zero bytes and a big-endian total size, then a C string.
    let length = u32::from_be_bytes(bytes.get(4..8)?.try_into().ok()?) as usize;
    let identity = bytes.get(8..length)?.strip_suffix(&[0])?;
    (bytes.get(..4)? == [0; 4] && !identity.is_empty() && !identity.contains(&0))
        .then_some(identity)
}

unsafe extern "C" {
    fn filesec_init() -> *mut libc::c_void;
    fn filesec_free(filesec: *mut libc::c_void);
    fn filesec_query_property(
        filesec: *mut libc::c_void,
        property: libc::c_int,
        present: *mut libc::c_int,
    ) -> libc::c_int;
    #[cfg_attr(target_arch = "x86_64", link_name = "fstatx_np$INODE64")]
    fn fstatx_np(fd: libc::c_int, stat: *mut libc::stat, filesec: *mut libc::c_void)
    -> libc::c_int;
}

/// Require positive absence of an extended ACL on the already opened file.
/// SDK sys/fcntl.h defines FILESEC_ACL=5; sys/stat.h defines fstatx_np.
pub(crate) fn verify_no_descriptor_acl(file: &std::fs::File) -> Result<(), ProviderError> {
    use std::os::fd::AsRawFd;
    let unavailable = || {
        ProviderError::CapabilityUnavailable(
            "descriptor ACL state unavailable or unsupported".into(),
        )
    };
    // SAFETY: filesec_init allocates an opaque object; we do not dereference it,
    // pass it only to its documented query functions, and free it exactly once.
    let security = unsafe { filesec_init() };
    if security.is_null() {
        return Err(unavailable());
    }
    let mut metadata = std::mem::MaybeUninit::<libc::stat>::uninit();
    let mut present: libc::c_int = -1;
    let mut owner_present: libc::c_int = -1;
    // SAFETY: file owns a live descriptor, stat has the SDK/libc layout and enough
    // writable space, and present is a live initialized integer. No path is used.
    let read = unsafe { fstatx_np(file.as_raw_fd(), metadata.as_mut_ptr(), security) };
    let (owner_query, acl_query) = if read == 0 {
        // SAFETY: the initialized opaque object remains alive until free below.
        unsafe {
            (
                filesec_query_property(security, 1, &mut owner_present),
                filesec_query_property(security, 5, &mut present),
            )
        }
    } else {
        (-1, -1)
    };
    // SAFETY: this is the unique allocation from filesec_init above.
    unsafe { filesec_free(security) };
    // Darwin's allocation failure can return zero with unset properties.
    if read == 0 && owner_query == 0 && owner_present == 1 && acl_query == 0 && present == 0 {
        Ok(())
    } else {
        Err(unavailable())
    }
}

fn explicitly_denied(path: &Path, operation: &std::ffi::CStr) -> Result<bool, ProviderError> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| ProviderError::InvalidScope("sandbox query path contains NUL".into()))?;
    let pid = libc::pid_t::try_from(std::process::id()).map_err(|_| {
        ProviderError::CapabilityUnavailable("sandbox process identity unavailable".into())
    })?;
    // SAFETY: This matches the maintained Darwin C ABI. Both NUL-terminated
    // strings remain alive for the call; the sole variadic argument is a path
    // pointer selected by SANDBOX_FILTER_PATH (1). The flag is an immutable
    // exported C constant. No buffers are written or retained by sandbox_check.
    let result = unsafe {
        sandbox_check(
            pid,
            operation.as_ptr(),
            1 | SANDBOX_CHECK_NO_REPORT,
            path.as_ptr(),
        )
    };
    match result {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProviderError::CapabilityUnavailable(
            "kernel sandbox policy query failed".into(),
        )),
    }
}

/// Require real MAC denial before SQLite can touch source shared-memory files.
pub(crate) fn verify_source_write_denial(root: &Path) -> Result<(), ProviderError> {
    let main = root.join("state_5.sqlite");
    if !main.is_file() {
        return Err(ProviderError::CapabilityUnavailable(
            "source database is missing".into(),
        ));
    }
    let mut paths = vec![root.to_path_buf(), main];
    for name in [
        "state_5.sqlite-wal",
        "state_5.sqlite-shm",
        "state_5.sqlite-journal",
    ] {
        let path = root.join(name);
        // sandbox_check can report denial for a nonexistent path even without
        // a sandbox. Existing objects and the directory establish actual policy.
        if path.exists() {
            paths.push(path);
        }
    }
    for path in paths {
        for operation in [
            c"file-write-data",
            c"file-write-create",
            c"file-write-unlink",
        ] {
            if !explicitly_denied(&path, operation)? {
                return Err(ProviderError::CapabilityUnavailable(
                    "source writes are not denied by the current kernel sandbox".into(),
                ));
            }
        }
    }
    Ok(())
}

/// Attest the fixed Desktop catalog directory and every existing SQLite file.
pub(crate) fn verify_catalog_write_denial(root: &Path) -> Result<(), ProviderError> {
    let directory = root.join("sqlite");
    let meta = std::fs::symlink_metadata(&directory).map_err(|_| {
        ProviderError::CapabilityUnavailable("catalog directory unavailable".into())
    })?;
    if !meta.is_dir() || meta.file_type().is_symlink() {
        return Err(ProviderError::CapabilityUnavailable(
            "catalog directory rejected".into(),
        ));
    }
    let mut paths = vec![directory.clone()];
    for (name, required) in [
        ("codex.db", true),
        ("codex.db-wal", false),
        ("codex.db-shm", false),
        ("codex.db-journal", false),
    ] {
        let path = directory.join(name);
        match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.is_file() && !meta.file_type().is_symlink() => paths.push(path),
            Err(e) if !required && e.kind() == std::io::ErrorKind::NotFound => (),
            _ => {
                return Err(ProviderError::CapabilityUnavailable(
                    "catalog file rejected".into(),
                ));
            }
        }
    }
    for path in paths {
        for operation in [
            c"file-write-data",
            c"file-write-create",
            c"file-write-unlink",
        ] {
            if !explicitly_denied(&path, operation)? {
                return Err(ProviderError::CapabilityUnavailable(
                    "catalog writes are not denied by the current kernel sandbox".into(),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn signing_identifier_query_reads_only_the_declared_blob() {
        let valid = b"\0\0\0\0\0\0\0\x0ecodex\0unused";
        assert_eq!(
            signing_identifier_from_blob(valid),
            Some(b"codex".as_slice())
        );
        for invalid in [
            b"\0\0\0\0\0\0\0\x0fshort\0".as_slice(),
            b"\0\0\0\0\0\0\0\x0dtrunc",
            b"\0\0\0\0\0\0\0\x0ea\0bcd\0",
            b"\x01\0\0\0\0\0\0\x0ecodex\0",
        ] {
            assert_eq!(signing_identifier_from_blob(invalid), None);
        }
    }

    #[cfg(target_arch = "aarch64")]
    #[test]
    fn process_query_reads_current_executable_identity() {
        let pid = libc::pid_t::try_from(std::process::id()).unwrap();
        let identity = process_identity(pid).unwrap();
        assert_eq!(identity.pid, pid);
        assert!(!identity.zombie);
        assert_eq!(
            process_executable(pid).unwrap(),
            std::env::current_exe().unwrap()
        );
        assert!(!process_signing_identifier(pid).unwrap().is_empty());
    }

    #[test]
    fn catalog_filesystem_permissions_do_not_count_as_kernel_sandbox_denial() {
        let root = tempfile::tempdir().unwrap();
        let sqlite = root.path().join("sqlite");
        std::fs::create_dir(&sqlite).unwrap();
        std::fs::write(sqlite.join("codex.db"), b"fixture").unwrap();
        assert!(verify_catalog_write_denial(root.path()).is_err());
    }
    #[test]
    fn filesystem_permissions_do_not_count_as_kernel_sandbox_denial() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state_5.sqlite");
        std::fs::write(&path, b"fixture").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
        assert!(!explicitly_denied(&path, c"file-write-data").unwrap());
        assert!(verify_source_write_denial(directory.path()).is_err());
    }
}
