#![forbid(unsafe_code)]

use atomic_write_file::AtomicWriteFile;
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WorkspaceError {
    #[error("path traversal attempt detected: {0}")]
    Traversal(String),
    #[error("symlink escape attempt detected: {0}")]
    SymlinkEscape(String),
    #[error("path not found: {0}")]
    NotFound(String),
    #[error("stale file identity: expected {expected}, observed {actual}")]
    StaleIdentity { expected: String, actual: String },
    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
    #[error("destination already exists: {0}")]
    AlreadyExists(String),
}

#[derive(Clone, Debug)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, WorkspaceError> {
        let root = fs::canonicalize(root.as_ref()).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                WorkspaceError::NotFound(root.as_ref().display().to_string())
            } else {
                WorkspaceError::Io(e)
            }
        })?;
        if !root.is_dir() {
            return Err(WorkspaceError::Io(io::Error::new(
                io::ErrorKind::NotADirectory,
                "workspace root must be a directory",
            )));
        }
        Ok(Self { root })
    }
    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn resolve_path<P: AsRef<Path>>(&self, rel_path: P) -> Result<PathBuf, WorkspaceError> {
        let rel = rel_path.as_ref();
        if rel.is_absolute() || rel.to_string_lossy().as_bytes().get(1) == Some(&b':') {
            return Err(WorkspaceError::Traversal(format!(
                "absolute path not allowed: {}",
                rel.display()
            )));
        }
        let mut components = Vec::new();
        for c in rel.components() {
            match c {
                Component::ParentDir => {
                    if components.pop().is_none() {
                        return Err(WorkspaceError::Traversal(rel.display().to_string()));
                    }
                }
                Component::Normal(c) => components.push(c.to_owned()),
                Component::CurDir => {}
                Component::RootDir | Component::Prefix(_) => {
                    return Err(WorkspaceError::Traversal(rel.display().to_string()))
                }
            }
        }
        let mut candidate = self.root.clone();
        for c in &components {
            candidate.push(c);
        }
        let mut ancestor = self.root.clone();
        for c in &components {
            ancestor.push(c);
            if fs::symlink_metadata(&ancestor).is_ok() {
                let resolved = fs::canonicalize(&ancestor).map_err(|e| {
                    if e.kind() == io::ErrorKind::NotFound {
                        WorkspaceError::NotFound(ancestor.display().to_string())
                    } else {
                        WorkspaceError::Io(e)
                    }
                })?;
                if !resolved.starts_with(&self.root) {
                    return Err(WorkspaceError::SymlinkEscape(format!(
                        "{} resolves outside workspace",
                        ancestor.display()
                    )));
                }
            }
        }
        if fs::symlink_metadata(&candidate).is_ok() {
            let resolved = fs::canonicalize(&candidate).map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    WorkspaceError::NotFound(candidate.display().to_string())
                } else {
                    WorkspaceError::Io(e)
                }
            })?;
            if !resolved.starts_with(&self.root) {
                return Err(WorkspaceError::SymlinkEscape(
                    candidate.display().to_string(),
                ));
            }
            Ok(resolved)
        } else {
            Ok(candidate)
        }
    }

    pub fn read_file<P: AsRef<Path>>(&self, rel_path: P) -> Result<Vec<u8>, WorkspaceError> {
        let resolved = self.resolve_path(rel_path)?;
        if !resolved.is_file() {
            return Err(WorkspaceError::NotFound(resolved.display().to_string()));
        }
        Ok(fs::read(resolved)?)
    }

    pub fn create_file_new<P: AsRef<Path>>(
        &self,
        rel_path: P,
        bytes: &[u8],
    ) -> Result<(), WorkspaceError> {
        let resolved = self.resolve_path(rel_path)?;
        if resolved.exists() {
            return Err(WorkspaceError::AlreadyExists(
                resolved.display().to_string(),
            ));
        }
        if let Some(parent) = resolved.parent() {
            fs::create_dir_all(parent)?;
            self.ensure_parent(parent)?;
        }
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&resolved)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        Ok(())
    }

    pub fn delete_file_checked<P: AsRef<Path>>(
        &self,
        rel_path: P,
        expected_hash: &str,
    ) -> Result<(), WorkspaceError> {
        let resolved = self.resolve_path(rel_path)?;
        let current = fs::read(&resolved)?;
        let actual = sha256(&current);
        if actual != expected_hash {
            return Err(WorkspaceError::StaleIdentity {
                expected: expected_hash.into(),
                actual,
            });
        }
        fs::remove_file(resolved)?;
        Ok(())
    }

    pub fn rename_file_checked<P: AsRef<Path>, Q: AsRef<Path>>(
        &self,
        source: P,
        destination: Q,
        expected_hash: &str,
        destination_absent: bool,
    ) -> Result<(), WorkspaceError> {
        let source = self.resolve_path(source)?;
        let destination = self.resolve_path(destination)?;
        let current = fs::read(&source)?;
        let actual = sha256(&current);
        if actual != expected_hash {
            return Err(WorkspaceError::StaleIdentity {
                expected: expected_hash.into(),
                actual,
            });
        }
        if destination_absent && destination.exists() {
            return Err(WorkspaceError::AlreadyExists(
                destination.display().to_string(),
            ));
        }
        if let Some(parent) = destination.parent() {
            fs::create_dir_all(parent)?;
            self.ensure_parent(parent)?;
        }
        fs::rename(source, destination)?;
        Ok(())
    }

    pub fn write_file_atomic<P: AsRef<Path>>(
        &self,
        rel_path: P,
        bytes: &[u8],
    ) -> Result<(), WorkspaceError> {
        let resolved = self.resolve_path(rel_path)?;
        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            }
            self.ensure_parent(parent)?;
        }
        let mut file = AtomicWriteFile::open(&resolved)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.commit()?;
        Ok(())
    }

    /// Stage and replace only if the object still has the observed content hash.
    pub fn write_file_atomic_checked<P: AsRef<Path>>(
        &self,
        rel_path: P,
        expected_hash: &str,
        bytes: &[u8],
    ) -> Result<(), WorkspaceError> {
        let resolved = self.resolve_path(rel_path)?;
        let observed = fs::read(&resolved)?;
        let actual = sha256(&observed);
        if actual != expected_hash {
            return Err(WorkspaceError::StaleIdentity {
                expected: expected_hash.into(),
                actual,
            });
        }
        if let Some(parent) = resolved.parent() {
            self.ensure_parent(parent)?;
        }
        let mut file = AtomicWriteFile::open(&resolved)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.commit()?;
        Ok(())
    }

    fn ensure_parent(&self, parent: &Path) -> Result<(), WorkspaceError> {
        let canonical = fs::canonicalize(parent)?;
        if !canonical.starts_with(&self.root) {
            return Err(WorkspaceError::SymlinkEscape(parent.display().to_string()));
        }
        Ok(())
    }
}

fn sha256(bytes: &[u8]) -> String {
    let mut h = Sha256::new();
    h.update(bytes);
    format!("{:x}", h.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    #[test]
    fn workspace_new_valid() {
        let t = TempDir::new().unwrap();
        assert!(Workspace::new(t.path()).is_ok());
    }
    #[test]
    fn traversal_rejected() {
        let t = TempDir::new().unwrap();
        let w = Workspace::new(t.path()).unwrap();
        assert!(matches!(
            w.resolve_path("../x"),
            Err(WorkspaceError::Traversal(_))
        ));
        assert!(matches!(
            w.resolve_path("/etc/passwd"),
            Err(WorkspaceError::Traversal(_))
        ));
    }
    #[cfg(unix)]
    #[test]
    fn symlink_escape_rejected() {
        use std::os::unix::fs::symlink;
        let t = TempDir::new().unwrap();
        let o = TempDir::new().unwrap();
        fs::write(o.path().join("x"), b"x").unwrap();
        symlink(o.path(), t.path().join("link")).unwrap();
        let w = Workspace::new(t.path()).unwrap();
        assert!(matches!(
            w.resolve_path("link/x"),
            Err(WorkspaceError::SymlinkEscape(_))
        ));
    }
    #[test]
    fn existing_file_replaced_and_stale_refused() {
        let t = TempDir::new().unwrap();
        let w = Workspace::new(t.path()).unwrap();
        w.write_file_atomic("x.txt", b"one").unwrap();
        let h = sha256(b"one");
        w.write_file_atomic_checked("x.txt", &h, b"two").unwrap();
        assert_eq!(w.read_file("x.txt").unwrap(), b"two");
        assert!(matches!(
            w.write_file_atomic_checked("x.txt", &h, b"three"),
            Err(WorkspaceError::StaleIdentity { .. })
        ));
    }
}
