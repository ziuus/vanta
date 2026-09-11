use std::io::Read;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Mutex;

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::theme::Theme;

// ── Cava backend ──
const CAVA_N_BARS: usize = 64;

static CAVA_BARS: Mutex<Vec<f32>> = Mutex::new(Vec::new());
static CAVA_RUNNING: AtomicBool = AtomicBool::new(false);
static CAVA_CHILD: Mutex<Option<u32>> = Mutex::new(None);
/// Last spawn attempt, so a missing `cava` binary is retried every few
/// seconds instead of on every frame (the old path slept 100ms per frame).
static LAST_SPAWN: Mutex<Option<std::time::Instant>> = Mutex::new(None);

// ── Visualizer style ──
// 0 = bars (bottom-up), 1 = mirror (center-out), 2 = wave (midline),
// 3 = peaks (bars with falling caps).
const STYLE_COUNT: usize = 4;
/// Per-column peak hold for the "peaks" style, in normalised 0..1 units.
static PEAK_CAPS: Mutex<Vec<f32>> = Mutex::new(Vec::new());
static VIZ_STYLE: AtomicUsize = AtomicUsize::new(0);

/// Cycle to the next visualizer style. Bound to `v` globally.
pub fn cycle_style() {
    let next = (VIZ_STYLE.load(Ordering::Relaxed) + 1) % STYLE_COUNT;
    VIZ_STYLE.store(next, Ordering::Relaxed);
}

/// Select a style by config name; unknown names fall back to bars.
pub fn set_style(name: &str) {
    let idx = match name {
        "mirror" => 1,
        "wave" => 2,
        "peaks" => 3,
        _ => 0,
    };
    VIZ_STYLE.store(idx, Ordering::Relaxed);
}

fn ensure_cava() {
    if CAVA_RUNNING.load(Ordering::Relaxed) {
        return;
    }
    {
        let mut last = LAST_SPAWN.lock().unwrap_or_else(|e| e.into_inner());
        if last.is_some_and(|t| t.elapsed() < std::time::Duration::from_secs(5)) {
            return;
        }
        *last = Some(std::time::Instant::now());
    }

    // Reap our own previous cava child (if any) so we don't leak processes.
    // Never touch cava instances we didn't spawn — the user may run their own.
    if let Ok(mut child) = CAVA_CHILD.lock() {
        if let Some(pid) = child.take() {
            let _ = Command::new("kill").arg(pid.to_string()).spawn();
        }
    }

    let source = detect_monitor_source().unwrap_or_else(|| String::from("auto"));

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

    if let Ok(mut saved) = CAVA_CHILD.lock() {
        *saved = Some(child.id());
    }

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
                        .as_chunks::<2>()
                        .0
                        .iter()
                        .map(|c| {
                            let val = u16::from_le_bytes([c[0], c[1]]);
                            val as f32 / 65535.0
                        })
                        .collect();
                    let has_audio = values.iter().any(|&v| v > 0.005);
                    if let Ok(mut b) = CAVA_BARS.lock() {
                        *b = values;
                    }
                    // Track silence in the always-running reader thread so
                    // is_active() stays correct even when the widget isn't
                    // being rendered (e.g. overview collapsed it on silence).
                    if let Ok(mut sf) = SILENCE_FRAMES.lock() {
                        if has_audio {
                            *sf = 0;
                        } else {
                            *sf = sf.saturating_add(1);
                        }
                    }
                }
                Err(_) => break,
            }
        }
        CAVA_RUNNING.store(false, Ordering::Relaxed);
        if let Ok(mut saved) = CAVA_CHILD.lock() {
            if *saved == Some(child_proc.id()) {
                *saved = None;
            }
        }
        let _ = child_proc.kill();
        let _ = child_proc.wait();
    });
}

/// Terminate the cava we spawned. Called on shutdown.
pub fn shutdown() {
    CAVA_RUNNING.store(false, Ordering::Relaxed);
    if let Ok(mut child) = CAVA_CHILD.lock() {
        if let Some(pid) = child.take() {
            let _ = Command::new("kill").arg(pid.to_string()).status();
        }
    }
    let _ = std::fs::remove_file(format!("/tmp/vanta-cava-{}.conf", std::process::id()));
}

/// Human label for the active style, for the status bar.
pub fn style_name() -> &'static str {
    match VIZ_STYLE.load(Ordering::Relaxed) % STYLE_COUNT {
        1 => "mirror",
        2 => "wave",
        3 => "peaks",
        _ => "bars",
    }
}

fn detect_monitor_source() -> Option<String> {
    let output = Command::new("pactl")
        .args(["list", "sources", "short"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Prefer a monitor that's actively RUNNING/IDLE (audio flowing right now).
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[1];
        let state = parts[3];
        if name.ends_with(".monitor") && (state == "RUNNING" || state == "IDLE") {
            return Some(name.to_string());
        }
    }

    // Nothing active (all SUSPENDED, e.g. no audio yet). Bind to the default
    // sink's monitor so cava still connects on PipeWire — bare "auto" often
    // fails to bind and leaves the bars dead.
    if let Ok(out) = Command::new("pactl").args(["get-default-sink"]).output() {
        let sink = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !sink.is_empty() {
            let monitor = format!("{sink}.monitor");
            // Confirm the monitor exists in the source list before using it.
            if stdout.lines().any(|l| l.contains(&monitor)) {
                return Some(monitor);
            }
        }
    }

    // Last resort: any .monitor at all, regardless of state.
    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() >= 2 && parts[1].ends_with(".monitor") {
            return Some(parts[1].to_string());
        }
    }

    None
}

fn read_cava_bars() -> Vec<f32> {
    CAVA_BARS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

// ── Spatial resample: max-fold preserves peak shapes when a narrow panel has
// fewer columns than cava has bars. cava's bars are already neighbour-smoothed
// (monstercat), so the fold stays visually continuous. ──
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
            src[start..end].iter().copied().fold(0.0f32, f32::max)
        })
        .collect()
}

/// Per-column smoothed heights, eased toward the latest frame every render.
static SMOOTHED: Mutex<Vec<f32>> = Mutex::new(Vec::new());

/// One easing step over a column state, in place. Rise fast, fall slow.
fn ease_step(state: &mut [f32], target: &[f32]) {
    for (cur, &t) in state.iter_mut().zip(target) {
        let alpha = if t > *cur { 0.5 } else { 0.15 };
        *cur += (t - *cur) * alpha;
    }
}

/// Ease each column toward its target: rise fast, fall slow. cava eases every
/// bar at its own 120fps; vanta only samples it at ~30fps, so without this the
/// bars snap between frames. The asymmetry is the classic cava "liquid" drip —
/// a spike jumps up, then drains.
///
/// ponytail: alphas are tuned for the ~30fps render loop; if the frame rate
/// becomes configurable, scale them by dt.
fn smooth_temporal(target: &[f32]) -> Vec<f32> {
    let mut s = SMOOTHED.lock().unwrap_or_else(|e| e.into_inner());
    if s.len() != target.len() {
        // First frame or a resize — snap, don't ease from stale state.
        *s = target.to_vec();
        return s.clone();
    }
    ease_step(&mut s, target);
    s.clone()
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
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, _tick: u64) {
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
        // No cava: fall back to the idle wave so the panel never looks dead,
        // and say why in the corner.
        let heights = idle_wave(term_cols, _tick);
        let lines = draw_bars(&heights, 1.0, term_cols, term_rows, theme, true);
        f.render_widget(Paragraph::new(lines), area);
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                " install cava for live audio ",
                Style::default().fg(theme.dim),
            )))
            .alignment(ratatui::layout::Alignment::Right),
            Rect::new(area.x, area.y, area.width, 1),
        );
        return;
    };

    // Silence is tracked by the reader thread; here we only read the counter.
    let is_silent = SILENCE_FRAMES.lock().map_or(true, |sf| *sf > 8);

    // When silent, synthesize a gentle "breathing" wave so the widget stays
    // alive-looking instead of showing dead/blank bars.
    let target: Vec<f32> = if is_silent {
        idle_wave(term_cols, _tick)
    } else {
        resample_max(&raw, term_cols)
    };
    // Ease toward the target so bars flow between frames instead of snapping.
    // Applied across the silent boundary too, so fading in/out glides.
    let heights = smooth_temporal(&target);

    // ── Decaying peak (normalization) ──
    let norm_peak = if is_silent {
        1.0 // idle wave is already 0..1
    } else {
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
        *peak
    };

    let style = VIZ_STYLE.load(Ordering::Relaxed) % STYLE_COUNT;
    let dim = is_silent;

    let lines = match style {
        1 => draw_mirror(&heights, norm_peak, term_cols, term_rows, theme, dim),
        2 => draw_wave(&heights, norm_peak, term_cols, term_rows, theme, dim),
        3 => draw_peaks(&heights, norm_peak, term_cols, term_rows, theme, dim),
        _ => draw_bars(&heights, norm_peak, term_cols, term_rows, theme, dim),
    };

    f.render_widget(Paragraph::new(lines), area);
}

/// A slow sine "breathing" pattern for the idle state, normalized 0..1.
fn idle_wave(cols: usize, tick: u64) -> Vec<f32> {
    let t = tick as f32 * 0.05;
    (0..cols)
        .map(|c| {
            let x = c as f32 * 0.25;
            // Two summed sines for a soft, non-repetitive breathing look.
            let v = (x + t).sin() * 0.5 + (x * 0.5 - t * 0.7).sin() * 0.5;
            (v * 0.5 + 0.5) * 0.35 // keep it low/gentle (max ~35% height)
        })
        .collect()
}

fn bar_color(theme: &Theme, height_frac: f32, filled: bool, dim: bool) -> ratatui::style::Color {
    if !filled {
        theme.bg
    } else if dim {
        theme.dim
    } else if height_frac > 0.6 {
        theme.secondary
    } else {
        theme.accent
    }
}

/// Classic bottom-up bars, one char per column.
fn draw_bars(
    heights: &[f32],
    norm_peak: f32,
    cols: usize,
    rows: usize,
    theme: &Theme,
    dim: bool,
) -> Vec<Line<'static>> {
    let display_rows = rows as f32;
    let mut lines: Vec<Line> = Vec::with_capacity(rows);
    for display_row in (0..rows).rev() {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(cols);
        for &h in heights {
            let bar_float = (h / norm_peak).min(1.0) * display_rows;
            let row_low = display_row as f32;
            let (ch, filled) = if bar_float <= row_low {
                (' ', false)
            } else if bar_float >= row_low + 1.0 {
                ('█', true)
            } else {
                let frac = bar_float - row_low;
                (BLOCKS[(frac * 8.0).round().clamp(1.0, 8.0) as usize], true)
            };
            let height_frac = display_row as f32 / display_rows;
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(bar_color(theme, height_frac, filled, dim)),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Mirrored bars growing out from the horizontal centre line.
fn draw_mirror(
    heights: &[f32],
    norm_peak: f32,
    cols: usize,
    rows: usize,
    theme: &Theme,
    dim: bool,
) -> Vec<Line<'static>> {
    let half = (rows / 2).max(1) as f32;
    let mid = rows / 2;
    let mut lines: Vec<Line> = Vec::with_capacity(rows);
    for row in 0..rows {
        // Distance from centre, in rows.
        let dist = if row >= mid {
            (row - mid) as f32
        } else {
            (mid - row) as f32 - 1.0
        };
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(cols);
        for &h in heights {
            let amp = (h / norm_peak).min(1.0) * half;
            let filled = amp > dist;
            let ch = if filled { '█' } else { ' ' };
            let height_frac = dist / half;
            spans.push(Span::styled(
                ch.to_string(),
                Style::default().fg(bar_color(theme, height_frac, filled, dim)),
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// A single-row waveform tracing the amplitude across a midline.
fn draw_wave(
    heights: &[f32],
    norm_peak: f32,
    cols: usize,
    rows: usize,
    theme: &Theme,
    dim: bool,
) -> Vec<Line<'static>> {
    let mid = rows / 2;
    // Row index (from top) the wave sits at for each column.
    let wave_rows: Vec<usize> = heights
        .iter()
        .map(|&h| {
            let amp = (h / norm_peak).min(1.0);
            let offset = (amp * (rows as f32 / 2.0)) as usize;
            mid.saturating_sub(offset)
        })
        .collect();
    let color = if dim { theme.dim } else { theme.accent };
    let mut lines: Vec<Line> = Vec::with_capacity(rows);
    for row in 0..rows {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(cols);
        for &wr in wave_rows.iter().take(cols) {
            let ch = if row == wr {
                '━'
            } else if row > wr && row <= mid {
                '│' // faint fill down to the midline
            } else {
                ' '
            };
            let c = if ch == '│' { theme.dim } else { color };
            spans.push(Span::styled(ch.to_string(), Style::default().fg(c)));
        }
        lines.push(Line::from(spans));
    }
    lines
}

/// Bars with a peak cap per column that holds briefly, then falls.
fn draw_peaks(
    heights: &[f32],
    norm_peak: f32,
    cols: usize,
    rows: usize,
    theme: &Theme,
    dim: bool,
) -> Vec<Line<'static>> {
    let mut caps = PEAK_CAPS.lock().unwrap_or_else(|e| e.into_inner());
    if caps.len() != cols {
        *caps = vec![0.0; cols];
    }
    let norm: Vec<f32> = heights.iter().map(|h| (h / norm_peak).min(1.0)).collect();
    for (cap, &h) in caps.iter_mut().zip(&norm) {
        if h >= *cap {
            *cap = h;
        } else {
            // Gravity: falls faster the longer it has been above the bar.
            *cap = (*cap - 0.012 - (*cap - h) * 0.04).max(h);
        }
    }

    let display_rows = rows as f32;
    let cap_col = if dim { theme.dim } else { theme.text };
    let mut lines: Vec<Line> = Vec::with_capacity(rows);
    for display_row in (0..rows).rev() {
        let row_low = display_row as f32;
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(cols);
        for (i, &h) in norm.iter().enumerate() {
            let bar_float = h * display_rows;
            let cap_row = ((caps[i] * display_rows).ceil() as usize).min(rows.saturating_sub(1));
            let (ch, filled) = if bar_float <= row_low {
                (' ', false)
            } else if bar_float >= row_low + 1.0 {
                ('█', true)
            } else {
                let frac = bar_float - row_low;
                (BLOCKS[(frac * 8.0).round().clamp(1.0, 8.0) as usize], true)
            };
            // The cap sits on the row just above the bar's current top.
            let is_cap = !filled && display_row == cap_row && caps[i] > 0.02;
            let height_frac = display_row as f32 / display_rows;
            let style = if is_cap {
                Style::default().fg(cap_col)
            } else {
                Style::default().fg(bar_color(theme, height_frac, filled, dim))
            };
            spans.push(Span::styled(
                if is_cap {
                    "▁".to_string()
                } else {
                    ch.to_string()
                },
                style,
            ));
        }
        lines.push(Line::from(spans));
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The liquid feel is the rise/fall asymmetry: a jump reaches its target
    /// far quicker than it drains away.
    #[test]
    fn ease_rises_faster_than_it_falls() {
        // Same magnitude of change, opposite direction.
        let mut up = [0.0f32];
        ease_step(&mut up, &[1.0]);

        let mut down = [1.0f32];
        ease_step(&mut down, &[0.0]);

        let rose = up[0]; // distance moved up from 0
        let fell = 1.0 - down[0]; // distance moved down from 1
        assert!(rose > fell, "attack {rose} should exceed release {fell}");
    }

    /// Easing must converge, not overshoot or oscillate.
    #[test]
    fn ease_converges_to_target() {
        let mut s = [0.0f32];
        for _ in 0..200 {
            ease_step(&mut s, &[0.7]);
        }
        assert!((s[0] - 0.7).abs() < 1e-3, "settled at {}", s[0]);
    }
}
