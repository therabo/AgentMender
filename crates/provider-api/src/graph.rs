//! Pure, provider-neutral observation wiring. Adapters supply rule interpretations.
//! This module never parses provider paths, reads files or infers ownership.
use agentmender_domain::*;
use std::{collections::BTreeMap, path::PathBuf};

fn evidence(raw: &Evidence, locator: Option<PathBuf>, association: bool) -> GraphEvidence {
    let rule = if association {
        format!("{}.association", raw.rule)
    } else {
        raw.rule.clone()
    };
    GraphEvidence {
        observation: raw.clone(),
        rule_id: EntityId::new(rule)
            .unwrap_or_else(|_| EntityId::new("unrecognized-rule").expect("static identity")),
        nature: if association {
            EvidenceNature::Inference
        } else {
            EvidenceNature::Observation
        },
        locator,
        observed_at: None,
    }
}

fn resolve(
    id: &EntityId,
    identities: &BTreeMap<EntityId, Vec<ArtifactId>>,
    builder: &mut GraphBuilder,
) -> Result<ArtifactId, GraphError> {
    if let Some(matches) = identities.get(id)
        && matches.len() == 1
    {
        return Ok(matches[0].clone());
    }
    // Missing and ambiguous references remain literal unresolved identities.
    // No first-match resolution or inferred absence is permitted.
    let node = builder.observe(NodeObservation::Unresolved { id: id.clone() });
    builder.add_root(ProtectedRoot {
        node: node.clone(),
        kind: RootKind::UnknownState,
    })?;
    Ok(node)
}

pub fn build_observation_graph(
    snapshot: &DiscoverySnapshot,
    rules: Vec<EvidenceRule>,
) -> Result<ArtifactGraph, GraphError> {
    let mut builder = ArtifactGraph::builder(GraphContext::from(snapshot));
    for rule in rules {
        builder.add_rule(rule)?;
    }
    let provider = builder.observe(NodeObservation::ProviderState);
    builder.add_root(ProtectedRoot {
        node: provider,
        kind: RootKind::ProviderGlobal,
    })?;
    let mut identities: BTreeMap<EntityId, Vec<ArtifactId>> = BTreeMap::new();
    let observations = snapshot
        .workspaces
        .iter()
        .cloned()
        .map(NodeObservation::Workspace)
        .chain(
            snapshot
                .projects
                .iter()
                .cloned()
                .map(NodeObservation::Project),
        )
        .chain(
            snapshot
                .sessions
                .iter()
                .cloned()
                .map(NodeObservation::Session),
        )
        .chain(
            snapshot
                .artifacts
                .iter()
                .cloned()
                .map(NodeObservation::Artifact),
        );
    for observation in observations {
        let (raw, locator, root) = match &observation {
            NodeObservation::Workspace(value) => (&value.evidence, value.path.clone(), None),
            NodeObservation::Project(value) => (&value.evidence, value.path.clone(), None),
            NodeObservation::Session(value) => (
                &value.evidence,
                None,
                Some(match value.state {
                    SessionState::Active => RootKind::ActiveSession,
                    SessionState::Archived => RootKind::ArchivedRetainedSession,
                    SessionState::Unknown => RootKind::UnknownState,
                }),
            ),
            NodeObservation::Artifact(value) => (
                &value.evidence,
                Some(value.locator.clone()),
                (value.kind == ArtifactKind::Unknown).then_some(RootKind::UnknownState),
            ),
            _ => unreachable!("only observed entity payloads are chained"),
        };
        let fact = evidence(raw, locator, false);
        let id = builder.observe(observation);
        builder.add_node_evidence(&id, fact)?;
        if let Some(kind) = root {
            builder.add_root(ProtectedRoot {
                node: id.clone(),
                kind,
            })?;
        }
        identities.entry(id.local_id.clone()).or_default().push(id);
    }
    for values in identities.values_mut() {
        values.sort();
        values.dedup();
    }
    for reference in &snapshot.references {
        let source = resolve(&reference.source, &identities, &mut builder)?;
        let target = resolve(&reference.target, &identities, &mut builder)?;
        builder.add_edge(ReferenceEdge {
            source,
            target,
            kind: match reference.kind {
                ReferenceKind::References => EdgeKind::References,
                ReferenceKind::SpawnedBy => EdgeKind::SpawnedBy,
                ReferenceKind::MapsTo => EdgeKind::MapsTo,
            },
            evidence: vec![evidence(&reference.evidence, None, false)],
        })?;
    }
    for session in &snapshot.sessions {
        if let Some(project) = &session.project_id {
            let source = resolve(&session.id, &identities, &mut builder)?;
            let target = resolve(project, &identities, &mut builder)?;
            builder.add_edge(ReferenceEdge {
                source,
                target,
                kind: EdgeKind::AssociatedWith,
                evidence: vec![evidence(&session.evidence, None, true)],
            })?;
        }
    }
    for project in &snapshot.projects {
        if let Some(workspace) = &project.workspace_id {
            let source = resolve(&project.id, &identities, &mut builder)?;
            let target = resolve(workspace, &identities, &mut builder)?;
            builder.add_edge(ReferenceEdge {
                source,
                target,
                kind: EdgeKind::AssociatedWith,
                evidence: vec![evidence(&project.evidence, project.path.clone(), true)],
            })?;
        }
    }
    builder.finish()
}
