//! Pure approval scope and lifecycle. Authentication is an adapter responsibility.
use crate::{
    ArtifactGraph, BindingDigest, IntegrityReport, JournalDigest, LifecyclePlan, PlanValidation,
    PlanningContext, RecoveryDigest, TransactionBinding, TransactionError, TransactionId,
    TransactionSnapshot, TransactionState,
};
use serde::{Deserialize, Deserializer, Serialize};
use std::{fmt, str::FromStr};

macro_rules! opaque {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
        #[serde(transparent)]
        pub struct $name(String);
        impl $name {
            pub fn from_bytes(bytes: [u8; 32]) -> Self {
                Self(bytes.iter().map(|b| format!("{b:02x}")).collect())
            }
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
                if s.len() == 64
                    && s.bytes()
                        .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                {
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
opaque!(ApprovalId);
opaque!(ApprovalNonce);
opaque!(ApprovalKeyId);
opaque!(AuthenticationTag);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalDirection {
    ForwardOnly,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ApprovalPrincipal {
    LocalInteractiveUser,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalScope {
    pub transaction_id: TransactionId,
    pub binding: TransactionBinding,
    pub recovery_digest: RecoveryDigest,
    pub operation_index: u32,
    pub direction: ApprovalDirection,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ApprovalGrant {
    pub id: ApprovalId,
    pub nonce: ApprovalNonce,
    pub scope: ApprovalScope,
    pub principal: ApprovalPrincipal,
    pub issued_at: u64,
    pub expires_at: u64,
}
impl ApprovalGrant {
    /// Checks scope and time only. This does not authenticate the grant.
    pub fn scope_and_time_valid(&self, snapshot: &TransactionSnapshot, now: u64) -> bool {
        self.scope.transaction_id == *snapshot.id()
            && self.scope.binding == *snapshot.binding()
            && snapshot
                .recovery()
                .is_some_and(|r| r.digest() == self.scope.recovery_digest)
            && self.scope.operation_index == 0
            && self.scope.direction == ApprovalDirection::ForwardOnly
            && self.principal == ApprovalPrincipal::LocalInteractiveUser
            && self
                .expires_at
                .checked_sub(self.issued_at)
                .is_some_and(|duration| (1..=300).contains(&duration))
            && self.issued_at <= now
            && now < self.expires_at
    }
    pub(crate) fn structurally_valid(&self, snapshot: &TransactionSnapshot) -> bool {
        self.scope_and_time_valid(snapshot, self.issued_at)
    }
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case", deny_unknown_fields)]
#[allow(clippy::large_enum_variant)] // Preserve the direct typed wire/API shape.
pub enum ApprovalAction {
    Granted {
        grant: ApprovalGrant,
    },
    Revoked {
        approval_id: ApprovalId,
        at: u64,
    },
    Consumed {
        approval_id: ApprovalId,
        at: u64,
        plan: crate::PlanDigest,
        state: crate::StateFingerprint,
        operation_index: u32,
    },
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AuthorizationAuthentication {
    pub key_id: ApprovalKeyId,
    pub tag: AuthenticationTag,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalState {
    None,
    Granted,
    Revoked,
    Consumed,
}
/// Time-aware presentation of recorded approval state; this does not authenticate it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalStatus {
    None,
    Granted,
    Expired,
    Invalid,
    Revoked,
    Consumed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApprovalSnapshot {
    pub state: ApprovalState,
    pub grant: Option<ApprovalGrant>,
}
impl Default for ApprovalSnapshot {
    fn default() -> Self {
        Self {
            state: ApprovalState::None,
            grant: None,
        }
    }
}

/// Ordered, versioned signing input. MAC computation and verification belong to persistence.
pub fn authorization_signing_bytes(
    key_id: &ApprovalKeyId,
    id: &TransactionId,
    sequence: u64,
    binding: &BindingDigest,
    previous: &Option<JournalDigest>,
    action: &ApprovalAction,
) -> Vec<u8> {
    let mut bytes = b"agentmender.authorization.v2\0".to_vec();
    bytes.extend(
        serde_json::to_vec(&(2u32, key_id, id, sequence, binding, previous, action))
            .expect("typed authorization serializes"),
    );
    bytes
}
/// All fresh inputs must come from trusted adapters; recovery_verified confirms durable bytes.
pub fn approval_eligible(
    snapshot: &TransactionSnapshot,
    graph: &ArtifactGraph,
    report: &IntegrityReport,
    context: &PlanningContext,
    recovery_verified: bool,
) -> bool {
    snapshot.schema_version() == 2
        && snapshot.state() == TransactionState::Prepared
        && snapshot.approval().state == ApprovalState::None
        && recovery_verified
        && snapshot
            .recovery()
            .is_some_and(|r| r.verify(snapshot.id(), snapshot.binding()))
        && snapshot.review_plan().is_some_and(|plan: &LifecyclePlan| {
            TransactionBinding::from_plan(plan).is_ok_and(|binding| binding == *snapshot.binding())
                && plan.validate_against(graph, report, context) == PlanValidation::Current
        })
}

/// Checks the recorded grant and its scope and time, without authenticating it.
/// Persistence must independently verify the authorization MAC.
pub fn approval_valid(snapshot: &TransactionSnapshot, grant: &ApprovalGrant, now: u64) -> bool {
    snapshot.schema_version() == 2
        && snapshot.state() == TransactionState::Prepared
        && snapshot.approval().state == ApprovalState::Granted
        && snapshot.approval().grant.as_ref() == Some(grant)
        && grant.scope_and_time_valid(snapshot, now)
}
