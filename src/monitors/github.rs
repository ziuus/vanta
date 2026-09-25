use serde::{Deserialize, Serialize};
use std::process::Command;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Serialize, Deserialize, Clone)]
pub struct GithubStats {
    pub username: String,
    pub total_contributions: usize,
    pub week_contributions: usize,
}

static CACHE: Mutex<Option<(Instant, GithubStats)>> = Mutex::new(None);

pub fn snapshot() -> Option<GithubStats> {
    let mut cache = CACHE.lock().unwrap();
    if let Some((time, stats)) = &*cache {
        if time.elapsed() < Duration::from_secs(3600) {
            return Some(stats.clone());
        }
    }

    // Try to fetch
    if let Ok(output) = Command::new("gh").args(["api", "graphql", "-f", "query={ viewer { login contributionsCollection { contributionCalendar { totalContributions weeks { contributionDays { contributionCount } } } } } }"]).output() {
        if output.status.success() {
            if let Ok(json) = serde_json::from_slice::<serde_json::Value>(&output.stdout) {
                if let Some(viewer) = json.get("data").and_then(|d| d.get("viewer")) {
                    let username = viewer.get("login").and_then(|v| v.as_str()).unwrap_or("").to_string();
                    let total = viewer.get("contributionsCollection")
                        .and_then(|c| c.get("contributionCalendar"))
                        .and_then(|c| c.get("totalContributions"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(0) as usize;

                    *cache = Some((Instant::now(), GithubStats {
                        username,
                        total_contributions: total,
                        week_contributions: 0, // Simplified for this implementation
                    }));
                    return cache.as_ref().map(|(_, s)| s.clone());
                }
            }
        }
    }
    None
}
