//! Metadata filtering and stable pagination for a fixed provider observation.
use agentmender_domain::{ArtifactId, LocalSession, SessionState};
use caseless::Caseless;
use serde::{Deserialize, Serialize};
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

#[derive(Serialize)]
pub struct InventoryPage<'a> {
    pub total: usize,
    pub records: Vec<&'a LocalSession>,
}

struct Entry {
    session: LocalSession,
    searchable: Vec<String>,
}

pub struct LocalInventory {
    entries: Vec<Entry>,
    children: BTreeMap<ArtifactId, Vec<usize>>,
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
    pub fn new(sessions: Vec<LocalSession>) -> Self {
        let mut children: BTreeMap<_, Vec<_>> = BTreeMap::new();
        let entries = sessions
            .into_iter()
            .enumerate()
            .map(|(index, session)| {
                if let Some(parent) = &session.parent {
                    children.entry(parent.clone()).or_default().push(index);
                }
                let meta = &session.metadata;
                let searchable = [
                    Some(meta.native_session_id.as_str()),
                    meta.title.as_deref(),
                    meta.project_label.as_deref(),
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

    pub fn sessions(&self) -> impl Iterator<Item = &LocalSession> {
        self.entries.iter().map(|entry| &entry.session)
    }

    pub fn get(&self, position: usize) -> Option<&LocalSession> {
        self.entries.get(position).map(|entry| &entry.session)
    }

    pub fn children(&self, session: &LocalSession) -> impl Iterator<Item = &LocalSession> {
        self.children
            .get(&session.metadata.identity)
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
                let meta = &item.session.metadata;
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
                InventorySort::Activity => compare(
                    left.metadata.last_active_at,
                    right.metadata.last_active_at,
                    query.ascending,
                ),
                InventorySort::Created => compare(
                    left.metadata.created_at,
                    right.metadata.created_at,
                    query.ascending,
                ),
                InventorySort::Size => compare(
                    left.transcript_bytes,
                    right.transcript_bytes,
                    query.ascending,
                ),
            };
            order.then_with(|| left.metadata.identity.cmp(&right.metadata.identity))
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
