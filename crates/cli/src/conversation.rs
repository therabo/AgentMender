//! Fixed launch composition for conversation tools and the native delete executor.
use super::{Args, CliError, ErrorCode, SelectedProvider};
use agentmender_application::{
    ApplicationError,
    conversation::{ConversationRequest, ConversationService},
};
use agentmender_mcp_server::{ConversationExecutor, NativeDeleteExecutor};
use agentmender_provider_api::ProviderError;
use std::{path::Path, sync::Arc};

pub(super) fn canonical_uuid(value: &str) -> Result<String, String> {
    if value.len() == 36
        && value.bytes().enumerate().all(|(i, b)| {
            if [8, 13, 18, 23].contains(&i) {
                b == b'-'
            } else {
                b.is_ascii_digit() || (b'a'..=b'f').contains(&b)
            }
        })
    {
        Ok(value.into())
    } else {
        Err("expected a canonical lowercase Codex UUID".into())
    }
}

pub(super) fn executor(
    args: &Args,
    state_root: Option<&Path>,
) -> Result<ConversationExecutor, CliError> {
    if !matches!(args.provider, Some(SelectedProvider::Codex))
        || args.fixture.is_some()
        || args.codex_desktop_user_data.is_some()
        || args.protect_codex_session.is_empty()
        || args.codex_root.is_none()
        || args.codex_executable.is_none()
        || args.project.is_some()
        || args.active_only
        || state_root.is_none()
    {
        return Err(CliError::new(
            ErrorCode::InvalidConfiguration,
            "session-removal requires Codex, explicit root/executable, trusted --protect-codex-session registration, and --state-root; fixtures, Desktop and filtered scopes are unsupported",
        ));
    }
    let config = args.clone();
    let state_root = state_root.expect("checked").to_path_buf();
    Ok(Arc::new(move |request, control, _| {
        control.check()?;
        if matches!(&request, ConversationRequest::EvidenceSearch { parameters, .. }
            if parameters.purpose == agentmender_domain::SearchPurpose::Management)
        {
            return Err(ApplicationError::CallerIdentityRequired {
                instance: "management_registration_requires_read_mode".into(),
            });
        }
        let adapter =
            super::lifecycle::scoped_adapter_version(&config, 2).map_err(|_| unavailable())?;
        // Only preparation may initialize AgentMender-owned state. Other calls
        // never create it, and no call grants/consumes approval or dispatches.
        let root = if matches!(
            request,
            ConversationRequest::Prepare { .. }
                | ConversationRequest::Status { .. }
                | ConversationRequest::RequestExecution { .. }
        ) {
            Some(
                super::approvals::root(
                    &config,
                    &state_root,
                    matches!(request, ConversationRequest::Prepare { .. }),
                )
                .map_err(|_| unavailable())?,
            )
        } else {
            None
        };
        ConversationService {
            metadata: &adapter,
            cleanup: &adapter,
            protected_sessions: &config.protect_codex_session,
            provider_root: config.codex_root.as_deref().expect("fixed root"),
        }
        .dispatch(request, root, control)
    }))
}
/// Composed separately from the reader; MCP invokes this only after confirmation.
pub(super) fn native_delete_executor(args: &Args) -> NativeDeleteExecutor {
    let root = args.codex_root.clone().expect("configured Codex root");
    let executable = args.codex_executable.clone();
    Arc::new(move |selection| {
        let (Some(executable), Ok(id)) = (
            &executable,
            uuid::Uuid::parse_str(&selection.session().native_session_id),
        ) else {
            return false;
        };
        let succeeded = agentmender_provider_api::process::run_codex_delete(
            executable,
            &root,
            id,
            std::time::Duration::from_secs(180),
        );
        #[cfg(target_os = "macos")]
        if succeeded {
            agentmender_provider_codex::schedule_desktop_catalog_reconciliation(&root, id);
        }
        succeeded
    })
}

fn unavailable() -> ApplicationError {
    ProviderError::CapabilityUnavailable(
        "protected conversation configuration or state unavailable".into(),
    )
    .into()
}

/// Current installation, fixed source root, bounded read evidence. No owned state or
/// cleanup adapter is constructed by this launch mode.
pub(super) fn read_only_executor(
    args: &Args,
    registration: Option<&Path>,
) -> Result<ConversationExecutor, CliError> {
    if !matches!(args.provider, Some(SelectedProvider::Codex))
        || args.fixture.is_some()
        || args.codex_desktop_user_data.is_some()
        || args.project.is_some()
        || args.active_only
    {
        return Err(CliError::new(
            ErrorCode::InvalidConfiguration,
            "session-search requires unfiltered Codex metadata; no fixtures or Desktop mutation configuration",
        ));
    }
    let config = args.clone();
    let root = config
        .codex_root
        .clone()
        .ok_or_else(|| CliError::new(ErrorCode::InvalidConfiguration, "Codex root unavailable"))?;
    let helper = std::env::current_exe()
        .map_err(|_| CliError::new(ErrorCode::InvalidConfiguration, "helper unavailable"))?;
    let home = std::env::var_os("HOME")
        .map(std::path::PathBuf::from)
        .filter(|h| h.is_absolute())
        .ok_or_else(|| CliError::new(ErrorCode::InvalidConfiguration, "absolute HOME required"))?;
    let registration = registration.map(Path::to_path_buf);
    let index = args
        .search_index_root
        .as_deref()
        .map(|path| super::search_index::open(args, path))
        .transpose()?;
    let mut nonce = [0u8; 16];
    getrandom::fill(&mut nonce)
        .map_err(|_| unavailable())
        .map_err(|_| {
            CliError::new(
                ErrorCode::InvalidConfiguration,
                "instance randomness unavailable",
            )
        })?;
    let instance: String = nonce.iter().map(|b| format!("{b:02x}")).collect();
    let adapter =
        agentmender_provider_codex::CodexAdapter::new(agentmender_provider_api::ProviderLocation {
            state_root: root.clone(),
            executable: config.codex_executable.clone(),
        })
        .with_evidence_helper(helper.clone(), home.clone());
    let resolution = std::sync::Mutex::new(
        agentmender_application::session_resolution::ResolutionStore::default(),
    );
    Ok(Arc::new(move |request, control, apparent_caller| {
        let management = super::management::read(registration.as_deref(), &instance, &root);
        let registrations: Vec<_> = config
            .protect_codex_session
            .iter()
            .map(|id| agentmender_domain::ArtifactId {
                provider: agentmender_domain::ProviderKind::Codex,
                state_root: root.clone(),
                class: agentmender_domain::GraphNodeKind::Session,
                local_id: agentmender_domain::EntityId::new(format!("codex:session:{id}"))
                    .expect("validated launch UUID"),
            })
            .collect();
        let indexed = index.as_ref().map(|index| {
            agentmender_application::indexed_search::IndexedSearchReader {
                reader: &adapter,
                index: index.clone(),
                root: &root,
            }
        });
        let reader: &dyn agentmender_provider_api::SessionSearchReadPort = indexed.as_ref().map_or(
            &adapter as &dyn agentmender_provider_api::SessionSearchReadPort,
            |i| i,
        );
        let service = agentmender_application::session_search::SessionSearchService {
            reader,
            provider_root: &root,
            provider: agentmender_domain::ProviderKind::Codex,
            management: management.as_ref(),
            apparent_caller: apparent_caller.as_ref(),
            registered: &registrations,
            instance: &instance,
        };
        match request {
            ConversationRequest::NativeDeleteSelection(reference) => {
                return agentmender_application::conversation::native_delete_selection(
                    &service, reference, &control,
                );
            }
            ConversationRequest::ValidateResolutionCaller => {
                use agentmender_domain::{
                    QueryExpr, SearchParameters, SearchPurpose, StructuredQuery,
                };
                let id = apparent_caller
                    .as_ref()
                    .ok_or(ApplicationError::DiscoveryCallerUnavailable)?
                    .native_id();
                service.search(
                    SearchParameters {
                        structured: Some(StructuredQuery {
                            filter: QueryExpr::NativeId {
                                values: vec![id.into()],
                            },
                            mode: Default::default(),
                            sort: Default::default(),
                            coverage: Default::default(),
                        }),
                        query: String::new(),
                        include_archived: true,
                        include_content: false,
                        project_label: None,
                        purpose: SearchPurpose::Discovery,
                        page_size: 1,
                        excerpts: false,
                    },
                    None,
                    control,
                )?;
                return Ok(agentmender_application::conversation::ConversationResponse::InternalAcknowledged);
            }
            ConversationRequest::ReleaseResolution(id) => {
                resolution
                    .lock()
                    .map_err(|_| unavailable())?
                    .release(&service, &id);
                return Ok(agentmender_application::conversation::ConversationResponse::InternalAcknowledged);
            }
            ConversationRequest::Resolve(action) => {
                let id = if action.is_start() {
                    let mut random = [0u8; 16];
                    getrandom::fill(&mut random).map_err(|_| unavailable())?;
                    Some(random.iter().map(|b| format!("{b:02x}")).collect())
                } else {
                    None
                };
                return resolution
                    .lock()
                    .map_err(|_| unavailable())?
                    .execute(&service, action, id, &control)
                    .map(|response| {
                        agentmender_application::conversation::ConversationResponse::Resolution(
                            Box::new(response),
                        )
                    });
            }
            ConversationRequest::EvidenceSearch {
                parameters,
                continuation,
            } => {
                return service
                    .search(parameters, continuation.map(|c| *c), control)
                    .map(
                        agentmender_application::conversation::ConversationResponse::EvidenceSearch,
                    );
            }
            ConversationRequest::ContextById {
                native_session_id,
                options,
            } => {
                return service
                    .open_session_window(&native_session_id, options, control)
                    .map(
                        agentmender_application::conversation::ConversationResponse::ContextWindow,
                    );
            }
            ConversationRequest::ContextWindow {
                reference,
                target,
                options,
            } => {
                return service
                    .context_window(reference, target, options, control)
                    .map(
                        agentmender_application::conversation::ConversationResponse::ContextWindow,
                    );
            }
            ConversationRequest::Context { reference, target } => {
                return service
                    .context(reference, target, control)
                    .map(agentmender_application::conversation::ConversationResponse::Context);
            }
            _ => {}
        }
        let mut protected = config.protect_codex_session.clone();
        if let Some(management) = management {
            protected.push(management.native_session_id);
        }
        agentmender_application::conversation::read_only_dispatch(
            &adapter, &root, &protected, request, control,
        )
    }))
}
