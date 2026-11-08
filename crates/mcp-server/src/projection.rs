//! Ephemeral keyed identities and bounded presentation pages.
use crate::dto::*;
use agentmender_application::{ReadOnlyResponse, TargetSelector};
use agentmender_domain as d;
use hmac::{Hmac, KeyInit, Mac};
use serde::Serialize;
use sha2::Sha256;
use std::sync::Arc;

type HmacSha256 = Hmac<Sha256>;
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectionError {
    InvalidCursor,
    StaleCursor,
    ResponseTooLarge,
}
#[derive(Clone)]
pub struct Projector {
    key: [u8; 32],
}
fn category<T: Serialize>(v: &T) -> Category {
    serde_json::from_value(serde_json::to_value(v).expect("enum serialization"))
        .expect("closed domain category")
}
fn known_version(v: &str) -> Option<String> {
    matches!(
        v,
        "0.164.0-alpha.11.1" | "0.165.0-alpha.1.2" | "2.1.311" | "4.29.0" | "2.1.222"
    )
    .then(|| v.to_owned())
}
fn known_interface(v: &str) -> Option<String> {
    matches!(
        v,
        "codex-app-server-0.164.0-alpha.11.1" | "codex-app-server-0.165.0-alpha.1.2"
    )
    .then(|| v.to_owned())
}
fn known_layout(v: &str) -> Option<String> {
    (v == "claude-jsonl-2.1.311-synthetic-v1").then(|| v.to_owned())
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
fn unhex(s: &str) -> Option<Vec<u8>> {
    if !s.len().is_multiple_of(2) || !s.is_ascii() {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}
impl Projector {
    pub fn new(key: [u8; 32]) -> Self {
        Self { key }
    }
    pub(crate) fn seal_search<T: Serialize>(&self, kind: &[u8], value: &T) -> String {
        let data = serde_json::to_vec(value).expect("bounded search token serializes");
        let mut signed = b"agentmender/search-token/v1\0".to_vec();
        signed.extend_from_slice(kind);
        signed.extend_from_slice(&data);
        format!(
            "{}.{}",
            hex(&data),
            hex(&self.mac(b"agentmender/search-token/v1\0", &signed))
        )
    }
    pub(crate) fn open_search<T: serde::de::DeserializeOwned>(
        &self,
        kind: &[u8],
        token: &str,
    ) -> Option<T> {
        if token.len() > 4096 || !token.is_ascii() {
            return None;
        }
        let (data, tag) = token.split_once('.')?;
        let data = unhex(data)?;
        let tag = unhex(tag)?;
        let mut signed = b"agentmender/search-token/v1\0".to_vec();
        signed.extend_from_slice(kind);
        signed.extend_from_slice(&data);
        let mut mac = <HmacSha256 as KeyInit>::new_from_slice(&self.key).ok()?;
        mac.update(b"agentmender/search-token/v1\0");
        mac.update(&signed);
        mac.verify_slice(&tag).ok()?;
        serde_json::from_slice(&data).ok()
    }
    fn mac(&self, domain: &[u8], data: &[u8]) -> Vec<u8> {
        let mut m = <HmacSha256 as KeyInit>::new_from_slice(&self.key).expect("HMAC key");
        m.update(domain);
        m.update(data);
        m.finalize().into_bytes().to_vec()
    }
    fn opaque<T: Serialize>(&self, domain: &[u8], x: &T) -> String {
        hex(&self.mac(
            domain,
            &serde_json::to_vec(x).expect("domain serialization"),
        ))
    }
    pub(crate) fn namespace_ref(&self, id: &d::ArtifactId) -> String {
        let mut namespace = id.clone();
        namespace.class = d::GraphNodeKind::ProviderState;
        namespace.local_id = d::EntityId::new("provider-state").expect("fixed ID");
        self.entity_ref(&namespace).token
    }
    fn local_ref(&self, id: &d::ArtifactId) -> String {
        let mut scoped = id.clone();
        scoped.class = d::GraphNodeKind::Unresolved;
        self.opaque(b"agentmender/scan-local-id/v1\0", &scoped)
    }
    pub fn entity_ref(&self, id: &d::ArtifactId) -> EntityRef {
        EntityRef {
            provider: id.provider.into(),
            class: id.class.into(),
            token: self.opaque(b"agentmender/entity/v1\0", id),
        }
    }
    pub fn validate_cursor(&self, cursor: &str) -> bool {
        if cursor.len() > 256 || !cursor.is_ascii() {
            return false;
        }
        let mut p = cursor.split('.');
        let (Some(o), Some(d), Some(m), None) = (p.next(), p.next(), p.next(), p.next()) else {
            return false;
        };
        if o.parse::<usize>().is_err() || d.len() != 64 || unhex(d).is_none() {
            return false;
        }
        let Some(sig) = unhex(m) else { return false };
        let mut mac = <HmacSha256 as KeyInit>::new_from_slice(&self.key).expect("HMAC key");
        mac.update(b"agentmender/cursor/v1\0");
        mac.update(format!("{o}.{d}").as_bytes());
        mac.verify_slice(&sig).is_ok()
    }
    pub fn selector(&self, target: &EntityRef) -> TargetSelector {
        let this = self.clone();
        let target = target.clone();
        Arc::new(move |id| this.entity_ref(id) == target)
    }
    fn evidence(&self, e: &d::GraphEvidence) -> EvidenceView {
        EvidenceView {
            known_version: e
                .observation
                .provider_version
                .as_deref()
                .and_then(known_version),
            version_ref: e
                .observation
                .provider_version
                .as_ref()
                .map(|v| self.opaque(b"agentmender/version/v1\0", v)),
            raw_rule_ref: self.opaque(b"agentmender/raw-rule/v1\0", &e.observation.rule),
            effective_confidence: None,
            fact_ref: self.opaque(b"agentmender/evidence/v1\0", e),
            source: category(&e.observation.source),
            confidence: category(&e.observation.confidence),
            nature: category(&e.nature),
            rule_ref: self.opaque(b"agentmender/rule/v1\0", &e.rule_id),
            locator_present: e.locator.is_some(),
            timestamp_present: e.observed_at.is_some(),
        }
    }
    fn evaluated(&self, e: &d::EvaluatedEvidence) -> EvidenceView {
        let mut view = self.evidence(&e.evidence);
        view.effective_confidence = Some(category(&e.confidence));
        view
    }
    fn explanation_evidence(
        &self,
        raw: &d::GraphEvidence,
        evaluated: &[d::EvaluatedEvidence],
    ) -> EvidenceView {
        let mut view = self.evidence(raw);
        if let Some(e) = evaluated
            .iter()
            .find(|e| self.opaque(b"agentmender/evidence/v1\0", &e.evidence) == view.fact_ref)
        {
            view.effective_confidence = Some(category(&e.confidence));
        }
        view
    }
    fn raw_evidence(&self, e: &d::Evidence) -> RawEvidenceView {
        RawEvidenceView {
            known_version: e.provider_version.as_deref().and_then(known_version),
            fact_ref: self.opaque(b"agentmender/raw-evidence/v1\0", e),
            source: category(&e.source),
            confidence: category(&e.confidence),
            version_ref: e
                .provider_version
                .as_ref()
                .map(|v| self.opaque(b"agentmender/version/v1\0", v)),
            rule_ref: self.opaque(b"agentmender/raw-rule/v1\0", &e.rule),
        }
    }
    fn fingerprint(&self, f: &d::ProviderFingerprint) -> FingerprintView {
        FingerprintView {
            known_version: f.executable_version.as_deref().and_then(known_version),
            known_interface: f.interface_id.as_deref().and_then(known_interface),
            known_layout: f.layout_id.as_deref().and_then(known_layout),
            reference: self.opaque(b"agentmender/fingerprint/v1\0", f),
            executable_present: f.executable.is_some(),
            version_ref: f
                .executable_version
                .as_ref()
                .map(|v| self.opaque(b"agentmender/version/v1\0", v)),
            adapter_ref: self.opaque(b"agentmender/adapter/v1\0", &f.adapter_version),
            interface_ref: f
                .interface_id
                .as_ref()
                .map(|v| self.opaque(b"agentmender/interface/v1\0", v)),
            layout_ref: f
                .layout_id
                .as_ref()
                .map(|v| self.opaque(b"agentmender/layout/v1\0", v)),
        }
    }
    fn node(&self, n: &d::ArtifactNode) -> NodeView {
        NodeView {
            namespace_ref: self.namespace_ref(&n.id),
            ownership: None,
            ownership_blockers: vec![],
            ownership_evidence: vec![],
            local_ref: self.local_ref(&n.id),
            id: self.entity_ref(&n.id),
            observation_count: n.observations.len(),
            conflicting: n.observations.len() > 1,
            protected: n.is_protected(),
            observations: n
                .observations
                .iter()
                .map(|o| match o {
                    d::NodeObservation::ProviderState => ObservationView {
                        path_present: false,
                        parent_ref: None,
                        timestamp_present: false,
                        raw_evidence: None,
                        fact_ref: self.opaque(b"agentmender/observation/v1\0", o),
                        kind: "provider_state".into(),
                        state: None,
                        artifact_kind: None,
                        size_bytes: None,
                    },
                    d::NodeObservation::Workspace(x) => ObservationView {
                        path_present: x.path.is_some(),
                        parent_ref: None,
                        timestamp_present: false,
                        raw_evidence: Some(self.raw_evidence(&x.evidence)),
                        fact_ref: self.opaque(b"agentmender/observation/v1\0", o),
                        kind: "workspace".into(),
                        state: None,
                        artifact_kind: None,
                        size_bytes: None,
                    },
                    d::NodeObservation::Project(x) => ObservationView {
                        path_present: x.path.is_some(),
                        parent_ref: x.workspace_id.as_ref().map(|i| {
                            let mut parent = n.id.clone();
                            parent.local_id = i.clone();
                            self.local_ref(&parent)
                        }),
                        timestamp_present: false,
                        raw_evidence: Some(self.raw_evidence(&x.evidence)),
                        fact_ref: self.opaque(b"agentmender/observation/v1\0", o),
                        kind: "project".into(),
                        state: None,
                        artifact_kind: None,
                        size_bytes: None,
                    },
                    d::NodeObservation::Session(s) => ObservationView {
                        path_present: false,
                        parent_ref: s.project_id.as_ref().map(|i| {
                            let mut parent = n.id.clone();
                            parent.local_id = i.clone();
                            self.local_ref(&parent)
                        }),
                        timestamp_present: s.last_active.is_some(),
                        raw_evidence: Some(self.raw_evidence(&s.evidence)),
                        fact_ref: self.opaque(b"agentmender/observation/v1\0", o),
                        kind: "session".into(),
                        state: Some(category(&s.state)),
                        artifact_kind: None,
                        size_bytes: None,
                    },
                    d::NodeObservation::Artifact(a) => ObservationView {
                        path_present: true,
                        parent_ref: None,
                        timestamp_present: false,
                        raw_evidence: Some(self.raw_evidence(&a.evidence)),
                        fact_ref: self.opaque(b"agentmender/observation/v1\0", o),
                        kind: "artifact".into(),
                        state: None,
                        artifact_kind: Some(category(&a.kind)),
                        size_bytes: a.size_bytes,
                    },
                    d::NodeObservation::Unresolved { .. } => ObservationView {
                        path_present: false,
                        parent_ref: None,
                        timestamp_present: false,
                        raw_evidence: None,
                        fact_ref: self.opaque(b"agentmender/observation/v1\0", o),
                        kind: "unresolved".into(),
                        state: None,
                        artifact_kind: None,
                        size_bytes: None,
                    },
                })
                .collect(),
            evidence: n.evidence.iter().map(|e| self.evidence(e)).collect(),
        }
    }
    fn edge(&self, e: &d::ReferenceEdge) -> EdgeView {
        EdgeView {
            namespace_ref: self.namespace_ref(&e.source),
            source: self.entity_ref(&e.source),
            target: self.entity_ref(&e.target),
            kind: category(&e.kind),
            evidence: e.evidence.iter().map(|x| self.evidence(x)).collect(),
        }
    }
    fn context(
        &self,
        p: &d::Provider,
        c: Option<d::Completeness>,
        e: d::EvidenceStatus,
        diagnostics: &[d::Diagnostic],
    ) -> ContextView {
        ContextView {
            namespace_ref: self
                .entity_ref(&d::ArtifactId {
                    provider: p.kind,
                    state_root: p.state_root.clone(),
                    class: d::GraphNodeKind::ProviderState,
                    local_id: d::EntityId::new("provider-state").expect("fixed ID"),
                })
                .token,
            coverage_ref: None,
            fingerprint: self.fingerprint(&p.fingerprint),
            provider_evidence_status: category(&p.evidence_status),
            interface_stability: category(&p.compatibility.interface_stability),
            capabilities: p.capabilities.iter().map(category).collect(),
            installations: p
                .installations
                .iter()
                .map(|i| InstallationView {
                    fact_ref: self.opaque(b"agentmender/installation/v1\0", i),
                    known_version: i.version.as_deref().and_then(known_version),
                    surface: category(&i.surface),
                    executable_present: i.executable.is_some(),
                    version_ref: i
                        .version
                        .as_ref()
                        .map(|v| self.opaque(b"agentmender/version/v1\0", v)),
                    version_source: category(&i.version_source),
                })
                .collect(),
            provider: p.kind.into(),
            presence: category(&p.presence),
            compatibility: category(&p.compatibility.status),
            evidence_status: category(&e),
            completeness: c.map(|x| category(&x)),
            diagnostics: p
                .diagnostics
                .iter()
                .chain(diagnostics)
                .map(|x| category(&x.code))
                .collect(),
        }
    }
    fn graph_context(&self, c: &d::GraphContext) -> ContextView {
        let mut v = self.context(
            &c.provider,
            Some(c.completeness),
            c.evidence_status,
            &c.diagnostics,
        );
        v.coverage_ref = Some(self.opaque(b"agentmender/coverage/v1\0", &c.coverage));
        v
    }
    fn rule(&self, r: &d::EvidenceRule) -> RuleView {
        let raw = r.id.as_str();
        let base = raw.strip_suffix(".association").unwrap_or(raw);
        let recognized = match r.provider {
            d::ProviderKind::Codex => matches!(
                base,
                "official_thread_list_metadata_use_state_db_only"
                    | "cwd_group_without_native_project_assignment"
                    | "protected_regular_file_metadata_only"
            ),
            d::ProviderKind::Claude => matches!(
                base,
                "explicit_cliSessionId"
                    | "explicit_forkedFromSessionId"
                    | "explicit_desktop_cwd"
                    | "exact_fields_observed_with_desktop_4_29_unknown_producer"
                    | "explicit_cwd_workspace_observation_no_existence_claim"
                    | "explicit_transcript_cwd_not_decoded_bucket"
                    | "synthetic_explicit_session_metadata"
                    | "explicit_sidecar_parentAgentId"
                    | "explicit_subagent_sessionId_groups_root"
                    | "regular_file_metadata_protected"
                    | "desktop_regular_file_metadata_protected"
            ),
        };
        RuleView {
            known_id: recognized.then(|| raw.to_owned()),
            descriptor_ref: self.opaque(b"agentmender/rule-descriptor/v1\0", r),
            reference: self.opaque(b"agentmender/rule/v1\0", &r.id),
            recognized,
            sources: r.admissible_sources.iter().map(category).collect(),
            max_confidence: category(&r.max_confidence),
        }
    }
    fn diagnostic(&self, d: &d::GraphDiagnostic) -> Record {
        use d::GraphDiagnostic as D;
        let (code, node, rule) = match d {
            D::ConflictingNodeObservations { node } => {
                ("conflicting_node_observations", Some(node), None)
            }
            D::ConflictingRuleDefinitions { rule } => {
                ("conflicting_rule_definitions", None, Some(rule))
            }
            D::ConflictingParentage { child } => ("conflicting_parentage", Some(child), None),
            D::UnrecognizedRule { rule } => ("unrecognized_rule", None, Some(rule)),
            D::InadmissibleSource { rule } => ("inadmissible_source", None, Some(rule)),
            D::UnresolvedEndpoint { node } => ("unresolved_endpoint", Some(node), None),
        };
        Record::Diagnostic {
            code: code.into(),
            node: node.map(|n| self.entity_ref(n)),
            rule_ref: rule.map(|r| self.opaque(b"agentmender/rule/v1\0", r)),
        }
    }
    fn graph_node(&self, g: &d::ArtifactGraph, n: &d::ArtifactNode) -> NodeView {
        let mut view = self.node(n);
        for (out, e) in view.evidence.iter_mut().zip(&n.evidence) {
            out.effective_confidence = Some(category(&g.effective_confidence(e)));
        }
        let ownership = g.ownership_assessment(&n.id).expect("existing graph node");
        view.ownership = Some(category(&ownership.classification));
        view.ownership_blockers = ownership.blockers.iter().map(category).collect();
        view.ownership_evidence = ownership
            .supporting_evidence
            .iter()
            .map(|e| {
                let mut v = self.evidence(e);
                v.effective_confidence = Some(category(&g.effective_confidence(e)));
                v
            })
            .collect();
        view
    }
    fn graph_edge(&self, g: &d::ArtifactGraph, e: &d::ReferenceEdge) -> EdgeView {
        let mut v = self.edge(e);
        for (out, raw) in v.evidence.iter_mut().zip(&e.evidence) {
            out.effective_confidence = Some(category(&g.effective_confidence(raw)));
        }
        v
    }
    fn scoped_ref(&self, namespace: &str, value: &str) -> String {
        self.opaque(b"agentmender/scoped-rule/v1\0", &(namespace, value))
    }
    fn scope_evidence(&self, namespace: &str, values: &mut [EvidenceView]) {
        for e in values {
            e.rule_ref = self.scoped_ref(namespace, &e.rule_ref);
            e.raw_rule_ref = self.scoped_ref(namespace, &e.raw_rule_ref);
        }
    }
    fn scope_observations(&self, namespace: &str, values: &mut [ObservationView]) {
        for o in values {
            if let Some(e) = &mut o.raw_evidence {
                e.rule_ref = self.scoped_ref(namespace, &e.rule_ref);
            }
        }
    }
    fn scope_node(&self, namespace: &str, n: &mut NodeView) {
        self.scope_evidence(namespace, &mut n.evidence);
        self.scope_evidence(namespace, &mut n.ownership_evidence);
        self.scope_observations(namespace, &mut n.observations);
    }
    fn scope_record(&self, namespace: &str, record: &mut Record) {
        match record {
            Record::Node { node } | Record::ScanEntity { node } => self.scope_node(namespace, node),
            Record::Edge { edge } => self.scope_evidence(namespace, &mut edge.evidence),
            Record::Rule { rule } => {
                rule.reference = self.scoped_ref(namespace, &rule.reference);
                rule.descriptor_ref = self.scoped_ref(namespace, &rule.descriptor_ref);
            }
            Record::Diagnostic { rule_ref, .. } => {
                if let Some(r) = rule_ref {
                    *r = self.scoped_ref(namespace, r);
                }
            }
            Record::OwnerRelation { rule_refs, .. } => {
                for r in rule_refs {
                    *r = self.scoped_ref(namespace, r);
                }
            }
            Record::Contract { evidence, .. }
            | Record::RelationshipCoverage { evidence, .. }
            | Record::Proof { evidence, .. }
            | Record::Residual { evidence, .. }
            | Record::PlannedArtifact { evidence, .. } => self.scope_evidence(namespace, evidence),
            Record::Finding {
                rule_refs,
                observations,
                relationships,
                evidence,
                ..
            } => {
                for r in rule_refs {
                    *r = self.scoped_ref(namespace, r);
                }
                self.scope_observations(namespace, observations);
                for edge in relationships {
                    self.scope_evidence(namespace, &mut edge.evidence);
                }
                self.scope_evidence(namespace, evidence);
            }
            Record::ScanReference { evidence, .. } => {
                evidence.rule_ref = self.scoped_ref(namespace, &evidence.rule_ref);
            }
            Record::Root { .. } | Record::Operation { .. } | Record::Provider { .. } => (),
        }
    }
    fn records_graph(&self, g: &d::ArtifactGraph, records: &mut Vec<Record>) {
        records.extend(g.nodes().iter().map(|n| Record::Node {
            node: self.graph_node(g, n),
        }));
        records.extend(g.edges().iter().map(|e| Record::Edge {
            edge: self.graph_edge(g, e),
        }));
        records.extend(g.roots().iter().map(|r| Record::Root {
            node: self.entity_ref(&r.node),
            root_kind: category(&r.kind),
        }));
        records.extend(
            g.rules()
                .iter()
                .map(|r| Record::Rule { rule: self.rule(r) }),
        );
        records.extend(g.diagnostics().iter().map(|d| self.diagnostic(d)));
    }
    fn failures(&self, f: &[agentmender_application::ProviderFailure]) -> Vec<BlockerView> {
        f.iter()
            .map(|x| BlockerView {
                namespace_ref: None,
                provider: Some(x.provider.into()),
                evidence: vec![],
                code: category(&x.code),
                affected: vec![],
            })
            .collect()
    }
    pub fn project(
        &self,
        tool: &str,
        args: &[u8],
        response: &ReadOnlyResponse,
        page: &PageInput,
    ) -> Result<PageView, ProjectionError> {
        let raw = match response {
            ReadOnlyResponse::Providers(x) => serde_json::to_vec(x),
            ReadOnlyResponse::Scan(x) => serde_json::to_vec(x),
            ReadOnlyResponse::Graph(x) => serde_json::to_vec(x),
            ReadOnlyResponse::Inspect(x) | ReadOnlyResponse::Explain(x) => serde_json::to_vec(x),
            ReadOnlyResponse::Verification(x) => serde_json::to_vec(x),
            ReadOnlyResponse::Planning(x) => serde_json::to_vec(x),
        }
        .expect("response serialization");
        let mut binding = Vec::new();
        binding.extend_from_slice(&(tool.len() as u64).to_be_bytes());
        binding.extend_from_slice(tool.as_bytes());
        binding.extend_from_slice(&(args.len() as u64).to_be_bytes());
        binding.extend_from_slice(args);
        binding.extend_from_slice(&raw);
        let digest = self.mac(b"agentmender/view/v1\0", &binding);
        let offset = if let Some(cursor) = &page.cursor {
            let mut p = cursor.split('.');
            let (Some(o), Some(d), Some(m), None) = (p.next(), p.next(), p.next(), p.next()) else {
                return Err(ProjectionError::InvalidCursor);
            };
            let Some(signature) = unhex(m) else {
                return Err(ProjectionError::InvalidCursor);
            };
            let mut mac = <HmacSha256 as KeyInit>::new_from_slice(&self.key).expect("HMAC key");
            mac.update(b"agentmender/cursor/v1\0");
            mac.update(format!("{o}.{d}").as_bytes());
            if mac.verify_slice(&signature).is_err() {
                return Err(ProjectionError::InvalidCursor);
            };
            if d != hex(&digest) {
                return Err(ProjectionError::StaleCursor);
            };
            o.parse::<usize>()
                .map_err(|_| ProjectionError::InvalidCursor)?
        } else {
            0
        };
        let mut v = PageView {
            verification: vec![],
            counts: RecordCounts {
                nodes: 0,
                edges: 0,
                roots: 0,
                rules: 0,
                diagnostics: 0,
                proofs: 0,
                findings: 0,
                residuals: 0,
                planned_artifacts: 0,
                operations: 0,
                providers: 0,
                scan_entities: 0,
                scan_references: 0,
            },
            contexts: vec![],
            failures: vec![],
            statuses: vec![],
            blockers: vec![],
            plan: None,
            total_records: 0,
            records: vec![],
            next_cursor: None,
            presentation_complete: false,
        };
        let mut records = vec![];
        let mut namespaces: Vec<String> = vec![];
        match response {
            ReadOnlyResponse::Providers(x) => {
                v.failures = self.failures(&x.failures);
                for p in &x.providers {
                    let context = self.context(p, None, p.evidence_status, &[]);
                    namespaces.push(context.namespace_ref.clone());
                    records.push(Record::Provider { context });
                }
            }
            ReadOnlyResponse::Scan(x) => {
                v.failures = self.failures(&x.failures);
                for s in &x.snapshots {
                    let mut context = self.context(
                        &s.provider,
                        Some(s.completeness),
                        s.evidence_status,
                        &s.diagnostics,
                    );
                    context.coverage_ref =
                        Some(self.opaque(b"agentmender/coverage/v1\0", &s.coverage));
                    v.contexts.push(context);
                    let mut add = |class, local_id, obs| {
                        let id = d::ArtifactId {
                            provider: s.provider.kind,
                            state_root: s.provider.state_root.clone(),
                            class,
                            local_id,
                        };
                        let n = d::ArtifactNode {
                            id,
                            observations: vec![obs],
                            physical_identity: d::PhysicalIdentity::Unavailable,
                            evidence: vec![],
                        };
                        records.push(Record::ScanEntity {
                            node: self.node(&n),
                        });
                    };
                    for a in &s.workspaces {
                        add(
                            d::GraphNodeKind::Workspace,
                            a.id.clone(),
                            d::NodeObservation::Workspace(a.clone()),
                        );
                    }
                    for a in &s.projects {
                        add(
                            d::GraphNodeKind::Project,
                            a.id.clone(),
                            d::NodeObservation::Project(a.clone()),
                        );
                    }
                    for a in &s.sessions {
                        add(
                            d::GraphNodeKind::Session,
                            a.id.clone(),
                            d::NodeObservation::Session(a.clone()),
                        );
                    }
                    for a in &s.artifacts {
                        add(
                            d::GraphNodeKind::Artifact,
                            a.id.clone(),
                            d::NodeObservation::Artifact(a.clone()),
                        );
                    }
                    for reference in &s.references {
                        records.push(Record::ScanReference {
                            evidence: self.raw_evidence(&reference.evidence),
                            source_ref: self.local_ref(&d::ArtifactId {
                                provider: s.provider.kind,
                                state_root: s.provider.state_root.clone(),
                                class: d::GraphNodeKind::Unresolved,
                                local_id: reference.source.clone(),
                            }),
                            target_ref: self.local_ref(&d::ArtifactId {
                                provider: s.provider.kind,
                                state_root: s.provider.state_root.clone(),
                                class: d::GraphNodeKind::Unresolved,
                                local_id: reference.target.clone(),
                            }),
                            reference_kind: category(&reference.kind),
                            evidence_source: category(&reference.evidence.source),
                            confidence: category(&reference.evidence.confidence),
                        });
                    }
                    namespaces.resize(
                        records.len(),
                        self.context(&s.provider, None, s.evidence_status, &[])
                            .namespace_ref,
                    );
                }
            }
            ReadOnlyResponse::Graph(x) => {
                v.failures = self.failures(&x.failures);
                for g in &x.graphs {
                    v.contexts.push(self.graph_context(g.context()));
                    self.records_graph(g, &mut records);
                    namespaces.resize(records.len(), self.graph_context(g.context()).namespace_ref);
                }
            }
            ReadOnlyResponse::Inspect(x) | ReadOnlyResponse::Explain(x) => {
                v.failures = self.failures(&x.failures);
                v.contexts = x.contexts.iter().map(|c| self.graph_context(c)).collect();
                let e = &x.explanation;
                let mut explained = self.node(&e.node);
                if matches!(response, ReadOnlyResponse::Explain(_)) {
                    explained.ownership = Some(category(&e.ownership.classification));
                    explained.ownership_blockers =
                        e.ownership.blockers.iter().map(category).collect();
                    explained.ownership_evidence = e
                        .ownership
                        .supporting_evidence
                        .iter()
                        .map(|v| self.explanation_evidence(v, &e.evidence))
                        .collect();
                    explained.evidence = e
                        .node
                        .evidence
                        .iter()
                        .map(|v| self.explanation_evidence(v, &e.evidence))
                        .collect();
                } else {
                    explained.evidence.clear();
                }
                records.push(Record::Node { node: explained });
                if matches!(response, ReadOnlyResponse::Explain(_)) {
                    records.extend(e.incoming.iter().chain(&e.outgoing).map(|edge| {
                        let mut view = self.edge(edge);
                        view.evidence = edge
                            .evidence
                            .iter()
                            .map(|v| self.explanation_evidence(v, &e.evidence))
                            .collect();
                        Record::Edge { edge: view }
                    }));
                    records.extend(e.roots.iter().map(|r| Record::Root {
                        node: self.entity_ref(&r.node),
                        root_kind: category(&r.kind),
                    }));
                    records.extend(x.rules.iter().map(|r| Record::Rule { rule: self.rule(r) }));
                    records.extend(x.graph_diagnostics.iter().map(|d| self.diagnostic(d)));
                    v.statuses.push(category(&e.ownership.classification));
                    v.blockers.extend(e.ownership.blockers.iter().map(|b| {
                        BlockerView {
                            namespace_ref: Some(self.namespace_ref(&e.node.id)),
                            provider: Some(e.node.id.provider.into()),
                            evidence: e
                                .ownership
                                .supporting_evidence
                                .iter()
                                .map(|x| self.explanation_evidence(x, &e.evidence))
                                .collect(),
                            code: category(b),
                            affected: vec![self.entity_ref(&e.node.id)],
                        }
                    }));
                }
                namespaces.resize(records.len(), self.namespace_ref(&e.node.id));
            }
            ReadOnlyResponse::Verification(x) => {
                v.failures = self.failures(&x.failures);
                for r in &x.reports {
                    v.contexts.push(self.graph_context(&r.context));
                    v.statuses.push(category(&r.verification_status));
                    v.verification.push(VerificationSummary {
                        namespace_ref: self.graph_context(&r.context).namespace_ref,
                        status: category(&r.verification_status),
                    });
                    v.blockers.extend(r.blockers.iter().map(|b| BlockerView {
                        namespace_ref: Some(self.graph_context(&r.context).namespace_ref),
                        provider: Some(r.context.provider.kind.into()),
                        evidence: b.evidence.iter().map(|x| self.evaluated(x)).collect(),
                        code: category(&b.code),
                        affected: b.affected.iter().map(|a| self.entity_ref(a)).collect(),
                    }));
                    records.extend(r.findings.iter().map(|f| {
                        Record::Finding {
                            identity_ref: self.opaque(b"agentmender/issue/v1\0", &f.identity),
                            subject: self.entity_ref(&f.identity.subject),
                            rule_refs: f
                                .identity
                                .rule_ids
                                .iter()
                                .map(|r| self.opaque(b"agentmender/rule/v1\0", r))
                                .collect(),
                            observations: {
                                let n = d::ArtifactNode {
                                    id: f.identity.subject.clone(),
                                    observations: f.observations.clone(),
                                    physical_identity: d::PhysicalIdentity::Unavailable,
                                    evidence: vec![],
                                };
                                self.node(&n).observations
                            },
                            relationships: f.relationships.iter().map(|e| self.edge(e)).collect(),
                            issue_kind: category(&f.kind),
                            severity: category(&f.severity),
                            disposition: category(&f.disposition),
                            affected: f.affected.iter().map(|a| self.entity_ref(a)).collect(),
                            evidence: f.evidence.iter().map(|e| self.evaluated(e)).collect(),
                        }
                    }));
                    records.extend(r.residuals.iter().map(|r| Record::Residual {
                        artifact: self.entity_ref(&r.artifact),
                        classification: category(&r.classification),
                        evidence: r.evidence.iter().map(|e| self.evaluated(e)).collect(),
                    }));
                    records.extend(r.rules.iter().map(|r| Record::Rule { rule: self.rule(r) }));
                    records.extend(r.graph_diagnostics.iter().map(|d| self.diagnostic(d)));
                    let pc = &r.proof_context;
                    records.extend(pc.node_coverage.iter().map(|c| Record::Proof {
                        proof_kind: Category::NodeCoverage,
                        subject: None,
                        target: None,
                        category: Some(category(&c.class)),
                        completeness: Some(category(&c.completeness)),
                        evidence: c.evidence.iter().map(|e| self.evidence(e)).collect(),
                    }));
                    if let Some(c) = &pc.contract {
                        records.push(Record::Contract {
                            descriptor_ref: self.opaque(b"agentmender/contract-descriptor/v1\0", c),
                            known_version: known_version(&c.provider_version),
                            reference: self.opaque(b"agentmender/contract/v1\0", &c.id),
                            coherent: c.coherent,
                            evidence: c.evidence.iter().map(|e| self.evidence(e)).collect(),
                        });
                    }
                    records.extend(pc.relationship_coverage.iter().map(|c| {
                        Record::RelationshipCoverage {
                            source_class: c.source_class.into(),
                            target_class: c.target_class.into(),
                            edge_kind: category(&c.kind),
                            completeness: category(&c.completeness),
                            evidence: c.evidence.iter().map(|e| self.evidence(e)).collect(),
                        }
                    }));
                    for c in &pc.required_owners {
                        records.extend(c.relations.iter().map(|r| {
                            Record::OwnerRelation {
                                artifact: self.entity_ref(&c.artifact),
                                owner_class: r.owner_class.into(),
                                edge_kind: category(&r.edge_kind),
                                direction: category(&r.direction),
                                rule_refs: r
                                    .rule_ids
                                    .iter()
                                    .map(|id| self.opaque(b"agentmender/rule/v1\0", id))
                                    .collect(),
                            }
                        }));
                    }
                    records.extend(pc.required_targets.iter().map(|c| Record::Proof {
                        proof_kind: category(&c.kind),
                        subject: Some(self.entity_ref(&c.source)),
                        target: Some(self.entity_ref(&c.target)),
                        category: Some(category(&c.edge_kind)),
                        completeness: None,
                        evidence: c.evidence.iter().map(|e| self.evidence(e)).collect(),
                    }));
                    records.extend(pc.required_owners.iter().map(|c| Record::Proof {
                        proof_kind: Category::RequiredOwner,
                        subject: Some(self.entity_ref(&c.artifact)),
                        target: None,
                        category: None,
                        completeness: None,
                        evidence: c.evidence.iter().map(|e| self.evidence(e)).collect(),
                    }));
                    records.extend(pc.residuals.iter().map(|c| {
                        Record::Proof {
                            proof_kind: Category::ResidualObservation,
                            subject: Some(self.entity_ref(&c.artifact)),
                            target: None,
                            category: None,
                            completeness: None,
                            evidence: c
                                .expectation_evidence
                                .iter()
                                .map(|e| self.evidence(e))
                                .collect(),
                        }
                    }));
                    namespaces.resize(records.len(), self.graph_context(&r.context).namespace_ref);
                }
            }
            ReadOnlyResponse::Planning(x) => {
                v.failures = self.failures(&x.failures);
                v.contexts = x.contexts.iter().map(|c| self.graph_context(c)).collect();
                let p = x.plan.content();
                v.plan = Some(PlanView {
                    schema_version: p.schema_version,
                    fingerprint: self.fingerprint(&p.provider),
                    compatibility: category(&p.compatibility_status),
                    evidence_status: category(&p.evidence_status),
                    scope_max_entries: p.observation_scope.max_entries,
                    scope_include_archived: p.observation_scope.include_archived,
                    scope_project_path_present: p.observation_scope.project_path.is_some(),
                    native_target_ref: p
                        .native_target_id
                        .as_ref()
                        .map(|i| self.opaque(b"agentmender/native-id/v1\0", i)),
                    integrity_contract_ref: p
                        .integrity
                        .contract_id
                        .as_ref()
                        .map(|i| self.opaque(b"agentmender/contract/v1\0", i)),
                    capability: p.capability.as_ref().map(|c| CapabilityView {
                        provider: c.provider.into(),
                        fingerprint: self.fingerprint(&c.fingerprint),
                        operation: category(&c.operation),
                        effect_scope: category(&c.effect_scope),
                        protocol_evidence_ref: self.opaque(
                            b"agentmender/protocol-evidence/v1\0",
                            &c.protocol_evidence_digest,
                        ),
                    }),
                    logical_known_subtotal_bytes: p.space.logical.known_subtotal_bytes,
                    logical_total_bytes: p.space.logical.total_bytes,
                    logical_unknown_reasons: p
                        .space
                        .logical
                        .unknown_reasons
                        .iter()
                        .map(category)
                        .collect(),
                    physical_bytes: p.space.physical.bytes,
                    physical_reason: category(&p.space.physical.reason),
                    integrity_status: category(&p.integrity.status),
                    integrity_blockers: p.integrity.blockers.iter().map(category).collect(),
                    integrity_findings: p.integrity.findings.iter().map(category).collect(),
                    effect_contract_ref: p
                        .effect_contract
                        .as_ref()
                        .map(|c| self.opaque(b"agentmender/contract/v1\0", &c.id)),
                    effect_affected: p
                        .effect_contract
                        .as_ref()
                        .map(|c| c.affected.iter().map(|a| self.entity_ref(a)).collect())
                        .unwrap_or_default(),
                    effect_rules: p
                        .effect_contract
                        .as_ref()
                        .map(|c| {
                            c.rule_ids
                                .iter()
                                .map(|r| self.opaque(b"agentmender/rule/v1\0", r))
                                .collect()
                        })
                        .unwrap_or_default(),
                    digest: x.plan.digest().as_str().into(),
                    state_fingerprint: p.state_fingerprint.as_str().into(),
                    digest_basis: "original_unredacted_domain_plan_v1".into(),
                    status: category(&p.status),
                    intent: category(&p.request.intent),
                    target: self.entity_ref(&p.request.target),
                    blockers: p.blockers.iter().map(category).collect(),
                    preconditions: p.preconditions.iter().map(category).collect(),
                    postconditions: p.postconditions.iter().map(category).collect(),
                    planning_only: true,
                    executed: false,
                    requires_human_approval: p.requires_human_approval,
                    recovery_required: p.recovery_required,
                });
                records.extend(p.artifacts.iter().map(|a| {
                    Record::PlannedArtifact {
                        id: self.entity_ref(&a.id),
                        role: category(&a.role),
                        ownership: category(&a.ownership.classification),
                        protected: a.explicitly_protected,
                        blockers: a.ownership.blockers.iter().map(category).collect(),
                        evidence: a
                            .ownership
                            .supporting_evidence
                            .iter()
                            .map(|e| self.evidence(e))
                            .collect(),
                    }
                }));
                records.extend(p.operations.iter().map(|o| Record::Operation {
                    native_id_ref: self.opaque(b"agentmender/native-id/v1\0", &o.native_id),
                    target: self.entity_ref(&o.target),
                    operation: category(&o.operation),
                    affected: o.affected.iter().map(|a| self.entity_ref(a)).collect(),
                }));
                namespaces.resize(records.len(), self.namespace_ref(&p.request.target));
            }
        }
        for record in &records {
            match record {
                Record::Node { .. } => v.counts.nodes += 1,
                Record::Edge { .. } => v.counts.edges += 1,
                Record::Root { .. } => v.counts.roots += 1,
                Record::Rule { .. } => v.counts.rules += 1,
                Record::Diagnostic { .. } => v.counts.diagnostics += 1,
                Record::Proof { .. }
                | Record::Contract { .. }
                | Record::OwnerRelation { .. }
                | Record::RelationshipCoverage { .. } => v.counts.proofs += 1,
                Record::Finding { .. } => v.counts.findings += 1,
                Record::Residual { .. } => v.counts.residuals += 1,
                Record::PlannedArtifact { .. } => v.counts.planned_artifacts += 1,
                Record::Operation { .. } => v.counts.operations += 1,
                Record::Provider { .. } => v.counts.providers += 1,
                Record::ScanEntity { .. } => v.counts.scan_entities += 1,
                Record::ScanReference { .. } => v.counts.scan_references += 1,
            }
        }
        let records: Vec<RecordEntry> = records
            .into_iter()
            .zip(namespaces)
            .map(|(mut record, namespace_ref)| {
                self.scope_record(&namespace_ref, &mut record);
                RecordEntry {
                    namespace_ref,
                    record,
                }
            })
            .collect();
        if let Some(plan) = &mut v.plan {
            let namespace = self.namespace_ref(&match response {
                ReadOnlyResponse::Planning(x) => x.plan.content().request.target.clone(),
                _ => unreachable!(),
            });
            for r in &mut plan.effect_rules {
                *r = self.scoped_ref(&namespace, r);
            }
        }
        for b in &mut v.blockers {
            if let Some(namespace) = &b.namespace_ref {
                self.scope_evidence(namespace, &mut b.evidence);
            }
        }
        v.total_records = records.len();
        if offset > records.len() || !page.valid() {
            return Err(ProjectionError::InvalidCursor);
        };
        let mut end = offset.saturating_add(page.page_size).min(records.len());
        loop {
            v.records = records[offset..end].to_vec();
            v.presentation_complete = end == v.total_records;
            v.next_cursor = if v.presentation_complete {
                None
            } else {
                let prefix = format!("{end}.{}", hex(&digest));
                let sig = self.mac(b"agentmender/cursor/v1\0", prefix.as_bytes());
                Some(format!("{prefix}.{}", hex(&sig)))
            };
            if serde_json::to_vec(&v).expect("view serialization").len() <= 80 * 1024 {
                return Ok(v);
            }
            if end <= offset + 1 {
                return Err(ProjectionError::ResponseTooLarge);
            }
            end -= 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmender_application::ProviderReport;
    use std::path::PathBuf;
    #[test]
    fn search_references_are_kind_and_process_bound_and_cannot_be_modified() {
        let projector = Projector::new([1; 32]);
        let value = serde_json::json!({"scope":"fixture","start":100});
        let token = projector.seal_search(b"cursor", &value);
        assert_eq!(
            projector.open_search::<serde_json::Value>(b"cursor", &token),
            Some(value)
        );
        assert!(
            projector
                .open_search::<serde_json::Value>(b"evidence", &token)
                .is_none()
        );
        assert!(
            Projector::new([2; 32])
                .open_search::<serde_json::Value>(b"cursor", &token)
                .is_none()
        );
        let mut forged = token.into_bytes();
        forged[0] = if forged[0] == b'0' { b'1' } else { b'0' };
        assert!(
            projector
                .open_search::<serde_json::Value>(b"cursor", std::str::from_utf8(&forged).unwrap())
                .is_none()
        );
    }
    fn id(root: &str, class: d::GraphNodeKind) -> d::ArtifactId {
        d::ArtifactId {
            provider: d::ProviderKind::Codex,
            state_root: PathBuf::from(root),
            class,
            local_id: d::EntityId::new("private-id-canary").unwrap(),
        }
    }
    fn provider(root: &str) -> d::Provider {
        d::Provider {
            kind: d::ProviderKind::Codex,
            presence: d::Presence::StateOnly,
            state_root: PathBuf::from(root),
            fingerprint: d::ProviderFingerprint {
                executable: Some(PathBuf::from("/secret-executable")),
                executable_version: Some("secret-version".into()),
                adapter_version: "secret-adapter".into(),
                interface_id: Some("secret-interface".into()),
                layout_id: None,
            },
            capabilities: Default::default(),
            compatibility: d::CompatibilityProfile {
                status: d::CompatibilityStatus::AuditOnlyUnknown,
                interface_stability: d::InterfaceStability::Unsupported,
                reason: "private-reason".into(),
            },
            installations: vec![],
            evidence_status: d::EvidenceStatus::Unverified,
            diagnostics: vec![d::Diagnostic {
                code: d::DiagnosticCode::IncompleteDiscovery,
                message: "private-message".into(),
                path: Some(PathBuf::from("/private-path")),
            }],
        }
    }
    #[test]
    fn identities_are_keyed_and_include_root_and_class() {
        let p = Projector::new([7; 32]);
        let a = id("/private-a", d::GraphNodeKind::Session);
        let b = id("/private-b", d::GraphNodeKind::Session);
        let c = id("/private-a", d::GraphNodeKind::Artifact);
        assert_ne!(p.entity_ref(&a), p.entity_ref(&b));
        assert_ne!(p.entity_ref(&a), p.entity_ref(&c));
        assert_ne!(p.entity_ref(&a), Projector::new([8; 32]).entity_ref(&a));
        assert!((p.selector(&p.entity_ref(&a)))(&a));
        assert!(!(p.selector(&p.entity_ref(&a)))(&b));
    }
    #[test]
    fn privacy_paging_and_stale_cursor() {
        let p = Projector::new([1; 32]);
        let response = ReadOnlyResponse::Providers(ProviderReport {
            providers: vec![provider("/private-a"), provider("/private-b")],
            failures: vec![],
        });
        let page = PageInput {
            page_size: 1,
            cursor: None,
        };
        let a = p.project("providers", b"{}", &response, &page).unwrap();
        let text = serde_json::to_string(&a).unwrap();
        for canary in [
            "private-a",
            "secret-executable",
            "secret-version",
            "secret-adapter",
            "secret-interface",
            "private-reason",
            "private-message",
            "private-path",
        ] {
            assert!(!text.contains(canary));
        }
        assert_eq!(
            text,
            serde_json::to_string(&p.project("providers", b"{}", &response, &page).unwrap())
                .unwrap()
        );
        let next = PageInput {
            page_size: 1,
            cursor: a.next_cursor.clone(),
        };
        let b = p.project("providers", b"{}", &response, &next).unwrap();
        assert!(b.presentation_complete);
        let changed = ReadOnlyResponse::Providers(ProviderReport {
            providers: vec![provider("/private-c")],
            failures: vec![],
        });
        assert_eq!(
            p.project("providers", b"{}", &changed, &next).unwrap_err(),
            ProjectionError::StaleCursor
        );
        assert_eq!(
            p.project("other", b"{}", &response, &next).unwrap_err(),
            ProjectionError::StaleCursor
        );
    }
    #[test]
    fn rule_references_are_scoped_to_provider_root() {
        let p = Projector::new([2; 32]);
        let a = id("/a", d::GraphNodeKind::Session);
        let b = id("/b", d::GraphNodeKind::Session);
        let ns_a = p.namespace_ref(&a);
        let ns_b = p.namespace_ref(&b);
        let rule = p.opaque(
            b"agentmender/rule/v1\0",
            &d::EntityId::new("same-rule").unwrap(),
        );
        assert_eq!(p.scoped_ref(&ns_a, &rule), p.scoped_ref(&ns_a, &rule));
        assert_ne!(p.scoped_ref(&ns_a, &rule), p.scoped_ref(&ns_b, &rule));
    }
    #[test]
    fn malformed_unicode_cursor_is_rejected() {
        let p = Projector::new([1; 32]);
        let r = ReadOnlyResponse::Providers(ProviderReport::default());
        let page = PageInput {
            page_size: 1,
            cursor: Some("1.abc.éé".into()),
        };
        assert_eq!(
            p.project("providers", b"{}", &r, &page).unwrap_err(),
            ProjectionError::InvalidCursor
        );
    }
    fn explain_response(evidence_count: usize) -> ReadOnlyResponse {
        let provider = provider("/private-state-root");
        let context = d::GraphContext {
            provider: provider.clone(),
            evidence_status: d::EvidenceStatus::Unverified,
            completeness: d::Completeness::Partial,
            coverage: "private-coverage".into(),
            diagnostics: vec![],
        };
        let raw = d::Evidence {
            source: d::EvidenceSource::Fixture,
            confidence: d::Confidence::Heuristic,
            provider_version: Some("private-producer-version".into()),
            rule: "private-evidence-rule".into(),
        };
        let observation = d::NodeObservation::Session(d::Session {
            id: d::EntityId::new("private-session-id").unwrap(),
            provider_session_id: "private-native-id".into(),
            project_id: None,
            state: d::SessionState::Unknown,
            last_active: Some("private-timestamp".into()),
            evidence: raw.clone(),
        });
        let ev: Vec<_> = (0..evidence_count)
            .map(|i| d::GraphEvidence {
                observation: raw.clone(),
                rule_id: d::EntityId::new(format!("private-rule-{i}")).unwrap(),
                nature: d::EvidenceNature::Observation,
                locator: Some(PathBuf::from(format!("/private-locator-{i}"))),
                observed_at: Some("private-observed-at".into()),
            })
            .collect();
        let node = d::ArtifactNode {
            id: id("/private-state-root", d::GraphNodeKind::Session),
            observations: vec![observation],
            physical_identity: d::PhysicalIdentity::Unavailable,
            evidence: ev,
        };
        ReadOnlyResponse::Explain(agentmender_application::GraphExplainReport {
            explanation: d::NodeExplanation {
                node,
                incoming: vec![],
                outgoing: vec![],
                roots: vec![],
                ownership: d::OwnershipAssessment {
                    classification: d::OwnershipClassification::Unknown,
                    supporting_evidence: vec![],
                    blockers: vec![d::ProtectionReason::PartialCoverage],
                },
                evidence: vec![],
            },
            rules: vec![d::EvidenceRule {
                id: d::EntityId::new("private-rule-name").unwrap(),
                provider: d::ProviderKind::Codex,
                description: "private-rule-description".into(),
                admissible_sources: vec![d::EvidenceSource::Fixture],
                max_confidence: d::Confidence::Heuristic,
                adapter_version: "private-rule-version".into(),
                interface_id: Some("private-rule-interface".into()),
                layout_id: Some("private-rule-layout".into()),
            }],
            context: context.clone(),
            contexts: vec![context],
            graph_diagnostics: vec![],
            failures: vec![agentmender_application::ProviderFailure {
                provider: d::ProviderKind::Codex,
                code: d::DiagnosticCode::IncompleteDiscovery,
                message: "private-error-prose".into(),
            }],
        })
    }
    #[test]
    fn nested_evidence_and_rule_strings_are_private() {
        let p = Projector::new([4; 32]);
        let response = explain_response(2);
        let output = p
            .project("explain", b"{}", &response, &PageInput::default())
            .unwrap();
        let json = serde_json::to_string(&output).unwrap();
        for canary in [
            "private-state-root",
            "private-coverage",
            "private-producer-version",
            "private-evidence-rule",
            "private-session-id",
            "private-native-id",
            "private-timestamp",
            "private-locator",
            "private-observed-at",
            "private-rule-name",
            "private-rule-description",
            "private-rule-version",
            "private-rule-interface",
            "private-rule-layout",
            "private-error-prose",
        ] {
            assert!(!json.contains(canary), "leaked {canary}");
        }
        assert!(json.contains("partial"));
        assert!(json.contains("partial_coverage"));
    }
    #[test]
    fn one_oversized_record_returns_error() {
        let p = Projector::new([4; 32]);
        let response = explain_response(2000);
        assert_eq!(
            p.project("explain", b"{}", &response, &PageInput::default())
                .unwrap_err(),
            ProjectionError::ResponseTooLarge
        );
    }
    #[test]
    fn explanation_preserves_edge_only_effective_confidence() {
        let p = Projector::new([4; 32]);
        let mut response = explain_response(1);
        let ReadOnlyResponse::Explain(report) = &mut response else {
            unreachable!()
        };
        let mut edge_evidence = report.explanation.node.evidence[0].clone();
        edge_evidence.rule_id = d::EntityId::new("private-edge-only-rule").unwrap();
        edge_evidence.observation.confidence = d::Confidence::Authoritative;
        report.explanation.outgoing.push(d::ReferenceEdge {
            source: report.explanation.node.id.clone(),
            target: report.explanation.node.id.clone(),
            kind: d::EdgeKind::References,
            evidence: vec![edge_evidence.clone()],
        });
        report
            .explanation
            .ownership
            .supporting_evidence
            .push(edge_evidence.clone());
        report.explanation.evidence.push(d::EvaluatedEvidence {
            evidence: edge_evidence,
            confidence: d::Confidence::Heuristic,
        });
        let view = p
            .project("explain", b"{}", &response, &PageInput::default())
            .unwrap();
        let wire = serde_json::to_value(view).unwrap();
        let edge = wire["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "edge")
            .unwrap();
        assert_eq!(edge["edge"]["evidence"][0]["confidence"], "authoritative");
        assert_eq!(
            edge["edge"]["evidence"][0]["effective_confidence"],
            "heuristic"
        );
        let node = wire["records"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["kind"] == "node")
            .unwrap();
        assert_eq!(
            node["node"]["ownership_evidence"][0]["effective_confidence"],
            "heuristic"
        );
        assert_eq!(
            wire["blockers"][0]["evidence"][0]["effective_confidence"],
            "heuristic"
        );
        assert!(!wire.to_string().contains("private-edge-only-rule"));
    }
    #[test]
    fn private_only_source_change_invalidates_cursor() {
        let p = Projector::new([2; 32]);
        let a = provider("/same");
        let mut b = a.clone();
        b.compatibility.reason = "different private reason".into();
        let first = ReadOnlyResponse::Providers(ProviderReport {
            providers: vec![a.clone(), a],
            failures: vec![],
        });
        let changed = ReadOnlyResponse::Providers(ProviderReport {
            providers: vec![b.clone(), b],
            failures: vec![],
        });
        let page = PageInput {
            page_size: 1,
            cursor: None,
        };
        let start = p.project("providers", b"{}", &first, &page).unwrap();
        let next = PageInput {
            page_size: 1,
            cursor: start.next_cursor,
        };
        assert_eq!(
            p.project("providers", b"{}", &changed, &next).unwrap_err(),
            ProjectionError::StaleCursor
        );
    }
    #[cfg(unix)]
    #[test]
    fn non_utf8_root_and_scan_local_refs_are_lossless() {
        use std::os::unix::ffi::OsStringExt;
        let p = Projector::new([3; 32]);
        let mut a = id("/root", d::GraphNodeKind::Session);
        a.state_root = std::ffi::OsString::from_vec(vec![47, 255]).into();
        let mut b = a.clone();
        b.state_root = std::ffi::OsString::from_vec(vec![47, 254]).into();
        assert_ne!(p.entity_ref(&a), p.entity_ref(&b));
        assert_ne!(p.local_ref(&a), p.local_ref(&b));
    }
    mod plan_tests {
        use super::*;
        use agentmender_domain::*;
        fn id(value: &str) -> EntityId {
            EntityId::new(value).unwrap()
        }
        fn evidence() -> GraphEvidence {
            GraphEvidence {
                observation: Evidence {
                    source: EvidenceSource::Fixture,
                    confidence: Confidence::Authoritative,
                    provider_version: Some("synthetic-v1".into()),
                    rule: "synthetic explicit identity".into(),
                },
                rule_id: id("synthetic-identity"),
                nature: EvidenceNature::Observation,
                locator: None,
                observed_at: Some("2026-10-28T00:00:00Z".into()),
            }
        }
        struct Case {
            graph: ArtifactGraph,
            proof: IntegrityContext,
            planning: PlanningContext,
            target: ArtifactId,
        }
        impl Case {
            fn plan(&self, intent: LifecycleIntent) -> LifecyclePlan {
                plan_lifecycle(
                    &self.graph,
                    &evaluate_integrity(&self.graph, &self.proof),
                    &self.planning,
                    LifecycleRequest {
                        target: self.target.clone(),
                        intent,
                    },
                )
            }
        }
        fn case(extra: usize) -> Case {
            let ev = evidence();
            let fingerprint = ProviderFingerprint {
                executable: None,
                executable_version: Some("synthetic-v1".into()),
                adapter_version: "synthetic-adapter-v1".into(),
                interface_id: Some("synthetic-interface-v1".into()),
                layout_id: Some("synthetic-layout-v1".into()),
            };
            let provider = agentmender_domain::Provider {
                kind: ProviderKind::Codex,
                presence: Presence::StateOnly,
                state_root: "/synthetic-planning".into(),
                fingerprint: fingerprint.clone(),
                capabilities: ProviderCapabilities::new(),
                compatibility: CompatibilityProfile {
                    status: CompatibilityStatus::ReadOnlyKnown,
                    interface_stability: InterfaceStability::OfficialStable,
                    reason: "synthetic only".into(),
                },
                installations: vec![],
                evidence_status: EvidenceStatus::SyntheticOnly,
                diagnostics: vec![],
            };
            let mut b = ArtifactGraph::builder(GraphContext {
                provider,
                evidence_status: EvidenceStatus::SyntheticOnly,
                completeness: Completeness::Complete,
                coverage: "synthetic full boundary".into(),
                diagnostics: vec![],
            });
            let identity_rule = EvidenceRule {
                id: ev.rule_id.clone(),
                provider: ProviderKind::Codex,
                description: "synthetic authoritative identity".into(),
                admissible_sources: vec![EvidenceSource::Fixture, EvidenceSource::ProviderApi],
                max_confidence: Confidence::Authoritative,
                adapter_version: fingerprint.adapter_version.clone(),
                interface_id: fingerprint.interface_id.clone(),
                layout_id: fingerprint.layout_id.clone(),
            };
            b.add_rule(identity_rule.clone()).unwrap();
            let mut effect_rule = identity_rule;
            effect_rule.id = id("synthetic-native-effects");
            effect_rule.description = "synthetic exact exhaustive native effects contract".into();
            b.add_rule(effect_rule).unwrap();
            let global = b.observe(NodeObservation::ProviderState);
            b.add_root(ProtectedRoot {
                node: global,
                kind: RootKind::ProviderGlobal,
            })
            .unwrap();
            let target = b.observe(NodeObservation::Session(Session {
                id: id("session:target"),
                provider_session_id: "native-target".into(),
                project_id: None,
                state: SessionState::Active,
                last_active: Some("2026-10-27T00:00:00Z".into()),
                evidence: ev.observation.clone(),
            }));
            b.add_node_evidence(&target, ev.clone()).unwrap();
            b.add_root(ProtectedRoot {
                node: target.clone(),
                kind: RootKind::ActiveSession,
            })
            .unwrap();
            b.add_edge(ReferenceEdge {
                source: target.clone(),
                target: target.clone(),
                kind: EdgeKind::References,
                evidence: vec![ev.clone()],
            })
            .unwrap();
            for n in 0..extra {
                let node = b.observe(NodeObservation::Artifact(Artifact {
                    id: id(&format!("artifact:{n}")),
                    kind: ArtifactKind::Attachment,
                    locator: format!("/synthetic-planning/artifact-{n}").into(),
                    size_bytes: Some(n as u64 + 1),
                    evidence: ev.observation.clone(),
                }));
                b.add_node_evidence(&node, ev.clone()).unwrap();
            }
            let mut classes = vec![GraphNodeKind::ProviderState, GraphNodeKind::Session];
            if extra > 0 {
                classes.push(GraphNodeKind::Artifact);
            }
            let proof = IntegrityContext {
                contract: Some(IntegrityContract {
                    id: id("synthetic-closed-contract"),
                    provider_version: "synthetic-v1".into(),
                    adapter_version: fingerprint.adapter_version.clone(),
                    interface_id: fingerprint.interface_id.clone(),
                    layout_id: fingerprint.layout_id.clone(),
                    coherent: true,
                    evidence: vec![ev.clone()],
                }),
                node_coverage: classes
                    .into_iter()
                    .map(|class| NodeCoverage {
                        class,
                        completeness: Completeness::Complete,
                        evidence: vec![ev.clone()],
                    })
                    .collect(),
                relationship_coverage: vec![RelationshipCoverage {
                    source_class: GraphNodeKind::Session,
                    target_class: GraphNodeKind::Session,
                    kind: EdgeKind::References,
                    completeness: Completeness::Complete,
                    evidence: vec![ev.clone()],
                }],
                required_targets: vec![RequiredTarget {
                    source: target.clone(),
                    target: target.clone(),
                    edge_kind: EdgeKind::References,
                    kind: RequiredTargetKind::Reference,
                    evidence: vec![ev.clone()],
                }],
                ..Default::default()
            };
            let capability = LifecycleCapability {
                provider: ProviderKind::Codex,
                fingerprint,
                operation: NativeLifecycleOperation::ArchiveSession,
                protocol_evidence_digest: format!("sha256:{}", "a".repeat(64)),
                effect_scope: NativeEffectScope::TargetOnly,
            };
            let mut effect_evidence = ev.clone();
            effect_evidence.rule_id = id("synthetic-native-effects");
            effect_evidence.observation.rule = "synthetic exhaustive native effect proof".into();
            let planning = PlanningContext {
                complete_removal_evidence: None,
                cleanup_evidence: None,
                scope: DiscoveryScope::default(),
                capabilities: vec![capability.clone()],
                effects: Some(NativeEffectBoundary {
                    contract_id: id("synthetic-archive-effects-v1"),
                    capability,
                    effect_rule_ids: vec![id("synthetic-native-effects")],
                    target: target.clone(),
                    operation: NativeLifecycleOperation::ArchiveSession,
                    affected: vec![target.clone()],
                    completeness: Completeness::Complete,
                    coherent: true,
                    evidence: vec![effect_evidence],
                }),
            };
            Case {
                graph: b.finish().unwrap(),
                proof,
                planning,
                target,
            }
        }

        #[test]
        fn integrity_findings_blockers_and_proofs_keep_structure_and_privacy() {
            let c = case(0);
            let mut report = evaluate_integrity(&c.graph, &c.proof);
            report.verification_status = VerificationStatus::Degraded;
            let ev = evidence();
            report.findings.push(IntegrityIssue {
                identity: IntegrityIssueIdentity {
                    provider: ProviderKind::Codex,
                    state_root: "/secret-integrity-root".into(),
                    kind: IntegrityIssueKind::DanglingReference,
                    subject: c.target.clone(),
                    affected: vec![c.target.clone()],
                    rule_ids: vec![id("secret-issue-rule")],
                },
                kind: IntegrityIssueKind::DanglingReference,
                severity: IntegritySeverity::Error,
                disposition: IssueDisposition::EstablishedViolation,
                affected: vec![c.target.clone()],
                evidence: vec![EvaluatedEvidence {
                    evidence: ev.clone(),
                    confidence: Confidence::Strong,
                }],
                explanation: "secret-issue-prose".into(),
                observations: c.graph.node(&c.target).unwrap().observations.clone(),
                relationships: c.graph.edges().to_vec(),
            });
            report.blockers.push(IntegrityBlocker {
                code: IntegrityBlockerCode::MissingCoverage,
                affected: vec![c.target.clone()],
                evidence: vec![EvaluatedEvidence {
                    evidence: ev,
                    confidence: Confidence::Unknown,
                }],
                explanation: "secret-blocker-prose".into(),
            });
            let response =
                ReadOnlyResponse::Verification(agentmender_application::VerificationReport {
                    reports: vec![report],
                    failures: vec![],
                });
            let view = Projector::new([5; 32])
                .project("verify_integrity", b"{}", &response, &PageInput::default())
                .unwrap();
            let json = serde_json::to_string(&view).unwrap();
            for canary in [
                "secret-integrity-root",
                "secret-issue-rule",
                "secret-issue-prose",
                "secret-blocker-prose",
                "synthetic-closed-contract",
                "native-target",
                "synthetic explicit identity",
            ] {
                assert!(!json.contains(canary), "leaked {canary}");
            }
            assert!(json.contains("degraded"));
            assert!(json.contains("missing_coverage"));
            assert!(json.contains("dangling_reference"));
            assert!(json.contains("\"effective_confidence\":\"strong\""));
            assert!(
                view.records
                    .iter()
                    .any(|r| matches!(&r.record, Record::Contract { .. }))
            );
            assert!(view.records.iter().any(|r|matches!(&r.record,Record::Finding{observations,relationships,..} if !observations.is_empty() && !relationships.is_empty())));
        }

        #[test]
        fn statuses_and_original_commitments_survive_privacy_projection() {
            let c = case(1);
            let projector = Projector::new([8; 32]);
            for (intent, expected) in [
                (LifecycleIntent::Archive, PlanStatus::Ready),
                (LifecycleIntent::Archive, PlanStatus::Blocked),
                (LifecycleIntent::Repair, PlanStatus::Unsupported),
            ] {
                let plan = if expected == PlanStatus::Blocked {
                    let mut blocked = case(1);
                    blocked.planning.effects = None;
                    blocked.plan(intent)
                } else {
                    c.plan(intent)
                };
                assert_eq!(plan.content().status, expected);
                let expected_digest = plan.digest().as_str().to_owned();
                let expected_state = plan.content().state_fingerprint.as_str().to_owned();
                let original_blockers = plan.content().blockers.len();
                let original_pre = plan.content().preconditions.len();
                let original_post = plan.content().postconditions.len();
                let original_artifacts = plan.content().artifacts.len();
                let response =
                    ReadOnlyResponse::Planning(agentmender_application::PlanningReport {
                        plan,
                        contexts: vec![c.graph.context().clone()],
                        failures: vec![],
                    });
                let view = projector
                    .project("plan_lifecycle", b"{}", &response, &PageInput::default())
                    .unwrap();
                let header = view.plan.as_ref().unwrap();
                assert_eq!(header.digest, expected_digest);
                assert_eq!(header.state_fingerprint, expected_state);
                assert_eq!(header.digest_basis, "original_unredacted_domain_plan_v1");
                assert_eq!(header.blockers.len(), original_blockers);
                assert_eq!(header.preconditions.len(), original_pre);
                assert_eq!(header.postconditions.len(), original_post);
                assert_eq!(
                    view.records
                        .iter()
                        .filter(|r| matches!(&r.record, Record::PlannedArtifact { .. }))
                        .count(),
                    original_artifacts
                );
                assert!(header.planning_only && !header.executed && header.requires_human_approval);
                let json = serde_json::to_string(&view).unwrap();
                for canary in [
                    "synthetic-planning",
                    "synthetic-adapter-v1",
                    "synthetic-interface-v1",
                    "synthetic-layout-v1",
                    "synthetic-closed-contract",
                    "synthetic-archive-effects-v1",
                    "native-target",
                    "synthetic explicit identity",
                    "2026-10-27",
                ] {
                    assert!(!json.contains(canary), "leaked {canary}");
                }
            }
        }
    }
}
