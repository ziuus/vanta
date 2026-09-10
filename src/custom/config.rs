//! Configuration types for custom widgets.
//!
//! These map 1-to-1 onto the `[[custom_widgets]]` TOML array entries in
//! `~/.config/vanta/config.toml`.  Every optional field has a serde default
//! so existing configs that predate this feature continue to load without error.

use serde::{Deserialize, Serialize};

// ── Source kind ───────────────────────────────────────────────────────────────

/// Where a custom widget fetches its raw string value from.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SourceKind {
    /// Run a shell command and capture its stdout.
    #[default]
    Command,
    /// Read a file (e.g. `/sys/class/…`, `/proc/…`).
    File,
    // Future: Http, Socket, Dbus, …
}

// ── Renderer kind ─────────────────────────────────────────────────────────────

/// How the fetched value is displayed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RendererKind {
    /// Single numeric or text value, large and centered.
    #[default]
    Value,
    /// Plain text output, word-wrapped.
    Text,
    /// Horizontal progress bar with optional min/max.
    Gauge,
    /// Single horizontal bar representing the current magnitude.
    Bar,
    /// Scrolling history graph (uses existing BlockGraph primitive).
    Graph,
}

// ── CustomWidgetConfig ────────────────────────────────────────────────────────

/// One entry under `[[custom_widgets]]` in `config.toml`.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct CustomWidgetConfig {
    /// Unique identifier used for focus/zoom routing (e.g. `"pi_battery"`).
    /// Must be non-empty after trimming.
    pub id: String,

    /// Human-readable panel title shown in the border.
    pub title: String,

    /// Data source type.
    pub source: SourceKind,

    /// Shell command to run (used when `source = "command"`).
    /// Split on whitespace; the first token is the executable.
    pub command: Option<String>,

    /// File path to read (used when `source = "file"`).
    pub path: Option<String>,

    /// How to render the value.
    pub renderer: RendererKind,

    /// How often to re-fetch the data, in seconds. Clamped to 0.1–3600.
    pub refresh: f64,

    /// Optional unit suffix appended to numeric displays (e.g. `"V"`, `"°C"`).
    pub unit: Option<String>,

    /// Minimum value for gauge/bar normalisation.  Defaults to 0.
    pub min: Option<f64>,

    /// Maximum value for gauge/bar normalisation.  Defaults to 100.
    pub max: Option<f64>,

    /// Set to `false` to hide this widget without removing the entry.
    pub enabled: bool,
}

impl Default for CustomWidgetConfig {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: String::new(),
            source: SourceKind::default(),
            command: None,
            path: None,
            renderer: RendererKind::default(),
            refresh: 5.0,
            unit: None,
            min: None,
            max: None,
            enabled: true,
        }
    }
}

impl CustomWidgetConfig {
    /// Clamp refresh interval to a sensible range.
    pub fn clamped_refresh(&self) -> f64 {
        self.refresh.clamp(0.1, 3600.0)
    }

    /// Effective minimum for gauge/bar rendering.
    pub fn effective_min(&self) -> f64 {
        self.min.unwrap_or(0.0)
    }

    /// Effective maximum for gauge/bar rendering.
    pub fn effective_max(&self) -> f64 {
        let m = self.max.unwrap_or(100.0);
        // Guard against degenerate max <= min.
        if m <= self.effective_min() {
            self.effective_min() + 1.0
        } else {
            m
        }
    }

    /// Returns `Err` with a human-readable reason if the config is invalid.
    /// Validation errors are non-fatal: the widget is skipped, not panicked.
    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("custom widget has an empty id".into());
        }
        match self.source {
            SourceKind::Command => {
                if self
                    .command
                    .as_deref()
                    .map(str::trim)
                    .unwrap_or("")
                    .is_empty()
                {
                    return Err(format!(
                        "custom widget {:?} has source=command but no command set",
                        self.id
                    ));
                }
            }
            SourceKind::File => {
                if self.path.as_deref().map(str::trim).unwrap_or("").is_empty() {
                    return Err(format!(
                        "custom widget {:?} has source=file but no path set",
                        self.id
                    ));
                }
            }
        }
        Ok(())
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn toml_widget(s: &str) -> CustomWidgetConfig {
        toml::from_str(s).expect("parse failed")
    }

    // ---- deserialization & defaults ----------------------------------------

    #[test]
    fn minimal_command_widget_deserialises() {
        let cfg = toml_widget(
            r#"
            id = "cpu_temp"
            title = "CPU Temp"
            source = "command"
            command = "cat /sys/class/thermal/thermal_zone0/temp"
            renderer = "value"
            refresh = 2.0
            "#,
        );
        assert_eq!(cfg.id, "cpu_temp");
        assert_eq!(cfg.source, SourceKind::Command);
        assert_eq!(cfg.renderer, RendererKind::Value);
        assert!(cfg.enabled);
        assert_eq!(cfg.clamped_refresh(), 2.0);
    }

    #[test]
    fn file_widget_with_gauge() {
        let cfg = toml_widget(
            r#"
            id = "battery"
            title = "Battery"
            source = "file"
            path = "/sys/class/power_supply/BAT0/capacity"
            renderer = "gauge"
            refresh = 1.0
            min = 0
            max = 100
            "#,
        );
        assert_eq!(cfg.source, SourceKind::File);
        assert_eq!(cfg.renderer, RendererKind::Gauge);
        assert_eq!(cfg.effective_min(), 0.0);
        assert_eq!(cfg.effective_max(), 100.0);
    }

    #[test]
    fn defaults_are_sane() {
        let cfg = toml_widget(
            r#"
            id = "x"
            command = "echo hi"
            "#,
        );
        assert!(cfg.enabled);
        assert_eq!(cfg.clamped_refresh(), 5.0);
        assert_eq!(cfg.effective_min(), 0.0);
        assert_eq!(cfg.effective_max(), 100.0);
        assert!(cfg.unit.is_none());
    }

    #[test]
    fn refresh_is_clamped() {
        let cfg = toml_widget(
            r#"
            id = "x"
            command = "echo hi"
            refresh = 0.0
            "#,
        );
        assert_eq!(cfg.clamped_refresh(), 0.1);

        let cfg = toml_widget(
            r#"
            id = "x"
            command = "echo hi"
            refresh = 99999.0
            "#,
        );
        assert_eq!(cfg.clamped_refresh(), 3600.0);
    }

    #[test]
    fn degenerate_max_is_fixed() {
        let cfg = toml_widget(
            r#"
            id = "x"
            command = "echo hi"
            min = 50.0
            max = 50.0
            "#,
        );
        assert!(cfg.effective_max() > cfg.effective_min());
    }

    #[test]
    fn enabled_can_be_set_false() {
        let cfg = toml_widget(
            r#"
            id = "x"
            command = "echo hi"
            enabled = false
            "#,
        );
        assert!(!cfg.enabled);
    }

    // ---- validation --------------------------------------------------------

    #[test]
    fn empty_id_fails_validation() {
        let cfg = toml_widget(
            r#"
            id = ""
            command = "echo hi"
            "#,
        );
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn command_source_without_command_fails() {
        let cfg = toml_widget(
            r#"
            id = "x"
            source = "command"
            "#,
        );
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn file_source_without_path_fails() {
        let cfg = toml_widget(
            r#"
            id = "x"
            source = "file"
            "#,
        );
        assert!(cfg.validate().is_err());
    }

    #[test]
    fn valid_command_widget_passes() {
        let cfg = toml_widget(
            r#"
            id = "x"
            source = "command"
            command = "echo hi"
            "#,
        );
        assert!(cfg.validate().is_ok());
    }

    #[test]
    fn valid_file_widget_passes() {
        let cfg = toml_widget(
            r#"
            id = "x"
            source = "file"
            path = "/tmp/test"
            "#,
        );
        assert!(cfg.validate().is_ok());
    }

    // ---- array deserialisation (as it appears in the parent Config) ---------

    #[test]
    fn array_of_widgets_parses() {
        let s = r#"
        [[custom_widgets]]
        id = "volt"
        title = "Pi Battery"
        source = "command"
        command = "vcgencmd measure_volts"
        renderer = "value"
        refresh = 2.0

        [[custom_widgets]]
        id = "bat"
        title = "Battery"
        source = "file"
        path = "/sys/class/power_supply/BAT0/capacity"
        renderer = "gauge"
        refresh = 1.0
        "#;

        #[derive(serde::Deserialize)]
        struct Wrapper {
            custom_widgets: Vec<CustomWidgetConfig>,
        }
        let w: Wrapper = toml::from_str(s).unwrap();
        assert_eq!(w.custom_widgets.len(), 2);
        assert_eq!(w.custom_widgets[0].id, "volt");
        assert_eq!(w.custom_widgets[1].renderer, RendererKind::Gauge);
    }
}
