pub mod aesthetic;
pub mod dashboard;
pub mod debug_logs;
pub mod help;
pub mod monitor;
pub mod settings;
pub mod workspace;

use ratatui::layout::{Alignment, Rect};
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::theme::Theme;

/// Panel chrome shared by every page: rounded border, small-caps title,
/// accent highlight when focused. Returns the inner area.
pub fn panel_full(
    f: &mut Frame,
    area: Rect,
    title: &str,
    right_title: Option<&str>,
    footer: Option<&str>,
    theme: &Theme,
    focused: bool,
) -> Rect {
    let (border, text) = if focused {
        (theme.accent, theme.accent)
    } else {
        (theme.surface, theme.dim)
    };

    let title_style = Style::default().fg(text);
    let mut block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::default().fg(border))
        .title_top(Line::from(Span::styled(
            format!(" {} ", title),
            title_style,
        )));

    if let Some(rt) = right_title {
        let rt_style = Style::default().fg(theme.dim);
        block = block.title_top(
            Line::from(Span::styled(format!(" {} ", rt), rt_style)).alignment(Alignment::Right),
        );
    }

    if let Some(ft) = footer {
        if focused {
            let ft_style = Style::default().fg(theme.accent);
            block = block.title_bottom(
                Line::from(Span::styled(format!(" {} ", ft), ft_style))
                    .alignment(Alignment::Center),
            );
        }
    }

    let inner = block.inner(area);
    f.render_widget(block, area);
    inner
}

pub fn panel(f: &mut Frame, area: Rect, title: &str, theme: &Theme, focused: bool) -> Rect {
    panel_full(f, area, title, None, None, theme, focused)
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
    let title = if id == P::PinnedMedia && app.panel_states.pinned_media_input_active {
        format!(
            "media path: {}_ · zoomed · esc to return",
            app.panel_states.pinned_media_input
        )
    } else {
        format!("{} · zoomed · esc to return", id.label())
    };
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
        P::Cpu => cpu::render(f, inner, theme, true),
        P::Memory => memory::render(f, inner, theme, true),
        P::Disk => disk::render(f, inner, theme, true),
        P::Storage => disk::render_storage(f, inner, theme),
        P::Network => network::render(f, inner, theme, true),
        P::Gpu => gpu::render(f, inner, theme, true),
        P::Clock => clock::render(
            f,
            inner,
            theme,
            app.config.ui.clock_24h,
            &app.config.ui.clock_font,
            &app.config.ui.clock_style,
            &app.config.ui.timezones,
        ),
        P::Media => media::render(f, inner, theme),
        P::Visualizer => music_viz::render(f, inner, theme, app.frame),
        P::Status => status::render(f, inner, theme, false, 0),
        P::Calendar => calendar::render(f, inner, theme, app.panel_states.calendar_month_offset),
        P::Matrix => matrix::render(f, inner, theme),
        P::Video => video::render(f, inner, theme, app.frame),
        P::Weather => crate::widgets::weather::render(f, inner, theme),
        P::UpNext => crate::widgets::upnext::render(f, inner, theme),
        P::Timer => crate::widgets::pomodoro::render(f, inner, theme, &app.config.ui, true),
        P::Agenda => crate::widgets::agenda::render(
            f,
            inner,
            theme,
            true,
            app.panel_states.agenda_selected,
            app.panel_states.agenda_input_active,
            &app.panel_states.agenda_input,
        ),
        P::Tasks => crate::widgets::tasks::render(
            f,
            inner,
            theme,
            true,
            app.panel_states.tasks_selected,
            app.panel_states.task_input_active,
            &app.panel_states.task_input,
        ),
        P::News => crate::widgets::news::render(f, inner, theme),
        P::PinnedMedia => crate::widgets::pinned_media::render(
            f,
            inner,
            theme,
            &app.config.ui.pinned_media_path,
            app.frame,
        ),
        P::WriterNotes => {}
        P::Files => {}
        P::GitHub | P::WorldClocks => {}
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
        P::Custom(idx) => {
            // The border with the zoom title is already drawn above (inner is
            // the area inside it).  Delegate pure content rendering.
            app.custom_widgets.render_widget_inner(f, inner, idx, theme);
        }
    }
}
