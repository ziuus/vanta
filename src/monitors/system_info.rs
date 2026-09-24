use std::fs;
use std::sync::LazyLock;

use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
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

#[derive(Clone, Copy, Debug, Default)]
pub struct Battery {
    pub pct: u8,
    pub charging: bool,
    /// Draw (or charge) in watts, when the driver reports it.
    pub watts: Option<f64>,
    /// Estimated seconds to empty (discharging) or full (charging).
    pub eta_secs: Option<u64>,
}

/// Power/energy detail for the first battery; separate from the summary tuple
/// because only the status panel wants it.
pub fn read_battery_detail() -> Option<Battery> {
    let dir = fs::read_dir("/sys/class/power_supply").ok()?;
    let bat = dir
        .flatten()
        .find(|e| e.file_name().to_string_lossy().starts_with("BAT"))?
        .path();
    let num =
        |f: &str| -> Option<f64> { fs::read_to_string(bat.join(f)).ok()?.trim().parse().ok() };
    let pct = num("capacity")? as u8;
    let status = fs::read_to_string(bat.join("status")).unwrap_or_default();
    let charging = matches!(status.trim(), "Charging" | "Full");
    // µW directly, or µA × µV.
    let watts = num("power_now")
        .or_else(|| Some(num("current_now")? * num("voltage_now")? / 1e6))
        .map(|uw| uw / 1e6)
        .filter(|w| *w > 0.05);
    let (now, full) = match (num("energy_now"), num("energy_full")) {
        (Some(n), Some(f)) => (Some(n / 1e6), Some(f / 1e6)),
        _ => match (num("charge_now"), num("charge_full"), num("voltage_now")) {
            (Some(n), Some(f), Some(v)) => (Some(n * v / 1e12), Some(f * v / 1e12)),
            _ => (None, None),
        },
    };
    let eta_secs = match (watts, now, full, status.trim()) {
        (Some(w), Some(n), _, "Discharging") => Some((n / w * 3600.0) as u64),
        (Some(w), Some(n), Some(f), "Charging") => Some(((f - n).max(0.0) / w * 3600.0) as u64),
        _ => None,
    };
    Some(Battery {
        pct,
        charging,
        watts,
        eta_secs,
    })
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

/// Pack a bitmap into braille lines at 2×4 dots per cell — the same subpixel
/// trick the graphs use, which renders as a solid shape on a capable font
/// rather than the stair-stepped block art it replaces. Any non-space cell in
/// a row is a lit dot; short rows are padded, so the input can be ragged.
fn logo_lines(id: &str, height: u16, width: u16) -> Vec<String> {
    block_logo(id, height, width)
        .into_iter()
        .map(String::from)
        .collect()
}

/// Distro art. Arch uses fastfetch-style ASCII art (compact in dashboard, full
/// in zoom). Other distros use clean block art.
fn block_logo(id: &str, height: u16, width: u16) -> Vec<&'static str> {
    match id {
        "arch" | "archarm" | "endeavouros" | "manjaro" | "cachyos" => {
            if height >= 19 && width >= 65 {
                vec![
                    "                  -`                 ",
                    "                 .o+`                ",
                    "                `ooo/                ",
                    "               `+oooo:               ",
                    "              `+oooooo:              ",
                    "              -+oooooo+:             ",
                    "            `/:-:++oooo+:            ",
                    "           `/++++/+++++++:           ",
                    "          `/++++++++++++++:          ",
                    "         `/+++ooooooooooooo/`        ",
                    "        ./ooosssso++osssssso+`       ",
                    "       .oossssso-````/ossssss+`      ",
                    "      -osssssso.      :ssssssso.     ",
                    "     :osssssss/        osssso+++.    ",
                    "    /ossssssss/        +ssssooo/-    ",
                    "  `/ossssso+/:-        -:/+osssso+-  ",
                    " `+sso+:-`                 `.-/+oso: ",
                    "`++:.                           `-/+/",
                    ".`                                 `/",
                ]
            } else {
                vec![
                    "          .o+`         ",
                    "         `ooo/         ",
                    "        `+oooo:        ",
                    "       -+oooooo+:      ",
                    "     `/:-:++oooo+:     ",
                    "    `/++++++++++++:    ",
                    "   ./ooosss++osssso+`  ",
                    "  .oossss-````/sssss+` ",
                    " -ossss/        /ssssso",
                    "`+sso+:-`     `.-/+oso:",
                ]
            }
        }
        "ubuntu" | "pop" | "linuxmint" => vec![
            "   ▄▄▄▄▄   ",
            " ◢█▀   ▀█◣ ",
            "▐█   ▄   █▌",
            "▐█  ▀ ▀  █▌",
            " ◥█▄   ▄█◤ ",
            "   ▀▀▀▀▀   ",
        ],
        "fedora" | "nobara" => vec![
            "  ▄▄▄▄▄▄▄  ",
            " ◢█▀   ▀█◣ ",
            "▐█  ▄▄▄█▌  ",
            "▐█  █▌ ▀   ",
            " ◥█▄█▌     ",
            "   ▀▀▀     ",
        ],
        "debian" | "raspbian" => vec![
            "   ▄▄▄▄▄   ",
            "  ◢█▀ ▀█◣  ",
            " ▐█  ▄  █▌ ",
            " ▐█ ▐█▌    ",
            "  ◥█▄▄█◤   ",
            "     ▀▀    ",
        ],
        "nixos" => vec![
            "  ◥█◣ ◢█◤  ",
            "▀▀▀◥█◣█◤▀▀▀",
            "   ◢█◤◥█◣  ",
            "  ◢█◤ ◥█◣  ",
            "▄▄◢█◤ ◥█◣▄▄",
            "  ◥█◣ ◢█◤  ",
        ],
        "gentoo" => vec![
            "  ▄▄▄▄▄▄   ",
            " ◢█▀  ▀█◣  ",
            "▐█  ▄▄  █▌ ",
            " ◥▀▀  ▄█◤  ",
            "   ◢██◤    ",
            "  ▀▀       ",
        ],
        "opensuse" | "opensuse-tumbleweed" | "opensuse-leap" => vec![
            "  ▄▄▄▄▄▄▄  ",
            " ◢█▀▀▀▀▀█◣ ",
            "▐█  ▄▄  █▌ ",
            "▐█ ▐██▌ ▄█▌",
            " ◥█▄▄▄▄▄█◤ ",
            "   ▀▀▀▀▀   ",
        ],
        _ => vec![
            "   ▄▄▄▄▄   ",
            "  ◢█▀▀▀█◣  ",
            " ▐█ ▀ ▀ █▌ ",
            " ▐█  ▄  █▌ ",
            "  ◥█▄▄▄█◤  ",
            "  ▀▀▀▀▀▀▀  ",
        ],
    }
}

/// Dashboard SYSTEM panel: distro logo left, neofetch-style facts right.
pub fn render_neofetch(f: &mut Frame, area: Rect, theme: &Theme, sum: &Summary, term: (u16, u16)) {
    if area.height < 3 || area.width < 20 {
        return;
    }
    let facts = &*FACTS;
    // VANTA_LOGO forces a distro logo, for previewing art on any machine.
    let os_id = std::env::var("VANTA_LOGO").unwrap_or_else(|_| facts.os_id.clone());
    let logo = logo_lines(&os_id, area.height, area.width);
    let max_len = logo.iter().map(|l| l.chars().count()).max().unwrap_or(0);
    // +2 for the leading indent and a column of air before the facts.
    let logo_w: u16 = if area.width >= (max_len + 20) as u16 {
        max_len as u16 + 2
    } else {
        0
    };

    let pad = if area.height > logo.len() as u16 + 4 {
        2
    } else {
        area.height.saturating_sub(logo.len() as u16) / 2
    };

    if logo_w > 0 {
        let mut lines: Vec<Line> = (0..pad).map(|_| Line::from("")).collect();
        let n = logo.len().max(1) as f32;
        lines.extend(logo.iter().enumerate().map(|(i, l)| {
            // Accent at the crown fading to secondary at the base: a flat fill
            // makes the art read as one undifferentiated blob.
            let col = crate::theme::blend(theme.accent, theme.secondary, i as f32 / n);
            Line::from(Span::styled(l.clone(), Style::default().fg(col)))
        }));
        f.render_widget(
            Paragraph::new(lines),
            Rect::new(area.x + 1, area.y, max_len as u16, area.height),
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

    let max_rows = kv_area.height.saturating_sub(pad) as usize;
    let kv: Vec<_> = kv.into_iter().take(max_rows).collect();
    let max_v = (kv_area.width as usize).saturating_sub(10);
    let mut rows: Vec<Line> = (0..pad).map(|_| Line::from("")).collect();
    for (k, v) in kv {
        rows.push(Line::from(vec![
            Span::styled(format!("{:<9} ", k), Style::default().fg(theme.dim)),
            Span::styled(meter::ellipsize(&v, max_v), Style::default().fg(theme.text)),
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
    let v = Style::default().fg(theme.text);
    let bat = match sum.battery {
        Some((p, true)) => format!("{}% ⚡", p),
        Some((p, false)) => format!("{}%", p),
        None => "AC".to_string(),
    };
    let left = [
        (
            "os",
            facts.os.split_whitespace().next().unwrap_or("Linux").into(),
        ),
        ("host", facts.host.clone()),
        ("kernel", facts.kernel.clone()),
    ];
    let right = [
        ("up", sum.uptime.clone()),
        ("bat", bat),
        ("procs", crate::monitors::processes::count().to_string()),
    ];
    // Values are ellipsized to their column so nothing clips mid-word.
    let lines = |items: &[(&str, String)], width: u16| -> Vec<Line<'static>> {
        let kw = items.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
        // One trailing cell keeps the left column off the right one.
        let vw = (width as usize).saturating_sub(kw + 3);
        items
            .iter()
            .map(|(key, val)| {
                Line::from(vec![
                    Span::styled(format!("{:>kw$}  ", key), k),
                    Span::styled(meter::ellipsize(val, vw), v),
                ])
            })
            .collect()
    };
    let cols = Layout::horizontal([Constraint::Ratio(3, 5), Constraint::Ratio(2, 5)]).split(area);
    f.render_widget(Paragraph::new(lines(&left, cols[0].width)), cols[0]);
    f.render_widget(Paragraph::new(lines(&right, cols[1].width)), cols[1]);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_arch_logos_uniform_lengths() {
        let compact = logo_lines("arch", 10, 60);
        assert_eq!(compact.len(), 10);
        for line in &compact {
            assert_eq!(line.chars().count(), 23);
        }

        let full = logo_lines("arch", 25, 80);
        assert_eq!(full.len(), 19);
        for line in &full {
            assert_eq!(line.chars().count(), 37);
        }
    }
}
