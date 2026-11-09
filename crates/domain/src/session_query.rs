//! Typed read-only search conditions. Three-valued logic never turns missing
//! metadata or unread conversation text into proof of absence. See ADR-0027.
use crate::{ProviderKind, SessionMetadata, SessionSearchHit, SessionState};
use chrono::{NaiveDate, TimeZone};
use chrono_tz::Tz;
use serde::{Deserialize, Serialize};
use std::{cell::OnceCell, cmp::Ordering};

/// Strong Kleene truth tables; in particular NOT UNKNOWN is UNKNOWN.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Truth {
    True,
    False,
    Unknown,
}
impl Truth {
    pub fn and(self, other: Self) -> Self {
        match (self, other) {
            (Self::False, _) | (_, Self::False) => Self::False,
            (Self::True, Self::True) => Self::True,
            _ => Self::Unknown,
        }
    }
    pub fn or(self, other: Self) -> Self {
        match (self, other) {
            (Self::True, _) | (_, Self::True) => Self::True,
            (Self::False, Self::False) => Self::False,
            _ => Self::Unknown,
        }
    }
    pub fn negate(self) -> Self {
        match self {
            Self::True => Self::False,
            Self::False => Self::True,
            Self::Unknown => Self::Unknown,
        }
    }
    fn known(value: bool) -> Self {
        if value { Self::True } else { Self::False }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchMode {
    #[default]
    Exhaustive,
    Ranked,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchSort {
    #[default]
    Identity,
    Created,
    LastActive,
    Relevance,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageRequirement {
    #[default]
    BestEffort,
    Complete,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct StructuredQuery {
    pub filter: QueryExpr,
    #[serde(default)]
    pub mode: SearchMode,
    #[serde(default)]
    pub sort: SearchSort,
    #[serde(default)]
    pub coverage: CoverageRequirement,
}
impl StructuredQuery {
    pub fn valid(&self) -> bool {
        // Exhaustive enumeration never silently becomes Top-K relevance search.
        (self.mode != SearchMode::Exhaustive || self.sort != SearchSort::Relevance)
            && self.filter.valid()
    }
    /// Ascending identity/time, descending relevance, always exact identity ties.
    /// The adapter must apply this ordering before paginating the eligible set.
    pub fn compare(&self, a: &SessionSearchHit, b: &SessionSearchHit) -> Ordering {
        match self.sort {
            SearchSort::Identity => Ordering::Equal,
            SearchSort::Created => cmp_time(a.metadata.created_at, b.metadata.created_at),
            SearchSort::LastActive => {
                cmp_time(a.metadata.last_active_at, b.metadata.last_active_at)
            }
            SearchSort::Relevance => b.score.cmp(&a.score),
        }
        .then_with(|| a.metadata.identity.cmp(&b.metadata.identity))
    }
}
fn cmp_time(a: Option<i64>, b: Option<i64>) -> Ordering {
    match (a, b) {
        (Some(a), Some(b)) => a.cmp(&b),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ProjectSource {
    /// Observed cwd basename only; never verified project ownership.
    #[default]
    ObservedLabel,
    VerifiedLabel,
    VerifiedRoot,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TextMatch {
    /// Unicode lowercase substring; explicit alternatives expand recall.
    #[default]
    Contains,
    /// Whitespace-normalized, Unicode lowercase contiguous phrase.
    Phrase,
    /// Complete message equality after Unicode lowercase conversion.
    Exact,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum QueryExpr {
    And {
        args: Vec<Self>,
    },
    Or {
        args: Vec<Self>,
    },
    Not {
        arg: Box<Self>,
    },
    Provider {
        values: Vec<ProviderKind>,
    },
    Project {
        values: Vec<String>,
        #[serde(default)]
        source: ProjectSource,
    },
    ProjectUnknown {
        #[serde(default)]
        source: ProjectSource,
    },
    State {
        values: Vec<SessionState>,
    },
    NativeId {
        values: Vec<String>,
    },
    Created {
        range: QueryRange,
    },
    LastActive {
        range: QueryRange,
    },
    Text {
        value: String,
        #[serde(default)]
        kind: TextMatch,
        #[serde(default)]
        alternatives: Vec<String>,
    },
    MessageTime {
        range: QueryRange,
    },
}

/// UTC Unix seconds with inclusive start and exclusive end. Unbounded sides are
/// explicit nulls/omissions; at least one finite bound is required in a query.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeRange {
    #[serde(default)]
    pub start: Option<i64>,
    #[serde(default)]
    pub end: Option<i64>,
}
impl TimeRange {
    pub fn valid(&self) -> bool {
        (self.start.is_some() || self.end.is_some())
            && self.start.zip(self.end).is_none_or(|(a, b)| a < b)
    }
    pub fn contains(&self, timestamp: i64) -> bool {
        self.start.is_none_or(|a| timestamp >= a) && self.end.is_none_or(|b| timestamp < b)
    }
    /// Includes both named calendar dates. A calendar day is not assumed to be
    /// 86,400 seconds: each boundary resolves independently using the IANA zone.
    pub fn calendar_dates(
        start: Option<&str>,
        end_inclusive: Option<&str>,
        timezone: &str,
    ) -> Result<Self, QueryTimeError> {
        let tz: Tz = timezone.parse().map_err(|_| QueryTimeError)?;
        let parse = |s: &str| {
            if s.len() != 10 {
                return Err(QueryTimeError);
            }
            NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| QueryTimeError)
        };
        let boundary = |date: NaiveDate| {
            let naive = date.and_hms_opt(0, 0, 0).ok_or(QueryTimeError)?;
            // Ambiguous/nonexistent midnight refuses rather than guessing.
            tz.from_local_datetime(&naive)
                .single()
                .map(|v| v.timestamp())
                .ok_or(QueryTimeError)
        };
        let start = start.map(|s| parse(s).and_then(boundary)).transpose()?;
        let end = end_inclusive
            .map(|s| {
                let next = parse(s)?.succ_opt().ok_or(QueryTimeError)?;
                boundary(next)
            })
            .transpose()?;
        let range = Self { start, end };
        range.valid().then_some(range).ok_or(QueryTimeError)
    }
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[error("invalid date, timezone, ambiguous boundary or empty/reversed interval")]
pub struct QueryTimeError;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CalendarRange {
    #[serde(default)]
    pub start_date: Option<String>,
    #[serde(default)]
    pub end_date: Option<String>,
    pub timezone: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum QueryRange {
    Utc(TimeRange),
    Calendar(CalendarRange),
}
impl QueryRange {
    pub fn bounds(&self) -> Result<TimeRange, QueryTimeError> {
        match self {
            Self::Utc(range) => range.valid().then_some(*range).ok_or(QueryTimeError),
            Self::Calendar(range) => TimeRange::calendar_dates(
                range.start_date.as_deref(),
                range.end_date.as_deref(),
                &range.timezone,
            ),
        }
    }
}

/// Bounded incremental observations bound to the exact query by the enclosing
/// authenticated cursor. No conversation text is retained. Atom IDs are stable
/// preorder node ordinals; only text and message-time leaves may be recorded.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueryObservation {
    pub matched_atoms: Vec<usize>,
    pub missing_timestamps: bool,
}

/// Parsed supported human-visible messages only, never system/tool instructions.
pub struct QueryText<'a> {
    pub text: &'a str,
    pub timestamp: Option<i64>,
}
pub struct QueryFacts<'a> {
    pub metadata: &'a SessionMetadata,
    pub messages: &'a [QueryText<'a>],
    /// False if unread, unsupported, redacted, truncated, or changed portions
    /// could contain a match. Full source traversal alone is insufficient.
    pub content_complete: bool,
    pub verified_project: Option<&'a str>,
    pub verified_root: Option<&'a str>,
}
impl<'a> QueryFacts<'a> {
    pub fn metadata_only(metadata: &'a SessionMetadata) -> Self {
        Self {
            metadata,
            messages: &[],
            content_complete: false,
            verified_project: None,
            verified_root: None,
        }
    }
    fn project(&self, source: ProjectSource) -> Option<&str> {
        match source {
            ProjectSource::ObservedLabel => self.metadata.project_label.as_deref(),
            ProjectSource::VerifiedLabel => self.verified_project,
            ProjectSource::VerifiedRoot => self.verified_root,
        }
    }
}
impl QueryExpr {
    pub fn valid(&self) -> bool {
        let mut remaining = 128;
        self.validate(0, &mut remaining)
    }
    fn validate(&self, depth: usize, remaining: &mut usize) -> bool {
        if depth > 12 || *remaining == 0 {
            return false;
        }
        *remaining -= 1;
        let strings = |values: &[String]| {
            !values.is_empty() && values.len() <= 32 && values.iter().all(|s| valid_text(s, 256))
        };
        match self {
            Self::And { args } | Self::Or { args } => {
                !args.is_empty()
                    && args.len() <= 32
                    && args.iter().all(|v| v.validate(depth + 1, remaining))
            }
            Self::Not { arg } => arg.validate(depth + 1, remaining),
            Self::Provider { values } => !values.is_empty() && values.len() <= 2,
            Self::Project { values, .. } | Self::NativeId { values } => strings(values),
            Self::ProjectUnknown { .. } => true,
            Self::State { values } => !values.is_empty() && values.len() <= 3,
            Self::Created { range } | Self::LastActive { range } | Self::MessageTime { range } => {
                range.bounds().is_ok()
            }
            Self::Text {
                value,
                alternatives,
                ..
            } => {
                valid_text(value, 256)
                    && alternatives.len() <= 16
                    && alternatives.iter().all(|v| valid_text(v, 256))
            }
        }
    }
    pub fn needs_content(&self) -> bool {
        match self {
            Self::Text { .. } | Self::MessageTime { .. } => true,
            Self::And { args } | Self::Or { args } => args.iter().any(Self::needs_content),
            Self::Not { arg } => arg.needs_content(),
            _ => false,
        }
    }
    /// Explicit lexical alternatives for candidate generation/ranking, not
    /// additional hard conditions or an implicit semantic interpretation.
    pub fn text_terms(&self) -> Vec<String> {
        let mut result = Vec::new();
        self.collect_terms(&mut result);
        result.sort();
        result.dedup();
        result
    }
    fn collect_terms(&self, result: &mut Vec<String>) {
        match self {
            Self::And { args } | Self::Or { args } => {
                for arg in args {
                    arg.collect_terms(result);
                }
            }
            Self::Not { .. } => {}
            Self::Text {
                value,
                alternatives,
                ..
            } => {
                result.push(value.to_lowercase());
                result.extend(alternatives.iter().map(|v| v.to_lowercase()));
            }
            _ => {}
        }
    }
    pub fn observation_valid(&self, observation: &QueryObservation) -> bool {
        if !self.valid()
            || observation.matched_atoms.len() > 128
            || observation.matched_atoms.windows(2).any(|v| v[0] >= v[1])
        {
            return false;
        }
        let mut atoms = Vec::new();
        self.visit_content(&mut 0, &mut |index, _| atoms.push(index));
        observation
            .matched_atoms
            .iter()
            .all(|v| atoms.binary_search(v).is_ok())
    }
    fn visit_content(&self, next: &mut usize, visitor: &mut impl FnMut(usize, &Self)) {
        let index = *next;
        *next += 1;
        match self {
            Self::And { args } | Self::Or { args } => {
                for arg in args {
                    arg.visit_content(next, visitor);
                }
            }
            Self::Not { arg } => arg.visit_content(next, visitor),
            Self::Text { .. } | Self::MessageTime { .. } => visitor(index, self),
            _ => {}
        }
    }
    /// Call only for supported, safe, source-validated messages. Source gaps,
    /// suppression, truncation and unread portions keep content_complete false.
    pub fn observe(&self, observation: &mut QueryObservation, message: &QueryText<'_>) {
        observation.missing_timestamps |= message.timestamp.is_none();
        let prepared = PreparedText::new(message.text);
        self.visit_content(&mut 0, &mut |index, atom| {
            let matched = match atom {
                Self::Text {
                    value,
                    kind,
                    alternatives,
                } => std::iter::once(value)
                    .chain(alternatives)
                    .any(|v| prepared.matches(v, *kind)),
                Self::MessageTime { range } => range
                    .bounds()
                    .ok()
                    .zip(message.timestamp)
                    .is_some_and(|(range, t)| range.contains(t)),
                _ => false,
            };
            if matched && let Err(at) = observation.matched_atoms.binary_search(&index) {
                observation.matched_atoms.insert(at, index);
            }
        });
    }
    pub fn evaluate_observed(
        &self,
        metadata: &SessionMetadata,
        observation: &QueryObservation,
        content_complete: bool,
    ) -> Truth {
        if !self.observation_valid(observation) {
            return Truth::Unknown;
        }
        self.evaluate_incremental(metadata, observation, content_complete, &mut 0)
    }
    fn evaluate_incremental(
        &self,
        metadata: &SessionMetadata,
        observation: &QueryObservation,
        content_complete: bool,
        next: &mut usize,
    ) -> Truth {
        let index = *next;
        *next += 1;
        match self {
            Self::And { args } => args.iter().fold(Truth::True, |v, arg| {
                v.and(arg.evaluate_incremental(metadata, observation, content_complete, next))
            }),
            Self::Or { args } => args.iter().fold(Truth::False, |v, arg| {
                v.or(arg.evaluate_incremental(metadata, observation, content_complete, next))
            }),
            Self::Not { arg } => arg
                .evaluate_incremental(metadata, observation, content_complete, next)
                .negate(),
            Self::Text { .. } | Self::MessageTime { .. } => {
                if observation.matched_atoms.binary_search(&index).is_ok() {
                    Truth::True
                } else if !content_complete
                    || (matches!(self, Self::MessageTime { .. }) && observation.missing_timestamps)
                {
                    Truth::Unknown
                } else {
                    Truth::False
                }
            }
            _ => self.metadata_truth(metadata),
        }
    }
    pub fn metadata_truth(&self, metadata: &SessionMetadata) -> Truth {
        self.evaluate(&QueryFacts::metadata_only(metadata))
    }
    pub fn evaluate(&self, facts: &QueryFacts<'_>) -> Truth {
        match self {
            Self::And { args } => args
                .iter()
                .fold(Truth::True, |v, arg| v.and(arg.evaluate(facts))),
            Self::Or { args } => args
                .iter()
                .fold(Truth::False, |v, arg| v.or(arg.evaluate(facts))),
            Self::Not { arg } => arg.evaluate(facts).negate(),
            Self::Provider { values } => {
                Truth::known(values.contains(&facts.metadata.identity.provider))
            }
            Self::Project { values, source } => {
                facts.project(*source).map_or(Truth::Unknown, |s| {
                    Truth::known(values.iter().any(|v| v == s))
                })
            }
            Self::ProjectUnknown { source } => Truth::known(facts.project(*source).is_none()),
            Self::State { values } => {
                if facts.metadata.state == SessionState::Unknown {
                    Truth::Unknown
                } else {
                    Truth::known(values.contains(&facts.metadata.state))
                }
            }
            Self::NativeId { values } => {
                Truth::known(values.contains(&facts.metadata.native_session_id))
            }
            Self::Created { range } => timestamp_truth(range, facts.metadata.created_at),
            Self::LastActive { range } => timestamp_truth(range, facts.metadata.last_active_at),
            Self::Text {
                value,
                kind,
                alternatives,
            } => {
                let matched = facts.messages.iter().any(|m| {
                    std::iter::once(value)
                        .chain(alternatives)
                        .any(|v| text_match(m.text, v, *kind))
                });
                if matched {
                    Truth::True
                } else if facts.content_complete {
                    Truth::False
                } else {
                    Truth::Unknown
                }
            }
            Self::MessageTime { range } => {
                let Ok(bounds) = range.bounds() else {
                    return Truth::Unknown;
                };
                if facts
                    .messages
                    .iter()
                    .any(|m| m.timestamp.is_some_and(|t| bounds.contains(t)))
                {
                    Truth::True
                } else if facts.content_complete
                    && facts.messages.iter().all(|m| m.timestamp.is_some())
                {
                    Truth::False
                } else {
                    Truth::Unknown
                }
            }
        }
    }
}
fn valid_text(text: &str, max: usize) -> bool {
    !text.trim().is_empty() && text.len() <= max && !text.chars().any(char::is_control)
}
fn timestamp_truth(range: &QueryRange, timestamp: Option<i64>) -> Truth {
    range
        .bounds()
        .ok()
        .zip(timestamp)
        .map_or(Truth::Unknown, |(r, t)| Truth::known(r.contains(t)))
}
fn normalized_phrase(value: &str) -> String {
    value
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}
struct PreparedText<'a> {
    original: &'a str,
    lower: OnceCell<String>,
    phrase: OnceCell<String>,
}
impl<'a> PreparedText<'a> {
    fn new(original: &'a str) -> Self {
        Self {
            original,
            lower: OnceCell::new(),
            phrase: OnceCell::new(),
        }
    }
    fn matches(&self, query: &str, kind: TextMatch) -> bool {
        // A large message is folded once per observed record, not once per
        // query leaf/alternative. Both allocations are bounded by record size.
        match kind {
            TextMatch::Contains => self
                .lower
                .get_or_init(|| self.original.to_lowercase())
                .contains(&query.to_lowercase()),
            TextMatch::Exact => {
                *self.lower.get_or_init(|| self.original.to_lowercase()) == query.to_lowercase()
            }
            TextMatch::Phrase => self
                .phrase
                .get_or_init(|| normalized_phrase(self.original))
                .contains(&normalized_phrase(query)),
        }
    }
}
fn text_match(text: &str, query: &str, kind: TextMatch) -> bool {
    PreparedText::new(text).matches(query, kind)
}

/// Only explicit RFC3339 timestamps are admitted. Missing/invalid values remain
/// unknown; source position is never substituted for message activity time.
pub fn parse_message_timestamp(value: &str) -> Option<i64> {
    if value.len() > 64 {
        return None;
    }
    chrono::DateTime::parse_from_rfc3339(value)
        .ok()
        .map(|v| v.timestamp())
}
