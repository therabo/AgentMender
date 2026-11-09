//! Validate independent operator declarations without admitting caller identity.
//! A privileged file does not prove actual MCP conversation routing. The installed
//! client supplies no enforceable exclusive per-conversation binding, so guaranteed
//! management requests remain fail-closed. No registration writer or tool argument.
use agentmender_domain::ManagementIdentity;
#[cfg(unix)]
use agentmender_domain::ProviderKind;
#[cfg(unix)]
use serde::Deserialize;
use std::path::Path;

#[cfg(unix)]
#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Registration {
    schema: u32,
    server_instance: String,
    provider: ProviderKind,
    state_root: std::path::PathBuf,
    root_device: u64,
    root_inode: u64,
    native_session_id: String,
    expires_at: u64,
    connection_scope: String,
}

/// File validation is retained for operator diagnostics, never host admission.
pub(super) fn read(path: Option<&Path>, instance: &str, root: &Path) -> Option<ManagementIdentity> {
    admit_file_declaration(validate_declaration(path, instance, root))
}

fn admit_file_declaration(_validated: Option<()>) -> Option<ManagementIdentity> {
    // No attested host transport/routing capability exists in this deployment.
    // Even a valid root-owned declaration cannot prove an exclusive connection.
    None
}

fn validate_declaration(path: Option<&Path>, instance: &str, root: &Path) -> Option<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let path = path?;
        if !path.is_absolute()
            || path.components().any(|c| {
                !matches!(
                    c,
                    std::path::Component::RootDir | std::path::Component::Normal(_)
                )
            })
        {
            return None;
        }
        let mut at = std::path::PathBuf::from("/");
        for component in path.components().skip(1) {
            at.push(component.as_os_str());
            let metadata = std::fs::symlink_metadata(&at).ok()?;
            if metadata.file_type().is_symlink()
                || metadata.uid() != 0
                || metadata.permissions().mode() & 0o022 != 0
            {
                return None;
            }
            agentmender_provider_api::filesystem::verify_operator_descriptor(
                &std::fs::File::open(&at).ok()?,
            )
            .ok()?;
        }
        let bytes =
            agentmender_provider_api::filesystem::read_bounded(Path::new("/"), path, 8192).ok()?;
        let r: Registration = serde_json::from_slice(&bytes).ok()?;
        let metadata = std::fs::symlink_metadata(root).ok()?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()?
            .as_secs();
        if r.schema != 1
            || r.server_instance != instance
            || r.provider != ProviderKind::Codex
            || r.state_root != root
            || !metadata.is_dir()
            || metadata.file_type().is_symlink()
            || r.root_device != metadata.dev()
            || r.root_inode != metadata.ino()
            || r.expires_at <= now
            || r.expires_at > now.checked_add(3600)?
            || r.connection_scope != "single_conversation_operator_verified"
            || super::conversation::canonical_uuid(&r.native_session_id).is_err()
        {
            return None;
        }
        // The declaration is well formed and independently owned. This proves
        // neither the actual caller nor single_conversation_operator_verified
        // routing. Never construct ManagementIdentity from these bytes.
        Some(())
    }
    #[cfg(not(unix))]
    {
        let _ = (path, instance, root);
        None
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn even_validated_independent_file_cannot_attest_actual_connection_routing() {
        assert!(super::admit_file_declaration(Some(())).is_none());
        assert!(super::admit_file_declaration(None).is_none());
    }
    #[test]
    fn absent_model_owned_and_arbitrary_registration_never_bind() {
        use std::path::Path;
        assert!(super::read(None, "instance", Path::new("/provider")).is_none());
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("registration.json");
        std::fs::write(
            &file,
            b"{\"native_session_id\":\"12345678-1234-1234-1234-123456789abc\"}",
        )
        .unwrap();
        assert!(super::read(Some(&file), "instance", temp.path()).is_none());
    }
}
