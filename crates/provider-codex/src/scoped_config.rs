//! Conservative current-configuration guard for the declared-root profile.
//! Absence is checked; configuration contents are never read or interpreted.
use agentmender_persistence::EffectError;
use std::path::{Component, Path};

fn rejected() -> EffectError {
    EffectError::InvalidEvidence
}

fn absent(path: &Path) -> Result<(), EffectError> {
    match std::fs::symlink_metadata(path) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        _ => Err(rejected()),
    }
}

fn no_link_directory(path: &Path) -> Result<(), EffectError> {
    if !path.is_absolute() {
        return Err(rejected());
    }
    if path.canonicalize().map_err(|_| rejected())?.as_os_str() != path.as_os_str() {
        return Err(rejected());
    }
    let mut prefix = std::path::PathBuf::from("/");
    for component in path.components() {
        match component {
            Component::RootDir => {}
            Component::Normal(name) => {
                prefix.push(name);
                let metadata = std::fs::symlink_metadata(&prefix).map_err(|_| rejected())?;
                if !metadata.is_dir() || metadata.file_type().is_symlink() {
                    return Err(rejected());
                }
            }
            _ => return Err(rejected()),
        }
    }
    Ok(())
}

pub(super) fn validate(root: &Path, cwd: &Path) -> Result<(), EffectError> {
    if std::env::var_os("CODEX_SQLITE_HOME").is_some() {
        return Err(rejected());
    }
    if std::env::var_os("CODEX_HOME").is_some_and(|value| Path::new(&value) != root) {
        return Err(rejected());
    }
    no_link_directory(root)?;
    no_link_directory(cwd)?;
    for entry in std::fs::read_dir(root).map_err(|_| rejected())? {
        let entry = entry.map_err(|_| rejected())?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return Err(rejected());
        };
        if name == "config.toml" || name.ends_with(".config.toml") || name == "config.d" {
            return Err(rejected());
        }
    }
    match std::fs::symlink_metadata("/etc/codex") {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
        Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {}
        _ => return Err(rejected()),
    }
    for path in [
        "/etc/codex/config.toml",
        "/etc/codex/managed_config.toml",
        "/etc/codex/requirements.toml",
    ] {
        absent(Path::new(path))?;
    }
    for ancestor in cwd.ancestors() {
        let codex = ancestor.join(".codex");
        match std::fs::symlink_metadata(&codex) {
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Ok(metadata) if metadata.is_dir() && !metadata.file_type().is_symlink() => {
                absent(&codex.join("config.toml"))?
            }
            _ => return Err(rejected()),
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn absent_configuration_is_required_and_symlinks_refuse() {
        let t = tempfile::tempdir().unwrap();
        let base = t.path().canonicalize().unwrap();
        let root = base.join("root");
        let cwd = base.join("project");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        no_link_directory(&cwd).unwrap();
        std::fs::write(root.join("config.toml"), "sqlite_home = '/elsewhere'").unwrap();
        assert!(absent(&root.join("config.toml")).is_err());
        std::fs::remove_file(root.join("config.toml")).unwrap();
        assert!(absent(&root.join("config.toml")).is_ok());
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(&cwd, base.join("alias")).unwrap();
            assert!(no_link_directory(&base.join("alias")).is_err());
        }
    }
}
