//! Counts from the same observation as the Claude Code inventory.
use crate::local::{SessionEvidence, Surface};
use agentmender_domain::{Session, SessionKind, SessionState};
use serde::Serialize;
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Exclusion {
    UnrecognizedLocator,
    ProtectedOrUnreadable,
    ChangedDuringScan,
    UnknownEntrypoint,
    IdentityConflict,
    UnverifiedFork,
    MissingIdentity,
    InvalidRecords,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ScanIssue {
    MalformedRecord,
    UnterminatedRecord,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Access {
    Available,
    Missing,
    Unavailable,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct TranscriptReport {
    pub main: usize,
    pub desktop_main: usize,
    pub cli_main: usize,
    pub mixed_main: usize,
    pub unknown_surface_main: usize,
    pub archived_main: usize,
    pub not_archived_main: usize,
    pub unknown_archive_state_main: usize,
    pub subagents: usize,
    pub forks: usize,
    pub sidechains: usize,
    pub unclassified: usize,
    pub partial: usize,
    pub exclusions: BTreeMap<Exclusion, usize>,
    pub partial_reasons: BTreeMap<ScanIssue, usize>,
}

impl TranscriptReport {
    pub(crate) fn exclude(&mut self, reason: Exclusion) {
        self.unclassified += 1;
        *self.exclusions.entry(reason).or_default() += 1;
    }

    pub(crate) fn record(&mut self, row: &Session, meta: &SessionEvidence) {
        if row.is_main() {
            self.main += 1;
            *match meta.surface {
                Some(Surface::Desktop) => &mut self.desktop_main,
                Some(Surface::Cli) => &mut self.cli_main,
                Some(Surface::Mixed) => &mut self.mixed_main,
                None => &mut self.unknown_surface_main,
            } += 1;
            *match row.state {
                SessionState::Archived => &mut self.archived_main,
                SessionState::Active => &mut self.not_archived_main,
                SessionState::Unknown => &mut self.unknown_archive_state_main,
            } += 1;
        } else {
            match row.kind {
                SessionKind::Subagent => self.subagents += 1,
                SessionKind::Fork => self.forks += 1,
                SessionKind::Unknown if meta.sidechain => self.sidechains += 1,
                _ => {}
            }
        }
        if !meta.metadata_complete {
            self.partial += 1;
            for &issue in &meta.scan_issues {
                *self.partial_reasons.entry(issue).or_default() += 1;
            }
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct RegistryRoot {
    #[serde(serialize_with = "agentmender_domain::path_serde::serialize")]
    pub path: PathBuf,
    pub access: Access,
    pub incomplete: bool,
    pub registrations: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct DesktopReport {
    pub roots: Vec<RegistryRoot>,
    pub registrations: usize,
    pub associated: usize,
    pub without_transcript: usize,
    pub unresolved: usize,
    pub without_cli_uuid: usize,
    pub unreadable_records: usize,
    pub incomplete: bool,
}

#[derive(Clone, Serialize)]
pub struct Reconciliation {
    pub transcripts: TranscriptReport,
    pub desktop: DesktopReport,
    pub projects_incomplete: bool,
    pub out_of_scope: [&'static str; 5],
}
