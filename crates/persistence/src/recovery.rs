//! Typed content-addressed reconciliation metadata; no provider bodies are stored.
use crate::OwnedStateRoot;
use agentmender_domain::{RecoveryDigest, RecoveryManifest};
use std::io::{self, Read, Write};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecoveryPoint {
    digest: RecoveryDigest,
}
impl RecoveryPoint {
    pub fn digest(&self) -> &RecoveryDigest {
        &self.digest
    }
}
#[derive(Clone, Debug)]
pub struct RecoveryStore {
    root: OwnedStateRoot,
}
impl RecoveryStore {
    pub fn new(root: OwnedStateRoot) -> Self {
        Self { root }
    }
    pub fn prepare(&self, manifest: &RecoveryManifest) -> io::Result<RecoveryPoint> {
        if manifest.schema_version() != 1 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "unsupported recovery schema",
            ));
        }
        let bytes = serde_json::to_vec(manifest).map_err(io::Error::other)?;
        let point = RecoveryPoint {
            digest: manifest.digest(),
        };
        let name = format!("recovery-{}.json", point.digest);
        let mut stage = self.root.create_recovery_stage()?;
        stage.file.write_all(&bytes)?;
        stage.file.sync_all()?;
        match self.root.publish_recovery(stage, &point.digest) {
            Ok(()) => {}
            Err(e) if e.kind() == io::ErrorKind::AlreadyExists => {
                // A concurrent publisher may have made bytes visible before
                // syncing the directory. Establish that boundary ourselves.
                let existing = self.root.open_child_existing(&name, true)?;
                existing.sync_all()?;
                self.root.sync_dir()?;
            }
            Err(e) => return Err(e),
        }
        self.verify(&point, manifest)?;
        Ok(point)
    }
    pub fn verify_manifest(&self, manifest: &RecoveryManifest) -> io::Result<()> {
        self.verify(
            &RecoveryPoint {
                digest: manifest.digest(),
            },
            manifest,
        )
    }
    pub fn verify(&self, point: &RecoveryPoint, expected: &RecoveryManifest) -> io::Result<()> {
        if expected.schema_version() != 1 || point.digest != expected.digest() {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "recovery binding mismatch",
            ));
        }
        let name = format!("recovery-{}.json", point.digest);
        let f = self.root.open_child(&name, false, false)?;
        let mut bytes = Vec::new();
        f.take(65537).read_to_end(&mut bytes)?;
        if bytes.len() > 65536 {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "oversized recovery metadata",
            ));
        }
        let actual: RecoveryManifest = serde_json::from_slice(&bytes)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        if &actual != expected
            || actual.digest() != point.digest
            || serde_json::to_vec(&actual).map_err(io::Error::other)? != bytes
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidData,
                "recovery integrity mismatch",
            ));
        }
        Ok(())
    }
}
