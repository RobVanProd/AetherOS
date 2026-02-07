use crate::telemetry::SysTelemetry;
use std::process::Command;

/// Help text for local commands.
pub fn help_text() -> String {
    [
        "",
        "  AETHER OS \u{2014} AI-Native Operating System",
        "",
        "  Just type what you want. The OS understands natural language.",
        "",
        "  Examples:",
        "    what is the weather in New York",
        "    show me ~/Documents/notes.txt",
        "    what files do I have about cars",
        "    how long has this system been running",
        "    write a poem about the ocean",
        "",
        "  Local commands:",
        "    sysinfo     System telemetry dashboard",
        "    help        This help screen",
        "    clear       Clear output",
        "    exit        Exit Nebula",
        "",
        "  Shell passthrough:",
        "    !ps aux     Run shell command directly",
        "    !ls -la     Any command prefixed with !",
        "",
        "  Navigation: \u{2191}\u{2193} history, PgUp/PgDn scroll",
        "",
    ]
    .join("\n")
}

/// System info formatted text.
pub fn sysinfo_text(t: &SysTelemetry) -> String {
    let mem_used = t.mem_total_mb.saturating_sub(t.mem_avail_mb);
    format!(
        "\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550} AETHER SYSTEM INFO \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\n\
         Kernel:   {}\n\
         Cores:    {}\n\
         Uptime:   {}s\n\
         CPU:      {:.1}%\n\
         Memory:   {}/{}MB ({:.0}%)\n\
         Procs:    {}\n\
         Network:  {}\n\
         \u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}\u{2550}",
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

/// Run a shell command and return output.
pub fn run_shell(cmd: &str) -> String {
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
