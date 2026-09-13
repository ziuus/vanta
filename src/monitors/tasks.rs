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
        path.push("todo.md");
        path
    } else {
        PathBuf::from("todo.md")
    }
}

pub fn start() {
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
            if let Ok(content) = fs::read_to_string(&file_path) {
                let mut tasks = Vec::new();
                for line in content.lines() {
                    let line = line.trim();
                    if line.starts_with("- [ ]")
                        || line.starts_with("- [x]")
                        || line.starts_with("- [X]")
                    {
                        let completed = line.contains("[x]") || line.contains("[X]");
                        let text = line[5..].trim().to_string();
                        let urgent =
                            text.contains("!") || text.contains("urgent") || text.contains("ASAP");
                        tasks.push(Task {
                            completed,
                            text,
                            urgent,
                        });
                    }
                }
                *SNAP.lock().unwrap() = TasksSnapshot {
                    tasks,
                    last_modified: modified,
                };
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(2));
    });
}
