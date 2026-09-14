use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::{Duration, Instant};

use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::cpu;
use crate::theme::Theme;

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
    /// `checkupdates` syncs package databases over the network; poll it far
    /// less often than the local facts.
    updates_stamp: Option<Instant>,
}

static CACHE: LazyLock<Mutex<Cached>> = LazyLock::new(|| {
    Mutex::new(Cached {
        facts: Facts::default(),
        stamp: None,
        updates_stamp: None,
    })
});

/// How long collected facts stay fresh. These change on the order of minutes,
/// not frames.
const TTL: Duration = Duration::from_secs(30);
const UPDATES_TTL: Duration = Duration::from_secs(15 * 60);

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

/// Refresh the slow facts if stale. Runs on the sampler thread; the shell-outs
/// (`checkupdates` alone can take seconds) never touch the render loop.
pub fn sample() {
    let (stale, updates_stale, prev_updates) = {
        let c = CACHE.lock().unwrap();
        (
            c.stamp.is_none_or(|t| t.elapsed() > TTL),
            c.updates_stamp.is_none_or(|t| t.elapsed() > UPDATES_TTL),
            c.facts.updates,
        )
    };
    if !stale {
        return;
    }
    let updates = if updates_stale {
        count_updates()
    } else {
        prev_updates
    };
    let fresh = Facts {
        packages: count_packages(),
        updates,
        wifi: read_wifi(),
        ip: read_ip(),
        docker: read_docker(),
    };
    let mut c = CACHE.lock().unwrap();
    c.facts = fresh;
    c.stamp = Some(Instant::now());
    if updates_stale {
        c.updates_stamp = Some(Instant::now());
    }
}

fn facts() -> Facts {
    CACHE.lock().unwrap().facts.clone()
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
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, is_focused: bool, selected: usize) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let fx = facts();

    let build_line = |idx: usize, k: &str, v_spans: Vec<Span<'static>>| {
        let key_style = if is_focused && idx == selected {
            Style::default().fg(theme.bg).bg(theme.accent)
        } else if is_focused {
            Style::default().fg(theme.text)
        } else {
            Style::default().fg(theme.dim)
        };
        
        let mut spans = vec![
            Span::styled(if is_focused && idx == selected { format!(" {} ", k) } else { format!(" {} ", k) }, key_style),
            Span::raw(" "),
        ];
        // Ensure standard width for key column
        let pad = 6_usize.saturating_sub(k.len() + 2);
        if pad > 0 {
            spans.push(Span::raw(" ".repeat(pad)));
        }
        spans.extend(v_spans);
        Line::from(spans)
    };

    let mut lines: Vec<Line> = Vec::new();
    let mut i = 0;

    if let Some((ssid, sig)) = &fx.wifi {
        lines.push(build_line(i, "wifi", vec![
            Span::styled(format!("{:<14}", crate::widgets::meter::ellipsize(ssid, 14)), Style::default().fg(theme.accent)),
            Span::styled(
                format!("{} {}%", signal_bars(*sig), sig),
                Style::default().fg(if *sig < 40 { theme.yellow } else { theme.dim }),
            ),
        ]));
        i += 1;
    }
    if let Some(ip) = &fx.ip {
        lines.push(build_line(i, "ip", vec![
            Span::styled(ip.clone(), Style::default().fg(theme.text)),
        ]));
        i += 1;
    }
    if let Some(n) = fx.packages {
        let upd = fx.updates.unwrap_or(0);
        let mut spans = vec![Span::styled(format!("{:<14}", n), Style::default().fg(theme.text))];
        if upd > 0 {
            spans.push(Span::styled(
                format!("{} updates", upd),
                Style::default().fg(theme.yellow),
            ));
        } else if fx.updates.is_some() {
            spans.push(Span::styled("up to date", Style::default().fg(theme.dim)));
        }
        lines.push(build_line(i, "pkgs", spans));
        i += 1;
    }
    if let Some((run_n, all_n)) = fx.docker {
        lines.push(build_line(i, "docker", vec![
            Span::styled(format!("{:<14}", format!("{}/{}", run_n, all_n)), Style::default().fg(theme.text)),
            Span::styled("running", Style::default().fg(theme.dim)),
        ]));
        i += 1;
    }
    let cpu = cpu::snapshot();
    {
        let cores = cpu.cores.len().max(1) as f64;
        let (one, five, fifteen) = cpu.load;
        let col = if one > cores {
            theme.red
        } else if one > cores * 0.7 {
            theme.yellow
        } else {
            theme.accent
        };
        lines.push(build_line(i, "load", vec![
            Span::styled(format!("{:<14}", format!("{:.2} {:.2} {:.2}", one, five, fifteen)), Style::default().fg(col)),
            Span::styled(format!("/{}", cores as usize), Style::default().fg(theme.dim)),
        ]));
        i += 1;
        lines.push(build_line(i, "procs", vec![
            Span::styled(crate::monitors::processes::count().to_string(), Style::default().fg(theme.text)),
        ]));
        i += 1;
    }

    if let Some(b) = crate::monitors::system_info::read_battery_detail() {
        let col = if b.charging || b.pct > 20 {
            theme.accent
        } else if b.pct > 10 {
            theme.yellow
        } else {
            theme.red
        };
        let mut spans = vec![
            Span::styled(format!("{:<14}", format!("{}%{}", b.pct, if b.charging { " ⚡" } else { "" })), Style::default().fg(col)),
        ];
        if let Some(w) = b.watts {
            spans.push(Span::styled(format!("{:<8.1}", w), Style::default().fg(theme.dim)));
        } else {
            spans.push(Span::styled(format!("{:<8}", ""), Style::default()));
        }
        if let Some(s) = b.eta_secs.filter(|s| *s > 0 && *s < 48 * 3600) {
            spans.push(Span::styled(
                format!("{}h{:02}m {}", s / 3600, (s % 3600) / 60, if b.charging { "to full" } else { "left" }),
                Style::default().fg(theme.dim),
            ));
        }
        lines.push(build_line(i, "bat", spans));
        i += 1;
    }

    if let Some(max) = cpu.max_temp() {
        let limit = if area.width > 35 { 8 } else { 4 };
        let mut t_str = cpu.temps.iter().take(limit).map(|t| format!("{:.0}°", t)).collect::<Vec<_>>().join(" ");
        if cpu.temps.len() > limit {
            t_str.push_str(" …");
        }
        lines.push(build_line(i, "temps", vec![
            Span::styled(t_str, Style::default().fg(theme.temp(max))),
        ]));
        i += 1;
    }

    if lines.is_empty() {
        return;
    }

    // Top-align: no vertical centering — fills from the top of the box
    f.render_widget(Paragraph::new(lines), area);
}

pub fn active_row_ids() -> Vec<&'static str> {
    let fx = facts();
    let mut ids = Vec::new();
    if fx.wifi.is_some() { ids.push("wifi"); }
    if fx.ip.is_some() { ids.push("ip"); }
    if fx.packages.is_some() { ids.push("pkgs"); }
    if fx.docker.is_some() { ids.push("docker"); }
    ids.push("load");
    ids.push("procs");
    if crate::monitors::system_info::read_battery_detail().is_some() { ids.push("bat"); }
    if crate::monitors::cpu::snapshot().max_temp().is_some() { ids.push("temps"); }
    ids
}
