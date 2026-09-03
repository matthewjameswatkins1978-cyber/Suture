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

    #[error("I/O error: {0}")]
    Io(#[from] io::Error),
}

#[derive(Clone, Debug)]
pub struct Workspace {
    root: PathBuf,
}

impl Workspace {
    /// Creates a new Workspace rooted at the given path.
    /// The path is canonicalized to ensure absolute resolution and symlink resolution of the root itself.
    pub fn new<P: AsRef<Path>>(root: P) -> Result<Self, WorkspaceError> {
        let canonical_root = fs::canonicalize(root.as_ref()).map_err(|e| {
            if e.kind() == io::ErrorKind::NotFound {
                WorkspaceError::NotFound(root.as_ref().to_string_lossy().into_owned())
            } else {
                WorkspaceError::Io(e)
            }
        })?;

        if !canonical_root.is_dir() {
            return Err(WorkspaceError::Io(io::Error::new(
                io::ErrorKind::NotADirectory,
                "workspace root must be a directory",
            )));
        }

        Ok(Workspace {
            root: canonical_root,
        })
    }

    /// Returns the canonicalized root path of the workspace.
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// Resolves a relative path within the workspace root.
    /// Verifies that the resolved path stays strictly within the workspace root
    /// and rejects traversal escapes and symlink escapes.
    pub fn resolve_path<P: AsRef<Path>>(&self, rel_path: P) -> Result<PathBuf, WorkspaceError> {
        let rel = rel_path.as_ref();

        // Reject absolute paths passed as relative parts or explicit root prefixes
        if rel.is_absolute() {
            return Err(WorkspaceError::Traversal(format!(
                "absolute path not allowed: {}",
                rel.display()
            )));
        }

        // Check for component-level traversal or suspicious segments
        let mut clean_components = Vec::new();
        for component in rel.components() {
            match component {
                Component::ParentDir => {
                    if clean_components.pop().is_none() {
                        return Err(WorkspaceError::Traversal(format!(
                            "path traversal escapes workspace root: {}",
                            rel.display()
                        )));
                    }
                }
                Component::Normal(c) => {
                    clean_components.push(c);
                }
                Component::CurDir => {}
                Component::RootDir | Component::Prefix(_) => {
                    return Err(WorkspaceError::Traversal(format!(
                        "invalid path component in relative path: {}",
                        rel.display()
                    )));
                }
            }
        }

        // Build tentative candidate path joined to root
        let mut candidate = self.root.clone();
        for c in &clean_components {
            candidate.push(c);
        }

        // Progressive ancestor symlink check
        let mut check_path = self.root.clone();
        for c in &clean_components {
            check_path.push(c);
            if check_path.exists() {
                let canonical_ancestor = fs::canonicalize(&check_path).map_err(|e| {
                    if e.kind() == io::ErrorKind::NotFound {
                        WorkspaceError::NotFound(check_path.to_string_lossy().into_owned())
                    } else {
                        WorkspaceError::Io(e)
                    }
                })?;
                if !canonical_ancestor.starts_with(&self.root) {
                    return Err(WorkspaceError::SymlinkEscape(format!(
                        "symlink escape detected: {} resolves to {}",
                        check_path.display(),
                        canonical_ancestor.display()
                    )));
                }
            }
        }

        // If the full path already exists, ensure its canonical form starts with root
        if candidate.exists() {
            let canonical_full = fs::canonicalize(&candidate).map_err(|e| {
                if e.kind() == io::ErrorKind::NotFound {
                    WorkspaceError::NotFound(candidate.to_string_lossy().into_owned())
                } else {
                    WorkspaceError::Io(e)
                }
            })?;
            if !canonical_full.starts_with(&self.root) {
                return Err(WorkspaceError::SymlinkEscape(format!(
                    "symlink escape detected: {} resolves to {}",
                    candidate.display(),
                    canonical_full.display()
                )));
            }
            Ok(canonical_full)
        } else {
            if !candidate.starts_with(&self.root) {
                return Err(WorkspaceError::Traversal(format!(
                    "path escapes workspace root: {}",
                    candidate.display()
                )));
            }
            Ok(candidate)
        }
    }

    /// Reads a file relative to the workspace root.
    pub fn read_file<P: AsRef<Path>>(&self, rel_path: P) -> Result<Vec<u8>, WorkspaceError> {
        let resolved = self.resolve_path(rel_path)?;
        if !resolved.is_file() {
            return Err(WorkspaceError::NotFound(
                resolved.to_string_lossy().into_owned(),
            ));
        }
        let bytes = fs::read(&resolved)?;
        Ok(bytes)
    }

    /// Writes a file atomically relative to the workspace root.
    /// Performs safe staging (writing to a sibling temp file in the same directory)
    /// and atomic rename (`fs::rename`).
    pub fn write_file_atomic<P: AsRef<Path>>(
        &self,
        rel_path: P,
        bytes: &[u8],
    ) -> Result<(), WorkspaceError> {
        let resolved = self.resolve_path(rel_path)?;

        // Ensure parent directory exists
        if let Some(parent) = resolved.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)?;
            } else {
                let canonical_parent = fs::canonicalize(parent)?;
                if !canonical_parent.starts_with(&self.root) {
                    return Err(WorkspaceError::SymlinkEscape(format!(
                        "parent directory escapes workspace root via symlink: {}",
                        parent.display()
                    )));
                }
            }
        }

        let parent_dir = resolved.parent().unwrap_or(&self.root);

        // Generate a unique temporary file name in the same directory
        let file_name = resolved
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("file");

        // Use a simple pseudo-random / timestamp suffix or counter for uniqueness
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .subsec_nanos();
        let tmp_file_name = format!(".suture_tmp_{}_{}.{}", file_name, std::process::id(), nanos);
        let tmp_path = parent_dir.join(tmp_file_name);

        // Write bytes to temp file with explicit flush and sync
        let mut file = fs::File::create(&tmp_path)?;
        file.write_all(bytes)?;
        file.flush()?;
        file.sync_all()?;
        drop(file);

        // Perform atomic rename
        if let Err(e) = fs::rename(&tmp_path, &resolved) {
            let _ = fs::remove_file(&tmp_path);
            return Err(WorkspaceError::Io(e));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_workspace_new_valid() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path());
        assert!(ws.is_ok());
        let ws = ws.unwrap();
        assert!(ws.root().is_absolute());
    }

    #[test]
    fn test_workspace_new_not_found() {
        let res = Workspace::new("/nonexistent_path_abc_123");
        assert!(matches!(res, Err(WorkspaceError::NotFound(_))));
    }

    #[test]
    fn test_resolve_path_valid() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();
        let resolved = ws.resolve_path("foo/bar.txt").unwrap();
        assert!(resolved.starts_with(ws.root()));
        assert!(resolved.ends_with("foo/bar.txt"));
    }

    #[test]
    fn test_resolve_path_traversal_rejection() {
        let tmp = TempDir::new().unwrap();
        let _ws = Workspace::new(tmp.path()).unwrap();

        let res1 = _ws.resolve_path("../outside.txt");
        assert!(matches!(res1, Err(WorkspaceError::Traversal(_))));

        let res2 = _ws.resolve_path("foo/../../outside.txt");
        assert!(matches!(res2, Err(WorkspaceError::Traversal(_))));

        let res3 = _ws.resolve_path("/etc/passwd");
        assert!(matches!(res3, Err(WorkspaceError::Traversal(_))));
    }

    #[test]
    fn test_symlink_escape_rejection() {
        let tmp = TempDir::new().unwrap();
        let _ws = Workspace::new(tmp.path()).unwrap();

        let outside_tmp = TempDir::new().unwrap();
        let outside_file = outside_tmp.path().join("secret.txt");
        fs::write(&outside_file, "secret").unwrap();

        let _symlink_path = tmp.path().join("evil_link");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink(outside_tmp.path(), &_symlink_path).unwrap();

            let res = _ws.resolve_path("evil_link/secret.txt");
            assert!(matches!(res, Err(WorkspaceError::SymlinkEscape(_))));
        }
    }

    #[test]
    fn test_read_write_roundtrip_and_atomic() {
        let tmp = TempDir::new().unwrap();
        let ws = Workspace::new(tmp.path()).unwrap();

        let rel = "config/settings.json";
        let content = b"{\"hello\": \"world\"}";

        ws.write_file_atomic(rel, content).unwrap();

        let read_bytes = ws.read_file(rel).unwrap();
        assert_eq!(read_bytes, content);

        let new_content = b"{\"hello\": \"suture\"}";
        ws.write_file_atomic(rel, new_content).unwrap();
        let read_bytes2 = ws.read_file(rel).unwrap();
        assert_eq!(read_bytes2, new_content);
    }
}
