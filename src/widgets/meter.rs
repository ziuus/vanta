/// Smooth horizontal bar: eighth-block resolution so a 20-cell meter has 160
/// distinct fill levels instead of 20.
const EIGHTHS: [&str; 9] = [" ", "▏", "▎", "▍", "▌", "▋", "▊", "▉", "█"];

/// Filled portion of a `width`-cell bar for `frac` in 0..=1.
pub fn bar(frac: f64, width: usize) -> String {
    if width == 0 {
        return String::new();
    }
    let levels = width * 8;
    let filled = (frac.clamp(0.0, 1.0) * levels as f64).round() as usize;
    (0..width)
        .map(|i| EIGHTHS[filled.saturating_sub(i * 8).min(8)])
        .collect()
}

/// Track-style bar: heavy line for the filled part, light for the rest.
/// Reads well in tight rows where a solid block would be too loud.
pub fn track(frac: f64, width: usize) -> (String, String) {
    let filled = ((frac.clamp(0.0, 1.0)) * width as f64).round() as usize;
    let filled = filled.min(width);
    ("━".repeat(filled), "─".repeat(width - filled))
}

pub fn fmt_bytes(b: u64) -> String {
    const GIB: f64 = 1_073_741_824.0;
    const MIB: f64 = 1_048_576.0;
    let bf = b as f64;
    if bf >= GIB {
        format!("{:.1}G", bf / GIB)
    } else if bf >= MIB {
        format!("{:.0}M", bf / MIB)
    } else {
        format!("{}K", b / 1024)
    }
}

/// Human rate for a value in KiB/s.
pub fn fmt_kbps(kbps: f64) -> String {
    if kbps >= 1024.0 {
        format!("{:.1} MB/s", kbps / 1024.0)
    } else if kbps >= 1.0 {
        format!("{:.0} KB/s", kbps)
    } else {
        format!("{:.0} B/s", kbps * 1024.0)
    }
}

/// Truncate to `max` chars, appending an ellipsis when cut. Char-based so
/// non-ASCII input can never panic on a byte boundary.
pub fn ellipsize(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max - 1).collect();
    out.push('…');
    out
}
