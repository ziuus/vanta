use serde::{Deserialize, Serialize};

use crate::custom::config::CustomWidgetConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub widgets: WidgetConfig,
    pub dashboard: DashboardConfig,
    /// User-defined custom widgets declared via `[[custom_widgets]]` entries.
    /// Existing configs that omit this field load fine — serde defaults to an
    /// empty `Vec`.
    pub custom_widgets: Vec<CustomWidgetConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct DashboardConfig {
    /// The currently selected layout preset name (e.g. "cockpit", "minimal").
    pub preset: String,
    /// Multi-column layout specifying the panel names in order for each column.
    /// Default: 3-column cockpit.
    pub layout: Vec<Vec<String>>,
}

impl DashboardConfig {
    pub fn preset_cockpit() -> Vec<Vec<String>> {
        vec![
            vec![
                "system".into(),
                "gauges".into(),
                "cpu".into(),
                "storage".into(),
            ],
            vec![
                "clock".into(),
                "media".into(),
                "visualizer".into(),
                "processes".into(),
            ],
            vec![
                "status".into(),
                "weather".into(),
                "memory".into(),
                "network".into(),
                "calendar".into(),
            ],
        ]
    }

    pub fn preset_minimal() -> Vec<Vec<String>> {
        vec![
            vec![
                "system".into(),
                "cpu".into(),
                "memory".into(),
                "network".into(),
            ],
            vec!["clock".into(), "processes".into()],
        ]
    }

    pub fn preset_aesthetic() -> Vec<Vec<String>> {
        vec![
            vec!["clock".into(), "weather".into(), "media".into()],
            vec!["visualizer".into(), "status".into()],
        ]
    }

    pub fn preset_workspace() -> Vec<Vec<String>> {
        vec![
            vec!["cpu".into(), "memory".into(), "network".into()],
            vec!["clock".into(), "media".into(), "processes".into()],
            vec!["notes".into(), "files".into()],
        ]
    }

    pub fn apply_preset(&mut self, preset_name: &str) {
        self.preset = preset_name.to_string();
        match preset_name {
            "cockpit" => self.layout = Self::preset_cockpit(),
            "minimal" => self.layout = Self::preset_minimal(),
            "aesthetic" => self.layout = Self::preset_aesthetic(),
            "workspace" => self.layout = Self::preset_workspace(),
            _ => {} // For custom, we just leave the layout as-is
        }
    }
}

impl Default for DashboardConfig {
    fn default() -> Self {
        Self {
            preset: "cockpit".to_string(),
            layout: Self::preset_cockpit(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct UiConfig {
    /// Seconds between data samples (CPU, memory, processes, ...).
    pub refresh_rate: f64,
    /// Render frames per second. Animations (visualizer, matrix, donut) run at this rate.
    pub fps: u32,
    pub theme: String,
    pub startup_mode: String,
    pub obsidian_vault: String,
    /// 24-hour clock (false = 12-hour with am/pm).
    pub clock_24h: bool,
    /// Clock font: standard | rounded | digital
    pub clock_font: String,
    pub pinned_media_path: String,
    /// Clock style (fill): solid | dotted | hollow
    pub clock_style: String,
    pub timezones: Vec<String>,
    /// Visualizer style: bars | mirror | wave | peaks.
    pub visualizer: String,
    /// Gauge style: arc | bars | vertical.
    pub gauge_style: String,
    /// History graph style: block | braille.
    pub graph_style: String,
}

impl Default for UiConfig {
    fn default() -> Self {
        Self {
            refresh_rate: 0.5,
            fps: 30,
            theme: "dark".to_string(),
            startup_mode: "dashboard".to_string(),
            obsidian_vault: "~".to_string(),
            clock_24h: true,
            clock_font: "standard".to_string(),
            pinned_media_path: "".to_string(),
            clock_style: "solid".to_string(),
            timezones: vec![
                "UTC".to_string(),
                "America/New_York".to_string(),
                "Asia/Tokyo".to_string(),
            ],
            visualizer: "bars".to_string(),
            gauge_style: "arc".to_string(),
            graph_style: "block".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct WidgetConfig {
    pub cpu: bool,
    pub memory: bool,
    pub disk: bool,
    pub network: bool,
    pub gpu: bool,
    pub clock: bool,
    pub calendar: bool,
    pub music_viz: bool,
    pub processes: bool,
    pub media: bool,
    pub matrix: bool,
    pub video: bool,
    pub pinned_media: bool,
    pub weather: bool,
    pub tasks: bool,
    pub agenda: bool,
    pub news: bool,
    pub news_feed: String,
}

impl Default for WidgetConfig {
    fn default() -> Self {
        Self {
            cpu: true,
            memory: true,
            disk: true,
            network: true,
            gpu: true,
            clock: true,
            calendar: true,
            music_viz: true,
            processes: true,
            media: true,
            matrix: true,
            video: true,
            pinned_media: true,
            weather: true,
            tasks: true,
            agenda: true,
            news: true,
            news_feed: "https://news.ycombinator.com/rss".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let content = std::fs::read_to_string(config_path()).unwrap_or_default();
        let mut cfg: Config = toml::from_str(&content).unwrap_or_default();
        cfg.ui.refresh_rate = cfg.ui.refresh_rate.clamp(0.1, 10.0);
        cfg.ui.fps = cfg.ui.fps.clamp(5, 120);
        if cfg.dashboard.layout.is_empty() {
            cfg.dashboard = DashboardConfig::default();
        }
        cfg
    }

    /// Persist only the runtime-mutable fields (`[ui]` and `[widgets]`).
    ///
    /// The `[[custom_widgets]]` array is entirely user-authored. Rewriting the
    /// whole file with `toml::to_string(self)` would serialise it as a flat
    /// inline array and destroy the user's hand-written table entries on every
    /// theme change or page switch. Instead we:
    ///   1. Parse the existing file into a `toml::Value` document tree.
    ///   2. Overwrite only the `[ui]` and `[widgets]` tables in that tree.
    ///   3. Write the mutated tree back — `[[custom_widgets]]` and any comments
    ///      survive untouched.
    pub fn save(&self) {
        let path = config_path();
        if let Some(parent) = std::path::Path::new(&path).parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        // Serialise only the mutable sections.
        let Ok(ui_val) = toml::Value::try_from(&self.ui) else {
            return;
        };
        let Ok(widgets_val) = toml::Value::try_from(&self.widgets) else {
            return;
        };
        let Ok(dashboard_val) = toml::Value::try_from(&self.dashboard) else {
            return;
        };

        // Load the existing document so we can do a surgical update.
        let existing = std::fs::read_to_string(&path).unwrap_or_default();
        let mut doc: toml::Value =
            toml::from_str(&existing).unwrap_or(toml::Value::Table(toml::map::Map::new()));

        if let toml::Value::Table(ref mut map) = doc {
            map.insert("ui".to_string(), ui_val);
            map.insert("widgets".to_string(), widgets_val);
            map.insert("dashboard".to_string(), dashboard_val);
            // Deliberately do NOT touch "custom_widgets".
        }

        if let Ok(out) = toml::to_string(&doc) {
            let _ = std::fs::write(&path, out);
        }
    }
}

pub fn config_path() -> String {
    if let Ok(xdg) = std::env::var("XDG_CONFIG_HOME") {
        format!("{}/vanta/config.toml", xdg)
    } else if let Ok(home) = std::env::var("HOME") {
        format!("{}/.config/vanta/config.toml", home)
    } else {
        "config.toml".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dashboard_config_defaults() {
        let toml_str = r#"
            [ui]
            fps = 60
        "#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.dashboard.layout.len(), 3);
        assert_eq!(
            cfg.dashboard.layout[0],
            vec!["system", "gauges", "cpu", "storage"]
        );
        assert_eq!(
            cfg.dashboard.layout[1],
            vec!["clock", "media", "visualizer", "processes"]
        );
        assert_eq!(
            cfg.dashboard.layout[2],
            vec!["status", "weather", "memory", "network", "calendar"]
        );
    }

    #[test]
    fn test_dashboard_config_custom_layout() {
        let toml_str = r#"
            [dashboard]
            layout = [
                ["clock", "system"],
                ["processes", "weather"]
            ]
        "#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.dashboard.layout.len(), 2);
        assert_eq!(cfg.dashboard.layout[0], vec!["clock", "system"]);
        assert_eq!(cfg.dashboard.layout[1], vec!["processes", "weather"]);
    }
}
