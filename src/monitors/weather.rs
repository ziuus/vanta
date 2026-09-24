use serde::Deserialize;
use std::sync::{Arc, OnceLock, RwLock};
use std::thread;
use std::time::Duration;

#[derive(Clone, Default)]
pub struct WeatherSnapshot {
    pub ready: bool,
    pub temp_c: f32,
    pub condition_code: u8,
    pub is_day: bool,
    pub location: String,
    pub feels_c: f32,
    pub humidity: u8,
    pub wind_kmh: f32,
    pub max_c: Option<f32>,
    pub min_c: Option<f32>,
    /// "HH:MM" local.
    pub sunrise: String,
    pub sunset: String,
    /// Hourly temperatures starting at the current hour.
    pub hourly_c: Vec<f32>,
    /// Hourly precipitation probability (%) aligned with `hourly_c`.
    pub hourly_rain: Vec<u8>,
    /// Local hour-of-day of `hourly_c[0]`.
    pub hour0: u8,
}

fn weather_store() -> &'static Arc<RwLock<WeatherSnapshot>> {
    static STORE: OnceLock<Arc<RwLock<WeatherSnapshot>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(RwLock::new(WeatherSnapshot::default())))
}

pub fn snapshot() -> WeatherSnapshot {
    weather_store().read().unwrap().clone()
}

/// Short human label for a WMO weather code.
pub fn describe(code: u8) -> &'static str {
    match code {
        0 => "Clear",
        1 => "Mostly clear",
        2 => "Partly cloudy",
        3 => "Overcast",
        45 | 48 => "Fog",
        51..=57 => "Drizzle",
        61..=67 => "Rain",
        71..=77 => "Snow",
        80..=82 => "Showers",
        85..=86 => "Snow showers",
        95..=99 => "Thunderstorm",
        _ => "Unknown",
    }
}

#[derive(Deserialize)]
struct IpApiResp {
    lat: f32,
    lon: f32,
    city: String,
}

#[derive(Deserialize)]
struct OpenMeteoResp {
    current: CurrentWeather,
    hourly: Option<Hourly>,
    daily: Option<Daily>,
}

#[derive(Deserialize)]
struct CurrentWeather {
    time: String,
    temperature_2m: f32,
    apparent_temperature: Option<f32>,
    relative_humidity_2m: Option<f32>,
    wind_speed_10m: Option<f32>,
    weather_code: u8,
    is_day: u8,
}

#[derive(Deserialize)]
struct Hourly {
    time: Vec<String>,
    temperature_2m: Vec<f32>,
    precipitation_probability: Option<Vec<Option<f32>>>,
}

#[derive(Deserialize)]
struct Daily {
    temperature_2m_max: Vec<f32>,
    temperature_2m_min: Vec<f32>,
    sunrise: Vec<String>,
    sunset: Vec<String>,
}

/// "2026-09-24T06:12" -> "06:12".
fn hhmm(iso: &str) -> String {
    iso.split_once('T')
        .map(|(_, t)| t.chars().take(5).collect())
        .unwrap_or_default()
}

fn apply(json: OpenMeteoResp, city: &str) -> WeatherSnapshot {
    let c = json.current;
    let mut w = WeatherSnapshot {
        ready: true,
        temp_c: c.temperature_2m,
        condition_code: c.weather_code,
        is_day: c.is_day == 1,
        location: city.to_string(),
        feels_c: c.apparent_temperature.unwrap_or(c.temperature_2m),
        humidity: c.relative_humidity_2m.unwrap_or(0.0).round() as u8,
        wind_kmh: c.wind_speed_10m.unwrap_or(0.0),
        ..Default::default()
    };
    if let Some(d) = json.daily {
        w.max_c = d.temperature_2m_max.first().copied();
        w.min_c = d.temperature_2m_min.first().copied();
        w.sunrise = d.sunrise.first().map(|s| hhmm(s)).unwrap_or_default();
        w.sunset = d.sunset.first().map(|s| hhmm(s)).unwrap_or_default();
    }
    if let Some(h) = json.hourly {
        // Hourly slots are "YYYY-MM-DDTHH:00"; match the current hour.
        let hour_key: String = c.time.chars().take(13).collect();
        let start = h
            .time
            .iter()
            .position(|t| t.starts_with(&hour_key))
            .unwrap_or(0);
        let end = (start + 48).min(h.temperature_2m.len());
        w.hourly_c = h.temperature_2m[start..end].to_vec();
        w.hourly_rain = h
            .precipitation_probability
            .map(|p| {
                p[start..end.min(p.len())]
                    .iter()
                    .map(|v| v.unwrap_or(0.0).round() as u8)
                    .collect()
            })
            .unwrap_or_default();
        w.hour0 = h
            .time
            .get(start)
            .and_then(|t| hhmm(t).get(..2).and_then(|s| s.parse().ok()))
            .unwrap_or(0);
    }
    w
}

pub fn start() {
    thread::spawn(move || {
        let mut lat = 0.0;
        let mut lon = 0.0;
        let mut city = String::new();

        // 1. Resolve location
        if let Ok(res) = ureq::get("http://ip-api.com/json/").call() {
            if let Ok(text) = res.into_body().read_to_string() {
                if let Ok(json) = serde_json::from_str::<IpApiResp>(&text) {
                    lat = json.lat;
                    lon = json.lon;
                    city = json.city;
                }
            }
        }

        if city.is_empty() {
            lat = 51.5074;
            lon = -0.1278;
            city = "London".to_string();
        }

        loop {
            // 2. Fetch weather
            let url = format!(
                "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}\
                 &current=temperature_2m,apparent_temperature,relative_humidity_2m,wind_speed_10m,weather_code,is_day\
                 &hourly=temperature_2m,precipitation_probability\
                 &daily=temperature_2m_max,temperature_2m_min,sunrise,sunset\
                 &timezone=auto&forecast_days=2",
                lat, lon
            );

            let fetched = ureq::get(&url)
                .call()
                .ok()
                .and_then(|res| res.into_body().read_to_string().ok())
                .and_then(|text| serde_json::from_str::<OpenMeteoResp>(&text).ok());
            let ok = fetched.is_some();
            if let Some(json) = fetched {
                *weather_store().write().unwrap() = apply(json, &city);
            }

            // Retry soon after a failure (offline at boot, flaky wifi).
            thread::sleep(Duration::from_secs(if ok { 15 * 60 } else { 60 }));
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_forecast_and_aligns_hourly_to_now() {
        let body = r#"{
            "current": {"time": "2026-09-24T22:45", "temperature_2m": 28.4,
                "apparent_temperature": 31.2, "relative_humidity_2m": 80,
                "wind_speed_10m": 12.5, "weather_code": 3, "is_day": 0},
            "hourly": {"time": ["2026-09-24T21:00","2026-09-24T22:00","2026-09-24T23:00"],
                "temperature_2m": [29.0, 28.0, 27.0],
                "precipitation_probability": [10, 20, null]},
            "daily": {"temperature_2m_max": [31.0], "temperature_2m_min": [25.0],
                "sunrise": ["2026-09-24T06:12"], "sunset": ["2026-09-24T18:20"]}
        }"#;
        let w = apply(serde_json::from_str(body).unwrap(), "Kochi");
        assert_eq!(w.hourly_c, vec![28.0, 27.0]);
        assert_eq!(w.hourly_rain, vec![20, 0]);
        assert_eq!(w.hour0, 22);
        assert_eq!(w.humidity, 80);
        assert_eq!((w.sunrise.as_str(), w.sunset.as_str()), ("06:12", "18:20"));
        assert_eq!(w.max_c, Some(31.0));
    }
}
