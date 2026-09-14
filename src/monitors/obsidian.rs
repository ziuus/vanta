use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::SystemTime;

#[derive(Clone)]
pub struct Note {
    pub path: PathBuf,
    pub title: String,
    pub modified: SystemTime,
    pub content: String,
}

impl Default for Note {
    fn default() -> Self {
        Self {
            path: PathBuf::new(),
            title: String::new(),
            modified: SystemTime::UNIX_EPOCH,
            content: String::new(),
        }
    }
}

#[derive(Clone, Default)]
pub struct ObsidianSnapshot {
    pub notes: Vec<Note>,
}

static SNAP: LazyLock<Mutex<ObsidianSnapshot>> =
    LazyLock::new(|| Mutex::new(ObsidianSnapshot::default()));

pub fn snapshot() -> ObsidianSnapshot {
    SNAP.lock().unwrap().clone()
}

pub fn start(vault_path_str: String) {
    std::thread::spawn(move || loop {
        let mut vault_path = PathBuf::from(&vault_path_str);
        if vault_path_str.starts_with("~/") || vault_path_str == "~" {
            if let Ok(home) = std::env::var("HOME") {
                if vault_path_str == "~" {
                    vault_path = PathBuf::from(home);
                } else {
                    vault_path = PathBuf::from(home).join(&vault_path_str[2..]);
                }
            }
        }

        let mut notes = Vec::new();

        let mut dirs = vec![vault_path];
        let mut depth = 0;

        while !dirs.is_empty() && depth < 4 {
            let mut next_dirs = Vec::new();
            for dir in dirs {
                if let Ok(entries) = std::fs::read_dir(&dir) {
                    for entry in entries.flatten() {
                        if let Ok(meta) = entry.metadata() {
                            if meta.is_dir() {
                                let name = entry.file_name();
                                let name_str = name.to_string_lossy();
                                if !name_str.starts_with('.') && name_str != "node_modules" {
                                    next_dirs.push(entry.path());
                                }
                            } else if entry.path().extension().is_some_and(|e| e == "md") {
                                let path = entry.path();
                                let title = path
                                    .file_stem()
                                    .unwrap_or_default()
                                    .to_string_lossy()
                                    .into_owned();
                                let modified = meta.modified().unwrap_or(SystemTime::UNIX_EPOCH);
                                let content = std::fs::read_to_string(&path).unwrap_or_default();
                                notes.push(Note {
                                    path,
                                    title,
                                    modified,
                                    content,
                                });
                            }
                        }
                    }
                }
            }
            dirs = next_dirs;
            depth += 1;
        }

        notes.sort_by(|a, b| b.modified.cmp(&a.modified));
        notes.truncate(50);

        *SNAP.lock().unwrap() = ObsidianSnapshot { notes };

        std::thread::sleep(std::time::Duration::from_secs(5));
    });
}
