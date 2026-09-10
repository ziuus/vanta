//! Custom widget system — public API.
//!
//! This module is the single entry point that `app.rs` and `screens/` use.
//! The internal sub-modules handle config, data acquisition, and rendering
//! independently.

pub mod config;
pub mod renderer;
pub mod source;
pub mod widget;

use ratatui::layout::Rect;
use ratatui::Frame;

use crate::custom::config::CustomWidgetConfig;
use crate::custom::widget::CustomWidget;
use crate::screens::panel;
use crate::theme::Theme;

// ── CustomWidgetManager ───────────────────────────────────────────────────────

/// Owns all live custom widgets and their background workers.
///
/// Created once by `App::new()`.  All methods are cheap to call on every
/// render frame; the actual work happens on background threads.
pub struct CustomWidgetManager {
    widgets: Vec<CustomWidget>,
}

impl CustomWidgetManager {
    /// Build from a slice of configs.  Invalid or disabled entries are silently
    /// skipped — they never crash the application.
    pub fn start_all(configs: &[CustomWidgetConfig]) -> Self {
        let widgets = configs
            .iter()
            .filter(|c| c.enabled)
            .filter_map(|c| {
                // validate() is also called inside DataWorker::spawn, but
                // logging a human-readable message here is useful.
                if let Err(e) = c.validate() {
                    log_skip(c, &e);
                    return None;
                }
                CustomWidget::new(c.clone())
            })
            .collect();
        Self { widgets }
    }

    /// Number of active (successfully started) custom widgets.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.widgets.len()
    }

    #[allow(dead_code)]
    pub fn is_empty(&self) -> bool {
        self.widgets.is_empty()
    }

    /// Advance history buffers.  Call once per render tick (cheap).
    /// This must be called from `App::render` (which has `&mut self`) before
    /// dispatching to any screen renderer.
    pub fn tick(&mut self) {
        for w in &mut self.widgets {
            w.tick();
        }
    }

    /// Borrow the widget at `index`, if it exists.
    #[allow(dead_code)]
    pub fn get(&self, index: usize) -> Option<&CustomWidget> {
        self.widgets.get(index)
    }

    /// Render widget `index` as a bordered panel inside `area`.
    ///
    /// The `focused` flag drives border/title highlight, matching the
    /// built-in panel convention from `screens::panel()`.
    /// Call `tick()` before entering the render loop to keep histories current.
    pub fn render_widget(
        &self,
        f: &mut Frame,
        area: Rect,
        index: usize,
        focused: bool,
        theme: &Theme,
    ) {
        let Some(w) = self.widgets.get(index) else {
            return;
        };
        let title = w.config.title.as_str();
        let result = w.result();
        let cfg = &w.config;
        let history = &w.history;

        let inner = panel(f, area, title, theme, focused);
        renderer::render(f, inner, cfg, &result, history, theme);
    }

    /// Render widget `index` zoomed to fill `area` (no border chrome — the
    /// zoom wrapper in `screens::render_panel` provides its own border).
    pub fn render_widget_inner(&self, f: &mut Frame, area: Rect, index: usize, theme: &Theme) {
        let Some(w) = self.widgets.get(index) else {
            return;
        };
        let result = w.result();
        let cfg = &w.config;
        let history = &w.history;
        renderer::render(f, area, cfg, &result, history, theme);
    }
}

// ── Helpers ───────────────────────────────────────────────────────────────────

fn log_skip(cfg: &CustomWidgetConfig, reason: &str) {
    if std::env::var("VANTA_DEBUG").as_deref() == Ok("1") {
        eprintln!("[vanta-custom] skipping widget {:?}: {}", cfg.id, reason);
    }
}
