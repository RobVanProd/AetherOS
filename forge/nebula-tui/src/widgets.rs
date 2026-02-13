/// ASCII bar chart renderer.
/// Returns a string like: [||||||||..........] 42%
pub fn mini_bar(value: f64, max: f64, width: usize) -> String {
    let pct = if max > 0.0 { value / max } else { 0.0 };
    let filled = (pct * width as f64).round() as usize;
    let empty = width.saturating_sub(filled);
    format!(
        "[{}{}] {:.0}%",
        "|".repeat(filled),
        ".".repeat(empty),
        pct * 100.0
    )
}

/// Sparkline renderer using Unicode block characters.
/// Takes a slice of values (0.0-100.0) and renders a single-row trend line.
/// Characters: _ . - ' ^ " for 6 levels of height.
pub fn sparkline(values: &[f64], width: usize) -> String {
    if values.is_empty() {
        return " ".repeat(width);
    }

    let chars = ['_', '.', '-', '\'', '^', '"'];

    // Take the last `width` values, or pad with the first value
    let start = if values.len() > width {
        values.len() - width
    } else {
        0
    };
    let slice = &values[start..];

    let min = slice.iter().cloned().fold(f64::INFINITY, f64::min);
    let max = slice.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    let range = max - min;

    let mut result = String::new();
    for &v in slice {
        let level = if range > 0.0 {
            ((v - min) / range * 5.0).round() as usize
        } else {
            2 // middle
        };
        result.push(chars[level.min(5)]);
    }

    // Pad if needed
    while result.len() < width {
        result.insert(0, ' ');
    }

    result
}

/// Progress bar renderer.
/// Returns: [=====>     ] 55%
pub fn progress_bar(progress: f64, width: usize) -> String {
    let inner = width.saturating_sub(2);
    let filled = (progress * inner as f64).round() as usize;
    let empty = inner.saturating_sub(filled + 1);
    let arrow = if filled < inner { ">" } else { "" };
    format!(
        "[{}{}{}]",
        "=".repeat(filled),
        arrow,
        " ".repeat(empty)
    )
}

/// Key-value pair renderer. Returns lines with aligned values.
/// Input: [("Key", "Value"), ...]
pub fn key_value_lines(pairs: &[(&str, &str)], key_width: usize) -> Vec<String> {
    pairs
        .iter()
        .map(|(k, v)| {
            let pad = key_width.saturating_sub(k.len());
            format!("{}{} {}", k, " ".repeat(pad), v)
        })
        .collect()
}

/// Relative time formatting.
pub fn relative_time(secs: u64) -> String {
    if secs < 5 {
        "just now".to_string()
    } else if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        let m = secs / 60;
        let s = secs % 60;
        if s > 0 {
            format!("{}m{}s ago", m, s)
        } else {
            format!("{}m ago", m)
        }
    } else {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m > 0 {
            format!("{}h{}m ago", h, m)
        } else {
            format!("{}h ago", h)
        }
    }
}
