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

/// Every logo is exactly this wide (in cells). The render column is sized from
/// it, and `logo_rows_are_uniform` enforces it.
pub const LOGO_W: usize = 11;

/// Pack a bitmap into braille lines at 2×4 dots per cell — the same subpixel
/// trick the graphs use, which renders as a solid shape on a capable font
/// rather than the stair-stepped block art it replaces. Any non-space cell in
/// a row is a lit dot; short rows are padded, so the input can be ragged.
fn braille_art<S: AsRef<str>>(bitmap: &[S]) -> Vec<String> {
    // Dot bit index for [row][col] within a cell, per the Unicode layout.
    const BITS: [[u8; 2]; 4] = [[0, 3], [1, 4], [2, 5], [6, 7]];
    let width = bitmap
        .iter()
        .map(|r| r.as_ref().chars().count())
        .max()
        .unwrap_or(0);
    let grid: Vec<Vec<bool>> = bitmap
        .iter()
        .map(|r| {
            let mut v: Vec<bool> = r.as_ref().chars().map(|c| c != ' ').collect();
            v.resize(width, false);
            v
        })
        .collect();
    let rows = grid.len();
    let lit = |x: usize, y: usize| y < rows && x < width && grid[y][x];
    let cell_w = width.div_ceil(2);
    let cell_h = rows.div_ceil(4);
    (0..cell_h)
        .map(|cy| {
            (0..cell_w)
                .map(|cx| {
                    let mut pat = 0u8;
                    for (dy, row) in BITS.iter().enumerate() {
                        for (dx, bit) in row.iter().enumerate() {
                            if lit(cx * 2 + dx, cy * 4 + dy) {
                                pat |= 1 << bit;
                            }
                        }
                    }
                    char::from_u32(0x2800 + pat as u32).unwrap_or(' ')
                })
                .collect::<String>()
        })
        .collect()
}

/// The Arch mountain, generated as a slim triangle with an inverted-V notch at
/// the base — 22×24 dots → 11×6 cells, matching `LOGO_W`. Procedural so the
/// edges are exact instead of hand-stair-stepped.
fn arch_bitmap() -> Vec<String> {
    const W: usize = 22;
    const H: usize = 24;
    let cx = (W as f64 - 1.0) / 2.0;
    let base_half = 10.0;
    (0..H)
        .map(|y| {
            let t = y as f64 / (H as f64 - 1.0); // 0 apex .. 1 base
            let half = t * base_half;
            // Inverted-V notch rising through the lower ~45% from the base.
            let notch_span = 0.45;
            let notch = if t > 1.0 - notch_span {
                (t - (1.0 - notch_span)) / notch_span * 3.4
            } else {
                0.0
            };
            (0..W)
                .map(|x| {
                    let dx = (x as f64 - cx).abs();
                    if dx <= half && dx > notch {
                        'X'
                    } else {
                        ' '
                    }
                })
                .collect::<String>()
        })
        .collect()
}

/// Lines for a distro logo. Arch renders as smooth braille; the rest still use
/// the block art until each can be redrawn and visually verified.
fn logo_lines(id: &str) -> Vec<String> {
    match id {
        "arch" | "archarm" | "endeavouros" | "manjaro" | "cachyos" => braille_art(&arch_bitmap()),
        _ => block_logo(id).into_iter().map(String::from).collect(),
    }
}

/// Block/half-block art, one entry per row, all `LOGO_W` wide.
fn block_logo(id: &str) -> Vec<&'static str> {
    match id {
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
    let logo = logo_lines(&os_id);
    // +2 for the leading indent and a column of air before the facts.
    let logo_w: u16 = if area.width >= 44 {
        LOGO_W as u16 + 2
    } else {
        0
    };

    if logo_w > 0 {
        let pad = area.height.saturating_sub(logo.len() as u16) / 2;
        let mut lines: Vec<Line> = (0..pad).map(|_| Line::from("")).collect();
        let n = logo.len().max(1) as f32;
        lines.extend(logo.iter().enumerate().map(|(i, l)| {
            // Accent at the crown fading to secondary at the base: a flat fill
            // makes the art read as one undifferentiated blob.
            let col = crate::theme::blend(theme.accent, theme.secondary, i as f32 / n);
            Line::from(Span::styled(
                l.clone(),
                Style::default().fg(col).add_modifier(Modifier::BOLD),
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
    let max_v = (kv_area.width as usize).saturating_sub(10);
    let mut rows: Vec<Line> = Vec::new();
    for (k, v) in kv {
        rows.push(Line::from(vec![
            Span::styled(format!("{:<9} ", k), Style::default().fg(theme.dim)),
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Ids covering every arm of `logo_lines`, including the fallback.
    const IDS: [&str; 9] = [
        "arch",
        "ubuntu",
        "fedora",
        "debian",
        "nixos",
        "gentoo",
        "opensuse",
        "",
        "some-unknown-distro",
    ];

    /// A row wider than its neighbours pokes out past the logo column and
    /// reads as a rendering glitch; a short one leaves a ragged edge. Both
    /// shipped before this test existed.
    #[test]
    fn logo_rows_are_uniform() {
        for id in IDS {
            for (i, line) in logo_lines(id).iter().enumerate() {
                assert_eq!(
                    line.chars().count(),
                    LOGO_W,
                    "logo {:?} row {} is {:?}",
                    id,
                    i,
                    line
                );
            }
        }
    }

    #[test]
    fn every_logo_has_the_same_height() {
        for id in IDS {
            assert_eq!(logo_lines(id).len(), 6, "logo {:?}", id);
        }
    }

    /// The art must stay in ranges terminal fonts actually cover: half and
    /// quadrant blocks (U+2580..U+259F), the geometric triangles
    /// (U+25E2..U+25E5), and braille (U+2800..U+28FF). A stray glyph outside
    /// them is what turns a logo into tofu on someone else's font.
    #[test]
    fn logo_glyphs_are_block_triangle_or_braille() {
        for id in IDS {
            for line in logo_lines(id) {
                for c in line.chars() {
                    let ok = c == ' '
                        || ('\u{2580}'..='\u{259F}').contains(&c)
                        || ('\u{25E2}'..='\u{25E5}').contains(&c)
                        || ('\u{2800}'..='\u{28FF}').contains(&c)
                        || c == '\u{25B2}';
                    assert!(ok, "logo {:?} has {:?} (U+{:04X})", id, c, c as u32);
                }
            }
        }
    }

    /// The braille packer: an all-lit 2×4 block is the full cell ⣿; a lit left
    /// column alone is ⡇ (dots 1-2-3-7). Guards the bit layout.
    #[test]
    fn braille_art_packs_dots() {
        assert_eq!(braille_art(&["XX", "XX", "XX", "XX"]), vec!["⣿"]);
        assert_eq!(braille_art(&["X ", "X ", "X ", "X "]), vec!["⡇"]);
        assert_eq!(braille_art(&["  ", "  ", "  ", "  "]), vec!["⠀"]);
    }
}
