use std::fs::File;
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Utc};
use ical::IcalParser;

#[derive(Clone, Default)]
pub struct Event {
    pub summary: String,
    pub start_time: DateTime<Local>,
    pub end_time: Option<DateTime<Local>>,
}

#[derive(Clone, Default)]
pub struct AgendaSnapshot {
    pub events: Vec<Event>,
    pub last_modified: u64,
}

static SNAP: LazyLock<Mutex<AgendaSnapshot>> =
    LazyLock::new(|| Mutex::new(AgendaSnapshot::default()));

pub fn snapshot() -> AgendaSnapshot {
    SNAP.lock().unwrap().clone()
}

pub fn get_agenda_file() -> PathBuf {
    if let Ok(home) = std::env::var("HOME") {
        let mut path = PathBuf::from(home);
        path.push(".config");
        path.push("vanta");
        path.push("agenda.ics");
        path
    } else {
        PathBuf::from("agenda.ics")
    }
}

fn parse_ical_date(dt: &str) -> Option<DateTime<Local>> {
    // Basic support for YYYYMMDDTHHMMSSZ
    if dt.ends_with('Z') {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(dt, "%Y%m%dT%H%M%SZ") {
            return Some(Utc.from_utc_datetime(&ndt).with_timezone(&Local));
        }
    } else {
        // Local time (ignoring TZID for now to keep it lightweight)
        if let Ok(ndt) = NaiveDateTime::parse_from_str(dt, "%Y%m%dT%H%M%S") {
            return Some(Local.from_local_datetime(&ndt).single()?);
        }
        // Date only
        if let Ok(nd) = chrono::NaiveDate::parse_from_str(dt, "%Y%m%d") {
            if let Some(ndt) = nd.and_hms_opt(0, 0, 0) {
                return Some(Local.from_local_datetime(&ndt).single()?);
            }
        }
    }
    None
}

pub fn start() {
    std::thread::spawn(|| loop {
        let file_path = get_agenda_file();

        let modified = std::fs::metadata(&file_path)
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
            if let Ok(file) = File::open(&file_path) {
                let buf = BufReader::new(file);
                let parser = IcalParser::new(buf);

                let mut events = Vec::new();
                for calendar in parser.flatten() {
                    for event in calendar.events {
                        let mut summary = String::new();
                        let mut start_time = None;
                        let mut end_time = None;

                        for prop in event.properties {
                            match prop.name.as_str() {
                                "SUMMARY" => {
                                    if let Some(val) = prop.value {
                                        summary = val;
                                    }
                                }
                                "DTSTART" => {
                                    if let Some(val) = prop.value {
                                        start_time = parse_ical_date(&val);
                                    }
                                }
                                "DTEND" => {
                                    if let Some(val) = prop.value {
                                        end_time = parse_ical_date(&val);
                                    }
                                }
                                _ => {}
                            }
                        }

                        if let Some(st) = start_time {
                            // Only keep future events or currently running ones
                            let now = Local::now();
                            if end_time.unwrap_or(st) >= now || st >= now {
                                events.push(Event {
                                    summary,
                                    start_time: st,
                                    end_time,
                                });
                            }
                        }
                    }
                }

                events.sort_by_key(|e| e.start_time);
                *SNAP.lock().unwrap() = AgendaSnapshot {
                    events,
                    last_modified: modified,
                };
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
}
