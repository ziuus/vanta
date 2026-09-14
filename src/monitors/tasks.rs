use std::fs;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Clone, Default)]
pub struct Task {
    pub completed: bool,
    pub text: String,
    pub urgent: bool,
}

#[derive(Clone, Default)]
pub struct TasksSnapshot {
    pub tasks: Vec<Task>,
    pub last_modified: u64,
}

static SNAP: LazyLock<Mutex<TasksSnapshot>> =
    LazyLock::new(|| Mutex::new(TasksSnapshot::default()));

pub fn snapshot() -> TasksSnapshot {
    SNAP.lock().unwrap().clone()
}

pub fn get_todo_file() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("vanta");
        let _ = fs::create_dir_all(&path);
        path.push("todo.md");
        path
    } else {
        PathBuf::from("todo.md")
    }
}

pub fn ensure_todo_file() {
    let file_path = get_todo_file();
    if !file_path.exists() {
        let default_content = "# Vanta Tasks\n\n- [ ] Welcome to Vanta Workspace!\n- [ ] Press Space to toggle completion\n- [ ] Press Enter or 'e' to edit in your terminal editor\n- [ ] Tasks marked with ! are urgent\n";
        let _ = fs::write(&file_path, default_content);
    }
}

pub fn rescan() {
    let file_path = get_todo_file();
    if let Ok(content) = fs::read_to_string(&file_path) {
        let mut tasks = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.starts_with("- [ ]") || line.starts_with("- [x]") || line.starts_with("- [X]") {
                let completed = line.contains("[x]") || line.contains("[X]");
                let text = line[5..].trim().to_string();
                let urgent = text.contains("!") || text.contains("urgent") || text.contains("ASAP");
                tasks.push(Task {
                    completed,
                    text,
                    urgent,
                });
            }
        }
        let modified = fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        *SNAP.lock().unwrap() = TasksSnapshot {
            tasks,
            last_modified: modified,
        };
    }
}

pub fn toggle_task(index: usize) {
    let file_path = get_todo_file();
    let Ok(content) = fs::read_to_string(&file_path) else {
        return;
    };

    let mut task_idx = 0;
    let mut new_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- [ ]")
            || trimmed.starts_with("- [x]")
            || trimmed.starts_with("- [X]")
        {
            if task_idx == index {
                if trimmed.starts_with("- [ ]") {
                    new_lines.push(line.replacen("- [ ]", "- [x]", 1));
                } else if trimmed.starts_with("- [x]") {
                    new_lines.push(line.replacen("- [x]", "- [ ]", 1));
                } else if trimmed.starts_with("- [X]") {
                    new_lines.push(line.replacen("- [X]", "- [ ]", 1));
                }
            } else {
                new_lines.push(line.to_string());
            }
            task_idx += 1;
        } else {
            new_lines.push(line.to_string());
        }
    }

    let updated_content = new_lines.join("\n") + "\n";
    let _ = fs::write(&file_path, updated_content);
    rescan();
}

pub fn add_task(text: &str) {
    let file_path = get_todo_file();
    ensure_todo_file();
    if let Ok(mut content) = fs::read_to_string(&file_path) {
        if !content.ends_with('\n') && !content.is_empty() {
            content.push('\n');
        }
        content.push_str(&format!("- [ ] {}\n", text.trim()));
        let _ = fs::write(&file_path, content);
        rescan();
    }
}

pub fn delete_task(index: usize) {
    let file_path = get_todo_file();
    let Ok(content) = fs::read_to_string(&file_path) else {
        return;
    };

    let mut task_idx = 0;
    let mut new_lines = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("- [ ]")
            || trimmed.starts_with("- [x]")
            || trimmed.starts_with("- [X]")
        {
            if task_idx != index {
                new_lines.push(line.to_string());
            }
            task_idx += 1;
        } else {
            new_lines.push(line.to_string());
        }
    }

    let updated_content = new_lines.join("\n") + "\n";
    let _ = fs::write(&file_path, updated_content);
    rescan();
}

pub fn start() {
    ensure_todo_file();
    rescan();

    std::thread::spawn(|| loop {
        let file_path = get_todo_file();

        let modified = fs::metadata(&file_path)
            .and_then(|m| m.modified())
            .unwrap_or(SystemTime::now())
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let mut needs_update = false;
        {
            let snap = SNAP.lock().unwrap();
            if snap.last_modified != modified {
                needs_update = true;
            }
        }

        if needs_update {
            rescan();
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}
