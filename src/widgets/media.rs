use std::process::Command;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Gauge, Paragraph};
use ratatui::Frame;

use crate::app;

/// Read current media info via playerctl. Returns None if no player is active.
fn get_media_info() -> Option<MediaInfo> {
    // Quick check — are any players running?
    let status_out = Command::new("playerctl")
        .arg("status")
        .output()
        .ok()?;
    if !status_out.status.success() {
        return None;
    }
    let status = String::from_utf8_lossy(&status_out.stdout)
        .trim()
        .to_string();
    if status.is_empty() {
        return None;
    }

    // Get metadata formatted string
    let meta_out = Command::new("playerctl")
        .args(["metadata", "--format", "{{playerName}}|{{artist}}|{{title}}|{{mpris:length}}|{{position}}"])
        .output()
        .ok()?;
    if !meta_out.status.success() {
        return None;
    }
    let meta_str = String::from_utf8_lossy(&meta_out.stdout);
    let parts: Vec<&str> = meta_str.trim().split('|').collect();

    let player = parts.first().unwrap_or(&"").to_string();
    let artist = parts.get(1).unwrap_or(&"").to_string();
    let title = parts.get(2).unwrap_or(&"").to_string();
    let length_usec: u64 = parts.get(3).unwrap_or(&"").parse().unwrap_or(0);
    let position_usec: u64 = parts.get(4).unwrap_or(&"").parse().unwrap_or(0);

    let position_sec = position_usec / 1_000_000;
    let length_sec = if length_usec > 0 {
        length_usec / 1_000_000
    } else {
        1 // avoid division by zero
    };
    let progress = if length_sec > 0 {
        (position_sec as f64 / length_sec as f64).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let artist_clean = if artist.is_empty() || artist == "(null)" {
        String::new()
    } else {
        artist
    };

    Some(MediaInfo {
        player,
        artist: artist_clean,
        title: if title.is_empty() || title == "(null)" {
            String::new()
        } else {
            title
        },
        status,
        position_sec,
        length_sec,
        progress,
    })
}

#[derive(Debug)]
struct MediaInfo {
    player: String,
    artist: String,
    title: String,
    status: String,
    position_sec: u64,
    length_sec: u64,
    progress: f64,
}

fn fmt_dur(secs: u64) -> String {
    let m = secs / 60;
    let s = secs % 60;
    format!("{}:{:02}", m, s)
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, demo: bool) {
    let info = if demo {
        Some(MediaInfo {
            player: "spotify".to_string(),
            artist: "Tame Impala".to_string(),
            title: "Let It Happen".to_string(),
            status: "Playing".to_string(),
            position_sec: 203,
            length_sec: 470,
            progress: 203.0 / 470.0,
        })
    } else {
        get_media_info()
    };

    let mut lines: Vec<Line> = Vec::new();

    if let Some(info) = info {
        // Play/pause indicator
        let play_icon = match info.status.as_str() {
            "Playing" => "▶",
            "Paused" => "⏸",
            _ => "⏹",
        };
        let status_color = match info.status.as_str() {
            "Playing" => theme.green,
            "Paused" => theme.yellow,
            _ => theme.dim,
        };

        lines.push(Line::from(vec![
            Span::styled(
                format!(" {} ", play_icon),
                Style::default().fg(status_color),
            ),
            Span::styled(
                if info.title.is_empty() {
                    "(no track)".to_string()
                } else if !info.artist.is_empty() {
                    format!("{} — {}", info.artist, info.title)
                } else {
                    info.title.clone()
                },
                Style::default().fg(theme.text),
            ),
        ]));

        // Progress bar
        let elapsed = fmt_dur(info.position_sec);
        let total = fmt_dur(info.length_sec);
        let gauge = Gauge::default()
            .gauge_style(Style::default().fg(theme.green).bg(theme.surface))
            .ratio(info.progress as f64)
            .label(format!(" {}/{} ", elapsed, total));
        f.render_widget(gauge, Rect::new(area.x + 1, area.y + 1, area.width.saturating_sub(2), 1));

        // Hint line with player name and controls
        lines.push(Line::from(vec![
            Span::styled(
                format!(" [{}]", info.player),
                Style::default().fg(theme.dim),
            ),
            Span::styled("  [Space] ⏯  [N] ⏭  [P] ⏮", Style::default().fg(theme.dim)),
        ]));
    } else {
        lines.push(Line::from(Span::styled(
            " (no media player active)",
            Style::default().fg(theme.dim),
        )));
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        area,
    );
}
