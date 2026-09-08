use std::fs;
use std::sync::LazyLock;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::monitors::{gpu, Summary};
use crate::theme::Theme;
use crate::widgets::meter;

/// Facts that never change for the lifetime of the process.
pub struct Facts {
    pub os: String,
    pub os_id: String,
    pub host: String,
    pub kernel: String,
    pub shell: String,
    pub cpu: String,
    pub threads: usize,
    pub term: String,
}

pub static FACTS: LazyLock<Facts> = LazyLock::new(|| {
    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let field = |key: &str| -> Option<String> {
        os_release.lines().find_map(|l| {
            l.strip_prefix(key)
                .and_then(|v| v.strip_prefix('='))
                .map(|v| v.trim_matches('"').to_string())
        })
    };
    let cpu = fs::read_to_string("/proc/cpuinfo")
        .ok()
        .and_then(|c| {
            c.lines()
                .find(|l| l.starts_with("model name"))
                .and_then(|l| l.split_once(':'))
                .map(|(_, v)| short_cpu(v.trim()))
        })
        .unwrap_or_default();
    Facts {
        os: field("PRETTY_NAME").unwrap_or_else(|| "Linux".into()),
        os_id: field("ID").unwrap_or_default().to_lowercase(),
        host: fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "?".into()),
        kernel: fs::read_to_string("/proc/sys/kernel/osrelease")
            .map(|s| s.trim().to_string())
            .unwrap_or_default(),
        shell: std::env::var("SHELL")
            .ok()
            .and_then(|s| s.rsplit('/').next().map(str::to_string))
            .unwrap_or_default(),
        cpu,
        threads: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        term: std::env::var("TERM_PROGRAM")
            .or_else(|_| std::env::var("TERM"))
            .unwrap_or_default(),
    }
});

fn short_cpu(model: &str) -> String {
    let mut s = model
        .replace("(R)", "")
        .replace("(TM)", "")
        .replace(" CPU", "")
        .replace("Intel Core ", "")
        .replace("AMD ", "")
        .replace(" Processor", "")
        .replace("-Core", "c");
    if let Some(idx) = s.find(" @ ") {
        s.truncate(idx);
    }
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// First BAT* capacity under /sys/class/power_supply, or None on AC-only machines.
pub fn read_battery() -> Option<(u8, bool)> {
    let dir = fs::read_dir("/sys/class/power_supply").ok()?;
    for entry in dir.flatten() {
        if !entry.file_name().to_string_lossy().starts_with("BAT") {
            continue;
        }
        let pct = fs::read_to_string(entry.path().join("capacity"))
            .ok()?
            .trim()
            .parse::<u8>()
            .ok()?;
        let charging = fs::read_to_string(entry.path().join("status"))
            .map(|s| matches!(s.trim(), "Charging" | "Full"))
            .unwrap_or(false);
        return Some((pct, charging));
    }
    None
}

pub fn fmt_uptime(secs: u64) -> String {
    let d = secs / 86400;
    let h = (secs % 86400) / 3600;
    let m = (secs % 3600) / 60;
    if d > 0 {
        format!("{}d {}h", d, h)
    } else if h > 0 {
        format!("{}h {}m", h, m)
    } else {
        format!("{}m", m)
    }
}

// ── Distro logo ────────────────────────────────────────────────

fn logo_lines(id: &str) -> Vec<&'static str> {
    match id {
        "arch" | "archarm" | "endeavouros" | "manjaro" | "cachyos" => vec![
            "     ▄     ",
            "    ▟█▙    ",
            "   ▟███▙   ",
            "  ▟█████▙  ",
            " ▟███▀▀███▙",
            "▟██▀    ▀██▙",
        ],
        "ubuntu" | "pop" | "linuxmint" => vec![
            "   ▄▄▄▄▄   ",
            " ▄█▀   ▀█▄ ",
            "██   ●   ██",
            "██  ●  ●  ██",
            " ▀█▄   ▄█▀ ",
            "   ▀▀▀▀▀   ",
        ],
        "fedora" | "nobara" => vec![
            "   ▄▄▄▄▄▄  ",
            "  █    ██  ",
            "  █  ▄▄▄▄  ",
            "▄▄█▄▄█     ",
            "█    █     ",
            "▀▀▀▀▀      ",
        ],
        "debian" | "raspbian" => vec![
            "  ▄▄▄▄▄    ",
            " █    ▀█   ",
            " █  ▄▄ █   ",
            " █  ▀▀▀    ",
            " ▀█▄       ",
            "   ▀▀      ",
        ],
        "nixos" => vec![
            " ▚▖   ▗▞  ▗",
            "  ▚▖ ▗▞▀▀▀▀",
            "▀▀▀▚▖▞▘    ",
            "    ▞▚▖▀▀▀▀",
            "▄▄▄▄▘ ▚▖   ",
            "  ▞▘   ▚▖  ",
        ],
        "gentoo" => vec![
            "   ▄▄▄▄▄   ",
            " ▄█▀   ▀█▄ ",
            "██  ▄▄▄  ██",
            " ▀▀▀  ▄▄█▀ ",
            "   ▄██▀    ",
            "  ▀▀       ",
        ],
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" => vec![
            "  ▄▄▄▄▄▄▄  ",
            " █  ▄▄▄▄▄█ ",
            "█  █ ▀█    ",
            "█  █▄▄█  ▄█",
            " █▄▄▄▄▄▄▄█ ",
            "           ",
        ],
        _ => vec![
            "   ▄▄▄▄▄   ",
            "  █ ▀ ▀ █  ",
            "  █  ▄  █  ",
            " ▄█▄▄▄▄▄█▄ ",
            " █▄▄▄▄▄▄▄█ ",
            "           ",
        ],
    }
}

/// Dashboard SYSTEM panel: distro logo left, neofetch-style facts right.
pub fn render_neofetch(f: &mut Frame, area: Rect, theme: &Theme, sum: &Summary, term: (u16, u16)) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let facts = &*FACTS;
    let logo = logo_lines(&facts.os_id);
    let logo_w: u16 = if area.width >= 44 { 13 } else { 0 };

    if logo_w > 0 {
        let pad = area.height.saturating_sub(logo.len() as u16) / 2;
        let mut lines: Vec<Line> = (0..pad).map(|_| Line::from("")).collect();
        lines.extend(logo.iter().map(|l| {
            Line::from(Span::styled(
                *l,
                Style::default()
                    .fg(theme.accent)
                    .add_modifier(Modifier::BOLD),
            ))
        }));
        f.render_widget(
            Paragraph::new(lines),
            Rect::new(area.x + 1, area.y, logo_w, area.height),
        );
    }

    let kv_area = Rect::new(
        area.x + logo_w,
        area.y,
        area.width.saturating_sub(logo_w),
        area.height,
    );
    let mem = crate::monitors::memory::snapshot();
    let gpu_name = gpu::name();
    let bat = match sum.battery {
        Some((p, true)) => format!("{}% ⚡", p),
        Some((p, false)) => format!("{}%", p),
        None => "AC".to_string(),
    };
    let mut kv: Vec<(&str, String)> = vec![
        ("os", facts.os.clone()),
        ("host", facts.host.clone()),
        ("kernel", facts.kernel.clone()),
        ("uptime", sum.uptime.clone()),
        ("shell", facts.shell.clone()),
        ("term", format!("{} {}×{}", facts.term, term.0, term.1)),
        ("cpu", format!("{} ({}t)", facts.cpu, facts.threads)),
    ];
    if !gpu_name.is_empty() {
        kv.push(("gpu", gpu_name));
    }
    kv.push((
        "memory",
        format!(
            "{} / {}",
            meter::fmt_bytes(mem.used),
            meter::fmt_bytes(mem.total)
        ),
    ));
    kv.push(("battery", bat));

    let max_rows = kv_area.height as usize;
    let kv: Vec<_> = kv.into_iter().take(max_rows).collect();
    let vpad = kv_area.height.saturating_sub(kv.len() as u16) / 2;
    let max_v = (kv_area.width as usize).saturating_sub(9);
    let mut rows: Vec<Line> = (0..vpad).map(|_| Line::from("")).collect();
    for (k, v) in kv {
        rows.push(Line::from(vec![
            Span::styled(format!("{:>7} ", k), Style::default().fg(theme.dim)),
            Span::styled(
                meter::ellipsize(&v, max_v),
                Style::default().fg(theme.text).add_modifier(Modifier::BOLD),
            ),
        ]));
    }
    f.render_widget(Paragraph::new(rows), kv_area);
}

/// Monitor page: compact two-column facts.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme, sum: &Summary) {
    if area.height < 1 {
        return;
    }
    let facts = &*FACTS;
    let k = Style::default().fg(theme.dim);
    let v = Style::default().fg(theme.text).add_modifier(Modifier::BOLD);
    let row = |key: &str, val: String| {
        Line::from(vec![
            Span::styled(format!("{:>7} ", key), k),
            Span::styled(val, v),
        ])
    };
    let bat = match sum.battery {
        Some((p, true)) => format!("{}% ⚡", p),
        Some((p, false)) => format!("{}%", p),
        None => "AC".to_string(),
    };
    let left = vec![
        row(
            "os",
            facts.os.split_whitespace().next().unwrap_or("Linux").into(),
        ),
        row("host", facts.host.clone()),
        row("kernel", facts.kernel.clone()),
    ];
    let right = vec![
        row("uptime", sum.uptime.clone()),
        row("battery", bat),
        row("procs", crate::monitors::processes::count().to_string()),
    ];
    let cols = Layout::horizontal([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)]).split(area);
    f.render_widget(Paragraph::new(left), cols[0]);
    f.render_widget(Paragraph::new(right), cols[1]);
}
