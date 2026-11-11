//! Bounded conversational handoff (ADR-0023). No human authorization or dispatch here.
use crate::{ApplicationError, ApprovalInputs, QueryControl, TargetSelector, cleanup_inputs};
use agentmender_domain::*;
use agentmender_persistence::{OwnedStateRoot, TransactionReader, TransactionStore};
use agentmender_provider_api::{ProviderError, ScopedCleanupReadPort, SessionMetadataReadPort};
use serde::Serialize;

pub enum ConversationRequest {
    Resolve(crate::session_resolution::ResolutionAction),
    EvidenceSearch {
        parameters: SearchParameters,
        continuation: Option<Box<SearchContinuation>>,
    },
    Context {
        reference: ContextReference,
        target: TargetSelector,
    },
    ContextWindow {
        reference: ContextReference,
        target: TargetSelector,
        options: ContextOptions,
    },
    Search {
        query: String,
        include_archived: bool,
    },
    Preview {
        target: TargetSelector,
    },
    Prepare {
        target: TargetSelector,
        expected_digest: String,
    },
    Status {
        transaction_id: Option<String>,
    },
    RequestExecution {
        transaction_id: String,
    },
}

pub struct SessionMatch {
    pub metadata: SessionMetadata,
    /// Preview is a fresh safety assessment, not a Ready or approved operation.
    pub can_request_preview: bool,
}
pub enum ConversationResponse {
    Resolution(Box<crate::session_resolution::ResolutionResponse>),
    EvidenceSearch(crate::session_search::SearchResult),
    Context(crate::session_search::ContextResult),
    ContextWindow(crate::session_search::ContextWindowResult),
    Search {
        sessions: Vec<SessionMatch>,
        completeness: Completeness,
        read_compatibility: Option<SessionReadCompatibility>,
    },
    Preview {
        session: SessionMetadata,
        inputs: Box<ApprovalInputs>,
    },
    ReadOnlyPreview {
        session: SessionMetadata,
        compatibility: SessionReadCompatibility,
    },
    Transactions(Vec<RemovalStatus>),
}

#[derive(Debug, Serialize)]
pub struct RemovalStatus {
    pub transaction_id: String,
    pub plan_digest: String,
    pub session_label: String,
    pub state: TransactionState,
    pub approval_status: String,
    pub recovery_available: bool,
    pub primary_verified: bool,
    pub other_sessions_preserved: bool,
    pub additional_artifacts_fully_cleaned: bool,
    pub graphical_confirmation: &'static str,
    pub verification_basis: &'static str,
    pub execution_channel: &'static str,
    pub retry_permitted: bool,
    pub logical_unlinked_bytes: Option<u64>,
    pub physical_reclaimed_bytes: Option<u64>,
}

fn unavailable() -> ApplicationError {
    ProviderError::CapabilityUnavailable(
        "conversational removal prerequisites unavailable; use the trusted external terminal"
            .into(),
    )
    .into()
}
fn storage_error(_: agentmender_persistence::StoreError) -> ApplicationError {
    unavailable()
}

pub fn removal_status(
    snapshot: &TransactionSnapshot,
    records: &[JournalRecord],
    recovery: bool,
) -> RemovalStatus {
    let report = records.iter().rev().find_map(|r| match r.event() {
        TransactionEvent::CleanupObserved { report } => Some(report),
        _ => None,
    });
    let verified = snapshot.state() == TransactionState::Succeeded
        && report.is_some_and(|r| {
            matches!(
                r.outcome,
                CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals
            ) && r.canonical_row_absent
                && r.retained_primary_unchanged
                && r.actions
                    .iter()
                    .all(|a| a.outcome == CleanupActionOutcome::Removed)
        });
    RemovalStatus {
        transaction_id: snapshot.id().to_string(),
        plan_digest: snapshot.binding().plan_digest().to_string(),
        session_label: snapshot
            .review_plan()
            .and_then(|p| p.content().native_target_id.as_ref())
            .map_or_else(|| "Unavailable".into(), |id| format!("Codex session {id}")),
        state: snapshot.state(),
        approval_status: format!("{:?}", snapshot.approval_status_at(now())),
        recovery_available: recovery,
        primary_verified: verified,
        other_sessions_preserved: verified && report.is_some_and(|r| r.retained_primary_unchanged),
        additional_artifacts_fully_cleaned: false,
        graphical_confirmation: "not_requested",
        verification_basis: if verified {
            "recorded_fenced_cutoff"
        } else {
            "not_verified"
        },
        execution_channel: "trusted_external_terminal",
        retry_permitted: false,
        logical_unlinked_bytes: report.map(|r| r.logical_unlinked_bytes),
        physical_reclaimed_bytes: None,
    }
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(u64::MAX, |t| t.as_secs())
}

/// Host supplies fixed ports, protected identities and an optional owned root. Tools
/// cannot supply paths, principals, grants, clocks or provider operations.
pub struct ConversationService<'a> {
    pub metadata: &'a dyn SessionMetadataReadPort,
    pub cleanup: &'a dyn ScopedCleanupReadPort,
    pub protected_sessions: &'a [String],
    pub provider_root: &'a std::path::Path,
}
impl ConversationService<'_> {
    fn resolve(
        &self,
        target: &TargetSelector,
    ) -> Result<(SessionMetadata, ApprovalInputs), ApplicationError> {
        if self.protected_sessions.is_empty() {
            return Err(ApplicationError::ManagementProtectionUnavailable);
        }
        let inventory = self.metadata.session_metadata()?;
        let mut matches = inventory
            .sessions
            .into_iter()
            .filter(|s| target(&s.identity));
        let session = matches.next().ok_or_else(unavailable)?;
        if self.protected_sessions.contains(&session.native_session_id) {
            return Err(ApplicationError::ProtectedManagementSession);
        }
        if matches.next().is_some()
            || session.identity.provider != ProviderKind::Codex
            || session.identity.class != GraphNodeKind::Session
            || session.identity.state_root != self.provider_root
            || !session.producer_version_matches
            || session.state == SessionState::Unknown
        {
            return Err(unavailable());
        }
        let inputs = cleanup_inputs(
            self.cleanup,
            &session.native_session_id,
            &DiscoveryScope::default(),
        )?;
        if inputs.plan.content().request.target != session.identity
            || self.protected_sessions.iter().any(|id| {
                !inputs.graph.roots().iter().any(|r| {
                    r.kind == RootKind::UserProtected
                        && r.node.provider == session.identity.provider
                        && r.node.state_root == session.identity.state_root
                        && r.node.local_id.as_str() == format!("codex:session:{id}")
                })
            })
        {
            return Err(ApplicationError::ManagementProtectionUnavailable);
        }
        Ok((session, inputs))
    }

    pub fn dispatch(
        &self,
        request: ConversationRequest,
        root: Option<OwnedStateRoot>,
        control: QueryControl,
    ) -> Result<ConversationResponse, ApplicationError> {
        control.check()?;
        match request {
            ConversationRequest::EvidenceSearch { parameters, .. }
                if parameters.purpose == SearchPurpose::Management =>
            {
                Err(ApplicationError::CallerIdentityRequired {
                    instance: "management_registration_requires_read_mode".into(),
                })
            }
            ConversationRequest::Resolve(_)
            | ConversationRequest::EvidenceSearch { .. }
            | ConversationRequest::Context { .. }
            | ConversationRequest::ContextWindow { .. } => Err(unavailable()),
            ConversationRequest::Search {
                query,
                include_archived,
            } => {
                if self.protected_sessions.is_empty() {
                    return Err(ApplicationError::ManagementProtectionUnavailable);
                }
                let inventory = self.metadata.session_metadata()?;
                search_inventory(
                    inventory,
                    &query,
                    include_archived,
                    self.protected_sessions,
                    self.provider_root,
                    true,
                    &control,
                )
            }
            ConversationRequest::Preview { target } => {
                let (session, inputs) = self.resolve(&target)?;
                control.check()?;
                Ok(ConversationResponse::Preview {
                    session,
                    inputs: Box::new(inputs),
                })
            }
            ConversationRequest::Prepare {
                target,
                expected_digest,
            } => {
                let (_, inputs) = self.resolve(&target)?;
                if inputs.plan.content().status != PlanStatus::Ready
                    || inputs.plan.digest().to_string() != expected_digest
                {
                    return Err(ApplicationError::RemovalPlanChanged);
                }
                control.check()?;
                let owned = root.ok_or_else(unavailable)?;
                let mut store = TransactionStore::open(owned.clone()).map_err(storage_error)?;
                // Identical retries recover the original attempt, including unknown or
                // terminal knowledge. No second approval or redispatch is inferred.
                let existing = store
                    .list()
                    .map_err(storage_error)?
                    .into_iter()
                    .find(|s| s.binding().plan_digest() == inputs.plan.digest());
                let snapshot = if let Some(snapshot) = existing {
                    snapshot
                } else {
                    let id = store
                        .create_transaction(&inputs.plan)
                        .map_err(storage_error)?;
                    store.prepare_recovery(&id, 0).map_err(storage_error)?;
                    store.prepare(&id, 1).map_err(storage_error)?;
                    store.inspect(&id).map_err(storage_error)?
                };
                drop(store);
                self.lookup(Some(owned), Some(snapshot.id().to_string()))
            }
            ConversationRequest::Status { transaction_id } => self.lookup(root, transaction_id),
            // Current same-user shell/PTY deployment cannot authenticate a human.
            // This is a handoff, and never opens a writer or consumes a grant.
            ConversationRequest::RequestExecution { transaction_id } => {
                self.lookup(root, Some(transaction_id))
            }
        }
    }

    fn lookup(
        &self,
        root: Option<OwnedStateRoot>,
        id: Option<String>,
    ) -> Result<ConversationResponse, ApplicationError> {
        let reader =
            TransactionReader::open(root.ok_or_else(unavailable)?).map_err(storage_error)?;
        let selected = id
            .as_deref()
            .map(str::parse::<TransactionId>)
            .transpose()
            .map_err(|_| unavailable())?;
        let mut statuses = Vec::new();
        for snapshot in reader.list().map_err(storage_error)? {
            if selected.as_ref().is_some_and(|id| snapshot.id() != id) {
                continue;
            }
            // This narrow surface does not leak native/Claude/other-root transactions.
            let Some(plan) = snapshot.review_plan() else {
                continue;
            };
            if plan.content().schema_version != 2
                || plan.content().cleanup_manifest.is_none()
                || plan.content().request.target.state_root != self.provider_root
                || !self.protected_sessions.iter().all(|id| {
                    plan.content().artifacts.iter().any(|a| {
                        a.explicitly_protected
                            && a.id.class == GraphNodeKind::Session
                            && a.id.local_id.as_str() == format!("codex:session:{id}")
                    })
                })
                || self.protected_sessions.is_empty()
            {
                continue;
            }
            let records = reader.records(snapshot.id()).map_err(storage_error)?;
            statuses.push(removal_status(
                &snapshot,
                &records,
                reader.recovery_available(snapshot.id()).unwrap_or(false),
            ));
        }
        if selected.is_some() && statuses.is_empty() {
            return Err(unavailable());
        }
        Ok(ConversationResponse::Transactions(statuses))
    }
}

fn search_inventory(
    inventory: SessionInventory,
    query: &str,
    include_archived: bool,
    protected_sessions: &[String],
    provider_root: &std::path::Path,
    primary: bool,
    control: &QueryControl,
) -> Result<ConversationResponse, ApplicationError> {
    if query.len() > 256 || query.chars().any(char::is_control) {
        return Err(unavailable());
    }
    let words: Vec<_> = query.split_whitespace().map(str::to_lowercase).collect();
    let mut sessions: Vec<_> = inventory
        .sessions
        .into_iter()
        // Trusted registrations belong to this provider/root, not a matching
        // title or UUID in another namespace. Exclude before matching/sorting
        // and before MCP can project a target or compute response budgets.
        .filter(|s| {
            !(s.identity.provider == ProviderKind::Codex
                && s.identity.class == GraphNodeKind::Session
                && s.identity.state_root == provider_root
                && protected_sessions.contains(&s.native_session_id))
        })
        .filter(|s| {
            let haystack = format!(
                "{} {} {}",
                s.title.as_deref().unwrap_or(""),
                s.project_label.as_deref().unwrap_or(""),
                s.native_session_id
            )
            .to_lowercase();
            (include_archived || s.state != SessionState::Archived)
                && words.iter().all(|w| haystack.contains(w))
        })
        .map(|s| {
            let can_request_preview = s.identity.provider == ProviderKind::Codex
                && s.identity.class == GraphNodeKind::Session
                && s.identity.state_root == provider_root
                && (!primary || s.producer_version_matches)
                && s.state != SessionState::Unknown;
            SessionMatch {
                metadata: s,
                can_request_preview,
            }
        })
        .collect();
    sessions.sort_by(|a, b| a.metadata.identity.cmp(&b.metadata.identity));

    control.check()?;
    Ok(ConversationResponse::Search {
        sessions,
        completeness: inventory.completeness,
        read_compatibility: inventory.read_compatibility,
    })
}

/// Useful discovery/assessment without any grant, durable preparation or cleanup
/// port. Caller registration is optional for reading and never inferred.
pub fn read_only_dispatch(
    metadata: &dyn SessionMetadataReadPort,
    provider_root: &std::path::Path,
    protected_sessions: &[String],
    request: ConversationRequest,
    control: QueryControl,
) -> Result<ConversationResponse, ApplicationError> {
    control.check()?;
    match request {
        ConversationRequest::Search {
            query,
            include_archived,
        } => search_inventory(
            metadata.session_metadata()?,
            &query,
            include_archived,
            protected_sessions,
            provider_root,
            false,
            &control,
        ),
        ConversationRequest::Preview { target } => {
            let inventory = metadata.session_metadata()?;
            let mut matches = inventory
                .sessions
                .into_iter()
                .filter(|s| target(&s.identity));
            let session = matches.next().ok_or_else(unavailable)?;
            if matches.next().is_some()
                || session.identity.provider != ProviderKind::Codex
                || session.identity.class != GraphNodeKind::Session
                || session.identity.state_root != provider_root
            {
                return Err(unavailable());
            }
            if protected_sessions.contains(&session.native_session_id) {
                return Err(ApplicationError::ProtectedManagementSession);
            }
            control.check()?;
            Ok(ConversationResponse::ReadOnlyPreview {
                session,
                compatibility: inventory.read_compatibility.ok_or_else(unavailable)?,
            })
        }
        _ => Err(ApplicationError::ReadOnlyRemovalUnavailable),
    }
}
