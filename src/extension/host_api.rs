//! Host functions exposed to WASM extensions.
//!
//! Extensions run in a sandbox with no filesystem, network, environment or
//! host config (see `docs/EXTENSIONS.md`). Without a host capability they can
//! only render pure computation, which is why telemetry-driven extensions
//! previously had to ship mock data.
//!
//! This module exposes the telemetry Vanta **already samples** on its sampler
//! thread, so extensions read real numbers at no additional collection cost.
//! The wire format is deliberately decoupled from the internal monitor structs:
//! internals stay free to change, the extension API does not.
//!
//! # Protocol
//!
//! One host function, `vanta_query(request_json) -> response_json`.
//!
//! Request: `{"topic": "cpu"}`, with optional per-topic arguments, e.g.
//! `{"topic": "processes", "limit": 20}`.
//!
//! Response: `{"ok": true, "data": {...}}` or `{"ok": false, "error": "..."}`.
//! A plugin should treat a malformed or `ok:false` response as "unavailable"
//! and say so in its UI rather than substituting invented values.
//!
//! Query `{"topic":"capabilities"}` to discover what this host serves; new
//! topics may be added over time, so feature-detect instead of assuming.

use extism::{Function, PTR};
use serde_json::{json, Value};

use crate::monitors;

/// Version of the telemetry wire format. Bump the minor when adding topics or
/// fields, the major only for a breaking change.
pub const TELEMETRY_API_VERSION: &str = "1.0";

/// Topics this host can answer. Keep in sync with `dispatch`.
const TOPICS: &[&str] = &[
    "capabilities",
    "host",
    "summary",
    "cpu",
    "memory",
    "network",
    "disk",
    "gpu",
    "processes",
];

/// Telemetry the host does *not* collect. Reported so extensions can render an
/// honest "unavailable" state instead of guessing, and so it is obvious what a
/// future host collector would unlock.
const UNAVAILABLE: &[(&str, &str)] = &[
    (
        "network.interfaces",
        "host aggregates /proc/net/dev across interfaces; no per-interface data",
    ),
    (
        "network.connections",
        "no socket/connection table is collected",
    ),
    ("containers", "no container runtime integration in host"),
    ("git", "no repository state is collected"),
    ("journal", "no system log/journal access"),
];

/// Build the host function list handed to every plugin.
pub fn functions() -> Vec<Function> {
    vec![Function::new(
        "vanta_query",
        [PTR],
        [PTR],
        extism::UserData::new(()),
        |plugin: &mut extism::CurrentPlugin,
         inputs: &[extism::Val],
         outputs: &mut [extism::Val],
         _ud: extism::UserData<()>|
         -> Result<(), extism::Error> {
            let request: String = plugin.memory_get_val(&inputs[0])?;
            let response = answer(&request);
            let handle = plugin.memory_new(&response)?;
            outputs[0] = plugin.memory_to_val(handle);
            Ok(())
        },
    )]
}

/// Parse a request and produce the response body. Never panics: a bad request
/// becomes an error response so a buggy plugin cannot take the host down.
fn answer(request: &str) -> String {
    let parsed: Value = match serde_json::from_str(request) {
        Ok(v) => v,
        // Bare topic strings are accepted as a convenience: `"cpu"`.
        Err(_) => json!({ "topic": request.trim().trim_matches('"') }),
    };
    let topic = parsed.get("topic").and_then(Value::as_str).unwrap_or("");
    let body = match dispatch(topic, &parsed) {
        Some(data) => json!({ "ok": true, "data": data }),
        None => json!({
            "ok": false,
            "error": format!("unknown topic: {topic}"),
            "topics": TOPICS,
        }),
    };
    body.to_string()
}

fn dispatch(topic: &str, args: &Value) -> Option<Value> {
    Some(match topic {
        "capabilities" => json!({
            "telemetry_api": TELEMETRY_API_VERSION,
            "host_version": env!("CARGO_PKG_VERSION"),
            "topics": TOPICS,
            "unavailable": UNAVAILABLE
                .iter()
                .map(|(k, why)| json!({ "topic": k, "reason": why }))
                .collect::<Vec<_>>(),
        }),
        "host" => json!({
            "version": env!("CARGO_PKG_VERSION"),
            "telemetry_api": TELEMETRY_API_VERSION,
        }),
        "summary" => {
            let s = monitors::summary();
            json!({
                "cpu_pct": s.cpu_pct,
                "mem_pct": s.mem_pct,
                "gpu_pct": s.gpu_pct,
                "disk_pct": s.disk_pct,
                "rx_kbps": s.rx_kbps,
                "tx_kbps": s.tx_kbps,
                "battery_pct": s.battery.map(|(p, _)| p),
                "battery_charging": s.battery.map(|(_, c)| c),
                "uptime": s.uptime,
                "temp_c": s.temp_c,
            })
        }
        "cpu" => {
            let c = monitors::cpu::snapshot();
            json!({
                "usage_pct": c.usage,
                "cores": c.cores,
                "core_count": c.cores.len(),
                "load1": c.load.0,
                "load5": c.load.1,
                "load15": c.load.2,
                "freq_mhz": c.freq_mhz,
                "temps_c": c.temps,
                "max_temp_c": c.max_temp(),
            })
        }
        "memory" => {
            let m = monitors::memory::snapshot();
            json!({
                "total_bytes": m.total,
                "used_bytes": m.used,
                "used_pct": m.pct(),
                "swap_total_bytes": m.swap_total,
                "swap_used_bytes": m.swap_used,
                "swap_used_pct": m.swap_pct(),
            })
        }
        "network" => {
            let n = monitors::network::snapshot();
            json!({
                "rx_kbps": n.rx_kbps,
                "tx_kbps": n.tx_kbps,
                "rx_total_bytes": n.rx_total,
                "tx_total_bytes": n.tx_total,
                "aggregate_only": true,
            })
        }
        "disk" => json!({
            "mounts": monitors::disk::mounts()
                .iter()
                .map(|m| json!({
                    "path": m.path,
                    "device": m.device,
                    "used_bytes": m.used,
                    "total_bytes": m.total,
                    "used_pct": m.pct(),
                }))
                .collect::<Vec<_>>(),
        }),
        "gpu" => match monitors::gpu::snapshot() {
            Some(g) => json!({
                "present": true,
                "name": g.name,
                "util_pct": g.util_pct,
                "temp_c": g.temp_c,
                "mem_used_mb": g.mem_used_mb,
                "mem_total_mb": g.mem_total_mb,
                "freq_mhz": g.freq_mhz,
            }),
            None => json!({ "present": false }),
        },
        "processes" => {
            // Bounded: plugins render inside a 10 ms budget, and the response
            // is serialised on the render thread. 200 is well past what any
            // terminal panel can show.
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(25)
                .clamp(1, 200) as usize;
            json!({
                "total": monitors::processes::count(),
                "returned_by": "cpu_desc",
                "processes": monitors::processes::top_by_cpu(limit)
                    .iter()
                    .map(|p| json!({
                        "pid": p.pid,
                        "ppid": p.ppid,
                        "name": p.name,
                        "cmdline": p.cmdline,
                        "cpu_pct": p.cpu_pct,
                        "mem_kb": p.mem_kb,
                        "state": p.state.to_string(),
                        "threads": p.threads,
                        "uid": p.uid,
                        "read_bps": p.read_bps,
                        "write_bps": p.write_bps,
                    }))
                    .collect::<Vec<_>>(),
            })
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn data(request: &str) -> Value {
        let v: Value = serde_json::from_str(&answer(request)).expect("valid json response");
        assert_eq!(v["ok"], json!(true), "request {request} failed: {v}");
        v["data"].clone()
    }

    #[test]
    fn capabilities_lists_topics_and_gaps() {
        let d = data(r#"{"topic":"capabilities"}"#);
        assert_eq!(d["telemetry_api"], json!(TELEMETRY_API_VERSION));
        let topics = d["topics"].as_array().unwrap();
        assert!(topics.iter().any(|t| t == "processes"));
        // The honesty contract: gaps are advertised, not hidden.
        assert!(!d["unavailable"].as_array().unwrap().is_empty());
    }

    #[test]
    fn bare_topic_string_is_accepted() {
        assert_eq!(
            data("cpu")["core_count"],
            data(r#"{"topic":"cpu"}"#)["core_count"]
        );
    }

    #[test]
    fn unknown_topic_is_an_error_not_a_panic() {
        let v: Value = serde_json::from_str(&answer(r#"{"topic":"nope"}"#)).unwrap();
        assert_eq!(v["ok"], json!(false));
        assert!(v["error"].as_str().unwrap().contains("nope"));
    }

    #[test]
    fn malformed_request_does_not_panic() {
        for bad in ["", "{", "null", "[]", "\u{feff}"] {
            let out = answer(bad);
            let v: Value = serde_json::from_str(&out).expect("still valid json");
            assert!(v["ok"].is_boolean());
        }
    }

    #[test]
    fn process_limit_is_clamped() {
        let d = data(r#"{"topic":"processes","limit":100000}"#);
        assert!(d["processes"].as_array().unwrap().len() <= 200);
        let d = data(r#"{"topic":"processes","limit":0}"#);
        assert!(d["processes"].as_array().unwrap().len() <= 1);
    }

    #[test]
    fn telemetry_topics_answer_with_expected_shape() {
        assert!(data("summary")["uptime"].is_string());
        assert!(data("cpu")["cores"].is_array());
        assert!(data("memory")["total_bytes"].is_u64());
        assert!(data("network")["rx_total_bytes"].is_u64());
        assert!(data("disk")["mounts"].is_array());
        assert!(data("gpu")["present"].is_boolean());
    }
}
