use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Default, Clone, Debug)]
pub struct GithubSnapshot {
    pub contributions: Vec<usize>,
    pub streak: usize,
    pub today: usize,
}

static STATE: LazyLock<Mutex<GithubSnapshot>> =
    LazyLock::new(|| Mutex::new(GithubSnapshot::default()));
static LAST_UPDATE: LazyLock<Mutex<Option<Instant>>> = LazyLock::new(|| Mutex::new(None));

pub fn snapshot() -> GithubSnapshot {
    let mut last = LAST_UPDATE.lock().unwrap();
    let now = Instant::now();

    if last.is_none() || now.duration_since(last.unwrap()) > Duration::from_secs(1800) {
        *last = Some(now);
        thread::spawn(|| {
            let res = Command::new("gh")
                .arg("api")
                .arg("graphql")
                .arg("-F")
                .arg("query=query { viewer { contributionsCollection { contributionCalendar { weeks { contributionDays { contributionCount date } } } } } }")
                .output();

            if let Ok(out) = res {
                if out.status.success() {
                    if let Ok(parsed) = serde_json::from_slice::<serde_json::Value>(&out.stdout) {
                        let mut conts = Vec::new();

                        if let Some(weeks) = parsed["data"]["viewer"]["contributionsCollection"]
                            ["contributionCalendar"]["weeks"]
                            .as_array()
                        {
                            for week in weeks {
                                if let Some(days) = week["contributionDays"].as_array() {
                                    for day in days {
                                        if let Some(count) = day["contributionCount"].as_u64() {
                                            conts.push(count as usize);
                                        }
                                    }
                                }
                            }
                        }

                        let mut today = 0;
                        let mut current_streak = 0;

                        if !conts.is_empty() {
                            today = *conts.last().unwrap();
                            for i in (0..conts.len()).rev() {
                                if conts[i] > 0 {
                                    current_streak += 1;
                                } else if i == conts.len() - 1 {
                                    // if today is 0, streak might still be alive from yesterday
                                } else {
                                    break;
                                }
                            }
                        }

                        let mut st = STATE.lock().unwrap();
                        st.contributions = conts;
                        st.streak = current_streak;
                        st.today = today;
                    }
                }
            }
        });
    }

    STATE.lock().unwrap().clone()
}
