use std::sync::{LazyLock, Mutex};

use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Paragraph, Sparkline};
use ratatui::Frame;

use crate::app::{self, Summary};

const HIST_LEN: usize = 120;

static CPU_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static MEM_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static DSK_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static TMP_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static H_IDX: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
static H_FILL: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
static LAST: LazyLock<Mutex<(f32, f64, f64, f64)>> =
    LazyLock::new(|| Mutex::new((0.0, 0.0, 0.0, 0.0)));

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
        cpu[pos] = sum.cpu_pct as f64;
        mem[pos] = sum.mem_pct;
        dsk[pos] = sum.disk_pct;
        tmp[pos] = sum.temp_c;
        *idx = (pos + 1) % HIST_LEN;
        *last = curr;
        let mut fill = H_FILL.lock().unwrap();
        *fill = (*fill + 1).min(HIST_LEN);
    }

    let fill = *H_FILL.lock().unwrap();
    (*cpu, *mem, *dsk, *tmp, *idx, fill)
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

fn temp_color(t: f64, theme: &app::Theme) -> Color {
    if t > 90.0 {
        theme.red
    } else if t > 75.0 {
        theme.yellow
    } else {
        theme.accent
    }
}

/// Extract the most-recent `n` samples from the ring buffer as u64 (scaled ×10 for precision).
fn sparkline_data(hist: &Hist, idx: usize, fill: usize, n: usize, scale: f64) -> Vec<u64> {
    let want = n.min(fill);
    if want < 2 {
        return vec![];
    }
    (0..want)
        .map(|i| {
            let v = hist[(idx + HIST_LEN - want + i) % HIST_LEN];
            // Scale to 0-1000 range for sparkline resolution
            ((v / scale) * 1000.0).clamp(0.0, 1000.0) as u64
        })
        .collect()
}

/// Compact analytics panel: braille sparklines for CPU + MEM, then plain value rows.
/// Each graphed metric: [LABEL  VALUE] row + 2-row sparkline.
/// Then compact value rows for TMP / DSK / NET / GPU.
pub fn render_compact(f: &mut Frame, area: Rect, theme: &app::Theme, sum: &Summary) {
    if area.height < 4 || area.width < 14 {
        return;
    }

    let (cpu_h, mem_h, _dsk_h, _tmp_h, idx, fill) = record(sum);

    // Each graphed metric uses: 1 header row + 2 sparkline rows = 3 rows per metric
    // Plain value rows at the bottom: TMP, DSK, NET, GPU = 1 row each
    // Total for 2 graphed + 4 plain = 6 + 4 = 10 rows minimum

    let graphed: &[(&str, f64, Color, &Hist)] = &[
        (
            "CPU",
            sum.cpu_pct as f64,
            usage_color(sum.cpu_pct as f64, theme),
            &cpu_h,
        ),
        ("MEM", sum.mem_pct, usage_color(sum.mem_pct, theme), &mem_h),
    ];

    let mut y = area.y;
    let bottom = area.y + area.height;
    let w = area.width as usize;

    for (label, value, col, hist) in graphed {
        if y + 3 > bottom {
            break;
        }

        // Header: LABEL left, VALUE right, value colored
        let val_str = format!("{:.0}%", value);
        let pad = w.saturating_sub(label.len() + val_str.len());
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(
                    *label,
                    Style::default().fg(theme.dim).add_modifier(Modifier::BOLD),
                ),
                Span::raw(" ".repeat(pad)),
                Span::styled(
                    val_str,
                    Style::default().fg(*col).add_modifier(Modifier::BOLD),
                ),
            ])),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;

        // Braille sparkline (2 rows)
        let spark_h = 2u16.min(bottom.saturating_sub(y));
        if spark_h > 0 {
            let data = sparkline_data(hist, idx, fill, w * 2, 100.0);
            if data.len() >= 2 {
                f.render_widget(
                    Sparkline::default()
                        .data(&data)
                        .max(1000)
                        .style(Style::default().fg(*col)),
                    Rect::new(area.x, y, area.width, spark_h),
                );
            }
            y += spark_h;
        }

        // Single blank line separator between metrics
        if y < bottom {
            y += 1;
        }
    }

    // Compact value rows: TMP / DSK / NET / GPU — two per row if width allows
    let plain: &[(&str, String, Color)] = &[
        (
            "TMP",
            format!("{:.0}°C", sum.temp_c),
            temp_color(sum.temp_c, theme),
        ),
        (
            "DSK",
            format!("{:.0}%", sum.disk_pct),
            usage_color(sum.disk_pct, theme),
        ),
        (
            "NET",
            format!("↓{} ↑{}", sum.net_dl, sum.net_ul),
            theme.secondary,
        ),
        (
            "GPU",
            format!("{}%", sum.gpu_pct),
            usage_color(sum.gpu_pct as f64, theme),
        ),
    ];

    // Two items per row (left half | right half)
    for pair in plain.chunks(2) {
        if y >= bottom {
            break;
        }
        let mut spans = Vec::new();
        for (i, (label, val, col)) in pair.iter().enumerate() {
            if i == 1 {
                // spacer to fill half
                let half = area.width as usize / 2;
                let used = pair[0].0.len() + 2 + pair[0].1.len();
                let gap = half.saturating_sub(used);
                spans.push(Span::raw(" ".repeat(gap + 2)));
            }
            spans.push(Span::styled(
                format!("{} ", label),
                Style::default().fg(theme.dim),
            ));
            spans.push(Span::styled(
                val.clone(),
                Style::default().fg(*col).add_modifier(Modifier::BOLD),
            ));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }
}
