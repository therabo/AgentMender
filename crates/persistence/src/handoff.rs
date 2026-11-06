//! Narrow typed lifecycle boundary. Implementations must retain no execution gate.
use crate::{DurableIntent, DurableObservation};
use agentmender_domain::{ArtifactGraph, CleanupReport, IntegrityReport, LifecyclePlan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectError {
    RequiresProviderClosed,
    WriterInventoryUnknown,
    UnsupportedVersion,
    UnsupportedOperation,
    ProfileMismatch,
    NativeIdentityMismatch,
    Transport,
    Timeout,
    ProcessTerminated,
    ProtocolMismatch,
    ProviderRejected,
    Unavailable,
    InvalidEvidence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeOutcome {
    Accepted,
    /// No writable provider process was started.
    NeverDispatched(EffectError),
    /// A provider process may have started or a request may have been sent.
    Unknown(EffectError),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeObservation {
    Active,
    Archived,
    Unknown,
}
#[derive(Debug)]
pub struct EffectAttempt {
    pub observation: DurableObservation,
    pub outcome: NativeOutcome,
}
#[derive(Debug)]
pub struct EffectObservation {
    pub native: NativeObservation,
    pub graph: ArtifactGraph,
    pub integrity: IntegrityReport,
}
/// Implementations must validate exact plan identity and the native profile. A
/// missing, conflicting, or incoherent observation is always Unknown.
pub trait DurableEffectPort {
    fn preflight(&self, plan: &LifecyclePlan) -> Result<(), EffectError>;
    fn dispatch(&self, intent: DurableIntent, plan: &LifecyclePlan) -> EffectAttempt;
    fn observe(&self, plan: &LifecyclePlan) -> Result<EffectObservation, EffectError>;
}

/// Cleanup is a separate, local operation. A successful report must describe
/// observations captured while the provider's exclusive mutation fence held.
#[derive(Debug)]
pub struct CleanupAttempt {
    pub observation: DurableObservation,
    pub outcome: NativeOutcome,
    pub report: Option<CleanupReport>,
}

pub trait DurableCleanupPort {
    fn preflight(&self, plan: &LifecyclePlan) -> Result<(), EffectError>;
    fn dispatch(&self, intent: DurableIntent, plan: &LifecyclePlan) -> CleanupAttempt;
    /// Read-only reconciliation of the frozen manifest; never dispatches.
    fn observe(&self, plan: &LifecyclePlan) -> Result<CleanupReport, EffectError>;
}

/// Observation-only progress in the existing durable journal. Failure stops
/// subsequent effects; this interface cannot issue another execution intent.
pub trait CompleteRemovalProgress {
    fn record(
        &mut self,
        receipt: &DurableObservation,
        progress: agentmender_domain::CompleteRemovalActionProgress,
    ) -> Result<(), EffectError>;
}

#[derive(Debug)]
pub struct CompleteRemovalAttempt {
    pub observation: DurableObservation,
    pub outcome: NativeOutcome,
    pub report: Option<agentmender_domain::CompleteRemovalReport>,
}

/// Schema-3 effects never fall back to the historical primary-only dispatcher.
pub trait DurableCompleteRemovalPort {
    fn preflight(&self, plan: &LifecyclePlan) -> Result<(), EffectError>;
    fn dispatch(
        &self,
        intent: DurableIntent,
        plan: &LifecyclePlan,
        progress: &mut dyn CompleteRemovalProgress,
    ) -> CompleteRemovalAttempt;
    /// Fresh observations only. Missing commit receipts never authorize retries.
    fn observe(
        &self,
        plan: &LifecyclePlan,
    ) -> Result<agentmender_domain::CompleteRemovalReport, EffectError>;
}
