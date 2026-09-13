use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::Instant;

#[derive(Clone, Debug)]
pub struct FileItem {
    pub name: String,
    pub path: PathBuf,
    pub is_dir: bool,
    pub size: u64,
}

#[derive(Clone, Debug)]
pub struct FilesSnapshot {
    pub current_dir: PathBuf,
    pub items: Vec<FileItem>,
    pub preview: Option<String>,
}

static STATE: Mutex<Option<FilesSnapshot>> = Mutex::new(None);

pub fn init(start_dir: &Path) {
    let snap = read_dir(start_dir);
    let first_path = snap.items.get(0).map(|i| i.path.clone());
    {
        let mut state = STATE.lock().unwrap();
        if state.is_none() {
            *state = Some(snap);
        }
    }
    if let Some(p) = first_path {
        update_preview(&p);
    }
}

pub fn read_dir(path: &Path) -> FilesSnapshot {
    let mut items = Vec::new();
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(meta) = entry.metadata() {
                items.push(FileItem {
                    name: entry.file_name().to_string_lossy().into_owned(),
                    path: entry.path(),
                    is_dir: meta.is_dir(),
                    size: meta.len(),
                });
            }
        }
    }
    // Sort directories first, then alphabetical
    items.sort_by(|a, b| match (a.is_dir, b.is_dir) {
        (true, false) => std::cmp::Ordering::Less,
        (false, true) => std::cmp::Ordering::Greater,
        _ => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
    });

    FilesSnapshot {
        current_dir: path.to_path_buf(),
        items,
        preview: None,
    }
}

pub fn update_preview(selected_path: &Path) {
    let preview = if selected_path.is_dir() {
        // Preview directory contents
        if let Ok(entries) = fs::read_dir(selected_path) {
            let mut list = String::new();
            for (i, entry) in entries.flatten().enumerate().take(20) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
                list.push_str(&format!("{} {}\n", if is_dir { "📁" } else { "📄" }, name));
            }
            Some(list)
        } else {
            None
        }
    } else {
        // Preview file contents (first few KB)
        fs::read_to_string(selected_path).ok().map(|s| s.chars().take(2000).collect())
    };

    let mut state = STATE.lock().unwrap();
    if let Some(snap) = state.as_mut() {
        snap.preview = preview;
    }
}

pub fn chdir(path: &Path) {
    let snap = read_dir(path);
    let first_path = snap.items.get(0).map(|i| i.path.clone());
    {
        let mut state = STATE.lock().unwrap();
        *state = Some(snap);
    }
    if let Some(p) = first_path {
        update_preview(&p);
    }
}

pub fn snapshot() -> FilesSnapshot {
    STATE.lock().unwrap().clone().unwrap_or_else(|| FilesSnapshot {
        current_dir: PathBuf::from("."),
        items: Vec::new(),
        preview: None,
    })
}
