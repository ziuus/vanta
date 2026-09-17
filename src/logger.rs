use log::{Level, Log, Metadata, Record};
use std::sync::RwLock;

pub struct LogEntry {
    pub level: Level,
    pub target: String,
    pub message: String,
    pub timestamp: String,
}

pub struct MemoryLogger;

pub static LOGS: RwLock<Vec<LogEntry>> = RwLock::new(Vec::new());

impl Log for MemoryLogger {
    fn enabled(&self, _metadata: &Metadata) -> bool {
        true
    }

    fn log(&self, record: &Record) {
        if self.enabled(record.metadata()) {
            let mut logs = LOGS.write().unwrap();
            logs.push(LogEntry {
                level: record.level(),
                target: record.target().to_string(),
                message: format!("{}", record.args()),
                timestamp: chrono::Local::now().format("%H:%M:%S").to_string(),
            });
            if logs.len() > 1000 {
                logs.remove(0);
            }
        }
    }

    fn flush(&self) {}
}

pub fn init() {
    log::set_logger(&MemoryLogger).unwrap();
    log::set_max_level(log::LevelFilter::Debug);
}
