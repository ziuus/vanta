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
// 3 = peaks (bars with falling caps), 4 = braille (high-res dot matrix).
const STYLE_COUNT: usize = 5;
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
        "braille" => 4,
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
framerate = 60
sleep_timer = 2

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

/// Start cava if it isn't running, so `audio_active` works even while the
/// visualizer itself is collapsed off screen. Cheap: retries are rate-limited.
pub fn ensure_running() {
    ensure_cava();
}

/// True while cava is delivering non-silent audio.
pub fn audio_active() -> bool {
    CAVA_RUNNING.load(Ordering::Relaxed) && SILENCE_FRAMES.lock().is_ok_and(|sf| *sf <= 8)
}

/// Overall loudness right now, 0..1 (mean of cava's bars); 0 when silent
/// or when cava isn't running.
pub fn energy() -> f32 {
    if !audio_active() {
        return 0.0;
    }
    let bars = read_cava_bars();
    (bars.iter().sum::<f32>() / bars.len().max(1) as f32).clamp(0.0, 1.0)
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
        4 => "braille",
        _ => "bars",
    }
}

fn detect_monitor_source() -> Option<String> {
    let output = Command::new("pactl")
        .args(["list", "sources", "short"])
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);

    // 1. Get the default sink first. If it exists and has a monitor, that's our best bet.
    let mut default_monitor = None;
    if let Ok(out) = Command::new("pactl").args(["get-default-sink"]).output() {
        let sink = String::from_utf8_lossy(&out.stdout).trim().to_string();
        if !sink.is_empty() {
            default_monitor = Some(format!("{sink}.monitor"));
        }
    }

    let mut running_monitor = None;
    let mut idle_monitor = None;
    let mut any_monitor = None;

    for line in stdout.lines() {
        let parts: Vec<&str> = line.split('\t').collect();
        if parts.len() < 4 {
            continue;
        }
        let name = parts[1];
        let state = parts[3];

        if name.ends_with(".monitor") {
            // If this is the default monitor and it's not explicitly suspended, use it right away!
            if Some(name) == default_monitor.as_deref() && state != "SUSPENDED" {
                return Some(name.to_string());
            }
            if state == "RUNNING" && running_monitor.is_none() {
                running_monitor = Some(name.to_string());
            } else if state == "IDLE" && idle_monitor.is_none() {
                idle_monitor = Some(name.to_string());
            }
            if any_monitor.is_none() {
                any_monitor = Some(name.to_string());
            }
        }
    }

    // 2. If default sink wasn't active, pick the first explicitly RUNNING monitor
    if let Some(m) = running_monitor {
        return Some(m);
    }

    // 3. Pick the first explicitly IDLE monitor
    if let Some(m) = idle_monitor {
        return Some(m);
    }

    // 4. Fallback to default sink monitor even if it's suspended, so Pipewire connects when it wakes
    if let Some(dm) = default_monitor {
        if stdout.lines().any(|l| l.contains(&dm)) {
            return Some(dm);
        }
    }

    // 5. Last resort
    any_monitor
}

fn read_cava_bars() -> Vec<f32> {
    CAVA_BARS.lock().unwrap_or_else(|e| e.into_inner()).clone()
}

// ── Spatial resample: logarithmic frequency spacing so bass, mid, and treble
// are evenly distributed across the panel width. Linear mapping clusters all
// energy into the left third (bass-heavy) leaving a dead gap on the right.
// Log spacing mirrors how human hearing works and how cava looks standalone. ──
fn resample_max(src: &[f32], dst_len: usize) -> Vec<f32> {
    if src.is_empty() || dst_len == 0 {
        return vec![0.0f32; dst_len];
    }
    let src_len = src.len();
    // Map each output column to a logarithmically-spaced input range.
    // log_min is slightly above 0 so log(0) is avoided; log_max = log(src_len).
    let log_min = 1.0f64;
    let log_max = (src_len as f64 + 1.0).ln();
    (0..dst_len)
        .map(|i| {
            // Fraction [0..1] along the output.
            let t0 = i as f64 / dst_len as f64;
            let t1 = (i + 1) as f64 / dst_len as f64;
            // Map through log scale to a source index range.
            let s0 = ((log_min + t0 * (log_max - log_min)).exp() - 1.0).round() as usize;
            let s1 = ((log_min + t1 * (log_max - log_min)).exp() - 1.0).round() as usize;
            let start = s0.min(src_len - 1);
            let end = (s1 + 1).min(src_len);
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
const BLOCKS: [char; 9] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

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

    let style = VIZ_STYLE.load(Ordering::Relaxed) % STYLE_COUNT;
    let logical_cols = if style == 4 { term_cols * 2 } else { term_cols };

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
        idle_wave(logical_cols, _tick)
    } else {
        crate::anim::request_full();
        resample_max(&raw, logical_cols)
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

    let dim = is_silent;

    let lines = match style {
        1 => draw_mirror(&heights, norm_peak, logical_cols, term_rows, theme, dim),
        2 => draw_wave(&heights, norm_peak, logical_cols, term_rows, theme, dim),
        3 => draw_peaks(&heights, norm_peak, logical_cols, term_rows, theme, dim),
        4 => draw_braille(&heights, norm_peak, logical_cols, term_rows, theme, dim),
        _ => draw_bars(&heights, norm_peak, logical_cols, term_rows, theme, dim),
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

/// Braille dot-matrix: each terminal cell holds a 2×4 braille dot grid, giving
/// twice the horizontal and four times the vertical resolution of block chars.
///
/// Layout:  col pairs → one braille char per two amplitude columns
///          rows packed 4 per terminal row (bottom 4 bits = left col, top 4 = right)
///
/// Unicode braille block: U+2800..U+28FF
/// Dot positions (bit indices in the Unicode braille table):
///   bit 0 = row 0 left,  bit 3 = row 3 left
///   bit 4 = row 0 right, bit 7 = row 3 right
///   rows are ordered top→bottom.
fn draw_braille(
    heights: &[f32],
    norm_peak: f32,
    cols: usize,
    rows: usize,
    theme: &Theme,
    dim: bool,
) -> Vec<Line<'static>> {
    // Each braille char covers 2 amplitude columns and 4 dot rows.
    // dot_rows = rows * 4, braille_cols = ceil(cols / 2)
    let dot_rows = rows * 4;
    let braille_cols = cols.div_ceil(2);

    // Normalise heights to [0..dot_rows].
    let norm: Vec<usize> = heights
        .iter()
        .map(|&h| ((h / norm_peak).min(1.0) * dot_rows as f32).round() as usize)
        .collect();

    // Braille bit layout (Unicode braille, standard ordering):
    // dot:  1 4    bit: 0 3
    //       2 5         1 4
    //       3 6         2 5
    //       7 8         6 7
    // Left column uses bits 0,1,2,6; right uses bits 3,4,5,7.
    // Row from bottom → bit index mapping (bottom-up rendering):
    const LEFT_BITS: [u8; 4] = [6, 2, 1, 0]; // dot rows 0(bottom)..3(top) left col
    const RIGHT_BITS: [u8; 4] = [7, 5, 4, 3]; // same for right col

    let color = if dim { theme.dim } else { theme.accent };
    let top_color = if dim { theme.dim } else { theme.secondary };

    let mut lines: Vec<Line> = Vec::with_capacity(rows);

    for term_row in (0..rows).rev() {
        let mut spans: Vec<Span<'static>> = Vec::with_capacity(braille_cols);
        for bc in 0..braille_cols {
            let left_col = bc * 2;
            let right_col = bc * 2 + 1;

            let left_h = if left_col < norm.len() {
                norm[left_col]
            } else {
                0
            };
            let right_h = if right_col < norm.len() {
                norm[right_col]
            } else {
                0
            };

            let mut bits: u8 = 0;
            // Fill dots bottom-up within this terminal row.
            for dot in 0..4usize {
                // dot 0 = bottom-most dot row of this cell
                let dot_row_abs = term_row * 4 + dot; // absolute dot row from bottom
                if left_h > dot_row_abs {
                    bits |= 1 << LEFT_BITS[dot];
                }
                if right_h > dot_row_abs {
                    bits |= 1 << RIGHT_BITS[dot];
                }
            }

            let ch = if bits == 0 {
                ' '
            } else {
                char::from_u32(0x2800 | bits as u32).unwrap_or(' ')
            };

            // Top portion of each bar → secondary color for gradient feel.
            let max_h = left_h.max(right_h);
            let cell_mid = term_row * 4 + 2;
            let c = if max_h >= cell_mid && !dim {
                top_color
            } else {
                color
            };

            spans.push(Span::styled(ch.to_string(), Style::default().fg(c)));
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
