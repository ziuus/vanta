use serde::{Deserialize, Serialize};

/// Dashboard modes — keyboard switching via 1-3
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DashboardMode {
    Overview,
    Monitor,
    Aesthetic,
}

impl DashboardMode {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Overview => "Overview",
            Self::Monitor => "Monitor",
            Self::Aesthetic => "Aesthetic",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "monitor" => Self::Monitor,
            "aesthetic" => Self::Aesthetic,
            _ => Self::Overview,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Monitor => "monitor",
            Self::Aesthetic => "aesthetic",
        }
    }
}
