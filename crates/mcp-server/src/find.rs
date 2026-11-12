//! Public job DTOs and the private bridge into the existing typed resolver.
use crate::{ConversationExecutor, conversation, projection::Projector, protocol::schema};
use agentmender_application::{ApplicationError, QueryControl, conversation::ConversationRequest};
use agentmender_domain::ApparentCodexCaller;
use rmcp::model::{ErrorData, Tool, ToolAnnotations};
use schemars::JsonSchema;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FindInput {
    /// The original natural-language request.
    #[schemars(length(min = 1, max = 4096))]
    pub query: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct JobInput {
    #[schemars(length(min = 32, max = 32))]
    job_id: String,
}

#[derive(Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct ResultInput {
    #[schemars(length(min = 32, max = 32))]
    job_id: String,
    /// The next offset returned by this job, or zero for the first page.
    #[serde(default)]
    offset: usize,
    /// Wait for a terminal result for up to 60 seconds. Defaults to 25.
    #[serde(default)]
    #[schemars(range(max = 60))]
    wait_seconds: Option<u8>,
}

#[derive(Clone)]
pub enum FindRequest {
    Start(String),
    Status(String),
    Result {
        id: String,
        offset: usize,
        wait_seconds: u8,
        count_call: bool,
    },
    Cancel(String),
}

pub type FindExecutor =
    Arc<dyn Fn(FindRequest, ScopedResolver) -> Result<Value, &'static str> + Send + Sync>;

#[derive(Clone)]
pub struct ScopedResolver {
    pub(super) executor: ConversationExecutor,
    pub(super) projector: Arc<Projector>,
    pub(super) caller: ApparentCodexCaller,
}

impl ScopedResolver {
    pub fn caller(&self) -> &str {
        self.caller.native_id()
    }

    fn call_resolver(
        &self,
        request: ConversationRequest,
        control: QueryControl,
    ) -> Result<Value, &'static str> {
        let result =
            (self.executor)(request, control, Some(self.caller.clone())).map_err(|error| {
                match error {
                    ApplicationError::DiscoveryCallerUnavailable => "discovery_caller_unavailable",
                    ApplicationError::Resolution(reason) => reason.code(),
                    error => error.public_code(),
                }
            })?;
        conversation::project(result, &self.projector).map_err(|_| "resolver_projection_failed")
    }

    pub fn validate_caller(&self, control: QueryControl) -> Result<(), &'static str> {
        self.call_resolver(ConversationRequest::ValidateResolutionCaller, control)
            .map(|_| ())
    }

    pub fn validate_intent(args: &Value) -> Result<(), String> {
        conversation::resolution_request(args)
            .map(|_| ())
            .map_err(|err| {
                err.data
                    .as_ref()
                    .and_then(|data| data.get("error"))
                    .and_then(Value::as_str)
                    .unwrap_or("invalid_resolution_request")
                    .into()
            })
    }

    pub fn resolve(&self, args: &Value, control: QueryControl) -> Result<Value, String> {
        let action = conversation::resolution_request(args).map_err(|err| {
            err.data
                .as_ref()
                .and_then(|data| data.get("error"))
                .and_then(Value::as_str)
                .unwrap_or("invalid_resolution_request")
                .to_owned()
        })?;
        self.call_resolver(ConversationRequest::Resolve(action), control)
            .map_err(str::to_owned)
    }

    pub fn release(&self, run: String) {
        let _ = self.call_resolver(
            ConversationRequest::ReleaseResolution(run),
            QueryControl::new(None),
        );
    }
}

pub(super) fn request(name: &str, args: Value) -> Result<FindRequest, ErrorData> {
    let invalid = || ErrorData::invalid_params("invalid job parameters", None);
    let id = |id: String| {
        if id.len() == 32
            && id
                .bytes()
                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        {
            Ok(id)
        } else {
            Err(invalid())
        }
    };
    match name {
        "session_find" => {
            let input: FindInput = serde_json::from_value(args).map_err(|_| invalid())?;
            let query = input.query;
            if query.trim().is_empty()
                || query.len() > 4096
                || query
                    .chars()
                    .any(|ch| ch.is_control() && !matches!(ch, '\n' | '\r' | '\t'))
            {
                return Err(invalid());
            }
            Ok(FindRequest::Start(query))
        }
        "session_find_status" | "session_find_cancel" => {
            let input: JobInput = serde_json::from_value(args).map_err(|_| invalid())?;
            let id = id(input.job_id)?;
            Ok(if name == "session_find_status" {
                FindRequest::Status(id)
            } else {
                FindRequest::Cancel(id)
            })
        }
        "session_find_result" => {
            let input: ResultInput = serde_json::from_value(args).map_err(|_| invalid())?;
            let wait_seconds = input.wait_seconds.unwrap_or(25);
            if wait_seconds > 60 {
                return Err(invalid());
            }
            Ok(FindRequest::Result {
                id: id(input.job_id)?,
                offset: input.offset,
                wait_seconds,
                count_call: true,
            })
        }
        _ => Err(invalid()),
    }
}

pub(super) fn tools() -> Vec<Tool> {
    [
        ("session_find", "Start autonomous read-only search of past Codex sessions. Pass the original natural-language request in query. AgentMender runs Luna Planner, complete Rust lexical retrieval, Luna Evaluator and verified publication internally. Use session_find_status or session_find_result with the returned job_id.", schema::<FindInput>(), false),
        ("session_find_status", "Read progress, stage and terminal status of an autonomous search.", schema::<JobInput>(), true),
        ("session_find_result", "Read final verified confirmed and possible unconfirmed session rows with signed evidence. Wait up to 60 seconds for a terminal result. Follow results.next_offset until all rows are read.", schema::<ResultInput>(), true),
        ("session_find_cancel", "Cancel an autonomous search and release its temporary resources. This does not modify any Codex session.", schema::<JobInput>(), true),
    ].into_iter().map(|(name, description, input, idempotent)| {
        Tool::new(name, description, input).with_annotations(
            ToolAnnotations::new().read_only(true).destructive(false).idempotent(idempotent).open_world(false)
        )
    }).collect()
}
