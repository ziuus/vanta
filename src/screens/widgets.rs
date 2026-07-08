use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use ratatui::widgets::{Block, Borders};
use ratatui::Frame;

use crate::config::Config;
use crate::widgets::{clock, matrix};
use crate::app;

pub fn render(f: &mut Frame, area: Rect, theme: &app::Theme, _config: &Config) {
    let chunks =
        Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)]).split(area);

    // Clock widget
    let clock_block = Block::default()
        .title(" ⏰ Clock ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.dim))
        .style(Style::default().bg(theme.surface));
    let clock_inner = clock_block.inner(chunks[0]);
    f.render_widget(clock_block, chunks[0]);
    clock::render(f, clock_inner, theme);

    // Matrix widget
    let matrix_block = Block::default()
        .title(" 🌧️ Matrix ")
        .borders(Borders::ALL)
        .border_style(Style::default().fg(theme.dim))
        .style(Style::default().bg(theme.surface));
    let matrix_inner = matrix_block.inner(chunks[1]);
    f.render_widget(matrix_block, chunks[1]);
    matrix::render(f, matrix_inner, theme);
}
