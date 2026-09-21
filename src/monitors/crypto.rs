use serde::{Deserialize, Serialize};
use std::sync::{LazyLock, Mutex};

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct CryptoAsset {
    pub symbol: String,
    pub price: f64,
    pub change_24h_pct: f64,
    pub volume_24h: f64,
    pub high_24h: f64,
    pub low_24h: f64,
}

#[derive(Clone, Default, Serialize, Deserialize, Debug)]
pub struct CryptoSnapshot {
    pub assets: Vec<CryptoAsset>,
    pub fear_and_greed: Option<u32>,
    pub ready: bool,
}

static SNAP: LazyLock<Mutex<CryptoSnapshot>> =
    LazyLock::new(|| Mutex::new(CryptoSnapshot::default()));

pub fn snapshot() -> CryptoSnapshot {
    SNAP.lock().unwrap().clone()
}

#[derive(Deserialize)]
struct BinanceTicker {
    symbol: String,
    #[serde(rename = "lastPrice")]
    last_price: String,
    #[serde(rename = "priceChangePercent")]
    price_change_percent: String,
    #[serde(rename = "quoteVolume")]
    quote_volume: String,
    #[serde(rename = "highPrice")]
    high_price: String,
    #[serde(rename = "lowPrice")]
    low_price: String,
}

#[derive(Deserialize)]
struct FngData {
    value: String,
}

#[derive(Deserialize)]
struct FngResponse {
    data: Vec<FngData>,
}

pub fn start() {
    std::thread::Builder::new()
        .name("vanta-crypto".into())
        .spawn(|| loop {
            let mut snap = CryptoSnapshot {
                ready: true,
                assets: vec![],
                fear_and_greed: None,
            };

            // 1. Fetch Fear and Greed
            if let Ok(res) = ureq::get("https://api.alternative.me/fng/").call() {
                if let Ok(json) = res.into_body().read_json::<FngResponse>() {
                    if let Some(first) = json.data.first() {
                        snap.fear_and_greed = first.value.parse::<u32>().ok();
                    }
                }
            }

            // 2. Fetch Binance Tickers
            if let Ok(res) = ureq::get("https://api.binance.com/api/v3/ticker/24hr").call() {
                if let Ok(tickers) = res.into_body().read_json::<Vec<BinanceTicker>>() {
                    for t in tickers {
                        if t.symbol.ends_with("USDT")
                            && !t.symbol.contains("DOWN")
                            && !t.symbol.contains("UP")
                        {
                            let base = t
                                .symbol
                                .strip_suffix("USDT")
                                .unwrap_or(&t.symbol)
                                .to_string();
                            if let (Ok(p), Ok(c), Ok(v), Ok(h), Ok(l)) = (
                                t.last_price.parse::<f64>(),
                                t.price_change_percent.parse::<f64>(),
                                t.quote_volume.parse::<f64>(),
                                t.high_price.parse::<f64>(),
                                t.low_price.parse::<f64>(),
                            ) {
                                snap.assets.push(CryptoAsset {
                                    symbol: base,
                                    price: p,
                                    change_24h_pct: c,
                                    volume_24h: v,
                                    high_24h: h,
                                    low_24h: l,
                                });
                            }
                        }
                    }
                    snap.assets.sort_by(|a, b| {
                        b.volume_24h
                            .partial_cmp(&a.volume_24h)
                            .unwrap_or(std::cmp::Ordering::Equal)
                    });
                    snap.assets.truncate(100);
                }
            }

            *SNAP.lock().unwrap() = snap;
            std::thread::sleep(std::time::Duration::from_secs(60));
        })
        .expect("spawn crypto thread");
}
