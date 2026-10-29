#[path = "../../persistence/tests/common/mod.rs"]
mod common;

use agentmender_application::transactions::{
    RecoveryAvailability, RecoveryCoordinator, TransactionInspection, TransactionQueryError,
    TransactionReadPort,
};
use agentmender_domain::*;

struct Reader(Vec<TransactionInspection>);
impl TransactionReadPort for Reader {
    fn list(&self) -> Result<Vec<TransactionInspection>, TransactionQueryError> {
        Ok(self.0.clone())
    }
    fn inspect(&self, id: &TransactionId) -> Result<TransactionInspection, TransactionQueryError> {
        self.0
            .iter()
            .find(|entry| &entry.transaction_id == id)
            .cloned()
            .ok_or(TransactionQueryError::NotFound)
    }
    fn journal(&self, _: &TransactionId) -> Result<Vec<JournalRecord>, TransactionQueryError> {
        Err(TransactionQueryError::Unavailable)
    }
}
fn snapshot(id: u8, final_state: TransactionState) -> TransactionSnapshot {
    let plan = common::case(0).plan(LifecycleIntent::Archive);
    let binding = TransactionBinding::from_plan(&plan).unwrap();
    let id = TransactionId::from_bytes([id; 32]);
    let mut records = vec![JournalRecord::new(
        id.clone(),
        0,
        binding.digest(),
        None,
        TransactionEvent::Created {
            binding: binding.clone(),
            plan: None,
        },
    )];
    if final_state == TransactionState::Created {
        return TransactionSnapshot::replay(&records).unwrap();
    }
    let events = [
        TransactionEvent::RecoveryPrepared {
            manifest: RecoveryManifest::new(id, &binding),
        },
        TransactionEvent::Prepared,
        TransactionEvent::Intent {
            plan: binding.plan_digest().clone(),
            state: binding.state_fingerprint().clone(),
            operation_index: 0,
        },
    ];
    for event in events {
        let snap = TransactionSnapshot::replay(&records).unwrap();
        records.push(snap.next_record(event).unwrap());
    }
    if final_state == TransactionState::OutcomeUnknown {
        return TransactionSnapshot::replay(&records).unwrap();
    }
    let snap = TransactionSnapshot::replay(&records).unwrap();
    records.push(
        snap.next_record(TransactionEvent::Observed {
            observation: Observation::NoEffect,
        })
        .unwrap(),
    );
    assert_eq!(final_state, TransactionState::Failed);
    TransactionSnapshot::replay(&records).unwrap()
}
#[test]
fn coordinator_reports_pending_and_unavailable_without_inventing_provider_state() {
    let created = snapshot(1, TransactionState::Created);
    let unknown = snapshot(2, TransactionState::OutcomeUnknown);
    let failed = snapshot(3, TransactionState::Failed);
    let protected = snapshot(4, TransactionState::Failed);
    let reader = Reader(vec![
        TransactionInspection::from_snapshot(&created, RecoveryAvailability::NotPrepared),
        TransactionInspection::from_snapshot(&unknown, RecoveryAvailability::Verified),
        TransactionInspection::from_snapshot(&failed, RecoveryAvailability::Verified),
        TransactionInspection::from_snapshot(&protected, RecoveryAvailability::Unavailable),
    ]);
    let pending = RecoveryCoordinator::new(&reader).pending().unwrap();
    assert_eq!(
        pending
            .iter()
            .map(|s| s.transaction_id.clone())
            .collect::<Vec<_>>(),
        vec![
            created.id().clone(),
            unknown.id().clone(),
            protected.id().clone()
        ]
    );
    assert_eq!(pending[1].state, TransactionState::OutcomeUnknown);
    assert!(pending[1].protected_recovery_required);
    assert!(pending[2].protected_recovery_required);
    assert!(
        pending
            .iter()
            .all(|entry| !entry.provider_state_verified && !entry.execution_authorized)
    );
}
