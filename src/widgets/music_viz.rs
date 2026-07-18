use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

// ── Cava backend ──
const CAVA_N_BARS: usize = 64;

static CAVA_BARS: Mutex<Vec<f32>> = Mutex::new(Vec::new());
static CAVA_RUNNING: AtomicBool = AtomicBool::new(false);

fn ensure_cava() {
    if CAVA_RUNNING.load(Ordering::Relaxed) {
        return;
    }

    // Kill other cava processes (but not ourselves or our spawned child)
    let my_pid = std::process::id().to_string();
    if let Ok(output) = Command::new("pgrep").arg("cava").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for pid in stdout.lines() {
            if pid.trim() == my_pid {
                continue;
            }
            let _ = Command::new("kill").arg(pid.trim()).output();
        }
    }
    std::thread::sleep(std::time::Duration::from_millis(200));

    let source = detect_monitor_source()
        .unwrap_or_else(|| String::from("auto"));

    // Use cava's built-in smoothing — same as what makes standalone cava look good
    let config = format!(
        "\
[general]
bars = {cava_n}
framerate = 120

[input]
method = pulse
source = {source}

[output]
method = raw
data_format = binary
bit_format = 16bit

[smoothing]
noise_reduction = 15
monstercat = 60
gravity = 30

[eq]
1 = 0.8
2 = 0.9
3 = 1.0
4 = 1.1
5 = 1.2
",
        cava_n = CAVA_N_BARS,
        source = source,
    );
    let config_path = format!("/tmp/vanta-cava-{}.conf", std::process::id());
    if std::fs::write(&config_path, &config).is_err() {
        return;
    }

    let mut child = match Command::new("cava")
        .args(["-p", &config_path])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut stdout = match child.stdout.take() {
        Some(s) => s,
        None => return,
    };

    if let Ok(mut b) = CAVA_BARS.lock() {
        *b = vec![0.0f32; CAVA_N_BARS];
    }

    CAVA_RUNNING.store(true, Ordering::Relaxed);

    std::thread::spawn(move || {
        let mut child_proc = child;
        let mut buf = vec![0u8; CAVA_N_BARS * 2]; // 16-bit per bar
        while CAVA_RUNNING.load(Ordering::Relaxed) {
            match stdout.read_exact(&mut buf) {
                Ok(()) => {
                    let values: Vec<f32> = buf
                        .chunks_exact(2)
                        .map(|c| {
                            let val = u16::from_le_bytes([c[0], c[1]]);
                            val as f32 / 65535.0
                        })
                        .collect();
                    if let Ok(mut b) = CAVA_BARS.lock() {
                        *b = values;
                    }
                }
                Err(_) => break,
            }
        }
        CAVA_RUNNING.store(false, Ordering::Relaxed);
        let _ = child_proc.kill();
        let _ = child_proc.wait();
    });
}

fn detect_monitor_source() -> Option<String> {
    let output = Command::new("pactl")
        .args(["list", "sources", "short"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[1];
        let state = parts[3];
        if name.ends_with(".monitor") && state == "RUNNING" {
            return Some(name.to_string());
        }
    }
    None
}

fn read_cava_bars() -> Vec<f32> {
    CAVA_BARS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

// ── Smooth resample: preserves peak shapes better than linear ──
fn resample_max(src: &[f32], dst_len: usize) -> Vec<f32> {
    if src.is_empty() || dst_len == 0 {
        return vec![0.0f32; dst_len];
    }
    let src_len = src.len();
    if src_len == 0 {
        return vec![0.0f32; dst_len];
    }
    (0..dst_len)
        .map(|i| {
            let start = (i as f64 * src_len as f64 / dst_len as f64) as usize;
            let end = ((i + 1) as f64 * src_len as f64 / dst_len as f64) as usize;
            let end = end.min(src_len);
            let end = end.max(start + 1);
            src[start..end]
                .iter()
                .copied()
                .fold(0.0f32, f32::max)
        })
        .collect()
}

// ── Narrow bar rendering ──
// Each column = 1 character using 8 block levels.
// Bars rendered bottom-up with a teal→white gradient.
const BLOCKS: [char; 9] = [' ', '▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

// Playerctl removed – audio detection uses cava data directly
// (Audacious, mpv, etc. all work now)
// ── Decaying peak ──
// Tracks the highest observed value and decays slowly.
// This gives the classic cava "bounce" — bars fill up and fall smoothly.
static PEAK: Mutex<f32> = Mutex::new(0.001);
// Count consecutive near-zero frames so we don't flash "no audio" on transient silence
static SILENCE_FRAMES: Mutex<u32> = Mutex::new(0);

// ── Public entry ──
pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, _tick: u64) {
    let term_cols = area.width as usize;
    let term_rows = area.height as usize;
    if term_cols < 4 || term_rows < 2 {
        return;
    }

    // Always try to keep cava alive — no playerctl gate
    ensure_cava();

    let raw = if CAVA_RUNNING.load(Ordering::Relaxed) {
        read_cava_bars()
    } else {
        let blank = Paragraph::new(Line::from(Span::styled(
            format!("{:^w$}", "  cava unavailable  ", w = term_cols),
            Style::default().fg(theme.dim),
        )))
        .style(Style::default().bg(theme.surface));
        f.render_widget(blank, area);
        return;
    };

    // Detect silence from cava data itself — any bar above 0.005 means audio
    let has_audio = raw.iter().any(|&v| v > 0.005);

    let mut silence_frames = SILENCE_FRAMES.lock().unwrap_or_else(|e| e.into_inner());
    if has_audio {
        *silence_frames = 0;
    } else {
        *silence_frames += 1;
    }
    let is_silent = *silence_frames > 8; // ~1 second of silence
    drop(silence_frames);

    if is_silent {
        let bg = Style::default().bg(theme.surface);
        let mut lines: Vec<Line> = Vec::with_capacity(term_rows);

        let msg = "(waiting for audio signal)";
        let pad = if term_cols > msg.len() { (term_cols - msg.len()) / 2 } else { 0 };
        let msg_line = format!("{}{}", " ".repeat(pad), msg);
        
        let center_row = term_rows / 2;

        for r in 0..term_rows {
            if r == center_row {
                lines.push(Line::from(Span::styled(&msg_line, Style::default().fg(theme.dim).bg(theme.surface))));
            } else {
                lines.push(Line::from(Span::styled(" ".repeat(term_cols), bg)));
            }
        }

        f.render_widget(Paragraph::new(lines).style(bg), area);
        return;
    }

    // Resample to panel width using max-pick (preserves peaks)
    let heights: Vec<f32> = resample_max(&raw, term_cols);

    // ── Decaying peak ──
    let current_max = heights.iter().copied().fold(0.0f32, f32::max);
    let mut peak = PEAK.lock().unwrap_or_else(|e| e.into_inner());

    if current_max > *peak {
        *peak = current_max;
    } else {
        *peak *= 0.98;
        if *peak < 0.001 {
            *peak = 0.001;
        }
    }
    let norm_peak = *peak;
    drop(peak);

    let display_rows = term_rows as f32;

    // ── Render bottom-up, 1 char per column ──
    let mut lines: Vec<Line> = Vec::with_capacity(term_rows);
    let bg = Style::default().bg(theme.surface);

    for display_row in (0..term_rows).rev() {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(term_cols);

        for &h in &heights {
            let bar_float = (h / norm_peak).min(1.0) * display_rows;

            let row_low = display_row as f32;
            let row_high = (display_row + 1) as f32;

            let (ch, fill_level) = if bar_float <= row_low {
                (' ', 0)
            } else if bar_float >= row_high {
                ('█', 8)
            } else {
                let frac = (bar_float - row_low) / (row_high - row_low);
                let idx = (frac * 8.0).round().clamp(1.0, 8.0) as usize;
                (BLOCKS[idx], idx)
            };

            // Brightness increases with fill level and height
            let height_frac = display_row as f32 / display_rows;
            let bright = 40 + (fill_level as f32 * 26.0) as i32
                + (height_frac * 30.0) as i32;
            let r = (74u32.saturating_add(bright as u32))
                .min(255) as u8;
            let g = (158u32.saturating_add(bright as u32))
                .min(255) as u8;
            let b = (146u32.saturating_add(bright as u32))
                .min(255) as u8;
            let color = if fill_level == 0 {
                theme.surface
            } else {
                Color::Rgb(r, g, b)
            };

            spans.push(Span::styled(ch.to_string(), Style::default().fg(color)));
        }

        lines.push(Line::from(spans));
    }

    f.render_widget(Paragraph::new(lines).style(bg), area);
}