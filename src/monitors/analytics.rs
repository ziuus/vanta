use std::sync::{LazyLock, Mutex};

use ratatui::layout::{Constraint, Layout, Rect};
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
/// Last recorded values, used to detect when Summary actually changed (vs. render spam)
static LAST: LazyLock<Mutex<(f32, f64, f64, f64)>> = LazyLock::new(|| Mutex::new((0.0, 0.0, 0.0, 0.0)));

fn push_hist(hist: &mut [f64], idx: usize, v: f64) {
    hist[idx] = v;
}

fn spark_str(hist: &[f64], idx: usize, n: usize) -> String {
    let n = n.clamp(4, HIST_LEN);
    let mut s = String::with_capacity(n);
    for i in 0..n {
        let k = (idx + HIST_LEN - n + i) % HIST_LEN;
        let v = hist[k];
        let level = ((v / 100.0).clamp(0.0, 1.0) * 7.0).round() as usize;
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

/// A metric cell: right-aligned label, bold colored value, inline sparkline.
#[allow(clippy::too_many_arguments)]
fn render_spark(
    f: &mut Frame,
    area: Rect,
    theme: &app::Theme,
    label: &str,
    value: String,
    col: Color,
    hist: &[f64],
    idx: usize,
) {
    if area.width < 12 {
        return;
    }
    let label_span = Span::styled(format!("{:<5}", label), Style::default().fg(theme.dim));
    let val_span = Span::styled(
        value.clone(),
        Style::default().fg(col).add_modifier(Modifier::BOLD),
    );
    let prefix_len = 5 + 1 + value.chars().count();
    let spark_n = area.width as usize - prefix_len - 1;
    let mut spans = vec![label_span, val_span, Span::styled(" ", Style::default())];
    if spark_n >= 4 {
        let spark = spark_str(hist, idx, spark_n);
        let spark_col = if value.ends_with('°') {
            theme.dim
        } else {
            col
        };
        spans.push(Span::styled(spark, Style::default().fg(spark_col)));
    }
    f.render_widget(Paragraph::new(Line::from(spans)), area);
}

/// A value cell: label + bold value, left-aligned.
fn render_value(
    f: &mut Frame,
    area: Rect,
    theme: &app::Theme,
    label: &str,
    value: String,
    col: Color,
) {
    if area.width < 8 {
        return;
    }
    let label_span = Span::styled(format!("{:<5}", label), Style::default().fg(theme.dim));
    let val_span = Span::styled(value, Style::default().fg(col).add_modifier(Modifier::BOLD));
    f.render_widget(Paragraph::new(Line::from(vec![label_span, val_span])), area);
}

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, sum: &Summary) {
    if area.height < 3 || area.width < 30 {
        return;
    }

    // Record histories from the shared summary. All four metrics share one
    // ring position per frame so their sparklines stay time-aligned. Only advance
    // when Summary actually changed (render fires ~125fps but Summary refreshes
    // every 0.5s — without this check, 90 slots fill in <1s with duplicates).
    let (cpu_h, mem_h, dsk_h, tmp_h, idx) = {
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
        }

        (*cpu, *mem, *dsk, *tmp, *idx)
    };

    let rows = Layout::vertical([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
        .spacing(1)
        .split(area);
    if rows.len() < 2 {
        return;
    }
    let top = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .spacing(1)
    .split(rows[0]);
    let bot = Layout::horizontal([
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
        Constraint::Ratio(1, 3),
    ])
    .spacing(1)
    .split(rows[1]);

    render_spark(
        f,
        top[0],
        theme,
        "CPU",
        format!("{:.0}%", sum.cpu_pct),
        usage_color(sum.cpu_pct as f64, theme),
        &cpu_h,
        idx,
    );
    render_spark(
        f,
        top[1],
        theme,
        "MEM",
        format!("{:.0}%", sum.mem_pct),
        usage_color(sum.mem_pct, theme),
        &mem_h,
        idx,
    );
    render_spark(
        f,
        top[2],
        theme,
        "DSK",
        format!("{:.0}%", sum.disk_pct),
        usage_color(sum.disk_pct, theme),
        &dsk_h,
        idx,
    );

    let temp_col = if sum.temp_c > 90.0 {
        theme.red
    } else if sum.temp_c > 75.0 {
        theme.yellow
    } else {
        theme.accent
    };
    render_spark(
        f,
        bot[0],
        theme,
        "TEMP",
        format!("{:.0}°C", sum.temp_c),
        temp_col,
        &tmp_h,
        idx,
    );
    render_value(
        f,
        bot[1],
        theme,
        "NET",
        format!("↓{} ↑{}", sum.net_dl, sum.net_ul),
        theme.secondary,
    );
    render_value(
        f,
        bot[2],
        theme,
        "GPU",
        format!("{}%", sum.gpu_pct),
        usage_color(sum.gpu_pct as f64, theme),
    );
}
