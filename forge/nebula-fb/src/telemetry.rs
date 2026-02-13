/// Telemetry — system monitoring, direct copy from nebula-tui with minimal changes.

use std::collections::VecDeque;
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

impl SysTelemetry {
    pub fn mem_used_pct(&self) -> f64 {
        if self.mem_total_mb > 0 {
            let used = self.mem_total_mb.saturating_sub(self.mem_avail_mb);
            (used as f64 / self.mem_total_mb as f64) * 100.0
        } else {
            0.0
        }
    }

    pub fn uptime_str(&self) -> String {
        let s = self.uptime_secs;
        if s >= 3600 {
            format!("{}h {}m", s / 3600, (s % 3600) / 60)
        } else if s >= 60 {
            format!("{}m {}s", s / 60, s % 60)
        } else {
            format!("{}s", s)
        }
    }
}

pub struct TelemetryHistory {
    snapshots: VecDeque<SysTelemetry>,
    max_snapshots: usize,
}

impl TelemetryHistory {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            max_snapshots,
        }
    }

    pub fn push(&mut self, snapshot: SysTelemetry) {
        self.snapshots.push_back(snapshot);
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }
    }

    pub fn latest(&self) -> Option<&SysTelemetry> {
        self.snapshots.back()
    }

    pub fn cpu_history(&self) -> Vec<f64> {
        self.snapshots.iter().map(|s| s.cpu_percent).collect()
    }

    pub fn mem_pct_history(&self) -> Vec<f64> {
        self.snapshots
            .iter()
            .map(|s| s.mem_used_pct())
            .collect()
    }
}

pub fn read_telemetry() -> SysTelemetry {
    let mut t = SysTelemetry::default();

    if let Ok(ver) = fs::read_to_string("/proc/version") {
        t.kernel = ver
            .split_whitespace()
            .nth(2)
            .unwrap_or("unknown")
            .to_string();
    }

    if let Ok(uptime) = fs::read_to_string("/proc/uptime") {
        if let Some(secs) = uptime.split_whitespace().next() {
            t.uptime_secs = secs.parse::<f64>().unwrap_or(0.0) as u64;
        }
    }

    if let Ok(meminfo) = fs::read_to_string("/proc/meminfo") {
        for line in meminfo.lines() {
            if line.starts_with("MemTotal:") {
                t.mem_total_mb = parse_kb(line) / 1024;
            } else if line.starts_with("MemAvailable:") {
                t.mem_avail_mb = parse_kb(line) / 1024;
            }
        }
    }

    if let Ok(cpuinfo) = fs::read_to_string("/proc/cpuinfo") {
        t.cores = cpuinfo
            .lines()
            .filter(|l| l.starts_with("processor"))
            .count() as u32;
    }

    t.cpu_percent = read_cpu_percent();

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
    if let Ok(stat) = fs::read_to_string("/proc/stat") {
        if let Some(line) = stat.lines().next() {
            let vals: Vec<u64> = line
                .split_whitespace()
                .skip(1)
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
    if let Ok(operstate) = fs::read_to_string("/sys/class/net/eth0/operstate") {
        if operstate.trim() == "up" {
            if let Ok(fib) = fs::read_to_string("/proc/net/fib_trie") {
                for line in fib.lines() {
                    let trimmed = line.trim();
                    if trimmed.starts_with("10.")
                        || trimmed.starts_with("192.168.")
                        || trimmed.starts_with("172.")
                    {
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
