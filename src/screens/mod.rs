pub mod aesthetic;
pub mod dashboard;
pub mod help;
pub mod monitor;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// Panel chrome shared by every page: rounded border, small-caps title,
/// accent highlight when focused. Returns the inner area.
pub fn panel(f: &mut Frame, area: Rect, title: &str, theme: &Theme, focused: bool) -> Rect {
    let (border, text) = if focused {
        (theme.accent, theme.accent)
    } else {
        (theme.surface, theme.dim)
    };
    let title_style = Style::default().fg(text).add_modifier(Modifier::BOLD);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title(Span::styled(format!(" {} ", title), title_style));
    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

/// Centred notice for when a page can't fit. `need` is the page area; the
/// message speaks in terminal size (page + title and status bars).
pub fn too_small(f: &mut Frame, area: Rect, theme: &Theme, need: (u16, u16)) {
    let term = f.area();
    let msg = format!(
        "terminal too small — need {}×{}, have {}×{}",
        need.0,
        need.1 + 2,
        term.width,
        term.height
    );
    let y = area.y + area.height / 2;
    f.render_widget(
        Paragraph::new(Line::from(Span::styled(
            msg,
            Style::default().fg(theme.dim),
        )))
        .alignment(Alignment::Center),
        Rect::new(area.x, y, area.width, 1),
    );
}

/// Render one panel filling `area` — used by zoom (Enter on a focused panel).
pub fn render_panel(f: &mut Frame, area: Rect, app: &crate::app::App, id: crate::app::PanelId) {
    use crate::app::PanelId as P;
    use crate::monitors::{cpu, disk, gpu, memory, network, system_info};
    use crate::widgets::{calendar, clock, gauge, matrix, media, music_viz, status, video};

    let theme = &app.theme;
    let title = format!("{} · zoomed · esc to return", id.label());
    let inner = panel(f, area, &title, theme, true);
    let sum = &app.summary;
    match id {
        P::System => {
            system_info::render_neofetch(f, inner, theme, sum, (f.area().width, f.area().height))
        }
        P::Gauges => {
            let m = [
                (
                    "cpu",
                    sum.cpu_pct as f64,
                    format!("{:.0}%", sum.cpu_pct),
                    theme.usage(sum.cpu_pct as f64),
                ),
                (
                    "mem",
                    sum.mem_pct,
                    format!("{:.0}%", sum.mem_pct),
                    theme.usage(sum.mem_pct),
                ),
                (
                    "disk",
                    sum.disk_pct.unwrap_or(0.0),
                    format!("{:.0}%", sum.disk_pct.unwrap_or(0.0)),
                    theme.usage(sum.disk_pct.unwrap_or(0.0)),
                ),
            ];
            gauge::render(f, inner, theme, &m)
        }
        P::Cpu => cpu::render(f, inner, theme),
        P::Memory => memory::render(f, inner, theme),
        P::Disk => disk::render(f, inner, theme),
        P::Storage => disk::render_storage(f, inner, theme),
        P::Network => network::render(f, inner, theme),
        P::Gpu => gpu::render(f, inner, theme),
        P::Clock => clock::render(f, inner, theme, app.config.ui.clock_24h),
        P::Media => media::render(f, inner, theme),
        P::Visualizer => music_viz::render(f, inner, theme, app.frame),
        P::Status => status::render(f, inner, theme),
        P::Calendar => calendar::render(f, inner, theme, app.panel_states.calendar_month_offset),
        P::Matrix => matrix::render(f, inner, theme),
        P::Video => video::render(f, inner, theme, app.frame),
        P::Processes => {
            let ps = &app.panel_states;
            crate::monitors::processes::render(
                f,
                inner,
                theme,
                ps.process_scroll_offset,
                ps.process_sort_field,
                ps.process_sort_asc,
                &ps.process_search,
                ps.process_search_active,
                ps.process_tree_mode,
                &ps.process_collapsed,
                ps.process_selected_pid,
                ps.process_compact_cmd,
            )
        }
    }
}
