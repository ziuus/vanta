use serde::{Deserialize, Serialize};

use crate::custom::config::CustomWidgetConfig;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub ui: UiConfig,
    pub widgets: WidgetConfig,
    pub dashboard: DashboardConfig,
    pub extensions: Option<toml::Value>,
    /// User-defined custom widgets declared via `[[custom_widgets]]` entries.
    /// Existing configs that omit this field load fine — serde defaults to an
    /// empty `Vec`.
    pub custom_widgets: Vec<CustomWidgetConfig>,
    /// User-defined custom pages declared via `[[pages]]`.
    #[serde(default)]
    pub pages: Vec<CustomPageConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CustomPageConfig {
    pub name: String,
    pub layout: Vec<Vec<String>>,
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
        let col = |names: &[&str]| names.iter().map(|s| s.to_string()).collect();
        vec![
            col(&["system", "gauges", "cpu", "memory", "storage"]),
            col(&["clock", "media", "processes", "network"]),
            col(&["weather", "calendar", "upnext", "status"]),
        ]
    }

    /// The cockpit layout shipped before v0.11. Configs that still hold it
    /// verbatim were never customised, so they're upgraded on load.
    fn legacy_cockpit() -> Vec<Vec<String>> {
        let col = |names: &[&str]| names.iter().map(|s| s.to_string()).collect();
        vec![
            col(&["system", "gauges", "cpu", "storage"]),
            col(&["clock", "media", "visualizer", "processes"]),
            col(&["status", "weather", "memory", "network", "calendar"]),
        ]
    }

    pub fn preset_minimal() -> Vec<Vec<String>> {
        vec![
            vec!["clock".into(), "weather".into()],
            vec!["system".into(), "cpu".into()],
        ]
    }

    pub fn preset_monitoring() -> Vec<Vec<String>> {
        vec![
            vec!["system".into(), "gauges".into(), "cpu".into()],
            vec![
                "memory".into(),
                "network".into(),
                "gpu".into(),
                "storage".into(),
            ],
            vec!["processes".into()],
        ]
    }

    pub fn preset_aesthetic() -> Vec<Vec<String>> {
        vec![
            vec!["clock".into(), "weather".into(), "media".into()],
            vec!["donut".into(), "matrix".into()],
            vec!["calendar".into(), "visualizer".into()],
        ]
    }

    pub fn preset_workspace() -> Vec<Vec<String>> {
        vec![
            vec!["clock".into(), "agenda".into(), "tasks".into()],
            vec!["notes".into()],
            vec!["files".into(), "processes".into()],
        ]
    }

    /// Fill an empty layout and upgrade an untouched legacy cockpit.
    fn migrate(&mut self) {
        if self.layout.is_empty() || self.layout == Self::legacy_cockpit() {
            self.layout = Self::preset_cockpit();
        }
    }

    pub fn apply_preset(&mut self, preset_name: &str) {
        self.preset = preset_name.to_string();
        match preset_name {
            "cockpit" => self.layout = Self::preset_cockpit(),
            "minimal" => self.layout = Self::preset_minimal(),
            "monitoring" => self.layout = Self::preset_monitoring(),
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
    /// Gauge style: arc | bars | vertical | dots | braille.
    pub gauge_style: String,
    /// History graph style: block | braille.
    pub graph_style: String,
    /// Meter style: block | braille | dots | ascii.
    #[serde(default)]
    pub meter_style: String,
    /// Motion enabled for 3D/animations (true | false)
    #[serde(default = "default_motion_enabled")]
    pub motion_enabled: bool,
    /// Motion speed multiplier (0.25 to 3.0)
    #[serde(default = "default_motion_speed")]
    pub motion_speed: f32,
    /// Motion mode: spin | tumble | wobble | swing
    #[serde(default = "default_motion_mode")]
    pub motion_mode: String,
    /// Transparent background
    pub transparent: Option<bool>,
    /// Seconds between Ambient scene changes (0 = stay on one scene).
    #[serde(default = "default_ambient_rotate_secs")]
    pub ambient_rotate_secs: u64,
    /// Pomodoro lengths in minutes.
    #[serde(default = "default_focus_minutes")]
    pub focus_minutes: u64,
    #[serde(default = "default_break_minutes")]
    pub break_minutes: u64,
    #[serde(default = "default_long_break_minutes")]
    pub long_break_minutes: u64,
    /// Dim every colour during these hours: "22:00-07:00", "always", or ""
    /// (off). Wraps past midnight.
    pub night_hours: String,
}

/// Presets offered in the settings menu, in cycle order.
pub const NIGHT_PRESETS: [&str; 6] = [
    "",
    "21:00-07:00",
    "22:00-07:00",
    "23:00-07:00",
    "00:00-07:00",
    "always",
];

fn parse_hhmm(s: &str) -> Option<u32> {
    let (h, m) = s.trim().split_once(':')?;
    let (h, m): (u32, u32) = (h.parse().ok()?, m.parse().ok()?);
    (h < 24 && m < 60).then_some(h * 60 + m)
}

impl UiConfig {
    /// Whether night dimming applies at `minute` (minutes since midnight).
    pub fn night_at(&self, minute: u32) -> bool {
        let spec = self.night_hours.trim();
        if spec == "always" {
            return true;
        }
        let Some((a, b)) = spec
            .split_once('-')
            .and_then(|(a, b)| Some((parse_hhmm(a)?, parse_hhmm(b)?)))
        else {
            return false;
        };
        if a <= b {
            (a..b).contains(&minute)
        } else {
            minute >= a || minute < b
        }
    }
}

fn default_ambient_rotate_secs() -> u64 {
    300
}
fn default_focus_minutes() -> u64 {
    25
}
fn default_break_minutes() -> u64 {
    5
}
fn default_long_break_minutes() -> u64 {
    15
}

fn default_motion_enabled() -> bool {
    true
}

fn default_motion_speed() -> f32 {
    1.0
}

fn default_motion_mode() -> String {
    "spin".to_string()
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
            meter_style: "block".to_string(),
            motion_enabled: true,
            motion_speed: 1.0,
            motion_mode: "spin".to_string(),
            transparent: None,
            ambient_rotate_secs: default_ambient_rotate_secs(),
            focus_minutes: default_focus_minutes(),
            break_minutes: default_break_minutes(),
            long_break_minutes: default_long_break_minutes(),
            night_hours: String::new(),
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
        clean_orphaned_extensions();
        let content = std::fs::read_to_string(config_path()).unwrap_or_default();
        let mut cfg: Config = toml::from_str(&content).unwrap_or_default();
        cfg.ui.refresh_rate = cfg.ui.refresh_rate.clamp(0.1, 10.0);
        cfg.ui.fps = cfg.ui.fps.clamp(5, 120);
        cfg.dashboard.migrate();
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

pub fn prune_components_from_enabled_str(content: &str, to_remove: &[&str]) -> String {
    let Some(start_idx) = content.find("enabled = [") else {
        return content.to_string();
    };
    let after_bracket = start_idx + "enabled = [".len();
    let Some(rel_end) = content[after_bracket..].find(']') else {
        return content.to_string();
    };
    let end_idx = after_bracket + rel_end;

    let array_content = &content[after_bracket..end_idx];
    let items: Vec<&str> = array_content
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    let mut kept = Vec::new();
    for item in items {
        let clean = item.trim_matches(|c| c == '"' || c == '\'' || c == ' ');
        if !to_remove.iter().any(|r| r.eq_ignore_ascii_case(clean)) {
            kept.push(item);
        }
    }

    let new_array_content = if kept.is_empty() {
        "".to_string()
    } else {
        kept.join(", ")
    };

    format!(
        "{}{}{}",
        &content[..after_bracket],
        new_array_content,
        &content[end_idx..]
    )
}

pub fn remove_page_block_from_str(content: &str, page_name: &str) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines: Vec<&str> = Vec::new();

    let mut current_block: Vec<&str> = Vec::new();
    let mut in_pages_block = false;
    let mut block_matches_target = false;

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with('[') {
            if in_pages_block {
                if !block_matches_target {
                    result_lines.extend(current_block);
                }
                current_block = Vec::new();
                in_pages_block = false;
                block_matches_target = false;
            }
            if trimmed == "[[pages]]" {
                in_pages_block = true;
                current_block.push(line);
                continue;
            }
        }

        if in_pages_block {
            if trimmed.starts_with("name") {
                if let Some((_k, v)) = trimmed.split_once('=') {
                    let clean_val = v.trim().trim_matches(|c| c == '"' || c == '\'');
                    if clean_val.eq_ignore_ascii_case(page_name) {
                        block_matches_target = true;
                    }
                }
            }
            current_block.push(line);
        } else {
            result_lines.push(line);
        }
    }

    if in_pages_block && !block_matches_target {
        result_lines.extend(current_block);
    }

    let mut out = result_lines.join("\n");
    if content.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn reset_startup_mode_if_matches_str(content: &str, targets: &[&str]) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut result_lines = Vec::new();

    for line in lines {
        let trimmed = line.trim();
        if trimmed.starts_with("startup_mode") {
            if let Some((_k, v)) = trimmed.split_once('=') {
                let clean_val = v.trim().trim_matches(|c| c == '"' || c == '\'');
                if targets.iter().any(|t| t.eq_ignore_ascii_case(clean_val)) {
                    let indent = line.len() - line.trim_start().len();
                    let spaces = &line[..indent];
                    result_lines.push(format!("{}startup_mode = \"dashboard\"", spaces));
                    continue;
                }
            }
        }
        result_lines.push(line.to_string());
    }

    let mut out = result_lines.join("\n");
    if content.ends_with('\n') && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn remove_page_from_config(page_name: &str, components: &[String]) -> Result<(), String> {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let comp_refs: Vec<&str> = components.iter().map(|s| s.as_str()).collect();
    let mut targets = vec![page_name];
    targets.extend(&comp_refs);

    let content = remove_page_block_from_str(&content, page_name);
    let content = prune_components_from_enabled_str(&content, &comp_refs);
    let new_content = reset_startup_mode_if_matches_str(&content, &targets);

    std::fs::write(&path, new_content)
        .map_err(|e| format!("Failed to write config.toml: {}", e))?;
    Ok(())
}

pub fn remove_component_from_config(comp_id: &str) -> Result<(), String> {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return Ok(()),
    };

    let targets = [comp_id];
    let content = prune_components_from_enabled_str(&content, &targets);
    let new_content = reset_startup_mode_if_matches_str(&content, &targets);

    std::fs::write(&path, new_content)
        .map_err(|e| format!("Failed to write config.toml: {}", e))?;
    Ok(())
}

pub fn clean_orphaned_extensions() {
    let path = config_path();
    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => return,
    };

    let ext_dir = match directories::ProjectDirs::from("", "", "vanta") {
        Some(p) => p.config_dir().join("extensions"),
        None => return,
    };

    let cfg: Config = match toml::from_str(&content) {
        Ok(c) => c,
        Err(_) => return,
    };

    let mut orphaned_comps = Vec::new();
    if let Some(ext_val) = &cfg.extensions {
        if let Some(arr) = ext_val.get("enabled").and_then(|v| v.as_array()) {
            for item in arr {
                if let Some(comp_id) = item.as_str() {
                    let wasm_file = ext_dir.join(format!("{}.wasm", comp_id));
                    if !wasm_file.exists() {
                        orphaned_comps.push(comp_id.to_string());
                    }
                }
            }
        }
    }

    let mut orphaned_pages = Vec::new();
    for page in &cfg.pages {
        let all_widgets: Vec<&str> = page
            .layout
            .iter()
            .flat_map(|col| col.iter().map(|s| s.as_str()))
            .collect();
        let has_any_valid = all_widgets.iter().any(|w| {
            matches!(
                w.to_lowercase().as_str(),
                "system"
                    | "gauges"
                    | "gauge"
                    | "cpu"
                    | "storage"
                    | "disk"
                    | "memory"
                    | "network"
                    | "gpu"
                    | "processes"
                    | "process"
                    | "clock"
                    | "calendar"
                    | "weather"
                    | "media"
                    | "music_viz"
                    | "visualizer"
                    | "matrix"
                    | "tasks"
                    | "agenda"
                    | "news"
                    | "status"
                    | "files"
                    | "notes"
                    | "donut"
            ) || cfg
                .custom_widgets
                .iter()
                .any(|c| c.id.eq_ignore_ascii_case(w))
                || ext_dir.join(format!("{}.wasm", w)).exists()
        });

        if !has_any_valid && !all_widgets.is_empty() {
            orphaned_pages.push(page.name.clone());
        }
    }

    if orphaned_comps.is_empty() && orphaned_pages.is_empty() {
        return;
    }

    let mut updated_content = content;
    for page_name in &orphaned_pages {
        updated_content = remove_page_block_from_str(&updated_content, page_name);
        updated_content = reset_startup_mode_if_matches_str(&updated_content, &[page_name]);
    }

    let comp_refs: Vec<&str> = orphaned_comps.iter().map(|s| s.as_str()).collect();
    updated_content = prune_components_from_enabled_str(&updated_content, &comp_refs);
    updated_content = reset_startup_mode_if_matches_str(&updated_content, &comp_refs);

    let _ = std::fs::write(&path, updated_content);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn night_hours_wrap_past_midnight() {
        let ui = |s: &str| UiConfig {
            night_hours: s.to_string(),
            ..Default::default()
        };
        let at = |h: u32, m: u32| h * 60 + m;
        let n = ui("22:00-07:00");
        assert!(n.night_at(at(22, 0)) && n.night_at(at(3, 0)) && n.night_at(at(6, 59)));
        assert!(!n.night_at(at(7, 0)) && !n.night_at(at(21, 59)) && !n.night_at(at(12, 0)));
        let d = ui("01:00-05:30");
        assert!(d.night_at(at(1, 0)) && !d.night_at(at(5, 30)) && !d.night_at(at(0, 59)));
        assert!(ui("always").night_at(at(12, 0)));
        for off in ["", "nonsense", "25:00-07:00"] {
            assert!(!ui(off).night_at(at(23, 0)), "{off:?}");
        }
    }

    #[test]
    fn test_dashboard_config_defaults() {
        let toml_str = r#"
            [ui]
            fps = 60
        "#;
        let cfg: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(cfg.dashboard.layout, DashboardConfig::preset_cockpit());
        assert_eq!(
            cfg.dashboard.layout[2],
            vec!["weather", "calendar", "upnext", "status"]
        );
    }

    #[test]
    fn legacy_cockpit_upgrades_but_custom_layouts_stay() {
        let mut legacy = DashboardConfig {
            preset: "cockpit".into(),
            layout: DashboardConfig::legacy_cockpit(),
        };
        legacy.migrate();
        assert_eq!(legacy.layout, DashboardConfig::preset_cockpit());

        let mut custom_layout = DashboardConfig::legacy_cockpit();
        custom_layout[0].push("gpu".into());
        let mut custom = DashboardConfig {
            preset: "cockpit".into(),
            layout: custom_layout.clone(),
        };
        custom.migrate();
        assert_eq!(custom.layout, custom_layout);
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

    #[test]
    fn test_prune_components_from_enabled_str() {
        let sample = r#"[extensions]
enabled = ["comp_a", "comp_b", "comp_c"]
"#;
        let pruned = prune_components_from_enabled_str(sample, &["comp_b"]);
        assert_eq!(
            pruned,
            r#"[extensions]
enabled = ["comp_a", "comp_c"]
"#
        );

        let pruned_all = prune_components_from_enabled_str(sample, &["comp_a", "comp_b", "comp_c"]);
        assert_eq!(
            pruned_all,
            r#"[extensions]
enabled = []
"#
        );
    }

    #[test]
    fn test_remove_page_block_from_str() {
        let sample = r#"[[pages]]
layout = [["a", "b"]]
name = "First Page"

[[pages]]
layout = [["c", "d"]]
name = "Second Page"

[ui]
theme = "dark"
"#;
        let result = remove_page_block_from_str(sample, "Second Page");
        assert!(!result.contains("Second Page"));
        assert!(result.contains("First Page"));
        assert!(result.contains("[ui]"));
    }

    #[test]
    fn test_reset_startup_mode_if_matches_str() {
        let sample = r#"[ui]
startup_mode = "CryptoPulse Terminal"
theme = "dark"
"#;
        let result = reset_startup_mode_if_matches_str(sample, &["CryptoPulse Terminal"]);
        assert!(result.contains("startup_mode = \"dashboard\""));
        assert!(result.contains("theme = \"dark\""));
    }
}
