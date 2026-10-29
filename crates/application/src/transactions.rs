//! Read-only durable knowledge. No operation here creates or advances transactions.
use agentmender_domain::{
    JournalRecord, RecoveryDigest, TransactionBinding, TransactionId, TransactionSnapshot,
    TransactionState,
};
use serde::Serialize;
use thiserror::Error;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryAvailability {
    NotPrepared,
    Verified,
    Unavailable,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct TransactionInspection {
    pub transaction_id: TransactionId,
    pub binding: TransactionBinding,
    pub state: TransactionState,
    /// Sequence is authoritative ordering; no provider-state conclusion is implied.
    pub last_sequence: u64,
    pub recovery: RecoveryAvailability,
    pub recovery_digest: Option<RecoveryDigest>,
    pub protected_recovery_required: bool,
    pub provider_state_verified: bool,
    pub execution_authorized: bool,
}
impl TransactionInspection {
    pub fn from_snapshot(snapshot: &TransactionSnapshot, recovery: RecoveryAvailability) -> Self {
        Self {
            transaction_id: snapshot.id().clone(),
            binding: snapshot.binding().clone(),
            state: snapshot.state(),
            last_sequence: snapshot.sequence(),
            recovery,
            recovery_digest: snapshot.recovery().map(|m| m.digest()),
            protected_recovery_required: recovery == RecoveryAvailability::Unavailable
                || matches!(
                    snapshot.state(),
                    TransactionState::OutcomeUnknown
                        | TransactionState::CompensationOutcomeUnknown
                        | TransactionState::RecoveryRequired
                ),
            provider_state_verified: false,
            execution_authorized: false,
        }
    }
}

#[derive(Debug, Error)]
pub enum TransactionQueryError {
    #[error("transaction state is unavailable or requires protected recovery")]
    Unavailable,
    #[error("transaction was not found")]
    NotFound,
}

/// Implemented at composition by a read-only persistence adapter, never a provider.
pub trait TransactionReadPort {
    fn list(&self) -> Result<Vec<TransactionInspection>, TransactionQueryError>;
    fn inspect(&self, id: &TransactionId) -> Result<TransactionInspection, TransactionQueryError>;
    fn journal(&self, id: &TransactionId) -> Result<Vec<JournalRecord>, TransactionQueryError>;
}

pub struct TransactionInspectionService<'a> {
    reader: &'a dyn TransactionReadPort,
}
impl<'a> TransactionInspectionService<'a> {
    pub fn new(reader: &'a dyn TransactionReadPort) -> Self {
        Self { reader }
    }
    pub fn list(&self) -> Result<Vec<TransactionInspection>, TransactionQueryError> {
        self.reader.list()
    }
    pub fn inspect(
        &self,
        id: &TransactionId,
    ) -> Result<TransactionInspection, TransactionQueryError> {
        self.reader.inspect(id)
    }
    pub fn journal(&self, id: &TransactionId) -> Result<Vec<JournalRecord>, TransactionQueryError> {
        self.reader.journal(id)
    }
}

/// Restart recovery reports the durable facts requiring future observation.
/// It cannot retry effects, compensate, approve or conclude provider state.
pub struct RecoveryCoordinator<'a> {
    reader: &'a dyn TransactionReadPort,
}
impl<'a> RecoveryCoordinator<'a> {
    pub fn new(reader: &'a dyn TransactionReadPort) -> Self {
        Self { reader }
    }
    pub fn pending(&self) -> Result<Vec<TransactionInspection>, TransactionQueryError> {
        Ok(self
            .reader
            .list()?
            .into_iter()
            .filter(|s| {
                s.protected_recovery_required
                    || !matches!(
                        s.state,
                        TransactionState::Succeeded
                            | TransactionState::Failed
                            | TransactionState::Compensated
                    )
            })
            .collect())
    }
}
