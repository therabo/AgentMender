//! SearchIntent v1 compiles ordinary semantic lookup into the existing query
//! language. Only explicit constraints contribute to eligible scope.
use crate::{
    ApparentCodexCaller, ProjectSource, ProviderKind, QueryExpr, QueryRange, SessionState,
};

pub const SEARCH_INTENT_VERSION: &str = "1";
pub const SUBJECT_LIMIT: usize = 512;
pub const TERM_LIMIT: usize = 64;
pub const TERMS_LIMIT: usize = 8;
pub const EXPANSION_LIMIT: usize = 4;
pub const PROJECT_LIMIT: usize = 4;
pub const PROJECT_NAME_LIMIT: usize = 128;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SearchActivity {
    #[default]
    Any,
    Discussion,
    Investigation,
    Implementation,
    Testing,
    Validation,
    Decision,
    Troubleshooting,
}

#[derive(Debug, Clone, Copy, serde::Deserialize, serde::Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum Outcome {
    Investigated,
    Proposed,
    Attempted,
    Implemented,
    Tested,
    Validated,
    Resolved,
    Reverted,
    Failed,
    Unresolved,
}

impl SearchActivity {
    pub fn evidence_outcome(self) -> Option<Outcome> {
        match self {
            Self::Investigation => Some(Outcome::Investigated),
            Self::Implementation => Some(Outcome::Implemented),
            Self::Testing => Some(Outcome::Tested),
            Self::Validation => Some(Outcome::Validated),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResultPolicy {
    #[default]
    AllRelevant,
    SingleBest,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CoverageGoal {
    #[default]
    Standard,
    Exhaustive,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipMode {
    Include,
    Exclude,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MembershipBasis {
    Observed,
    Verified,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ProjectMembership {
    pub mode: MembershipMode,
    pub names: Vec<String>,
    pub basis: MembershipBasis,
}

#[derive(Debug, Clone, Copy, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ArchiveState {
    Active,
    Archived,
}

#[derive(Debug, Clone, Default, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchConstraints {
    pub project_membership: Option<ProjectMembership>,
    pub target_provider: Option<ProviderKind>,
    pub created_range: Option<QueryRange>,
    pub last_active_range: Option<QueryRange>,
    pub archive_state: Option<ArchiveState>,
    pub native_session_id: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchHints {
    pub terms: Vec<String>,
    #[serde(default)]
    pub project_names: Vec<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SearchIntent {
    pub schema_version: String,
    pub subject: String,
    #[serde(default)]
    pub activity: SearchActivity,
    pub required_outcome: Option<Outcome>,
    pub excluded_outcome: Option<Outcome>,
    #[serde(default)]
    pub result_policy: ResultPolicy,
    #[serde(default)]
    pub coverage_goal: CoverageGoal,
    pub hints: SearchHints,
    pub constraints: Option<SearchConstraints>,
}

pub struct ValidatedSearchIntent {
    intent: SearchIntent,
    hard_filter: Option<QueryExpr>,
    lexical: QueryExpr,
}

impl ValidatedSearchIntent {
    pub fn into_parts(self) -> (SearchIntent, Option<QueryExpr>, QueryExpr) {
        (self.intent, self.hard_filter, self.lexical)
    }

    pub fn intent(&self) -> &SearchIntent {
        &self.intent
    }

    pub fn hard_filter(&self) -> Option<&QueryExpr> {
        self.hard_filter.as_ref()
    }

    pub fn lexical(&self) -> &QueryExpr {
        &self.lexical
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
pub enum SearchIntentError {
    #[error("Set search_intent.schema_version to \"1\".")]
    UnsupportedSchemaVersion,
    #[error("Use a nonempty subject and bounded, distinct lexical terms and project names.")]
    InvalidSearchIntent,
    #[error("Use distinct, bounded project names and a canonical native session UUID.")]
    InvalidConstraints,
    #[error("The requested activity or required outcome also appears as the excluded outcome.")]
    ConflictingConstraints,
    #[error("Use a nonempty ordered date range with an explicit IANA zone or UTC bounds.")]
    InvalidDateRange,
    #[error("Supply 1–4 distinct new lexical alternatives under terms.")]
    InvalidQueryExpansion,
}

impl SearchIntentError {
    pub fn code(self) -> &'static str {
        match self {
            Self::UnsupportedSchemaVersion => "unsupported_schema_version",
            Self::InvalidSearchIntent => "invalid_search_intent",
            Self::InvalidConstraints => "invalid_constraints",
            Self::ConflictingConstraints => "conflicting_constraints",
            Self::InvalidDateRange => "invalid_date_range",
            Self::InvalidQueryExpansion => "invalid_query_expansion",
        }
    }
}

impl SearchIntent {
    pub fn validate(self) -> Result<ValidatedSearchIntent, SearchIntentError> {
        use SearchIntentError::*;
        if self.schema_version != SEARCH_INTENT_VERSION {
            return Err(UnsupportedSchemaVersion);
        }
        if !bounded_text(&self.subject, SUBJECT_LIMIT)
            || self.hints.terms.is_empty()
            || !bounded_list(&self.hints.terms, TERMS_LIMIT, TERM_LIMIT)
            || !bounded_list(&self.hints.project_names, PROJECT_LIMIT, PROJECT_NAME_LIMIT)
        {
            return Err(InvalidSearchIntent);
        }
        if let Some(excluded) = self.excluded_outcome
            && (self.required_outcome == Some(excluded)
                || self.activity.evidence_outcome() == Some(excluded))
        {
            return Err(ConflictingConstraints);
        }
        let hard_filter = self
            .constraints
            .as_ref()
            .map(SearchConstraints::compile)
            .transpose()?
            .flatten();
        let lexical = lexical_query(&self.hints.terms);
        Ok(ValidatedSearchIntent {
            intent: self,
            hard_filter,
            lexical,
        })
    }
}

impl SearchConstraints {
    fn compile(&self) -> Result<Option<QueryExpr>, SearchIntentError> {
        use SearchIntentError::*;
        let mut args = Vec::new();
        if let Some(project) = &self.project_membership {
            if project.names.is_empty()
                || !bounded_list(&project.names, PROJECT_LIMIT, PROJECT_NAME_LIMIT)
            {
                return Err(InvalidConstraints);
            }
            let clause = QueryExpr::Project {
                values: project.names.clone(),
                source: match project.basis {
                    MembershipBasis::Observed => ProjectSource::ObservedLabel,
                    MembershipBasis::Verified => ProjectSource::VerifiedLabel,
                },
            };
            args.push(match project.mode {
                MembershipMode::Include => clause,
                MembershipMode::Exclude => QueryExpr::Not {
                    arg: Box::new(clause),
                },
            });
        }
        if let Some(provider) = self.target_provider {
            args.push(QueryExpr::Provider {
                values: vec![provider],
            });
        }
        let range = |value: &QueryRange| {
            value
                .bounds()
                .map(QueryRange::Utc)
                .map_err(|_| InvalidDateRange)
        };
        if let Some(created) = &self.created_range {
            args.push(QueryExpr::Created {
                range: range(created)?,
            });
        }
        if let Some(active) = &self.last_active_range {
            args.push(QueryExpr::LastActive {
                range: range(active)?,
            });
        }
        if let Some(archive) = self.archive_state {
            args.push(QueryExpr::State {
                values: vec![match archive {
                    ArchiveState::Active => SessionState::Active,
                    ArchiveState::Archived => SessionState::Archived,
                }],
            });
        }
        if let Some(id) = &self.native_session_id {
            if ApparentCodexCaller::from_metadata(id, None).is_none() {
                return Err(InvalidConstraints);
            }
            args.push(QueryExpr::NativeId {
                values: vec![id.clone()],
            });
        }
        Ok((!args.is_empty()).then_some(QueryExpr::And { args }))
    }
}

pub fn compile_expansion(terms: &[String]) -> Result<QueryExpr, SearchIntentError> {
    if terms.is_empty() || !bounded_list(terms, EXPANSION_LIMIT, TERM_LIMIT) {
        return Err(SearchIntentError::InvalidQueryExpansion);
    }
    Ok(lexical_query(terms))
}

fn lexical_query(terms: &[String]) -> QueryExpr {
    QueryExpr::Or {
        args: terms
            .iter()
            .map(|value| QueryExpr::Text {
                value: value.trim().to_owned(),
                kind: Default::default(),
                alternatives: vec![],
            })
            .collect(),
    }
}

fn bounded_list(values: &[String], count: usize, length: usize) -> bool {
    let mut distinct = std::collections::BTreeSet::new();
    values.len() <= count
        && values
            .iter()
            .all(|v| bounded_text(v, length) && distinct.insert(v.trim().to_lowercase()))
}

fn bounded_text(text: &str, limit: usize) -> bool {
    !text.trim().is_empty() && text.len() <= limit && !text.chars().any(char::is_control)
}
