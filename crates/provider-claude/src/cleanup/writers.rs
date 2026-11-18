//! Offline checks for the macOS PID domain and known Claude executables.
use super::Refusal;
use crate::{
    read::verify,
    storage::{Budget, EntryKind},
};
use agentmender_provider_api::{filesystem, process};
use rustix::process::{Pid, test_kill_process};
use serde::Deserialize;
use std::{collections::BTreeMap, path::Path};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Marker {
    #[serde(alias = "session_id")]
    session_id: Option<String>,
    pid: Option<i32>,
    pid_domain: Option<String>,
    proc_start: Option<String>,
    #[serde(alias = "state")]
    status: Option<String>,
}

fn process_row(line: &str) -> Option<(i32, String, &str)> {
    let mut rest = line.trim_start();
    let mut next = || {
        let (word, tail) = rest.split_once(char::is_whitespace)?;
        rest = tail.trim_start();
        Some(word)
    };
    let pid = next()?.parse().ok()?;
    let start = [next()?, next()?, next()?, next()?, next()?].join(" ");
    valid_start(&start).then_some((pid, start, rest))
}

fn valid_start(start: &str) -> bool {
    chrono::NaiveDateTime::parse_from_str(start, "%a %b %e %H:%M:%S %Y").is_ok()
}

fn claude_executable(command: &str) -> bool {
    let command = command.to_ascii_lowercase();
    command.contains("/claude.app/")
        || command.contains("/claude-code/")
        || Path::new(&command).file_name().is_some_and(|name| {
            matches!(name.to_str(), Some("claude" | "claude-code" | "claude.exe"))
        })
}

pub(super) fn closed(root: &Path, id: &str) -> Result<(), Refusal> {
    let output = process::claude_process_table().map_err(|_| Refusal::WriterUnverified)?;
    let mut starts = BTreeMap::new();
    for line in output.lines().filter(|line| !line.trim().is_empty()) {
        let (pid, start, command) = process_row(line).ok_or(Refusal::WriterUnverified)?;
        if claude_executable(command) {
            return Err(Refusal::ActiveWriter);
        }
        starts.insert(pid, start);
    }
    if starts.is_empty() {
        return Err(Refusal::WriterUnverified);
    }
    let mut budget = Budget::new(1024);
    let mut bytes_left = 4 * 1024 * 1024;
    for entry in budget.list(&root.join("sessions")) {
        if budget.expired() {
            return Err(Refusal::WriterUnverified);
        }
        let filename = entry.path.file_stem().and_then(|part| part.to_str());
        let pid_from_name = filename.and_then(|value| value.parse::<i32>().ok());
        if entry.kind != EntryKind::File || entry.path.extension().is_none_or(|ext| ext != "json") {
            return Err(Refusal::WriterUnverified);
        }
        let file = filesystem::open_observed_read(root, &entry.path)
            .map_err(|_| Refusal::WriterUnverified)?;
        let revision =
            crate::Revision::read(&file, &entry.path).map_err(|_| Refusal::WriterUnverified)?;
        let data = filesystem::read_bounded(root, &entry.path, 256 * 1024)
            .map_err(|_| Refusal::WriterUnverified)?;
        if data.len() > bytes_left {
            return Err(Refusal::WriterUnverified);
        }
        bytes_left -= data.len();
        verify(root, &entry.path, &revision).map_err(|_| Refusal::WriterUnverified)?;
        let marker: Marker =
            serde_json::from_slice(&data).map_err(|_| Refusal::WriterUnverified)?;
        if filename == Some(id)
            && marker
                .session_id
                .as_deref()
                .is_some_and(|value| value != id)
        {
            return Err(Refusal::WriterUnverified);
        }
        if marker
            .session_id
            .as_deref()
            .unwrap_or_else(|| filename.unwrap_or(""))
            != id
        {
            if marker.session_id.is_none() && pid_from_name.is_some() {
                return Err(Refusal::WriterUnverified);
            }
            continue;
        }
        if let Some(pid) = pid_from_name
            && marker.pid.is_some_and(|recorded| recorded != pid)
        {
            return Err(Refusal::WriterUnverified);
        }
        check_marker(marker, pid_from_name, &starts)?;
    }
    if budget.issues != 0 {
        return Err(Refusal::WriterUnverified);
    }
    Ok(())
}

fn check_marker(
    marker: Marker,
    from_name: Option<i32>,
    starts: &BTreeMap<i32, String>,
) -> Result<(), Refusal> {
    if marker
        .pid_domain
        .as_deref()
        .is_some_and(|domain| domain != "darwin")
    {
        return Err(Refusal::WriterUnverified);
    }
    let Some(pid) = marker.pid.or(from_name) else {
        return match marker.status.as_deref() {
            Some("stopped" | "completed" | "failed") => Ok(()),
            _ => Err(Refusal::WriterUnverified),
        };
    };
    let pid_handle = Pid::from_raw(pid)
        .filter(|_| pid > 1)
        .ok_or(Refusal::WriterUnverified)?;
    match test_kill_process(pid_handle) {
        Err(rustix::io::Errno::SRCH) => Ok(()),
        Err(_) => Err(Refusal::WriterUnverified),
        Ok(()) => {
            let expected = marker.proc_start.ok_or(Refusal::ActiveWriter)?;
            let expected = expected.split_whitespace().collect::<Vec<_>>().join(" ");
            let actual = starts.get(&pid).ok_or(Refusal::WriterUnverified)?;
            if !valid_start(&expected) {
                return Err(Refusal::WriterUnverified);
            }
            if &expected == actual {
                return Err(Refusal::ActiveWriter);
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn checks_exact_pid_start_and_blocks_legacy_live_markers() {
        let pid = i32::try_from(std::process::id()).unwrap();
        let marker = |start: Option<&str>| Marker {
            session_id: None,
            pid: Some(pid),
            pid_domain: Some("darwin".into()),
            proc_start: start.map(str::to_owned),
            status: None,
        };
        let baseline = "Wed Nov 18 17:00:00 2026";
        let starts = BTreeMap::from([(pid, baseline.into())]);
        assert_eq!(
            check_marker(marker(Some(baseline)), None, &starts),
            Err(Refusal::ActiveWriter)
        );
        assert_eq!(
            check_marker(marker(None), None, &starts),
            Err(Refusal::ActiveWriter)
        );
        assert_eq!(
            check_marker(marker(Some("Wed Nov 18 17:01:00 2026")), None, &starts),
            Ok(())
        );
        assert_eq!(
            check_marker(marker(Some("n/a")), None, &starts),
            Err(Refusal::WriterUnverified)
        );
        assert!(claude_executable(
            "/Applications/Claude.app/Contents/Frameworks/Claude Helper"
        ));
        assert_eq!(
            process_row("99 Wed Nov 18 17:00:00 2026 /Applications/Claude.app/Claude Helper")
                .unwrap()
                .2,
            "/Applications/Claude.app/Claude Helper"
        );
    }
}
