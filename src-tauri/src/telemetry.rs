use crate::ssh_pool::SshPool;
use serde::{Deserialize, Serialize};
use std::time::{Duration, Instant};
use tauri::ipc::Channel;

pub const TELEMETRY_PROBE_CMD: &str = r#"LC_ALL=C sh -c 'echo "---LOAD---"; cat /proc/loadavg 2>/dev/null || sysctl -n vm.loadavg 2>/dev/null; echo "---CPU---"; head -n 1 /proc/stat 2>/dev/null; echo "---MEM---"; grep -E "^(MemTotal|MemAvailable|SwapTotal|SwapFree):" /proc/meminfo 2>/dev/null; echo "---NET---"; cat /proc/net/dev 2>/dev/null; echo "---DISK---"; df -kP / 2>/dev/null | tail -n 1; echo "---PROC---"; (ps -eo pid,pcpu,pmem,comm --sort=-pcpu 2>/dev/null || ps -Ao pid,pcpu,pmem,comm -r 2>/dev/null) | head -n 6'"#;

#[derive(Debug, Clone, Default)]
pub struct RawCpuCounters {
    pub total: u64,
    pub idle: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessItem {
    pub pid: u32,
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub command: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerMetrics {
    pub host_id: String,
    pub load_one: f32,
    pub cpu_percent: f32,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    pub swap_used_mb: u64,
    pub swap_total_mb: u64,
    pub disk_used_gb: f32,
    pub disk_total_gb: f32,
    pub disk_percent: f32,
    pub net_rx_kbps: f32,
    pub net_tx_kbps: f32,
    pub top_processes: Vec<ProcessItem>,
}

pub fn parse_linux_telemetry(
    host_id: &str,
    raw: &str,
    prev: Option<&RawCpuCounters>,
    elapsed_secs: f32,
) -> (ServerMetrics, RawCpuCounters) {
    let mut section = "";
    let mut load_one = 0.0f32;
    let mut cpu_total = 0u64;
    let mut cpu_idle = 0u64;
    let mut mem_total_kb = 0u64;
    let mut mem_avail_kb = 0u64;
    let mut swap_total_kb = 0u64;
    let mut swap_free_kb = 0u64;
    let mut rx_bytes = 0u64;
    let mut tx_bytes = 0u64;
    let mut disk_total_kb = 0u64;
    let mut disk_used_kb = 0u64;
    let mut procs = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("---") && trimmed.ends_with("---") {
            section = trimmed;
            continue;
        }
        match section {
            "---LOAD---" => {
                let cleaned = trimmed.trim_matches(|c| c == '{' || c == '}').trim();
                if let Some(first) = cleaned.split_whitespace().next() {
                    load_one = first.parse().unwrap_or(0.0);
                }
            }
            "---CPU---" => {
                if trimmed.starts_with("cpu ") {
                    let vals: Vec<u64> = trimmed
                        .split_whitespace()
                        .skip(1)
                        .filter_map(|v| v.parse().ok())
                        .collect();
                    cpu_total = vals.iter().sum();
                    cpu_idle =
                        vals.get(3).copied().unwrap_or(0) + vals.get(4).copied().unwrap_or(0);
                }
            }
            "---MEM---" => {
                let mut parts = trimmed.split_whitespace();
                let key = parts.next().unwrap_or("");
                let val: u64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                match key {
                    "MemTotal:" => mem_total_kb = val,
                    "MemAvailable:" => mem_avail_kb = val,
                    "SwapTotal:" => swap_total_kb = val,
                    "SwapFree:" => swap_free_kb = val,
                    _ => {}
                }
            }
            "---NET---" => {
                if let Some((iface, rest)) = trimmed.split_once(':') {
                    let iface = iface.trim();
                    if iface != "lo" {
                        let cols: Vec<u64> = rest
                            .split_whitespace()
                            .filter_map(|v| v.parse().ok())
                            .collect();
                        if cols.len() >= 9 {
                            rx_bytes += cols[0];
                            tx_bytes += cols[8];
                        }
                    }
                }
            }
            "---DISK---" => {
                let cols: Vec<&str> = trimmed.split_whitespace().collect();
                if cols.len() >= 5 {
                    disk_total_kb = cols[1].parse().unwrap_or(0);
                    disk_used_kb = cols[2].parse().unwrap_or(0);
                }
            }
            "---PROC---" => {
                let cols: Vec<&str> = trimmed.split_whitespace().collect();
                if cols.len() >= 4 && cols[0] != "PID" {
                    if let Ok(pid) = cols[0].parse::<u32>() {
                        procs.push(ProcessItem {
                            pid,
                            cpu_pct: cols[1].parse().unwrap_or(0.0),
                            mem_pct: cols[2].parse().unwrap_or(0.0),
                            command: cols[3..].join(" "),
                        });
                    }
                }
            }
            _ => {}
        }
    }

    let (cpu_percent, net_rx_kbps, net_tx_kbps) = if let Some(p) = prev {
        let d_total = cpu_total.saturating_sub(p.total);
        let d_idle = cpu_idle.saturating_sub(p.idle);
        let cpu = if d_total > 0 {
            100.0 * (1.0 - (d_idle as f32 / d_total as f32))
        } else if !procs.is_empty() {
            (procs.iter().map(|pr| pr.cpu_pct).sum::<f32>() / 8.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let dt = elapsed_secs.max(0.1);
        let rx_kbps = (rx_bytes.saturating_sub(p.rx_bytes) as f32 / 1024.0) / dt;
        let tx_kbps = (tx_bytes.saturating_sub(p.tx_bytes) as f32 / 1024.0) / dt;
        (cpu.clamp(0.0, 100.0), rx_kbps, tx_kbps)
    } else {
        (0.0, 0.0, 0.0)
    };

    let mem_used_kb = mem_total_kb.saturating_sub(mem_avail_kb);
    let swap_used_kb = swap_total_kb.saturating_sub(swap_free_kb);
    let disk_percent = if disk_total_kb > 0 {
        (disk_used_kb as f32 / disk_total_kb as f32) * 100.0
    } else {
        0.0
    };

    let metrics = ServerMetrics {
        host_id: host_id.to_string(),
        load_one,
        cpu_percent,
        mem_used_mb: mem_used_kb / 1024,
        mem_total_mb: mem_total_kb / 1024,
        swap_used_mb: swap_used_kb / 1024,
        swap_total_mb: swap_total_kb / 1024,
        disk_used_gb: disk_used_kb as f32 / (1024.0 * 1024.0),
        disk_total_gb: disk_total_kb as f32 / (1024.0 * 1024.0),
        disk_percent,
        net_rx_kbps,
        net_tx_kbps,
        top_processes: procs,
    };

    let counters = RawCpuCounters {
        total: cpu_total,
        idle: cpu_idle,
        rx_bytes,
        tx_bytes,
    };

    (metrics, counters)
}

pub fn parse_linux_snapshot(
    host_id: &str,
    raw: &str,
    prev: Option<&RawCpuCounters>,
    elapsed_secs: f32,
) -> (ServerMetrics, RawCpuCounters) {
    parse_linux_telemetry(host_id, raw, prev, elapsed_secs)
}

async fn probe_local_snapshot() -> Result<String, String> {
    let out = tokio::process::Command::new("sh")
        .arg("-c")
        .arg(TELEMETRY_PROBE_CMD)
        .output()
        .await
        .map_err(|e| e.to_string())?;
    Ok(String::from_utf8_lossy(&out.stdout).to_string())
}

pub fn start_monitoring(
    host_id: String,
    pool: SshPool,
    channel: Channel<ServerMetrics>,
    mut stop_rx: tokio::sync::mpsc::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut prev_counters: Option<RawCpuCounters> = None;
        let mut last_tick = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(2));

        loop {
            tokio::select! {
                _ = stop_rx.recv() => break,
                _ = interval.tick() => {
                    let raw_res = if host_id == "local" {
                        probe_local_snapshot().await
                    } else {
                        if !pool.has_active_session(&host_id).await {
                            break;
                        }
                        pool.exec_command(&host_id, TELEMETRY_PROBE_CMD).await
                    };

                    if let Ok(raw) = raw_res {
                        let elapsed = last_tick.elapsed().as_secs_f32().max(0.1);
                        last_tick = Instant::now();
                        let (metrics, counters) =
                            parse_linux_telemetry(&host_id, &raw, prev_counters.as_ref(), elapsed);
                        prev_counters = Some(counters);
                        if channel.send(metrics).is_err() {
                            break;
                        }
                    }
                }
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computes_accurate_cpu_and_network_deltas() {
        let sample1 = r#"
---LOAD---
0.42 0.31 0.25 1/240 1234
---CPU---
cpu  1000 0 500 8500 0 0 0 0 0 0
---MEM---
MemTotal:       8192000 kB
MemAvailable:   4096000 kB
SwapTotal:      2048000 kB
SwapFree:       1024000 kB
---NET---
  lo: 5000 10 0 0 0 0 0 0 5000 10 0 0 0 0 0 0
eth0: 102400 100 0 0 0 0 0 0 51200 80 0 0 0 0 0 0
---DISK---
/dev/sda1 104857600 52428800 52428800 50% /
---PROC---
  PID %CPU %MEM COMMAND
 1001 24.5  3.2 nginx
"#;
        let sample2 = sample1
            .replace("cpu  1000 0 500 8500", "cpu  1300 0 700 9000")
            .replace("eth0: 102400", "eth0: 307200")
            .replace("51200 80", "153600 120");

        let (_, prev) = parse_linux_telemetry("prod", sample1, None, 2.0);
        let (m2, _) = parse_linux_telemetry("prod", &sample2, Some(&prev), 2.0);

        // Delta total = 11000 - 10000 = 1000, delta idle = 9000 - 8500 = 500 => 50% CPU
        assert!((m2.cpu_percent - 50.0).abs() < 0.1);
        assert_eq!(m2.mem_used_mb, 4000);
        assert_eq!(m2.mem_total_mb, 8000);
        assert_eq!(m2.swap_used_mb, 1000);
        assert_eq!(m2.swap_total_mb, 2000);
        assert!((m2.disk_percent - 50.0).abs() < 0.1);
        // Delta RX = 204800 bytes = 200 KB over 2s => 100 KB/s
        assert!((m2.net_rx_kbps - 100.0).abs() < 0.1);
        // Delta TX = 102400 bytes = 100 KB over 2s => 50 KB/s
        assert!((m2.net_tx_kbps - 50.0).abs() < 0.1);
        assert_eq!(m2.top_processes.len(), 1);
        assert_eq!(m2.top_processes[0].command, "nginx");
    }
}
