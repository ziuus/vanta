use serde::{Deserialize, Serialize};

/// Dashboard modes — keyboard switching via 1-6.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DashboardMode {
    Overview,
    Monitor,
    Processes,
    Media,
    Aesthetic,
    Settings,
}

impl DashboardMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Monitor => "Monitor",
            Self::Processes => "Processes",
            Self::Media => "Media",
            Self::Aesthetic => "Aesthetic",
            Self::Settings => "Settings",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "monitor" => Self::Monitor,
            "processes" => Self::Processes,
            "media" => Self::Media,
            "aesthetic" => Self::Aesthetic,
            "settings" => Self::Settings,
            _ => Self::Overview,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Monitor => "monitor",
            Self::Processes => "processes",
            Self::Media => "media",
            Self::Aesthetic => "aesthetic",
            Self::Settings => "settings",
        }
    }

    /// Hotkey used to select this mode.
    pub fn hotkey(&self) -> char {
        match self {
            Self::Overview => '1',
            Self::Monitor => '2',
            Self::Processes => '3',
            Self::Media => '4',
            Self::Aesthetic => '5',
            Self::Settings => '6',
        }
    }
}
