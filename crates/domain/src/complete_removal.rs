//! Frozen semantic scope and observations for separately approved complete removal.
use crate::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompleteRemovalAction {
    PrimaryPayload,
    StateThreadRecords,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WriterRequirement {
    CompatibleLifetimeFence,
    CodexClosedWithLifetimeFence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProfileAdmission {
    PrimaryOnly,
    CompleteAccepted,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalProfile {
    pub identity: EntityId,
    pub admission: ProfileAdmission,
    pub writer_requirement: WriterRequirement,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentSurface {
    pub id: EntityId,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalManifest {
    pub primary: CleanupManifest,
    pub actions: Vec<CompleteRemovalAction>,
    pub profile: CompleteRemovalProfile,
    pub required_surfaces: Vec<PersistentSurface>,
    pub technical_residuals: Vec<CleanupResidual>,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CompleteRemovalEvidence {
    pub manifest: CompleteRemovalManifest,
    pub primary_evidence: ScopedCleanupEvidence,
}
/// Only the reviewed expanded primary contract is executable; admission is never
/// obtained from serialized claims or a provider observation.
pub fn validate_complete_removal_manifest(m: &CompleteRemovalManifest) -> bool {
    validate_cleanup_manifest(&m.primary)
        && m.primary.optional_index.is_none()
        && m.primary.profile.adapter == "scoped-cleanup-v2"
        && m.actions
            == [
                CompleteRemovalAction::PrimaryPayload,
                CompleteRemovalAction::StateThreadRecords,
            ]
        && m.profile.identity.as_str() == "codex-scoped-cleanup-v2"
        && m.profile.admission == ProfileAdmission::PrimaryOnly
        && m.profile.writer_requirement == WriterRequirement::CompatibleLifetimeFence
        && m.required_surfaces.len() == 1
        && m.required_surfaces[0].id.as_str() == "codex-desktop-reconstruction-unverified"
        && !m.required_surfaces.is_empty()
        && m.required_surfaces
            .iter()
            .map(|s| &s.id)
            .collect::<BTreeSet<_>>()
            .len()
            == m.required_surfaces.len()
        && m.technical_residuals == m.primary.residuals
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PersistentObservation {
    Absent,
    Present,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PersistentSurfaceObservation {
    pub id: EntityId,
    pub observation: PersistentObservation,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalActionProgress {
    pub action: CompleteRemovalAction,
    pub outcome: CleanupActionOutcome,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompleteRemovalReport {
    pub primary: CleanupReport,
    pub actions: Vec<CompleteRemovalActionProgress>,
    pub surfaces: Vec<PersistentSurfaceObservation>,
}
pub fn validate_complete_removal_report(
    m: &CompleteRemovalManifest,
    r: &CompleteRemovalReport,
) -> bool {
    validate_complete_removal_manifest(m)
        && validate_cleanup_report(&m.primary, &r.primary)
        && (!matches!(
            r.primary.outcome,
            CleanupOutcome::Blocked | CleanupOutcome::Failed
        ) || (r.actions.is_empty()
            && r.primary.actions.iter().all(|a| {
                matches!(
                    a.outcome,
                    CleanupActionOutcome::Retained | CleanupActionOutcome::Failed
                )
            })))
        && r.actions.iter().all(|a| {
            matches!(
                a.outcome,
                CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
            )
        })
        && r.actions.len() <= m.actions.len()
        && r.actions
            .iter()
            .zip(&m.actions)
            .all(|(a, b)| a.action == *b)
        && (!matches!(
            r.primary.outcome,
            CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals
        ) || (r.actions.len() == m.actions.len()
            && r.actions.iter().all(|a| {
                matches!(
                    a.outcome,
                    CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
                )
            })))
        && (!matches!(
            r.primary.outcome,
            CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals
        ) || (r.primary.canonical_row_absent
            && r.primary.retained_primary_unchanged
            && r.primary.actions.iter().all(|a| {
                matches!(
                    a.outcome,
                    CleanupActionOutcome::Removed | CleanupActionOutcome::AlreadyAbsent
                )
            })))
        && r.actions.first().is_none_or(|a| {
            a.outcome != CleanupActionOutcome::AlreadyAbsent
                || r.primary
                    .actions
                    .iter()
                    .all(|p| p.outcome != CleanupActionOutcome::Removed)
        })
        && r.surfaces.len() == m.required_surfaces.len()
        && r.surfaces
            .iter()
            .zip(&m.required_surfaces)
            .all(|(a, b)| a.id == b.id)
}
/// No currently compiled Desktop profile has isolated restart acceptance.
pub fn assess_complete_removal(
    m: &CompleteRemovalManifest,
    r: &CompleteRemovalReport,
) -> SessionRemovalAssessment {
    if !validate_complete_removal_report(m, r) {
        return SessionRemovalAssessment {
            outcome: SessionRemovalOutcome::InvalidEvidence,
            maximum_safe_cleanup: MaximumSafeCleanup::Unknown,
            coverage: None,
            primary_removed: false,
            projection_clean: false,
        };
    }
    assess_legacy_session_removal(&m.primary, &r.primary)
}
