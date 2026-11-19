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

#[derive(Debug, Serialize)]
pub struct DeletionPlan {
    #[serde(skip)]
    root: PathBuf,
    trees: Vec<Tree>,
}

#[derive(Debug, Serialize)]
struct Tree {
    id: Uuid,
    descendants: Vec<Descendant>,
}

#[derive(Debug, Serialize)]
struct Descendant {
    id: Uuid,
    parent: Uuid,
    archived: bool,
    present: bool,
    depth: usize,
}

#[derive(Default)]
pub struct Deletion {
    pub native_succeeded: usize,
    pub verified_absent: usize,
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

    // A reviewed node can still be addressed if native deletion already erased
    // its inbound edge. A new parent or unreviewed child makes another native
    // call unsafe because the command may operate on descendants as well.
    fn blocked_writes(&self, tree: &Tree, changes: &mut BTreeSet<String>) -> BTreeSet<Uuid> {
        let reviewed: BTreeMap<_, _> = std::iter::once((tree.id, (None, true)))
            .chain(
                tree.descendants
                    .iter()
                    .map(|child| (child.id, (Some(child.parent), child.present))),
            )
            .collect();
        let mut blocked = BTreeSet::new();
        for (&id, &(parent, was_present)) in &reviewed {
            if self.nodes.get(&id).is_some_and(|node| {
                node.uncertain
                    || (!was_present && node.present)
                    || node.parents.iter().any(|found| Some(*found) != parent)
            }) {
                blocked.insert(id);
                changes.insert(format!(
                    "{id}: relazione o presenza cambiata dopo l’anteprima."
                ));
            }

            for &child in self.children.get(&id).into_iter().flatten() {
                if reviewed.get(&child).is_none_or(|&(expected, was_present)| {
                    expected != Some(id) || (!was_present && self.present(child))
                }) {
                    blocked.insert(id);
                    changes.insert(format!(
                        "{id}: discendente {child} fuori dal piano confermato."
                    ));
                }
            }
        }
        for child in tree.descendants.iter().rev() {
            if blocked.contains(&child.id)
                && self
                    .children
                    .get(&child.parent)
                    .is_some_and(|ids| ids.contains(&child.id))
            {
                blocked.insert(child.parent);
            }
        }
        blocked
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
        let archive = self.archive()?;
        let protected = protected
            .iter()
            .filter_map(|id| Uuid::parse_str(id).ok())
            .collect();
        let trees = ids
            .into_iter()
            .map(|id| archive.tree(id, &protected))
            .collect::<Result<_, _>>()?;
        Ok(DeletionPlan { root, trees })
    }

    pub fn delete_reviewed(&self, plan: DeletionPlan) -> Result<Deletion, String> {
        let _deletion = self.deletion.lock().expect("Codex deletion");
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
        Ok(delete(&plan, native, || {
            if self.selected_root()? != plan.root {
                return Err(
                    "L’archivio Codex è cambiato. Impossibile verificare l’eliminazione.".into(),
                );
            }
            self.archive()
        }))
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
    let mut native_successes = BTreeMap::new();
    let mut run_native = |id| {
        let message = native(id)?;
        native_successes.insert(id, message);
        Ok::<_, String>(())
    };
    for tree in &plan.trees {
        match run_native(tree.id) {
            Ok(()) => attempted.push((tree, None)),
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

    let targets: BTreeSet<_> = attempted
        .iter()
        .flat_map(|(tree, _)| {
            std::iter::once(tree.id).chain(
                tree.descendants
                    .iter()
                    .filter(|child| child.present)
                    .map(|child| child.id),
            )
        })
        .collect();
    let mut verified_absent = BTreeSet::new();
    let mut changes = BTreeSet::new();
    let mut verify = |errors: &mut Vec<String>| match observe() {
        Ok(archive) => {
            verified_absent = targets
                .iter()
                .copied()
                .filter(|id| !archive.present(*id))
                .collect();
            Some(archive)
        }
        Err(error) => {
            errors.push(error);
            None
        }
    };

    let mut archive = verify(&mut outcome.errors);
    if let Some(observed) = &archive {
        let mut residuals_attempted = false;
        for (tree, _) in &attempted {
            let blocked = observed.blocked_writes(tree, &mut changes);
            for child in tree.descendants.iter().rev() {
                if child.present && observed.present(child.id) && !blocked.contains(&child.id) {
                    residuals_attempted = true;
                    if let Err(error) = run_native(child.id) {
                        outcome.errors.push(format!("{}: {error}", child.id));
                    }
                }
            }
        }
        if residuals_attempted {
            archive = verify(&mut outcome.errors);
        }
    }

    if let Some(observed) = &archive {
        let mut retried = false;
        for (tree, refusal) in &attempted {
            let Some(error) = refusal else { continue };
            let blocked = observed.blocked_writes(tree, &mut changes);
            if blocked.contains(&tree.id)
                || tree
                    .descendants
                    .iter()
                    .any(|child| observed.present(child.id))
            {
                outcome.errors.push(format!("{}: {error}", tree.id));
            } else if observed.present(tree.id) {
                retried = true;
                if let Err(error) = run_native(tree.id) {
                    outcome.errors.push(format!("{}: {error}", tree.id));
                }
            }
        }
        if retried {
            archive = verify(&mut outcome.errors);
        }
    } else {
        for (tree, refusal) in &attempted {
            if let Some(error) = refusal {
                outcome.errors.push(format!("{}: {error}", tree.id));
            }
        }
    }

    if let Some(observed) = &archive {
        for (tree, _) in &attempted {
            observed.blocked_writes(tree, &mut changes);
        }
        for id in targets.difference(&verified_absent) {
            let reason = if observed.nodes.get(id).is_some_and(|node| node.uncertain) {
                "presenza della sessione non verificabile"
            } else {
                "sessione ancora presente nell’archivio Codex"
            };
            outcome.errors.push(format!("{id}: {reason}."));
        }
    }
    outcome.errors.extend(changes);
    outcome.native_succeeded = native_successes.len();
    outcome.verified_absent = verified_absent.len();
    outcome.messages = attempted
        .iter()
        .filter(|(tree, _)| verified_absent.contains(&tree.id))
        .filter_map(|(tree, _)| native_successes.get(&tree.id).cloned())
        .collect();

    // Native success and verified cascades can name the same UUID. Clean the
    // union once, even when an observation needed for residuals failed.
    verified_absent.extend(native_successes.into_keys());
    for id in verified_absent {
        cleanup(&plan.root, id, &mut outcome);
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
