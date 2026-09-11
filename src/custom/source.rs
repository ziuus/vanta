//! Background data-source workers for custom widgets.
//!
//! Each enabled custom widget gets one `DataWorker` that runs on its own
//! thread.  The worker polls its source at the configured interval and writes
//! the result into a `Mutex<FetchResult>` that the render thread reads.
//!
//! Rules:
//! - Never blocks the render / UI thread.
//! - Never modifies files.
//! - Command execution does NOT use a shell (`sh -c`) to avoid injection.
//! - Timeouts kill the child process and surface a `Timeout` error state.

use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::custom::config::{CustomWidgetConfig, SourceKind};

// ── Public result type ────────────────────────────────────────────────────────

/// The last known outcome of fetching a widget's data.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FetchResult {
    /// Not yet fetched (widget just started).
    Loading,
    /// Successfully fetched raw string (trimmed).
    Ok(String),
    /// Command returned non-zero or produced no output.
    CommandFailed(String),
    /// File was not found or not readable.
    Unavailable,
    /// Command did not finish within the timeout.
    Timeout,
    /// Any other transient error.
    Error(String),
}

impl FetchResult {
    /// Short display string for the UI when data is unavailable.
    pub fn error_label(&self) -> &str {
        match self {
            Self::Loading => "Loading…",
            Self::Ok(_) => "",
            Self::CommandFailed(_) => "Command failed",
            Self::Unavailable => "Unavailable",
            Self::Timeout => "Timeout",
            Self::Error(_) => "Error",
        }
    }
}

// ── Shared slot ───────────────────────────────────────────────────────────────

pub type SharedResult = Arc<Mutex<FetchResult>>;

// ── DataWorker ────────────────────────────────────────────────────────────────

/// Opaque handle to a spawned worker thread. Dropping it does **not** kill the
/// thread (threads are daemon-style), but the internal `Arc` keeps the shared
/// slot alive as long as either side holds it.
pub struct DataWorker {
    pub result: SharedResult,
}

/// Hard cap on how long we wait for a command before declaring a timeout.
const CMD_TIMEOUT: Duration = Duration::from_secs(10);

impl DataWorker {
    /// Spawn a background worker for the given config.
    /// Returns `None` if the config fails validation (caller logs the reason).
    pub fn spawn(cfg: &CustomWidgetConfig) -> Option<Self> {
        if cfg.validate().is_err() {
            return None;
        }
        let result: SharedResult = Arc::new(Mutex::new(FetchResult::Loading));
        let slot = Arc::clone(&result);
        let cfg = cfg.clone();

        std::thread::Builder::new()
            .name(format!("vanta-custom-{}", cfg.id))
            .spawn(move || {
                let interval = Duration::from_secs_f64(cfg.clamped_refresh());
                loop {
                    let started = Instant::now();
                    let outcome = fetch(&cfg);
                    *slot.lock().unwrap() = outcome;
                    let elapsed = started.elapsed();
                    if interval > elapsed {
                        std::thread::sleep(interval - elapsed);
                    }
                }
            })
            .ok()?;

        Some(Self { result })
    }
}

// ── Fetch dispatch ────────────────────────────────────────────────────────────

fn fetch(cfg: &CustomWidgetConfig) -> FetchResult {
    match cfg.source {
        SourceKind::Command => {
            let cmd = cfg.command.as_deref().unwrap_or("");
            fetch_command(cmd)
        }
        SourceKind::File => {
            let path = cfg.path.as_deref().unwrap_or("");
            fetch_file(path)
        }
    }
}

// ── Command source ────────────────────────────────────────────────────────────

/// Split a command string into argv without invoking a shell.
/// Supports single/double quotes and backslash escapes.
/// Returns `None` if the string is empty after trimming.
fn split_command(cmd: &str) -> Option<Vec<String>> {
    let mut args = Vec::new();
    let mut current = String::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut escaped = false;

    for c in cmd.chars() {
        if escaped {
            current.push(c);
            escaped = false;
        } else if c == '\\' && !in_single_quote {
            escaped = true;
        } else if c == '\'' && !in_double_quote {
            in_single_quote = !in_single_quote;
        } else if c == '"' && !in_single_quote {
            in_double_quote = !in_double_quote;
        } else if c.is_whitespace() && !in_single_quote && !in_double_quote {
            if !current.is_empty() {
                args.push(current.clone());
                current.clear();
            }
        } else {
            current.push(c);
        }
    }
    if !current.is_empty() {
        args.push(current);
    }
    if args.is_empty() {
        None
    } else {
        Some(args)
    }
}

fn fetch_command(cmd: &str) -> FetchResult {
    fetch_command_with_timeout(cmd, CMD_TIMEOUT)
}

fn fetch_command_with_timeout(cmd: &str, timeout: Duration) -> FetchResult {
    let Some(parts) = split_command(cmd) else {
        return FetchResult::CommandFailed("empty command".into());
    };
    let exe = &parts[0];
    let args = &parts[1..];

    // Spawn without a shell.
    let mut child = match std::process::Command::new(exe)
        .args(args)
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::null())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            log_debug(&format!("custom widget: failed to spawn {:?}: {}", exe, e));
            return FetchResult::CommandFailed(format!("{}", e));
        }
    };

    // Poll until the child finishes or we hit the timeout.
    let deadline = Instant::now() + timeout;
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                use std::io::Read;
                let mut out = String::new();
                if let Some(mut stdout) = child.stdout.take() {
                    let _ = stdout.read_to_string(&mut out);
                }
                let trimmed = out.trim().to_string();
                if status.success() {
                    if trimmed.is_empty() {
                        return FetchResult::CommandFailed("no output".into());
                    }
                    return FetchResult::Ok(trimmed);
                } else {
                    log_debug(&format!(
                        "custom widget: {:?} exited {:?}",
                        exe,
                        status.code()
                    ));
                    return FetchResult::CommandFailed(format!(
                        "exit {}",
                        status.code().unwrap_or(-1)
                    ));
                }
            }
            Ok(None) => {
                if Instant::now() >= deadline {
                    let _ = child.kill();
                    // kill() only sends the signal. Without a wait() the child
                    // stays a zombie for the life of the process — Rust's
                    // Child::drop does not reap. vanta runs all day, so a
                    // widget whose command hangs would leak one PID per poll.
                    let _ = child.wait();
                    log_debug(&format!("custom widget: {:?} timed out", exe));
                    return FetchResult::Timeout;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                return FetchResult::Error(format!("{}", e));
            }
        }
    }
}

// ── File source ───────────────────────────────────────────────────────────────

fn fetch_file(path: &str) -> FetchResult {
    match std::fs::read_to_string(path) {
        Ok(s) => {
            let trimmed = s.trim().to_string();
            if trimmed.is_empty() {
                FetchResult::Unavailable
            } else {
                FetchResult::Ok(trimmed)
            }
        }
        Err(e) => {
            log_debug(&format!("custom widget: read {:?}: {}", path, e));
            FetchResult::Unavailable
        }
    }
}

// ── Debug logging ─────────────────────────────────────────────────────────────

/// Write a line to stderr when `VANTA_DEBUG=1` is set.  The UI is never
/// cluttered with raw error messages.
fn log_debug(msg: &str) {
    if std::env::var("VANTA_DEBUG").as_deref() == Ok("1") {
        eprintln!("[vanta-custom] {}", msg);
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    fn cmd_cfg(id: &str, cmd: &str) -> CustomWidgetConfig {
        CustomWidgetConfig {
            id: id.into(),
            title: id.into(),
            source: SourceKind::Command,
            command: Some(cmd.into()),
            refresh: 60.0, // long — we never want tests to re-poll
            ..Default::default()
        }
    }

    fn file_cfg(id: &str, path: &str) -> CustomWidgetConfig {
        CustomWidgetConfig {
            id: id.into(),
            title: id.into(),
            source: SourceKind::File,
            path: Some(path.into()),
            refresh: 60.0,
            ..Default::default()
        }
    }

    // ---- command source --------------------------------------------------------

    #[test]
    fn successful_command_returns_ok() {
        let result = fetch_command("echo hello");
        assert_eq!(result, FetchResult::Ok("hello".into()));
    }

    #[test]
    fn command_with_args_works() {
        let result = fetch_command("echo 12.4 V");
        assert_eq!(result, FetchResult::Ok("12.4 V".into()));
    }

    #[test]
    fn nonexistent_command_fails_gracefully() {
        let result = fetch_command("__vanta_no_such_binary_xyz__");
        assert!(
            matches!(result, FetchResult::CommandFailed(_)),
            "expected CommandFailed, got {:?}",
            result
        );
    }

    #[test]
    fn nonzero_exit_returns_command_failed() {
        let result = fetch_command("false");
        assert!(matches!(result, FetchResult::CommandFailed(_)));
    }

    #[test]
    fn empty_command_string_fails() {
        let result = fetch_command("");
        assert!(matches!(result, FetchResult::CommandFailed(_)));
    }

    /// Child processes of this process that have exited but not been reaped.
    fn zombie_children() -> usize {
        let me = std::process::id().to_string();
        let Ok(entries) = std::fs::read_dir("/proc") else {
            return 0;
        };
        entries
            .flatten()
            .filter(|e| {
                let Ok(pid) = e.file_name().to_string_lossy().parse::<u32>() else {
                    return false;
                };
                let Ok(status) = std::fs::read_to_string(format!("/proc/{}/status", pid)) else {
                    return false;
                };
                let zombie = status
                    .lines()
                    .any(|l| l.starts_with("State:") && l.contains('Z'));
                let ours = status
                    .lines()
                    .any(|l| l.strip_prefix("PPid:").is_some_and(|v| v.trim() == me));
                zombie && ours
            })
            .count()
    }

    /// Regression: the timeout path must reap, not just signal. `kill()` alone
    /// left a zombie per timed-out poll, which on a day-long session is a slow
    /// PID leak.
    #[test]
    fn timed_out_command_is_reaped() {
        let result = fetch_command_with_timeout("sleep 30", Duration::from_millis(200));
        assert!(matches!(result, FetchResult::Timeout));

        // Every other path reaps via try_wait(), so the steady state is zero —
        // but sibling tests run in parallel and their children can be mid-reap,
        // so poll instead of sampling once. An unreaped child never clears.
        let deadline = Instant::now() + Duration::from_secs(2);
        while zombie_children() > 0 && Instant::now() < deadline {
            std::thread::sleep(Duration::from_millis(50));
        }
        assert_eq!(
            zombie_children(),
            0,
            "timed-out child was killed but never reaped"
        );
    }

    // ---- file source -----------------------------------------------------------

    #[test]
    fn existing_file_is_read() {
        let path = std::env::temp_dir().join("vanta_test_existing.txt");
        std::fs::write(&path, "  42  \n").unwrap();
        let result = fetch_file(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        assert_eq!(result, FetchResult::Ok("42".into()));
    }

    #[test]
    fn missing_file_returns_unavailable() {
        let result = fetch_file("/nonexistent/path/that/does/not/exist");
        assert_eq!(result, FetchResult::Unavailable);
    }

    #[test]
    fn empty_file_returns_unavailable() {
        let path = std::env::temp_dir().join("vanta_test_empty.txt");
        std::fs::write(&path, "").unwrap();
        let result = fetch_file(path.to_str().unwrap());
        let _ = std::fs::remove_file(&path);
        assert_eq!(result, FetchResult::Unavailable);
    }

    // ---- worker spawn ----------------------------------------------------------

    #[test]
    fn worker_spawns_for_valid_config() {
        let cfg = cmd_cfg("test_w", "echo 99");
        let worker = DataWorker::spawn(&cfg);
        assert!(worker.is_some());
        // Give it a moment to execute.
        std::thread::sleep(Duration::from_millis(300));
        let r = worker.unwrap().result.lock().unwrap().clone();
        assert_eq!(r, FetchResult::Ok("99".into()));
    }

    #[test]
    fn worker_none_for_invalid_config() {
        let cfg = CustomWidgetConfig {
            id: "".into(), // invalid
            ..Default::default()
        };
        assert!(DataWorker::spawn(&cfg).is_none());
    }

    #[test]
    fn worker_reads_file_source() {
        let path = std::env::temp_dir().join("vanta_test_worker.txt");
        std::fs::write(&path, "77\n").unwrap();
        let cfg = file_cfg("bat_test", path.to_str().unwrap());
        let worker = DataWorker::spawn(&cfg).unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let r = worker.result.lock().unwrap().clone();
        let _ = std::fs::remove_file(&path);
        assert_eq!(r, FetchResult::Ok("77".into()));
    }

    // ---- split_command ---------------------------------------------------------

    #[test]
    fn split_command_simple() {
        assert_eq!(
            split_command("cat /sys/class/thermal/thermal_zone0/temp"),
            Some(vec![
                "cat".into(),
                "/sys/class/thermal/thermal_zone0/temp".into()
            ])
        );
    }

    #[test]
    fn split_command_empty_inputs() {
        assert_eq!(split_command(""), None);
        assert_eq!(split_command("   "), None);
    }

    #[test]
    fn split_command_double_quotes() {
        assert_eq!(
            split_command(r#"echo "hello world""#),
            Some(vec!["echo".into(), "hello world".into()])
        );
    }

    #[test]
    fn split_command_single_quotes() {
        assert_eq!(
            split_command("echo 'hello world'"),
            Some(vec!["echo".into(), "hello world".into()])
        );
    }

    #[test]
    fn split_command_backslash_escape() {
        assert_eq!(
            split_command(r"echo hello\ world"),
            Some(vec!["echo".into(), "hello world".into()])
        );
    }

    #[test]
    fn split_command_single_token() {
        assert_eq!(split_command("hostname"), Some(vec!["hostname".into()]));
    }

    #[test]
    fn split_command_mixed_quotes_and_plain() {
        assert_eq!(
            split_command(r#"grep -r "foo bar" /tmp"#),
            Some(vec![
                "grep".into(),
                "-r".into(),
                "foo bar".into(),
                "/tmp".into()
            ])
        );
    }
}
