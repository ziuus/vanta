use std::time::Duration;

use dbus::arg::Variant;
use dbus::blocking::{BlockingSender, Connection};
use dbus::Message;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph};
use ratatui::symbols::line;
use ratatui::Frame;

use crate::app;

/// Full metadata extracted from an MPRIS player.
#[derive(Debug, Default)]
struct TrackInfo {
    title: String,
    artist: String,
    length_usec: i64,
}

/// Read metadata dict from a Properties.Get reply using inline Iter walking.
/// Properties.Get for Metadata returns Variant(a{sv}).
fn read_metadata(conn: &Connection, player: &str, timeout: Duration) -> TrackInfo {
    let msg = Message::call_with_args(
        player,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
        "Get",
        ("org.mpris.MediaPlayer2.Player", "Metadata"),
    );
    let reply = match conn.send_with_reply_and_block(msg, timeout) {
        Ok(r) => r,
        Err(_) => return TrackInfo::default(),
    };

    let mut info = TrackInfo::default();
    let mut outer = reply.iter_init();

    // Skip the outer Variant to reach the dict (a{sv})
    if let Some(mut dict) = outer.recurse(dbus::arg::ArgType::Variant) {
        // Iterate dictionary entries
        while let Some(mut entry) = dict.recurse(dbus::arg::ArgType::DictEntry) {
            let key: String = match entry.read() {
                Ok(k) => k,
                Err(_) => continue,
            };

            // Value is wrapped in Variant
            if let Some(mut val) = entry.recurse(dbus::arg::ArgType::Variant) {
                match key.as_str() {
                    "xesam:title" => {
                        if let Ok(t) = val.read::<String>() {
                            info.title = t;
                        } else if let Some(mut arr) = val.recurse(dbus::arg::ArgType::Array) {
                            if let Ok(t) = arr.read::<String>() {
                                info.title = t;
                            }
                        }
                    }
                    "xesam:artist" => {
                        if let Ok(s) = val.read::<String>() {
                            info.artist = s;
                        } else if let Some(mut arr) = val.recurse(dbus::arg::ArgType::Array) {
                            let mut v = Vec::new();
                            while let Ok(s) = arr.read::<String>() {
                                v.push(s);
                            }
                            if !v.is_empty() {
                                info.artist = v.join(", ");
                            }
                        }
                    }
                    "mpris:length" => {
                        if let Ok(len) = val.read::<i64>() {
                            info.length_usec = len;
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    info
}

/// Get a string property via org.freedesktop.DBus.Properties.Get.
fn get_prop(conn: &Connection, player: &str, prop: &str, timeout: Duration) -> Option<String> {
    let msg = Message::call_with_args(
        player,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
        "Get",
        ("org.mpris.MediaPlayer2.Player", prop),
    );
    let reply = conn.send_with_reply_and_block(msg, timeout).ok()?;
    let v: Variant<String> = reply.get1()?;
    Some(v.0)
}

/// Get an i64 property.
fn get_prop_i64(conn: &Connection, player: &str, prop: &str, timeout: Duration) -> Option<i64> {
    let msg = Message::call_with_args(
        player,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
        "Get",
        ("org.mpris.MediaPlayer2.Player", prop),
    );
    let reply = conn.send_with_reply_and_block(msg, timeout).ok()?;
    let v: Variant<i64> = reply.get1()?;
    Some(v.0)
}

/// Get a double property.
fn get_prop_f64(conn: &Connection, player: &str, prop: &str, timeout: Duration) -> Option<f64> {
    let msg = Message::call_with_args(
        player,
        "/org/mpris/MediaPlayer2",
        "org.freedesktop.DBus.Properties",
        "Get",
        ("org.mpris.MediaPlayer2.Player", prop),
    );
    let reply = conn.send_with_reply_and_block(msg, timeout).ok()?;
    let v: Variant<f64> = reply.get1()?;
    Some(v.0)
}

fn fmt_dur(usec: i64) -> String {
    if usec <= 0 {
        return "0:00".into();
    }
    let secs = usec / 1_000_000;
    let m = secs / 60;
    let s = secs % 60;
    format!("{}:{:02}", m, s)
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let conn = Connection::new_session().ok();
    let timeout = Duration::from_secs(5);

    if let Some(conn) = conn {
        // List MPRIS players on the session bus
        let list_msg = Message::call_with_args(
            "org.freedesktop.DBus",
            "/",
            "org.freedesktop.DBus",
            "ListNames",
            (),
        );
        let list_reply = conn.send_with_reply_and_block(list_msg, timeout).ok();
        let player_name = list_reply.and_then(|r| {
            let (names,): (Vec<String>,) = r.read_all().ok()?;
            names
                .iter()
                .find(|n| n.starts_with("org.mpris.MediaPlayer2."))
                .cloned()
        });

        if let Some(ref player) = player_name {
            let status = get_prop(&conn, player, "PlaybackStatus", timeout)
                .unwrap_or_else(|| "Stopped".into());

            if status != "Stopped" {
                let track = read_metadata(&conn, player, timeout);
                let position_usec = get_prop_i64(&conn, player, "Position", timeout).unwrap_or(0);
                let _volume = get_prop_f64(&conn, player, "Volume", timeout).unwrap_or(0.0);

                let icon = if status == "Playing" { "▶" } else { "⏸" };

                let inner_w = area.width.saturating_sub(2) as usize;
                let title = if track.title.len() > inner_w.saturating_sub(5) {
                    let mut t = track.title;
                    t.truncate(inner_w.saturating_sub(8));
                    t
                } else {
                    track.title.clone()
                };
                let display_line = if track.artist.is_empty() {
                    format!("{} {}", icon, title)
                } else {
                    format!("{} {} — {}", icon, track.artist, title)
                };

                let pos_str = fmt_dur(position_usec);
                let len_str = fmt_dur(track.length_usec);
                let progress_pct = if track.length_usec > 0 {
                    ((position_usec as f64 / track.length_usec as f64) * 100.0) as u16
                } else {
                    0
                };

                // 2 lines: info + progress gauge
                let chunks = ratatui::layout::Layout::vertical([
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Length(1),
                ])
                .split(area);

                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        &display_line,
                        Style::default().fg(theme.text),
                    )))
                    .style(Style::default().bg(theme.bg)),
                    chunks[0],
                );

                let gauge = LineGauge::default()
                    .filled_style(Style::default().fg(theme.accent).bg(theme.surface))
                    .ratio((progress_pct as f64 / 100.0).clamp(0.0, 1.0))
                    .label(format!("{} / {}", pos_str, len_str))
                    .line_set(line::THICK);
                f.render_widget(gauge, chunks[1]);
                return;
            }
        }
    }

    // No player active
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            " (no media)",
            Style::default().fg(theme.dim),
        )))
        .style(Style::default().bg(theme.bg)),
        area,
    );
}
