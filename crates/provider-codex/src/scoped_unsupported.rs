//! No scoped cleanup authority on platforms without the reviewed fence.
use crate::CodexAdapter;
use agentmender_domain::{CleanupReport, DiscoveryScope, LifecyclePlan};
use agentmender_persistence::{
    CleanupAttempt, DurableCleanupPort, DurableIntent, EffectError, NativeOutcome,
};
use agentmender_provider_api::{ProviderError, ScopedCleanupObservation, ScopedCleanupReadPort};

pub struct CodexScopedCleanupAdapter;
impl CodexScopedCleanupAdapter {
    pub fn new(_: CodexAdapter) -> Self {
        Self
    }
    pub fn new_expanded(_: CodexAdapter) -> Self {
        Self
    }
    pub fn with_protected_sessions(self, _: Vec<String>) -> Self {
        self
    }
}
impl agentmender_provider_api::SessionMetadataReadPort for CodexScopedCleanupAdapter {
    fn session_metadata(&self) -> Result<agentmender_domain::SessionInventory, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "exact metadata profile unsupported on this platform".into(),
        ))
    }
}
impl ScopedCleanupReadPort for CodexScopedCleanupAdapter {
    fn observe_cleanup(
        &self,
        _: &str,
        _: &DiscoveryScope,
    ) -> Result<ScopedCleanupObservation, ProviderError> {
        Err(ProviderError::CapabilityUnavailable(
            "scoped cleanup is unsupported on this platform".into(),
        ))
    }
}
impl DurableCleanupPort for CodexScopedCleanupAdapter {
    fn preflight(&self, _: &LifecyclePlan) -> Result<(), EffectError> {
        Err(EffectError::UnsupportedVersion)
    }
    fn dispatch(&self, intent: DurableIntent, _: &LifecyclePlan) -> CleanupAttempt {
        CleanupAttempt {
            observation: intent.into_observation(),
            outcome: NativeOutcome::NeverDispatched(EffectError::UnsupportedVersion),
            report: None,
        }
    }
    fn observe(&self, _: &LifecyclePlan) -> Result<CleanupReport, EffectError> {
        Err(EffectError::UnsupportedVersion)
    }
}

impl agentmender_persistence::DurableCompleteRemovalPort for CodexScopedCleanupAdapter {
    fn preflight(&self, _: &LifecyclePlan) -> Result<(), EffectError> {
        Err(EffectError::UnsupportedVersion)
    }
    fn dispatch(
        &self,
        intent: DurableIntent,
        _: &LifecyclePlan,
        _: &mut dyn agentmender_persistence::CompleteRemovalProgress,
    ) -> agentmender_persistence::CompleteRemovalAttempt {
        agentmender_persistence::CompleteRemovalAttempt {
            observation: intent.into_observation(),
            outcome: NativeOutcome::NeverDispatched(EffectError::UnsupportedVersion),
            report: None,
        }
    }
    fn observe(
        &self,
        _: &LifecyclePlan,
    ) -> Result<agentmender_domain::CompleteRemovalReport, EffectError> {
        Err(EffectError::UnsupportedVersion)
    }
}
