use std::fs;
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::Theme;

/// Facts that need a subprocess or a directory walk. Refreshed on a TTL so the
/// ~125fps render loop never pays for them.
#[derive(Clone, Default)]
struct Facts {
    packages: Option<usize>,
    updates: Option<usize>,
    wifi: Option<(String, u8)>,
    ip: Option<String>,
    docker: Option<(usize, usize)>, // running, total
}

struct Cached {
    facts: Facts,
    stamp: Option<Instant>,
}

static CACHE: LazyLock<Mutex<Cached>> = LazyLock::new(|| {
    Mutex::new(Cached {
        facts: Facts::default(),
        stamp: None,
    })
});

/// How long collected facts stay fresh. These change on the order of minutes,
/// not frames.
const TTL: Duration = Duration::from_secs(30);

fn run(cmd: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(cmd).args(args).output().ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn count_packages() -> Option<usize> {
    // Try the common package managers; first one that answers wins.
    for (cmd, args) in [
        ("pacman", &["-Qq"][..]),
        ("dpkg-query", &["-f", ".\n", "-W"][..]),
        ("rpm", &["-qa"][..]),
    ] {
        if let Some(out) = run(cmd, args) {
            return Some(out.lines().count());
        }
    }
    None
}

fn count_updates() -> Option<usize> {
    // checkupdates (arch) exits 2 when there is nothing to do, which `run`
    // already filters out as a non-success status.
    run("checkupdates", &[]).map(|o| o.lines().filter(|l| !l.trim().is_empty()).count())
}

fn read_wifi() -> Option<(String, u8)> {
    let out = run("nmcli", &["-t", "-f", "active,ssid,signal", "dev", "wifi"])?;
    for line in out.lines() {
        let mut parts = line.split(':');
        if parts.next()? != "yes" {
            continue;
        }
        let ssid = parts.next()?.to_string();
        let signal = parts.next()?.parse().unwrap_or(0);
        return Some((ssid, signal));
    }
    None
}

/// Primary IPv4 address, skipping loopback and container/virtual bridges.
fn read_ip() -> Option<String> {
    let out = run("ip", &["-o", "-4", "addr", "show"])?;
    for line in out.lines() {
        let mut f = line.split_whitespace();
        let _idx = f.next()?;
        let iface = f.next()?;
        if iface == "lo" || iface.starts_with("docker") || iface.starts_with("waydroid") {
            continue;
        }
        let addr = f.nth(1)?;
        return Some(format!("{} {}", iface, addr.split('/').next()?));
    }
    None
}

fn read_docker() -> Option<(usize, usize)> {
    let all = run("docker", &["ps", "-aq"])?;
    let running = run("docker", &["ps", "-q"]).unwrap_or_default();
    Some((
        running.lines().filter(|l| !l.trim().is_empty()).count(),
        all.lines().filter(|l| !l.trim().is_empty()).count(),
    ))
}

fn facts() -> Facts {
    let mut c = CACHE.lock().unwrap();
    let stale = c.stamp.is_none_or(|t| t.elapsed() > TTL);
    if stale {
        c.facts = Facts {
            packages: count_packages(),
            updates: count_updates(),
            wifi: read_wifi(),
            ip: read_ip(),
            docker: read_docker(),
        };
        c.stamp = Some(Instant::now());
    }
    c.facts.clone()
}

/// Load average and the running/total process counts from /proc/loadavg.
fn loadavg() -> Option<(f64, f64, f64, String)> {
    let s = fs::read_to_string("/proc/loadavg").ok()?;
    let mut f = s.split_whitespace();
    let one = f.next()?.parse().ok()?;
    let five = f.next()?.parse().ok()?;
    let fifteen = f.next()?.parse().ok()?;
    let procs = f.next()?.to_string();
    Some((one, five, fifteen, procs))
}

fn signal_bars(pct: u8) -> &'static str {
    match pct {
        0..=20 => "▂___",
        21..=40 => "▂▄__",
        41..=60 => "▂▄▆_",
        _ => "▂▄▆█",
    }
}

/// System/user status: network, packages, containers, load, session.
/// Everything expensive is TTL-cached; the rest is a /proc read.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let fx = facts();

    let key = |k: &str| Span::styled(format!("{:<9}", k), Style::default().fg(theme.dim));
    let val = |v: String, c: ratatui::style::Color| {
        Span::styled(v, Style::default().fg(c).add_modifier(Modifier::BOLD))
    };

    let mut lines: Vec<Line> = Vec::new();

    if let Some((ssid, sig)) = &fx.wifi {
        lines.push(Line::from(vec![
            key("WIFI"),
            val(ssid.clone(), theme.accent),
            Span::styled(
                format!("  {} {}%", signal_bars(*sig), sig),
                Style::default().fg(if *sig < 40 { theme.yellow } else { theme.dim }),
            ),
        ]));
    }
    if let Some(ip) = &fx.ip {
        lines.push(Line::from(vec![key("IP"), val(ip.clone(), theme.text)]));
    }
    if let Some(n) = fx.packages {
        let upd = fx.updates.unwrap_or(0);
        let mut spans = vec![key("PKGS"), val(n.to_string(), theme.text)];
        if upd > 0 {
            spans.push(Span::styled(
                format!("  {} updates", upd),
                Style::default().fg(theme.yellow),
            ));
        } else if fx.updates.is_some() {
            spans.push(Span::styled("  up to date", Style::default().fg(theme.dim)));
        }
        lines.push(Line::from(spans));
    }
    if let Some((run_n, all_n)) = fx.docker {
        lines.push(Line::from(vec![
            key("DOCKER"),
            val(format!("{}/{}", run_n, all_n), theme.text),
            Span::styled(" running", Style::default().fg(theme.dim)),
        ]));
    }
    if let Some((one, five, fifteen, procs)) = loadavg() {
        let cores = std::thread::available_parallelism()
            .map(|n| n.get() as f64)
            .unwrap_or(1.0);
        let col = if one > cores {
            theme.red
        } else if one > cores * 0.7 {
            theme.yellow
        } else {
            theme.accent
        };
        lines.push(Line::from(vec![
            key("LOAD"),
            val(format!("{:.2} {:.2} {:.2}", one, five, fifteen), col),
        ]));
        lines.push(Line::from(vec![
            key("PROCS"),
            val(procs, theme.text),
        ]));
    }

    if lines.is_empty() {
        return;
    }

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
    );
}
