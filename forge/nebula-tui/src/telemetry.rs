use std::fs;

#[derive(Default, Clone)]
pub struct SysTelemetry {
    pub cpu_percent: f64,
    pub mem_total_mb: u64,
    pub mem_avail_mb: u64,
    pub uptime_secs: u64,
    pub num_procs: u32,
    pub ip_addr: String,
    pub kernel: String,
    pub cores: u32,
}

pub fn read_telemetry() -> SysTelemetry {
    let mut t = SysTelemetry::default();

    // Kernel version
    if let Ok(ver) = fs::read_to_string("/proc/version") {
        t.kernel = ver.split_whitespace().nth(2).unwrap_or("unknown").to_string();
    }

    // Uptime
    if let Ok(uptime) = fs::read_to_string("/proc/uptime") {
        if let Some(secs) = uptime.split_whitespace().next() {
            t.uptime_secs = secs.parse::<f64>().unwrap_or(0.0) as u64;
        }
    }

    // Memory
    if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                t.mem_total_mb = parse_kb(line) / 1024;
            } else if line.starts_with("MemAvailable:") {
                t.mem_avail_mb = parse_kb(line) / 1024;
            }
        }
    }

    // CPU count
    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        t.cores = cpuinfo.lines().filter(|l| l.starts_with("processor")).count() as u32;
    }

    // CPU usage (simplified: from /proc/stat)
    t.cpu_percent = read_cpu_percent();

    // Process count
    if let Ok(entries) = fs::read_dir("/proc") {
        t.num_procs = entries
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.file_name()
                    .to_str()
                    .map(|s| s.chars().all(|c| c.is_ascii_digit()))
                    .unwrap_or(false)
            })
            .count() as u32;
    }

    // IP address
    t.ip_addr = read_ip_addr();

    t
}

fn parse_kb(line: &str) -> u64 {
    line.split_whitespace()
        .nth(1)
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0)
}

fn read_cpu_percent() -> f64 {
    // Read /proc/stat to get CPU usage
    // This is a snapshot, not a delta — good enough for a TUI refresh
    if let Ok(stat) = fs::read_to_string("/proc/stat") {
        if let Some(line) = stat.lines().next() {
            let vals: Vec<u64> = line
                .split_whitespace()
                .skip(1) // "cpu"
                .filter_map(|s| s.parse().ok())
                .collect();
            if vals.len() >= 4 {
                let idle = vals[3];
                let total: u64 = vals.iter().sum();
                if total > 0 {
                    return ((total - idle) as f64 / total as f64) * 100.0;
                }
            }
        }
    }
    0.0
}

fn read_ip_addr() -> String {
    // Try to read from /proc/net/fib_trie or parse ip command output
    // Simplest: check /sys/class/net/eth0/
    if let Ok(operstate) = fs::read_to_string("/sys/class/net/eth0/operstate") {
        if operstate.trim() == "up" {
            // Try to find IP from /proc/net/fib_trie
            if let Ok(fib) = fs::read_to_string("/proc/net/fib_trie") {
                // Look for non-loopback addresses
                for line in fib.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("/32 host LOCAL") {
                        // The IP is on the previous line — this is a simplification
                    }
                    if trimmed.starts_with("10.") || trimmed.starts_with("192.168.") || trimmed.starts_with("172.") {
                        if !trimmed.contains('/') {
                            return trimmed.to_string();
                        }
                    }
                }
            }
            return "eth0 up".to_string();
        }
    }
    "no network".to_string()
}
