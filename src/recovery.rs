#![forbid(unsafe_code)]

use crate::engine::compute_sha256;
use crate::workspace::{Workspace, WorkspaceError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Journal {
    pub protocol_version: String,
    pub transaction_id: String,
    pub entries: Vec<JournalEntry>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct JournalEntry {
    pub path: String,
    pub pre_hash: String,
    pub candidate_hash: String,
    pub original: Vec<u8>,
    pub candidate: Vec<u8>,
}

pub fn write_journal(workspace: &Workspace, journal: &Journal) -> Result<(), WorkspaceError> {
    let dir = workspace.root().join(".suture-recovery");
    fs::create_dir_all(&dir)?;
    let path = dir.join(format!("{}.json", safe_id(&journal.transaction_id)));
    let bytes = serde_json::to_vec_pretty(journal)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)?;
    std::io::Write::write_all(&mut file, &bytes)?;
    file.sync_all()?;
    Ok(())
}

pub fn remove_journal(workspace: &Workspace, transaction_id: &str) -> Result<(), WorkspaceError> {
    let path = workspace
        .root()
        .join(".suture-recovery")
        .join(format!("{}.json", safe_id(transaction_id)));
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e.into()),
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryState {
    Original,
    Candidate,
    Manual,
}

pub fn recover_all(workspace: &Workspace) -> RecoveryReport {
    let dir = workspace.root().join(".suture-recovery");
    let mut report = RecoveryReport {
        inspected: 0,
        completed: 0,
        restored: 0,
        cleaned: 0,
        manual: Vec::new(),
    };
    let entries = match fs::read_dir(&dir) {
        Ok(x) => x,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return report,
        Err(_) => return report,
    };
    for item in entries.flatten() {
        if item.path().extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        report.inspected += 1;
        let journal: Journal = match fs::read(item.path())
            .ok()
            .and_then(|b| serde_json::from_slice(&b).ok())
        {
            Some(x) => x,
            None => {
                report
                    .manual
                    .push(item.file_name().to_string_lossy().into());
                continue;
            }
        };
        let mut states = Vec::with_capacity(journal.entries.len());
        for entry in &journal.entries {
            let current = match workspace.read_file(&entry.path) {
                Ok(x) => x,
                Err(_) => {
                    states.push(EntryState::Manual);
                    report.manual.push(format!("{}: unreadable", entry.path));
                    continue;
                }
            };
            let current_hash = compute_sha256(&current);
            if current_hash == entry.candidate_hash {
                states.push(EntryState::Candidate);
            } else if current_hash != entry.pre_hash {
                states.push(EntryState::Manual);
                report
                    .manual
                    .push(format!("{}: changed after interruption", entry.path));
            } else {
                states.push(EntryState::Original);
            }
        }
        if states.contains(&EntryState::Manual) {
            continue;
        }
        if states.iter().all(|state| *state == EntryState::Candidate) {
            if fs::remove_file(item.path()).is_ok() {
                report.completed += 1;
                report.cleaned += 1;
            }
            continue;
        }
        let mut safe = true;
        for (entry, state) in journal.entries.iter().zip(states) {
            if state != EntryState::Candidate {
                continue;
            }
            match workspace.write_file_atomic_checked(
                &entry.path,
                &entry.candidate_hash,
                &entry.original,
            ) {
                Ok(()) => report.restored += 1,
                Err(_) => {
                    safe = false;
                    report.manual.push(entry.path.clone());
                }
            }
        }
        if safe && fs::remove_file(item.path()).is_ok() {
            report.cleaned += 1;
        }
    }
    report
}

#[derive(Serialize, Clone, Debug)]
pub struct RecoveryReport {
    pub inspected: usize,
    pub completed: usize,
    pub restored: usize,
    pub cleaned: usize,
    pub manual: Vec<String>,
}

fn safe_id(value: &str) -> String {
    value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn journal_for(workspace: &Workspace, transaction_id: &str, entries: Vec<JournalEntry>) {
        write_journal(
            workspace,
            &Journal {
                protocol_version: "1.1.0".into(),
                transaction_id: transaction_id.into(),
                entries,
            },
        )
        .unwrap();
    }

    fn entry(path: &str, original: &[u8], candidate: &[u8]) -> JournalEntry {
        JournalEntry {
            path: path.into(),
            pre_hash: compute_sha256(original),
            candidate_hash: compute_sha256(candidate),
            original: original.to_vec(),
            candidate: candidate.to_vec(),
        }
    }

    #[test]
    fn recovery_preserves_a_fully_completed_commit() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::new(temp.path()).unwrap();
        workspace.write_file_atomic("x.txt", b"new").unwrap();
        journal_for(
            &workspace,
            "completed",
            vec![entry("x.txt", b"old", b"new")],
        );

        let report = recover_all(&workspace);

        assert_eq!(report.completed, 1);
        assert_eq!(report.restored, 0);
        assert_eq!(workspace.read_file("x.txt").unwrap(), b"new");
        assert!(!temp.path().join(".suture-recovery/completed.json").exists());
    }

    #[test]
    fn recovery_rolls_back_only_a_partial_commit() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::new(temp.path()).unwrap();
        workspace.write_file_atomic("a.txt", b"new-a").unwrap();
        workspace.write_file_atomic("b.txt", b"old-b").unwrap();
        journal_for(
            &workspace,
            "partial",
            vec![
                entry("a.txt", b"old-a", b"new-a"),
                entry("b.txt", b"old-b", b"new-b"),
            ],
        );

        let report = recover_all(&workspace);

        assert_eq!(report.completed, 0);
        assert_eq!(report.restored, 1);
        assert_eq!(workspace.read_file("a.txt").unwrap(), b"old-a");
        assert_eq!(workspace.read_file("b.txt").unwrap(), b"old-b");
        assert!(!temp.path().join(".suture-recovery/partial.json").exists());
    }

    #[test]
    fn recovery_refuses_to_touch_an_entry_changed_after_interruption() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::new(temp.path()).unwrap();
        workspace.write_file_atomic("x.txt", b"unexpected").unwrap();
        journal_for(&workspace, "changed", vec![entry("x.txt", b"old", b"new")]);

        let report = recover_all(&workspace);

        assert_eq!(report.restored, 0);
        assert_eq!(report.completed, 0);
        assert!(!report.manual.is_empty());
        assert_eq!(workspace.read_file("x.txt").unwrap(), b"unexpected");
        assert!(temp.path().join(".suture-recovery/changed.json").exists());
    }
}
