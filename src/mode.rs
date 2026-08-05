use serde::{Deserialize, Serialize};

/// Dashboard pages — keyboard switching via 1-3.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DashboardMode {
    /// All-in-one: monitoring overview + clock/calendar/media/visualizer.
    Dashboard,
    /// btop-style detail: large graphs + full process table.
    Monitor,
    /// Pure eye candy: clock, calendar, visualizer, matrix rain, 3D demo.
    Aesthetic,
}

impl DashboardMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Monitor => "Monitor",
            Self::Aesthetic => "Aesthetic",
        }
    }

    /// Parse from config. Old mode names map to the nearest new page so
    /// existing config.toml files keep working.
    pub fn from_str(s: &str) -> Self {
        match s {
            // New names
            "dashboard" => Self::Dashboard,
            "monitor" => Self::Monitor,
            "aesthetic" => Self::Aesthetic,
            // Legacy names → nearest page
            "processes" => Self::Monitor,
            "media" => Self::Aesthetic,
            _ => Self::Dashboard, // "overview", "settings", unknown
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Monitor => "monitor",
            Self::Aesthetic => "aesthetic",
        }
    }

    /// Hotkey used to select this page.
    pub fn hotkey(&self) -> char {
        match self {
            Self::Dashboard => '1',
            Self::Monitor => '2',
            Self::Aesthetic => '3',
        }
    }
}
