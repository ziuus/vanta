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
}

fn weather_store() -> &'static Arc<RwLock<WeatherSnapshot>> {
    static STORE: OnceLock<Arc<RwLock<WeatherSnapshot>>> = OnceLock::new();
    STORE.get_or_init(|| Arc::new(RwLock::new(WeatherSnapshot::default())))
}

pub fn snapshot() -> WeatherSnapshot {
    weather_store().read().unwrap().clone()
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
}

#[derive(Deserialize)]
struct CurrentWeather {
    temperature_2m: f32,
    weather_code: u8,
    is_day: u8,
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
                "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code,is_day",
                lat, lon
            );

            if let Ok(res) = ureq::get(&url).call() {
                if let Ok(text) = res.into_body().read_to_string() {
                    if let Ok(json) = serde_json::from_str::<OpenMeteoResp>(&text) {
                        let mut w = weather_store().write().unwrap();
                        w.ready = true;
                        w.temp_c = json.current.temperature_2m;
                        w.condition_code = json.current.weather_code;
                        w.is_day = json.current.is_day == 1;
                        w.location = city.clone();
                    }
                }
            }

            thread::sleep(Duration::from_secs(15 * 60));
        }
    });
}
