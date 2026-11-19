//! Confirmed Codex deletion uses native commands for the reviewed lineage.
use crate::{CodexAdapter, worker::LineageFact};
use agentmender_domain::{ProviderKind, SessionIdentity, canonical_uuid};
use agentmender_provider_api::process;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};
use uuid::Uuid;

#[derive(Debug, PartialEq, Eq, Serialize)]
pub struct DeletionPlan {
    #[serde(skip)]
    root: PathBuf,
    trees: Vec<Tree>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Tree {
    id: Uuid,
    descendants: Vec<Descendant>,
}

#[derive(Debug, PartialEq, Eq, Serialize)]
struct Descendant {
    id: Uuid,
    parent: Uuid,
    archived: bool,
    present: bool,
    depth: usize,
}

#[derive(Default)]
pub struct Deletion {
    pub removed: usize,
    pub messages: Vec<String>,
    pub errors: Vec<String>,
}

#[derive(Default)]
struct Node {
    parents: BTreeSet<Uuid>,
    present: bool,
    archived: bool,
    uncertain: bool,
}

#[derive(Default)]
struct Archive {
    nodes: BTreeMap<Uuid, Node>,
    children: BTreeMap<Uuid, BTreeSet<Uuid>>,
}

impl Archive {
    fn from_facts(facts: Vec<LineageFact>) -> Self {
        let mut archive = Self::default();
        for fact in facts {
            match fact {
                LineageFact::Present { id, archived } => {
                    let node = archive.nodes.entry(id).or_default();
                    node.present = true;
                    node.archived |= archived;
                }
                LineageFact::Edge { parent, child } => {
                    archive
                        .nodes
                        .entry(child)
                        .or_default()
                        .parents
                        .insert(parent);
                    archive.children.entry(parent).or_default().insert(child);
                }
                LineageFact::Uncertain(id) => archive.nodes.entry(id).or_default().uncertain = true,
            }
        }
        archive
    }

    fn present(&self, id: Uuid) -> bool {
        self.nodes
            .get(&id)
            .is_some_and(|node| node.present || node.uncertain)
    }

    fn tree(&self, id: Uuid, protected: &BTreeSet<Uuid>) -> Result<Tree, String> {
        let root = self
            .nodes
            .get(&id)
            .ok_or("Sessione Codex non più presente.")?;
        if !root.present || root.uncertain || !root.parents.is_empty() {
            return Err(format!(
                "{id}: identità o relazione della sessione Codex non verificabile."
            ));
        }

        let mut descendants = Vec::new();
        let mut pending = vec![(id, 0)];
        let mut seen = BTreeSet::from([id]);
        let mut offset = 0;
        while let Some(&(parent, depth)) = pending.get(offset) {
            offset += 1;
            if protected.contains(&parent) {
                return Err(format!(
                    "{parent}: sessione protetta dalla configurazione di avvio."
                ));
            }
            for &child in self.children.get(&parent).into_iter().flatten() {
                let node = &self.nodes[&child];
                if !seen.insert(child) || node.parents.len() != 1 || node.uncertain {
                    return Err(format!("{child}: discendenza Codex ambigua o ciclica."));
                }
                descendants.push(Descendant {
                    id: child,
                    parent,
                    archived: node.archived,
                    present: node.present,
                    depth: depth + 1,
                });
                pending.push((child, depth + 1));
            }
        }
        Ok(Tree { id, descendants })
    }

    fn matches_reviewed_edges(&self, tree: &Tree) -> bool {
        let reviewed: BTreeMap<_, _> = tree
            .descendants
            .iter()
            .map(|target| (target.id, target.parent))
            .collect();
        std::iter::once((tree.id, None))
            .chain(
                tree.descendants
                    .iter()
                    .map(|target| (target.id, Some(target.parent))),
            )
            .all(|(id, parent)| {
                self.nodes.get(&id).is_none_or(|node| {
                    !node.uncertain && node.parents.iter().all(|found| Some(*found) == parent)
                }) && self
                    .children
                    .get(&id)
                    .into_iter()
                    .flatten()
                    .all(|child| reviewed.get(child) == Some(&id))
            })
    }
}

impl CodexAdapter {
    fn archive(&self) -> Result<Archive, String> {
        self.lineage()
            .map(Archive::from_facts)
            .map_err(|_| "Impossibile verificare il grafo o leggere l’archivio Codex.".into())
    }

    fn selected_root(&self) -> Result<PathBuf, String> {
        self.location
            .state_root
            .canonicalize()
            .map_err(|error| format!("Archivio Codex non disponibile: {error}"))
    }

    pub fn deletion_plan(
        &self,
        sessions: &[SessionIdentity],
        protected: &[String],
    ) -> Result<DeletionPlan, String> {
        let root = self.selected_root()?;
        let ids = sessions
            .iter()
            .map(|identity| {
                if identity.provider != ProviderKind::Codex
                    || identity.state_root != root
                    || identity.namespace.is_some()
                    || !canonical_uuid(&identity.native_id)
                {
                    return Err("Sessione estranea all’archivio Codex.".to_owned());
                }
                Uuid::parse_str(&identity.native_id).map_err(|_| "UUID Codex non valido.".into())
            })
            .collect::<Result<Vec<_>, String>>()?;
        if ids.is_empty() {
            return Err("Nessuna sessione Codex da eliminare.".into());
        }
        self.plan(root, &ids, protected)
    }

    fn plan(
        &self,
        root: PathBuf,
        ids: &[Uuid],
        protected: &[String],
    ) -> Result<DeletionPlan, String> {
        let archive = self.archive()?;
        let protected = protected
            .iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();
        let trees = ids
            .iter()
            .map(|id| archive.tree(*id, &protected))
            .collect::<Result<_, _>>()?;
        Ok(DeletionPlan { root, trees })
    }

    pub fn delete_reviewed(
        &self,
        plan: DeletionPlan,
        protected: &[String],
    ) -> Result<Deletion, String> {
        let _deletion = self.deletion.lock().expect("Codex deletion");
        let root = self.selected_root()?;
        let ids: Vec<_> = plan.trees.iter().map(|tree| tree.id).collect();
        if self.plan(root, &ids, protected)? != plan {
            return Err(
                "Il grafo delle sessioni è cambiato. Apri una nuova anteprima Codex.".into(),
            );
        }

        let executable = self
            .location
            .executable
            .as_deref()
            .ok_or("Eseguibile Codex non disponibile.")?;
        let path = plan
            .root
            .to_str()
            .ok_or("Percorso dell’archivio Codex non rappresentabile.")?;
        let setting = format!("sqlite_home={}", toml::Value::String(path.to_owned()));
        let native = |id| process::codex_delete(executable, &plan.root, &setting, id);
        Ok(delete(&plan, native, || self.archive()))
    }
}

fn waiting_for_children(error: &str) -> bool {
    let error = error.to_ascii_lowercase();
    error.contains("cannot delete")
        && ["attached thread", "child thread", "linked child"]
            .iter()
            .any(|reason| error.contains(reason))
}

fn delete(
    plan: &DeletionPlan,
    native: impl Fn(Uuid) -> Result<String, String>,
    observe: impl Fn() -> Result<Archive, String>,
) -> Deletion {
    let mut outcome = Deletion::default();
    let mut attempted = Vec::new();
    let mut messages = BTreeMap::new();
    for tree in &plan.trees {
        match native(tree.id) {
            Ok(message) => {
                messages.insert(tree.id, message);
                attempted.push((tree, None));
            }
            Err(error)
                if tree.descendants.iter().any(|child| child.present)
                    && waiting_for_children(&error) =>
            {
                attempted.push((tree, Some(error)));
            }
            Err(error) => {
                outcome.errors.push(format!("{}: {error}", tree.id));
                break;
            }
        }
    }
    if attempted.is_empty() {
        return outcome;
    }

    let mut archive = match observe() {
        Ok(archive) => archive,
        Err(error) => {
            outcome.errors.push(error);
            return outcome;
        }
    };
    let mut residuals_attempted = false;
    let mut retry = Vec::new();
    for (tree, refusal) in &attempted {
        if !archive.matches_reviewed_edges(tree) {
            outcome.errors.push(format!(
                "{}: relazioni cambiate durante l’eliminazione.",
                tree.id
            ));
            continue;
        }
        for child in tree.descendants.iter().rev() {
            if child.present && archive.present(child.id) {
                residuals_attempted = true;
                if let Err(error) = native(child.id) {
                    outcome.errors.push(format!("{}: {error}", child.id));
                }
            }
        }
        if let Some(error) = refusal {
            retry.push((*tree, error));
        }
    }
    if residuals_attempted {
        archive = match observe() {
            Ok(archive) => archive,
            Err(error) => {
                outcome.errors.push(error);
                retry.clear();
                archive
            }
        };
    }

    let mut retried = false;
    for (tree, initial_error) in retry {
        if !archive.matches_reviewed_edges(tree)
            || tree
                .descendants
                .iter()
                .any(|child| archive.present(child.id))
        {
            outcome.errors.push(format!("{}: {initial_error}", tree.id));
            continue;
        }
        if archive.present(tree.id) {
            retried = true;
            match native(tree.id) {
                Ok(message) => {
                    messages.insert(tree.id, message);
                }
                Err(error) => outcome.errors.push(format!("{}: {error}", tree.id)),
            }
        }
    }
    if retried {
        archive = match observe() {
            Ok(archive) => archive,
            Err(error) => {
                outcome.errors.push(error);
                archive
            }
        };
    }

    for (tree, _) in attempted {
        let ids = std::iter::once(tree.id).chain(
            tree.descendants
                .iter()
                .filter(|child| child.present)
                .map(|child| child.id),
        );
        for id in ids {
            if archive.present(id) {
                outcome.errors.push(format!(
                    "{id}: sessione ancora presente nell’archivio Codex."
                ));
            } else {
                if let Some(message) = messages.remove(&id) {
                    outcome.messages.push(message);
                }
                outcome.removed += 1;
                cleanup(&plan.root, id, &mut outcome);
            }
        }
    }
    outcome
}

fn cleanup(root: &Path, id: Uuid, outcome: &mut Deletion) {
    #[cfg(target_os = "macos")]
    outcome.errors.extend(
        crate::cleanup::after_native_delete(root, id)
            .into_iter()
            .map(|error| format!("{id}: pulizia aggiuntiva incompleta. {error}")),
    );
    #[cfg(not(target_os = "macos"))]
    let _ = (root, id, outcome);
}
