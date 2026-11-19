//! Metadata filtering and stable pagination for a fixed provider observation.
use agentmender_domain::{Session, SessionState};
use caseless::Caseless;
use serde::Deserialize;
use std::cmp::Ordering;
use unicode_normalization::UnicodeNormalization;

pub(crate) fn folded(value: &str) -> String {
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
    pub min_bytes: Option<u64>,
    pub max_bytes: Option<u64>,
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
}

pub(crate) fn compare<T: Ord>(left: Option<T>, right: Option<T>, ascending: bool) -> Ordering {
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
        let entries = sessions
            .into_iter()
            .filter(Session::is_main)
            .map(|session| {
                let searchable = [
                    Some(session.identity.native_id.as_str()),
                    session.title.as_deref(),
                    session.project.as_deref(),
                    session.workspace.as_deref(),
                    session.identity.namespace.as_deref(),
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
        Self { entries }
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

    pub fn page(&self, query: &InventoryQuery) -> InventoryPage<'_> {
        self.page_with_labels(query, &[])
    }

    pub(crate) fn matching<'a, 'b: 'a>(
        &'b self,
        query: &'a InventoryQuery,
        labels: &'a [String],
    ) -> impl Iterator<Item = &'b Session> + 'a {
        let search = folded(query.search.trim());
        self.entries
            .iter()
            .filter(move |item| {
                let meta = &item.session;
                query.state.is_none_or(|state| state == meta.state)
                    && query
                        .since
                        .is_none_or(|since| meta.last_active_at.is_some_and(|date| date >= since))
                    && query
                        .before
                        .is_none_or(|before| meta.last_active_at.is_some_and(|date| date < before))
                    && query
                        .min_bytes
                        .is_none_or(|min| meta.transcript_bytes.is_some_and(|bytes| bytes >= min))
                    && query
                        .max_bytes
                        .is_none_or(|max| meta.transcript_bytes.is_some_and(|bytes| bytes <= max))
                    && search.split_whitespace().all(|word| {
                        item.searchable
                            .iter()
                            .chain(labels)
                            .any(|field| field.contains(word))
                    })
            })
            .map(|entry| &entry.session)
    }

    pub(crate) fn page_with_labels(
        &self,
        query: &InventoryQuery,
        labels: &[String],
    ) -> InventoryPage<'_> {
        let mut records: Vec<_> = self.matching(query, labels).collect();
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
