//! Read-only process prerequisite for the exact closed Codex Desktop profile.
use std::path::Path;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DesktopWriterInventory {
    Closed,
    Active,
    Unknown,
}

/// Bind the admitted root owner to the same unprivileged principal as the census.
pub fn desktop_writer_scope_matches_owner(uid: u32) -> bool {
    #[cfg(target_os = "macos")]
    {
        crate::macos_sandbox::current_unprivileged_uid() == Some(uid)
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = uid;
        false
    }
}

/// Conservatively require the current user's Codex writers to be closed.
/// The caller verifies the exact bundle and CLI pins independently. This
/// census neither establishes root attribution nor prevents a later launch.
pub fn desktop_writer_inventory(pinned_bundle: &Path, stable_cli: &Path) -> DesktopWriterInventory {
    #[cfg(target_os = "macos")]
    {
        use crate::macos_sandbox::desktop_processes;

        let Ok(bundle) = pinned_bundle.canonicalize() else {
            return DesktopWriterInventory::Unknown;
        };
        let Ok(cli) = stable_cli.canonicalize() else {
            return DesktopWriterInventory::Unknown;
        };
        if !bundle.is_dir() || !cli.is_file() {
            return DesktopWriterInventory::Unknown;
        }
        let first = desktop_processes();
        let second = desktop_processes();
        let mut inventory = DesktopWriterInventory::Closed;
        for census in [&first, &second].into_iter().flatten() {
            for process in &census.processes {
                match classify(
                    &process.executable,
                    &process.signing_identifier,
                    &bundle,
                    &cli,
                ) {
                    DesktopWriterInventory::Active => return DesktopWriterInventory::Active,
                    DesktopWriterInventory::Unknown => inventory = DesktopWriterInventory::Unknown,
                    DesktopWriterInventory::Closed => {}
                }
            }
        }
        if !first.as_ref().is_some_and(|census| census.complete) || first != second {
            return DesktopWriterInventory::Unknown;
        }
        inventory
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (pinned_bundle, stable_cli);
        DesktopWriterInventory::Unknown
    }
}

#[cfg(any(target_os = "macos", test))]
fn classify(
    executable: &Path,
    signing_id: &[u8],
    bundle: &Path,
    cli: &Path,
) -> DesktopWriterInventory {
    use DesktopWriterInventory::{Active, Closed, Unknown};

    let codex_signing_id = matches!(
        signing_id,
        b"com.openai.codex"
            | b"codex"
            | b"codex-aarch64-apple-darwin"
            | b"codex-x86_64-apple-darwin"
            | b"codex-app-server"
            | b"codex-code-mode-host"
            | b"codex-voice-host"
            | b"com.openai.sky.CUAService"
    ) || signing_id.starts_with(b"com.openai.codex.")
        || signing_id.starts_with(b"com.openai.sky.CUAService.");
    if executable == cli {
        return Active;
    }
    if let Ok(relative) = executable.strip_prefix(bundle.join("Contents")) {
        // CUA's bundled Node executables have generic code-signing identifiers.
        let cua_runtime = (relative == Path::new("Resources/cua_node/bin/node")
            && signing_id == b"node")
            || (relative == Path::new("Resources/cua_node/bin/node_repl")
                && signing_id == b"node_repl");
        return if codex_signing_id || cua_runtime {
            Active
        } else {
            Unknown
        };
    }

    // A foreign installation or unsigned/ad-hoc distribution may be a writer.
    // Path components are candidate hints only and never establish identity.
    const BUNDLE_CANDIDATES: &[&str] = &[
        "ChatGPT.app",
        "Codex.app",
        "CodexCLI.app",
        "Codex Computer Use.app",
        "cua_node",
    ];
    const EXECUTABLE_CANDIDATES: &[&str] = &[
        "codex",
        "codex-aarch64-apple-darwin",
        "codex-x86_64-apple-darwin",
        "codex-app-server",
        "codex-code-mode-host",
        "codex-voice-host",
        "SkyComputerUseService",
        "SkyComputerUseClient",
        "CUALockScreenGuardian",
    ];
    let candidate_bundle = executable.components().any(|component| {
        BUNDLE_CANDIDATES
            .iter()
            .any(|name| component.as_os_str().eq_ignore_ascii_case(name))
    });
    let candidate_executable = executable.file_name().is_some_and(|file_name| {
        EXECUTABLE_CANDIDATES
            .iter()
            .any(|name| file_name.eq_ignore_ascii_case(name))
    });
    if codex_signing_id || candidate_bundle || candidate_executable {
        Unknown
    } else {
        Closed
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn observed(path: &str, id: &[u8]) -> DesktopWriterInventory {
        classify(
            Path::new(path),
            id,
            Path::new("/Applications/ChatGPT.app"),
            Path::new("/pinned/codex-stable"),
        )
    }

    #[test]
    fn pinned_desktop_cli_and_helpers_are_active() {
        use DesktopWriterInventory::Active;
        assert_eq!(
            observed(
                "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
                b"com.openai.codex"
            ),
            Active
        );
        assert_eq!(
            observed("/pinned/codex-stable", b"codex-aarch64-apple-darwin"),
            Active
        );
        assert_eq!(
            observed(
                "/Applications/ChatGPT.app/Contents/Resources/codex-cli/CodexCLI.app/Contents/MacOS/codex",
                b"codex"
            ),
            Active
        );
        assert_eq!(
            observed(
                "/Applications/ChatGPT.app/Contents/Frameworks/Codex Framework.framework/Versions/156.0.8078.79/Helpers/Codex (Renderer).app/Contents/MacOS/Codex (Renderer)",
                b"com.openai.codex.helper.renderer"
            ),
            Active
        );
        assert_eq!(
            observed(
                "/Applications/ChatGPT.app/Contents/Resources/cua_node/bin/node",
                b"node"
            ),
            Active
        );
        assert_eq!(
            observed(
                "/Applications/ChatGPT.app/Contents/Resources/cua_node/bin/node_repl",
                b"node_repl"
            ),
            Active
        );
    }

    #[test]
    fn uncertain_identity_or_scope_blocks_without_name_only_confirmation() {
        use DesktopWriterInventory::Unknown;
        assert_eq!(observed("/usr/local/bin/codex", b"unrecognized"), Unknown);
        assert_eq!(observed("/other/renamed", b"com.openai.codex"), Unknown);
        assert_eq!(
            observed(
                "/Library/PrivilegedHelperTools/unqualified",
                b"com.openai.sky.CUAService"
            ),
            Unknown
        );
        assert_eq!(
            observed(
                "/Applications/ChatGPT.app/Contents/MacOS/ChatGPT",
                b"unrecognized"
            ),
            Unknown
        );
    }

    #[test]
    fn name_fragments_and_unrelated_generic_runtimes_do_not_match() {
        use DesktopWriterInventory::Closed;
        assert_eq!(observed("/usr/local/bin/node", b"node"), Closed);
        assert_eq!(
            observed(
                "/usr/local/bin/codex-debug-tool",
                b"com.example.codex-debug-tool"
            ),
            Closed
        );
        assert_eq!(
            observed(
                "/Applications/ChatGPT.app-copy/Contents/MacOS/foo",
                b"com.example.foo"
            ),
            Closed
        );
    }
}
