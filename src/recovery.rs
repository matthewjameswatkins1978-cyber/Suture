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

pub fn recover_all(workspace: &Workspace) -> RecoveryReport {
    let dir = workspace.root().join(".suture-recovery");
    let mut report = RecoveryReport {
        inspected: 0,
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
        let mut safe = true;
        for entry in &journal.entries {
            let current = match workspace.read_file(&entry.path) {
                Ok(x) => x,
                Err(_) => {
                    safe = false;
                    report.manual.push(format!("{}: unreadable", entry.path));
                    continue;
                }
            };
            let current_hash = compute_sha256(&current);
            if current_hash == entry.candidate_hash {
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
            } else if current_hash != entry.pre_hash {
                safe = false;
                report
                    .manual
                    .push(format!("{}: changed after interruption", entry.path));
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
