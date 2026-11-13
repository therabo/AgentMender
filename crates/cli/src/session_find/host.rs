//! Private bounded stdio bridge. Rust owns and reaps each direct child.
use super::{Job, Launch, Phase};
use agentmender_mcp_server::ScopedResolver;
use agentmender_provider_api::{evidence::EvidenceSandbox, process};
use serde_json::{Value, json};
use std::{
    io::{BufRead, BufReader, Read, Write},
    process::Child,
    sync::mpsc::{Receiver, RecvTimeoutError, SyncSender, sync_channel},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const FRAME: u64 = 1024 * 1024;
const TICK: Duration = Duration::from_millis(50);
const TURN_DEADLINE: Duration = Duration::from_secs(120);

struct ManagedChild {
    child: Child,
    output: Option<Receiver<Result<Value, &'static str>>>,
    input: Option<SyncSender<Vec<u8>>>,
    ack: Receiver<bool>,
    threads: Vec<JoinHandle<()>>,
}

impl ManagedChild {
    fn new(child: Child) -> Result<Self, &'static str> {
        let (input, writes) = sync_channel::<Vec<u8>>(1);
        let (wrote, ack) = sync_channel(1);
        let (send, output) = sync_channel(16);
        let mut managed = Self {
            child,
            output: Some(output),
            input: Some(input),
            ack,
            threads: vec![],
        };
        let stdout = managed
            .child
            .stdout
            .take()
            .ok_or("child_pipe_unavailable")?;
        let mut stdin = managed.child.stdin.take().ok_or("child_pipe_unavailable")?;
        managed.threads.push(
            thread::Builder::new()
                .name("find-child-reader".into())
                .spawn(move || {
                    let mut stream = BufReader::new(stdout);
                    loop {
                        let mut bytes = Vec::new();
                        let frame = match stream
                            .by_ref()
                            .take(FRAME + 1)
                            .read_until(b'\n', &mut bytes)
                        {
                            Ok(0) => break,
                            Ok(n) if n as u64 <= FRAME && bytes.ends_with(b"\n") => {
                                serde_json::from_slice(&bytes).map_err(|_| "child_invalid_json")
                            }
                            _ => Err("child_invalid_frame"),
                        };
                        let bad = frame.is_err();
                        if send.send(frame).is_err() || bad {
                            break;
                        }
                    }
                })
                .map_err(|_| "child_pipe_unavailable")?,
        );
        managed.threads.push(
            thread::Builder::new()
                .name("find-child-writer".into())
                .spawn(move || {
                    for bytes in writes {
                        let success = stdin.write_all(&bytes).is_ok();
                        if wrote.send(success).is_err() || !success {
                            break;
                        }
                    }
                })
                .map_err(|_| "child_pipe_unavailable")?,
        );
        Ok(managed)
    }

    fn receive(&self, job: &Job, timeout: Duration) -> Result<Value, &'static str> {
        wait(self.output.as_ref().ok_or("rpc_closed")?, job, timeout)?
    }

    fn send(&self, job: &Job, value: &Value) -> Result<(), &'static str> {
        let mut data = serde_json::to_vec(value).map_err(|_| "child_invalid_json")?;
        data.push(b'\n');
        if data.len() as u64 > FRAME {
            return Err("child_invalid_frame");
        }
        self.input
            .as_ref()
            .ok_or("rpc_closed")?
            .try_send(data)
            .map_err(|_| "rpc_closed")?;
        if wait(&self.ack, job, Duration::from_secs(5))? {
            Ok(())
        } else {
            Err("rpc_closed")
        }
    }

    fn close(&mut self, job: &Job) -> Result<(), &'static str> {
        self.input.take();
        self.output.take();
        let deadline = Instant::now() + Duration::from_secs(5);
        while Instant::now() < deadline && job.control.check().is_ok() {
            if let Some(exit) = self.child.try_wait().map_err(|_| "child_exit_unknown")? {
                return if exit.success() {
                    Ok(())
                } else {
                    Err("child_exit_failure")
                };
            }
            thread::sleep(TICK);
        }
        Err("child_close_deadline")
    }
}

impl Drop for ManagedChild {
    fn drop(&mut self) {
        self.output.take();
        self.input.take();
        if self.child.try_wait().ok().flatten().is_none() {
            let _ = self.child.kill();
        }
        let _ = self.child.wait();
        for worker in self.threads.drain(..) {
            let _ = worker.join();
        }
    }
}

fn wait<T>(receiver: &Receiver<T>, job: &Job, timeout: Duration) -> Result<T, &'static str> {
    let until = Instant::now() + timeout;
    loop {
        job.control.check().map_err(|_| "job_stopping")?;
        match receiver.recv_timeout(TICK.min(until.saturating_duration_since(Instant::now()))) {
            Ok(value) => return Ok(value),
            Err(RecvTimeoutError::Disconnected) => return Err("rpc_closed"),
            Err(RecvTimeoutError::Timeout) if Instant::now() >= until => {
                return Err("rpc_deadline");
            }
            Err(RecvTimeoutError::Timeout) => {}
        }
    }
}

const ASSETS: [(&str, &[u8]); 5] = [
    (
        "scripts/resolve_with_luna.py",
        include_bytes!("../../../../scripts/resolve_with_luna.py"),
    ),
    (
        "scripts/check_semantic_assessment.py",
        include_bytes!("../../../../scripts/check_semantic_assessment.py"),
    ),
    (
        "docs/search-intent-v1.schema.json",
        include_bytes!("../../../../docs/search-intent-v1.schema.json"),
    ),
    (
        ".codex/agents/session-planner.toml",
        include_bytes!("../../../../.codex/agents/session-planner.toml"),
    ),
    (
        ".codex/agents/session-evaluator.toml",
        include_bytes!("../../../../.codex/agents/session-evaluator.toml"),
    ),
];

fn install_assets(sandbox: &EvidenceSandbox) -> Result<(), &'static str> {
    sandbox.validate().map_err(|_| "luna_sandbox_unavailable")?;
    for (name, bytes) in ASSETS {
        let path = sandbox.path().join(name);
        std::fs::create_dir_all(path.parent().expect("embedded relative path"))
            .map_err(|_| "luna_assets_unavailable")?;
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(path)
            .and_then(|mut file| file.write_all(bytes))
            .map_err(|_| "luna_assets_unavailable")?;
    }
    Ok(())
}

pub(super) fn run(
    launch: &Launch,
    job: &Job,
    resolver: ScopedResolver,
    query: String,
) -> Result<(bool, Option<String>), String> {
    if !cfg!(target_os = "macos") {
        return Err("luna_isolation_unsupported".into());
    }
    resolver.validate_caller(job.control.clone())?;
    let executable = launch
        .codex
        .as_deref()
        .ok_or("codex_executable_unavailable")?;
    process::probe_version(agentmender_domain::ProviderKind::Codex, executable)
        .map_err(|_| "codex_executable_unverified")?;
    let sandbox =
        EvidenceSandbox::new(&launch.root, &launch.home).map_err(|_| "luna_sandbox_unavailable")?;
    install_assets(&sandbox)?;
    let mut python = ManagedChild::new(
        process::spawn_luna_coordinator(&sandbox).map_err(|_| "coordinator_launch_failed")?,
    )?;
    let mut bridge = Bridge {
        job,
        launch,
        sandbox: &sandbox,
        executable,
        resolver,
        query: Some(query),
        child: None,
        spawned: 0,
        validated_intent: None,
        run_id: None,
        phase: Phase::Planning,
    };
    job.state
        .lock()
        .map_err(|_| "job_unavailable")?
        .phase(Phase::Planning);
    let terminal = loop {
        let packet = python.receive(job, TURN_DEADLINE + Duration::from_secs(30))?;
        let operation = packet["operation"].as_str().ok_or("bridge_protocol")?;
        if matches!(operation, "done" | "failed") {
            python.send(job, &json!({"value": null}))?;
            break if operation == "done" {
                let result = &packet["result"];
                if bridge.child.is_some() || !bridge.phase.terminal() {
                    Err("bridge_terminal_out_of_order".into())
                } else {
                    let complete = bridge.phase == Phase::Completed;
                    job.state.lock().map_err(|_| "job_unavailable")?.selection = result.clone();
                    let stop =
                        job.state.lock().map_err(|_| "job_unavailable")?.progress["stop_reason"]
                            .as_str()
                            .map(str::to_owned);
                    Ok((complete, stop))
                }
            } else {
                Err(packet["reason"]
                    .as_str()
                    .unwrap_or("host_failed")
                    .to_owned())
            };
        }
        let reply = match bridge.handle_packet(operation, &packet) {
            Ok(value) => json!({"value": value}),
            Err(error) => json!({"error": error}),
        };
        python.send(job, &reply)?;
    };
    python.close(job)?;
    terminal
}

struct Bridge<'a> {
    job: &'a Job,
    launch: &'a Launch,
    sandbox: &'a EvidenceSandbox,
    executable: &'a std::path::Path,
    resolver: ScopedResolver,
    query: Option<String>,
    child: Option<ManagedChild>,
    spawned: usize,
    validated_intent: Option<Value>,
    run_id: Option<String>,
    phase: Phase,
}

impl Drop for Bridge<'_> {
    fn drop(&mut self) {
        self.child.take();
        if let Some(run) = self.run_id.take() {
            self.resolver.release(run);
        }
    }
}

impl Bridge<'_> {
    fn handle_packet(&mut self, operation: &str, packet: &Value) -> Result<Value, String> {
        let job = self.job;
        match operation {
            "query" => return Ok(json!(self.query.take().ok_or("bridge_protocol")?)),
            "spawn" => {
                let expected = match (self.spawned, self.phase) {
                    (0, Phase::Planning) => "planner",
                    (1, Phase::Evaluating) => "evaluator",
                    _ => return Err("unexpected_luna_spawn".into()),
                };
                if self.child.is_some() || packet["role"] != expected {
                    return Err("unexpected_luna_spawn".into());
                }
                self.spawned += 1;
                let role = if expected == "planner" {
                    process::LunaRole::Planner
                } else {
                    process::LunaRole::Evaluator
                };
                let child = process::spawn_codex_luna(self.executable, self.sandbox, role)
                    .map_err(|error| match error {
                        agentmender_provider_api::ProviderError::InvalidScope(_) => {
                            "luna_credentials_unavailable"
                        }
                        agentmender_provider_api::ProviderError::DiscoveryFailed { .. }
                        | agentmender_provider_api::ProviderError::PermissionDenied(_) => {
                            "luna_credentials_or_sandbox_unavailable"
                        }
                        _ => "app_server_launch_failed",
                    })?;
                self.child = Some(ManagedChild::new(child)?);
                return Ok(json!(role.private_home(self.sandbox)));
            }
            "close" => {
                let mut child = self.child.take().ok_or("rpc_closed")?;
                child.close(job)?;
            }
            "send" => {
                let message = &packet["message"];
                if !matches!(
                    message["method"].as_str(),
                    Some("initialize" | "initialized" | "thread/start" | "turn/start")
                ) && !(message.get("id").is_some() && message["error"]["code"] == -32601)
                {
                    return Err("app_server_method_refused".into());
                }
                self.child
                    .as_ref()
                    .ok_or("rpc_closed")?
                    .send(job, message)?;
            }
            "receive" => {
                let timeout =
                    Duration::from_millis(packet["wait_ms"].as_u64().ok_or("bridge_protocol")?);
                return Ok(self
                    .child
                    .as_ref()
                    .ok_or("rpc_closed")?
                    .receive(job, timeout.min(TURN_DEADLINE))?);
            }
            "validate" => {
                if self.phase != Phase::Planning || self.child.is_none() || self.spawned != 1 {
                    return Err("intent_validation_out_of_order".into());
                }
                let args = json!({"action": "start", "search_intent": packet["intent"]});
                ScopedResolver::validate_intent(&args)?;
                self.validated_intent = Some(args);
            }
            "resolve" => return self.resolve(&packet["arguments"]),
            "trace" => {
                let event = &packet["event"];
                if matches!(
                    event["event"].as_str(),
                    Some("luna_ready" | "luna_turn_completed" | "luna_closed" | "luna_failure")
                ) && let Some(role @ ("planner" | "evaluator")) = event["role"].as_str()
                {
                    let mut state = job.state.lock().map_err(|_| "job_unavailable")?;
                    let snapshot = state
                        .luna
                        .entry(role.to_owned())
                        .or_insert_with(|| json!({"turns": 0}));
                    match event["event"].as_str() {
                        Some("luna_ready") => {
                            snapshot["thread"] = event["thread"].clone();
                            snapshot["model"] = event["model"].clone();
                            snapshot["effort"] = event["effort"].clone();
                        }
                        Some("luna_turn_completed") => {
                            snapshot["turns"] = json!(snapshot["turns"].as_u64().unwrap_or(0) + 1);
                            snapshot["last_completed_turn"] = event["turn"].clone();
                            snapshot["token_usage"] = event["token_usage"].clone();
                        }
                        Some("luna_closed") => snapshot["closed"] = json!(true),
                        _ => snapshot["failure"] = event.clone(),
                    }
                }
            }
            "results" => {
                if self.child.is_some()
                    || !matches!(
                        self.phase,
                        Phase::Publishing | Phase::Completed | Phase::Interrupted
                    )
                {
                    return Err("publication_out_of_order".into());
                }
                job.state.lock().map_err(|_| "job_unavailable")?.insert(
                    &packet["rows"],
                    &packet["invalidated"],
                    packet["winner"].as_str(),
                    self.launch,
                )?;
            }
            _ => return Err("bridge_protocol".into()),
        }
        Ok(Value::Null)
    }

    fn resolve(&mut self, args: &Value) -> Result<Value, String> {
        if self.run_id.is_none() {
            if self.child.is_some()
                || self.validated_intent.as_ref() != Some(args)
                || self.spawned != 1
            {
                return Err("retrieval_out_of_order".into());
            }
        } else if args["action"] != "advance" || args["run"].as_str() != self.run_id.as_deref() {
            return Err("resolver_run_mismatch".into());
        }
        let publish = args["publish"] == true;
        if publish
            && (self.child.is_some()
                || !matches!(self.phase, Phase::Evaluating | Phase::Publishing))
        {
            return Err("publication_out_of_order".into());
        }
        if ["assessments", "context_requests"].iter().any(|field| {
            args[*field]
                .as_array()
                .is_some_and(|items| !items.is_empty())
        }) && (self.spawned != 2 || self.child.is_none())
        {
            return Err("evaluation_out_of_order".into());
        }
        let mut state = self.job.state.lock().map_err(|_| "job_unavailable")?;
        state.resolver_operations += 1;
        if self.run_id.is_none() {
            state.phase(Phase::Retrieving);
            state.search_intent = Some(args["search_intent"].clone());
        } else if publish {
            state.phase(Phase::Publishing);
        }
        drop(state);
        let value = self.resolver.resolve(args, self.job.control.clone())?;
        if self.run_id.is_none() {
            self.run_id = Some(value["run"].as_str().ok_or("bridge_protocol")?.to_owned());
        }
        self.phase = match value["phase"].as_str() {
            Some("retrieving") => Phase::Retrieving,
            Some("evaluating") => Phase::Evaluating,
            Some("publishing") => Phase::Publishing,
            Some("completed") => Phase::Completed,
            Some("interrupted") => Phase::Interrupted,
            _ => return Err("bridge_protocol".into()),
        };
        let mut state = self.job.state.lock().map_err(|_| "job_unavailable")?;
        if !self.phase.terminal() {
            state.phase(self.phase);
        }
        state.progress = value["progress"].clone();
        state.coverage = value["progressive_coverage"].clone();
        Ok(value)
    }
}
