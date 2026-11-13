//! Per-run UUID inventory, owned by the confined read-only provider worker.
use super::*;
use std::collections::VecDeque;

const RETENTION: Duration = Duration::from_secs(600);
const COHORTS: usize = 4;
const ROW_BYTES: usize = 20 * 1024;

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
    scan_replies: VecDeque<(CohortPosition, CohortPage)>,
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
                                 candidate BLOB, receipt BLOB) WITHOUT ROWID;
             CREATE INDEX staged_candidates ON cohort(id) WHERE candidate IS NOT NULL;
             CREATE INDEX pending_receipts ON cohort(id) WHERE receipt IS NOT NULL;",
        )
        .map_err(|_| unavailable())?;
        let started = Instant::now();
        let sink = db.transaction().map_err(|_| unavailable())?;
        let (len, examined) = provider_read(root, |source| {
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
            let mut examined = 0;
            while let Some(row) = rows.next().map_err(|_| unavailable())? {
                deadline(end)?;
                rollout_reader::record_metadata_row();
                examined += 1;
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
            Ok((len, examined))
        })?;
        sink.commit().map_err(|_| unavailable())?;
        rollout_reader::record_cohort(len, examined, started.elapsed());
        Ok(Self {
            scope: scope.clone(),
            db,
            len,
            touched: Instant::now(),
            scan_replies: VecDeque::new(),
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
        if read.refresh.is_none()
            && let Some((_, page)) = self
                .scan_replies
                .iter()
                .find(|(position, _)| position == &read.position)
        {
            return Ok(page.clone());
        }
        let mut candidates = Vec::new();
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
                .prepare("SELECT id,metadata FROM cohort WHERE id>?1 ORDER BY id LIMIT ?2")
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
            true,
        )?;
        let mut page = CohortPage {
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
        };
        if read.refresh.is_none() {
            let transaction = self.db.transaction().map_err(|_| unavailable())?;
            for hit in &page.sessions {
                let payload = encode_row(hit)?;
                let changed = transaction
                    .execute(
                        "UPDATE cohort SET candidate=?2 WHERE id=?1 AND candidate IS NULL",
                        params![hit.metadata.native_session_id, payload],
                    )
                    .map_err(|_| unavailable())?;
                if changed != 1 {
                    return Err(cohort_unavailable());
                }
            }
            deadline(end)?;
            transaction.commit().map_err(|_| unavailable())?;
        }
        page.coverage.work = rollout_reader::work();
        page.coverage.work.provider.cohort_spool_bytes = self.spool_bytes();
        if read.refresh.is_none() {
            if self.scan_replies.len() == COHORT_SCAN_PAGES {
                self.scan_replies.pop_front();
            }
            self.scan_replies
                .push_back((read.position.clone(), page.clone()));
        }
        Ok(page)
    }

    fn spool_bytes(&self) -> u64 {
        let pragma = |name| {
            self.db
                .pragma_query_value(None, name, |row| row.get::<_, i64>(0))
                .ok()
                .and_then(|value| u64::try_from(value).ok())
                .unwrap_or_default()
        };
        pragma("page_count").saturating_mul(pragma("page_size"))
    }

    fn queue(
        &mut self,
        scope: &CohortScope,
        queue: &CohortQueue,
        root: &Path,
        end: Instant,
    ) -> Result<CohortQueuePage, ProviderError> {
        if scope != &self.scope {
            return Err(cohort_unavailable());
        }
        deadline(end)?;
        self.touched = Instant::now();
        Ok(match queue {
            CohortQueue::Candidates { after } => CohortQueuePage::Candidates(self.rows(
                "SELECT candidate FROM cohort WHERE candidate IS NOT NULL AND id>?1 ORDER BY id LIMIT ?2",
                after.as_deref(),
            )?),
            CohortQueue::Receipts { after } => CohortQueuePage::Receipts(self.rows(
                "SELECT receipt FROM cohort WHERE receipt IS NOT NULL AND id>?1 ORDER BY id LIMIT ?2",
                after.as_deref(),
            )?),
            CohortQueue::Record { receipts } => {
                if receipts.len() > COHORT_BATCH {
                    return Err(cohort_unavailable());
                }
                let transaction = self.db.transaction().map_err(|_| unavailable())?;
                for receipt in receipts {
                    let id = &receipt.metadata.native_session_id;
                    let binding = discovery_binding(&identity(root, &scope.caller)?);
                    let valid_basis = match receipt.basis {
                        ReceiptBasis::VerifiedMetadata => receipt.assessment.is_none() && receipt.citations.is_empty(),
                        ReceiptBasis::SemanticEvidence => receipt.assessment.as_ref().is_some_and(|a| {
                            a.disposition == Disposition::VerifiedRelevant && !receipt.citations.is_empty()
                        }),
                        ReceiptBasis::PlausibleEvidence => receipt.assessment.as_ref().is_some_and(|a| {
                            a.disposition == Disposition::Insufficient && a.plausible_alternative && !receipt.citations.is_empty()
                        }),
                        ReceiptBasis::RetrievedUnverified => receipt.assessment.as_ref().is_some_and(|a| {
                            a.disposition == Disposition::Insufficient && !a.plausible_alternative
                        }),
                    };
                    if !metadata::uuid(id)
                        || receipt.metadata.identity != identity(root, id)?
                        || !valid_basis
                        || receipt.assessment.as_ref().is_some_and(|a| a.native_session_id != *id)
                        || receipt.citations.len() > 4
                        || receipt.citations.is_empty() != receipt.metadata_ref.is_some()
                        || receipt.metadata_ref.as_ref().is_some_and(|r| {
                            *r != MetadataSelection::new(&receipt.metadata, &binding)
                        })
                        || receipt.citations.iter().any(|reference| {
                            reference.native_session_id != *id
                                || reference.local_id != receipt.metadata.identity.local_id
                                || reference.purpose != SearchPurpose::Discovery
                                || reference.binding_revision != binding
                        })
                    {
                        return Err(cohort_unavailable());
                    }
                    let payload = encode_row(receipt)?;
                    let changed = transaction
                        .execute(
                            "UPDATE cohort SET receipt=?2 WHERE id=?1 AND candidate IS NOT NULL AND (receipt IS NULL OR receipt=?2)",
                            params![id, payload],
                        )
                        .map_err(|_| unavailable())?;
                    if changed != 1 {
                        return Err(cohort_unavailable());
                    }
                }
                deadline(end)?;
                transaction.commit().map_err(|_| unavailable())?;
                CohortQueuePage::Acknowledged { spool_bytes: self.spool_bytes() }
            }
            CohortQueue::KeepAlive => CohortQueuePage::Acknowledged { spool_bytes: self.spool_bytes() },
        })
    }

    fn rows<T: serde::de::DeserializeOwned>(
        &self,
        query: &str,
        after: Option<&str>,
    ) -> Result<Vec<T>, ProviderError> {
        if after.is_some_and(|id| !metadata::uuid(id)) {
            return Err(cohort_unavailable());
        }
        let mut statement = self.db.prepare(query).map_err(|_| unavailable())?;
        let mut rows = statement
            .query(params![after.unwrap_or(""), COHORT_BATCH as i64])
            .map_err(|_| unavailable())?;
        let mut page = Vec::new();
        while let Some(row) = rows.next().map_err(|_| unavailable())? {
            let payload: Vec<u8> = row.get(0).map_err(|_| unavailable())?;
            page.push(serde_json::from_slice(&payload).map_err(|_| unavailable())?);
        }
        Ok(page)
    }
}

fn encode_row(row: &impl Serialize) -> Result<Vec<u8>, ProviderError> {
    let payload = serde_json::to_vec(row).map_err(|_| unavailable())?;
    if payload.len() > ROW_BYTES {
        return Err(cohort_unavailable());
    }
    Ok(payload)
}

fn frozen_candidate(root: &Path, id: String, bytes: &[u8]) -> Result<Candidate, ProviderError> {
    serde_json::from_slice::<FrozenRow>(bytes)
        .map_err(|_| unavailable())?
        .candidate(root, id)
}

impl Cohorts {
    pub(super) fn expire(&mut self) {
        self.0
            .retain(|_, cohort| cohort.touched.elapsed() < RETENTION);
    }

    pub(super) fn until_expiry(&self) -> Duration {
        self.0
            .values()
            .map(|cohort| RETENTION.saturating_sub(cohort.touched.elapsed()))
            .min()
            .unwrap_or(RETENTION)
    }

    pub(super) fn handle(&mut self, request: &Request) -> Option<Result<Response, ProviderError>> {
        let end = Instant::now() + Duration::from_secs(8);
        let root = &request.source_root;
        self.expire();
        rollout_reader::work_reset();
        Some(match &request.operation {
            Operation::CaptureCohort { scope, parameters } => {
                if self.0.len() >= COHORTS || self.0.contains_key(&scope.id) {
                    return Some(Err(cohort_unavailable()));
                }
                Cohort::capture(scope, parameters, root, end).map(|cohort| {
                    let mut work = rollout_reader::work().provider;
                    work.cohort_spool_bytes = cohort.spool_bytes();
                    self.0.insert(scope.id.clone(), cohort);
                    Response::CapturedCohort(work)
                })
            }
            Operation::ReadCohort(read) => self
                .0
                .get_mut(&read.scope.id)
                .ok_or_else(cohort_unavailable)
                .and_then(|cohort| cohort.read(read, root, end))
                .map(Response::Cohort),
            Operation::CohortQueue { scope, queue } => self
                .0
                .get_mut(&scope.id)
                .ok_or_else(cohort_unavailable)
                .and_then(|cohort| cohort.queue(scope, queue, root, end))
                .map(Response::CohortQueue),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn idle_expiry_drops_only_expired_spools() {
        let make = |id: &str, touched| Cohort {
            scope: CohortScope {
                id: id.into(),
                caller: "00000000-0000-4000-8000-000000000001".into(),
                excluded: vec![],
            },
            db: Connection::open("").unwrap(),
            len: 0,
            touched,
            scan_replies: VecDeque::new(),
        };
        let now = Instant::now();
        let mut cohorts = Cohorts(BTreeMap::from([
            ("1".into(), make("1", now - RETENTION)),
            ("2".into(), make("2", now)),
        ]));
        assert_eq!(cohorts.until_expiry(), Duration::ZERO);
        cohorts.expire();
        assert_eq!(cohorts.0.keys().collect::<Vec<_>>(), vec!["2"]);
        assert!(cohorts.until_expiry() > Duration::ZERO);
    }
}
