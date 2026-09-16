use serde::{Deserialize, Serialize};

/// Dashboard pages — keyboard switching via numbers.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DashboardMode {
    Dashboard,
    Monitor,
    Aesthetic,
    Workspace,
    Extension(String),
}

impl DashboardMode {
    pub fn label(&self) -> &str {
        match self {
            Self::Dashboard => "Dashboard",
            Self::Monitor => "Monitor",
            Self::Aesthetic => "Aesthetic",
            Self::Workspace => "Workspace",
            Self::Extension(name) => name,
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "dashboard" => Self::Dashboard,
            "monitor" => Self::Monitor,
            "aesthetic" => Self::Aesthetic,
            "workspace" | "writer" => Self::Workspace,
            "processes" => Self::Monitor,
            "media" => Self::Aesthetic,
            // If it's none of the above, we map it to Extension (or fallback to Dashboard if needed).
            // For safety, we fallback to Dashboard for unknown ones right now.
            other => Self::Extension(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Dashboard => "dashboard",
            Self::Monitor => "monitor",
            Self::Aesthetic => "aesthetic",
            Self::Workspace => "workspace",
            Self::Extension(name) => name,
        }
    }
}
