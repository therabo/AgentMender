//! The sole audited C-ABI boundary. See ADR-0004 and WebKit's SandboxSPI.h.
#![allow(unsafe_code)]

use crate::ProviderError;
use std::{ffi::CString, os::unix::ffi::OsStrExt, path::Path};

#[link(name = "sandbox")]
unsafe extern "C" {
    fn sandbox_check(
        pid: libc::pid_t,
        operation: *const libc::c_char,
        filter: libc::c_int,
        ...
    ) -> libc::c_int;
    static SANDBOX_CHECK_NO_REPORT: libc::c_int;
}

fn explicitly_denied(path: &Path, operation: &std::ffi::CStr) -> Result<bool, ProviderError> {
    let path = CString::new(path.as_os_str().as_bytes())
        .map_err(|_| ProviderError::InvalidScope("sandbox query path contains NUL".into()))?;
    let pid = libc::pid_t::try_from(std::process::id()).map_err(|_| {
        ProviderError::CapabilityUnavailable("sandbox process identity unavailable".into())
    })?;
    // SAFETY: This matches the maintained Darwin C ABI. Both NUL-terminated
    // strings remain alive for the call; the sole variadic argument is a path
    // pointer selected by SANDBOX_FILTER_PATH (1). The flag is an immutable
    // exported C constant. No buffers are written or retained by sandbox_check.
    let result = unsafe {
        sandbox_check(
            pid,
            operation.as_ptr(),
            1 | SANDBOX_CHECK_NO_REPORT,
            path.as_ptr(),
        )
    };
    match result {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProviderError::CapabilityUnavailable(
            "kernel sandbox policy query failed".into(),
        )),
    }
}

/// Require real MAC denial before SQLite can touch source shared-memory files.
pub(crate) fn verify_source_write_denial(root: &Path) -> Result<(), ProviderError> {
    let main = root.join("state_5.sqlite");
    if !main.is_file() {
        return Err(ProviderError::CapabilityUnavailable(
            "source database is missing".into(),
        ));
    }
    let mut paths = vec![root.to_path_buf(), main];
    for name in [
        "state_5.sqlite-wal",
        "state_5.sqlite-shm",
        "state_5.sqlite-journal",
    ] {
        let path = root.join(name);
        // sandbox_check can report denial for a nonexistent path even without
        // a sandbox. Existing objects and the directory establish actual policy.
        if path.exists() {
            paths.push(path);
        }
    }
    for path in paths {
        for operation in [
            c"file-write-data",
            c"file-write-create",
            c"file-write-unlink",
        ] {
            if !explicitly_denied(&path, operation)? {
                return Err(ProviderError::CapabilityUnavailable(
                    "source writes are not denied by the current kernel sandbox".into(),
                ));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn filesystem_permissions_do_not_count_as_kernel_sandbox_denial() {
        use std::os::unix::fs::PermissionsExt;
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("state_5.sqlite");
        std::fs::write(&path, b"fixture").unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400)).unwrap();
        assert!(!explicitly_denied(&path, c"file-write-data").unwrap());
        assert!(verify_source_write_denial(directory.path()).is_err());
    }
}
