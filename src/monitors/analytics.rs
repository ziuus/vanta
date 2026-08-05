use std::sync::{LazyLock, Mutex};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{self, Summary};

/// Consolidated live metrics panel: CPU / MEM / DSK / TEMP with sparklines,
/// plus NET / GPU as value cells. Everything reads from the cached Summary,
/// so it never spawns subprocesses on its own.
const HIST_LEN: usize = 90;
const SPARK: [char; 8] = ['▁', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

static CPU_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static MEM_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static DSK_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static TMP_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static H_IDX: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
/// Number of real samples recorded so far (saturates at HIST_LEN). Without this
/// the untouched zeros in the ring render as a flat floor under the live data.
static H_FILL: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
/// Last recorded values, used to detect when Summary actually changed (vs. render spam)
static LAST: LazyLock<Mutex<(f32, f64, f64, f64)>> = LazyLock::new(|| Mutex::new((0.0, 0.0, 0.0, 0.0)));

fn push_hist(hist: &mut [f64], idx: usize, v: f64) {
    hist[idx] = v;
}

/// Sparkline over the most recent `n` samples, scaled to `scale` (the value that
/// maps to a full block). Slots with no data yet render blank rather than as a
/// flat row of ▁, which would read as "pinned at zero".
fn spark_str(hist: &[f64], idx: usize, n: usize, fill: usize, scale: f64) -> String {
    let n = n.clamp(4, HIST_LEN);
    let scale = if scale.abs() < f64::EPSILON { 1.0 } else { scale };
    let mut s = String::with_capacity(n);
    for i in 0..n {
        // How far back this column is from the newest sample.
        let age = n - i;
        if age > fill {
            s.push(' ');
            continue;
        }
        let k = (idx + HIST_LEN - age) % HIST_LEN;
        let level = ((hist[k] / scale).clamp(0.0, 1.0) * 7.0).round() as usize;
        s.push(SPARK[level.min(7)]);
    }
    s
}

fn usage_color(v: f64, theme: &app::Theme) -> Color {
    if v > 90.0 {
        theme.red
    } else if v > 75.0 {
        theme.yellow
    } else {
        theme.accent
    }
}

/// Record the current Summary into the shared ring buffers and return
/// (cpu, mem, dsk, tmp, idx). All four metrics share one ring position so their
/// graphs stay time-aligned; we only advance when Summary actually changed
/// (render fires ~125fps but Summary refreshes every 0.5s).
type Hist = [f64; HIST_LEN];
fn record(sum: &Summary) -> (Hist, Hist, Hist, Hist, usize, usize) {
    let mut last = LAST.lock().unwrap();
    let curr = (sum.cpu_pct, sum.mem_pct, sum.disk_pct, sum.temp_c);
    let changed = *last != curr;

    let mut idx = H_IDX.lock().unwrap();
    let mut cpu = CPU_H.lock().unwrap();
    let mut mem = MEM_H.lock().unwrap();
    let mut dsk = DSK_H.lock().unwrap();
    let mut tmp = TMP_H.lock().unwrap();

    if changed {
        let pos = *idx;
        push_hist(&mut cpu[..], pos, sum.cpu_pct as f64);
        push_hist(&mut mem[..], pos, sum.mem_pct);
        push_hist(&mut dsk[..], pos, sum.disk_pct);
        push_hist(&mut tmp[..], pos, sum.temp_c);
        *idx = (pos + 1) % HIST_LEN;
        *last = curr;
        let mut fill = H_FILL.lock().unwrap();
        *fill = (*fill + 1).min(HIST_LEN);
    }

    let fill = *H_FILL.lock().unwrap();
    (*cpu, *mem, *dsk, *tmp, *idx, fill)
}

fn temp_color(t: f64, theme: &app::Theme) -> Color {
    if t > 90.0 {
        theme.red
    } else if t > 75.0 {
        theme.yellow
    } else {
        theme.accent
    }
}

/// Compact vertical stack: one row per metric, `LABEL  VALUE  ▁▃▅█▇▅` — lvsk
/// style. Fits 6 metrics in 6 rows, for a narrow sidebar column.
pub fn render_compact(f: &mut Frame, area: Rect, theme: &app::Theme, sum: &Summary) {
    if area.height < 6 || area.width < 18 {
        return;
    }
    let (cpu_h, mem_h, dsk_h, tmp_h, idx, fill) = record(sum);

    // label(4) + space + value(8) + space => graph gets the rest
    let graph_n = (area.width as usize).saturating_sub(14);

    let metrics: [(&str, String, Color, Option<&Hist>, f64); 6] = [
        (
            "CPU",
            format!("{:.0}%", sum.cpu_pct),
            usage_color(sum.cpu_pct as f64, theme),
            Some(&cpu_h),
            100.0,
        ),
        (
            "MEM",
            format!("{:.0}%", sum.mem_pct),
            usage_color(sum.mem_pct, theme),
            Some(&mem_h),
            100.0,
        ),
        (
            "DSK",
            format!("{:.0}%", sum.disk_pct),
            usage_color(sum.disk_pct, theme),
            Some(&dsk_h),
            100.0,
        ),
        (
            "TMP",
            format!("{:.0}°C", sum.temp_c),
            temp_color(sum.temp_c, theme),
            Some(&tmp_h),
            // temps are degrees, not percent: full block at 100 °C
            100.0,
        ),
        (
            "GPU",
            format!("{}%", sum.gpu_pct),
            usage_color(sum.gpu_pct as f64, theme),
            None,
            0.0,
        ),
        (
            "NET",
            format!("↓{} ↑{}", sum.net_dl, sum.net_ul),
            theme.secondary,
            None,
            0.0,
        ),
    ];

    let lines: Vec<Line> = metrics
        .iter()
        .map(|(label, value, col, hist, scale)| {
            let mut spans = vec![
                Span::styled(format!("{:<4}", label), Style::default().fg(theme.dim)),
                Span::styled(
                    format!("{:>7} ", value),
                    Style::default().fg(*col).add_modifier(Modifier::BOLD),
                ),
            ];
            if let Some(h) = hist {
                if graph_n >= 4 {
                    spans.push(Span::styled(
                        spark_str(*h, idx, graph_n, fill, *scale),
                        Style::default().fg(*col),
                    ));
                }
            }
            Line::from(spans)
        })
        .collect();

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
    );
}
