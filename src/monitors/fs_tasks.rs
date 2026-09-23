use serde::Serialize;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex};
use std::thread;
use std::time::Instant;

#[derive(Clone, Debug, Serialize)]
pub struct TaskProgress {
    pub id: String,
    pub operation: String,
    pub source: String,
    pub dest: String,
    pub progress: f64,
    pub bytes_processed: u64,
    pub total_bytes: u64,
    pub speed_bps: f64,
    pub status: String,
    pub error: Option<String>,
}

pub struct FsTasks {
    tasks: Mutex<Vec<TaskProgress>>,
}

pub static TASKS: LazyLock<Arc<FsTasks>> = LazyLock::new(|| {
    Arc::new(FsTasks {
        tasks: Mutex::new(Vec::new()),
    })
});

pub fn start_copy(id: String, src: String, dest: String) {
    let t_id = id.clone();
    let src_path = PathBuf::from(&src);
    let dest_path = PathBuf::from(&dest);

    {
        let mut t = TASKS.tasks.lock().unwrap();
        t.push(TaskProgress {
            id: id.clone(),
            operation: "Copy".into(),
            source: src.clone(),
            dest: dest.clone(),
            progress: 0.0,
            bytes_processed: 0,
            total_bytes: 0,
            speed_bps: 0.0,
            status: "Running".into(),
            error: None,
        });
    }

    thread::spawn(move || {
        let meta = match fs::metadata(&src_path) {
            Ok(m) => m,
            Err(e) => {
                set_error(&t_id, &e.to_string());
                return;
            }
        };
        let total = meta.len();
        set_total(&t_id, total);

        let mut src_file = match fs::File::open(&src_path) {
            Ok(f) => f,
            Err(e) => {
                set_error(&t_id, &e.to_string());
                return;
            }
        };
        let mut dest_file = match fs::File::create(&dest_path) {
            Ok(f) => f,
            Err(e) => {
                set_error(&t_id, &e.to_string());
                return;
            }
        };

        use std::io::{Read, Write};
        let mut buf = vec![0; 1024 * 64];
        let mut processed = 0;
        let start_time = Instant::now();

        loop {
            match src_file.read(&mut buf) {
                Ok(0) => break,
                Ok(n) => {
                    if let Err(e) = dest_file.write_all(&buf[..n]) {
                        set_error(&t_id, &e.to_string());
                        return;
                    }
                    processed += n as u64;
                    let elapsed = start_time.elapsed().as_secs_f64();
                    let speed = if elapsed > 0.0 {
                        processed as f64 / elapsed
                    } else {
                        0.0
                    };
                    update_progress(&t_id, processed, speed);
                }
                Err(e) => {
                    set_error(&t_id, &e.to_string());
                    return;
                }
            }
        }
        complete(&t_id);
    });
}

pub fn start_trash(id: String, path: String) {
    let t_id = id.clone();

    {
        let mut t = TASKS.tasks.lock().unwrap();
        t.push(TaskProgress {
            id: id.clone(),
            operation: "Trash".into(),
            source: path.clone(),
            dest: "".into(),
            progress: 0.0,
            bytes_processed: 0,
            total_bytes: 0,
            speed_bps: 0.0,
            status: "Running".into(),
            error: None,
        });
    }

    thread::spawn(move || {
        if let Err(e) = trash::delete(&path) {
            set_error(&t_id, &e.to_string());
        } else {
            complete(&t_id);
        }
    });
}

fn set_total(id: &str, total: u64) {
    let mut t = TASKS.tasks.lock().unwrap();
    if let Some(task) = t.iter_mut().find(|x| x.id == id) {
        task.total_bytes = total;
    }
}

fn update_progress(id: &str, processed: u64, speed: f64) {
    let mut t = TASKS.tasks.lock().unwrap();
    if let Some(task) = t.iter_mut().find(|x| x.id == id) {
        task.bytes_processed = processed;
        task.speed_bps = speed;
        if task.total_bytes > 0 {
            task.progress = processed as f64 / task.total_bytes as f64;
        }
    }
}

fn set_error(id: &str, err: &str) {
    let mut t = TASKS.tasks.lock().unwrap();
    if let Some(task) = t.iter_mut().find(|x| x.id == id) {
        task.status = "Error".into();
        task.error = Some(err.to_string());
    }
}

fn complete(id: &str) {
    let mut t = TASKS.tasks.lock().unwrap();
    if let Some(task) = t.iter_mut().find(|x| x.id == id) {
        task.status = "Complete".into();
        task.progress = 1.0;
        task.bytes_processed = task.total_bytes;
    }
}

pub fn snapshot() -> Vec<TaskProgress> {
    TASKS.tasks.lock().unwrap().clone()
}

pub fn start_search(id: String, dir: String, query: String) {
    let t_id = id.clone();

    {
        let mut t = TASKS.tasks.lock().unwrap();
        t.push(TaskProgress {
            id: id.clone(),
            operation: "Search".into(),
            source: dir.clone(),
            dest: query.clone(),
            progress: 0.0,
            bytes_processed: 0,
            total_bytes: 0,
            speed_bps: 0.0,
            status: "Running".into(),
            error: None,
        });
    }

    thread::spawn(move || {
        let q = query.to_lowercase();
        let mut results = Vec::new();
        let mut count = 0;
        let start_time = Instant::now();

        for entry in walkdir::WalkDir::new(&dir)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if entry
                .file_name()
                .to_string_lossy()
                .to_lowercase()
                .contains(&q)
            {
                results.push(entry.path().to_string_lossy().to_string());
                count += 1;
                if count > 1000 {
                    break;
                } // limit to avoid massive lists
            }
            if start_time.elapsed().as_secs() > 10 {
                // Timeout after 10 seconds
                break;
            }
        }

        {
            let mut t = TASKS.tasks.lock().unwrap();
            if let Some(task) = t.iter_mut().find(|x| x.id == t_id) {
                task.status = "Complete".into();
                task.progress = 1.0;
                // serialize results into error field since we don't have a results array, or just as a JSON string in dest
                task.error = Some(serde_json::to_string(&results).unwrap_or_default());
            }
        }
    });
}
