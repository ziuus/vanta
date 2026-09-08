use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use dbus::arg::{PropMap, RefArg, Variant};
use dbus::blocking::{BlockingSender, Connection};
use dbus::Message;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;
use crate::widgets::braille_image;
use crate::widgets::meter;

const TIMEOUT: Duration = Duration::from_millis(40);
const MPRIS_PATH: &str = "/org/mpris/MediaPlayer2";
const PLAYER_IFACE: &str = "org.mpris.MediaPlayer2.Player";

#[derive(Clone, Debug, Default, PartialEq)]
pub enum Status {
    Playing,
    Paused,
    #[default]
    Stopped,
}

#[derive(Clone, Debug, Default)]
pub struct Track {
    pub player: String,
    pub status: Status,
    pub title: String,
    pub artist: String,
    pub album: String,
    pub length_us: i64,
    pub position_us: i64,
    pub volume: Option<f64>,
    pub art_url: String,
}

struct State {
    conn: Option<Connection>,
    track: Option<Track>,
    /// When `track.position_us` was read, so the bar can advance smoothly
    /// between samples while playing.
    stamp: Instant,
    art: Option<(String, u16, u16, Vec<Line<'static>>)>,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        conn: None,
        track: None,
        stamp: Instant::now(),
        art: None,
    })
});

fn prop_msg(player: &str, prop: &str) -> Message {
    Message::call_with_args(
        player,
        MPRIS_PATH,
        "org.freedesktop.DBus.Properties",
        "Get",
        (PLAYER_IFACE, prop),
    )
}

fn get<T: for<'a> dbus::arg::Get<'a>>(conn: &Connection, player: &str, prop: &str) -> Option<T> {
    let reply = conn
        .send_with_reply_and_block(prop_msg(player, prop), TIMEOUT)
        .ok()?;
    reply.get1::<Variant<T>>().map(|v| v.0)
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' && i + 2 < bytes.len() {
            if let Ok(v) = u8::from_str_radix(&s[i + 1..i + 3], 16) {
                out.push(v);
                i += 3;
                continue;
            }
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn read_track(conn: &Connection, player: &str) -> Option<Track> {
    let status = match get::<String>(conn, player, "PlaybackStatus")?.as_str() {
        "Playing" => Status::Playing,
        "Paused" => Status::Paused,
        _ => return None,
    };
    let mut t = Track {
        player: player
            .trim_start_matches("org.mpris.MediaPlayer2.")
            .split('.')
            .next()
            .unwrap_or("")
            .to_string(),
        status,
        position_us: get::<i64>(conn, player, "Position").unwrap_or(0),
        volume: get::<f64>(conn, player, "Volume"),
        ..Default::default()
    };
    if let Some(map) = get::<PropMap>(conn, player, "Metadata") {
        let s = |k: &str| map.get(k).and_then(|v| v.0.as_str()).map(str::to_string);
        t.title = s("xesam:title").unwrap_or_default();
        t.album = s("xesam:album").unwrap_or_default();
        t.art_url = s("mpris:artUrl").unwrap_or_default();
        t.length_us = map
            .get("mpris:length")
            .and_then(|v| v.0.as_i64())
            .unwrap_or(0);
        if let Some(a) = map.get("xesam:artist") {
            t.artist = a.0.as_str().map(str::to_string).unwrap_or_else(|| {
                a.0.as_iter()
                    .map(|it| {
                        it.filter_map(|x| x.as_str().map(str::to_string))
                            .collect::<Vec<_>>()
                            .join(", ")
                    })
                    .unwrap_or_default()
            });
        }
        // VLC on a bare file publishes no title; derive one from the URL.
        if t.title.is_empty() {
            if let Some(url) = s("xesam:url") {
                let name = url.rsplit('/').next().unwrap_or(&url);
                let name = percent_decode(name.split('?').next().unwrap_or(name));
                let stem = name.rsplit_once('.').map(|(a, _)| a).unwrap_or(&name);
                t.title = stem.replace(['.', '_'], " ").trim().to_string();
            }
        }
    }
    Some(t)
}

/// Poll MPRIS once. Prefers a playing player, then a paused one.
pub fn sample() {
    // Take the connection out so the D-Bus round trips happen unlocked.
    let conn = {
        let mut st = STATE.lock().unwrap();
        match st.conn.take() {
            Some(c) => Some(c),
            None => Connection::new_session().ok(),
        }
    };
    let Some(conn_owned) = conn else {
        STATE.lock().unwrap().track = None;
        return;
    };
    let conn = &conn_owned;
    let names = Message::call_with_args(
        "org.freedesktop.DBus",
        "/",
        "org.freedesktop.DBus",
        "ListNames",
        (),
    );
    let players: Vec<String> = match conn.send_with_reply_and_block(names, TIMEOUT) {
        Ok(r) => r
            .read_all::<(Vec<String>,)>()
            .map(|(v,)| v)
            .unwrap_or_default()
            .into_iter()
            .filter(|n| n.starts_with("org.mpris.MediaPlayer2.") && !n.ends_with(".playerctld"))
            .collect(),
        Err(_) => {
            // Bus went away (session restart); reconnect next tick.
            STATE.lock().unwrap().track = None;
            return;
        }
    };
    let mut best: Option<Track> = None;
    for p in players {
        if let Some(t) = read_track(conn, &p) {
            let better = match &best {
                None => true,
                Some(b) => t.status == Status::Playing && b.status != Status::Playing,
            };
            if better {
                let playing = t.status == Status::Playing;
                best = Some(t);
                if playing {
                    break;
                }
            }
        }
    }
    let mut st = STATE.lock().unwrap();
    st.track = best;
    st.stamp = Instant::now();
    st.conn = Some(conn_owned);
}

/// Playback control on the currently displayed player. Fire-and-forget.
pub fn control(action: Action) {
    let st = STATE.lock().unwrap();
    let (Some(conn), Some(track)) = (st.conn.as_ref(), st.track.as_ref()) else {
        return;
    };
    let player = format!("org.mpris.MediaPlayer2.{}", track.player);
    // The full bus name may carry an instance suffix; fall back to a scan.
    let target = full_name(conn, &track.player).unwrap_or(player);
    match action {
        Action::PlayPause | Action::Next | Action::Previous => {
            let method = match action {
                Action::PlayPause => "PlayPause",
                Action::Next => "Next",
                _ => "Previous",
            };
            let msg = Message::new_method_call(&target, MPRIS_PATH, PLAYER_IFACE, method)
                .expect("static dbus names are valid");
            let _ = conn.send_with_reply_and_block(msg, TIMEOUT);
        }
        Action::VolumeUp | Action::VolumeDown => {
            let cur = track.volume.unwrap_or(1.0);
            let delta = if action == Action::VolumeUp {
                0.05
            } else {
                -0.05
            };
            let v = (cur + delta).clamp(0.0, 1.0);
            let msg = Message::call_with_args(
                &target,
                MPRIS_PATH,
                "org.freedesktop.DBus.Properties",
                "Set",
                (PLAYER_IFACE, "Volume", Variant(v)),
            );
            let _ = conn.send_with_reply_and_block(msg, TIMEOUT);
        }
    }
}

fn full_name(conn: &Connection, short: &str) -> Option<String> {
    let names = Message::call_with_args(
        "org.freedesktop.DBus",
        "/",
        "org.freedesktop.DBus",
        "ListNames",
        (),
    );
    let r = conn.send_with_reply_and_block(names, TIMEOUT).ok()?;
    let (v,): (Vec<String>,) = r.read_all().ok()?;
    let prefix = format!("org.mpris.MediaPlayer2.{}", short);
    v.into_iter().find(|n| n.starts_with(&prefix))
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    PlayPause,
    Next,
    Previous,
    VolumeUp,
    VolumeDown,
}

fn fmt_dur(us: i64) -> String {
    let s = (us.max(0) / 1_000_000) as u64;
    if s >= 3600 {
        format!("{}:{:02}:{:02}", s / 3600, (s % 3600) / 60, s % 60)
    } else {
        format!("{}:{:02}", s / 60, s % 60)
    }
}

fn art_lines(st: &mut State, url: &str, w: u16, h: u16) -> Option<Vec<Line<'static>>> {
    if let Some((u, cw, ch, lines)) = &st.art {
        if u == url && *cw == w && *ch == h {
            return (!lines.is_empty()).then(|| lines.clone());
        }
    }
    // Cache misses too (empty vec) so an undecodable file isn't retried per frame.
    let lines = url
        .strip_prefix("file://")
        .and_then(|p| braille_image::render_path(&percent_decode(p), w, h))
        .unwrap_or_default();
    st.art = Some((url.to_string(), w, h, lines.clone()));
    (!lines.is_empty()).then_some(lines)
}

pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 1 || area.width < 16 {
        return;
    }
    let mut st = STATE.lock().unwrap();
    let Some(track) = st.track.clone() else {
        let y = area.y + area.height.saturating_sub(1) / 2;
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "nothing playing",
                Style::default().fg(theme.dim),
            )))
            .alignment(ratatui::layout::Alignment::Center),
            Rect::new(area.x, y, area.width, 1),
        );
        return;
    };

    // Album art on the left when there is room for it.
    let art_w: u16 = if area.height >= 4 && area.width >= 48 {
        (area.height * 2).min(16)
    } else {
        0
    };
    let art = if art_w > 0 && !track.art_url.is_empty() {
        art_lines(&mut st, &track.art_url, art_w, area.height)
    } else {
        None
    };
    drop(st);

    let text_area = match &art {
        Some(lines) => {
            let h = (lines.len() as u16).min(area.height);
            let top = area.height.saturating_sub(h) / 2;
            f.render_widget(
                Paragraph::new(lines.clone()),
                Rect::new(area.x, area.y + top, art_w, h),
            );
            Rect::new(
                area.x + art_w + 2,
                area.y,
                area.width.saturating_sub(art_w + 2),
                area.height,
            )
        }
        None => area,
    };

    // Smooth position: advance by wall time since the last sample while playing.
    let elapsed = STATE.lock().unwrap().stamp.elapsed().as_micros() as i64;
    let pos = if track.status == Status::Playing {
        (track.position_us + elapsed).min(track.length_us.max(track.position_us))
    } else {
        track.position_us
    };
    let progress = if track.length_us > 0 {
        (pos as f64 / track.length_us as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let icon = match track.status {
        Status::Playing => "▶",
        Status::Paused => "⏸",
        Status::Stopped => "■",
    };
    let w = text_area.width as usize;
    let mut lines: Vec<Line> = vec![Line::from(vec![
        Span::styled(format!("{} ", icon), Style::default().fg(theme.accent)),
        Span::styled(
            meter::ellipsize(&track.title, w.saturating_sub(2)),
            Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
        ),
    ])];
    let mut sub = track.artist.clone();
    if !track.album.is_empty()
        && text_area.height >= 3
        && sub.chars().count() + track.album.chars().count() + 3 < w
    {
        if !sub.is_empty() {
            sub.push_str(" · ");
        }
        sub.push_str(&track.album);
    }
    if !sub.is_empty() {
        lines.push(Line::from(Span::styled(
            format!("  {}", meter::ellipsize(&sub, w.saturating_sub(2))),
            Style::default().fg(theme.secondary),
        )));
    }

    // Progress row: time · bar · time · player · volume
    let pos_s = fmt_dur(pos);
    let len_s = fmt_dur(track.length_us);
    let vol = track
        .volume
        .map(|v| format!("  vol {:>3.0}%", v * 100.0))
        .unwrap_or_default();
    let tail = format!(" {}  {}{}", len_s, track.player, vol);
    let head = format!("{} ", pos_s);
    let bar_w = w.saturating_sub(head.len() + tail.len()).max(4);
    let (on, off) = meter::track(progress, bar_w);
    let progress_line = Line::from(vec![
        Span::styled(head, Style::default().fg(theme.dim)),
        Span::styled(on, Style::default().fg(theme.accent)),
        Span::styled(off, Style::default().fg(theme.surface)),
        Span::styled(tail, Style::default().fg(theme.dim)),
    ]);

    let text_h = (lines.len() as u16 + 2).min(text_area.height); // + blank + progress
    let top = text_area.height.saturating_sub(text_h) / 2;
    let block = Rect::new(text_area.x, text_area.y + top, text_area.width, text_h);
    let rows = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(block);
    f.render_widget(Paragraph::new(lines), rows[0]);
    if text_h >= 2 {
        f.render_widget(Paragraph::new(progress_line), rows[1]);
    }
}
