//! Pure interpretation of independently captured protocol shapes, not execution authority.
use crate::capture::ExactProfile;
use agentmender_domain::{
    ArtifactGraph, CompatibilityStatus, DiscoveryScope, LifecycleCapability, NativeEffectScope,
    NativeLifecycleOperation, PlanningContext, ProviderKind,
};
use agentmender_provider_api::ProviderError;

const EVIDENCE_DIGEST: &str =
    "sha256:14a6e75b4bae9ec7b4bb41f3ed9840296f4d671295f300532efc7a7ca9b3d686";
const V165_LAYOUT: &str =
    "codex-state-5:1b88a3f88c8edb6f742d219835a821d0515b7b811b4342cd650e6ac8069f4647";

pub(super) fn planning_context(
    graph: &ArtifactGraph,
    scope: &DiscoveryScope,
) -> Result<PlanningContext, ProviderError> {
    let provider = &graph.context().provider;
    if provider.kind != ProviderKind::Codex {
        return Err(ProviderError::ProviderUnsupported(provider.kind));
    }
    let fingerprint = &provider.fingerprint;
    let profile = fingerprint
        .executable_version
        .as_deref()
        .and_then(ExactProfile::lookup);
    let matched = matches!(profile, Some(ExactProfile::V165))
        && fingerprint.interface_id.as_deref() == profile.map(ExactProfile::interface_id)
        && fingerprint.layout_id.as_deref() == Some(V165_LAYOUT)
        && fingerprint.adapter_version == env!("CARGO_PKG_VERSION")
        && provider.compatibility.status == CompatibilityStatus::ReadOnlyKnown;
    let capabilities = if matched {
        [
            (
                NativeLifecycleOperation::ArchiveSession,
                NativeEffectScope::TargetAndAttemptedDescendants,
            ),
            (
                NativeLifecycleOperation::UnarchiveSession,
                NativeEffectScope::TargetRestoreWithProviderManagedState,
            ),
            (
                NativeLifecycleOperation::DeleteSession,
                NativeEffectScope::TargetAndDescendants,
            ),
        ]
        .into_iter()
        .map(|(operation, effect_scope)| LifecycleCapability {
            provider: ProviderKind::Codex,
            fingerprint: fingerprint.clone(),
            operation,
            protocol_evidence_digest: EVIDENCE_DIGEST.into(),
            effect_scope,
        })
        .collect()
    } else {
        Vec::new()
    };
    Ok(PlanningContext {
        scope: scope.clone(),
        capabilities,
        effects: None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::CodexAdapter;
    use agentmender_domain::{CompatibilityStatus, DiscoverySnapshot};
    use agentmender_provider_api::ProviderAdapter;
    use serde_json::Value;
    use sha2::{Digest, Sha256};
    use std::path::Path;

    fn snapshot() -> DiscoverySnapshot {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../fixtures/codex/known");
        let mut snapshot = CodexAdapter::from_fixture(path)
            .unwrap()
            .discover(&DiscoveryScope::default())
            .unwrap();
        snapshot.provider.fingerprint.executable_version = Some("0.165.0-alpha.1.2".into());
        snapshot.provider.fingerprint.interface_id = Some(ExactProfile::V165.interface_id().into());
        snapshot.provider.fingerprint.layout_id = Some(V165_LAYOUT.into());
        snapshot.provider.fingerprint.adapter_version = env!("CARGO_PKG_VERSION").into();
        snapshot.provider.compatibility.status = CompatibilityStatus::ReadOnlyKnown;
        snapshot
    }
    fn context(snapshot: &DiscoverySnapshot) -> Result<PlanningContext, ProviderError> {
        let graph = crate::graph::build_graph(snapshot)?;
        planning_context(&graph, &DiscoveryScope::default())
    }
    #[test]
    fn descriptors_require_every_exact_profile_field_and_never_supply_effect_closure() {
        let s = snapshot();
        let result = context(&s).unwrap();
        assert_eq!(result.capabilities.len(), 3);
        assert!(result.effects.is_none());
        assert_eq!(
            result
                .capabilities
                .iter()
                .map(|c| (c.operation, c.effect_scope))
                .collect::<Vec<_>>(),
            vec![
                (
                    NativeLifecycleOperation::ArchiveSession,
                    NativeEffectScope::TargetAndAttemptedDescendants
                ),
                (
                    NativeLifecycleOperation::UnarchiveSession,
                    NativeEffectScope::TargetRestoreWithProviderManagedState
                ),
                (
                    NativeLifecycleOperation::DeleteSession,
                    NativeEffectScope::TargetAndDescendants
                ),
            ]
        );
        let mut variants = Vec::new();
        let mut v = s.clone();
        v.provider.fingerprint.executable_version = Some("0.164.0-alpha.11.1".into());
        variants.push(v);
        let mut v = s.clone();
        v.provider.fingerprint.executable_version = Some("unknown".into());
        variants.push(v);
        let mut v = s.clone();
        v.provider.fingerprint.interface_id = Some("mismatch".into());
        variants.push(v);
        let mut v = s.clone();
        v.provider.fingerprint.layout_id = Some("mismatch".into());
        variants.push(v);
        let mut v = s.clone();
        v.provider.fingerprint.adapter_version = "mismatch".into();
        variants.push(v);
        let mut v = s.clone();
        v.provider.compatibility.status = CompatibilityStatus::AuditOnlyUnknown;
        variants.push(v);
        for v in variants {
            assert!(context(&v).unwrap().capabilities.is_empty());
        }
        let mut foreign = s;
        foreign.provider.kind = ProviderKind::Claude;
        // Test the hook itself with a foreign graph, independently of graph construction.
        let foreign_graph = agentmender_domain::ArtifactGraph::builder(
            agentmender_domain::GraphContext::from(&foreign),
        )
        .finish()
        .unwrap();
        assert!(planning_context(&foreign_graph, &DiscoveryScope::default()).is_err());
    }
    fn hex(bytes: &[u8]) -> String {
        bytes.iter().map(|byte| format!("{byte:02x}")).collect()
    }
    #[test]
    fn captured_protocol_evidence_matches_files_and_method_shapes() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../compatibility/codex/0.165.0-alpha.1.2/lifecycle");
        let summary: Value = serde_json::from_slice(
            &std::fs::read(root.join("lifecycle-evidence-summary.json")).unwrap(),
        )
        .unwrap();
        let evidence = &summary["protocol_evidence"];
        let digest = format!(
            "sha256:{}",
            hex(&Sha256::digest(serde_json::to_vec(evidence).unwrap()))
        );
        assert_eq!(digest, EVIDENCE_DIGEST);
        for entry in evidence["files"].as_array().unwrap() {
            let bytes = std::fs::read(root.join(entry["path"].as_str().unwrap())).unwrap();
            assert_eq!(
                hex(&Sha256::digest(bytes)),
                entry["sha256"].as_str().unwrap()
            );
        }
        for (file, pairs) in [
            (
                "ClientRequest.json",
                [
                    ("thread/archive", "ThreadArchiveParams"),
                    ("thread/unarchive", "ThreadUnarchiveParams"),
                    ("thread/delete", "ThreadDeleteParams"),
                ],
            ),
            (
                "ServerNotification.json",
                [
                    ("thread/archived", "ThreadArchivedNotification"),
                    ("thread/unarchived", "ThreadUnarchivedNotification"),
                    ("thread/deleted", "ThreadDeletedNotification"),
                ],
            ),
        ] {
            let schema: Value =
                serde_json::from_slice(&std::fs::read(root.join(file)).unwrap()).unwrap();
            for (method, shape) in pairs {
                assert!(schema["oneOf"].as_array().unwrap().iter().any(
                    |v| v["properties"]["method"]["enum"] == serde_json::json!([method])
                        && v["properties"]["params"]["$ref"] == format!("#/definitions/{shape}")
                ));
                let standalone: Value = serde_json::from_slice(
                    &std::fs::read(root.join(format!("v2/{shape}.json"))).unwrap(),
                )
                .unwrap();
                assert_eq!(standalone["title"], shape);
            }
        }
    }
}
