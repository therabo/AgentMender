//! Independent operator registration. No registration writer or tool argument.
//! Current host authentication is not inferred from environment/clientInfo.
use agentmender_domain::ManagementIdentity;
#[cfg(unix)]
use agentmender_domain::{ArtifactId, EntityId, GraphNodeKind, ProviderKind};
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

/// Only an independently controlled root-owned chain can register this exact
/// running server instance. A model-writable file or a previous process refuses.
pub(super) fn read(path: Option<&Path>, instance: &str, root: &Path) -> Option<ManagementIdentity> {
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
        // The fresh file bytes and physical root are part of continuation/context
        // freshness. This digest is not a signature or destructive authority.
        let revision = agentmender_domain::search_scope(
            &agentmender_domain::SearchParameters {
                query: String::new(),
                include_archived: true,
                include_content: false,
                project_label: None,
                purpose: agentmender_domain::SearchPurpose::Management,
                page_size: 1,
                excerpts: false,
            },
            root,
            &[],
            &String::from_utf8(bytes).ok()?,
        );
        Some(ManagementIdentity {
            identity: ArtifactId {
                provider: r.provider,
                state_root: r.state_root,
                class: GraphNodeKind::Session,
                local_id: EntityId::new(format!("codex:session:{}", r.native_session_id)).ok()?,
            },
            native_session_id: r.native_session_id,
            revision,
        })
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
