use chrono::{DateTime, Utc};
use rss::Channel;
use std::io::BufReader;
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Default)]
pub struct NewsItem {
    pub title: String,
    pub published_at: Option<DateTime<Utc>>,
}

#[derive(Clone, Default)]
pub struct NewsSnapshot {
    pub channel_title: String,
    pub items: Vec<NewsItem>,
    pub ready: bool,
}

static SNAP: LazyLock<Mutex<NewsSnapshot>> = LazyLock::new(|| Mutex::new(NewsSnapshot::default()));

pub fn snapshot() -> NewsSnapshot {
    SNAP.lock().unwrap().clone()
}

pub fn start(feed_url: String) {
    std::thread::spawn(move || loop {
        if let Ok(res) = ureq::get(&feed_url).call() {
            let body = res.into_body().read_to_vec().unwrap_or_default();
            if let Ok(channel) = Channel::read_from(BufReader::new(body.as_slice())) {
                let mut items = Vec::new();
                for item in channel.items().iter().take(10) {
                    let title = item.title().unwrap_or("No Title").to_string();
                    let published_at = item
                        .pub_date()
                        .and_then(|d| DateTime::parse_from_rfc2822(d).ok())
                        .map(|d| d.with_timezone(&Utc));
                    items.push(NewsItem {
                        title,
                        published_at,
                    });
                }

                *SNAP.lock().unwrap() = NewsSnapshot {
                    channel_title: channel.title().to_string(),
                    items,
                    ready: true,
                };
            }
        }
        std::thread::sleep(std::time::Duration::from_secs(900)); // Refresh every 15 mins
    });
}
