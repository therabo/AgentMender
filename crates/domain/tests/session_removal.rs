use agentmender_domain::*;

fn id(s: &str) -> EntityId {
    EntityId::new(s).unwrap()
}
fn witness() -> SessionRemovalEvidence {
    let scope = RemovalScope {
        target: id("target"),
        root: id("root"),
        profile: id("profile"),
    };
    SessionRemovalEvidence {
        inventory_scope: scope.clone(),
        units_scope: scope.clone(),
        scope: scope.clone(),
        inventory_completeness: Completeness::Complete,
        required_surfaces: vec![id("catalog")],
        surfaces: vec![ProjectionSurface {
            scope: scope.clone(),
            id: id("catalog"),
            kind: SurfaceKind::RequiredPersistentProjection,
            observation: RemovalObservation::Absent,
        }],
        structural_profile_evidence: Some(ExactProfileStructuralEvidence {
            profile: scope.profile.clone(),
            reconstruction_source_coverage: Completeness::Complete,
            collateral: Preservation::Unchanged,
        }),
        primary_absence: RemovalObservation::Absent,
        canonical_catalog_absence: RemovalObservation::Absent,
        collateral: Preservation::Unchanged,
        units_completeness: Completeness::Complete,
        approved_units: vec![id("target-record")],
        units: vec![RemovalUnit {
            scope,
            id: id("target-record"),
            ownership: UnitOwnership::ExclusiveTarget,
            disposition: RemovalDisposition::Removed,
            residual_class: None,
            projection_risk: ReconstructionRisk::None,
            primary_risk: ReconstructionRisk::None,
            retention_reason: None,
            size_bytes: Some(12),
        }],
    }
}
fn outcome(e: &SessionRemovalEvidence) -> SessionRemovalOutcome {
    assess_session_removal(e).outcome
}
fn success(o: SessionRemovalOutcome) -> bool {
    matches!(
        o,
        SessionRemovalOutcome::Removed | SessionRemovalOutcome::RemovedWithResiduals
    )
}

#[test]
fn complete_synthetic_witness_and_exact_record_preserve_collateral() {
    let e = witness();
    assert_eq!(outcome(&e), SessionRemovalOutcome::Removed);
    assert_eq!(
        assess_session_removal(&e).maximum_safe_cleanup,
        MaximumSafeCleanup::Complete
    );
}
#[test]
fn persistent_sources_and_exact_structural_profile_fail_closed() {
    for observation in [RemovalObservation::Present, RemovalObservation::Unknown] {
        let mut e = witness();
        e.surfaces[0].observation = observation;
        assert_eq!(outcome(&e), SessionRemovalOutcome::ProjectionIncomplete);
    }
    let mut e = witness();
    e.structural_profile_evidence = None;
    assert_eq!(outcome(&e), SessionRemovalOutcome::ProjectionIncomplete);
    let mut e = witness();
    e.structural_profile_evidence.as_mut().unwrap().profile = id("wrong-profile");
    assert_eq!(outcome(&e), SessionRemovalOutcome::ProjectionIncomplete);
    let mut e = witness();
    e.structural_profile_evidence
        .as_mut()
        .unwrap()
        .reconstruction_source_coverage = Completeness::Partial;
    assert_eq!(outcome(&e), SessionRemovalOutcome::ProjectionIncomplete);
    let mut e = witness();
    e.surfaces.push(ProjectionSurface {
        scope: e.scope.clone(),
        id: id("unknown"),
        kind: SurfaceKind::UnknownReconstructionSurface,
        observation: RemovalObservation::Absent,
    });
    assert_eq!(outcome(&e), SessionRemovalOutcome::ProjectionIncomplete);
    for observation in [
        RemovalObservation::Absent,
        RemovalObservation::Present,
        RemovalObservation::Unknown,
    ] {
        let mut e = witness();
        e.surfaces.push(ProjectionSurface {
            scope: e.scope.clone(),
            id: id("live-ui"),
            kind: SurfaceKind::EphemeralUiProjection,
            observation,
        });
        assert_eq!(outcome(&e), SessionRemovalOutcome::Removed);
    }
    let mut e = witness();
    e.surfaces.clear();
    assert!(!success(outcome(&e)));
    let mut e = witness();
    e.required_surfaces.push(id("another-required"));
    assert!(!success(outcome(&e)));
    let mut e = witness();
    e.inventory_completeness = Completeness::Partial;
    assert!(!success(outcome(&e)));
}
#[test]
fn residual_classes_and_maximum_cleanup_are_independent() {
    let mut e = witness();
    let u = &mut e.units[0];
    u.disposition = RemovalDisposition::RetainedNonProjecting;
    u.retention_reason = Some(RetentionReason::FreshIdentityChanged);
    u.residual_class = Some(ResidualClass::SafeTechnicalResidual);
    assert_eq!(outcome(&e), SessionRemovalOutcome::RemovedWithResiduals);
    for risk in [None, Some(ResidualClass::ProjectionResidual)] {
        e.units[0].residual_class = risk;
        assert!(!success(outcome(&e)));
    }
    e.units[0].residual_class = Some(ResidualClass::UnknownTechnicalResidual);
    assert_eq!(outcome(&e), SessionRemovalOutcome::RemovedWithResiduals);
    e.units[0].primary_risk = ReconstructionRisk::Unknown;
    assert!(!success(outcome(&e)));
    e.units[0].primary_risk = ReconstructionRisk::None;
    e.units[0].residual_class = Some(ResidualClass::SafeTechnicalResidual);
    e.units[0].disposition = RemovalDisposition::Skipped;
    assert!(!success(outcome(&e)));
    assert_eq!(
        assess_session_removal(&e).maximum_safe_cleanup,
        MaximumSafeCleanup::Incomplete
    );
}
#[test]
fn destructive_actions_must_be_approved_exact_exclusive_units() {
    for ownership in [
        UnitOwnership::Shared,
        UnitOwnership::Ambiguous,
        UnitOwnership::Unknown,
        UnitOwnership::Unsupported,
    ] {
        let mut e = witness();
        e.units[0].ownership = ownership;
        assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    }
    let mut e = witness();
    e.approved_units.clear();
    assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    let mut e = witness();
    e.units[0].scope.target = id("other-session");
    assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    let mut e = witness();
    let mut newly_discovered = e.units[0].clone();
    newly_discovered.id = id("new");

    newly_discovered.disposition = RemovalDisposition::Skipped;
    newly_discovered.residual_class = Some(ResidualClass::SafeTechnicalResidual);
    e.units.push(newly_discovered);
    assert!(!success(outcome(&e)));
}
#[test]
fn collateral_and_primary_uncertainty_never_succeed() {
    for preservation in [Preservation::Changed, Preservation::Unknown] {
        let mut e = witness();
        e.collateral = preservation;
        assert!(!success(outcome(&e)));
    }
    for observation in [RemovalObservation::Present, RemovalObservation::Unknown] {
        let mut e = witness();
        e.primary_absence = observation;
        assert!(!success(outcome(&e)));
        let mut e = witness();
        e.canonical_catalog_absence = observation;
        assert!(!success(outcome(&e)));
    }
    let mut e = witness();
    e.units_completeness = Completeness::Partial;
    assert_eq!(
        assess_session_removal(&e).maximum_safe_cleanup,
        MaximumSafeCleanup::Unknown
    );
    assert!(!success(outcome(&e)));
}
#[test]
fn coverage_is_exact_permutation_invariant_and_fails_closed() {
    let mut e = witness();
    let mut u = e.units[0].clone();
    u.id = id("two");
    u.size_bytes = None;
    e.approved_units.push(u.id.clone());
    e.units.push(u);
    let a = assess_session_removal(&e);
    let c = a.coverage.unwrap();
    assert_eq!(
        (
            c.units,
            c.removed_units,
            c.known_bytes,
            c.unknown_size_units,
            c.removed_known_bytes,
            c.removed_unknown_size_units
        ),
        (2, 2, 12, 1, 12, 1)
    );
    e.units.reverse();
    assert_eq!(assess_session_removal(&e).coverage.unwrap(), c);
    e.units[1].id = e.units[0].id.clone();
    assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    e.units[1].id = id("different");
    e.units[0].size_bytes = Some(u64::MAX);
    assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
}
#[test]
fn scope_and_duplicate_surfaces_are_rejected() {
    let mut e = witness();
    e.inventory_scope.root = id("foreign");
    assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    let mut e = witness();
    e.surfaces.push(e.surfaces[0].clone());
    assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    let mut e = witness();
    e.surfaces[0].scope.profile = id("foreign");
    assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
}

#[test]
fn removed_units_cannot_hide_reconstruction_risk_or_retention() {
    for disposition in [
        RemovalDisposition::Removed,
        RemovalDisposition::AlreadyAbsent,
    ] {
        for risk in [ReconstructionRisk::Present, ReconstructionRisk::Unknown] {
            let mut e = witness();
            e.units[0].disposition = disposition;
            e.units[0].projection_risk = risk;
            assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
            e.units[0].projection_risk = ReconstructionRisk::None;
            e.units[0].primary_risk = risk;
            assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
        }
        let mut e = witness();
        e.units[0].disposition = disposition;
        e.units[0].retention_reason = Some(RetentionReason::FreshIdentityChanged);
        assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    }
}

#[test]
fn reconstruction_risk_is_reflected_in_each_product_predicate() {
    let mut e = witness();
    e.units[0].disposition = RemovalDisposition::FreshSafetyFailure;
    e.units[0].retention_reason = Some(RetentionReason::FreshIdentityChanged);
    e.units[0].residual_class = Some(ResidualClass::UnknownTechnicalResidual);
    for risk in [ReconstructionRisk::Present, ReconstructionRisk::Unknown] {
        e.units[0].projection_risk = risk;
        let a = assess_session_removal(&e);
        assert!(!a.projection_clean);
        assert!(a.primary_removed);
        assert!(!a.product_complete());
        e.units[0].projection_risk = ReconstructionRisk::None;
        e.units[0].primary_risk = risk;
        let a = assess_session_removal(&e);
        assert!(a.projection_clean);
        assert!(!a.primary_removed);
        assert!(!a.product_complete());
        e.units[0].primary_risk = ReconstructionRisk::None;
    }
    e.units[0].residual_class = Some(ResidualClass::ProjectionResidual);
    assert!(!assess_session_removal(&e).projection_clean);
}

#[test]
fn approved_protected_units_and_already_absent_accounting_are_explicit() {
    for ownership in [
        UnitOwnership::Shared,
        UnitOwnership::Ambiguous,
        UnitOwnership::Unknown,
        UnitOwnership::Unsupported,
    ] {
        let mut e = witness();
        e.units[0].ownership = ownership;
        e.units[0].disposition = RemovalDisposition::RetainedNonProjecting;
        e.units[0].residual_class = Some(ResidualClass::SafeTechnicalResidual);
        assert_eq!(outcome(&e), SessionRemovalOutcome::InvalidEvidence);
    }
    let mut e = witness();
    e.units[0].disposition = RemovalDisposition::AlreadyAbsent;
    let a = assess_session_removal(&e);
    assert!(a.product_complete());
    let c = a.coverage.unwrap();
    assert_eq!(c.removed_units, 0);
    assert_eq!(c.already_absent_units, 1);
    assert_eq!(c.removed_known_bytes, 0);
}

// Deterministic generation exercises the contract across distinct exact unit IDs and sizes.
#[test]
fn fixed_seed_contract_properties() {
    let mut seed = 0x4d38_3231_5f76_3300u64;
    for _ in 0..256 {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1);
        let mut base = witness();
        base.units[0].id = id(&format!("unit-{seed}"));
        base.approved_units[0] = base.units[0].id.clone();
        base.units[0].size_bytes = Some(seed % 1_000_000);
        assert_eq!(outcome(&base), SessionRemovalOutcome::Removed);
        for case in 0..13 {
            let mut e = base.clone();
            match case {
                0 => e.surfaces[0].observation = RemovalObservation::Present,
                1 => e.surfaces[0].observation = RemovalObservation::Unknown,
                2 => {
                    e.structural_profile_evidence
                        .as_mut()
                        .unwrap()
                        .reconstruction_source_coverage = Completeness::Partial
                }
                3 => {
                    e.structural_profile_evidence.as_mut().unwrap().collateral =
                        Preservation::Unknown
                }
                4 => e.primary_absence = RemovalObservation::Unknown,
                5 => e.canonical_catalog_absence = RemovalObservation::Unknown,
                6 => e.collateral = Preservation::Changed,
                7 => e.units[0].ownership = UnitOwnership::Shared,
                8 => e.units[0].ownership = UnitOwnership::Unknown,
                9 => e.units[0].scope.target = id("other-session"),
                10 => e.approved_units.clear(),
                11 => {
                    e.units[0].disposition = RemovalDisposition::Skipped;
                    e.units[0].residual_class = Some(ResidualClass::SafeTechnicalResidual);
                }
                _ => {
                    e.units[0].disposition = RemovalDisposition::RetainedNonProjecting;
                    e.units[0].residual_class = Some(ResidualClass::UnknownTechnicalResidual);
                    e.units[0].primary_risk = ReconstructionRisk::Unknown;
                }
            }
            assert!(!success(outcome(&e)), "case {case}, seed {seed}");
        }
    }
}
