use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::Theme;

/// Per-filesystem capacity meters. The dashboard's ANALYTICS row only carries a
/// single aggregate DSK figure, which hides a nearly-full /home or /boot.
pub fn render(f: &mut Frame, area: Rect, theme: &Theme) {
    if area.height < 2 || area.width < 24 {
        return;
    }

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let mut rows: Vec<(String, f64, u64, u64)> = Vec::new();
    for d in disks.iter() {
        let fs = d.file_system().to_str().unwrap_or("");
        let name = d.name().to_str().unwrap_or("");
        if fs.is_empty()
            || name.contains("loop")
            || name.contains("squashfs")
            || name.contains("tmpfs")
            || name.contains("overlay")
        {
            continue;
        }
        let mount = d.mount_point().to_string_lossy().to_string();
        // Skip nested bind/container mounts and transient helper mounts; they
        // duplicate a real filesystem or are always ~100% by construction.
        if mount.contains("/waydroid")
            || mount.contains("/efivars")
            || mount.starts_with("/tmp/.mount")
            || mount.starts_with("/run")
            || mount.starts_with("/var/lib/docker")
            || mount.starts_with("/snap")
        {
            continue;
        }
        let total = d.total_space();
        if total == 0 {
            continue;
        }
        let used = total.saturating_sub(d.available_space());
        let pct = used as f64 / total as f64 * 100.0;
        rows.push((mount, pct, used, total));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    rows.dedup_by(|a, b| a.0 == b.0);
    rows.truncate(area.height as usize);

    if rows.is_empty() {
        return;
    }

    let gb = |b: u64| b as f64 / 1_073_741_824.0;
    // mount(10) + used/total(14) + pct(5) + spacing
    let bar_w = (area.width as usize).saturating_sub(32).max(6);

    let lines: Vec<Line> = rows
        .iter()
        .map(|(mount, pct, used, total)| {
            let col = if *pct > 90.0 {
                theme.red
            } else if *pct > 75.0 {
                theme.yellow
            } else {
                theme.accent
            };
            let filled = (((*pct / 100.0).clamp(0.0, 1.0)) * bar_w as f64).round() as usize;
            let filled = filled.min(bar_w);
            let label: String = mount.chars().take(9).collect();
            Line::from(vec![
                Span::styled(format!("{:<10}", label), Style::default().fg(theme.dim)),
                Span::styled("■".repeat(filled), Style::default().fg(col)),
                Span::styled(
                    "·".repeat(bar_w.saturating_sub(filled)),
                    Style::default().fg(theme.surface),
                ),
                Span::styled(
                    format!(" {:>3.0}%", pct),
                    Style::default().fg(col).add_modifier(Modifier::BOLD),
                ),
                Span::styled(
                    format!(" {:>5.1}/{:<5.1}G", gb(*used), gb(*total)),
                    Style::default().fg(theme.dim),
                ),
            ])
        })
        .collect();

    let top = (area.height.saturating_sub(lines.len() as u16)) / 2;
    f.render_widget(
        Paragraph::new(lines),
        Rect::new(area.x, area.y + top, area.width, area.height - top),
    );
}
