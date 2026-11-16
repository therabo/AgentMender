//! Metadata filtering and stable pagination for a fixed provider observation.
use agentmender_domain::{Session, SessionIdentity, SessionState};
use caseless::Caseless;
use serde::Deserialize;
use std::{cmp::Ordering, collections::BTreeMap};
use unicode_normalization::UnicodeNormalization;

fn folded(value: &str) -> String {
    value.nfkc().default_case_fold().nfkc().collect()
}

#[derive(Clone, Copy, Debug, Default, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InventorySort {
    #[default]
    Activity,
    Created,
    Size,
}

#[derive(Debug, Default)]
pub struct InventoryQuery {
    pub search: String,
    pub state: Option<SessionState>,
    pub since: Option<i64>,
    pub before: Option<i64>,
    pub sort: InventorySort,
    pub ascending: bool,
    pub offset: usize,
    pub limit: usize,
}

pub struct InventoryPage<'a> {
    pub total: usize,
    pub records: Vec<&'a Session>,
}

struct Entry {
    session: Session,
    searchable: Vec<String>,
}

pub struct LocalInventory {
    entries: Vec<Entry>,
    children: BTreeMap<SessionIdentity, Vec<usize>>,
}

fn compare<T: Ord>(left: Option<T>, right: Option<T>, ascending: bool) -> Ordering {
    match (left, right) {
        (Some(left), Some(right)) if ascending => left.cmp(&right),
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

impl LocalInventory {
    pub fn new(sessions: Vec<Session>) -> Self {
        let mut children: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let entries = sessions
            .into_iter()
            .enumerate()
            .map(|(index, session)| {
                if let Some(parent) = &session.parent {
                    children.entry(parent.clone()).or_default().push(index);
                }
                let searchable = [
                    Some(session.identity.native_id.as_str()),
                    session.title.as_deref(),
                    session.project.as_deref(),
                    session.workspace.as_deref(),
                ]
                .into_iter()
                .flatten()
                .map(folded)
                .collect();
                Entry {
                    session,
                    searchable,
                }
            })
            .collect();
        Self { entries, children }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.entries.iter().map(|entry| &entry.session)
    }

    pub fn get(&self, position: usize) -> Option<&Session> {
        self.entries.get(position).map(|entry| &entry.session)
    }

    pub fn children(&self, session: &Session) -> impl Iterator<Item = &Session> {
        self.children
            .get(&session.identity)
            .into_iter()
            .flatten()
            .map(|&index| &self.entries[index].session)
    }

    pub fn page(&self, query: &InventoryQuery) -> InventoryPage<'_> {
        let search = folded(query.search.trim());
        let words: Vec<_> = search.split_whitespace().collect();
        let mut records: Vec<_> = self
            .entries
            .iter()
            .filter(|item| {
                let meta = &item.session;
                query.state.is_none_or(|state| state == meta.state)
                    && query
                        .since
                        .is_none_or(|since| meta.last_active_at.is_some_and(|date| date >= since))
                    && query
                        .before
                        .is_none_or(|before| meta.last_active_at.is_some_and(|date| date < before))
                    && words
                        .iter()
                        .all(|word| item.searchable.iter().any(|field| field.contains(word)))
            })
            .map(|item| &item.session)
            .collect();
        records.sort_unstable_by(|left, right| {
            let order = match query.sort {
                InventorySort::Activity => {
                    compare(left.last_active_at, right.last_active_at, query.ascending)
                }
                InventorySort::Created => {
                    compare(left.created_at, right.created_at, query.ascending)
                }
                InventorySort::Size => compare(
                    left.transcript_bytes,
                    right.transcript_bytes,
                    query.ascending,
                ),
            };
            order.then_with(|| left.identity.cmp(&right.identity))
        });
        InventoryPage {
            total: records.len(),
            records: records
                .into_iter()
                .skip(query.offset)
                .take(query.limit)
                .collect(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use agentmender_domain::{ProviderKind, SessionKind};

    fn session(id: &str, value: Option<i64>) -> Session {
        Session {
            identity: SessionIdentity {
                provider: ProviderKind::Codex,
                state_root: "/fixture".into(),
                namespace: None,
                native_id: id.into(),
            },
            title: Some("Straße Café".into()),
            workspace: Some("/fixture/Atlas".into()),
            project: Some("Atlas".into()),
            created_at: value,
            last_active_at: value,
            state: SessionState::Archived,
            transcript_bytes: value.map(|size| u64::try_from(size).unwrap()),
            kind: SessionKind::Main,
            forked_from: None,
            parent: None,
        }
    }

    #[test]
    fn normalized_filter_and_all_sorts_keep_unknowns_last_and_ties_stable() {
        let inventory = LocalInventory::new(vec![
            session("z", None),
            session("b", Some(0)),
            session("c", Some(3)),
            session("a", Some(0)),
        ]);
        for sort in [
            InventorySort::Activity,
            InventorySort::Created,
            InventorySort::Size,
        ] {
            for ascending in [false, true] {
                let query = InventoryQuery {
                    sort,
                    ascending,
                    search: "STRASSE CAFE\u{301} atlas".into(),
                    limit: 4,
                    ..Default::default()
                };
                let page = inventory.page(&query);
                assert_eq!(page.total, 4);
                let ids: Vec<_> = page
                    .records
                    .iter()
                    .map(|row| row.identity.native_id.as_str())
                    .collect();
                assert_eq!(
                    ids,
                    if ascending {
                        ["a", "b", "c", "z"]
                    } else {
                        ["c", "a", "b", "z"]
                    }
                );
                let next = inventory.page(&InventoryQuery {
                    offset: 2,
                    limit: 1,
                    ..query
                });
                assert_eq!(next.records[0].identity.native_id, ids[2]);
            }
        }
    }
}
