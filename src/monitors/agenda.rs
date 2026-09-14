use std::fs::{self, File};
use std::io::BufReader;
use std::path::PathBuf;
use std::sync::{LazyLock, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use chrono::{DateTime, Local, NaiveDateTime, TimeZone, Timelike, Utc};
use ical::IcalParser;

#[derive(Clone, Default, Debug, PartialEq)]
pub struct Event {
    pub uid: String,
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
        let _ = fs::create_dir_all(&path);
        path.push("agenda.ics");
        path
    } else {
        PathBuf::from("agenda.ics")
    }
}

pub fn ensure_agenda_file() {
    let file_path = get_agenda_file();
    if !file_path.exists() {
        let now = Local::now();
        let dtstart = (now + chrono::Duration::hours(2))
            .format("%Y%m%dT%H%M00")
            .to_string();
        let dtend = (now + chrono::Duration::hours(3))
            .format("%Y%m%dT%H%M00")
            .to_string();
        let default_content = format!(
            "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Vanta//EN\nBEGIN:VEVENT\nSUMMARY:Team Standup & Review\nDTSTART:{}\nDTEND:{}\nEND:VEVENT\nEND:VCALENDAR\n",
            dtstart, dtend
        );
        let _ = fs::write(&file_path, default_content);
    }
}

fn parse_ical_date(dt: &str) -> Option<DateTime<Local>> {
    if dt.ends_with('Z') {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(dt, "%Y%m%dT%H%M%SZ") {
            return Some(Utc.from_utc_datetime(&ndt).with_timezone(&Local));
        }
    } else {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(dt, "%Y%m%dT%H%M%S") {
            return Local.from_local_datetime(&ndt).single();
        }
        if let Ok(nd) = chrono::NaiveDate::parse_from_str(dt, "%Y%m%d") {
            if let Some(ndt) = nd.and_hms_opt(0, 0, 0) {
                return Local.from_local_datetime(&ndt).single();
            }
        }
    }
    None
}

pub fn rescan() {
    let file_path = get_agenda_file();
    let Ok(file) = File::open(&file_path) else {
        return;
    };

    let buf = BufReader::new(file);
    let parser = IcalParser::new(buf);
    let mut events = Vec::new();

    for calendar in parser.flatten() {
        for event in calendar.events {
            let mut summary = String::new();
            let mut uid = String::new();
            let mut start_time = None;
            let mut end_time = None;

            for prop in event.properties {
                match prop.name.as_str() {
                    "UID" => {
                        if let Some(val) = prop.value {
                            uid = val;
                        }
                    }
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
                let now = Local::now();
                if end_time.unwrap_or(st) >= now || st >= now {
                    events.push(Event {
                        uid,
                        summary,
                        start_time: st,
                        end_time,
                    });
                }
            }
        }
    }

    events.sort_by_key(|e| e.start_time);
    let modified = fs::metadata(&file_path)
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::now())
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    *SNAP.lock().unwrap() = AgendaSnapshot {
        events,
        last_modified: modified,
    };
}

pub fn start() {
    ensure_agenda_file();
    rescan();

    std::thread::spawn(|| loop {
        let file_path = get_agenda_file();
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
        std::thread::sleep(std::time::Duration::from_secs(10));
    });
}

fn parse_time_str(s: &str) -> Option<(u32, u32)> {
    let s = s.trim().to_lowercase();
    let is_pm = s.ends_with("pm");
    let is_am = s.ends_with("am");
    let bare = if is_pm || is_am {
        &s[..s.len().saturating_sub(2)]
    } else {
        &s
    };

    let (h_raw, m) = if let Some((h_str, m_str)) = bare.split_once(':') {
        let h: u32 = h_str.parse().ok()?;
        let m: u32 = m_str.parse().ok()?;
        (h, m)
    } else {
        let h: u32 = bare.parse().ok()?;
        if !is_pm && !is_am {
            return None;
        }
        (h, 0)
    };

    if m >= 60 {
        return None;
    }

    let hour = if is_pm {
        if h_raw == 12 {
            12
        } else if h_raw <= 12 {
            h_raw + 12
        } else {
            return None;
        }
    } else if is_am {
        if h_raw == 12 {
            0
        } else if h_raw <= 12 {
            h_raw
        } else {
            return None;
        }
    } else {
        if h_raw > 23 {
            return None;
        }
        h_raw
    };

    Some((hour, m))
}

fn parse_relative_duration(s: &str) -> Option<chrono::Duration> {
    if let Some(num_str) = s.strip_suffix('h') {
        let h: i64 = num_str.parse().ok()?;
        Some(chrono::Duration::hours(h))
    } else if let Some(num_str) = s.strip_suffix('m') {
        let m: i64 = num_str.parse().ok()?;
        Some(chrono::Duration::minutes(m))
    } else if let Some(num_str) = s.strip_suffix('d') {
        let d: i64 = num_str.parse().ok()?;
        Some(chrono::Duration::days(d))
    } else {
        None
    }
}

pub fn parse_event_input(input: &str) -> (String, DateTime<Local>, DateTime<Local>) {
    let now = Local::now();
    let words: Vec<&str> = input.split_whitespace().collect();
    if words.is_empty() {
        let start = (now + chrono::Duration::hours(1))
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .unwrap_or(now);
        let end = start + chrono::Duration::hours(1);
        return ("Event".to_string(), start, end);
    }

    let mut target_date = now.date_naive();
    let mut date_specified = false;
    let mut parsed_time: Option<(u32, u32)> = None;
    let mut indices_to_remove = Vec::new();

    // Check for "tomorrow" or "today"
    for (i, word) in words.iter().enumerate() {
        let lower = word.to_lowercase();
        if lower == "tomorrow" || lower == "tmrw" {
            target_date = target_date + chrono::Days::new(1);
            date_specified = true;
            indices_to_remove.push(i);
            break;
        } else if lower == "today" {
            date_specified = true;
            indices_to_remove.push(i);
            break;
        }
    }

    // Check for explicit YYYY-MM-DD or MM-DD or MM/DD date
    if !date_specified {
        for (i, word) in words.iter().enumerate() {
            if indices_to_remove.contains(&i) {
                continue;
            }
            if let Ok(d) = chrono::NaiveDate::parse_from_str(word, "%Y-%m-%d") {
                target_date = d;
                date_specified = true;
                indices_to_remove.push(i);
                break;
            } else if let Ok(d) = chrono::NaiveDate::parse_from_str(
                &format!("{}-{}", now.format("%Y"), word),
                "%Y-%m-%d",
            ) {
                target_date = d;
                date_specified = true;
                indices_to_remove.push(i);
                break;
            } else if let Ok(d) = chrono::NaiveDate::parse_from_str(
                &format!("{}-{}", now.format("%Y"), word.replace('/', "-")),
                "%Y-%m-%d",
            ) {
                target_date = d;
                date_specified = true;
                indices_to_remove.push(i);
                break;
            }
        }
    }

    // Check for time: e.g. "14:00", "2:30pm", "9am", "@15:00", "@3pm", "at 14:00"
    for (i, word) in words.iter().enumerate() {
        if indices_to_remove.contains(&i) {
            continue;
        }
        let clean = word.trim_start_matches('@').to_lowercase();
        if let Some((h, m)) = parse_time_str(&clean) {
            parsed_time = Some((h, m));
            indices_to_remove.push(i);
            if i > 0 && (words[i - 1].eq_ignore_ascii_case("at") || words[i - 1] == "@") {
                indices_to_remove.push(i - 1);
            }
            break;
        }
    }

    // Check for relative duration like "in 2h" or "in 30m"
    if parsed_time.is_none() {
        for (i, word) in words.iter().enumerate() {
            if indices_to_remove.contains(&i) {
                continue;
            }
            if word.eq_ignore_ascii_case("in") && i + 1 < words.len() {
                let next = words[i + 1].to_lowercase();
                if let Some(dur) = parse_relative_duration(&next) {
                    let rel_start = now + dur;
                    target_date = rel_start.date_naive();
                    parsed_time = Some((rel_start.hour(), rel_start.minute()));
                    indices_to_remove.push(i);
                    indices_to_remove.push(i + 1);
                    date_specified = true;
                    break;
                }
            }
        }
    }

    let start_time = if let Some((h, m)) = parsed_time {
        let naive_dt = target_date
            .and_hms_opt(h, m, 0)
            .unwrap_or_else(|| now.naive_local());
        let mut local_dt = Local.from_local_datetime(&naive_dt).single().unwrap_or(now);
        // If no explicit date was specified and time has already passed today, schedule for tomorrow
        if !date_specified && local_dt < now {
            local_dt += chrono::Duration::days(1);
        }
        local_dt
    } else {
        let next_hour = now + chrono::Duration::hours(1);
        next_hour
            .with_minute(0)
            .and_then(|t| t.with_second(0))
            .unwrap_or(now)
    };

    let end_time = start_time + chrono::Duration::hours(1);

    let summary_words: Vec<&str> = words
        .iter()
        .enumerate()
        .filter(|(i, _)| !indices_to_remove.contains(i))
        .map(|(_, w)| *w)
        .collect();

    let summary = if summary_words.is_empty() {
        "Event".to_string()
    } else {
        summary_words.join(" ")
    };

    (summary, start_time, end_time)
}

pub fn add_event_to_content(content: &str, input: &str) -> (String, Event) {
    let (summary, start_time, end_time) = parse_event_input(input);
    let uid = format!(
        "vanta-{}@localhost",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis())
            .unwrap_or(0)
    );
    let dtstart = start_time.format("%Y%m%dT%H%M00").to_string();
    let dtend = end_time.format("%Y%m%dT%H%M00").to_string();

    let event_block = format!(
        "BEGIN:VEVENT\nUID:{}\nSUMMARY:{}\nDTSTART:{}\nDTEND:{}\nEND:VEVENT\n",
        uid, summary, dtstart, dtend
    );

    let new_content = if let Some(pos) = content.rfind("END:VCALENDAR") {
        let mut s = content.to_string();
        s.insert_str(pos, &event_block);
        s
    } else {
        format!(
            "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Vanta//EN\n{}{}END:VCALENDAR\n",
            content, event_block
        )
    };

    (
        new_content,
        Event {
            uid,
            summary,
            start_time,
            end_time: Some(end_time),
        },
    )
}

pub fn delete_event_from_content(
    content: &str,
    target_uid: &str,
    target_summary: &str,
    dtstart_prefix: &str,
) -> Option<String> {
    let mut before_events = Vec::new();
    let mut event_blocks: Vec<Vec<String>> = Vec::new();
    let mut after_events = Vec::new();

    let mut in_event = false;
    let mut current_block = Vec::new();
    let mut seen_any_event = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "BEGIN:VEVENT" {
            in_event = true;
            seen_any_event = true;
            current_block = vec![line.to_string()];
        } else if trimmed == "END:VEVENT" {
            current_block.push(line.to_string());
            event_blocks.push(std::mem::take(&mut current_block));
            in_event = false;
        } else if in_event {
            current_block.push(line.to_string());
        } else if !seen_any_event {
            before_events.push(line.to_string());
        } else {
            after_events.push(line.to_string());
        }
    }

    let mut matched_idx = None;
    for (i, block) in event_blocks.iter().enumerate() {
        let block_text = block.join("\n");
        if !target_uid.is_empty() && block_text.contains(&format!("UID:{}", target_uid)) {
            matched_idx = Some(i);
            break;
        }
        if !target_summary.is_empty()
            && block_text.contains(&format!("SUMMARY:{}", target_summary))
            && block_text.contains(dtstart_prefix)
        {
            matched_idx = Some(i);
            break;
        }
    }

    let idx = matched_idx?;
    event_blocks.remove(idx);

    let mut new_content = String::new();
    for line in before_events {
        new_content.push_str(&line);
        new_content.push('\n');
    }
    for block in event_blocks {
        for line in block {
            new_content.push_str(&line);
            new_content.push('\n');
        }
    }
    for line in after_events {
        new_content.push_str(&line);
        new_content.push('\n');
    }

    Some(new_content)
}

pub fn add_event(input: &str) -> bool {
    let input = input.trim();
    if input.is_empty() {
        return false;
    }
    let file_path = get_agenda_file();
    ensure_agenda_file();

    let content = fs::read_to_string(&file_path).unwrap_or_default();
    let (new_content, _) = add_event_to_content(&content, input);

    if fs::write(&file_path, new_content).is_ok() {
        rescan();
        true
    } else {
        false
    }
}

pub fn delete_event(index: usize) -> bool {
    let snap = snapshot();
    if index >= snap.events.len() {
        return false;
    }
    let target = &snap.events[index];
    let file_path = get_agenda_file();
    let Ok(content) = fs::read_to_string(&file_path) else {
        return false;
    };

    let dtstart_prefix = target.start_time.format("%Y%m%d").to_string();
    if let Some(new_content) =
        delete_event_from_content(&content, &target.uid, &target.summary, &dtstart_prefix)
    {
        let _ = fs::write(&file_path, new_content);
        rescan();
        true
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_time_str() {
        assert_eq!(parse_time_str("14:30"), Some((14, 30)));
        assert_eq!(parse_time_str("09:05"), Some((9, 5)));
        assert_eq!(parse_time_str("2pm"), Some((14, 0)));
        assert_eq!(parse_time_str("11am"), Some((11, 0)));
        assert_eq!(parse_time_str("12pm"), Some((12, 0)));
        assert_eq!(parse_time_str("12am"), Some((0, 0)));
        assert_eq!(parse_time_str("3:45pm"), Some((15, 45)));
        assert_eq!(parse_time_str("8:15am"), Some((8, 15)));
        assert_eq!(parse_time_str("25:00"), None);
        assert_eq!(parse_time_str("12:65"), None);
        assert_eq!(parse_time_str("random"), None);
    }

    #[test]
    fn test_parse_relative_duration() {
        assert_eq!(
            parse_relative_duration("2h"),
            Some(chrono::Duration::hours(2))
        );
        assert_eq!(
            parse_relative_duration("30m"),
            Some(chrono::Duration::minutes(30))
        );
        assert_eq!(
            parse_relative_duration("1d"),
            Some(chrono::Duration::days(1))
        );
        assert_eq!(parse_relative_duration("invalid"), None);
    }

    #[test]
    fn test_parse_event_input_time_and_summary() {
        let (summary, start, end) = parse_event_input("15:00 Team Standup");
        assert_eq!(summary, "Team Standup");
        assert_eq!(start.hour(), 15);
        assert_eq!(start.minute(), 0);
        assert_eq!(end.hour(), 16);
    }

    #[test]
    fn test_parse_event_input_with_at_and_pm() {
        let (summary, start, _end) = parse_event_input("Design Review @ 3:30pm");
        assert_eq!(summary, "Design Review");
        assert_eq!(start.hour(), 15);
        assert_eq!(start.minute(), 30);
    }

    #[test]
    fn test_parse_event_input_tomorrow() {
        let (summary, start, _end) = parse_event_input("tomorrow 10:00 Dentist");
        assert_eq!(summary, "Dentist");
        assert_eq!(start.hour(), 10);
        assert_eq!(start.minute(), 0);
        let expected_date = Local::now().date_naive() + chrono::Days::new(1);
        assert_eq!(start.date_naive(), expected_date);
    }

    #[test]
    fn test_parse_event_input_empty() {
        let (summary, _, _) = parse_event_input("");
        assert_eq!(summary, "Event");
    }

    #[test]
    fn test_add_and_delete_event_from_content() {
        let initial = "BEGIN:VCALENDAR\nVERSION:2.0\nPRODID:-//Vanta//EN\nEND:VCALENDAR\n";
        let (with_event, ev) = add_event_to_content(initial, "14:00 Sprint Review");
        assert_eq!(ev.summary, "Sprint Review");
        assert!(with_event.contains("SUMMARY:Sprint Review"));
        assert!(with_event.contains("BEGIN:VEVENT"));
        assert!(with_event.contains("END:VCALENDAR"));

        let dtstart_prefix = ev.start_time.format("%Y%m%d").to_string();
        let deleted = delete_event_from_content(&with_event, &ev.uid, &ev.summary, &dtstart_prefix);
        assert!(deleted.is_some());
        let final_content = deleted.unwrap();
        assert!(!final_content.contains("SUMMARY:Sprint Review"));
        assert!(final_content.contains("BEGIN:VCALENDAR"));
        assert!(final_content.contains("END:VCALENDAR"));
    }
}
