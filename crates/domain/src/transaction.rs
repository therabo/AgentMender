//! Pure transaction knowledge. Records are integrity checks, not authority.
use crate::{
    ApprovalAction, ApprovalSnapshot, ApprovalState, ApprovalStatus, AuthorizationAuthentication,
    CleanupOutcome, CleanupReport, LifecycleIntent, LifecyclePlan, NativeLifecycleOperation,
    PlanDigest, PlanStatus, ProviderKind, StateFingerprint, validate_cleanup_report,
};
use crate::{
    CleanupActionOutcome, CompleteRemovalActionProgress, CompleteRemovalReport,
    validate_complete_removal_report,
};
use serde::{Deserialize, Deserializer, Serialize};
use sha2::{Digest, Sha256};
use std::{fmt, str::FromStr};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Error)]
#[error("invalid transaction record or transition")]
pub struct TransactionError;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn valid_hex(s: &str) -> bool {
    s.len() == 64
        && s.bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}
fn hash<T: Serialize>(domain: &str, value: &T) -> String {
    let mut h = Sha256::new();
    h.update(domain.as_bytes());
    h.update([0]);
    h.update(serde_json::to_vec(value).expect("typed transaction values serialize"));
    hex(&h.finalize())
}
macro_rules! opaque {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(f)
            }
        }
        impl FromStr for $name {
            type Err = TransactionError;
            fn from_str(s: &str) -> Result<Self, Self::Err> {
                if valid_hex(s) {
                    Ok(Self(s.into()))
                } else {
                    Err(TransactionError)
                }
            }
        }
        impl<'de> Deserialize<'de> for $name {
            fn deserialize<D: Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
                String::deserialize(d)?
                    .parse()
                    .map_err(serde::de::Error::custom)
            }
        }
    };
}
opaque!(TransactionId);
opaque!(JournalDigest);
opaque!(RecoveryDigest);
opaque!(BindingDigest);
opaque!(Commitment);
impl TransactionId {
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(hex(&bytes))
    }
}
fn commitment<T: Serialize>(domain: &str, value: &T) -> Commitment {
    Commitment(hash(domain, value))
}

// M4 specifies these particular plan fields as sets. Journal payloads are never
// passed through this helper: their order is part of their integrity contract.
fn set_value<T: Serialize>(value: &T) -> serde_json::Value {
    let mut values = serde_json::to_value(value).expect("plan field serializes");
    if let serde_json::Value::Array(items) = &mut values {
        items.sort_by_key(|item| serde_json::to_vec(item).expect("JSON value"));
        items.dedup();
    }
    values
}
/// The persisted binding deliberately excludes provider identities and prose.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TransactionBinding {
    plan_digest: PlanDigest,
    state_fingerprint: StateFingerprint,
    provider: ProviderKind,
    intent: LifecycleIntent,
    operation: NativeLifecycleOperation,
    target: Commitment,
    native_identity: Commitment,
    profile: Commitment,
    capability_effect: Commitment,
    conditions: Commitment,
    operation_commitment: Commitment,
}
impl TransactionBinding {
    pub fn from_plan(plan: &LifecyclePlan) -> Result<Self, TransactionError> {
        let c = plan.content();
        if c.status != PlanStatus::Ready
            || c.operations.len() != 1
            || (c.request.intent != LifecycleIntent::CleanupSession
                && (c.capability.is_none() || c.effect_contract.is_none()))
        {
            return Err(TransactionError);
        }
        let op = &c.operations[0];
        if c.request.intent == LifecycleIntent::CleanupSession {
            let manifest = if c.schema_version == 3 {
                &c.complete_removal_manifest
                    .as_ref()
                    .ok_or(TransactionError)?
                    .primary
            } else {
                c.cleanup_manifest.as_ref().ok_or(TransactionError)?
            };
            if !matches!(c.schema_version, 2 | 3)
                || (c.schema_version == 3
                    && !c
                        .complete_removal_manifest
                        .as_ref()
                        .is_some_and(crate::validate_complete_removal_manifest))
                || !crate::validate_cleanup_manifest(manifest)
                || op.operation != NativeLifecycleOperation::ScopedCleanupSession
                || c.capability.is_some()
                || c.effect_contract.is_some()
            {
                return Err(TransactionError);
            }
            return Ok(Self {
                plan_digest: plan.digest().clone(),
                state_fingerprint: plan.state_fingerprint().clone(),
                provider: manifest.target.provider,
                intent: c.request.intent,
                operation: op.operation,
                target: commitment("agentmender.target.v1", &c.request.target),
                native_identity: commitment("agentmender.native-id.v1", &op.native_id),
                profile: commitment("agentmender.profile.v1", &c.provider),
                capability_effect: if c.schema_version == 3 {
                    commitment(
                        "agentmender.complete-removal-manifest.v3",
                        c.complete_removal_manifest
                            .as_ref()
                            .ok_or(TransactionError)?,
                    )
                } else {
                    commitment("agentmender.scoped-cleanup-manifest.v2", manifest)
                },
                conditions: commitment(
                    "agentmender.conditions.v1",
                    &(set_value(&c.preconditions), set_value(&c.postconditions)),
                ),
                operation_commitment: commitment(
                    "agentmender.operation.v1",
                    &(
                        &op.target,
                        &op.native_id,
                        op.operation,
                        set_value(&op.affected),
                    ),
                ),
            });
        }
        let capability = c.capability.as_ref().ok_or(TransactionError)?;
        let effect = c.effect_contract.as_ref().ok_or(TransactionError)?;
        let effect_value = (
            &effect.id,
            set_value(&effect.affected),
            set_value(&effect.rule_ids),
        );
        let operation_value = (
            &op.target,
            &op.native_id,
            op.operation,
            set_value(&op.affected),
        );
        Ok(Self {
            plan_digest: plan.digest().clone(),
            state_fingerprint: plan.state_fingerprint().clone(),
            provider: capability.provider,
            intent: c.request.intent,
            operation: op.operation,
            target: commitment("agentmender.target.v1", &c.request.target),
            native_identity: commitment("agentmender.native-id.v1", &op.native_id),
            profile: commitment("agentmender.profile.v1", &c.provider),
            capability_effect: commitment(
                "agentmender.capability-effect.v1",
                &(&c.capability, effect_value),
            ),
            conditions: commitment(
                "agentmender.conditions.v1",
                &(set_value(&c.preconditions), set_value(&c.postconditions)),
            ),
            operation_commitment: commitment("agentmender.operation.v1", &operation_value),
        })
    }
    pub fn digest(&self) -> BindingDigest {
        BindingDigest(hash("agentmender.binding.v1", self))
    }
    pub fn plan_digest(&self) -> &PlanDigest {
        &self.plan_digest
    }
    pub fn state_fingerprint(&self) -> &StateFingerprint {
        &self.state_fingerprint
    }
    pub fn provider(&self) -> ProviderKind {
        self.provider
    }
    pub fn intent(&self) -> LifecycleIntent {
        self.intent
    }
    fn valid(&self) -> bool {
        matches!(
            (self.intent, self.operation),
            (
                LifecycleIntent::Archive,
                NativeLifecycleOperation::ArchiveSession
            ) | (
                LifecycleIntent::Unarchive,
                NativeLifecycleOperation::UnarchiveSession
            ) | (
                LifecycleIntent::Purge,
                NativeLifecycleOperation::DeleteSession
            ) | (
                LifecycleIntent::CleanupSession,
                NativeLifecycleOperation::ScopedCleanupSession
            )
        )
    }
    pub fn operation(&self) -> NativeLifecycleOperation {
        self.operation
    }
    pub fn target(&self) -> &Commitment {
        &self.target
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryStrategy {
    ReconciliationOnly,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub enum CompensationContract {
    NotCompensatable,
    ReviewedSemantic {
        operation: NativeLifecycleOperation,
        evidence: Commitment,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryManifest {
    schema_version: u32,
    transaction_id: TransactionId,
    binding: BindingDigest,
    plan: PlanDigest,
    state: StateFingerprint,
    provider: ProviderKind,
    operation: NativeLifecycleOperation,
    target: Commitment,
    strategy: RecoveryStrategy,
    compensation: CompensationContract,
}
impl RecoveryManifest {
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn new(id: TransactionId, binding: &TransactionBinding) -> Self {
        Self {
            schema_version: 1,
            transaction_id: id,
            binding: binding.digest(),
            plan: binding.plan_digest.clone(),
            state: binding.state_fingerprint.clone(),
            provider: binding.provider,
            operation: binding.operation,
            target: binding.target.clone(),
            strategy: RecoveryStrategy::ReconciliationOnly,
            compensation: CompensationContract::NotCompensatable,
        }
    }
    pub fn with_compensation(mut self, contract: CompensationContract) -> Self {
        self.compensation = contract;
        self
    }
    pub fn digest(&self) -> RecoveryDigest {
        RecoveryDigest(hash("agentmender.recovery.v1", self))
    }
    pub fn verify(&self, id: &TransactionId, binding: &TransactionBinding) -> bool {
        self.schema_version == 1
            && &self.transaction_id == id
            && self.binding == binding.digest()
            && self.plan == binding.plan_digest
            && self.state == binding.state_fingerprint
            && self.provider == binding.provider
            && self.operation == binding.operation
            && self.target == binding.target
            && self.strategy == RecoveryStrategy::ReconciliationOnly
            && (binding.intent != LifecycleIntent::CleanupSession
                || self.compensation == CompensationContract::NotCompensatable)
    }
    pub fn compensation(&self) -> &CompensationContract {
        &self.compensation
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Observation {
    Applied,
    NoEffect,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Verification {
    Passed,
    Failed,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)] // Preserve the direct typed wire/API shape.
pub enum TransactionEvent {
    Created {
        binding: TransactionBinding,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        plan: Option<LifecyclePlan>,
    },
    Authorization {
        action: ApprovalAction,
        authentication: AuthorizationAuthentication,
    },
    RecoveryPrepared {
        manifest: RecoveryManifest,
    },
    Prepared,
    Intent {
        plan: PlanDigest,
        state: StateFingerprint,
        operation_index: u32,
    },
    Observed {
        observation: Observation,
    },
    CleanupObserved {
        report: CleanupReport,
    },
    CompleteRemovalActionObserved {
        progress: CompleteRemovalActionProgress,
    },
    CompleteRemovalObserved {
        report: CompleteRemovalReport,
    },
    VerificationStarted,
    Verified {
        verification: Verification,
    },
    CompensationIntent {
        plan: PlanDigest,
        state: StateFingerprint,
        operation_index: u32,
    },
    CompensationObserved {
        observation: Observation,
    },
    CompensationVerificationStarted,
    CompensationVerified {
        verification: Verification,
    },
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransactionState {
    Created,
    RecoveryPrepared,
    Prepared,
    OutcomeUnknown,
    EffectObserved,
    Verifying,
    Succeeded,
    Failed,
    RecoveryRequired,
    CompensationOutcomeUnknown,
    CompensationEffectObserved,
    CompensationVerifying,
    Compensated,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JournalRecord {
    schema_version: u32,
    transaction_id: TransactionId,
    sequence: u64,
    binding_digest: BindingDigest,
    previous_digest: Option<JournalDigest>,
    event: TransactionEvent,
    digest: JournalDigest,
}
impl JournalRecord {
    pub fn new(
        id: TransactionId,
        sequence: u64,
        binding_digest: BindingDigest,
        previous_digest: Option<JournalDigest>,
        event: TransactionEvent,
    ) -> Self {
        Self::with_version(1, id, sequence, binding_digest, previous_digest, event)
    }
    pub fn new_v2(
        id: TransactionId,
        sequence: u64,
        binding_digest: BindingDigest,
        previous_digest: Option<JournalDigest>,
        event: TransactionEvent,
    ) -> Self {
        Self::with_version(2, id, sequence, binding_digest, previous_digest, event)
    }
    fn with_version(
        version: u32,
        id: TransactionId,
        sequence: u64,
        binding_digest: BindingDigest,
        previous_digest: Option<JournalDigest>,
        event: TransactionEvent,
    ) -> Self {
        let digest = JournalDigest(hash(
            if version == 1 {
                "agentmender.journal.v1"
            } else {
                "agentmender.journal.v2"
            },
            &(
                version,
                &id,
                sequence,
                &binding_digest,
                &previous_digest,
                &event,
            ),
        ));
        Self {
            schema_version: version,
            transaction_id: id,
            sequence,
            binding_digest,
            previous_digest,
            event,
            digest,
        }
    }
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn digest(&self) -> &JournalDigest {
        &self.digest
    }
    pub fn event(&self) -> &TransactionEvent {
        &self.event
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn transaction_id(&self) -> &TransactionId {
        &self.transaction_id
    }
    pub fn binding_digest(&self) -> &BindingDigest {
        &self.binding_digest
    }
    pub fn previous_digest(&self) -> Option<&JournalDigest> {
        self.previous_digest.as_ref()
    }
    fn valid_digest(&self) -> bool {
        matches!(self.schema_version, 1 | 2)
            && Self::with_version(
                self.schema_version,
                self.transaction_id.clone(),
                self.sequence,
                self.binding_digest.clone(),
                self.previous_digest.clone(),
                self.event.clone(),
            )
            .digest
                == self.digest
    }
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TransactionSnapshot {
    id: TransactionId,
    binding: TransactionBinding,
    state: TransactionState,
    recovery: Option<RecoveryManifest>,
    schema_version: u32,
    review_plan: Option<LifecyclePlan>,
    approval: ApprovalSnapshot,
    complete_progress: Vec<CompleteRemovalActionProgress>,
    sequence: u64,
    last_digest: JournalDigest,
}
impl TransactionSnapshot {
    pub fn replay(records: &[JournalRecord]) -> Result<Self, TransactionError> {
        let first = records.first().ok_or(TransactionError)?;
        let TransactionEvent::Created { binding, plan } = &first.event else {
            return Err(TransactionError);
        };
        if (first.schema_version == 1 && binding.intent == LifecycleIntent::CleanupSession)
            || !(matches!((first.schema_version, plan), (1, None))
                || (first.schema_version == 2
                    && plan.as_ref().is_some_and(|p| {
                        TransactionBinding::from_plan(p).is_ok_and(|b| b == *binding)
                    })))
            || first.sequence != 0
            || first.previous_digest.is_some()
            || !binding.valid()
            || first.binding_digest != binding.digest()
            || !first.valid_digest()
        {
            return Err(TransactionError);
        }
        let mut s = Self {
            id: first.transaction_id.clone(),
            binding: binding.clone(),
            state: TransactionState::Created,
            recovery: None,
            schema_version: first.schema_version,
            review_plan: plan.clone(),
            approval: ApprovalSnapshot::default(),
            complete_progress: Vec::new(),
            sequence: 0,
            last_digest: first.digest.clone(),
        };
        for r in &records[1..] {
            s.apply(r)?;
        }
        Ok(s)
    }
    fn apply(&mut self, r: &JournalRecord) -> Result<(), TransactionError> {
        if !r.valid_digest()
            || r.schema_version != self.schema_version
            || r.transaction_id != self.id
            || r.binding_digest != self.binding.digest()
            || self.sequence.checked_add(1) != Some(r.sequence)
            || r.previous_digest.as_ref() != Some(&self.last_digest)
        {
            return Err(TransactionError);
        }
        use TransactionEvent as E;
        use TransactionState as S;
        let current = |plan: &PlanDigest, state: &StateFingerprint, index: &u32| {
            plan == &self.binding.plan_digest
                && state == &self.binding.state_fingerprint
                && *index == 0
        };
        if let E::Authorization { action, .. } = &r.event {
            if self.schema_version != 2 || self.state != S::Prepared {
                return Err(TransactionError);
            }
            match action {
                ApprovalAction::Granted { grant }
                    if self.approval.state == ApprovalState::None
                        && grant.structurally_valid(self) =>
                {
                    self.approval = ApprovalSnapshot {
                        state: ApprovalState::Granted,
                        grant: Some(grant.clone()),
                    };
                }
                ApprovalAction::Revoked { approval_id, at }
                    if self.approval.state == ApprovalState::Granted
                        && self
                            .approval
                            .grant
                            .as_ref()
                            .is_some_and(|g| &g.id == approval_id && *at >= g.issued_at) =>
                {
                    self.approval.state = ApprovalState::Revoked
                }
                ApprovalAction::Consumed {
                    approval_id,
                    at,
                    plan,
                    state,
                    operation_index,
                } if self.approval.state == ApprovalState::Granted
                    && current(plan, state, operation_index)
                    && self.approval.grant.as_ref().is_some_and(|g| {
                        &g.id == approval_id && g.scope_and_time_valid(self, *at)
                    }) =>
                {
                    self.approval.state = ApprovalState::Consumed;
                    self.state = S::OutcomeUnknown;
                }
                _ => return Err(TransactionError),
            }
            self.sequence = r.sequence;
            self.last_digest = r.digest.clone();
            return Ok(());
        }
        if self.binding.intent == LifecycleIntent::CleanupSession {
            let plan_version = self
                .review_plan
                .as_ref()
                .map(|p| p.content().schema_version);
            if let E::CompleteRemovalActionObserved { progress } = &r.event {
                let m = self
                    .review_plan
                    .as_ref()
                    .and_then(|p| p.content().complete_removal_manifest.as_ref())
                    .ok_or(TransactionError)?;
                if plan_version != Some(3)
                    || !matches!(
                        progress.outcome,
                        CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
                    )
                    || !matches!(self.state, S::OutcomeUnknown | S::RecoveryRequired)
                    || m.actions.get(self.complete_progress.len()) != Some(&progress.action)
                    || self.complete_progress.last().is_some_and(|p| {
                        !matches!(
                            p.outcome,
                            CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
                        )
                    })
                {
                    return Err(TransactionError);
                }
                self.complete_progress.push(progress.clone());
                self.sequence = r.sequence;
                self.last_digest = r.digest.clone();
                return Ok(());
            }
            if let E::CompleteRemovalObserved { report } = &r.event {
                let m = self
                    .review_plan
                    .as_ref()
                    .and_then(|p| p.content().complete_removal_manifest.as_ref())
                    .ok_or(TransactionError)?;
                if plan_version != Some(3)
                    || !matches!(self.state, S::OutcomeUnknown | S::RecoveryRequired)
                    || report.actions != self.complete_progress
                    || !validate_complete_removal_report(m, report)
                {
                    return Err(TransactionError);
                }
                self.state = match report.primary.outcome {
                    CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals => S::Succeeded,
                    CleanupOutcome::Blocked | CleanupOutcome::Failed => S::Failed,
                    CleanupOutcome::OutcomeUnknown => S::RecoveryRequired,
                };
                self.sequence = r.sequence;
                self.last_digest = r.digest.clone();
                return Ok(());
            }
            if let E::CleanupObserved { report } = &r.event {
                let manifest = self
                    .review_plan
                    .as_ref()
                    .and_then(|p| p.content().cleanup_manifest.as_ref())
                    .ok_or(TransactionError)?;
                if self.schema_version != 2
                    || plan_version != Some(2)
                    || !matches!(self.state, S::OutcomeUnknown | S::RecoveryRequired)
                    || !validate_cleanup_report(manifest, report)
                {
                    return Err(TransactionError);
                }
                self.state = match report.outcome {
                    CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals => S::Succeeded,
                    CleanupOutcome::Blocked | CleanupOutcome::Failed => S::Failed,
                    CleanupOutcome::OutcomeUnknown => S::RecoveryRequired,
                };
                self.sequence = r.sequence;
                self.last_digest = r.digest.clone();
                return Ok(());
            }
            if matches!(
                r.event,
                E::Observed { .. } | E::VerificationStarted | E::Verified { .. }
            ) {
                return Err(TransactionError);
            }
        }
        self.state = match (&self.state, &r.event) {
            (S::Created, E::RecoveryPrepared { manifest })
                if manifest.verify(&self.id, &self.binding) =>
            {
                self.recovery = Some(manifest.clone());
                S::RecoveryPrepared
            }
            (S::RecoveryPrepared, E::Prepared) => S::Prepared,
            (
                S::Prepared,
                E::Intent {
                    plan,
                    state,
                    operation_index,
                },
            ) if self.schema_version == 1 && current(plan, state, operation_index) => {
                S::OutcomeUnknown
            }
            (
                S::OutcomeUnknown,
                E::Observed {
                    observation: Observation::Applied,
                },
            ) => S::EffectObserved,
            (
                S::OutcomeUnknown,
                E::Observed {
                    observation: Observation::NoEffect,
                },
            ) => S::Failed,
            (
                S::OutcomeUnknown,
                E::Observed {
                    observation: Observation::Unknown,
                },
            ) => S::RecoveryRequired,
            (S::EffectObserved, E::VerificationStarted) => S::Verifying,
            (
                S::Verifying,
                E::Verified {
                    verification: Verification::Passed,
                },
            ) => S::Succeeded,
            (
                S::Verifying,
                E::Verified {
                    verification: Verification::Failed | Verification::Unknown,
                },
            ) => S::RecoveryRequired,
            (
                S::EffectObserved,
                E::CompensationIntent {
                    plan,
                    state,
                    operation_index,
                },
            ) if self.schema_version == 1
                && current(plan, state, operation_index)
                && self.recovery.as_ref().is_some_and(|m| {
                    matches!(
                        m.compensation,
                        CompensationContract::ReviewedSemantic { .. }
                    )
                }) =>
            {
                S::CompensationOutcomeUnknown
            }
            (
                S::CompensationOutcomeUnknown,
                E::CompensationObserved {
                    observation: Observation::Applied,
                },
            ) => S::CompensationEffectObserved,
            (
                S::CompensationOutcomeUnknown,
                E::CompensationObserved {
                    observation: Observation::NoEffect | Observation::Unknown,
                },
            ) => S::RecoveryRequired,
            (S::CompensationEffectObserved, E::CompensationVerificationStarted) => {
                S::CompensationVerifying
            }
            (
                S::CompensationVerifying,
                E::CompensationVerified {
                    verification: Verification::Passed,
                },
            ) => S::Compensated,
            (
                S::CompensationVerifying,
                E::CompensationVerified {
                    verification: Verification::Failed | Verification::Unknown,
                },
            ) => S::RecoveryRequired,
            _ => return Err(TransactionError),
        };
        self.sequence = r.sequence;
        self.last_digest = r.digest.clone();
        Ok(())
    }
    /// Validates one subsequent record without changing the original snapshot.
    pub fn advanced(&self, record: &JournalRecord) -> Result<Self, TransactionError> {
        let mut next = self.clone();
        next.apply(record)?;
        Ok(next)
    }
    pub fn next_record(&self, event: TransactionEvent) -> Result<JournalRecord, TransactionError> {
        let r = JournalRecord::with_version(
            self.schema_version,
            self.id.clone(),
            self.sequence.checked_add(1).ok_or(TransactionError)?,
            self.binding.digest(),
            Some(self.last_digest.clone()),
            event,
        );
        let mut check = self.clone();
        check.apply(&r)?;
        Ok(r)
    }
    pub fn schema_version(&self) -> u32 {
        self.schema_version
    }
    pub fn review_plan(&self) -> Option<&LifecyclePlan> {
        self.review_plan.as_ref()
    }
    /// Reports the recorded lifecycle at a caller-supplied time; does not authenticate it.
    pub fn approval_status_at(&self, now: u64) -> ApprovalStatus {
        match self.approval.state {
            ApprovalState::None => ApprovalStatus::None,
            ApprovalState::Revoked => ApprovalStatus::Revoked,
            ApprovalState::Consumed => ApprovalStatus::Consumed,
            ApprovalState::Granted => match self.approval.grant.as_ref() {
                Some(grant) if now < grant.issued_at => ApprovalStatus::Invalid,
                Some(grant) if now >= grant.expires_at => ApprovalStatus::Expired,
                Some(grant) if crate::approval_valid(self, grant, now) => ApprovalStatus::Granted,
                _ => ApprovalStatus::Invalid,
            },
        }
    }
    pub fn approval(&self) -> &ApprovalSnapshot {
        &self.approval
    }
    pub fn state(&self) -> TransactionState {
        self.state
    }
    pub fn id(&self) -> &TransactionId {
        &self.id
    }
    pub fn binding(&self) -> &TransactionBinding {
        &self.binding
    }
    pub fn recovery(&self) -> Option<&RecoveryManifest> {
        self.recovery.as_ref()
    }
    pub fn sequence(&self) -> u64 {
        self.sequence
    }
    pub fn last_digest(&self) -> &JournalDigest {
        &self.last_digest
    }
}
