use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Mutex;

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
            for (_i, entry) in entries.flatten().enumerate().take(20) {
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.metadata().map(|m| m.is_dir()).unwrap_or(false);
                list.push_str(&format!("{} {}
", if is_dir { "📁" } else { "📄" }, name));
            }
            Some(list)
        } else {
            None
        }
    } else {
        // Preview file contents (first few KB)
        match fs::read_to_string(selected_path) {
            Ok(s) => Some(s.chars().take(2000).collect()),
            Err(_) => {
                if let Ok(metadata) = fs::metadata(selected_path) {
                    let size = metadata.len();
                    let size_str = if size >= 1024 * 1024 {
                        format!("{:.1} MB", size as f64 / 1024.0 / 1024.0)
                    } else if size >= 1024 {
                        format!("{:.1} KB", size as f64 / 1024.0)
                    } else {
                        format!("{} bytes", size)
                    };
                    let ext = selected_path.extension().unwrap_or_default().to_string_lossy();
                    let file_type = if ext == "png" || ext == "jpg" || ext == "jpeg" || ext == "gif" {
                        "Image File"
                    } else if ext == "pdf" {
                        "PDF Document"
                    } else if ext == "mp4" || ext == "mkv" {
                        "Video File"
                    } else if ext == "zip" || ext == "tar" || ext == "gz" {
                        "Archive"
                    } else {
                        "Binary File"
                    };
                    Some(format!("
  [ {} ]

  Size: {}

  (Preview not available for this file type)", file_type, size_str))
                } else {
                    Some("Unable to read file".to_string())
                }
            }
        }
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
