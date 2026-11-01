//! Narrow typed lifecycle boundary. Implementations must retain no execution gate.
use crate::{DurableIntent, DurableObservation};
use agentmender_domain::{ArtifactGraph, CleanupReport, IntegrityReport, LifecyclePlan};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectError {
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
