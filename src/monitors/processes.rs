use std::collections::{HashMap, HashSet};
use std::fs;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::SortField;
use crate::theme::Theme;
use crate::widgets::meter;

#[derive(Debug, Clone)]
pub struct ProcInfo {
    pub name: String,
    pub cmdline: String,
    pub pid: u32,
    pub ppid: u32,
    pub mem_kb: u64,
    pub cpu_pct: f64,
    pub state: char,
    pub threads: u64,
    pub uid: u32,
    pub read_bps: f64,
    pub write_bps: f64,
}

struct Prev {
    jiffies: u64,
    read: u64,
    write: u64,
    /// argv never changes for a live PID, so read it once.
    cmdline: String,
}

struct State {
    snapshot: Vec<ProcInfo>,
    prev: HashMap<u32, Prev>,
    prev_total_jiffies: f64,
    prev_time: Option<Instant>,
    total_mem_kb: f64,
}

static STATE: LazyLock<Mutex<State>> = LazyLock::new(|| {
    Mutex::new(State {
        snapshot: Vec::new(),
        prev: HashMap::new(),
        prev_total_jiffies: 0.0,
        prev_time: None,
        total_mem_kb: 1.0,
    })
});

// ── /proc readers ──────────────────────────────────────────────

fn read_total_jiffies() -> f64 {
    fs::read_to_string("/proc/stat")
        .ok()
        .and_then(|c| {
            c.lines().find(|l| l.starts_with("cpu ")).map(|l| {
                l.split_whitespace()
                    .skip(1)
                    .filter_map(|s| s.parse::<u64>().ok())
                    .sum::<u64>() as f64
            })
        })
        .unwrap_or(1.0)
}

/// Parse /proc/[pid]/status once for the fields we need.
fn read_status(pid: u32) -> Option<(String, char, u64, u64, u32)> {
    let content = fs::read_to_string(format!("/proc/{}/status", pid)).ok()?;
    let (mut name, mut state, mut rss, mut threads, mut uid) =
        (String::new(), '?', 0u64, 0u64, 0u32);
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("Name:") {
            name = v.trim().to_string();
        } else if let Some(v) = line.strip_prefix("State:") {
            state = v.trim().chars().next().unwrap_or('?');
        } else if let Some(v) = line.strip_prefix("VmRSS:") {
            rss = v.split_whitespace().next()?.parse().ok()?;
        } else if let Some(v) = line.strip_prefix("Threads:") {
            threads = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("Uid:") {
            uid = v.split_whitespace().next()?.parse().unwrap_or(0);
        }
    }
    Some((name, state, rss, threads, uid))
}

/// (ppid, utime+stime) from /proc/[pid]/stat. The comm field can contain
/// spaces, so split after the closing paren.
fn read_stat(pid: u32) -> Option<(u32, u64)> {
    let content = fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let rest = content.rsplit_once(')')?.1;
    let p: Vec<&str> = rest.split_whitespace().collect();
    // rest[0]=state rest[1]=ppid ... rest[11]=utime rest[12]=stime
    if p.len() < 13 {
        return None;
    }
    let ppid = p[1].parse().ok()?;
    let ut: u64 = p[11].parse().ok()?;
    let st: u64 = p[12].parse().ok()?;
    Some((ppid, ut + st))
}

fn read_io(pid: u32) -> (u64, u64) {
    let Ok(content) = fs::read_to_string(format!("/proc/{}/io", pid)) else {
        return (0, 0);
    };
    let (mut r, mut w) = (0, 0);
    for line in content.lines() {
        if let Some(v) = line.strip_prefix("read_bytes:") {
            r = v.trim().parse().unwrap_or(0);
        } else if let Some(v) = line.strip_prefix("write_bytes:") {
            w = v.trim().parse().unwrap_or(0);
        }
    }
    (r, w)
}

fn read_cmdline(pid: u32) -> String {
    fs::read(format!("/proc/{}/cmdline", pid))
        .ok()
        .map(|b| {
            String::from_utf8_lossy(&b)
                .split('\0')
                .filter(|s| !s.is_empty())
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default()
}

/// Walk /proc once and rebuild the snapshot. Called on the tick, never per frame.
pub fn sample(total_mem_bytes: u64) {
    let now = Instant::now();
    let total_jiffies = read_total_jiffies();
    let ncpu = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1) as f64;

    // Pull the previous counters out and drop the lock: the /proc walk below
    // takes tens of milliseconds and the render thread must not wait on it.
    let (prev, prev_total, prev_time, cap) = {
        let mut st = STATE.lock().unwrap();
        (
            std::mem::take(&mut st.prev),
            st.prev_total_jiffies,
            st.prev_time,
            st.snapshot.len(),
        )
    };
    let dt = prev_time
        .map(|t| now.duration_since(t).as_secs_f64())
        .unwrap_or(0.0);
    let d_total = total_jiffies - prev_total;

    let mut procs = Vec::with_capacity(cap + 64);
    let mut next_prev: HashMap<u32, Prev> = HashMap::with_capacity(prev.len() + 64);

    let Ok(entries) = fs::read_dir("/proc") else {
        return;
    };
    for entry in entries.flatten() {
        let Ok(pid) = entry.file_name().to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Some((name, state, mem_kb, threads, uid)) = read_status(pid) else {
            continue;
        };
        if mem_kb == 0 {
            continue; // kernel threads
        }
        let Some((ppid, jiffies)) = read_stat(pid) else {
            continue;
        };
        let (read, write) = read_io(pid);

        let (cpu_pct, read_bps, write_bps) = match prev.get(&pid) {
            Some(p) if d_total > 0.0 && dt > 0.0 => (
                jiffies.saturating_sub(p.jiffies) as f64 / d_total * 100.0 * ncpu,
                read.saturating_sub(p.read) as f64 / dt,
                write.saturating_sub(p.write) as f64 / dt,
            ),
            _ => (0.0, 0.0, 0.0),
        };
        let cmdline = match prev.get(&pid) {
            Some(p) if !p.cmdline.is_empty() => p.cmdline.clone(),
            _ => read_cmdline(pid),
        };
        next_prev.insert(
            pid,
            Prev {
                jiffies,
                read,
                write,
                cmdline: cmdline.clone(),
            },
        );
        procs.push(ProcInfo {
            cmdline,
            name,
            pid,
            ppid,
            mem_kb,
            cpu_pct,
            state,
            threads,
            uid,
            read_bps,
            write_bps,
        });
    }

    let mut st = STATE.lock().unwrap();
    st.snapshot = procs;
    st.prev = next_prev;
    st.prev_total_jiffies = total_jiffies;
    st.prev_time = Some(now);
    st.total_mem_kb = (total_mem_bytes as f64 / 1024.0).max(1.0);
}

/// Name of a live PID from the last snapshot.
pub fn name_of(pid: u32) -> Option<String> {
    STATE
        .lock()
        .unwrap()
        .snapshot
        .iter()
        .find(|p| p.pid == pid)
        .map(|p| p.name.clone())
}

/// uid → login name from /etc/passwd, read once.
static USERS: LazyLock<HashMap<u32, String>> = LazyLock::new(|| {
    fs::read_to_string("/etc/passwd")
        .unwrap_or_default()
        .lines()
        .filter_map(|l| {
            let mut f = l.split(':');
            let name = f.next()?;
            let uid = f.nth(1)?.parse().ok()?;
            Some((uid, name.to_string()))
        })
        .collect()
});

fn user_name(uid: u32) -> String {
    USERS.get(&uid).cloned().unwrap_or_else(|| uid.to_string())
}

pub fn count() -> usize {
    STATE.lock().unwrap().snapshot.len()
}

/// Top `n` by CPU for the dashboard preview.
pub fn top_by_cpu(n: usize) -> Vec<ProcInfo> {
    let st = STATE.lock().unwrap();
    let mut v: Vec<&ProcInfo> = st.snapshot.iter().collect();
    v.sort_by(|a, b| {
        b.cpu_pct
            .total_cmp(&a.cpu_pct)
            .then(b.mem_kb.cmp(&a.mem_kb))
    });
    v.into_iter().take(n).cloned().collect()
}

fn cmp(a: &ProcInfo, b: &ProcInfo, by: SortField, asc: bool) -> std::cmp::Ordering {
    let o = match by {
        SortField::Mem => a.mem_kb.cmp(&b.mem_kb),
        SortField::Pid => a.pid.cmp(&b.pid),
        SortField::Name => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
        SortField::Cpu => a.cpu_pct.total_cmp(&b.cpu_pct),
    };
    if asc {
        o
    } else {
        o.reverse()
    }
}

/// Filtered + sorted view of the current snapshot.
fn view(sort_field: SortField, sort_asc: bool, search: &str) -> (Vec<ProcInfo>, f64) {
    let st = STATE.lock().unwrap();
    let lower = search.to_lowercase();
    let mut procs: Vec<ProcInfo> = st
        .snapshot
        .iter()
        .filter(|p| {
            lower.is_empty()
                || p.name.to_lowercase().contains(&lower)
                || p.cmdline.to_lowercase().contains(&lower)
                || p.pid.to_string() == lower
        })
        .cloned()
        .collect();
    procs.sort_by(|a, b| cmp(a, b, sort_field, sort_asc));
    (procs, st.total_mem_kb)
}

// ── Tree ───────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct Row {
    info: ProcInfo,
    depth: usize,
    has_children: bool,
    expanded: bool,
}

/// Depth-first flatten of the parent/child graph, children sorted like the
/// flat list. Processes whose parent isn't in the (possibly filtered) set
/// become roots.
fn tree_rows(procs: &[ProcInfo], by: SortField, asc: bool, collapsed: &HashSet<u32>) -> Vec<Row> {
    let pids: HashSet<u32> = procs.iter().map(|p| p.pid).collect();
    let mut children: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut roots: Vec<usize> = Vec::new();
    for (i, p) in procs.iter().enumerate() {
        if p.ppid == 0 || p.ppid == p.pid || !pids.contains(&p.ppid) {
            roots.push(i);
        } else {
            children.entry(p.ppid).or_default().push(i);
        }
    }
    let sort_idx = |v: &mut Vec<usize>| v.sort_by(|&a, &b| cmp(&procs[a], &procs[b], by, asc));
    sort_idx(&mut roots);
    for v in children.values_mut() {
        sort_idx(v);
    }

    let mut out = Vec::with_capacity(procs.len());
    // Explicit stack: (index, depth). Pushed in reverse so pop order = sorted order.
    let mut stack: Vec<(usize, usize)> = roots.iter().rev().map(|&i| (i, 0)).collect();
    while let Some((i, depth)) = stack.pop() {
        let p = &procs[i];
        let kids = children.get(&p.pid);
        let has_children = kids.is_some_and(|k| !k.is_empty());
        let expanded = !collapsed.contains(&p.pid);
        out.push(Row {
            info: p.clone(),
            depth,
            has_children,
            expanded,
        });
        if has_children && expanded {
            for &k in kids.unwrap().iter().rev() {
                stack.push((k, depth + 1));
            }
        }
    }
    out
}

fn rows_for(
    sort_field: SortField,
    sort_asc: bool,
    search: &str,
    tree_mode: bool,
    collapsed: &HashSet<u32>,
) -> (Vec<Row>, f64) {
    let (procs, total_mem) = view(sort_field, sort_asc, search);
    let rows = if tree_mode {
        tree_rows(&procs, sort_field, sort_asc, collapsed)
    } else {
        procs
            .into_iter()
            .map(|info| Row {
                info,
                depth: 0,
                has_children: false,
                expanded: false,
            })
            .collect()
    };
    (rows, total_mem)
}

/// PID at a given row of the current view — used for select/kill/collapse.
pub fn get_pid_at(
    index: usize,
    sort_field: SortField,
    sort_asc: bool,
    search: &str,
    tree_mode: bool,
    collapsed: &HashSet<u32>,
) -> Option<u32> {
    rows_for(sort_field, sort_asc, search, tree_mode, collapsed)
        .0
        .get(index)
        .map(|r| r.info.pid)
}

// ── Formatting ─────────────────────────────────────────────────

fn fmt_cpu(pct: f64) -> String {
    if pct > 0.0 && pct < 0.05 {
        " <0.1".to_string()
    } else {
        format!("{:>5.1}", pct)
    }
}

fn fmt_rss(mem_kb: u64) -> String {
    let mb = mem_kb as f64 / 1024.0;
    if mb >= 1024.0 {
        format!("{:.1}G", mb / 1024.0)
    } else {
        format!("{:.0}M", mb)
    }
}

fn fmt_io(bps: f64) -> String {
    if bps < 1.0 {
        "--".to_string()
    } else if bps >= 1e9 {
        format!("{:.1}G", bps / 1e9)
    } else if bps >= 1e6 {
        format!("{:.0}M", bps / 1e6)
    } else if bps >= 1e3 {
        format!("{:.0}K", bps / 1e3)
    } else {
        format!("{:.0}B", bps)
    }
}

fn pad(s: &str, width: usize) -> String {
    format!("{:<w$}", meter::ellipsize(s, width), w = width)
}

fn state_style(ch: char, theme: &Theme) -> Style {
    match ch {
        'R' => Style::default()
            .fg(theme.green)
            .add_modifier(Modifier::BOLD),
        'D' | 'Z' => Style::default().fg(theme.red),
        'T' | 't' => Style::default().fg(theme.yellow),
        _ => Style::default().fg(theme.dim),
    }
}

fn tree_prefix(r: &Row) -> String {
    let glyph = match (r.has_children, r.expanded) {
        (true, true) => "▾ ",
        (true, false) => "▸ ",
        (false, _) => "· ",
    };
    format!("{}{}", "  ".repeat(r.depth.min(6)), glyph)
}

fn cmd_basename(cmd: &str) -> &str {
    let first = cmd.split_whitespace().next().unwrap_or(cmd);
    first.rsplit('/').next().unwrap_or(first)
}

// ── Render ─────────────────────────────────────────────────────

#[allow(clippy::too_many_arguments)]
pub fn render(
    f: &mut Frame,
    area: Rect,
    theme: &Theme,
    scroll_offset: usize,
    sort_field: SortField,
    sort_asc: bool,
    search: &str,
    search_active: bool,
    tree_mode: bool,
    collapsed: &HashSet<u32>,
    selected_pid: Option<u32>,
    compact_cmd: bool,
) {
    // header + at least one row + detail strip
    if area.height < 4 || area.width < 40 {
        return;
    }
    let (rows, total_mem_kb) = rows_for(sort_field, sort_asc, search, tree_mode, collapsed);

    let detail_h = 2u16;
    let table_h = area.height - detail_h;
    let table = Rect {
        height: table_h,
        ..area
    };
    let detail = Rect::new(area.x, area.y + table_h, area.width, detail_h);

    // ── Columns ──
    let w = area.width as usize;
    let (c_pid, c_cpu, c_mem, c_rss, c_st, c_usr, c_thr) = (7usize, 6, 6, 7, 2, 8, 4);
    let fixed = c_pid + c_cpu + c_mem + c_rss + c_st + c_usr + c_thr + 7; // 7 separators
    let show_io = w >= fixed + 8 + 14 + 16;
    let io_w = if show_io { 14 } else { 0 };
    let remaining = w.saturating_sub(fixed + io_w + 1);
    let c_name = (remaining * 2 / 5).clamp(8.min(remaining), 28);
    let c_cmd = remaining.saturating_sub(c_name + 1);

    let page = table_h.saturating_sub(1) as usize;
    let total = rows.len();
    let max_scroll = total.saturating_sub(page);
    let scroll = scroll_offset.min(max_scroll);

    // ── Header ──
    let hs = Style::default().fg(theme.dim).bg(theme.surface);
    let hs_on = Style::default()
        .fg(theme.accent)
        .bg(theme.surface)
        .add_modifier(Modifier::BOLD);
    let arrow = if sort_asc { "▴" } else { "▾" };
    let h = |label: &str, width: usize, field: Option<SortField>, right: bool| -> Span<'static> {
        let on = field == Some(sort_field);
        let text = if on {
            format!("{}{}", arrow, label)
        } else {
            label.to_string()
        };
        let s = if right {
            format!("{:>w$}", text, w = width)
        } else {
            format!("{:<w$}", text, w = width)
        };
        Span::styled(s, if on { hs_on } else { hs })
    };
    let mut hdr = vec![
        h("PID", c_pid, Some(SortField::Pid), true),
        Span::styled(" ", hs),
        h(
            if tree_mode { "NAME ⌥tree" } else { "NAME" },
            c_name,
            Some(SortField::Name),
            false,
        ),
        Span::styled(" ", hs),
        h("CPU%", c_cpu, Some(SortField::Cpu), true),
        Span::styled(" ", hs),
        h("MEM%", c_mem, Some(SortField::Mem), true),
        Span::styled(" ", hs),
        h("RSS", c_rss, None, true),
        Span::styled(" ", hs),
        h("S", c_st, None, true),
        Span::styled(" ", hs),
        h("USER", c_usr, None, true),
        Span::styled(" ", hs),
        h("THR", c_thr, None, true),
    ];
    if show_io {
        hdr.push(Span::styled(" ", hs));
        hdr.push(h("R/s", 6, None, true));
        hdr.push(Span::styled(" ", hs));
        hdr.push(h("W/s", 6, None, true));
    }
    if c_cmd > 4 {
        hdr.push(Span::styled(" ", hs));
        hdr.push(h("COMMAND", c_cmd, None, false));
    }
    let used: usize = hdr.iter().map(|s| s.content.chars().count()).sum();
    if used < w {
        hdr.push(Span::styled(" ".repeat(w - used), hs));
    }
    let mut lines = vec![Line::from(hdr)];

    // ── Rows ──
    for (i, r) in rows.iter().skip(scroll).take(page).enumerate() {
        let p = &r.info;
        let is_sel = selected_pid == Some(p.pid) || (selected_pid.is_none() && i == 0);
        let mem_pct = (p.mem_kb as f64 / total_mem_kb * 100.0).clamp(0.0, 100.0);
        let (bg, fg) = if is_sel {
            (theme.surface, theme.accent)
        } else {
            (theme.bg, theme.text)
        };
        let base = Style::default().bg(bg);
        let cpu_col = if p.cpu_pct >= 50.0 {
            theme.red
        } else if p.cpu_pct >= 10.0 {
            theme.yellow
        } else if p.cpu_pct >= 1.0 {
            theme.text
        } else {
            theme.dim
        };
        let mem_col = if mem_pct >= 20.0 {
            theme.red
        } else if mem_pct >= 5.0 {
            theme.yellow
        } else {
            theme.dim
        };

        let name = if tree_mode {
            format!("{}{}", tree_prefix(r), p.name)
        } else {
            p.name.clone()
        };
        let user = meter::ellipsize(&user_name(p.uid), c_usr);

        let mut spans = vec![
            Span::styled(
                format!(
                    "{}{:>w$}",
                    if is_sel { "▸" } else { " " },
                    p.pid,
                    w = c_pid - 1
                ),
                base.fg(fg),
            ),
            Span::styled(" ", base),
            Span::styled(
                pad(&name, c_name),
                base.fg(fg).add_modifier(if is_sel {
                    Modifier::BOLD
                } else {
                    Modifier::empty()
                }),
            ),
            Span::styled(" ", base),
            Span::styled(format!("{:>6}", fmt_cpu(p.cpu_pct)), base.fg(cpu_col)),
            Span::styled(" ", base),
            Span::styled(format!("{:>5.1}%", mem_pct), base.fg(mem_col)),
            Span::styled(" ", base),
            Span::styled(format!("{:>7}", fmt_rss(p.mem_kb)), base.fg(theme.dim)),
            Span::styled(" ", base),
            Span::styled(
                format!("{:>2}", p.state),
                state_style(p.state, theme).bg(bg),
            ),
            Span::styled(" ", base),
            Span::styled(
                format!("{:>w$}", user, w = c_usr),
                base.fg(if p.uid == 0 {
                    theme.yellow
                } else {
                    theme.secondary
                }),
            ),
            Span::styled(" ", base),
            Span::styled(format!("{:>4}", p.threads.min(9999)), base.fg(theme.dim)),
        ];
        if show_io {
            spans.push(Span::styled(
                format!(" {:>6}", fmt_io(p.read_bps)),
                base.fg(theme.secondary),
            ));
            spans.push(Span::styled(
                format!(" {:>6}", fmt_io(p.write_bps)),
                base.fg(theme.secondary),
            ));
        }
        if c_cmd > 4 {
            let raw = if p.cmdline.is_empty() {
                p.name.as_str()
            } else {
                p.cmdline.as_str()
            };
            let cmd = if compact_cmd && !is_sel {
                cmd_basename(raw)
            } else {
                raw
            };
            spans.push(Span::styled(" ", base));
            spans.push(Span::styled(
                pad(cmd, c_cmd),
                base.fg(if is_sel { theme.secondary } else { theme.dim }),
            ));
        }
        let used: usize = spans.iter().map(|s| s.content.chars().count()).sum();
        if used < w {
            spans.push(Span::styled(" ".repeat(w - used), base));
        }
        lines.push(Line::from(spans));
    }

    if rows.is_empty() {
        let msg = if search.is_empty() {
            "  collecting…".to_string()
        } else {
            format!("  no match for \"{}\"", search)
        };
        lines.push(Line::from(Span::styled(
            msg,
            Style::default().fg(theme.dim),
        )));
    }
    f.render_widget(Paragraph::new(lines), table);

    // ── Detail strip ──
    let sel = selected_pid
        .and_then(|pid| rows.iter().find(|r| r.info.pid == pid))
        .or_else(|| rows.get(scroll));
    let ds = Style::default().fg(theme.dim).bg(theme.surface);
    let mut l1: Vec<Span> = Vec::new();
    if search_active || !search.is_empty() {
        l1.push(Span::styled(" / ", ds.fg(theme.accent)));
        l1.push(Span::styled(
            search.to_string(),
            ds.fg(theme.text).add_modifier(Modifier::BOLD),
        ));
        if search_active {
            l1.push(Span::styled("▏", ds.fg(theme.accent)));
        }
        l1.push(Span::styled(
            format!("  {} match{}", total, if total == 1 { "" } else { "es" }),
            ds,
        ));
    } else if let Some(r) = sel {
        let p = &r.info;
        let mem_pct = p.mem_kb as f64 / total_mem_kb * 100.0;
        l1.push(Span::styled(
            format!(" {} ", p.name),
            ds.fg(theme.text).add_modifier(Modifier::BOLD),
        ));
        l1.push(Span::styled(
            format!(
                "pid {}  ppid {}  cpu {}%  mem {:.1}% ({})  thr {}  io ↓{} ↑{}   {}/{} · {} procs",
                p.pid,
                p.ppid,
                fmt_cpu(p.cpu_pct).trim(),
                mem_pct,
                fmt_rss(p.mem_kb),
                p.threads,
                fmt_io(p.read_bps),
                fmt_io(p.write_bps),
                scroll + 1,
                total,
                count(),
            ),
            ds,
        ));
    }
    let l2 = sel
        .map(|r| {
            let raw = if r.info.cmdline.is_empty() {
                r.info.name.as_str()
            } else {
                r.info.cmdline.as_str()
            };
            Line::from(Span::styled(
                format!(" {}", meter::ellipsize(raw, w.saturating_sub(2))),
                ds.fg(theme.secondary),
            ))
        })
        .unwrap_or_default();
    f.render_widget(
        Paragraph::new(vec![Line::from(l1), l2]).style(Style::default().bg(theme.surface)),
        detail,
    );
}
