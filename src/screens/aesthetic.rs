use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::Frame;

use crate::app::{App, PanelId};
use crate::screens::{panel, too_small};
use crate::widgets::{calendar, clock, matrix, music_viz, video};

const MIN: (u16, u16) = (70, 24);

/// Eye candy: huge clock and calendar up top, matrix rain and the spinning
/// donut in the middle, a full-width visualizer along the bottom.
pub fn render(f: &mut Frame, area: Rect, app: &App) {
    if area.width < MIN.0 || area.height < MIN.1 {
        too_small(f, area, &app.theme, MIN);
        return;
    }
    let theme = &app.theme;
    let cfg = &app.config.widgets;
    let focus = |p: PanelId| app.focused_panel == Some(p);

    // Clock gets 12 rows when tall enough for the 2× glyphs, else 9.
    let clock_h = if area.height >= 40 { 14 } else { 9 };
    let viz_h = if cfg.music_viz {
        (area.height / 4).clamp(6, 12)
    } else {
        0
    };
    let rows = Layout::vertical([
        Constraint::Length(clock_h),
        Constraint::Min(6),
        Constraint::Length(viz_h),
    ])
    .split(area);

    let top = Layout::horizontal([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)]).split(rows[0]);
    let inner = panel(f, top[0], "clock", theme, focus(PanelId::Clock));
    clock::render(f, inner, theme, app.config.ui.clock_24h);
    let inner = panel(f, top[1], "calendar", theme, focus(PanelId::Calendar));
    calendar::render(f, inner, theme, app.panel_states.calendar_month_offset);

    match (cfg.matrix, cfg.video) {
        (true, true) => {
            let mid = Layout::horizontal([Constraint::Ratio(1, 2), Constraint::Ratio(1, 2)])
                .split(rows[1]);
            let inner = panel(f, mid[0], "matrix", theme, focus(PanelId::Matrix));
            matrix::render(f, inner, theme);
            let inner = panel(f, mid[1], "donut", theme, focus(PanelId::Video));
            video::render(f, inner, theme, app.frame);
        }
        (true, false) => {
            let inner = panel(f, rows[1], "matrix", theme, focus(PanelId::Matrix));
            matrix::render(f, inner, theme);
        }
        (false, _) => {
            let inner = panel(f, rows[1], "donut", theme, focus(PanelId::Video));
            video::render(f, inner, theme, app.frame);
        }
    }

    if cfg.music_viz {
        let inner = panel(f, rows[2], "visualizer", theme, focus(PanelId::Visualizer));
        music_viz::render(f, inner, theme, app.frame);
    }
}
