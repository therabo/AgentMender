//! Per-run UUID inventory, owned by the confined read-only provider worker.
use super::*;

const RETENTION: Duration = Duration::from_secs(600);
const COHORTS: usize = 4;

#[derive(Serialize, Deserialize)]
struct FrozenRow {
    title: Option<String>,
    project: Option<String>,
    created: Option<i64>,
    updated: Option<i64>,
    state: SessionState,
    path: Option<PathBuf>,
}

impl From<Candidate> for FrozenRow {
    fn from(candidate: Candidate) -> Self {
        let metadata = candidate.metadata;
        Self {
            title: metadata.title,
            project: metadata.project_label,
            created: metadata.created_at,
            updated: metadata.last_active_at,
            state: metadata.state,
            path: candidate.path,
        }
    }
}

impl FrozenRow {
    fn candidate(self, root: &Path, id: String) -> Result<Candidate, ProviderError> {
        Ok(Candidate {
            metadata: SessionMetadata {
                identity: identity(root, &id)?,
                native_session_id: id,
                title: self.title,
                project_label: self.project,
                created_at: self.created,
                last_active_at: self.updated,
                state: self.state,
                producer_version_matches: false,
                producer_version: None,
                parent_native_session_id: None,
            },
            path: self.path,
        })
    }
}

struct Cohort {
    scope: CohortScope,
    db: Connection,
    len: usize,
    touched: Instant,
}

#[derive(Default)]
pub(super) struct Cohorts(BTreeMap<String, Cohort>);

fn provider_read<T>(
    root: &Path,
    read: impl FnOnce(&Connection) -> Result<T, ProviderError>,
) -> Result<T, ProviderError> {
    let mut db = open_provider_db(root)?;
    let tx = db.transaction().map_err(|_| unavailable())?;
    read(&tx)
}

impl Cohort {
    fn capture(
        scope: &CohortScope,
        parameters: &SearchParameters,
        root: &Path,
        end: Instant,
    ) -> Result<Self, ProviderError> {
        if scope.id.is_empty()
            || scope.id.len() > 64
            || !scope.id.bytes().all(|byte| byte.is_ascii_hexdigit())
            || !parameters.valid()
            || parameters.purpose != SearchPurpose::Discovery
            || !metadata::uuid(&scope.caller)
        {
            return Err(cohort_unavailable());
        }
        let excluded = excluded_ids(&scope.excluded, root)?;
        if !excluded.contains(&scope.caller) {
            return Err(discovery_caller_unavailable());
        }
        // SQLite's empty filename opens a temporary database. Its bundled
        // Unix VFS opens it exclusively and unlinks it immediately. The
        // worker's attested TMPDIR is its private sandbox.
        let mut db = Connection::open("").map_err(|_| unavailable())?;
        db.execute_batch(
            "PRAGMA temp_store=FILE; PRAGMA cache_size=-512; PRAGMA journal_mode=MEMORY;
             CREATE TABLE cohort(id TEXT PRIMARY KEY NOT NULL, metadata BLOB NOT NULL,
                                 offered INTEGER NOT NULL DEFAULT 0) WITHOUT ROWID;",
        )
        .map_err(|_| unavailable())?;
        let started = Instant::now();
        let sink = db.transaction().map_err(|_| unavailable())?;
        let len = provider_read(root, |source| {
            if !has_one_thread(source, &scope.caller)? {
                return Err(discovery_caller_unavailable());
            }
            let (title, producer, path) = metadata_projection(&thread_columns(source)?);
            let mut select = source
                .prepare(&format!(
                    "SELECT id,{title},cwd,created_at,updated_at,archived,{producer},{path}
                 FROM threads WHERE id NOT IN (SELECT value FROM json_each(?1))
                 AND (?2 OR archived=0)"
                ))
                .map_err(|_| unavailable())?;
            let mut rows = select
                .query(params![
                    serde_json::to_string(&excluded).map_err(|_| unavailable())?,
                    parameters.include_archived
                ])
                .map_err(|_| unavailable())?;
            let mut insert = sink
                .prepare("INSERT INTO cohort(id,metadata) VALUES (?1,?2)")
                .map_err(|_| unavailable())?;
            let mut len = 0;
            while let Some(row) = rows.next().map_err(|_| unavailable())? {
                deadline(end)?;
                rollout_reader::record_metadata_row();
                let candidate = candidate_row(row, root)?;
                if candidate_eligible(&candidate, parameters) {
                    let id = candidate.metadata.native_session_id.clone();
                    let row = serde_json::to_vec(&FrozenRow::from(candidate))
                        .map_err(|_| unavailable())?;
                    insert
                        .execute(params![id, row])
                        .map_err(|_| unavailable())?;
                    len += 1;
                }
            }
            Ok(len)
        })?;
        sink.commit().map_err(|_| unavailable())?;
        rollout_reader::record_cohort(len, started.elapsed());
        Ok(Self {
            scope: scope.clone(),
            db,
            len,
            touched: Instant::now(),
        })
    }

    fn read(
        &mut self,
        read: &CohortRead,
        root: &Path,
        end: Instant,
    ) -> Result<CohortPage, ProviderError> {
        if read.scope != self.scope
            || !read.parameters.valid()
            || read.parameters.purpose != SearchPurpose::Discovery
            || read.remember.len() > 5
            || read.position.index > self.len
            || (read.position.index > 0) != read.position.after.is_some()
            || read
                .position
                .after
                .as_deref()
                .is_some_and(|id| !metadata::uuid(id))
        {
            return Err(cohort_unavailable());
        }
        self.touched = Instant::now();
        if !read.remember.is_empty() {
            let tx = self.db.transaction().map_err(|_| unavailable())?;
            for id in &read.remember {
                let changed = tx
                    .execute("UPDATE cohort SET offered=1 WHERE id=?1", [id])
                    .map_err(|_| unavailable())?;
                if changed != 1 {
                    return Err(cohort_unavailable());
                }
            }
            tx.commit().map_err(|_| unavailable())?;
        }

        let mut candidates = Vec::new();
        let mut skips = BTreeSet::new();
        let eligible = if let Some(id) = &read.refresh {
            if read.position.index != 0 || !metadata::uuid(id) {
                return Err(cohort_unavailable());
            }
            let bytes: Vec<u8> = self
                .db
                .query_row("SELECT metadata FROM cohort WHERE id=?1", [id], |row| {
                    row.get(0)
                })
                .map_err(|_| cohort_unavailable())?;
            let mut candidate = frozen_candidate(root, id.clone(), &bytes)?;
            // Refresh only the exact locator. Membership predicates still see
            // the initial metadata of the captured identity.
            let reference = ContextReference {
                decoder_progress: None,
                native_session_id: id.clone(),
                local_id: candidate.metadata.identity.local_id.clone(),
                record_index: 0,
                position: None,
                source_fingerprint: String::new(),
                source_bytes: 0,
                purpose: SearchPurpose::Discovery,
                binding_revision: discovery_binding(&identity(root, &read.scope.caller)?),
            };
            candidate.path = provider_read(root, |db| {
                context_candidate(db, root, &reference, &read.scope.excluded, true)
            })?
            .path;
            candidates.push(candidate);
            1
        } else {
            let mut select = self
                .db
                .prepare("SELECT id,metadata,offered FROM cohort WHERE id>?1 ORDER BY id LIMIT ?2")
                .map_err(|_| unavailable())?;
            let mut rows = select
                .query(params![
                    read.position.after.as_deref().unwrap_or(""),
                    SEARCH_BATCH as i64
                ])
                .map_err(|_| unavailable())?;
            while let Some(row) = rows.next().map_err(|_| unavailable())? {
                deadline(end)?;
                let id: String = row.get(0).map_err(|_| unavailable())?;
                let bytes: Vec<u8> = row.get(1).map_err(|_| unavailable())?;
                if read.expansion && row.get::<_, bool>(2).map_err(|_| unavailable())? {
                    skips.insert(id.clone());
                }
                candidates.push(frozen_candidate(root, id, &bytes)?);
            }
            self.len
        };
        let request = SearchReadRequest {
            parameters: read.parameters.clone(),
            cached: Vec::new(),
            excluded: Vec::new(),
            discovery_session: None,
            management_session: None,
            continuation: SearchContinuation {
                start: read.position.index,
                stream: read.position.stream.clone(),
                ..Default::default()
            },
        };
        let ids: Vec<_> = candidates
            .iter()
            .map(|candidate| candidate.metadata.native_session_id.clone())
            .collect();
        let page = search_batch(
            root,
            &request,
            end,
            Inventory {
                batch: candidates,
                eligible,
                fingerprint: String::new(),
                limited: false,
                corpus_fingerprint: String::new(),
                next_window: None,
            },
            Some(&skips),
        )?;
        Ok(CohortPage {
            sessions: page.sessions,
            unresolved: page.unresolved,
            coverage: page.coverage,
            next: page.next.map(|p| {
                let after = if p.start == read.position.index {
                    read.position.after.clone()
                } else {
                    ids.get(p.start - read.position.index - 1).cloned()
                };
                CohortPosition {
                    index: p.start,
                    after,
                    stream: p.stream,
                }
            }),
        })
    }
}

fn frozen_candidate(root: &Path, id: String, bytes: &[u8]) -> Result<Candidate, ProviderError> {
    serde_json::from_slice::<FrozenRow>(bytes)
        .map_err(|_| unavailable())?
        .candidate(root, id)
}

impl Cohorts {
    pub(super) fn handle(&mut self, request: &Request) -> Option<Result<Response, ProviderError>> {
        let end = Instant::now() + Duration::from_secs(8);
        let root = &request.source_root;
        self.0
            .retain(|_, cohort| cohort.touched.elapsed() < RETENTION);
        rollout_reader::work_reset();
        Some(match &request.operation {
            Operation::CaptureCohort { scope, parameters } => {
                if self.0.len() >= COHORTS || self.0.contains_key(&scope.id) {
                    return Some(Err(cohort_unavailable()));
                }
                Cohort::capture(scope, parameters, root, end).map(|cohort| {
                    self.0.insert(scope.id.clone(), cohort);
                    Response::CapturedCohort(rollout_reader::work().provider)
                })
            }
            Operation::ReadCohort(read) => self
                .0
                .get_mut(&read.scope.id)
                .ok_or_else(cohort_unavailable)
                .and_then(|cohort| cohort.read(read, root, end))
                .map(|mut page| {
                    page.coverage.work = rollout_reader::work();
                    Response::Cohort(page)
                }),
            Operation::ReleaseCohort(id) => {
                self.0.remove(id);
                Ok(Response::ReleasedCohort)
            }
            Operation::Verify {
                references,
                excluded,
            } => provider_read(root, |db| {
                let sessions = verify_evidence(db, root, references, excluded, end)?;
                Ok(Response::Verified(EvidenceVerification {
                    sessions,
                    work: rollout_reader::work().provider,
                }))
            }),
            Operation::Window {
                reference,
                options,
                excluded,
            } if reference.purpose == SearchPurpose::Discovery => provider_read(root, |db| {
                context_window(db, root, reference, options, excluded, end)
            })
            .map(|mut window| {
                window.work = rollout_reader::work();
                Response::Window(window)
            }),
            Operation::Context {
                reference,
                excluded,
            } if reference.purpose == SearchPurpose::Discovery => {
                provider_read(root, |db| context(db, root, reference, excluded, end))
                    .map(Response::Context)
            }
            Operation::OpenWindow {
                reference,
                options,
                excluded,
            } => provider_read(root, |db| {
                if reference.purpose != SearchPurpose::Discovery
                    || !matches!(
                        options.direction,
                        ContextDirection::Start | ContextDirection::Occurrences
                    )
                    || options.message_offset.is_some()
                {
                    return Err(unavailable());
                }
                let candidate = context_candidate(db, root, reference, excluded, true)?;
                let (file, _) =
                    open_transcript(&candidate, root).map_err(|_| session_source_unavailable())?;
                let metadata = file.metadata().map_err(|_| unavailable())?;
                let mut reference = reference.clone();
                reference.record_index = 1;
                reference.source_fingerprint = hash(stamp(&metadata).as_bytes());
                reference.source_bytes =
                    usize::try_from(metadata.len()).map_err(|_| unavailable())?;
                let mut window = context_window(db, root, &reference, options, excluded, end)
                    .map_err(|error| match error {
                        ProviderError::CapabilityUnavailable(_) => session_source_unavailable(),
                        error => error,
                    })?;
                window.work = rollout_reader::work();
                Ok(Response::OpenedWindow(reference, window))
            }),
            _ => return None,
        })
    }
}
