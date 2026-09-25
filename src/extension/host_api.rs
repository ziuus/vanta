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
pub const TELEMETRY_API_VERSION: &str = "1.1";

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
    "io",
    "connections",
    "process_tree",
    "services",
];

/// Telemetry the host does *not* collect. Reported so extensions can render an
/// honest "unavailable" state instead of guessing, and so it is obvious what a
/// future host collector would unlock.
const UNAVAILABLE: &[(&str, &str)] = &[
    (
        "network.interfaces",
        "host aggregates /proc/net/dev across interfaces; no per-interface data",
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
        "media" => crate::widgets::media::snapshot_json(),
        "media_control" => {
            if let Some(action) = args.get("action").and_then(|v| v.as_str()) {
                let a = match action {
                    "play_pause" => Some(crate::widgets::media::Action::PlayPause),
                    "next" => Some(crate::widgets::media::Action::Next),
                    "previous" => Some(crate::widgets::media::Action::Previous),
                    "volume_up" => Some(crate::widgets::media::Action::VolumeUp),
                    "volume_down" => Some(crate::widgets::media::Action::VolumeDown),
                    _ => None,
                };
                if let Some(a) = a {
                    crate::widgets::media::control(a);
                    return Some(serde_json::json!({ "status": "ok" }));
                }
            }
            return None;
        }
        "state_get" => {
            let key = args.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let val = crate::monitors::state_store::get(key);
            serde_json::json!({ "value": val })
        }
        "state_set" => {
            let key = args
                .get("key")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            let val = args
                .get("value")
                .cloned()
                .unwrap_or(serde_json::Value::Null);
            crate::monitors::state_store::set(key, val);
            serde_json::json!({ "status": "ok" })
        }
        "fs_list" => {
            let path_str = args.get("path").and_then(|v| v.as_str()).unwrap_or(".");
            let path = std::path::PathBuf::from(path_str);
            let mut items = Vec::new();
            if let Ok(entries) = std::fs::read_dir(&path) {
                for entry in entries.flatten() {
                    if let Ok(meta) = entry.metadata() {
                        items.push(serde_json::json!({
                            "name": entry.file_name().to_string_lossy(),
                            "path": entry.path().to_string_lossy(),
                            "is_dir": meta.is_dir(),
                            "is_symlink": meta.file_type().is_symlink(),
                            "size": meta.len(),
                            "modified": meta.modified().unwrap_or(std::time::SystemTime::UNIX_EPOCH).duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default().as_secs()
                        }));
                    }
                }
            }

            items.sort_by(|a, b| {
                let a_dir = a["is_dir"].as_bool().unwrap_or(false);
                let b_dir = b["is_dir"].as_bool().unwrap_or(false);
                if a_dir != b_dir {
                    b_dir.cmp(&a_dir)
                } else {
                    let a_name = a["name"].as_str().unwrap_or("");
                    let b_name = b["name"].as_str().unwrap_or("");
                    a_name.to_lowercase().cmp(&b_name.to_lowercase())
                }
            });

            let parent = path.parent().map(|p| p.to_string_lossy().to_string());

            serde_json::json!({
                "current_dir": std::fs::canonicalize(&path).unwrap_or(path.clone()).to_string_lossy(),
                "parent": parent,
                "items": items
            })
        }
        "fs_action" => {
            let action = args.get("action").and_then(|v| v.as_str()).unwrap_or("");
            let id = format!(
                "{}",
                std::time::SystemTime::now()
                    .duration_since(std::time::SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_millis()
            );
            match action {
                "copy" => {
                    let src = args
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let dest = args
                        .get("dest")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    crate::monitors::fs_tasks::start_copy(id.clone(), src, dest);
                    serde_json::json!({ "status": "started", "id": id })
                }
                "search" => {
                    let dir = args
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    let query = args
                        .get("dest")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    crate::monitors::fs_tasks::start_search(id.clone(), dir, query);
                    serde_json::json!({ "status": "started", "id": id })
                }
                "trash" => {
                    let src = args
                        .get("source")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .to_string();
                    crate::monitors::fs_tasks::start_trash(id.clone(), src);
                    serde_json::json!({ "status": "started", "id": id })
                }
                "rename" => {
                    let src = args.get("source").and_then(|v| v.as_str()).unwrap_or("");
                    let dest = args.get("dest").and_then(|v| v.as_str()).unwrap_or("");
                    match std::fs::rename(src, dest) {
                        Ok(_) => serde_json::json!({ "status": "ok" }),
                        Err(e) => serde_json::json!({ "status": "error", "error": e.to_string() }),
                    }
                }
                _ => serde_json::json!({ "status": "error", "error": "unknown action" }),
            }
        }
        "fs_ops" => {
            let snap = crate::monitors::fs_tasks::snapshot();
            serde_json::json!(snap)
        }
        "crypto" => {
            let s = monitors::crypto::snapshot();
            json!(s)
        }
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
                        "name": p.name.as_ref(),
                        "cmdline": p.cmdline.as_ref(),
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

        // ── New topics (API v1.1) ──────────────────────────────────────────
        "io" => {
            // Per-process I/O throughput.  `read_bps` / `write_bps` are
            // bytes-per-second averages computed by the process sampler from
            // `/proc/<pid>/io` between consecutive sample windows.
            // `limit` trims the list; ordering is total-io descending.
            let limit = args
                .get("limit")
                .and_then(Value::as_u64)
                .unwrap_or(25)
                .clamp(1, 200) as usize;
            let procs = monitors::processes::top_by_io(limit);
            let total_read: f64 = procs.iter().map(|p| p.read_bps).sum();
            let total_write: f64 = procs.iter().map(|p| p.write_bps).sum();
            json!({
                "returned_by": "io_desc",
                "total_read_bps": total_read,
                "total_write_bps": total_write,
                "processes": procs.iter().map(|p| json!({
                    "pid": p.pid,
                    "name": p.name.as_ref(),
                    "read_bps": p.read_bps,
                    "write_bps": p.write_bps,
                    "total_bps": p.read_bps + p.write_bps,
                    "cpu_pct": p.cpu_pct,
                    "mem_kb": p.mem_kb,
                })).collect::<Vec<_>>(),
            })
        }

        "connections" => {
            // TCP + TCP6 socket table with process attribution.
            // PIDs owned by other users surface with pid/process_name = null.
            let conns = monitors::connections::snapshot();
            // Tally states for the summary field.
            let established = conns
                .iter()
                .filter(|c| c.state == monitors::connections::ConnState::Established)
                .count();
            let listen = conns
                .iter()
                .filter(|c| c.state == monitors::connections::ConnState::Listen)
                .count();
            let time_wait = conns
                .iter()
                .filter(|c| c.state == monitors::connections::ConnState::TimeWait)
                .count();
            json!({
                "total": conns.len(),
                "established": established,
                "listen": listen,
                "time_wait": time_wait,
                "connections": conns.iter().map(|c| json!({
                    "protocol": c.protocol,
                    "local_addr": c.local_addr,
                    "remote_addr": c.remote_addr,
                    "state": c.state.as_str(),
                    "pid": c.pid,
                    "process_name": c.process_name,
                })).collect::<Vec<_>>(),
            })
        }

        "process_tree" => {
            // Full process list with PID + PPID so extensions can build their
            // own tree.  Ordered by PID ascending (stable, predictable).
            // Extensions that need a specific ordering should sort client-side.
            let mut procs = monitors::processes::snapshot_all();
            procs.sort_by_key(|p| p.pid);
            json!({
                "total": procs.len(),
                "processes": procs.iter().map(|p| json!({
                    "pid": p.pid,
                    "ppid": p.ppid,
                    "name": p.name.as_ref(),
                    "command": p.cmdline.as_ref(),
                    "state": p.state.to_string(),
                    "threads": p.threads,
                    "uid": p.uid,
                })).collect::<Vec<_>>(),
            })
        }

        "services" => {
            let srvs = monitors::services::snapshot();
            json!({
                "total": srvs.len(),
                "services": srvs.iter().map(|s| json!({
                    "name": s.name,
                    "load_state": s.load_state,
                    "active_state": s.active_state,
                    "sub_state": s.sub_state,
                    "pid": s.pid,
                    "start_ts": s.start_ts,
                })).collect::<Vec<_>>(),
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

    // ── v1.1 topics ───────────────────────────────────────────────────────────

    #[test]
    fn capabilities_lists_v1_1_topics() {
        let d = data(r#"{"topic":"capabilities"}"#);
        assert_eq!(d["telemetry_api"], json!(TELEMETRY_API_VERSION));
        let topics = d["topics"].as_array().unwrap();
        for expected in ["io", "connections", "process_tree"] {
            assert!(
                topics.iter().any(|t| t == expected),
                "topic '{expected}' missing from capabilities"
            );
        }
        // network.connections should no longer be listed as unavailable
        let unavail: Vec<String> = d["unavailable"]
            .as_array()
            .unwrap()
            .iter()
            .map(|u| u["topic"].as_str().unwrap_or("").to_string())
            .collect();
        assert!(
            !unavail.iter().any(|u| u == "network.connections"),
            "network.connections should be promoted from unavailable now"
        );
    }

    #[test]
    fn io_topic_has_expected_shape() {
        let d = data(r#"{"topic":"io"}"#);
        assert_eq!(d["returned_by"], json!("io_desc"));
        assert!(d["total_read_bps"].is_f64() || d["total_read_bps"].is_u64());
        assert!(d["total_write_bps"].is_f64() || d["total_write_bps"].is_u64());
        assert!(d["processes"].is_array());
        // If any process row is present, validate its fields.
        if let Some(p) = d["processes"].as_array().unwrap().first() {
            assert!(p["pid"].is_u64(), "pid must be integer");
            assert!(p["name"].is_string(), "name must be string");
            assert!(p["read_bps"].is_f64() || p["read_bps"].is_u64());
            assert!(p["write_bps"].is_f64() || p["write_bps"].is_u64());
            assert!(p["total_bps"].is_f64() || p["total_bps"].is_u64());
        }
    }

    #[test]
    fn io_limit_is_clamped() {
        let d = data(r#"{"topic":"io","limit":100000}"#);
        assert!(d["processes"].as_array().unwrap().len() <= 200);
    }

    #[test]
    fn connections_topic_has_expected_shape() {
        let d = data(r#"{"topic":"connections"}"#);
        assert!(d["total"].is_u64(), "total must be u64");
        assert!(d["established"].is_u64());
        assert!(d["listen"].is_u64());
        assert!(d["time_wait"].is_u64());
        assert!(d["connections"].is_array());
        // Validate shape of each row — empty list is also valid (CI containers).
        for conn in d["connections"].as_array().unwrap() {
            let proto = conn["protocol"].as_str().unwrap_or("");
            assert!(
                proto == "tcp" || proto == "tcp6",
                "unexpected protocol: {proto}"
            );
            assert!(conn["local_addr"].is_string());
            assert!(conn["remote_addr"].is_string());
            assert!(conn["state"].is_string());
            // pid and process_name are nullable (other users' sockets)
            assert!(conn["pid"].is_u64() || conn["pid"].is_null());
            assert!(conn["process_name"].is_string() || conn["process_name"].is_null());
        }
    }

    #[test]
    fn process_tree_topic_has_expected_shape() {
        let d = data(r#"{"topic":"process_tree"}"#);
        assert!(d["total"].is_u64(), "total must be u64");
        assert!(d["processes"].is_array());
        if let Some(p) = d["processes"].as_array().unwrap().first() {
            assert!(p["pid"].is_u64());
            assert!(p["ppid"].is_u64());
            assert!(p["name"].is_string());
            assert!(p["command"].is_string());
            assert!(p["state"].is_string());
            assert!(p["threads"].is_u64());
            assert!(p["uid"].is_u64());
        }
    }

    #[test]
    fn process_tree_is_sorted_by_pid() {
        let procs = data(r#"{"topic":"process_tree"}"#);
        let pids: Vec<u64> = procs["processes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| p["pid"].as_u64().unwrap_or(0))
            .collect();
        let mut sorted = pids.clone();
        sorted.sort_unstable();
        assert_eq!(pids, sorted, "process_tree must be sorted by pid asc");
    }
}
