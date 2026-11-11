//! Derived lexical evidence, never ownership or approval. See ADR-0027.
//! Contentless FTS retains sensitive terms; callers must opt in to disk storage.
use crate::OwnedStateRoot;
use rusqlite::{Connection, OpenFlags, OptionalExtension, params};
use std::{collections::BTreeSet, io, path::Path};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum SearchIndexError {
    #[error("private search storage unavailable")]
    Storage(#[from] io::Error),
    #[error("search index unavailable; explicit reset/rebuild may be required")]
    Sqlite(#[from] rusqlite::Error),
    #[error("invalid or stale search index input")]
    Invalid,
}
#[derive(Debug, Clone)]
pub struct IndexDocument<'a> {
    pub ordinal: u64,
    pub chunk: u32,
    pub text: &'a str,
    /// Opaque source-bound navigation data; never interpreted as authority.
    pub reference: &'a str,
}
#[derive(Debug, Clone, serde::Serialize)]
pub struct IndexStatus {
    pub schema: u32,
    pub sqlite_version: String,
    pub persistent: bool,
    pub sources: usize,
    pub complete_sources: usize,
    pub documents: usize,
    pub database_bytes: u64,
    pub contains_sensitive_terms: bool,
    pub contains_message_text: bool,
}
#[derive(Debug, Clone)]
pub struct IndexMatch {
    pub source: String,
    pub revision: String,
    pub ordinal: u64,
    pub chunk: u32,
    pub reference: String,
    /// SQLite BM25: lower values rank first. Not semantic confidence.
    pub bm25: f64,
}
/// Fixed local index with no provider access and no executable input.
pub struct SearchTextIndex {
    db: Connection,
    root: Option<OwnedStateRoot>,
    held: Option<std::fs::File>,
    lock: Option<std::fs::File>,
}
impl SearchTextIndex {
    pub fn memory() -> Result<Self, SearchIndexError> {
        Self::initialize(Connection::open_in_memory()?, None)
    }
    /// The private root capability enforces owner-only permissions and rejects
    /// overlap with protected provider roots. No arbitrary filename is accepted.
    pub fn persistent(root: OwnedStateRoot) -> Result<Self, SearchIndexError> {
        validate_sidecars(&root)?;
        let lock = root.open_child("search.lock", true, false)?;
        lock.try_lock_shared()
            .map_err(|_| SearchIndexError::Invalid)?;
        let held = root.open_child("search.sqlite3", true, false)?;
        let db = Connection::open_with_flags(
            std::fs::canonicalize(
                root.search_index_path()
                    .parent()
                    .ok_or(SearchIndexError::Invalid)?,
            )?
            .join("search.sqlite3"),
            OpenFlags::SQLITE_OPEN_READ_WRITE
                | OpenFlags::SQLITE_OPEN_NOFOLLOW
                | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        )?;
        // Keep the verified inode alive until SQLite has opened the same path;
        // repeated capability validation detects replacement before operations.
        let verified = root.open_child_existing("search.sqlite3", true)?;
        if !same_file(&held.metadata()?, &verified.metadata()?) {
            return Err(SearchIndexError::Invalid);
        }
        let mut result = Self::initialize(db, Some(root))?;
        result.held = Some(held);
        result.lock = Some(lock);
        result.check()?;
        Ok(result)
    }
    fn initialize(db: Connection, root: Option<OwnedStateRoot>) -> Result<Self, SearchIndexError> {
        db.busy_timeout(std::time::Duration::from_millis(250))?;
        db.execute_batch("PRAGMA journal_mode=DELETE; PRAGMA secure_delete=ON; PRAGMA temp_store=MEMORY; PRAGMA cache_size=-4096; PRAGMA max_page_count=262144; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;")?;
        let version: i64 = db.query_row("PRAGMA user_version", [], |r| r.get(0))?;
        if version != 0 && version != 1 {
            return Err(SearchIndexError::Invalid);
        }
        db.execute_batch("CREATE TABLE IF NOT EXISTS search_sources(source TEXT PRIMARY KEY, revision TEXT NOT NULL, next_offset INTEGER NOT NULL, complete INTEGER NOT NULL);
            CREATE TABLE IF NOT EXISTS search_documents(id INTEGER PRIMARY KEY,source TEXT NOT NULL REFERENCES search_sources(source) ON DELETE CASCADE,ordinal INTEGER NOT NULL,chunk INTEGER NOT NULL,reference TEXT NOT NULL,body TEXT NOT NULL DEFAULT '',UNIQUE(source,ordinal,chunk));
            CREATE TABLE IF NOT EXISTS search_source_state(source TEXT PRIMARY KEY REFERENCES search_sources(source) ON DELETE CASCADE,metadata TEXT NOT NULL,checkpoint TEXT,exhausted INTEGER NOT NULL);
            CREATE VIRTUAL TABLE IF NOT EXISTS search_terms USING fts5(text,content='',contentless_delete=1,tokenize='unicode61');
            INSERT INTO search_terms(search_terms,rank) VALUES('secure-delete',1);
            PRAGMA user_version=1;")?;
        db.query_row("PRAGMA quick_check", [], |r| r.get::<_, String>(0))
            .and_then(|s| {
                if s == "ok" {
                    Ok(())
                } else {
                    Err(rusqlite::Error::InvalidQuery)
                }
            })?;
        Ok(Self {
            db,
            root,
            held: None,
            lock: None,
        })
    }
    fn check(&self) -> Result<(), SearchIndexError> {
        if let Some(root) = &self.root {
            validate_sidecars(root)?;
            if let Some(lock) = &self.lock {
                let current_lock = root.open_child_existing("search.lock", false)?;
                if !same_file(&lock.metadata()?, &current_lock.metadata()?) {
                    return Err(SearchIndexError::Invalid);
                }
            }
            let current = root.open_child_existing("search.sqlite3", true)?;
            if let Some(held) = &self.held
                && !same_file(&held.metadata()?, &current.metadata()?)
            {
                return Err(SearchIndexError::Invalid);
            }
        }
        Ok(())
    }
    /// Compatibility helper for ephemeral per-message BM25 scoring.
    pub fn add(&mut self, ordinal: u64, text: &str) -> Result<(), SearchIndexError> {
        self.add_chunk(ordinal, 0, text)
    }
    pub fn add_chunk(
        &mut self,
        ordinal: u64,
        chunk: u32,
        text: &str,
    ) -> Result<(), SearchIndexError> {
        let reference = ordinal.to_string();
        self.append_page(
            "ephemeral",
            "ephemeral-v1",
            0,
            0,
            false,
            &[IndexDocument {
                ordinal,
                chunk,
                text,
                reference: &reference,
            }],
        )
    }
    /// Atomically append a validated page. Revision changes invalidate all old
    /// postings. Same-page replay is idempotent; out-of-order pages refuse.
    pub fn append_page(
        &mut self,
        source: &str,
        revision: &str,
        start_offset: u64,
        next_offset: u64,
        complete: bool,
        documents: &[IndexDocument<'_>],
    ) -> Result<(), SearchIndexError> {
        self.check()?;
        if source.is_empty()
            || source.len() > 4096
            || revision.is_empty()
            || revision.len() > 512
            || next_offset < start_offset
            || next_offset > i64::MAX as u64
            || documents.len() > 4096
            || documents.iter().any(|d| {
                d.text.len() > 1024 * 1024
                    || d.ordinal > i64::MAX as u64
                    || d.reference.len() > 16 * 1024
                    || !agentmender_domain::safe_conversation_text(d.text)
            })
        {
            return Err(SearchIndexError::Invalid);
        }
        let persist_text = self.root.is_some();
        let tx = self.db.transaction()?;
        let current: Option<(String, i64)> = tx
            .query_row(
                "SELECT revision,next_offset FROM search_sources WHERE source=?1",
                [source],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .optional()?;
        if current.as_ref().is_some_and(|(r, _)| r != revision) {
            if start_offset != 0 {
                return Err(SearchIndexError::Invalid);
            }
            tx.execute("DELETE FROM search_terms WHERE rowid IN (SELECT id FROM search_documents WHERE source=?1)",[source])?;
            tx.execute("DELETE FROM search_sources WHERE source=?1", [source])?;
        } else if current.as_ref().is_some_and(|(_, offset)| {
            *offset != start_offset as i64 && *offset != next_offset as i64
        }) || (current.is_none() && start_offset != 0)
        {
            return Err(SearchIndexError::Invalid);
        }
        tx.execute("INSERT INTO search_sources(source,revision,next_offset,complete) VALUES(?1,?2,?3,?4) ON CONFLICT(source) DO UPDATE SET next_offset=excluded.next_offset,complete=excluded.complete",params![source,revision,next_offset as i64,complete])?;
        for d in documents {
            let old: Option<i64> = tx
                .query_row(
                    "SELECT id FROM search_documents WHERE source=?1 AND ordinal=?2 AND chunk=?3",
                    params![source, d.ordinal as i64, d.chunk],
                    |r| r.get(0),
                )
                .optional()?;
            if let Some(id) = old {
                tx.execute("DELETE FROM search_terms WHERE rowid=?1", [id])?;
            }
            tx.execute("INSERT INTO search_documents(source,ordinal,chunk,reference,body) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(source,ordinal,chunk) DO UPDATE SET reference=excluded.reference,body=excluded.body",params![source,d.ordinal as i64,d.chunk,d.reference,if persist_text {d.text} else {""}])?;
            let id: i64 = tx.query_row(
                "SELECT id FROM search_documents WHERE source=?1 AND ordinal=?2 AND chunk=?3",
                params![source, d.ordinal as i64, d.chunk],
                |r| r.get(0),
            )?;
            tx.execute(
                "INSERT INTO search_terms(rowid,text) VALUES(?1,?2)",
                params![id, d.text],
            )?;
        }
        tx.commit()?;
        self.check()
    }
    pub fn set_source_state(
        &self,
        source: &str,
        metadata: &str,
        checkpoint: Option<&str>,
        exhausted: bool,
    ) -> Result<(), SearchIndexError> {
        self.check()?;
        if metadata.len() > 32 * 1024 || checkpoint.is_some_and(|s| s.len() > 64 * 1024) {
            return Err(SearchIndexError::Invalid);
        }
        self.db.execute("INSERT INTO search_source_state(source,metadata,checkpoint,exhausted) VALUES(?1,?2,?3,?4) ON CONFLICT(source) DO UPDATE SET metadata=excluded.metadata,checkpoint=excluded.checkpoint,exhausted=excluded.exhausted",params![source,metadata,checkpoint,exhausted])?;
        Ok(())
    }
    pub fn source_state(
        &self,
        source: &str,
    ) -> Result<Option<StoredSearchSource>, SearchIndexError> {
        self.check()?;
        Ok(self.db.query_row("SELECT s.source,s.revision,s.next_offset,s.complete,a.metadata,a.checkpoint,a.exhausted FROM search_sources s JOIN search_source_state a ON a.source=s.source WHERE s.source=?1",[source],stored_source).optional()?)
    }
    pub fn source_states(&self, limit: usize) -> Result<Vec<StoredSearchSource>, SearchIndexError> {
        self.check()?;
        if limit > 1000 {
            return Err(SearchIndexError::Invalid);
        }
        let mut stmt=self.db.prepare("SELECT s.source,s.revision,s.next_offset,s.complete,a.metadata,a.checkpoint,a.exhausted FROM search_sources s JOIN search_source_state a ON a.source=s.source ORDER BY s.source LIMIT ?1")?;
        Ok(stmt
            .query_map([limit as i64], stored_source)?
            .collect::<Result<_, _>>()?)
    }
    /// Stream opted-in safe message text one row at a time; no corpus-sized Vec.
    pub fn visit_documents(
        &self,
        source: &str,
        mut visit: impl FnMut(u64, &str, &str) -> bool,
    ) -> Result<bool, SearchIndexError> {
        self.check()?;
        let mut stmt=self.db.prepare("SELECT ordinal,body,reference FROM search_documents WHERE source=?1 ORDER BY ordinal,chunk")?;
        let mut rows = stmt.query([source])?;
        while let Some(row) = rows.next()? {
            let text: String = row.get(1)?;
            let reference: String = row.get(2)?;
            if !visit(row.get::<_, i64>(0)? as u64, &text, &reference) {
                return Ok(false);
            }
        }
        Ok(true)
    }
    pub fn checkpoint(
        &self,
        source: &str,
        revision: &str,
    ) -> Result<Option<(u64, bool)>, SearchIndexError> {
        self.check()?;
        Ok(self
            .db
            .query_row(
                "SELECT next_offset,complete FROM search_sources WHERE source=?1 AND revision=?2",
                params![source, revision],
                |r| Ok((r.get::<_, i64>(0)? as u64, r.get(1)?)),
            )
            .optional()?)
    }
    /// Literal token OR candidate generation. Query syntax/SQL are never accepted.
    pub fn search(&self, query: &str, limit: usize) -> Result<Vec<IndexMatch>, SearchIndexError> {
        self.check()?;
        if query.len() > 4096 || limit == 0 || limit > 1000 {
            return Err(SearchIndexError::Invalid);
        }
        let Some(expression) = literal_match(query) else {
            return Ok(Vec::new());
        };
        let mut statement=self.db.prepare("SELECT d.source,s.revision,d.ordinal,d.chunk,d.reference,bm25(search_terms) FROM search_terms JOIN search_documents d ON d.id=search_terms.rowid JOIN search_sources s ON s.source=d.source WHERE search_terms MATCH ?1 ORDER BY bm25(search_terms),d.source,d.ordinal,d.chunk LIMIT ?2")?;
        let rows = statement.query_map(params![expression, limit as i64], |r| {
            Ok(IndexMatch {
                source: r.get(0)?,
                revision: r.get(1)?,
                ordinal: r.get::<_, i64>(2)? as u64,
                chunk: r.get(3)?,
                reference: r.get(4)?,
                bm25: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Propose cache sources among the caller's eligible keys. Global BM25
    /// statistics from protected or ineligible sources do not affect the order.
    pub fn matching_sources(
        &self,
        query: &str,
        eligible: &[&str],
    ) -> Result<BTreeSet<String>, SearchIndexError> {
        self.check()?;
        if eligible.is_empty() {
            return Ok(BTreeSet::new());
        }
        if query.len() > 4096
            || eligible.len() > 1000
            || eligible.iter().any(|key| key.len() > 4096)
        {
            return Err(SearchIndexError::Invalid);
        }
        let Some(expression) = literal_match(query) else {
            return Ok(BTreeSet::new());
        };
        let keys = serde_json::to_string(eligible).map_err(|_| SearchIndexError::Invalid)?;
        let mut statement = self.db.prepare(
            "SELECT DISTINCT d.source FROM search_terms
             JOIN search_documents d ON d.id=search_terms.rowid
             WHERE search_terms MATCH ?1 AND d.source IN (SELECT value FROM json_each(?2))",
        )?;
        let rows = statement.query_map(params![expression, keys], |row| row.get(0))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn rank(&self, query: &str, limit: usize) -> Result<Vec<(u64, f64)>, SearchIndexError> {
        Ok(self
            .search(query, limit)?
            .into_iter()
            .map(|m| (m.ordinal, m.bm25))
            .collect())
    }
    /// Remove stale/missing sources only after a complete trusted inventory.
    /// A partial inventory must never be used to infer deletion.
    pub fn retain_sources(
        &mut self,
        sources: &[String],
        inventory_complete: bool,
    ) -> Result<(), SearchIndexError> {
        self.check()?;
        if !inventory_complete {
            return Err(SearchIndexError::Invalid);
        }
        let values = serde_json::to_string(sources).map_err(|_| SearchIndexError::Invalid)?;
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM search_terms WHERE rowid IN (SELECT id FROM search_documents WHERE source NOT IN (SELECT value FROM json_each(?1)))",[&values])?;
        tx.execute(
            "DELETE FROM search_sources WHERE source NOT IN (SELECT value FROM json_each(?1))",
            [values],
        )?;
        tx.commit()?;
        Ok(())
    }
    pub fn remove_source(&mut self, source: &str) -> Result<(), SearchIndexError> {
        self.check()?;
        let tx = self.db.transaction()?;
        tx.execute("DELETE FROM search_terms WHERE rowid IN (SELECT id FROM search_documents WHERE source=?1)",[source])?;
        tx.execute("DELETE FROM search_sources WHERE source=?1", [source])?;
        tx.commit()?;
        Ok(())
    }
    pub fn reset(&mut self) -> Result<(), SearchIndexError> {
        self.retain_sources(&[], true)?;
        self.db.execute_batch("PRAGMA incremental_vacuum;")?;
        Ok(())
    }
    pub fn status(&self) -> Result<IndexStatus, SearchIndexError> {
        self.check()?;
        let count = |sql| {
            self.db
                .query_row(sql, [], |r| r.get::<_, i64>(0).map(|n| n as usize))
        };
        let pages: i64 = self.db.query_row("PRAGMA page_count", [], |r| r.get(0))?;
        let size: i64 = self.db.query_row("PRAGMA page_size", [], |r| r.get(0))?;
        Ok(IndexStatus {
            schema: 1,
            sqlite_version: rusqlite::version().into(),
            persistent: self.root.is_some(),
            sources: count("SELECT count(*) FROM search_sources")?,
            complete_sources: count("SELECT count(*) FROM search_sources WHERE complete=1")?,
            documents: count("SELECT count(*) FROM search_documents")?,
            database_bytes: (pages * size) as u64,
            contains_sensitive_terms: true,
            contains_message_text: self.root.is_some(),
        })
    }
    pub fn integrity_check(&self) -> Result<(), SearchIndexError> {
        self.check()?;
        self.db.execute(
            "INSERT INTO search_terms(search_terms) VALUES('integrity-check')",
            [],
        )?;
        Ok(())
    }
}

fn literal_match(query: &str) -> Option<String> {
    let terms: Vec<_> = query
        .split_whitespace()
        .take(64)
        .filter(|term| term.chars().any(char::is_alphanumeric))
        .map(|term| format!("\"{}\"", term.replace('"', "\"\"")))
        .collect();
    (!terms.is_empty()).then(|| terms.join(" OR "))
}
#[derive(Debug, Clone)]
pub struct StoredSearchSource {
    pub source: String,
    pub revision: String,
    pub next_offset: u64,
    pub complete: bool,
    pub metadata: String,
    pub checkpoint: Option<String>,
    pub exhausted: bool,
}
fn stored_source(row: &rusqlite::Row<'_>) -> rusqlite::Result<StoredSearchSource> {
    Ok(StoredSearchSource {
        source: row.get(0)?,
        revision: row.get(1)?,
        next_offset: row.get::<_, i64>(2)? as u64,
        complete: row.get(3)?,
        metadata: row.get(4)?,
        checkpoint: row.get(5)?,
        exhausted: row.get(6)?,
    })
}
fn validate_sidecars(root: &OwnedStateRoot) -> Result<(), SearchIndexError> {
    root.revalidate()?;
    for name in ["search.sqlite3-wal", "search.sqlite3-shm"] {
        match root.open_child_existing(name, false) {
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            _ => return Err(SearchIndexError::Invalid),
        }
    }
    match root.open_child_existing("search.sqlite3-journal", false) {
        Ok(_) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e.into()),
    }
    Ok(())
}
fn same_file(a: &std::fs::Metadata, b: &std::fs::Metadata) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        a.dev() == b.dev() && a.ino() == b.ino()
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        a.creation_time() == b.creation_time() && a.file_size() == b.file_size()
    }
}
/// Open/create only the explicitly selected dedicated directory, disjoint from
/// every configured provider root. Its parent directory must already exist.
pub fn open_search_index(
    path: &Path,
    protected: &[std::path::PathBuf],
) -> Result<SearchTextIndex, SearchIndexError> {
    let root = match OwnedStateRoot::open(path, protected) {
        Ok(r) => r,
        Err(e) if e.kind() == io::ErrorKind::NotFound => OwnedStateRoot::create(path, protected)?,
        Err(e) => return Err(e.into()),
    };
    SearchTextIndex::persistent(root)
}

/// Explicit offline rebuild after corruption. Only fixed derived files inside
/// the validated private capability can be removed. Active index connections
/// hold shared locks and prevent this operation.
pub fn rebuild_search_index(
    path: &Path,
    protected: &[std::path::PathBuf],
) -> Result<SearchTextIndex, SearchIndexError> {
    let root = OwnedStateRoot::open(path, protected)?;
    validate_sidecars(&root)?;
    let lock = root.open_child("search.lock", true, false)?;
    lock.try_lock().map_err(|_| SearchIndexError::Invalid)?;
    for name in ["search.sqlite3-journal", "search.sqlite3"] {
        match root.open_child_existing(name, true) {
            Ok(held) => {
                root.revalidate()?;
                let current = root.open_child_existing(name, true)?;
                if !same_file(&held.metadata()?, &current.metadata()?) {
                    return Err(SearchIndexError::Invalid);
                }
                drop(current);
                drop(held);
                let path = root.search_index_path();
                let target = path.parent().ok_or(SearchIndexError::Invalid)?.join(name);
                std::fs::remove_file(target)?;
            }
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    drop(lock);
    SearchTextIndex::persistent(root)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contentless_bm25_incremental_replacement_and_invalidation() {
        let mut index = SearchTextIndex::memory().unwrap();
        let docs = [
            IndexDocument {
                ordinal: 1,
                chunk: 0,
                text: "Earth Cesium camera rendering",
                reference: "signed-ref",
            },
            IndexDocument {
                ordinal: 2,
                chunk: 0,
                text: "PostgreSQL indexing query plan",
                reference: "other",
            },
        ];
        index
            .append_page("codex:root:session", "rev1", 0, 100, false, &docs)
            .unwrap();
        index
            .append_page("codex:root:session", "rev1", 0, 100, false, &docs)
            .unwrap();
        assert_eq!(index.status().unwrap().documents, 2);
        assert_eq!(index.search("Cesium", 10).unwrap()[0].ordinal, 1);
        assert_eq!(
            index.checkpoint("codex:root:session", "rev1").unwrap(),
            Some((100, false))
        );
        assert!(
            index
                .append_page("codex:root:session", "rev1", 200, 300, true, &[])
                .is_err()
        );
        index
            .append_page("codex:root:session", "rev2", 0, 120, true, &docs[1..])
            .unwrap();
        assert!(index.search("Cesium", 10).unwrap().is_empty());
        assert_eq!(index.status().unwrap().complete_sources, 1);
        assert!(index.retain_sources(&[], false).is_err());
        index.retain_sources(&[], true).unwrap();
        assert_eq!(index.status().unwrap().documents, 0);
        index.integrity_check().unwrap();
    }
    #[test]
    fn literal_query_and_secret_suppression() {
        let mut index = SearchTextIndex::memory().unwrap();
        index.add(1, "Cesium orbital graphics").unwrap();
        assert!(index.add(2, "password hidden").is_err());
        let _ = index
            .search("\" OR NEAR (DELETE FROM threads)", 10)
            .unwrap();
        let text: Option<String> = index
            .db
            .query_row("SELECT text FROM search_terms LIMIT 1", [], |r| r.get(0))
            .unwrap();
        assert_eq!(text, None);
    }
    #[test]
    fn private_persistence_and_reset() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("provider");
        std::fs::create_dir(&source).unwrap();
        let path = temp.path().join("index");
        {
            let mut index = open_search_index(&path, std::slice::from_ref(&source)).unwrap();
            index.add(1, "private discussion").unwrap();
        }
        let mut index = open_search_index(&path, std::slice::from_ref(&source)).unwrap();
        assert_eq!(index.search("discussion", 10).unwrap().len(), 1);
        index.reset().unwrap();
        assert_eq!(index.status().unwrap().documents, 0);
        assert!(open_search_index(&source, std::slice::from_ref(&source)).is_err());
    }
    #[test]
    fn offline_rebuild_refuses_open_connections_and_recovers_corruption() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("provider");
        std::fs::create_dir(&source).unwrap();
        let path = temp.path().join("index");
        let index = open_search_index(&path, std::slice::from_ref(&source)).unwrap();
        assert!(rebuild_search_index(&path, std::slice::from_ref(&source)).is_err());
        drop(index);
        std::fs::write(path.join("search.sqlite3"), b"corrupt derived data").unwrap();
        assert!(open_search_index(&path, std::slice::from_ref(&source)).is_err());
        let rebuilt = rebuild_search_index(&path, std::slice::from_ref(&source)).unwrap();
        assert_eq!(rebuilt.status().unwrap().documents, 0);
        assert!(rebuilt.status().unwrap().contains_message_text);
    }
    #[test]
    fn memory_mode_retains_no_original_body_and_chunk_postings_remain_distinct() {
        let mut index = SearchTextIndex::memory().unwrap();
        index.add_chunk(1, 0, "Cesium rendering").unwrap();
        index.add_chunk(1, 1, "PostgreSQL indexing").unwrap();
        assert_eq!(index.status().unwrap().documents, 2);
        assert_eq!(index.search("Cesium", 10).unwrap()[0].ordinal, 1);
        assert_eq!(index.search("PostgreSQL", 10).unwrap()[0].ordinal, 1);
        assert!(!index.status().unwrap().contains_message_text);
        let mut bodies = Vec::new();
        assert!(
            index
                .visit_documents("ephemeral", |_, body, _| {
                    bodies.push(body.to_string());
                    true
                })
                .unwrap()
        );
        assert!(bodies.iter().all(String::is_empty));
    }
}
