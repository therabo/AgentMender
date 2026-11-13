//! Installation-owned read-only search jobs. No lifecycle ports or ambient launch configuration.
mod host;

use agentmender_application::QueryControl;
use agentmender_mcp_server::{FindExecutor, FindRequest, ScopedResolver};
use agentmender_provider_api::evidence::EvidenceSandbox;
use rusqlite::{Connection, params};
use serde::Serialize;
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{
        Arc, Condvar, Mutex, Weak,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const JOB_DEADLINE: Duration = Duration::from_secs(30 * 60);
const RETENTION: Duration = Duration::from_secs(10 * 60);
const ACTIVE_LIMIT: usize = 2;
const JOB_LIMIT: usize = 8;
const PAGE_BYTES: usize = 70 * 1024;
const PAGE_ROWS: usize = 8;

#[derive(Clone)]
pub(super) struct Launch {
    pub codex: Option<PathBuf>,
    pub root: PathBuf,
    pub home: PathBuf,
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Phase {
    Queued,
    Planning,
    Retrieving,
    Evaluating,
    Publishing,
    Completed,
    Interrupted,
    Cancelled,
}

impl Phase {
    fn terminal(self) -> bool {
        matches!(self, Self::Completed | Self::Interrupted | Self::Cancelled)
    }
}

struct ResultSpool {
    // The SQLite connection closes before the directory is removed.
    db: Connection,
    _directory: EvidenceSandbox,
}

impl ResultSpool {
    fn open(launch: &Launch) -> Result<Self, &'static str> {
        let unavailable = "result_spool_unavailable";
        let directory =
            EvidenceSandbox::new(&launch.root, &launch.home).map_err(|_| unavailable)?;
        let db = Connection::open(directory.path().join("verified-results.sqlite"))
            .map_err(|_| unavailable)?;
        db.execute_batch("PRAGMA journal_mode=MEMORY; PRAGMA cache_size=-512;
            PRAGMA max_page_count=32768;
            CREATE TABLE results(id TEXT PRIMARY KEY, kind TEXT NOT NULL, row TEXT NOT NULL) WITHOUT ROWID;")
            .map_err(|_| unavailable)?;
        Ok(Self {
            db,
            _directory: directory,
        })
    }
}

struct State {
    phase: Phase,
    stage_started: Instant,
    finished: Option<Instant>,
    reason: Option<String>,
    progress: Value,
    coverage: Value,
    selection: Value,
    search_intent: Option<Value>,
    elapsed: BTreeMap<&'static str, u64>,
    luna: BTreeMap<String, Value>,
    resolver_operations: usize,
    result_spool_bytes: u64,
    selection_omitted: usize,
    spool: Option<ResultSpool>,
}

impl State {
    fn new(now: Instant) -> Self {
        Self {
            phase: Phase::Queued,
            stage_started: now,
            finished: None,
            reason: None,
            progress: Value::Null,
            coverage: Value::Null,
            selection: Value::Null,
            search_intent: None,
            elapsed: BTreeMap::new(),
            luna: BTreeMap::new(),
            resolver_operations: 0,
            result_spool_bytes: 0,
            selection_omitted: 0,
            spool: None,
        }
    }

    fn phase(&mut self, phase: Phase) {
        if phase == self.phase {
            return;
        }
        let name = match self.phase {
            Phase::Planning => Some("planning"),
            Phase::Retrieving => Some("retrieving"),
            Phase::Evaluating => Some("evaluating"),
            Phase::Publishing => Some("publishing"),
            _ => None,
        };
        if let Some(name) = name {
            self.elapsed
                .insert(name, self.stage_started.elapsed().as_millis() as u64);
        }
        self.stage_started = Instant::now();
        self.phase = phase;
        if phase.terminal() {
            self.finished = Some(self.stage_started);
        }
    }

    fn insert(
        &mut self,
        rows: &Value,
        invalidated: &Value,
        winner: Option<&str>,
        launch: &Launch,
    ) -> Result<(), &'static str> {
        let unavailable = |_| "result_spool_unavailable";
        if self.spool.is_none() {
            self.spool = Some(ResultSpool::open(launch)?);
        }
        let db = &mut self.spool.as_mut().expect("opened spool").db;
        let tx = db.transaction().map_err(unavailable)?;
        for id in invalidated.as_array().ok_or("bridge_protocol")? {
            tx.execute(
                "DELETE FROM results WHERE id=?1",
                [id.as_str().ok_or("bridge_protocol")?],
            )
            .map_err(unavailable)?;
        }
        let mut omitted = 0;
        for kind in ["matches", "alternatives"] {
            for row in rows[kind].as_array().ok_or("bridge_protocol")? {
                let id = row["native_session_id"].as_str().ok_or("bridge_protocol")?;
                if kind == "matches" && winner.is_some_and(|winner| winner != id) {
                    omitted += 1;
                    continue;
                }
                let encoded = row.to_string();
                if encoded.len() > PAGE_BYTES {
                    return Err("result_row_too_large");
                }
                tx.execute(
                    "INSERT OR REPLACE INTO results(id,kind,row) VALUES (?1,?2,?3)",
                    params![id, kind, encoded],
                )
                .map_err(unavailable)?;
            }
        }
        tx.commit().map_err(unavailable)?;
        self.selection_omitted += omitted;
        let pages: i64 = db
            .pragma_query_value(None, "page_count", |row| row.get(0))
            .map_err(unavailable)?;
        let size: i64 = db
            .pragma_query_value(None, "page_size", |row| row.get(0))
            .map_err(unavailable)?;
        self.result_spool_bytes = self.result_spool_bytes.max((pages * size) as u64);
        Ok(())
    }

    fn finalize_counts(&mut self) {
        if !self.progress.is_object() || !self.coverage.is_object() {
            return;
        }
        let omitted = self.selection_omitted as u64;
        let published = self.progress["published"]
            .as_u64()
            .unwrap_or_default()
            .saturating_sub(omitted)
            * u64::from(self.spool.is_some());
        let staged = self.progress["candidates_staged"]
            .as_u64()
            .unwrap_or_default();
        let not_published = staged.saturating_sub(published);
        for data in [&mut self.progress, &mut self.coverage] {
            data["published"] = json!(published);
            data["not_published"] = json!(not_published);
        }
        let reasons = &mut self.coverage["not_published_reasons"];
        if omitted > 0 {
            reasons["single_best_selection"] = json!(omitted);
        }
        let accounted = reasons
            .as_object()
            .into_iter()
            .flat_map(|r| r.values())
            .filter_map(Value::as_u64)
            .sum::<u64>();
        if not_published > accounted {
            let already_interrupted = reasons["publication_interrupted"]
                .as_u64()
                .unwrap_or_default();
            reasons["publication_interrupted"] =
                json!(already_interrupted + not_published - accounted);
        }
    }

    fn results(&self, offset: usize) -> Result<Value, &'static str> {
        let mut confirmed = Vec::new();
        let mut possible = Vec::new();
        let mut next = None;
        if self.phase.terminal()
            && let Some(spool) = &self.spool
        {
            let unavailable = |_| "result_spool_unavailable";
            let mut statement = spool
                .db
                .prepare("SELECT kind,row FROM results ORDER BY id LIMIT ?1 OFFSET ?2")
                .map_err(unavailable)?;
            let offset = i64::try_from(offset).map_err(|_| "invalid_result_offset")?;
            let mut rows = statement
                .query(params![PAGE_ROWS as i64 + 1, offset])
                .map_err(unavailable)?;
            let mut size = 0;
            let mut count = 0;
            while let Some(row) = rows.next().map_err(unavailable)? {
                let kind: String = row.get(0).map_err(unavailable)?;
                let encoded: String = row.get(1).map_err(unavailable)?;
                if count == PAGE_ROWS || size + encoded.len() > PAGE_BYTES {
                    next = Some(offset as usize + count);
                    break;
                }
                size += encoded.len();
                count += 1;
                let value = serde_json::from_str::<Value>(&encoded)
                    .map_err(|_| "result_spool_unavailable")?;
                if kind == "matches" {
                    confirmed.push(value);
                } else {
                    possible.push(value);
                }
            }
        }
        Ok(json!({"confirmed": confirmed, "possible_unconfirmed": possible, "next_offset": next}))
    }
}

struct Job {
    owner: String,
    query: Mutex<Option<String>>,
    started: Instant,
    control: QueryControl,
    cancelled: AtomicBool,
    calls: AtomicUsize,
    state: Mutex<State>,
}

impl Job {
    fn snapshot(&self, id: &str, offset: Option<usize>) -> Result<Value, &'static str> {
        let state = self.state.lock().map_err(|_| "job_unavailable")?;
        let mut value = json!({
            "job_id": id,
            "state": state.phase,
            "reason": state.reason,
            "cancel_requested": self.cancelled.load(Ordering::Acquire),
            "authenticated_caller_binding": false,
            "progress": state.progress,
            "coverage": state.coverage,
            "selection": state.selection,
            "metrics": {
                "elapsed_ms": state.finished.unwrap_or_else(Instant::now)
                    .duration_since(self.started).as_millis(),
                "phase_ms": state.elapsed,
                "luna": state.luna,
                "resolver_operations": state.resolver_operations,
                "result_spool_peak_bytes": state.result_spool_bytes,
                "mcp_tool_calls": self.calls.load(Ordering::Relaxed),
            },
        });
        if let Some(offset) = offset
            && state.phase.terminal()
        {
            value["results"] = state.results(offset)?;
            if offset == 0
                && let Some(intent) = &state.search_intent
            {
                value["search_intent"] = intent.clone();
            }
        }
        Ok(value)
    }

    fn cancel(&self) {
        if let Ok(state) = self.state.lock()
            && !state.phase.terminal()
        {
            self.cancelled.store(true, Ordering::Release);
            self.control.cancel();
        }
    }
}

struct Running {
    job: Arc<Job>,
    worker: JoinHandle<()>,
}

#[derive(Default)]
struct Registry {
    jobs: Mutex<BTreeMap<String, Running>>,
    shutdown: Condvar,
    closing: AtomicBool,
}

struct FindHost {
    registry: Arc<Registry>,
    reaper: Option<JoinHandle<()>>,
    launch: Launch,
}

impl FindHost {
    fn reap(registry: Weak<Registry>) {
        let Some(registry) = registry.upgrade() else {
            return;
        };
        let Ok(mut jobs) = registry.jobs.lock() else {
            return;
        };
        while !registry.closing.load(Ordering::Acquire) {
            jobs.retain(|_, running| {
                if !running.worker.is_finished() {
                    return true;
                }
                let Ok(mut state) = running.job.state.lock() else {
                    return false;
                };
                if !state.phase.terminal() {
                    state.spool = None;
                    state.reason = Some("job_worker_exited".into());
                    state.phase(Phase::Interrupted);
                }
                state.finished.is_some_and(|t| t.elapsed() < RETENTION)
            });
            let Ok((next, _)) = registry.shutdown.wait_timeout(jobs, Duration::from_secs(5)) else {
                return;
            };
            jobs = next;
        }
    }

    fn call(&self, request: FindRequest, resolver: ScopedResolver) -> Result<Value, &'static str> {
        let mut jobs = self.registry.jobs.lock().map_err(|_| "job_unavailable")?;
        let (id, offset, cancel, count_call) = match request {
            FindRequest::Start(query) => {
                let duplicate = jobs.iter().find(|(_, running)| {
                    running.job.owner == resolver.caller()
                        && running
                            .job
                            .query
                            .lock()
                            .is_ok_and(|q| q.as_deref() == Some(query.as_str()))
                        && running.job.state.lock().is_ok_and(|s| !s.phase.terminal())
                });
                if let Some((id, running)) = duplicate {
                    running.job.calls.fetch_add(1, Ordering::Relaxed);
                    return running.job.snapshot(id, None);
                }
                if jobs.len() >= JOB_LIMIT
                    || jobs.values().filter(|r| !r.worker.is_finished()).count() >= ACTIVE_LIMIT
                {
                    return Err("job_capacity");
                }
                let mut bytes = [0; 16];
                getrandom::fill(&mut bytes).map_err(|_| "job_unavailable")?;
                let id: String = bytes.iter().map(|byte| format!("{byte:02x}")).collect();
                if jobs.contains_key(&id) {
                    return Err("job_unavailable");
                }
                let now = Instant::now();
                let job = Arc::new(Job {
                    owner: resolver.caller().to_owned(),
                    query: Mutex::new(Some(query.clone())),
                    started: now,
                    control: QueryControl::new(Some(now + JOB_DEADLINE)),
                    cancelled: AtomicBool::new(false),
                    calls: AtomicUsize::new(1),
                    state: Mutex::new(State::new(now)),
                });
                let reply = job.snapshot(&id, None)?;
                let worker_job = job.clone();
                let launch = self.launch.clone();
                let worker = thread::Builder::new()
                    .name("session-find".into())
                    .spawn(move || {
                        let outcome = host::run(&launch, &worker_job, resolver, query);
                        if let Ok(mut state) = worker_job.state.lock() {
                            let (phase, reason) = if worker_job.cancelled.load(Ordering::Acquire) {
                                state.spool = None;
                                (Phase::Cancelled, None)
                            } else if worker_job.control.check().is_err() {
                                state.spool = None;
                                (Phase::Interrupted, Some("job_deadline".into()))
                            } else {
                                match outcome {
                                    Ok((true, _)) => (Phase::Completed, None),
                                    Ok((false, reason)) => (Phase::Interrupted, reason),
                                    Err(code) => {
                                        state.spool = None;
                                        (Phase::Interrupted, Some(code))
                                    }
                                }
                            };
                            state.finalize_counts();
                            state.reason = reason;
                            state.phase(phase);
                        }
                        if let Ok(mut query) = worker_job.query.lock() {
                            *query = None;
                        }
                    })
                    .map_err(|_| "job_unavailable")?;
                jobs.insert(id, Running { job, worker });
                return Ok(reply);
            }
            FindRequest::Status(id) => (id, None, false, true),
            FindRequest::Result {
                id,
                offset,
                count_call,
                ..
            } => (id, Some(offset), false, count_call),
            FindRequest::Cancel(id) => (id, None, true, true),
        };
        let running = jobs
            .get(&id)
            .filter(|r| r.job.owner == resolver.caller())
            .ok_or("job_unavailable")?;
        if cancel {
            running.job.cancel();
        }
        if count_call {
            running.job.calls.fetch_add(1, Ordering::Relaxed);
        }
        running.job.snapshot(&id, offset)
    }
}

impl Drop for FindHost {
    fn drop(&mut self) {
        let jobs = {
            let Ok(mut jobs) = self.registry.jobs.lock() else {
                return;
            };
            self.registry.closing.store(true, Ordering::Release);
            for running in jobs.values() {
                running.job.cancel();
            }
            self.registry.shutdown.notify_one();
            std::mem::take(&mut *jobs)
        };
        for (_, running) in jobs {
            let _ = running.worker.join();
        }
        if let Some(reaper) = self.reaper.take() {
            let _ = reaper.join();
        }
    }
}

pub(super) fn executor(launch: Launch) -> std::io::Result<FindExecutor> {
    let registry = Arc::new(Registry::default());
    let weak = Arc::downgrade(&registry);
    let reaper = thread::Builder::new()
        .name("session-find-expiry".into())
        .spawn(move || FindHost::reap(weak))?;
    let host = FindHost {
        registry,
        reaper: Some(reaper),
        launch,
    };
    Ok(Arc::new(move |request, resolver| {
        host.call(request, resolver)
    }))
}
