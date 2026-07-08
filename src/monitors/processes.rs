use std::collections::{HashMap, HashSet};
use std::fs;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{self, SortField};

#[derive(Debug, Clone)]
struct ProcInfo {
    name: String,
    pid: u32,
    ppid: u32,
    mem_kb: u64,
    cpu_pct: f64,
    state: String,
}

fn read_proc_name(pid: u32) -> String {
    let path = format!("/proc/{}/comm", pid);
    fs::read_to_string(&path)
        .ok()
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "?".to_string())
}

fn read_proc_state(pid: u32) -> String {
    let path = format!("/proc/{}/status", pid);
    if let Ok(content) = fs::read_to_string(&path) {
        for line in content.lines() {
            if let Some(state) = line.strip_prefix("State:") {
                let trimmed = state.trim();
                return trimmed.chars().next().unwrap_or('?').to_string();
            }
        }
    }
    "?".to_string()
}

fn read_proc_vmrss(pid: u32) -> u64 {
    let path = format!("/proc/{}/status", pid);
    if let Ok(content) = fs::read_to_string(&path) {
        for line in content.lines() {
            if let Some(rss) = line.strip_prefix("VmRSS:") {
                let val: String = rss.chars().filter(|c| c.is_ascii_digit()).collect();
                return val.parse::<u64>().unwrap_or(0);
            }
        }
    }
    0
}

fn read_proc_cpu(pid: u32, total_jiffies: f64) -> f64 {
    let path = format!("/proc/{}/stat", pid);
    if let Ok(content) = fs::read_to_string(&path) {
        let parts: Vec<&str> = content.split_whitespace().collect();
        if parts.len() > 21 {
            if let (Ok(utime), Ok(stime)) = (parts[13].parse::<u64>(), parts[14].parse::<u64>()) {
                if total_jiffies > 0.0 {
                    return ((utime + stime) as f64 / total_jiffies) * 100.0;
                }
            }
        }
    }
    0.0
}

fn read_proc_ppid(pid: u32) -> u32 {
    let path = format!("/proc/{}/stat", pid);
    if let Ok(content) = fs::read_to_string(&path) {
        let rest = content.split(')').last().unwrap_or("");
        let parts: Vec<&str> = rest.split_whitespace().collect();
        if parts.len() > 2 {
            return parts[1].parse::<u32>().unwrap_or(0);
        }
    }
    0
}

fn read_total_jiffies() -> f64 {
    if let Ok(content) = fs::read_to_string("/proc/stat") {
        for line in content.lines() {
            if line.starts_with("cpu ") {
                let sum: u64 = line
                    .split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse::<u64>().ok())
                    .sum();
                return sum as f64;
            }
        }
    }
    1.0
}

fn collect_processes(
    sort_field: SortField,
    sort_asc: bool,
    search: &str,
) -> Vec<ProcInfo> {
    let total_jiffies = read_total_jiffies();
    let mut procs: Vec<ProcInfo> = Vec::new();

    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            let name = entry.file_name();
            let name_str = name.to_string_lossy();
            if let Ok(pid) = name_str.parse::<u32>() {
                let mem = read_proc_vmrss(pid);
                if mem > 0 {
                    procs.push(ProcInfo {
                        name: read_proc_name(pid),
                        pid,
                        ppid: read_proc_ppid(pid),
                        mem_kb: mem,
                        cpu_pct: read_proc_cpu(pid, total_jiffies),
                        state: read_proc_state(pid),
                    });
                }
            }
        }
    }

    if !search.is_empty() {
        let lower = search.to_lowercase();
        procs.retain(|p| p.name.to_lowercase().contains(&lower));
    }

    match sort_field {
        SortField::Mem => {
            procs.sort_by(|a, b| {
                if sort_asc {
                    a.mem_kb.cmp(&b.mem_kb)
                } else {
                    b.mem_kb.cmp(&a.mem_kb)
                }
            });
        }
        SortField::Pid => {
            procs.sort_by(|a, b| {
                if sort_asc {
                    a.pid.cmp(&b.pid)
                } else {
                    b.pid.cmp(&a.pid)
                }
            });
        }
        SortField::Name => {
            procs.sort_by(|a, b| {
                if sort_asc {
                    a.name.cmp(&b.name)
                } else {
                    b.name.cmp(&a.name)
                }
            });
        }
        SortField::Cpu => {
            procs.sort_by(|a, b| {
                if sort_asc {
                    a.cpu_pct.total_cmp(&b.cpu_pct as &f64)
                } else {
                    b.cpu_pct.total_cmp(&a.cpu_pct)
                }
            });
        }
    }

    procs
}

// ── Tree building ──────────────────────────────────────────────

#[derive(Debug)]
struct ProcNode {
    info: ProcInfo,
    children: Vec<ProcNode>,
    depth: usize,
}

fn build_tree(procs: &[ProcInfo]) -> Vec<ProcNode> {
    let mut children: HashMap<u32, Vec<usize>> = HashMap::new();
    let pid_set: HashSet<u32> = procs.iter().map(|p| p.pid).collect();

    let mut roots: Vec<&ProcInfo> = Vec::new();
    for (i, p) in procs.iter().enumerate() {
        if p.ppid == 0 || !pid_set.contains(&p.ppid) {
            roots.push(p);
        } else {
            children.entry(p.ppid).or_default().push(i);
        }
    }

    fn build_subtree(
        procs: &[ProcInfo],
        children: &HashMap<u32, Vec<usize>>,
        pid: u32,
        depth: usize,
    ) -> Option<ProcNode> {
        let info = procs.iter().find(|p| p.pid == pid)?;
        let node_children: Vec<ProcNode> = children
            .get(&pid)
            .map(|indices| {
                indices
                    .iter()
                    .filter_map(|&i| build_subtree(procs, children, procs[i].pid, depth + 1))
                    .collect()
            })
            .unwrap_or_default();

        Some(ProcNode {
            info: info.clone(),
            children: node_children,
            depth,
        })
    }

    let root_nodes: Vec<ProcNode> = roots
        .iter()
        .filter_map(|r| build_subtree(procs, &children, r.pid, 0))
        .collect();

    root_nodes
}

#[derive(Debug, Clone)]
struct TreeRow {
    pid: u32,
    name: String,
    mem_kb: u64,
    cpu_pct: f64,
    state: String,
    depth: usize,
    has_children: bool,
    expanded: bool,
}

fn flatten_tree(
    nodes: &[ProcNode],
    collapsed: &HashSet<u32>,
) -> Vec<TreeRow> {
    let mut rows = Vec::new();

    fn walk(
        nodes: &[ProcNode],
        collapsed: &HashSet<u32>,
        rows: &mut Vec<TreeRow>,
    ) {
        for node in nodes {
            let is_collapsed = collapsed.contains(&node.info.pid);
            rows.push(TreeRow {
                pid: node.info.pid,
                name: node.info.name.clone(),
                mem_kb: node.info.mem_kb,
                cpu_pct: node.info.cpu_pct,
                state: node.info.state.clone(),
                depth: node.depth,
                has_children: !node.children.is_empty(),
                expanded: !is_collapsed,
            });
            if !is_collapsed {
                walk(&node.children, collapsed, rows);
            }
        }
    }

    walk(nodes, collapsed, &mut rows);
    rows
}

fn sort_tree(nodes: &mut Vec<ProcNode>, by: SortField, asc: bool) {
    nodes.sort_by(|a, b| {
        let cmp = match by {
            SortField::Mem => a.info.mem_kb.cmp(&b.info.mem_kb),
            SortField::Pid => a.info.pid.cmp(&b.info.pid),
            SortField::Name => a.info.name.cmp(&b.info.name),
            SortField::Cpu => a.info.cpu_pct.total_cmp(&b.info.cpu_pct as &f64),
        };
        if asc {
            cmp
        } else {
            cmp.reverse()
        }
    });
    for child in nodes.iter_mut() {
        sort_tree(&mut child.children, by, asc);
    }
}

// ── Public API ─────────────────────────────────────────────────

pub fn get_pid_at(
    scroll_offset: usize,
    sort_field: SortField,
    sort_asc: bool,
    search: &str,
    tree_mode: bool,
    collapsed: &HashSet<u32>,
) -> Option<u32> {
    if tree_mode {
        let procs = collect_processes(sort_field, sort_asc, search);
        let mut roots = build_tree(&procs);
        sort_tree(&mut roots, sort_field, sort_asc);
        let tree_rows = flatten_tree(&roots, collapsed);
        tree_rows.get(scroll_offset).map(|r| r.pid)
    } else {
        let procs = collect_processes(sort_field, sort_asc, search);
        procs.get(scroll_offset).map(|p| p.pid)
    }
}

/// Format a CPU percentage for display — shows "<0.1" instead of "0.0" for tiny values.
fn fmt_cpu(pct: f64) -> String {
    if pct > 0.0 && pct < 0.05 {
        "<0.1".to_string()
    } else {
        format!("{:>5.1}", pct)
    }
}

/// Format memory KB to human-readable string.
fn fmt_mem(mem_kb: u64) -> String {
    let mb = mem_kb as f64 / 1024.0;
    if mb > 1024.0 {
        format!("{:>7.1}G", mb / 1024.0)
    } else {
        format!("{:>7.0}M", mb)
    }
}

/// Truncate a name to fit, adding "…" if too long.
fn trunc_name(name: &str, width: usize) -> String {
    if name.len() > width {
        format!("{}…", &name[..width.saturating_sub(1)])
    } else {
        format!("{:width$}", name, width = width)
    }
}

/// Get a Style for a state character
fn state_style(ch: &str, theme: &app::Theme) -> Style {
    let color = match ch {
        "R" => theme.green,
        "S" => theme.dim,
        "D" | "Z" => theme.red,
        "T" => theme.yellow,
        _ => theme.dim,
    };
    // Bold for running state
    let modifier = if ch == "R" {
        Modifier::BOLD
    } else {
        Modifier::empty()
    };
    Style::default().fg(color).add_modifier(modifier)
}

/// Generate the prefix string for tree-mode indentation.
fn tree_prefix(depth: usize, has_children: bool, expanded: bool, tree_mode: bool) -> String {
    if !tree_mode {
        return String::new();
    }
    if depth == 0 && has_children {
        return if expanded {
            " ▾ ".to_string()
        } else {
            " ▸ ".to_string()
        };
    }
    if depth > 0 {
        let indent_width = (depth.saturating_sub(1) * 2).min(8);
        let spaces = " ".repeat(indent_width);
        let branch = if has_children {
            if expanded { "▾─" } else { "▸─" }
        } else {
            " ├─"
        };
        return format!("{}{}", spaces, branch);
    }
    String::new()
}

/// Generate a stable fake process list for demo/screenshot mode.
fn generate_demo_procs() -> Vec<ProcInfo> {
    let demos = [
        ("systemd", 1, 0, 4096, 0.3, "S"),
        ("init", 2, 0, 128, 0.0, "S"),
        ("kthreadd", 3, 0, 0, 0.0, "S"),
        ("ksoftirqd/0", 6, 2, 0, 0.1, "S"),
        ("migration/0", 7, 2, 0, 0.0, "S"),
        ("rcu_sched", 9, 2, 0, 0.2, "S"),
        ("shell", 512, 1, 2048, 0.5, "S"),
        ("sshd", 768, 1, 4096, 0.1, "S"),
        ("bash", 1024, 512, 3072, 0.2, "S"),
        ("login", 1536, 1, 4096, 0.0, "S"),
        ("vim", 2048, 1024, 8192, 1.2, "S"),
        ("kitty", 2304, 1024, 16384, 0.8, "S"),
        ("firefox", 3072, 1024, 245760, 4.5, "S"),
        ("Web Content", 3073, 3072, 81920, 3.1, "S"),
        ("Web Content", 3074, 3072, 65536, 2.8, "S"),
        ("GPU Process", 3075, 3072, 49152, 1.5, "S"),
        ("chrome", 4096, 1024, 196608, 3.2, "S"),
        ("Chrome_child", 4097, 4096, 65536, 2.1, "S"),
        ("Chrome_child", 4098, 4096, 49152, 1.8, "S"),
        ("code", 5120, 1024, 184320, 6.7, "S"),
        ("code_helper", 5121, 5120, 32768, 0.5, "S"),
        ("node", 5184, 5120, 45056, 2.3, "S"),
        ("nvim", 5632, 1024, 16384, 0.9, "S"),
        ("spotify", 6144, 1024, 81920, 1.4, "S"),
        ("discord", 6656, 1024, 131072, 2.2, "S"),
        ("kitty", 7168, 1024, 16384, 0.6, "S"),
        ("zsh", 7169, 7168, 4096, 0.1, "S"),
        ("cargo", 7720, 7169, 14336, 8.5, "R"),
        ("rustc", 7721, 7720, 98304, 15.2, "R"),
        ("sway", 8192, 1, 24576, 0.4, "S"),
        ("pipewire", 8448, 1, 16384, 0.3, "S"),
        ("wireplumber", 8704, 1, 8192, 0.2, "S"),
        ("mutter", 8960, 1, 32768, 0.5, "S"),
        ("gnome-shell", 9216, 1, 65536, 1.1, "S"),
        ("containerd", 9728, 1, 20480, 0.3, "S"),
        ("dockerd", 9984, 1, 49152, 0.6, "S"),
        ("vanta", 10001, 1024, 12288, 0.8, "S"),
    ];
    demos
        .iter()
        .map(|&(name, pid, ppid, mem_kb, cpu_pct, state)| ProcInfo {
            name: name.to_string(),
            pid,
            ppid,
            mem_kb,
            cpu_pct,
            state: state.to_string(),
        })
        .collect()
}

pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &app::Theme,
    scroll_offset: usize,
    sort_field: SortField,
    sort_asc: bool,
    search: &str,
    tree_mode: bool,
    collapsed: &HashSet<u32>,
    demo: bool,
) {
    let procs = if demo {
        // Stable fake process list for screenshots
        generate_demo_procs()
    } else {
        collect_processes(sort_field, sort_asc, search)
    };

    let (display_rows, total_items) = if tree_mode {
        let mut roots = build_tree(&procs);
        sort_tree(&mut roots, sort_field, sort_asc);
        let rows = flatten_tree(&roots, collapsed);
        let total = rows.len();
        (rows, total)
    } else {
        let total = procs.len();
        let rows: Vec<TreeRow> = procs
            .iter()
            .map(|p| TreeRow {
                pid: p.pid,
                name: p.name.clone(),
                mem_kb: p.mem_kb,
                cpu_pct: p.cpu_pct,
                state: p.state.clone(),
                depth: 0,
                has_children: false,
                expanded: false,
            })
            .collect();
        (rows, total)
    };

    // Dynamic page size: available rows = area height - 1 (header line)
    let page_size = area.height.saturating_sub(1) as usize;
    let max_scroll = total_items.saturating_sub(page_size);
    let scroll = scroll_offset.min(max_scroll);

    let mut lines: Vec<Line> = Vec::new();

    // ── Header line ──
    let scroll_hint = if total_items > page_size {
        format!(" {:>3}/{}", scroll + 1, total_items)
    } else {
        String::new()
    };

    let sort_cpu = if sort_field == SortField::Cpu {
        if sort_asc { "▴" } else { "▾" }
    } else { "" };
    let sort_mem = if sort_field == SortField::Mem {
        if sort_asc { "▴" } else { "▾" }
    } else { "" };
    let sort_pid = if sort_field == SortField::Pid {
        if sort_asc { "▴" } else { "▾" }
    } else { "" };
    let sort_name = if sort_field == SortField::Name {
        if sort_asc { "▴" } else { "▾" }
    } else { "" };

    let mode_tag = if tree_mode { " [T]" } else { " [F]" };

    // Use a slightly brighter header style
    let hdr_style = Style::default().fg(theme.dim).bg(theme.bg);
    lines.push(Line::from(vec![
        Span::styled(format!("{}{:>5}", sort_pid, "PID"), hdr_style),
        Span::styled(
            format!(" {}{:12}", sort_name, format!("NAME{}", mode_tag)),
            hdr_style,
        ),
        Span::styled(format!(" {}{:>5}", sort_cpu, "CPU"), hdr_style),
        Span::styled(format!(" {}{:>8}", sort_mem, "MEM"), hdr_style),
        Span::styled(format!("  S  {}", scroll_hint), Style::default().fg(theme.dim)),
    ]));

    // ── Data rows ──
    let name_width = 14usize; // slightly wider for full-width panel

    for (i, row) in display_rows.iter().skip(scroll).take(page_size).enumerate() {
        let is_selected = i == 0;

        let mem_str = fmt_mem(row.mem_kb);
        let name_display = trunc_name(&row.name, name_width);

        let cpu_display = fmt_cpu(row.cpu_pct);

        let cpu_color = if row.cpu_pct > 50.0 {
            theme.red
        } else if row.cpu_pct > 20.0 {
            theme.yellow
        } else {
            theme.dim
        };

        // Selected row gets a background + brighter foreground
        let (row_bg, row_fg) = if is_selected {
            (theme.surface, theme.accent)
        } else {
            (theme.bg, theme.text)
        };

        let indicator = if is_selected { "▸ " } else { "  " };

        let prefix = tree_prefix(row.depth, row.has_children, row.expanded, tree_mode);
        let pid_str = format!("{:>5}", row.pid);

        // Build the full row spans
        let mut spans: Vec<Span> = Vec::new();

        // Indicator + PID with bg
        spans.push(Span::styled(
            format!("{}{}", indicator, pid_str),
            Style::default().fg(row_fg).bg(row_bg),
        ));

        // Name + prefix
        let name_part = format!(" {}{}", prefix, name_display);
        spans.push(Span::styled(
            name_part,
            Style::default().fg(row_fg).bg(row_bg),
        ));

        // CPU
        spans.push(Span::styled(
            format!(" {}", cpu_display),
            Style::default().fg(cpu_color).bg(row_bg),
        ));

        // MEM
        spans.push(Span::styled(
            format!(" {}", mem_str),
            Style::default().fg(theme.dim).bg(row_bg),
        ));

        // State
        spans.push(Span::styled(
            format!("  {}", row.state),
            state_style(&row.state, theme).bg(row_bg),
        ));

        lines.push(Line::from(spans));
    }

    // ── Empty states ──
    if display_rows.is_empty() {
        if search.is_empty() {
            lines.push(Line::from(Span::styled(
                " (no process data)",
                Style::default().fg(theme.dim),
            )));
        } else {
            lines.push(Line::from(Span::styled(
                format!(" (no match for \"{}\")", search),
                Style::default().fg(theme.dim),
            )));
        }
    }

    f.render_widget(
        Paragraph::new(lines).style(Style::default().bg(theme.bg)),
        area,
    );
}
