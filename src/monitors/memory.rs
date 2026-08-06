use std::sync::{LazyLock, Mutex};

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app;

fn usage_color(usage: f64, theme: &app::Theme) -> Color {
    if usage < 80.0 {
        theme.accent
    } else if usage < 95.0 {
        theme.yellow
    } else {
        theme.red
    }
}

const HIST_LEN: usize = 240;
static MEM_HISTORY: LazyLock<Mutex<([f64; HIST_LEN], usize)>> =
    LazyLock::new(|| Mutex::new(([0.0; HIST_LEN], 0)));

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme) {
    let sys = crate::app::SYS.lock().unwrap();
    let (total, used, swap_total, swap_used) = (
        sys.total_memory(),
        sys.used_memory(),
        sys.total_swap(),
        sys.used_swap(),
    );

    let used_gb = used as f64 / 1_073_741_824.0;
    let total_gb = total as f64 / 1_073_741_824.0;
    let pct = if total > 0 {
        (used as f64 / total as f64) * 100.0
    } else {
        0.0
    };

    // Record this sample into the ring buffer.
    {
        let mut mem = MEM_HISTORY.lock().unwrap();
        let idx = mem.1;
        mem.0[idx] = pct;
        mem.1 = (idx + 1) % HIST_LEN;
    }

    let chunks = Layout::vertical([
        Constraint::Length(1), // RAM text line
        Constraint::Length(2), // Trend sparkline
        Constraint::Length(1), // blank spacer
        Constraint::Length(1), // Swap text line
        Constraint::Min(0),    // Padding
    ])
    .split(area);

    let color = usage_color(pct, theme);

    // RAM Line
    let label = "RAM";
    let stats = format!("{:>4.1}/{:<4.1} GiB", used_gb, total_gb);
    let pct_str = format!("{:>3.0}%", pct);
    let needed_w = label.len() + stats.len() + pct_str.len() + 4; // spaces
    let dots_w = area.width.saturating_sub(needed_w as u16) as usize;
    let dots = if dots_w > 0 {
        "·".repeat(dots_w)
    } else {
        String::new()
    };

    let ram_line = Line::from(vec![
        Span::styled(format!("{} ", label), Style::default().fg(theme.dim)),
        Span::styled(format!("{} ", stats), Style::default().fg(theme.text)),
        Span::styled(dots, Style::default().fg(theme.dim)),
        Span::styled(
            format!(" {}", pct_str),
            Style::default()
                .fg(color)
                .add_modifier(ratatui::style::Modifier::BOLD),
        ),
    ]);
    f.render_widget(Paragraph::new(ram_line), chunks[0]);

    // Sparkline (Braille Graph)
    let max_w = (chunks[1].width as usize * 2).min(HIST_LEN);
    let hist: Vec<f64> = {
        let mem = MEM_HISTORY.lock().unwrap();
        let idx = mem.1;
        (0..max_w)
            .map(|i| mem.0[(idx + HIST_LEN - 1 - i) % HIST_LEN])
            .rev()
            .collect()
    };

    let braille = crate::widgets::block_graph::BlockGraph::new(&hist)
        .min(0.0)
        .max(100.0)
        .colors(theme.green, theme.yellow, theme.red);

    f.render_widget(braille, chunks[1]);

    // Swap Line
    if swap_total > 0 {
        let swap_used_gb = swap_used as f64 / 1_073_741_824.0;
        let swap_total_gb = swap_total as f64 / 1_073_741_824.0;
        let swap_pct = (swap_used as f64 / swap_total as f64) * 100.0;
        let swap_color = usage_color(swap_pct, theme);

        let label = "Swap";
        let stats = format!("{:>4.1}/{:<4.1} GiB", swap_used_gb, swap_total_gb);
        let pct_str = format!("{:>3.0}%", swap_pct);
        let needed_w = label.len() + stats.len() + pct_str.len() + 4;
        let dots_w = area.width.saturating_sub(needed_w as u16) as usize;
        let dots = if dots_w > 0 {
            "·".repeat(dots_w)
        } else {
            String::new()
        };

        let swap_line = Line::from(vec![
            Span::styled(format!("{} ", label), Style::default().fg(theme.dim)),
            Span::styled(format!("{} ", stats), Style::default().fg(theme.text)),
            Span::styled(dots, Style::default().fg(theme.dim)),
            Span::styled(
                format!(" {}", pct_str),
                Style::default()
                    .fg(swap_color)
                    .add_modifier(ratatui::style::Modifier::BOLD),
            ),
        ]);
        f.render_widget(Paragraph::new(swap_line), chunks[3]);
    } else {
        f.render_widget(
            Paragraph::new(Line::from(Span::styled(
                "Swap: None",
                Style::default().fg(theme.dim),
            ))),
            chunks[3],
        );
    }
}
