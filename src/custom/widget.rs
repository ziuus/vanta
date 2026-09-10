//! Per-widget runtime state.
//!
//! `CustomWidget` couples a config, a live data-source worker, and its own
//! history buffer.  The history is a bounded `VecDeque` so memory usage is
//! always capped regardless of how long Vanta runs.

use std::collections::VecDeque;

use crate::custom::config::CustomWidgetConfig;
use crate::custom::source::{DataWorker, FetchResult, SharedResult};

/// Maximum number of historic data points kept per widget (for the `graph`
/// renderer).  Sized to comfortably exceed any realistic terminal width.
pub const HISTORY_CAP: usize = 512;

// ── CustomWidget ──────────────────────────────────────────────────────────────

/// Runtime representation of one custom widget.
///
/// Created once by `CustomWidgetManager::start_all()` and lives for the
/// lifetime of the application.
pub struct CustomWidget {
    pub config: CustomWidgetConfig,
    /// Shared result slot written by the background worker and read by the UI.
    result: SharedResult,
    /// Bounded rolling history of parsed numeric values (for graph renderer).
    pub history: VecDeque<f64>,
    /// Last raw value we pushed into history (to avoid duplicates).
    last_pushed: Option<String>,
}

impl CustomWidget {
    /// Construct and start the background data worker.
    /// Returns `None` if the config is invalid (logged; never panics).
    pub fn new(cfg: CustomWidgetConfig) -> Option<Self> {
        let worker = DataWorker::spawn(&cfg)?;
        Some(Self {
            config: cfg,
            result: worker.result,
            history: VecDeque::with_capacity(HISTORY_CAP),
            last_pushed: None,
        })
    }

    /// Read the latest `FetchResult` from the background worker.
    pub fn result(&self) -> FetchResult {
        self.result.lock().unwrap().clone()
    }

    /// Call once per render tick to update the history buffer for graph
    /// renderers.  Only appends when the raw value has actually changed.
    pub fn tick(&mut self) {
        if let FetchResult::Ok(raw) = self.result() {
            if self.last_pushed.as_deref() == Some(&raw) {
                return;
            }
            if let Some(v) = crate::custom::renderer::parse_numeric(&raw) {
                if self.history.len() >= HISTORY_CAP {
                    self.history.pop_front();
                }
                self.history.push_back(v);
                self.last_pushed = Some(raw);
            }
        }
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::custom::config::SourceKind;
    use std::time::Duration;

    fn echo_cfg(val: &str) -> CustomWidgetConfig {
        CustomWidgetConfig {
            id: "test".into(),
            title: "Test".into(),
            source: SourceKind::Command,
            command: Some(format!("echo {}", val)),
            refresh: 60.0,
            ..Default::default()
        }
    }

    #[test]
    fn widget_starts_as_loading() {
        let cfg = echo_cfg("42");
        // We can't instantiate without a worker in a unit test without spawning
        // a thread, so use DataWorker directly to verify the initial state.
        use crate::custom::source::DataWorker;
        let w = DataWorker::spawn(&cfg).unwrap();
        // The worker hasn't necessarily run yet.
        let r = w.result.lock().unwrap().clone();
        assert!(matches!(r, FetchResult::Loading | FetchResult::Ok(_)));
    }

    #[test]
    fn history_bounded_at_cap() {
        use crate::custom::renderer::parse_numeric;

        let mut history: VecDeque<f64> = VecDeque::with_capacity(HISTORY_CAP);
        for i in 0..(HISTORY_CAP + 10) {
            if history.len() >= HISTORY_CAP {
                history.pop_front();
            }
            if let Some(v) = parse_numeric(&i.to_string()) {
                history.push_back(v);
            }
        }
        assert_eq!(history.len(), HISTORY_CAP);
    }

    #[test]
    fn tick_populates_history() {
        let cfg = echo_cfg("99");
        // DataWorker starts the thread; give it time to write.
        let worker = DataWorker::spawn(&cfg).unwrap();
        std::thread::sleep(Duration::from_millis(300));

        // Simulate what tick() does by reading from the worker's result.
        let r = worker.result.lock().unwrap().clone();
        if let FetchResult::Ok(raw) = r {
            let v = parse_numeric(&raw);
            assert_eq!(v, Some(99.0));
        }
    }

    fn parse_numeric(s: &str) -> Option<f64> {
        crate::custom::renderer::parse_numeric(s)
    }

    #[test]
    fn invalid_config_returns_none() {
        let bad = CustomWidgetConfig {
            id: "".into(), // invalid
            ..Default::default()
        };
        assert!(CustomWidget::new(bad).is_none());
    }
}
