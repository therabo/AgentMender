//! Pure product assessment. These observations neither authorize nor execute effects.
use serde::Serialize;
use std::collections::BTreeSet;

use crate::{
    CleanupManifest, CleanupOutcome, CleanupReport, Completeness, EntityId, validate_cleanup_report,
};

/// Exact profile and root identity are opaque identifiers, not paths or ownership claims.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalScope {
    pub target: EntityId,
    pub root: EntityId,
    pub profile: EntityId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalObservation {
    Absent,
    Present,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Preservation {
    Unchanged,
    Changed,
    Unknown,
}

/// Reconstruction role is established by the inventory, not inferred from UI visibility.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SurfaceKind {
    RequiredPersistentProjection,
    EphemeralUiProjection,
    UnknownReconstructionSurface,
}
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectionSurface {
    pub scope: RemovalScope,
    pub id: EntityId,
    pub kind: SurfaceKind,
    /// Current structural observation; ephemeral UI is not an authority.
    pub observation: RemovalObservation,
}
/// Adapter-owned structural compatibility evidence under ADR-0021.
/// This evidence cannot be imported or supplied by a graphical observation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactProfileStructuralEvidence {
    pub profile: EntityId,
    pub reconstruction_source_coverage: Completeness,
    pub collateral: Preservation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnitOwnership {
    ExclusiveTarget,
    Shared,
    Ambiguous,
    Unknown,
    Unsupported,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RemovalDisposition {
    Removed,
    AlreadyAbsent,
    /// Fresh safety checks failed; the unit was retained.
    FreshSafetyFailure,
    /// The retained unit is proven unable to reconstruct a projection or primary state.
    RetainedNonProjecting,
    Skipped,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResidualClass {
    ProjectionResidual,
    SafeTechnicalResidual,
    UnknownTechnicalResidual,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReconstructionRisk {
    None,
    Present,
    Unknown,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RetentionReason {
    FreshIdentityChanged,
    UnsafePhysicalIdentity,
    UnsupportedRemoval,
}

/// A semantic exact unit, including an exact record inside a shared container.
/// IDs identify units; they are not SQL, file paths, or mutation instructions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemovalUnit {
    pub scope: RemovalScope,
    pub id: EntityId,
    pub ownership: UnitOwnership,
    pub disposition: RemovalDisposition,
    pub residual_class: Option<ResidualClass>,
    /// Residual reconstruction risk at verification, not the unit's former role.
    pub projection_risk: ReconstructionRisk,
    pub primary_risk: ReconstructionRisk,
    pub retention_reason: Option<RetentionReason>,
    pub size_bytes: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionRemovalEvidence {
    pub scope: RemovalScope,
    pub inventory_scope: RemovalScope,
    pub inventory_completeness: Completeness,
    pub required_surfaces: Vec<EntityId>,
    pub surfaces: Vec<ProjectionSurface>,
    pub structural_profile_evidence: Option<ExactProfileStructuralEvidence>,
    pub primary_absence: RemovalObservation,
    pub canonical_catalog_absence: RemovalObservation,
    /// Covers unrelated records, keys, ordering/bytes, and projection state.
    pub collateral: Preservation,
    pub units_scope: RemovalScope,
    pub units_completeness: Completeness,
    /// Exact unit IDs frozen in the approved scope; this is evidence, not authority.
    pub approved_units: Vec<EntityId>,
    pub units: Vec<RemovalUnit>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionRemovalOutcome {
    Removed,
    RemovedWithResiduals,
    ProjectionIncomplete,
    Incomplete,
    InvalidEvidence,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MaximumSafeCleanup {
    Complete,
    Incomplete,
    Unknown,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UnitCoverage {
    pub count: usize,
    pub known_bytes: u64,
    pub unknown_size_units: usize,
}
impl UnitCoverage {
    fn add(&mut self, bytes: Option<u64>) -> Option<()> {
        self.count = self.count.checked_add(1)?;
        match bytes {
            Some(n) => self.known_bytes = self.known_bytes.checked_add(n)?,
            None => self.unknown_size_units = self.unknown_size_units.checked_add(1)?,
        }
        Some(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct RemovalCoverage {
    pub safe_attributable: UnitCoverage,
    pub planned: UnitCoverage,
    pub retained_shared: UnitCoverage,
    pub retained_ambiguous: UnitCoverage,
    pub retained_unknown: UnitCoverage,
    pub retained_unsupported: UnitCoverage,
    pub units: usize,
    pub removed_units: usize,
    pub already_absent_units: usize,
    /// Sum of known unit sizes. Unknown sizes are counted separately.
    pub known_bytes: u64,
    pub unknown_size_units: usize,
    pub removed_known_bytes: u64,
    pub removed_unknown_size_units: usize,
}
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SessionRemovalAssessment {
    pub outcome: SessionRemovalOutcome,
    pub maximum_safe_cleanup: MaximumSafeCleanup,
    pub coverage: Option<RemovalCoverage>,
    pub primary_removed: bool,
    pub projection_clean: bool,
}

/// Evaluate adapter-provided observations only; this type intentionally cannot deserialize proof.
pub fn assess_session_removal(e: &SessionRemovalEvidence) -> SessionRemovalAssessment {
    let invalid = || SessionRemovalAssessment {
        outcome: SessionRemovalOutcome::InvalidEvidence,
        maximum_safe_cleanup: MaximumSafeCleanup::Unknown,
        coverage: None,
        primary_removed: false,
        projection_clean: false,
    };
    if e.inventory_scope != e.scope || e.units_scope != e.scope {
        return invalid();
    }
    let mut surfaces = BTreeSet::new();
    for s in &e.surfaces {
        if s.scope != e.scope || !surfaces.insert(&s.id) {
            return invalid();
        }
    }
    let required: BTreeSet<_> = e.required_surfaces.iter().collect();
    if required.len() != e.required_surfaces.len() {
        return invalid();
    }
    let approved: BTreeSet<_> = e.approved_units.iter().collect();
    if approved.len() != e.approved_units.len() {
        return invalid();
    }
    let mut ids = BTreeSet::new();
    let empty = || UnitCoverage {
        count: 0,
        known_bytes: 0,
        unknown_size_units: 0,
    };
    let mut coverage = RemovalCoverage {
        safe_attributable: empty(),
        planned: empty(),
        retained_shared: empty(),
        retained_ambiguous: empty(),
        retained_unknown: empty(),
        retained_unsupported: empty(),
        units: 0,
        removed_units: 0,
        already_absent_units: 0,
        known_bytes: 0,
        unknown_size_units: 0,
        removed_known_bytes: 0,
        removed_unknown_size_units: 0,
    };
    let mut maximum = MaximumSafeCleanup::Complete;
    let mut residual = false;
    let mut projection_risk = false;
    let mut primary_risk = false;
    for unit in &e.units {
        if unit.scope != e.scope || !ids.insert(&unit.id) {
            return invalid();
        }
        // Approval membership cannot turn a protected unit into a cleanup target.
        if approved.contains(&unit.id) && unit.ownership != UnitOwnership::ExclusiveTarget {
            return invalid();
        }
        let removed = matches!(
            unit.disposition,
            RemovalDisposition::Removed | RemovalDisposition::AlreadyAbsent
        );
        if removed
            && (unit.ownership != UnitOwnership::ExclusiveTarget
                || !approved.contains(&unit.id)
                || unit.residual_class.is_some()
                || unit.projection_risk != ReconstructionRisk::None
                || unit.primary_risk != ReconstructionRisk::None
                || unit.retention_reason.is_some())
        {
            return invalid();
        }
        if !removed {
            residual = true;
            projection_risk |= unit.residual_class.is_none()
                || unit.residual_class == Some(ResidualClass::ProjectionResidual)
                || unit.projection_risk != ReconstructionRisk::None;
            primary_risk |=
                unit.residual_class.is_none() || unit.primary_risk != ReconstructionRisk::None;
            if unit.ownership == UnitOwnership::ExclusiveTarget {
                match unit.disposition {
                    RemovalDisposition::FreshSafetyFailure
                    | RemovalDisposition::RetainedNonProjecting
                        if approved.contains(&unit.id) && unit.retention_reason.is_some() => {}
                    _ => maximum = MaximumSafeCleanup::Incomplete,
                }
            }
        }
        if unit.ownership == UnitOwnership::ExclusiveTarget
            && coverage.safe_attributable.add(unit.size_bytes).is_none()
        {
            return invalid();
        }
        if approved.contains(&unit.id) && coverage.planned.add(unit.size_bytes).is_none() {
            return invalid();
        }
        if !removed {
            let category = match unit.ownership {
                UnitOwnership::Shared => Some(&mut coverage.retained_shared),
                UnitOwnership::Ambiguous => Some(&mut coverage.retained_ambiguous),
                UnitOwnership::Unknown => Some(&mut coverage.retained_unknown),
                UnitOwnership::Unsupported => Some(&mut coverage.retained_unsupported),
                UnitOwnership::ExclusiveTarget => None,
            };
            if category.is_some_and(|c| c.add(unit.size_bytes).is_none()) {
                return invalid();
            }
        }
        let Some(units) = coverage.units.checked_add(1) else {
            return invalid();
        };
        coverage.units = units;
        if unit.disposition == RemovalDisposition::Removed {
            let Some(count) = coverage.removed_units.checked_add(1) else {
                return invalid();
            };
            coverage.removed_units = count;
        } else if unit.disposition == RemovalDisposition::AlreadyAbsent {
            let Some(count) = coverage.already_absent_units.checked_add(1) else {
                return invalid();
            };
            coverage.already_absent_units = count;
        }
        match unit.size_bytes {
            Some(bytes) => {
                let Some(sum) = coverage.known_bytes.checked_add(bytes) else {
                    return invalid();
                };
                coverage.known_bytes = sum;
                if unit.disposition == RemovalDisposition::Removed {
                    let Some(sum) = coverage.removed_known_bytes.checked_add(bytes) else {
                        return invalid();
                    };
                    coverage.removed_known_bytes = sum;
                }
            }
            None => {
                let Some(count) = coverage.unknown_size_units.checked_add(1) else {
                    return invalid();
                };
                coverage.unknown_size_units = count;
                if unit.disposition == RemovalDisposition::Removed {
                    let Some(count) = coverage.removed_unknown_size_units.checked_add(1) else {
                        return invalid();
                    };
                    coverage.removed_unknown_size_units = count;
                }
            }
        }
    }
    if !approved.is_subset(&ids) {
        maximum = MaximumSafeCleanup::Incomplete;
    }
    if e.units_completeness != Completeness::Complete {
        maximum = MaximumSafeCleanup::Unknown;
    }
    let primary_complete = e.primary_absence == RemovalObservation::Absent
        && e.canonical_catalog_absence == RemovalObservation::Absent
        && !primary_risk;
    let observed_required: BTreeSet<_> = e
        .surfaces
        .iter()
        .filter(|s| s.kind == SurfaceKind::RequiredPersistentProjection)
        .map(|s| &s.id)
        .collect();
    let profile_verified = e.structural_profile_evidence.as_ref().is_some_and(|a| {
        a.profile == e.scope.profile
            && a.reconstruction_source_coverage == Completeness::Complete
            && a.collateral == Preservation::Unchanged
    });
    let projection_complete = !projection_risk
        && e.inventory_completeness == Completeness::Complete
        && !required.is_empty()
        && required == observed_required
        && profile_verified
        && e.surfaces.iter().all(|s| match s.kind {
            SurfaceKind::RequiredPersistentProjection => {
                s.observation == RemovalObservation::Absent
            }
            SurfaceKind::EphemeralUiProjection => true,
            SurfaceKind::UnknownReconstructionSurface => false,
        });
    let outcome = if !projection_complete {
        SessionRemovalOutcome::ProjectionIncomplete
    } else if !primary_complete
        || e.collateral != Preservation::Unchanged
        || maximum != MaximumSafeCleanup::Complete
    {
        SessionRemovalOutcome::Incomplete
    } else if residual {
        SessionRemovalOutcome::RemovedWithResiduals
    } else {
        SessionRemovalOutcome::Removed
    };
    SessionRemovalAssessment {
        outcome,
        maximum_safe_cleanup: maximum,
        coverage: Some(coverage),
        primary_removed: primary_complete,
        projection_clean: projection_complete,
    }
}

/// Historical reports only prove the narrower M8.2 primary cleanup contract.
pub fn assess_legacy_session_removal(
    manifest: &CleanupManifest,
    report: &CleanupReport,
) -> SessionRemovalAssessment {
    let outcome = if validate_cleanup_report(manifest, report)
        && matches!(
            report.outcome,
            CleanupOutcome::Removed | CleanupOutcome::RemovedWithResiduals
        ) {
        SessionRemovalOutcome::ProjectionIncomplete
    } else {
        SessionRemovalOutcome::Incomplete
    };
    SessionRemovalAssessment {
        outcome,
        maximum_safe_cleanup: MaximumSafeCleanup::Unknown,
        coverage: None,
        primary_removed: outcome == SessionRemovalOutcome::ProjectionIncomplete,
        projection_clean: false,
    }
}

impl SessionRemovalAssessment {
    pub fn product_complete(&self) -> bool {
        matches!(
            self.outcome,
            SessionRemovalOutcome::Removed | SessionRemovalOutcome::RemovedWithResiduals
        )
    }
}
