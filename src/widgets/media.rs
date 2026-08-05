use std::time::Duration;

use dbus::arg::{PropMap, RefArg, Variant};
use dbus::blocking::{BlockingSender, Connection};
use dbus::Message;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::symbols::line;
use ratatui::text::{Line, Span};
use ratatui::widgets::{LineGauge, Paragraph};
use ratatui::Frame;

use crate::app;

/// Full metadata extracted from an MPRIS player.
#[derive(Debug, Default)]
struct TrackInfo {
    title: String,
    artist: String,
    length_usec: i64,
    art_url: String,
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

    if let Some(variant) = reply.get1::<Variant<PropMap>>() {
        let map = variant.0;

        if let Some(title) = map.get("xesam:title") {
            if let Some(t) = title.0.as_str() {
                info.title = t.to_string();
            }
        }

        if let Some(artist) = map.get("xesam:artist") {
            if let Some(a) = artist.0.as_str() {
                info.artist = a.to_string();
            } else if let Some(arr) = artist.0.as_iter() {
                let mut v = Vec::new();
                for item in arr {
                    if let Some(s) = item.as_str() {
                        v.push(s.to_string());
                    }
                }
                if !v.is_empty() {
                    info.artist = v.join(", ");
                }
            }
        }

        if let Some(length) = map.get("mpris:length") {
            if let Some(l) = length.0.as_i64() {
                info.length_usec = l;
            }
        }

        if let Some(art) = map.get("mpris:artUrl") {
            if let Some(a) = art.0.as_str() {
                info.art_url = a.to_string();
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

/// MPRIS `mpris:artUrl` is usually a `file://` URI. Return a readable local
/// path, or `None` for remote/unsupported schemes.
fn local_art_path(url: &str) -> Option<String> {
    let path = url.strip_prefix("file://")?;
    // Percent-decode the few characters that actually show up in music paths.
    let decoded = path
        .replace("%20", " ")
        .replace("%28", "(")
        .replace("%29", ")")
        .replace("%27", "'");
    std::path::Path::new(&decoded).exists().then_some(decoded)
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let conn = Connection::new_session().ok();
    let timeout = Duration::from_millis(50);

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
        let player_name: Option<String> = list_reply.and_then(|r| {
            let (names,): (Vec<String>,) = r.read_all().ok()?;
            names
                .into_iter()
                .find(|n| n.starts_with("org.mpris.MediaPlayer2."))
        });

        if let Some(ref player) = player_name {
            let status = get_prop(&conn, player, "PlaybackStatus", timeout)
                .unwrap_or_else(|| "Stopped".into());

            if status != "Stopped" {
                let track = read_metadata(&conn, player, timeout);
                let position_usec = get_prop_i64(&conn, player, "Position", timeout).unwrap_or(0);
                let _volume = get_prop_f64(&conn, player, "Volume", timeout).unwrap_or(0.0);

                let icon = if status == "Playing" { "▶" } else { "⏸" };

                let inner_w = area.width.saturating_sub(20) as usize;
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

                // Album art (braille) on the left when available, text on the right.
                let art_w: u16 = if area.height >= 6 { 16 } else { 0 };
                let art_lines = if art_w > 0 {
                    local_art_path(&track.art_url)
                        .and_then(|p| {
                            crate::widgets::braille_image::render_path(
                                &p,
                                art_w,
                                area.height.saturating_sub(1),
                            )
                        })
                } else {
                    None
                };

                let text_area = match &art_lines {
                    Some(lines) => {
                        let h = lines.len() as u16;
                        let top = area.height.saturating_sub(h) / 2;
                        f.render_widget(
                            Paragraph::new(lines.clone()),
                            Rect::new(area.x, area.y + top, art_w, h),
                        );
                        Rect::new(
                            area.x + art_w + 1,
                            area.y,
                            area.width.saturating_sub(art_w + 1),
                            area.height,
                        )
                    }
                    None => area,
                };

                // 2 lines: info + progress gauge (vertically centered)
                let top = text_area.height.saturating_sub(2) / 2;
                let content = Rect::new(text_area.x, text_area.y + top, text_area.width, 2);
                let chunks = ratatui::layout::Layout::vertical([
                    ratatui::layout::Constraint::Length(1),
                    ratatui::layout::Constraint::Length(1),
                ])
                .split(content);

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

    // No player active — quiet placeholder. (The dashboard has its own
    // full-width visualizer strip, so we don't duplicate it here.)
    let top = area.height.saturating_sub(1) / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            "  no media playing",
            Style::default().fg(theme.dim),
        )))
        .alignment(ratatui::layout::Alignment::Center)
        .style(Style::default().bg(theme.bg)),
        Rect::new(area.x, area.y + top, area.width, 1),
    );
}
