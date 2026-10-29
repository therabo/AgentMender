//! Pure transaction knowledge. Records are integrity checks, not authority.
use crate::{
    LifecycleIntent, LifecyclePlan, NativeLifecycleOperation, PlanDigest, PlanStatus, ProviderKind,
    StateFingerprint,
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
            || c.capability.is_none()
            || c.effect_contract.is_none()
        {
            return Err(TransactionError);
        }
        let op = &c.operations[0];
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
pub enum TransactionEvent {
    Created {
        binding: TransactionBinding,
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
        let digest = JournalDigest(hash(
            "agentmender.journal.v1",
            &(
                1u32,
                &id,
                sequence,
                &binding_digest,
                &previous_digest,
                &event,
            ),
        ));
        Self {
            schema_version: 1,
            transaction_id: id,
            sequence,
            binding_digest,
            previous_digest,
            event,
            digest,
        }
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
        self.schema_version == 1
            && Self::new(
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
    sequence: u64,
    last_digest: JournalDigest,
}
impl TransactionSnapshot {
    pub fn replay(records: &[JournalRecord]) -> Result<Self, TransactionError> {
        let first = records.first().ok_or(TransactionError)?;
        let TransactionEvent::Created { binding } = &first.event else {
            return Err(TransactionError);
        };
        if first.sequence != 0
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
            ) if current(plan, state, operation_index) => S::OutcomeUnknown,
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
            ) if current(plan, state, operation_index)
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
        let r = JournalRecord::new(
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
