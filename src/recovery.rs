#![forbid(unsafe_code)]

use crate::engine::compute_sha256;
use crate::workspace::{Workspace, WorkspaceError};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;

const RECOVERY_DIR: &str = ".threadmoth-recovery";
const LEGACY_RECOVERY_DIR: &str = ".suture-recovery";
const MAX_JOURNAL_BYTES: usize = 8 * 1024 * 1024;
const MAX_JOURNAL_ENTRIES: usize = 256;

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
    let dir = workspace.root().join(RECOVERY_DIR);
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
    let filename = format!("{}.json", safe_id(transaction_id));
    for dir in recovery_dirs(workspace) {
        match fs::remove_file(dir.join(&filename)) {
            Ok(()) => break,
            Err(e) if e.kind() == io::ErrorKind::NotFound => {}
            Err(e) => return Err(e.into()),
        }
    }
    for dir in recovery_dirs(workspace) {
        remove_recovery_dir_if_empty(&dir);
    }
    Ok(())
}

fn recovery_dirs(workspace: &Workspace) -> [std::path::PathBuf; 2] {
    [
        workspace.root().join(RECOVERY_DIR),
        workspace.root().join(LEGACY_RECOVERY_DIR),
    ]
}

fn remove_recovery_dir_if_empty(dir: &std::path::Path) {
    // This is deliberately best-effort. A non-empty directory means another
    // transaction or a manual-recovery journal still needs it; remove_dir then
    // fails safely without disturbing that state.
    let _ = fs::remove_dir(dir);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum EntryState {
    Original,
    Candidate,
    Manual,
}

pub fn recover_all(workspace: &Workspace) -> RecoveryReport {
    let mut report = RecoveryReport {
        inspected: 0,
        completed: 0,
        restored: 0,
        cleaned: 0,
        manual: Vec::new(),
    };
    for dir in recovery_dirs(workspace) {
        recover_dir(workspace, &dir, &mut report);
    }
    report
}

#[derive(Serialize, Clone, Debug)]
pub struct RecoveryListReport {
    pub entries: Vec<RecoveryListEntry>,
}

#[derive(Serialize, Clone, Debug)]
pub struct RecoveryListEntry {
    pub transaction_id: String,
    pub journal_validity: String,
    pub member_count: usize,
    pub apparent_state: String,
    pub automatic_recovery_safe: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct RecoveryInspectReport {
    pub transaction_id: String,
    pub journal_validity: String,
    pub member_count: usize,
    pub members: Vec<RecoveryMemberInspection>,
    pub automatic_recovery_safe: bool,
}

#[derive(Serialize, Clone, Debug)]
pub struct RecoveryMemberInspection {
    pub path: String,
    pub pre_hash: String,
    pub candidate_hash: String,
    pub current_hash: Option<String>,
    pub classification: String,
    pub automatic_recovery_safe: bool,
}

pub fn list(workspace: &Workspace) -> RecoveryListReport {
    let mut entries = Vec::new();
    for path in journal_paths(workspace) {
        let transaction_id = path
            .file_stem()
            .and_then(|value| value.to_str())
            .unwrap_or("unreadable")
            .to_owned();
        let journal = match validate_journal(workspace, &path) {
            Ok(journal) => journal,
            Err(_) => {
                entries.push(RecoveryListEntry {
                    transaction_id,
                    journal_validity: if path.exists() {
                        "invalid"
                    } else {
                        "unreadable"
                    }
                    .into(),
                    member_count: 0,
                    apparent_state: "invalid".into(),
                    automatic_recovery_safe: false,
                });
                continue;
            }
        };
        let states = member_inspections(workspace, &journal);
        let safe = states.iter().all(|member| member.automatic_recovery_safe);
        let apparent_state = if !safe {
            "drifted"
        } else if states
            .iter()
            .all(|member| member.classification == "CANDIDATE")
        {
            "complete"
        } else {
            "partial"
        };
        entries.push(RecoveryListEntry {
            transaction_id,
            journal_validity: "valid".into(),
            member_count: states.len(),
            apparent_state: apparent_state.into(),
            automatic_recovery_safe: safe,
        });
    }
    RecoveryListReport { entries }
}

pub fn inspect(workspace: &Workspace, transaction_id: &str) -> RecoveryInspectReport {
    let Some(path) = find_journal(workspace, transaction_id) else {
        return RecoveryInspectReport {
            transaction_id: transaction_id.into(),
            journal_validity: "unreadable".into(),
            member_count: 0,
            members: Vec::new(),
            automatic_recovery_safe: false,
        };
    };
    let journal = match validate_journal(workspace, &path) {
        Ok(journal) => journal,
        Err(_) => {
            return RecoveryInspectReport {
                transaction_id: transaction_id.into(),
                journal_validity: "invalid".into(),
                member_count: 0,
                members: Vec::new(),
                automatic_recovery_safe: false,
            }
        }
    };
    let members = member_inspections(workspace, &journal);
    RecoveryInspectReport {
        transaction_id: transaction_id.into(),
        journal_validity: "valid".into(),
        member_count: members.len(),
        automatic_recovery_safe: members.iter().all(|member| member.automatic_recovery_safe),
        members,
    }
}

pub fn recover_transaction(workspace: &Workspace, transaction_id: &str) -> RecoveryReport {
    let mut report = RecoveryReport {
        inspected: 0,
        completed: 0,
        restored: 0,
        cleaned: 0,
        manual: Vec::new(),
    };
    if let Some(path) = find_journal(workspace, transaction_id) {
        report.inspected = 1;
        recover_item(workspace, &path, &mut report);
    } else {
        report
            .manual
            .push(format!("transaction not found: {transaction_id}"));
    }
    report
}

fn recover_dir(workspace: &Workspace, dir: &std::path::Path, report: &mut RecoveryReport) {
    let entries = match fs::read_dir(dir) {
        Ok(x) => x,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return,
        Err(_) => return,
    };
    for item in entries.flatten() {
        if item.path().extension().and_then(|x| x.to_str()) != Some("json") {
            continue;
        }
        report.inspected += 1;
        recover_item(workspace, &item.path(), report);
    }
    remove_recovery_dir_if_empty(dir);
}

fn recover_item(workspace: &Workspace, path: &std::path::Path, report: &mut RecoveryReport) {
    let journal: Journal = match validate_journal(workspace, path) {
        Ok(x) => x,
        Err(reason) => {
            report.manual.push(format!("{}: {reason}", path.display()));
            return;
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
        return;
    }
    if states.iter().all(|state| *state == EntryState::Candidate) {
        if fs::remove_file(path).is_ok() {
            report.completed += 1;
            report.cleaned += 1;
        }
        return;
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
    if safe && fs::remove_file(path).is_ok() {
        report.cleaned += 1;
    }
}

fn journal_paths(workspace: &Workspace) -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    for dir in recovery_dirs(workspace) {
        let Ok(entries) = fs::read_dir(dir) else {
            continue;
        };
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|value| value.to_str()) == Some("json") {
                paths.push(entry.path());
            }
        }
    }
    paths.sort();
    paths
}

fn find_journal(workspace: &Workspace, transaction_id: &str) -> Option<std::path::PathBuf> {
    if transaction_id.is_empty() || safe_id(transaction_id) != transaction_id {
        return None;
    }
    journal_paths(workspace)
        .into_iter()
        .find(|path| path.file_stem().and_then(|value| value.to_str()) == Some(transaction_id))
}

fn validate_journal(workspace: &Workspace, path: &std::path::Path) -> Result<Journal, String> {
    let filename_id = path
        .file_stem()
        .and_then(|value| value.to_str())
        .ok_or_else(|| "invalid journal filename".to_owned())?;
    let bytes = fs::read(path).map_err(|error| format!("unreadable journal: {error}"))?;
    if bytes.len() > MAX_JOURNAL_BYTES {
        return Err(format!("journal exceeds {MAX_JOURNAL_BYTES} bytes"));
    }
    let journal: Journal =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid JSON: {error}"))?;
    if journal.protocol_version != "1.1.0" {
        return Err("unsupported journal protocol version".into());
    }
    if journal.transaction_id.is_empty()
        || safe_id(&journal.transaction_id) != journal.transaction_id
        || filename_id != journal.transaction_id
    {
        return Err("transaction ID and filename disagree".into());
    }
    if journal.entries.is_empty() || journal.entries.len() > MAX_JOURNAL_ENTRIES {
        return Err("invalid journal entry count".into());
    }
    let mut paths = std::collections::HashSet::new();
    for entry in &journal.entries {
        if entry.path.is_empty()
            || entry.path.starts_with('/')
            || entry.path.contains('\\')
            || entry
                .path
                .split('/')
                .any(|part| part == ".." || part.is_empty())
            || !paths.insert(entry.path.clone())
        {
            return Err(format!("invalid or duplicate member path: {}", entry.path));
        }
        workspace
            .resolve_path(&entry.path)
            .map_err(|error| format!("unsafe member path {}: {error}", entry.path))?;
        if entry.original.len() > crate::protocol::MAX_FILE_BYTES
            || entry.candidate.len() > crate::protocol::MAX_FILE_BYTES
        {
            return Err(format!("embedded payload too large: {}", entry.path));
        }
        if compute_sha256(&entry.original) != entry.pre_hash
            || compute_sha256(&entry.candidate) != entry.candidate_hash
        {
            return Err(format!("embedded payload hash mismatch: {}", entry.path));
        }
    }
    Ok(journal)
}

fn member_inspections(workspace: &Workspace, journal: &Journal) -> Vec<RecoveryMemberInspection> {
    journal
        .entries
        .iter()
        .map(|entry| match workspace.read_file(&entry.path) {
            Ok(current) => {
                let current_hash = compute_sha256(&current);
                let classification = if current_hash == entry.pre_hash {
                    "ORIGINAL"
                } else if current_hash == entry.candidate_hash {
                    "CANDIDATE"
                } else {
                    "DRIFTED"
                };
                RecoveryMemberInspection {
                    path: entry.path.clone(),
                    pre_hash: entry.pre_hash.clone(),
                    candidate_hash: entry.candidate_hash.clone(),
                    current_hash: Some(current_hash),
                    classification: classification.into(),
                    automatic_recovery_safe: classification != "DRIFTED",
                }
            }
            Err(_) => RecoveryMemberInspection {
                path: entry.path.clone(),
                pre_hash: entry.pre_hash.clone(),
                candidate_hash: entry.candidate_hash.clone(),
                current_hash: None,
                classification: "UNREADABLE".into(),
                automatic_recovery_safe: false,
            },
        })
        .collect()
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
        assert!(!temp.path().join(".threadmoth-recovery").exists());
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
        assert!(!temp.path().join(".threadmoth-recovery").exists());
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
        assert!(temp
            .path()
            .join(".threadmoth-recovery/changed.json")
            .exists());
        assert!(temp.path().join(".threadmoth-recovery").exists());
    }

    #[test]
    fn remove_journal_removes_the_directory_when_it_becomes_empty() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::new(temp.path()).unwrap();
        workspace.write_file_atomic("x.txt", b"old").unwrap();
        journal_for(&workspace, "finished", vec![entry("x.txt", b"old", b"new")]);

        remove_journal(&workspace, "finished").unwrap();

        assert!(!temp.path().join(".threadmoth-recovery").exists());
    }

    #[test]
    fn remove_journal_keeps_directory_when_another_journal_remains() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::new(temp.path()).unwrap();
        workspace.write_file_atomic("x.txt", b"old").unwrap();
        let recovery_entry = entry("x.txt", b"old", b"new");
        journal_for(&workspace, "one", vec![recovery_entry.clone()]);
        journal_for(&workspace, "two", vec![recovery_entry]);

        remove_journal(&workspace, "one").unwrap();

        assert!(temp.path().join(".threadmoth-recovery").exists());
        assert!(temp.path().join(".threadmoth-recovery/two.json").exists());
    }

    #[test]
    fn recovery_reads_legacy_suture_directory() {
        let temp = TempDir::new().unwrap();
        let workspace = Workspace::new(temp.path()).unwrap();
        workspace.write_file_atomic("x.txt", b"new").unwrap();
        let journal = Journal {
            protocol_version: "1.1.0".into(),
            transaction_id: "legacy".into(),
            entries: vec![entry("x.txt", b"old", b"new")],
        };
        let legacy = temp.path().join(LEGACY_RECOVERY_DIR);
        fs::create_dir_all(&legacy).unwrap();
        fs::write(
            legacy.join("legacy.json"),
            serde_json::to_vec_pretty(&journal).unwrap(),
        )
        .unwrap();

        let report = recover_all(&workspace);

        assert_eq!(report.completed, 1);
        assert_eq!(report.cleaned, 1);
        assert!(!legacy.exists());
        assert_eq!(workspace.read_file("x.txt").unwrap(), b"new");
    }
}
