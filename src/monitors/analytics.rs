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

static CPU_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static MEM_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static DSK_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static TMP_H: LazyLock<Mutex<[f64; HIST_LEN]>> = LazyLock::new(|| Mutex::new([0.0; HIST_LEN]));
static H_IDX: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
/// Number of real samples recorded so far (saturates at HIST_LEN). Without this
/// the untouched zeros in the ring render as a flat floor under the live data.
static H_FILL: LazyLock<Mutex<usize>> = LazyLock::new(|| Mutex::new(0));
/// Last recorded values, used to detect when Summary actually changed (vs. render spam)
static LAST: LazyLock<Mutex<(f32, f64, f64, f64)>> =
    LazyLock::new(|| Mutex::new((0.0, 0.0, 0.0, 0.0)));

fn push_hist(hist: &mut [f64], idx: usize, v: f64) {
    hist[idx] = v;
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
/// Graph-forward metrics column: each metric gets a header row (label + value)
/// and its own braille history graph beneath, so the graph gets the panel's
/// full width instead of competing with the label for it.
pub fn render_compact(f: &mut Frame, area: Rect, theme: &app::Theme, sum: &Summary) {
    if area.height < 6 || area.width < 18 {
        return;
    }
    let (cpu_h, mem_h, dsk_h, tmp_h, idx, fill) = record(sum);

    // Graphed metrics get a header + graph block; NET/GPU are value-only rows.
    let graphed: [(&str, String, Color, &Hist, f64); 4] = [
        (
            "CPU",
            format!("{:.0}%", sum.cpu_pct),
            usage_color(sum.cpu_pct as f64, theme),
            &cpu_h,
            100.0,
        ),
        (
            "MEM",
            format!("{:.0}%", sum.mem_pct),
            usage_color(sum.mem_pct, theme),
            &mem_h,
            100.0,
        ),
        (
            "TMP",
            format!("{:.0}\u{00b0}C", sum.temp_c),
            temp_color(sum.temp_c, theme),
            &tmp_h,
            100.0,
        ),
        (
            "DSK",
            format!("{:.0}%", sum.disk_pct),
            usage_color(sum.disk_pct, theme),
            &dsk_h,
            100.0,
        ),
    ];

    // Two rows of chrome (NET, GPU) plus one header row per graphed metric;
    // whatever is left is split evenly between the graphs.
    let n = graphed.len() as u16;
    let chrome = n + 2;
    let graph_space = area.height.saturating_sub(chrome);
    let per_graph = (graph_space / n).max(1);

    let mut y = area.y;
    let bottom = area.y + area.height;

    for (label, value, col, hist, scale) in graphed.iter() {
        if y >= bottom {
            break;
        }
        // Header: label left, value right-aligned against the panel edge.
        let pad = (area.width as usize).saturating_sub(4 + value.chars().count());
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{:<4}", label), Style::default().fg(theme.dim)),
                Span::raw(" ".repeat(pad)),
                Span::styled(
                    value.clone(),
                    Style::default().fg(*col).add_modifier(Modifier::BOLD),
                ),
            ])),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;

        let h = per_graph.min(bottom.saturating_sub(y));
        if h == 0 {
            break;
        }
        // Only feed the graph samples we actually recorded; the untouched tail
        // of the ring would otherwise draw as a flat floor at zero.
        let cap = area.width as usize * 2;
        let want = cap.min(fill);
        if want >= 2 {
            let series: Vec<f64> = (0..want)
                .map(|i| hist[(idx + HIST_LEN - want + i) % HIST_LEN])
                .collect();
            // Until the ring fills, draw into the right-hand slice so the trace
            // grows leftward from "now" rather than hugging the left edge.
            let cells = (want as u16).div_ceil(2).min(area.width);
            let gx = area.x + area.width - cells;
            let graph = crate::widgets::block_graph::BlockGraph::new(&series)
                .min(0.0)
                .max(*scale)
                .colors(*col, theme.yellow, theme.red);
            f.render_widget(graph, Rect::new(gx, y, cells, h));
        }
        y += h;
    }

    // NET / GPU as plain value rows at the foot of the column.
    for (label, value, col) in [
        (
            "NET",
            format!("\u{2193}{} \u{2191}{}", sum.net_dl, sum.net_ul),
            theme.secondary,
        ),
        (
            "GPU",
            format!("{}%", sum.gpu_pct),
            usage_color(sum.gpu_pct as f64, theme),
        ),
    ] {
        if y >= bottom {
            break;
        }
        f.render_widget(
            Paragraph::new(Line::from(vec![
                Span::styled(format!("{:<4}", label), Style::default().fg(theme.dim)),
                Span::styled(value, Style::default().fg(col).add_modifier(Modifier::BOLD)),
            ])),
            Rect::new(area.x, y, area.width, 1),
        );
        y += 1;
    }
}
