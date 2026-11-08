//! Fixed trusted launch composition. Human approval remains in approvals.rs.
use super::{Args, CliError, ErrorCode, SelectedProvider};
use agentmender_application::{
    ApplicationError,
    conversation::{ConversationRequest, ConversationService},
};
use agentmender_mcp_server::ConversationExecutor;
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
    Ok(Arc::new(move |request, control| {
        control.check()?;
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
fn unavailable() -> ApplicationError {
    ProviderError::CapabilityUnavailable(
        "protected conversation configuration or state unavailable".into(),
    )
    .into()
}

/// Current installation, fixed source root, metadata only. No owned state or
/// cleanup adapter is constructed by this launch mode.
pub(super) fn read_only_executor(args: &Args) -> Result<ConversationExecutor, CliError> {
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
    Ok(Arc::new(move |request, control| {
        let adapter = agentmender_provider_codex::CodexAdapter::new(
            agentmender_provider_api::ProviderLocation {
                state_root: root.clone(),
                executable: config.codex_executable.clone(),
            },
        )
        .with_evidence_helper(helper.clone(), home.clone());
        agentmender_application::conversation::read_only_dispatch(
            &adapter,
            &root,
            &config.protect_codex_session,
            request,
            control,
        )
    }))
}
