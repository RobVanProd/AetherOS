use std::collections::VecDeque;
use std::fs;

use crate::feed::Priority;

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

/// Kinds of telemetry alerts.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum AlertKind {
    HighCpu,
    HighMemory,
    LowMemory,
    NetworkDown,
    NetworkUp,
    ProcessSpike,
    UptimeMilestone,
}

impl AlertKind {
    pub fn label(&self) -> &'static str {
        match self {
            AlertKind::HighCpu => "High CPU Usage",
            AlertKind::HighMemory => "Memory Spike",
            AlertKind::LowMemory => "Low Memory",
            AlertKind::NetworkDown => "Network Down",
            AlertKind::NetworkUp => "Network Connected",
            AlertKind::ProcessSpike => "Process Spike",
            AlertKind::UptimeMilestone => "Uptime Milestone",
        }
    }
}

/// A telemetry-triggered alert.
pub struct TelemetryAlert {
    pub kind: AlertKind,
    pub message: String,
    pub priority: Priority,
}

/// Keeps a rolling window of telemetry snapshots for trend detection.
pub struct TelemetryHistory {
    snapshots: VecDeque<SysTelemetry>,
    max_snapshots: usize,
    prev_network_up: Option<bool>,
    reported_milestones: Vec<u64>,
}

impl TelemetryHistory {
    pub fn new(max_snapshots: usize) -> Self {
        Self {
            snapshots: VecDeque::new(),
            max_snapshots,
            prev_network_up: None,
            reported_milestones: Vec::new(),
        }
    }

    /// Record a new telemetry snapshot.
    pub fn push(&mut self, snapshot: SysTelemetry) {
        self.snapshots.push_back(snapshot);
        if self.snapshots.len() > self.max_snapshots {
            self.snapshots.pop_front();
        }
    }

    /// Get the most recent snapshot.
    pub fn latest(&self) -> Option<&SysTelemetry> {
        self.snapshots.back()
    }

    /// Average CPU over the last N snapshots.
    pub fn avg_cpu(&self, n: usize) -> f64 {
        let count = n.min(self.snapshots.len());
        if count == 0 {
            return 0.0;
        }
        let sum: f64 = self
            .snapshots
            .iter()
            .rev()
            .take(count)
            .map(|s| s.cpu_percent)
            .sum();
        sum / count as f64
    }

    /// CPU trend over recent snapshots (for sparkline rendering later).
    pub fn cpu_history(&self) -> Vec<f64> {
        self.snapshots.iter().map(|s| s.cpu_percent).collect()
    }

    /// Memory percent history.
    pub fn mem_pct_history(&self) -> Vec<f64> {
        self.snapshots
            .iter()
            .map(|s| {
                if s.mem_total_mb > 0 {
                    let used = s.mem_total_mb.saturating_sub(s.mem_avail_mb);
                    (used as f64 / s.mem_total_mb as f64) * 100.0
                } else {
                    0.0
                }
            })
            .collect()
    }

    /// Check for threshold crossings and generate alerts.
    pub fn check_thresholds(&mut self) -> Vec<TelemetryAlert> {
        let mut alerts = Vec::new();
        let latest = match self.snapshots.back() {
            Some(s) => s.clone(),
            None => return alerts,
        };

        // High CPU: sustained >80% over last 3 readings
        if self.snapshots.len() >= 3 && self.avg_cpu(3) > 80.0 {
            alerts.push(TelemetryAlert {
                kind: AlertKind::HighCpu,
                message: format!(
                    "CPU at {:.0}% (avg {:.0}% over last 3 readings)",
                    latest.cpu_percent,
                    self.avg_cpu(3)
                ),
                priority: Priority::Urgent,
            });
        }

        // Low memory: available < 15%
        if latest.mem_total_mb > 0 {
            let avail_pct =
                (latest.mem_avail_mb as f64 / latest.mem_total_mb as f64) * 100.0;
            if avail_pct < 15.0 {
                alerts.push(TelemetryAlert {
                    kind: AlertKind::LowMemory,
                    message: format!(
                        "Only {:.0}% memory available ({}MB / {}MB)",
                        avail_pct, latest.mem_avail_mb, latest.mem_total_mb
                    ),
                    priority: Priority::Urgent,
                });
            }

            // Memory spike: usage jumped 20%+ in one tick
            if self.snapshots.len() >= 2 {
                let prev = &self.snapshots[self.snapshots.len() - 2];
                let prev_used_pct = if prev.mem_total_mb > 0 {
                    let used = prev.mem_total_mb.saturating_sub(prev.mem_avail_mb);
                    (used as f64 / prev.mem_total_mb as f64) * 100.0
                } else {
                    0.0
                };
                let curr_used_pct = {
                    let used = latest.mem_total_mb.saturating_sub(latest.mem_avail_mb);
                    (used as f64 / latest.mem_total_mb as f64) * 100.0
                };
                if curr_used_pct - prev_used_pct > 20.0 {
                    alerts.push(TelemetryAlert {
                        kind: AlertKind::HighMemory,
                        message: format!(
                            "Memory usage jumped from {:.0}% to {:.0}%",
                            prev_used_pct, curr_used_pct
                        ),
                        priority: Priority::Normal,
                    });
                }
            }
        }

        // Network state change
        let net_up = latest.ip_addr.starts_with("10.")
            || latest.ip_addr.starts_with("192.")
            || latest.ip_addr.starts_with("172.")
            || latest.ip_addr.contains("up");

        if let Some(prev_up) = self.prev_network_up {
            if !prev_up && net_up {
                alerts.push(TelemetryAlert {
                    kind: AlertKind::NetworkUp,
                    message: format!("Network connected: {}", latest.ip_addr),
                    priority: Priority::Normal,
                });
            } else if prev_up && !net_up {
                alerts.push(TelemetryAlert {
                    kind: AlertKind::NetworkDown,
                    message: "Network connection lost".to_string(),
                    priority: Priority::Urgent,
                });
            }
        }
        self.prev_network_up = Some(net_up);

        // Process count spike
        if self.snapshots.len() >= 2 {
            let prev = &self.snapshots[self.snapshots.len() - 2];
            if latest.num_procs > prev.num_procs + 20 {
                alerts.push(TelemetryAlert {
                    kind: AlertKind::ProcessSpike,
                    message: format!(
                        "Process count jumped from {} to {}",
                        prev.num_procs, latest.num_procs
                    ),
                    priority: Priority::Normal,
                });
            }
        }

        // Uptime milestones
        let milestones = [3600, 21600, 86400]; // 1h, 6h, 24h
        for &m in &milestones {
            if latest.uptime_secs >= m && !self.reported_milestones.contains(&m) {
                let label = match m {
                    3600 => "1 hour",
                    21600 => "6 hours",
                    86400 => "24 hours",
                    _ => continue,
                };
                self.reported_milestones.push(m);
                alerts.push(TelemetryAlert {
                    kind: AlertKind::UptimeMilestone,
                    message: format!("System has been running for {}", label),
                    priority: Priority::Low,
                });
            }
        }

        alerts
    }
}

pub fn read_telemetry() -> SysTelemetry {
    let mut t = SysTelemetry::default();

    // Kernel version
    if let Ok(ver) = fs::read_to_string("/proc/version") {
        t.kernel = ver
            .split_whitespace()
            .nth(2)
            .unwrap_or("unknown")
            .to_string();
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
        t.cores = cpuinfo
            .lines()
            .filter(|l| l.starts_with("processor"))
            .count() as u32;
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
