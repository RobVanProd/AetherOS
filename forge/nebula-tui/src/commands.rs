use crate::aurora_client;
use crate::telemetry::SysTelemetry;

use std::process::Command;

/// Execute a command and return the output string.
pub fn execute(cmd: &str, telemetry: &SysTelemetry, aurora: &aurora_client::AuroraStatus) -> String {
    let parts: Vec<&str> = cmd.split_whitespace().collect();
    if parts.is_empty() {
        return String::new();
    }

    match parts[0] {
        "help" => help_text(),
        "sysinfo" => sysinfo_text(telemetry),
        "predict" => {
            if !aurora.connected {
                return "Aurora AI is offline. Start cfcd on host for predictions.".into();
            }
            aurora_client::predict()
        }
        "introspect" => {
            if !aurora.connected {
                return "Aurora AI is offline.".into();
            }
            aurora_client::introspect()
        }
        "learning" => {
            if parts.len() < 2 {
                return "Usage: learning on|off".into();
            }
            match parts[1] {
                "on" | "enable" => aurora_client::set_learning(true),
                "off" | "disable" => aurora_client::set_learning(false),
                _ => "Usage: learning on|off".into(),
            }
        }
        "weights" => {
            if parts.len() >= 2 && parts[1] == "save" {
                aurora_client::save_weights()
            } else {
                "Usage: weights save".into()
            }
        }
        "clear" => {
            // Return a special marker that main can handle
            "__CLEAR__".into()
        }
        "exit" | "quit" => {
            "__QUIT__".into()
        }
        // Shell passthrough — execute via BusyBox
        _ => run_shell(cmd),
    }
}

fn help_text() -> String {
    [
        "╔══════════════════════════════════════════╗",
        "║       NEBULA SHELL — AETHER OS v0.3      ║",
        "╠══════════════════════════════════════════╣",
        "║ System                                    ║",
        "║   sysinfo     System status dashboard     ║",
        "║   ps          Running processes           ║",
        "║   free        Memory usage                ║",
        "║   df          Disk usage                  ║",
        "║   dmesg       Kernel messages             ║",
        "║                                            ║",
        "║ AI / Aurora                                ║",
        "║   predict     Run CFC-JEPA prediction     ║",
        "║   introspect  Model introspection          ║",
        "║   learning on/off  Toggle online learning ║",
        "║   weights save     Save model weights     ║",
        "║                                            ║",
        "║ Files & Network                            ║",
        "║   ls, cat, cp, mv, rm, mkdir              ║",
        "║   ip addr, ping, wget                     ║",
        "║                                            ║",
        "║ Shell                                      ║",
        "║   clear       Clear output                ║",
        "║   exit        Exit Nebula                 ║",
        "║                                            ║",
        "║ Navigation: ↑↓ history, PgUp/PgDn scroll  ║",
        "╚══════════════════════════════════════════╝",
    ]
    .join("\n")
}

fn sysinfo_text(t: &SysTelemetry) -> String {
    let mem_used = t.mem_total_mb.saturating_sub(t.mem_avail_mb);
    format!(
        "══════ AETHER SYSTEM INFO ══════\n\
         Kernel:   {}\n\
         Cores:    {}\n\
         Uptime:   {}s\n\
         CPU:      {:.1}%\n\
         Memory:   {}/{}MB ({:.0}%)\n\
         Procs:    {}\n\
         Network:  {}\n\
         ════════════════════════════════",
        t.kernel,
        t.cores,
        t.uptime_secs,
        t.cpu_percent,
        mem_used,
        t.mem_total_mb,
        if t.mem_total_mb > 0 { (mem_used as f64 / t.mem_total_mb as f64) * 100.0 } else { 0.0 },
        t.num_procs,
        t.ip_addr,
    )
}

fn run_shell(cmd: &str) -> String {
    match Command::new("/bin/sh").args(["-c", cmd]).output() {
        Ok(output) => {
            let mut result = String::new();
            if !output.stdout.is_empty() {
                result.push_str(&String::from_utf8_lossy(&output.stdout));
            }
            if !output.stderr.is_empty() {
                if !result.is_empty() {
                    result.push('\n');
                }
                result.push_str(&String::from_utf8_lossy(&output.stderr));
            }
            if result.is_empty() && !output.status.success() {
                result = format!("Command failed with exit code {}", output.status.code().unwrap_or(-1));
            }
            result.trim_end().to_string()
        }
        Err(e) => format!("Failed to execute: {}", e),
    }
}
