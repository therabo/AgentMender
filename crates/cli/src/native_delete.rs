use std::path::Path;
use std::time::Duration;

pub(crate) fn dispatch(executable: &Path, root: &Path, id: uuid::Uuid, confined: bool) -> bool {
    use agentmender_provider_api::process::{run_codex_delete, run_confined_codex_delete};

    let command = if confined {
        run_confined_codex_delete
    } else {
        run_codex_delete
    };
    let succeeded = command(executable, root, id, Duration::from_secs(180));
    #[cfg(target_os = "macos")]
    if succeeded {
        agentmender_provider_codex::schedule_desktop_catalog_reconciliation(root, id);
    }
    succeeded
}
