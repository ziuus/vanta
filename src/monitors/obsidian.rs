use std::path::{Path, PathBuf};
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
    pub vault_name: String,
    #[allow(dead_code)]
    pub vault_path: PathBuf,
    pub notes: Vec<Note>,
}

static SNAP: LazyLock<Mutex<ObsidianSnapshot>> =
    LazyLock::new(|| Mutex::new(ObsidianSnapshot::default()));

pub fn snapshot() -> ObsidianSnapshot {
    SNAP.lock().unwrap().clone()
}

pub fn detect_vault_path(configured: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();

    // 1. Explicit user path configured
    if !configured.is_empty() && configured != "~" {
        if let Some(stripped) = configured.strip_prefix("~/") {
            return PathBuf::from(&home).join(stripped);
        }
        return PathBuf::from(configured);
    }

    // 2. Auto-detect from Obsidian's official config
    let config_candidates = [
        PathBuf::from(&home).join(".config/obsidian/obsidian.json"),
        PathBuf::from(&home).join("Library/Application Support/obsidian/obsidian.json"),
    ];

    for cfg_path in &config_candidates {
        if let Ok(data) = std::fs::read_to_string(cfg_path) {
            if let Ok(val) = serde_json::from_str::<serde_json::Value>(&data) {
                if let Some(vaults) = val.get("vaults").and_then(|v| v.as_object()) {
                    // Try to find open vault first
                    for (_, v_info) in vaults {
                        if v_info
                            .get("open")
                            .and_then(|o| o.as_bool())
                            .unwrap_or(false)
                        {
                            if let Some(p) = v_info.get("path").and_then(|p| p.as_str()) {
                                let pb = PathBuf::from(p);
                                if pb.exists() {
                                    return pb;
                                }
                            }
                        }
                    }
                    // Fall back to any listed vault
                    for (_, v_info) in vaults {
                        if let Some(p) = v_info.get("path").and_then(|p| p.as_str()) {
                            let pb = PathBuf::from(p);
                            if pb.exists() {
                                return pb;
                            }
                        }
                    }
                }
            }
        }
    }

    // 3. Common directory locations
    let common_fallbacks = [
        PathBuf::from(&home).join("Documents/Obsidian Vault"),
        PathBuf::from(&home).join("Documents/Obsidian"),
        PathBuf::from(&home).join("Documents/Notes"),
        PathBuf::from(&home).join("Obsidian Vault"),
        PathBuf::from(&home).join("Obsidian"),
        PathBuf::from(&home).join("vault"),
    ];
    for fb in common_fallbacks {
        if fb.exists() {
            return fb;
        }
    }

    if configured == "~" {
        PathBuf::from(home)
    } else {
        PathBuf::from(configured)
    }
}

fn scan_vault(vault_path: &Path) -> ObsidianSnapshot {
    let mut notes = Vec::new();
    let mut dirs = vec![vault_path.to_path_buf()];
    let mut depth = 0;

    while !dirs.is_empty() && depth < 5 {
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

    notes.sort_by_key(|a| std::cmp::Reverse(a.modified));
    notes.truncate(100);

    let vault_name = vault_path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Vault".to_string());

    ObsidianSnapshot {
        vault_name,
        vault_path: vault_path.to_path_buf(),
        notes,
    }
}

pub fn rescan() {
    let cfg = crate::config::Config::load();
    let vault_path = detect_vault_path(&cfg.ui.obsidian_vault);
    let fresh = scan_vault(&vault_path);
    *SNAP.lock().unwrap() = fresh;
}

pub fn start(configured_vault: String) {
    let vault_path = detect_vault_path(&configured_vault);
    // Initial scan
    *SNAP.lock().unwrap() = scan_vault(&vault_path);

    std::thread::spawn(move || loop {
        std::thread::sleep(std::time::Duration::from_secs(4));
        let vault_path = detect_vault_path(&configured_vault);
        let fresh = scan_vault(&vault_path);
        *SNAP.lock().unwrap() = fresh;
    });
}
