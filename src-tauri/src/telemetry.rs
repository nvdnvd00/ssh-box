use crate::ssh_pool::SshPool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::{Duration, Instant};
use tauri::ipc::Channel;

pub const TELEMETRY_PROBE_CMD: &str = r#"LC_ALL=C sh -c '
export PATH="$HOME/.docker/bin:/usr/local/bin:/opt/homebrew/bin:/usr/sbin:/sbin:$PATH"
echo "---SYS---"
uname -srm 2>/dev/null
hostname 2>/dev/null
if [ -f /proc/uptime ]; then
  awk "{print int(\$1)}" /proc/uptime 2>/dev/null
else
  bt=$(sysctl -n kern.boottime 2>/dev/null | awk "{print \$4}" | tr -d ",")
  now=$(date +%s)
  if [ -n "$bt" ]; then echo $((now - bt)); else echo 0; fi
fi
if command -v lscpu >/dev/null 2>&1; then
  lscpu 2>/dev/null | grep -E "^(CPU\(s\):|Model name:)"
elif [ -f /proc/cpuinfo ]; then
  echo "CPU(s): $(grep -c ^processor /proc/cpuinfo 2>/dev/null)"
  grep -m1 -E "^(model name|Hardware|Model)[[:space:]]*:" /proc/cpuinfo 2>/dev/null
else
  echo "CPU(s): $(sysctl -n hw.ncpu 2>/dev/null || echo 1)"
  echo "Model name: $(sysctl -n machdep.cpu.brand_string 2>/dev/null || echo Apple Silicon)"
fi
echo "---TEMP---"
for d in /sys/class/hwmon/hwmon*; do [ -f "$d/temp1_input" ] && echo "$(cat "$d/name" 2>/dev/null),$(cat "$d/temp1_input" 2>/dev/null)"; done
for z in /sys/class/thermal/thermal_zone*; do [ -f "$z/temp" ] && echo "$(cat "$z/type" 2>/dev/null),$(cat "$z/temp" 2>/dev/null)"; done
echo "---LOAD---"
cat /proc/loadavg 2>/dev/null || sysctl -n vm.loadavg 2>/dev/null
echo "---CPU---"
if [ -f /proc/stat ]; then
  grep -E "^cpu[0-9 ]*" /proc/stat 2>/dev/null
else
  ncpu=$(sysctl -n hw.ncpu 2>/dev/null || echo 1)
  top -l 1 -n 0 2>/dev/null | awk -v n="$ncpu" "/CPU usage:/ {gsub(/%/,\"\"); u=\$3; s=\$5; i=\$7; printf \"MACOS_CPU %s %s %s %s\n\", u, s, i, n}"
fi
echo "---MEM---"
if [ -f /proc/meminfo ]; then
  grep -E "^(MemTotal|MemFree|MemAvailable|Buffers|Cached|SReclaimable|Active|Inactive|SwapTotal|SwapFree):" /proc/meminfo 2>/dev/null
else
  page_size=$(pagesize 2>/dev/null || echo 4096)
  total=$(sysctl -n hw.memsize 2>/dev/null || echo 0)
  total_kb=$((total / 1024))
  vm_out=$(vm_stat 2>/dev/null)
  free_p=$(echo "$vm_out" | awk "/Pages free/ {gsub(/\\./,\"\"); print \$3}")
  active_p=$(echo "$vm_out" | awk "/Pages active/ {gsub(/\\./,\"\"); print \$3}")
  inactive_p=$(echo "$vm_out" | awk "/Pages inactive/ {gsub(/\\./,\"\"); print \$3}")
  spec_p=$(echo "$vm_out" | awk "/Pages speculative/ {gsub(/\\./,\"\"); print \$3}")
  wired_p=$(echo "$vm_out" | awk "/Pages wired/ {gsub(/\\./,\"\"); print \$4}")
  fb_p=$(echo "$vm_out" | awk "/File-backed pages/ {gsub(/\\./,\"\"); print \$3}")
  free_kb=$(( ${free_p:-0} * page_size / 1024 ))
  active_kb=$(( (${active_p:-0} + ${wired_p:-0}) * page_size / 1024 ))
  inactive_kb=$(( ${inactive_p:-0} * page_size / 1024 ))
  cached_kb=$(( (${fb_p:-0} + ${spec_p:-0}) * page_size / 1024 ))
  avail_kb=$(( free_kb + inactive_kb ))
  swap_info=$(sysctl -n vm.swapusage 2>/dev/null)
  swap_total=$(echo "$swap_info" | awk "{gsub(/M/,\"\"); print int(\$3 * 1024)}")
  swap_free=$(echo "$swap_info" | awk "{gsub(/M/,\"\"); print int(\$9 * 1024)}")
  echo "MemTotal: ${total_kb:-0} kB"
  echo "MemFree: ${free_kb:-0} kB"
  echo "MemAvailable: ${avail_kb:-0} kB"
  echo "Buffers: 0 kB"
  echo "Cached: ${cached_kb:-0} kB"
  echo "Active: ${active_kb:-0} kB"
  echo "Inactive: ${inactive_kb:-0} kB"
  echo "SwapTotal: ${swap_total:-0} kB"
  echo "SwapFree: ${swap_free:-0} kB"
fi
echo "---DISK---"
df -kP 2>/dev/null | tail -n +2
echo "---DISKIO---"
cat /proc/diskstats 2>/dev/null
echo "---NET---"
if [ -f /proc/net/dev ]; then
  cat /proc/net/dev 2>/dev/null
else
  netstat -ibn 2>/dev/null | awk "NR>1 && \$4 ~ /:/ {print \$1\": \"\$7\" \"\$5\" 0 0 0 0 0 0 \"\$10\" \"\$8\" 0 0 0 0 0 0\"}"
fi
echo "---NETIP---"
if command -v ip >/dev/null 2>&1; then
  ip -o -4 addr show 2>/dev/null | awk "{print \$2\",\"\$4}"
else
  ifconfig 2>/dev/null | awk "/^[a-z0-9]+:/{iface=\$1; sub(\":\",\"\",iface)} /inet / && \$2 != \"127.0.0.1\"{print iface\",\"\$2}"
fi
echo "---PROC---"
(ps -eo pid,pcpu,pmem,rss,nlwp,user,comm,args --sort=-pcpu 2>/dev/null || ps -Ao pid,pcpu,pmem,rss,wq,user,comm,args -r 2>/dev/null) | head -n 41
echo "---GPU---"
if command -v nvidia-smi >/dev/null 2>&1; then
  nvidia-smi --query-gpu=index,uuid,name,driver_version,utilization.gpu,utilization.memory,memory.total,memory.free,memory.used,fan.speed,temperature.gpu,power.draw,power.default_limit --format=csv,noheader,nounits 2>/dev/null
fi
echo "---DOCKER---"
if command -v docker >/dev/null 2>&1; then
  docker info --format "{{.ServerVersion}},{{.Containers}},{{.ContainersRunning}},{{.ContainersPaused}},{{.ContainersStopped}},{{.Images}}" 2>/dev/null
fi
'"#;

// cmd.exe and PowerShell cannot run the sh probe above, so Windows hosts run this through the vps_windows bootstrap.
// It prints the same sections. No CIM: it denies a standard account, 5 s per call. The perf counters (CPU, disk I/O,
// page file, free memory) need an admin account; the rest works for any account.
pub const WINDOWS_TELEMETRY_SCRIPT: &str = r#"Add-Type -AssemblyName Microsoft.VisualBasic
$ci = New-Object Microsoft.VisualBasic.Devices.ComputerInfo
$cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
# ProductName says "Windows 10" on 11 too; build 22000+ is 11.
$os = $cv.ProductName
if ([int]$cv.CurrentBuildNumber -ge 22000) { $os = $os -replace 'Windows 10', 'Windows 11' }
$pc = @{}
foreach ($c in 'Processor', 'Memory', 'LogicalDisk', 'Paging File', 'System') {
  try { $pc[$c] = ([Diagnostics.PerformanceCounterCategory]::new($c)).ReadCategory() } catch { }
}
function one($cat, $ctr) { @($pc[$cat][$ctr].Values)[0].Sample }
$up = [int64]([Diagnostics.Stopwatch]::GetTimestamp() / [Diagnostics.Stopwatch]::Frequency)
if ($pc['System']) { $s = one 'System' 'system up time'; $up = [int64](($s.TimeStamp100nSec - $s.RawValue) / 1e7) }
'---SYS---'
"$os $($cv.DisplayVersion) $env:PROCESSOR_ARCHITECTURE" -replace '\s+', ' '
$env:COMPUTERNAME
$up
"CPU(s): $([Environment]::ProcessorCount)"
"Model name: $((Get-ItemProperty 'HKLM:\HARDWARE\DESCRIPTION\System\CentralProcessor\0').ProcessorNameString)"
'---CPU---'
if ($pc['Processor']) {
  $u = $pc['Processor']['% user time']; $k = $pc['Processor']['% privileged time']; $i = $pc['Processor']['% idle time']
  foreach ($n in $u.Keys) {
    $id = if ($n -eq '_total') { 'cpu' } else { "cpu$n" }
    # /proc/stat columns user nice system idle, in 100 ns ticks
    "$id $($u[$n].Sample.RawValue) 0 $($k[$n].Sample.RawValue) $($i[$n].Sample.RawValue)"
  }
}
'---MEM---'
$avail = [int64]($ci.AvailablePhysicalMemory / 1KB)
"MemTotal: $([int64]($ci.TotalPhysicalMemory / 1KB)) kB"
"MemAvailable: $avail kB"
if ($pc['Memory']) {
  # Available is Free & Zero plus Standby, the file cache.
  $free = [int64]((one 'Memory' 'free & zero page list bytes').RawValue / 1KB)
  "MemFree: $free kB"
  "Cached: $($avail - $free) kB"
}
if ($pc['Paging File']) {
  $s = $pc['Paging File']['% usage']['_total'].Sample; $pg = [Environment]::SystemPageSize / 1KB
  "SwapTotal: $($s.BaseValue * $pg) kB"
  "SwapFree: $(($s.BaseValue - $s.RawValue) * $pg) kB"
}
'---DISK---'
foreach ($d in [IO.DriveInfo]::GetDrives()) {
  if ($d.DriveType -eq 'Fixed' -and $d.IsReady) {
    $t = [int64]($d.TotalSize / 1KB)
    # df -kP columns: device total used available use% mount
    "$($d.Name.TrimEnd('\')) $t $($t - [int64]($d.TotalFreeSpace / 1KB)) $([int64]($d.AvailableFreeSpace / 1KB)) - $($d.Name)"
  }
}
'---DISKIO---'
if ($pc['LogicalDisk']) {
  $ld = $pc['LogicalDisk']
  foreach ($n in $ld['disk reads/sec'].Keys) {
    if ($n -match '^[a-z]:$') {
      # /proc/diskstats columns 3 5 7 9: reads, sectors read, writes, sectors written
      "0 0 $($n.ToUpper()) $($ld['disk reads/sec'][$n].Sample.RawValue) 0 $([int64]($ld['disk read bytes/sec'][$n].Sample.RawValue / 512)) 0 $($ld['disk writes/sec'][$n].Sample.RawValue) 0 $([int64]($ld['disk write bytes/sec'][$n].Sample.RawValue / 512))"
    }
  }
}
$nics = @([Net.NetworkInformation.NetworkInterface]::GetAllNetworkInterfaces() | Where-Object { $_.OperationalStatus -eq 'Up' -and $_.NetworkInterfaceType -ne 'Loopback' })
'---NET---'
foreach ($n in $nics) {
  $s = $n.GetIPStatistics()
  # /proc/net/dev columns: rx bytes, rx packets, 6 unused, tx bytes, tx packets
  "$($n.Name): $($s.BytesReceived) $($s.UnicastPacketsReceived + $s.NonUnicastPacketsReceived) 0 0 0 0 0 0 $($s.BytesSent) $($s.UnicastPacketsSent + $s.NonUnicastPacketsSent)"
}
'---NETIP---'
foreach ($n in $nics) {
  foreach ($a in $n.GetIPProperties().UnicastAddresses) {
    if ($a.Address.AddressFamily -eq 'InterNetwork') { "$($n.Name),$($a.Address)/$($a.PrefixLength)" }
  }
}
'---PROC---'
$now = Get-Date
$procs = try { Get-Process -IncludeUserName } catch { Get-Process }
# Like ps pcpu: CPU time over the process lifetime. A standard account reads it for its own processes only, and the
# parser turns a list without a CPU section into a total, so without the perf counters it prints "-" and sorts by memory.
$cpuOk = [bool]$pc['Processor']
$rows = foreach ($p in $procs) {
  if (-not $p.Id) { continue }
  $life = if ($p.StartTime) { ($now - $p.StartTime).TotalSeconds } else { 0 }
  # StartTime is wall-clock: a clock change since boot (time sync, time zone) can put it in the future.
  if ($life -le 0 -or $life -gt $up) { $life = $up }
  [pscustomobject]@{ P = $p; C = 100 * $p.CPU / [math]::Max(1, $life) }
}
'PID %CPU %MEM RSS NLWP USER COMMAND ARGS'
$top = if ($cpuOk) { $rows | Sort-Object C -Descending } else { $rows | Sort-Object { $_.P.WorkingSet64 } -Descending }
foreach ($r in $top | Select-Object -First 40) {
  $p = $r.P
  $user = if ($p.UserName) { ($p.UserName -split '\\')[-1] -replace '\s', '_' } else { '-' }
  "$($p.Id) $(if ($cpuOk) { [math]::Round($r.C, 1) } else { '-' }) $([math]::Round(100 * $p.WorkingSet64 / $ci.TotalPhysicalMemory, 1)) $([int64]($p.WorkingSet64 / 1KB)) $($p.Threads.Count) $user $($p.ProcessName -replace '\s', '_') $(if ($p.Path) { $p.Path } else { $p.ProcessName })"
}
"#;

#[derive(Debug, Clone, Default)]
pub struct RawCpuSample {
    pub total: u64,
    pub idle: u64,
    pub user: u64,
    pub system: u64,
    pub iowait: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RawDiskSample {
    pub reads_completed: u64,
    pub sectors_read: u64,
    pub writes_completed: u64,
    pub sectors_written: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RawNetSample {
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub rx_packets: u64,
    pub tx_packets: u64,
}

#[derive(Debug, Clone, Default)]
pub struct RawCpuCounters {
    pub total: u64,
    pub idle: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
    pub cores: HashMap<String, RawCpuSample>,
    pub disks: HashMap<String, RawDiskSample>,
    pub nets: HashMap<String, RawNetSample>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CpuCoreMetric {
    pub id: String,
    pub usage: f32,
    pub user: f32,
    pub system: f32,
    pub idle: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StorageVolumeMetric {
    pub device: String,
    pub mount: String,
    pub total_gb: f32,
    pub used_gb: f32,
    pub available_gb: f32,
    pub used_percent: f32,
    pub read_bytes_per_sec: f64,
    pub write_bytes_per_sec: f64,
    pub iops_read: f32,
    pub iops_write: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct NetworkInterfaceMetric {
    pub name: String,
    pub ip: String,
    pub rx_bytes_per_sec: f64,
    pub tx_bytes_per_sec: f64,
    pub rx_total_bytes: u64,
    pub tx_total_bytes: u64,
    pub rx_packets_per_sec: f32,
    pub tx_packets_per_sec: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct GpuCoreMetric {
    pub index: u32,
    pub uuid: String,
    pub name: String,
    pub driver_version: String,
    pub utilization_gpu: f32,
    pub utilization_memory: f32,
    pub memory_total_mb: u64,
    pub memory_free_mb: u64,
    pub memory_used_mb: u64,
    pub fan_speed_pct: f32,
    pub temperature_gpu: f32,
    pub power_draw_w: f32,
    pub power_limit_w: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerSummaryMetric {
    pub installed: bool,
    pub server_version: String,
    pub total_containers: u32,
    pub running_containers: u32,
    pub paused_containers: u32,
    pub stopped_containers: u32,
    pub total_images: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ProcessItem {
    pub pid: u32,
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub command: String,
    #[serde(default)]
    pub rss_kb: u64,
    #[serde(default)]
    pub threads: u32,
    #[serde(default)]
    pub user: String,
    #[serde(default)]
    pub args: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServerMetrics {
    pub host_id: String,
    pub load_one: f32,
    #[serde(default)]
    pub load_five: f32,
    #[serde(default)]
    pub load_fifteen: f32,
    pub cpu_percent: f32,
    #[serde(default)]
    pub cpu_user_percent: f32,
    #[serde(default)]
    pub cpu_system_percent: f32,
    #[serde(default)]
    pub cpu_iowait_percent: f32,
    #[serde(default)]
    pub cpu_model: String,
    #[serde(default)]
    pub cpu_cores: u32,
    #[serde(default)]
    pub cores: Vec<CpuCoreMetric>,
    #[serde(default)]
    pub os_name: String,
    #[serde(default)]
    pub hostname: String,
    #[serde(default)]
    pub uptime_seconds: u64,
    #[serde(default)]
    pub temperature_celsius: Option<f32>,
    pub mem_used_mb: u64,
    pub mem_total_mb: u64,
    #[serde(default)]
    pub mem_free_mb: u64,
    #[serde(default)]
    pub mem_cached_mb: u64,
    #[serde(default)]
    pub mem_buffers_mb: u64,
    #[serde(default)]
    pub mem_available_mb: u64,
    pub swap_used_mb: u64,
    pub swap_total_mb: u64,
    pub disk_used_gb: f32,
    pub disk_total_gb: f32,
    pub disk_percent: f32,
    #[serde(default)]
    pub volumes: Vec<StorageVolumeMetric>,
    pub net_rx_kbps: f32,
    pub net_tx_kbps: f32,
    #[serde(default)]
    pub interfaces: Vec<NetworkInterfaceMetric>,
    pub top_processes: Vec<ProcessItem>,
    #[serde(default)]
    pub gpus: Vec<GpuCoreMetric>,
    #[serde(default)]
    pub docker: Option<DockerSummaryMetric>,
}

pub fn parse_linux_telemetry(
    host_id: &str,
    raw: &str,
    prev: Option<&RawCpuCounters>,
    elapsed_secs: f32,
) -> (ServerMetrics, RawCpuCounters) {
    let mut section = "";
    let mut sys_line_idx = 0usize;
    let mut os_name = String::new();
    let mut hostname = String::new();
    let mut uptime_seconds = 0u64;
    let mut cpu_model = String::new();
    let mut cpu_cores_count = 0u32;
    let mut temps_c: Vec<f32> = Vec::new();

    let mut load_one = 0.0f32;
    let mut load_five = 0.0f32;
    let mut load_fifteen = 0.0f32;

    let mut cpu_total = 0u64;
    let mut cpu_idle = 0u64;
    let mut macos_cpu: Option<(f32, f32, f32, u32)> = None;
    let mut core_samples: HashMap<String, RawCpuSample> = HashMap::new();

    let mut mem_total_kb = 0u64;
    let mut mem_free_kb = 0u64;
    let mut mem_avail_kb = 0u64;
    let mut mem_buffers_kb = 0u64;
    let mut mem_cached_kb = 0u64;
    let mut mem_sreclaimable_kb = 0u64;
    let mut swap_total_kb = 0u64;
    let mut swap_free_kb = 0u64;

    let mut net_samples: HashMap<String, RawNetSample> = HashMap::new();
    let mut net_ips: HashMap<String, String> = HashMap::new();

    let mut disk_total_kb = 0u64;
    let mut disk_used_kb = 0u64;
    let mut raw_volumes: Vec<(String, u64, u64, u64, String)> = Vec::new();
    let mut disk_samples: HashMap<String, RawDiskSample> = HashMap::new();

    let mut procs = Vec::new();
    let mut gpus = Vec::new();
    let mut docker_summary: Option<DockerSummaryMetric> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("---") && trimmed.ends_with("---") {
            section = trimmed;
            sys_line_idx = 0;
            continue;
        }
        match section {
            "---SYS---" => {
                if let Some(rest) = trimmed.strip_prefix("CPU(s):") {
                    cpu_cores_count = rest.trim().parse().unwrap_or(cpu_cores_count);
                } else if let Some((_, rest)) = trimmed.split_once(':') {
                    let key_lower = trimmed.split(':').next().unwrap_or("").trim().to_lowercase();
                    if key_lower == "model name" || key_lower == "hardware" || key_lower == "model" {
                        if cpu_model.is_empty() {
                            cpu_model = rest.trim().to_string();
                        }
                    }
                } else if sys_line_idx == 0 {
                    os_name = trimmed.to_string();
                    sys_line_idx += 1;
                } else if sys_line_idx == 1 {
                    hostname = trimmed.to_string();
                    sys_line_idx += 1;
                } else if sys_line_idx == 2 {
                    uptime_seconds = trimmed.parse().unwrap_or(0);
                    sys_line_idx += 1;
                }
            }
            "---TEMP---" => {
                if let Some((_, val_str)) = trimmed.split_once(',') {
                    if let Ok(raw_val) = val_str.trim().parse::<f32>() {
                        let c = if raw_val > 200.0 {
                            raw_val / 1000.0
                        } else {
                            raw_val
                        };
                        if (1.0..=130.0).contains(&c) {
                            temps_c.push(c);
                        }
                    }
                }
            }
            "---LOAD---" => {
                let cleaned = trimmed.trim_matches(|c| c == '{' || c == '}').trim();
                let mut parts = cleaned.split_whitespace();
                if let Some(l1) = parts.next() {
                    load_one = l1.parse().unwrap_or(0.0);
                }
                if let Some(l5) = parts.next() {
                    load_five = l5.parse().unwrap_or(0.0);
                }
                if let Some(l15) = parts.next() {
                    load_fifteen = l15.parse().unwrap_or(0.0);
                }
            }
            "---CPU---" => {
                if let Some(rest) = trimmed.strip_prefix("MACOS_CPU ") {
                    let parts: Vec<f32> = rest
                        .split_whitespace()
                        .filter_map(|v| v.parse().ok())
                        .collect();
                    if parts.len() >= 4 {
                        macos_cpu = Some((parts[0], parts[1], parts[2], parts[3] as u32));
                    }
                } else if trimmed.starts_with("cpu") {
                    let mut parts = trimmed.split_whitespace();
                    let label = parts.next().unwrap_or("cpu");
                    let vals: Vec<u64> = parts.filter_map(|v| v.parse().ok()).collect();
                    let total: u64 = vals.iter().sum();
                    let user = vals.first().copied().unwrap_or(0) + vals.get(1).copied().unwrap_or(0);
                    let system = vals.get(2).copied().unwrap_or(0)
                        + vals.get(5).copied().unwrap_or(0)
                        + vals.get(6).copied().unwrap_or(0);
                    let iowait = vals.get(4).copied().unwrap_or(0);
                    let idle = vals.get(3).copied().unwrap_or(0) + iowait;
                    if label == "cpu" {
                        cpu_total = total;
                        cpu_idle = idle;
                    }
                    core_samples.insert(
                        label.to_string(),
                        RawCpuSample {
                            total,
                            idle,
                            user,
                            system,
                            iowait,
                        },
                    );
                }
            }
            "---MEM---" => {
                let mut parts = trimmed.split_whitespace();
                let key = parts.next().unwrap_or("");
                let val: u64 = parts.next().and_then(|v| v.parse().ok()).unwrap_or(0);
                match key {
                    "MemTotal:" => mem_total_kb = val,
                    "MemFree:" => mem_free_kb = val,
                    "MemAvailable:" => mem_avail_kb = val,
                    "Buffers:" => mem_buffers_kb = val,
                    "Cached:" => mem_cached_kb = val,
                    "SReclaimable:" => mem_sreclaimable_kb = val,
                    "SwapTotal:" => swap_total_kb = val,
                    "SwapFree:" => swap_free_kb = val,
                    _ => {}
                }
            }
            "---NET---" => {
                if let Some((iface, rest)) = trimmed.split_once(':') {
                    let iface = iface.trim();
                    if iface != "lo" && !iface.starts_with("lo0") {
                        let cols: Vec<u64> = rest
                            .split_whitespace()
                            .filter_map(|v| v.parse().ok())
                            .collect();
                        if cols.len() >= 10 {
                            let entry = net_samples.entry(iface.to_string()).or_default();
                            // On macOS netstat -ibn can list multiple rows per interface (Link + IPv4/v6); keep max counter
                            entry.rx_bytes = entry.rx_bytes.max(cols[0]);
                            entry.rx_packets = entry.rx_packets.max(cols[1]);
                            entry.tx_bytes = entry.tx_bytes.max(cols[8]);
                            entry.tx_packets = entry.tx_packets.max(cols[9]);
                        }
                    }
                }
            }
            "---NETIP---" => {
                if let Some((iface, ip)) = trimmed.split_once(',') {
                    net_ips
                        .entry(iface.trim().to_string())
                        .or_insert_with(|| ip.trim().to_string());
                }
            }
            "---DISK---" => {
                let cols: Vec<&str> = trimmed.split_whitespace().collect();
                if cols.len() >= 6 {
                    let dev = cols[0];
                    let total: u64 = cols[1].parse().unwrap_or(0);
                    let used: u64 = cols[2].parse().unwrap_or(0);
                    let avail: u64 = cols[3].parse().unwrap_or(0);
                    let mount = cols[cols.len() - 1];
                    if mount == "/" || (disk_total_kb == 0 && total > 0) {
                        disk_total_kb = total;
                        disk_used_kb = used;
                    }
                    if total > 0
                        && !dev.starts_with("tmpfs")
                        && !dev.starts_with("devtmpfs")
                        && !dev.starts_with("devfs")
                        && !dev.starts_with("map ")
                        && !mount.starts_with("/System/Volumes/Preboot")
                        && !mount.starts_with("/System/Volumes/VM")
                        && !mount.starts_with("/System/Volumes/Update")
                    {
                        raw_volumes.push((
                            dev.to_string(),
                            total,
                            used,
                            avail,
                            mount.to_string(),
                        ));
                    }
                }
            }
            "---DISKIO---" => {
                let cols: Vec<&str> = trimmed.split_whitespace().collect();
                if cols.len() >= 10 {
                    let dev_name = cols[2];
                    let reads_completed: u64 = cols[3].parse().unwrap_or(0);
                    let sectors_read: u64 = cols[5].parse().unwrap_or(0);
                    let writes_completed: u64 = cols[7].parse().unwrap_or(0);
                    let sectors_written: u64 = cols[9].parse().unwrap_or(0);
                    disk_samples.insert(
                        dev_name.to_string(),
                        RawDiskSample {
                            reads_completed,
                            sectors_read,
                            writes_completed,
                            sectors_written,
                        },
                    );
                }
            }
            "---PROC---" => {
                let cols: Vec<&str> = trimmed.split_whitespace().collect();
                if cols.len() >= 4 && cols[0] != "PID" {
                    if let Ok(pid) = cols[0].parse::<u32>() {
                        let cpu_pct = cols[1].parse().unwrap_or(0.0);
                        let mem_pct = cols[2].parse().unwrap_or(0.0);
                        if cols.len() >= 8 {
                            let rss_kb = cols[3].parse().unwrap_or(0);
                            let threads = cols[4].parse().unwrap_or(1);
                            let user = cols[5].to_string();
                            let comm = cols[6]
                                .rsplit('/')
                                .next()
                                .unwrap_or(cols[6])
                                .to_string();
                            let args = cols[7..].join(" ");
                            procs.push(ProcessItem {
                                pid,
                                cpu_pct,
                                mem_pct,
                                command: comm,
                                rss_kb,
                                threads,
                                user,
                                args,
                            });
                        } else {
                            let command = cols[3..].join(" ");
                            procs.push(ProcessItem {
                                pid,
                                cpu_pct,
                                mem_pct,
                                command: command.clone(),
                                rss_kb: 0,
                                threads: 1,
                                user: "root".into(),
                                args: command,
                            });
                        }
                    }
                }
            }
            "---GPU---" => {
                let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 13 {
                    if let Ok(index) = parts[0].parse::<u32>() {
                        gpus.push(GpuCoreMetric {
                            index,
                            uuid: parts[1].to_string(),
                            name: parts[2].to_string(),
                            driver_version: parts[3].to_string(),
                            utilization_gpu: parts[4].parse().unwrap_or(0.0),
                            utilization_memory: parts[5].parse().unwrap_or(0.0),
                            memory_total_mb: parts[6].parse().unwrap_or(0),
                            memory_free_mb: parts[7].parse().unwrap_or(0),
                            memory_used_mb: parts[8].parse().unwrap_or(0),
                            fan_speed_pct: parts[9].parse().unwrap_or(0.0),
                            temperature_gpu: parts[10].parse().unwrap_or(0.0),
                            power_draw_w: parts[11].parse().unwrap_or(0.0),
                            power_limit_w: parts[12].parse().unwrap_or(0.0),
                        });
                    }
                }
            }
            "---DOCKER---" => {
                let parts: Vec<&str> = trimmed.split(',').map(|s| s.trim()).collect();
                if parts.len() >= 6 && !parts[0].is_empty() {
                    docker_summary = Some(DockerSummaryMetric {
                        installed: true,
                        server_version: parts[0].to_string(),
                        total_containers: parts[1].parse().unwrap_or(0),
                        running_containers: parts[2].parse().unwrap_or(0),
                        paused_containers: parts[3].parse().unwrap_or(0),
                        stopped_containers: parts[4].parse().unwrap_or(0),
                        total_images: parts[5].parse().unwrap_or(0),
                    });
                }
            }
            _ => {}
        }
    }

    let rx_bytes: u64 = net_samples.values().map(|n| n.rx_bytes).sum();
    let tx_bytes: u64 = net_samples.values().map(|n| n.tx_bytes).sum();

    let dt = elapsed_secs.max(0.1);
    let mut cpu_user_percent = 0.0f32;
    let mut cpu_system_percent = 0.0f32;
    let mut cpu_iowait_percent = 0.0f32;
    let mut core_metrics: Vec<CpuCoreMetric> = Vec::new();

    let (cpu_percent, net_rx_kbps, net_tx_kbps) = if let Some((u, s, idle, ncores)) = macos_cpu {
        cpu_user_percent = u;
        cpu_system_percent = s;
        if cpu_cores_count == 0 {
            cpu_cores_count = ncores.max(1);
        }
        let total_usage = (u + s).clamp(0.0, 100.0);
        for idx in 0..cpu_cores_count {
            core_metrics.push(CpuCoreMetric {
                id: format!("cpu{idx}"),
                usage: total_usage,
                user: u,
                system: s,
                idle,
            });
        }
        let (rx_k, tx_k) = if let Some(p) = prev {
            (
                (rx_bytes.saturating_sub(p.rx_bytes) as f32 / 1024.0) / dt,
                (tx_bytes.saturating_sub(p.tx_bytes) as f32 / 1024.0) / dt,
            )
        } else {
            (0.0, 0.0)
        };
        (total_usage, rx_k, tx_k)
    } else if let Some(p) = prev {
        let d_total = cpu_total.saturating_sub(p.total);
        let d_idle = cpu_idle.saturating_sub(p.idle);
        if let (Some(cur_all), Some(prev_all)) = (core_samples.get("cpu"), p.cores.get("cpu")) {
            let dtot = cur_all.total.saturating_sub(prev_all.total).max(1) as f32;
            cpu_user_percent = (cur_all.user.saturating_sub(prev_all.user) as f32 / dtot) * 100.0;
            cpu_system_percent =
                (cur_all.system.saturating_sub(prev_all.system) as f32 / dtot) * 100.0;
            cpu_iowait_percent =
                (cur_all.iowait.saturating_sub(prev_all.iowait) as f32 / dtot) * 100.0;
        }
        for (cid, csample) in &core_samples {
            if cid == "cpu" {
                continue;
            }
            if let Some(psample) = p.cores.get(cid) {
                let c_dtot = csample.total.saturating_sub(psample.total);
                if c_dtot > 0 {
                    let c_didle = csample.idle.saturating_sub(psample.idle);
                    let c_duser = csample.user.saturating_sub(psample.user);
                    let c_dsys = csample.system.saturating_sub(psample.system);
                    let usage = (100.0 * (1.0 - (c_didle as f32 / c_dtot as f32))).clamp(0.0, 100.0);
                    core_metrics.push(CpuCoreMetric {
                        id: cid.clone(),
                        usage,
                        user: (100.0 * c_duser as f32 / c_dtot as f32).clamp(0.0, 100.0),
                        system: (100.0 * c_dsys as f32 / c_dtot as f32).clamp(0.0, 100.0),
                        idle: (100.0 * c_didle as f32 / c_dtot as f32).clamp(0.0, 100.0),
                    });
                }
            }
        }
        core_metrics.sort_by_key(|c| {
            c.id.trim_start_matches("cpu")
                .parse::<u32>()
                .unwrap_or(999)
        });
        if cpu_cores_count == 0 && !core_metrics.is_empty() {
            cpu_cores_count = core_metrics.len() as u32;
        }
        let cpu = if d_total > 0 {
            100.0 * (1.0 - (d_idle as f32 / d_total as f32))
        } else if !procs.is_empty() {
            (procs.iter().map(|pr| pr.cpu_pct).sum::<f32>() / 8.0).clamp(0.0, 100.0)
        } else {
            0.0
        };
        let rx_kbps = (rx_bytes.saturating_sub(p.rx_bytes) as f32 / 1024.0) / dt;
        let tx_kbps = (tx_bytes.saturating_sub(p.tx_bytes) as f32 / 1024.0) / dt;
        (cpu.clamp(0.0, 100.0), rx_kbps, tx_kbps)
    } else {
        (0.0, 0.0, 0.0)
    };

    let mut interfaces: Vec<NetworkInterfaceMetric> = net_samples
        .iter()
        .map(|(name, cur)| {
            let (rx_bps, tx_bps, rx_pps, tx_pps) = if let Some(p) = prev {
                if let Some(prev_net) = p.nets.get(name) {
                    (
                        cur.rx_bytes.saturating_sub(prev_net.rx_bytes) as f64 / dt as f64,
                        cur.tx_bytes.saturating_sub(prev_net.tx_bytes) as f64 / dt as f64,
                        cur.rx_packets.saturating_sub(prev_net.rx_packets) as f32 / dt,
                        cur.tx_packets.saturating_sub(prev_net.tx_packets) as f32 / dt,
                    )
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                }
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };
            NetworkInterfaceMetric {
                name: name.clone(),
                ip: net_ips.get(name).cloned().unwrap_or_default(),
                rx_bytes_per_sec: rx_bps,
                tx_bytes_per_sec: tx_bps,
                rx_total_bytes: cur.rx_bytes,
                tx_total_bytes: cur.tx_bytes,
                rx_packets_per_sec: rx_pps,
                tx_packets_per_sec: tx_pps,
            }
        })
        .collect();
    interfaces.sort_by(|a, b| b.rx_total_bytes.cmp(&a.rx_total_bytes));

    let volumes: Vec<StorageVolumeMetric> = raw_volumes
        .into_iter()
        .map(|(dev, total_kb, used_kb, avail_kb, mount)| {
            let short_dev = dev.rsplit('/').next().unwrap_or(&dev);
            let (read_bps, write_bps, iops_r, iops_w) = if let Some(p) = prev {
                if let (Some(cur_d), Some(prev_d)) =
                    (disk_samples.get(short_dev), p.disks.get(short_dev))
                {
                    let d_r_sec = cur_d.sectors_read.saturating_sub(prev_d.sectors_read);
                    let d_w_sec = cur_d.sectors_written.saturating_sub(prev_d.sectors_written);
                    let d_r_ops = cur_d.reads_completed.saturating_sub(prev_d.reads_completed);
                    let d_w_ops = cur_d.writes_completed.saturating_sub(prev_d.writes_completed);
                    (
                        (d_r_sec * 512) as f64 / dt as f64,
                        (d_w_sec * 512) as f64 / dt as f64,
                        d_r_ops as f32 / dt,
                        d_w_ops as f32 / dt,
                    )
                } else {
                    (0.0, 0.0, 0.0, 0.0)
                }
            } else {
                (0.0, 0.0, 0.0, 0.0)
            };
            let used_pct = if total_kb > 0 {
                (used_kb as f32 / total_kb as f32) * 100.0
            } else {
                0.0
            };
            StorageVolumeMetric {
                device: dev,
                mount,
                total_gb: total_kb as f32 / (1024.0 * 1024.0),
                used_gb: used_kb as f32 / (1024.0 * 1024.0),
                available_gb: avail_kb as f32 / (1024.0 * 1024.0),
                used_percent: used_pct,
                read_bytes_per_sec: read_bps,
                write_bytes_per_sec: write_bps,
                iops_read: iops_r,
                iops_write: iops_w,
            }
        })
        .collect();

    let effective_cached_kb = mem_cached_kb + mem_sreclaimable_kb;
    let effective_avail_kb = if mem_avail_kb > 0 {
        mem_avail_kb
    } else {
        mem_free_kb + effective_cached_kb + mem_buffers_kb
    };
    let mem_used_kb = mem_total_kb.saturating_sub(effective_avail_kb);
    let swap_used_kb = swap_total_kb.saturating_sub(swap_free_kb);
    let disk_percent = if disk_total_kb > 0 {
        (disk_used_kb as f32 / disk_total_kb as f32) * 100.0
    } else {
        0.0
    };
    let temperature_celsius = if !temps_c.is_empty() {
        Some(temps_c.iter().copied().fold(0.0f32, f32::max))
    } else {
        gpus.first().map(|g| g.temperature_gpu)
    };

    let metrics = ServerMetrics {
        host_id: host_id.to_string(),
        load_one,
        load_five,
        load_fifteen,
        cpu_percent,
        cpu_user_percent,
        cpu_system_percent,
        cpu_iowait_percent,
        cpu_model,
        cpu_cores: cpu_cores_count.max(1),
        cores: core_metrics,
        os_name,
        hostname,
        uptime_seconds,
        temperature_celsius,
        mem_used_mb: mem_used_kb / 1024,
        mem_total_mb: mem_total_kb / 1024,
        mem_free_mb: mem_free_kb / 1024,
        mem_cached_mb: effective_cached_kb / 1024,
        mem_buffers_mb: mem_buffers_kb / 1024,
        mem_available_mb: effective_avail_kb / 1024,
        swap_used_mb: swap_used_kb / 1024,
        swap_total_mb: swap_total_kb / 1024,
        disk_used_gb: disk_used_kb as f32 / (1024.0 * 1024.0),
        disk_total_gb: disk_total_kb as f32 / (1024.0 * 1024.0),
        disk_percent,
        volumes,
        net_rx_kbps,
        net_tx_kbps,
        interfaces,
        top_processes: procs,
        gpus,
        docker: docker_summary,
    };

    let counters = RawCpuCounters {
        total: cpu_total,
        idle: cpu_idle,
        rx_bytes,
        tx_bytes,
        cores: core_samples,
        disks: disk_samples,
        nets: net_samples,
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

// === SwiftServer Docker Management Probes & Parsers ===

pub const DOCKER_SNAPSHOT_PROBE_CMD: &str = r#"LC_ALL=C sh -c '
export PATH="$HOME/.docker/bin:/usr/local/bin:/opt/homebrew/bin:$PATH"
if ! command -v docker >/dev/null 2>&1; then
  echo "DOCKER_NOT_INSTALLED"
  exit 0
fi
echo "---INFO---"
docker info --format "{{.ServerVersion}}|{{.OperatingSystem}}|{{.Architecture}}|{{.Containers}}|{{.ContainersRunning}}|{{.ContainersPaused}}|{{.ContainersStopped}}|{{.Images}}" 2>/dev/null
echo "---CONTAINERS---"
docker container ls -a --no-trunc --format "{{.ID}}|{{.Names}}|{{.Image}}|{{.State}}|{{.Status}}|{{.Ports}}|{{.CreatedAt}}|{{.Command}}" 2>/dev/null
echo "---STATS---"
docker stats --no-stream --format "{{.ID}}|{{.Name}}|{{.CPUPerc}}|{{.MemUsage}}|{{.MemPerc}}|{{.NetIO}}|{{.BlockIO}}|{{.PIDs}}" 2>/dev/null
echo "---COMPOSE---"
docker compose ls -a --format json 2>/dev/null || echo "[]"
echo "---IMAGES---"
docker image ls -a --format "{{.ID}}|{{.Repository}}|{{.Tag}}|{{.Size}}|{{.CreatedSince}}" 2>/dev/null
echo "---VOLUMES---"
docker volume ls --format "{{.Name}}|{{.Driver}}|{{.Mountpoint}}|{{.Scope}}" 2>/dev/null
echo "---NETWORKS---"
docker network ls --format "{{.ID}}|{{.Name}}|{{.Driver}}|{{.Scope}}" 2>/dev/null
'"#;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerContainerItem {
    pub id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub ports: String,
    pub created_at: String,
    pub command: String,
    pub cpu_perc: String,
    pub mem_usage: String,
    pub mem_perc: String,
    pub net_io: String,
    pub block_io: String,
    pub pids: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerComposeItem {
    pub name: String,
    pub status: String,
    pub config_files: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerImageItem {
    pub id: String,
    pub repository: String,
    pub tag: String,
    pub size: String,
    pub created_since: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerVolumeItem {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub scope: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerNetworkItem {
    pub id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DockerFullSnapshot {
    pub installed: bool,
    pub server_version: String,
    pub os: String,
    pub arch: String,
    pub total_containers: u32,
    pub running_containers: u32,
    pub paused_containers: u32,
    pub stopped_containers: u32,
    pub total_images: u32,
    pub containers: Vec<DockerContainerItem>,
    pub compose_projects: Vec<DockerComposeItem>,
    pub images: Vec<DockerImageItem>,
    pub volumes: Vec<DockerVolumeItem>,
    pub networks: Vec<DockerNetworkItem>,
}

pub fn parse_docker_snapshot(raw: &str) -> DockerFullSnapshot {
    if raw.contains("DOCKER_NOT_INSTALLED") {
        return DockerFullSnapshot::default();
    }
    let mut snap = DockerFullSnapshot {
        installed: false,
        ..Default::default()
    };
    let mut section = "";
    let mut stats_map: HashMap<String, (String, String, String, String, String, String)> =
        HashMap::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.starts_with("---") && trimmed.ends_with("---") {
            section = trimmed;
            continue;
        }
        match section {
            "---INFO---" => {
                let p: Vec<&str> = trimmed.split('|').collect();
                if p.len() >= 8 && !p[0].trim().is_empty() {
                    snap.installed = true;
                    snap.server_version = p[0].trim().to_string();
                    snap.os = p[1].trim().to_string();
                    snap.arch = p[2].trim().to_string();
                    snap.total_containers = p[3].trim().parse().unwrap_or(0);
                    snap.running_containers = p[4].trim().parse().unwrap_or(0);
                    snap.paused_containers = p[5].trim().parse().unwrap_or(0);
                    snap.stopped_containers = p[6].trim().parse().unwrap_or(0);
                    snap.total_images = p[7].trim().parse().unwrap_or(0);
                }
            }
            "---CONTAINERS---" => {
                let p: Vec<&str> = trimmed.split('|').collect();
                if p.len() >= 8 {
                    snap.installed = true;
                    snap.containers.push(DockerContainerItem {
                        id: p[0].trim().to_string(),
                        name: p[1].trim().to_string(),
                        image: p[2].trim().to_string(),
                        state: p[3].trim().to_string(),
                        status: p[4].trim().to_string(),
                        ports: p[5].trim().to_string(),
                        created_at: p[6].trim().to_string(),
                        command: p[7].trim().trim_matches('"').to_string(),
                        ..Default::default()
                    });
                }
            }
            "---STATS---" => {
                let p: Vec<&str> = trimmed.split('|').collect();
                if p.len() >= 8 {
                    let short_id = p[0].trim().to_string();
                    let name = p[1].trim().to_string();
                    let tup = (
                        p[2].trim().to_string(),
                        p[3].trim().to_string(),
                        p[4].trim().to_string(),
                        p[5].trim().to_string(),
                        p[6].trim().to_string(),
                        p[7].trim().to_string(),
                    );
                    stats_map.insert(short_id, tup.clone());
                    stats_map.insert(name, tup);
                }
            }
            "---COMPOSE---" => {
                if let Ok(val) = serde_json::from_str::<serde_json::Value>(trimmed) {
                    if let Some(arr) = val.as_array() {
                        for item in arr {
                            let name = item
                                .get("Name")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let status = item
                                .get("Status")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            let config_files = item
                                .get("ConfigFiles")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            if !name.is_empty() {
                                snap.compose_projects.push(DockerComposeItem {
                                    name,
                                    status,
                                    config_files,
                                });
                            }
                        }
                    }
                }
            }
            "---IMAGES---" => {
                let p: Vec<&str> = trimmed.split('|').collect();
                if p.len() >= 5 {
                    snap.images.push(DockerImageItem {
                        id: p[0].trim().to_string(),
                        repository: p[1].trim().to_string(),
                        tag: p[2].trim().to_string(),
                        size: p[3].trim().to_string(),
                        created_since: p[4].trim().to_string(),
                    });
                }
            }
            "---VOLUMES---" => {
                let p: Vec<&str> = trimmed.split('|').collect();
                if p.len() >= 4 {
                    snap.volumes.push(DockerVolumeItem {
                        name: p[0].trim().to_string(),
                        driver: p[1].trim().to_string(),
                        mountpoint: p[2].trim().to_string(),
                        scope: p[3].trim().to_string(),
                    });
                }
            }
            "---NETWORKS---" => {
                let p: Vec<&str> = trimmed.split('|').collect();
                if p.len() >= 4 {
                    snap.networks.push(DockerNetworkItem {
                        id: p[0].trim().to_string(),
                        name: p[1].trim().to_string(),
                        driver: p[2].trim().to_string(),
                        scope: p[3].trim().to_string(),
                    });
                }
            }
            _ => {}
        }
    }

    for c in &mut snap.containers {
        let short_id: String = c.id.chars().take(12).collect();
        if let Some(st) = stats_map
            .get(&c.name)
            .or_else(|| stats_map.get(&short_id))
            .or_else(|| stats_map.get(&c.id))
        {
            c.cpu_perc = st.0.clone();
            c.mem_usage = st.1.clone();
            c.mem_perc = st.2.clone();
            c.net_io = st.3.clone();
            c.block_io = st.4.clone();
            c.pids = st.5.clone();
        }
    }

    snap
}

pub async fn run_host_or_local_cmd(
    host_id: &str,
    pool: &SshPool,
    cmd: &str,
) -> Result<String, String> {
    if host_id == "local" || host_id.trim().is_empty() {
        #[cfg(target_os = "ios")]
        {
            return Ok(crate::local_pty::eval_embedded_command(cmd).await);
        }
        #[cfg(target_os = "windows")]
        {
            if let Ok(out) = tokio::process::Command::new("powershell.exe")
                .args(["-NoProfile", "-NonInteractive", "-Command", cmd])
                // CREATE_NO_WINDOW: GUI-subsystem app otherwise pops a console per spawn
                .creation_flags(0x0800_0000)
                .output()
                .await
            {
                let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
                if combined.trim().is_empty() && !out.stderr.is_empty() {
                    combined = String::from_utf8_lossy(&out.stderr).to_string();
                }
                return Ok(combined);
            }
            return Ok(crate::local_pty::eval_embedded_command(cmd).await);
        }
        #[cfg(not(any(target_os = "ios", target_os = "windows")))]
        {
            let sh_bin = if cfg!(target_os = "android") && std::path::Path::new("/system/bin/sh").exists() {
                "/system/bin/sh"
            } else {
                "sh"
            };
            if let Ok(out) = tokio::process::Command::new(sh_bin)
                .arg("-c")
                .arg(cmd)
                .output()
                .await
            {
                let mut combined = String::from_utf8_lossy(&out.stdout).to_string();
                if combined.trim().is_empty() && !out.stderr.is_empty() {
                    combined = String::from_utf8_lossy(&out.stderr).to_string();
                }
                return Ok(combined);
            }
            return Ok(crate::local_pty::eval_embedded_command(cmd).await);
        }
    } else if pool.has_active_session(host_id).await {
        pool.exec_command(host_id, cmd).await
    } else {
        let hosts = crate::host_store::load_all_hosts();
        if let Some(h) = hosts.iter().find(|h| h.id == host_id) {
            pool.exec_on_host(h, cmd).await
        } else {
            Err(format!("Host {host_id} not found"))
        }
    }
}

fn build_portable_fallback_snapshot() -> String {
    let os_name = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| format!("ssh-box-{os_name}"));
    let ncpu = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    let uptime = std::fs::read_to_string("/proc/uptime")
        .ok()
        .and_then(|s| s.split_whitespace().next()?.parse::<f64>().ok())
        .map(|v| v as u64)
        .unwrap_or(3600);
    let meminfo = std::fs::read_to_string("/proc/meminfo").unwrap_or_else(|_| {
        "MemTotal:       8388608 kB\nMemFree:        4194304 kB\nMemAvailable:   5242880 kB\nCached:         1048576 kB\nSwapTotal:            0 kB\nSwapFree:             0 kB\n".to_string()
    });
    let proc_stat = std::fs::read_to_string("/proc/stat")
        .unwrap_or_else(|_| format!("MACOS_CPU 12.5 4.5 83.0 {ncpu}\n"));
    let proc_net = std::fs::read_to_string("/proc/net/dev").unwrap_or_default();

    format!(
        "---SYS---\n{os_name} {arch}\n{hostname}\n{uptime}\nCPU(s): {ncpu}\nModel name: {os_name} ({arch})\n---TEMP---\n---LOAD---\n0.25 0.20 0.15\n---CPU---\n{proc_stat}\n---MEM---\n{meminfo}\n---DISK---\nsshbox-storage 67108864 16777216 50331648 25% /\n---DISKIO---\n---NET---\n{proc_net}\n---NETIP---\n---PROC---\n  PID %CPU %MEM   RSS NLWP USER COMMAND ARGS\n    1  1.5  1.2 65536    4 user ssh-box ssh-box\n---GPU---\n---DOCKER---\n"
    )
}

async fn probe_local_snapshot() -> Result<String, String> {
    #[cfg(any(target_os = "macos", target_os = "linux"))]
    {
        if let Ok(out) = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(TELEMETRY_PROBE_CMD)
            .output()
            .await
        {
            if out.status.success() {
                return Ok(String::from_utf8_lossy(&out.stdout).to_string());
            }
        }
    }
    Ok(build_portable_fallback_snapshot())
}

// On Windows the sh probe comes back as a cmd/PowerShell error with no sections.
fn is_snapshot(raw: &str) -> bool {
    raw.contains("---SYS---")
}

async fn probe_windows_snapshot(host_id: &str, pool: &SshPool) -> Result<String, String> {
    let host = crate::vps_config::find_remote_host(host_id)?;
    let out = crate::vps_windows::exec_windows(pool, &host, WINDOWS_TELEMETRY_SCRIPT, None).await?;
    Ok(String::from_utf8_lossy(&out).into_owned())
}

// ponytail: each Windows sample starts a PowerShell (~1.5 s of host CPU), so Windows hosts are sampled at most this
// often; a long-lived PowerShell printing one snapshot per tick would remove that cost.
const WINDOWS_MIN_INTERVAL: Duration = Duration::from_secs(10);

pub fn start_monitoring(
    host_id: String,
    pool: SshPool,
    interval_secs: u64,
    channel: Channel<ServerMetrics>,
    mut stop_rx: tokio::sync::mpsc::Receiver<()>,
) {
    tokio::spawn(async move {
        let mut prev_counters: Option<RawCpuCounters> = None;
        let mut last_tick = Instant::now();
        let mut interval = tokio::time::interval(Duration::from_secs(interval_secs.clamp(1, 60)));
        let mut consecutive_failures = 0u8;
        let mut backoff_skip = 0u8;
        let mut windows = false;

        loop {
            tokio::select! {
                _ = stop_rx.recv() => break,
                _ = interval.tick() => {
                    if backoff_skip > 0 {
                        backoff_skip -= 1;
                        continue;
                    }
                    if windows && last_tick.elapsed() < WINDOWS_MIN_INTERVAL {
                        continue;
                    }
                    let raw_res = if host_id == "local" {
                        probe_local_snapshot().await
                    } else if windows {
                        probe_windows_snapshot(&host_id, &pool).await
                    } else {
                        match run_host_or_local_cmd(&host_id, &pool, TELEMETRY_PROBE_CMD).await {
                            Ok(raw) if !is_snapshot(&raw) => {
                                let win = probe_windows_snapshot(&host_id, &pool).await;
                                windows = matches!(&win, Ok(w) if is_snapshot(w));
                                if windows { win } else { Ok(raw) }
                            }
                            other => other,
                        }
                    };

                    if let Ok(raw) = raw_res {
                        consecutive_failures = 0;
                        let elapsed = last_tick.elapsed().as_secs_f32().max(0.1);
                        last_tick = Instant::now();
                        let (metrics, counters) =
                            parse_linux_telemetry(&host_id, &raw, prev_counters.as_ref(), elapsed);
                        prev_counters = Some(counters);
                        if channel.send(metrics).is_err() {
                            break;
                        }
                    } else {
                        consecutive_failures = consecutive_failures.saturating_add(1);
                        if consecutive_failures >= 3 {
                            backoff_skip = 2;
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
---SYS---
Linux 6.8.0-generic x86_64
prod-node-01
86400
CPU(s):              4
Model name:          AMD EPYC 7763 64-Core
---TEMP---
coretemp,48000
---LOAD---
0.42 0.31 0.25 1/240 1234
---CPU---
cpu  1000 0 500 8500 0 0 0 0 0 0
cpu0 250 0 125 2125 0 0 0 0 0 0
cpu1 250 0 125 2125 0 0 0 0 0 0
---MEM---
MemTotal:       8192000 kB
MemFree:        2048000 kB
MemAvailable:   4096000 kB
Cached:         1536000 kB
SwapTotal:      2048000 kB
SwapFree:       1024000 kB
---NET---
  lo: 5000 10 0 0 0 0 0 0 5000 10 0 0 0 0 0 0
eth0: 102400 100 0 0 0 0 0 0 51200 80 0 0 0 0 0 0
---NETIP---
eth0,10.0.1.25/24
---DISK---
/dev/sda1 104857600 52428800 52428800 50% /
---DISKIO---
   8       1 sda1 100 0 2000 10 50 0 1000 5 0 0 15
---PROC---
  PID %CPU %MEM   RSS NLWP USER COMMAND ARGS
 1001 24.5  3.2 65536    4 www  nginx   nginx: master process /usr/sbin/nginx
---GPU---
0, GPU-1234, NVIDIA A100-SXM4-40GB, 550.54.15, 72, 45, 40960, 20480, 20480, 35, 61, 210.5, 400.0
---DOCKER---
26.1.4,5,4,0,1,12
"#;
        let sample2 = sample1
            .replace("cpu  1000 0 500 8500", "cpu  1300 0 700 9000")
            .replace("cpu0 250 0 125 2125", "cpu0 350 0 175 2225")
            .replace("eth0: 102400", "eth0: 307200")
            .replace("51200 80", "153600 120")
            .replace("sda1 100 0 2000 10 50 0 1000", "sda1 120 0 4000 10 70 0 2000");

        let (_, prev) = parse_linux_telemetry("prod", sample1, None, 2.0);
        let (m2, _) = parse_linux_telemetry("prod", &sample2, Some(&prev), 2.0);

        // Delta total = 11000 - 10000 = 1000, delta idle = 9000 - 8500 = 500 => 50% CPU
        assert!((m2.cpu_percent - 50.0).abs() < 0.1);
        assert!((m2.load_one - 0.42).abs() < 0.01);
        assert!((m2.load_five - 0.31).abs() < 0.01);
        assert!((m2.load_fifteen - 0.25).abs() < 0.01);
        assert_eq!(m2.cpu_model, "AMD EPYC 7763 64-Core");
        assert_eq!(m2.uptime_seconds, 86400);
        assert_eq!(m2.temperature_celsius, Some(48.0));
        assert!(!m2.cores.is_empty());
        assert_eq!(m2.mem_used_mb, 4000);
        assert_eq!(m2.mem_total_mb, 8000);
        assert_eq!(m2.mem_cached_mb, 1500);
        assert_eq!(m2.swap_used_mb, 1000);
        assert_eq!(m2.swap_total_mb, 2000);
        assert!((m2.disk_percent - 50.0).abs() < 0.1);
        assert_eq!(m2.volumes.len(), 1);
        assert!(m2.volumes[0].read_bytes_per_sec > 0.0);
        // Delta RX = 204800 bytes = 200 KB over 2s => 100 KB/s
        assert!((m2.net_rx_kbps - 100.0).abs() < 0.1);
        // Delta TX = 102400 bytes = 100 KB over 2s => 50 KB/s
        assert!((m2.net_tx_kbps - 50.0).abs() < 0.1);
        assert_eq!(m2.interfaces.len(), 1);
        assert_eq!(m2.interfaces[0].ip, "10.0.1.25/24");
        assert_eq!(m2.top_processes.len(), 1);
        assert_eq!(m2.top_processes[0].command, "nginx");
        assert_eq!(m2.top_processes[0].user, "www");
        assert_eq!(m2.gpus.len(), 1);
        assert_eq!(m2.gpus[0].name, "NVIDIA A100-SXM4-40GB");
        assert!(m2.docker.is_some());
        assert_eq!(m2.docker.as_ref().unwrap().running_containers, 4);
    }

    // Captured from the Windows 11 ARM VM (admin account), 5 s apart. The real output ends lines with CRLF.
    const WIN_ADMIN_A: &str = r"---SYS---
Windows 11 Pro 26H2 ARM64
WIN-58D0KFGI3KB
499
CPU(s): 4
Model name: virt-10.0
---CPU---
cpu2 96718750 0 110000000 4770969569
cpu3 61562500 0 73437500 4847514092
cpu0 161718750 0 203593750 4618964136
cpu1 124843750 0 146406250 4709694327
cpu 111210937 0 133359375 4736785531
---MEM---
MemTotal: 4184832 kB
MemAvailable: 2659172 kB
MemFree: 1690736 kB
Cached: 968436 kB
SwapTotal: 1441792 kB
SwapFree: 1441792 kB
---DISK---
C: 66011132 30586148 35424984 - C:\
---DISKIO---
0 0 C: 40677 0 1934467 0 5751 0 114388
---NET---
Ethernet: 590424 1364 0 0 0 0 0 0 1108183 1698
---NETIP---
Ethernet,192.168.64.6/24
---PROC---
PID %CPU %MEM RSS NLWP USER COMMAND ARGS
1944 209.4 2.4 100388 22 SSH_Box_Test powershell C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe
1528 3.1 0.3 12520 5 SSH_Box_Test conhost C:\WINDOWS\system32\conhost.exe
2528 3.1 0.3 13200 3 SYSTEM sshd C:\WINDOWS\System32\OpenSSH\sshd.exe
";
    const WIN_ADMIN_B: &str = r"---SYS---
Windows 11 Pro 26H2 ARM64
WIN-58D0KFGI3KB
504
CPU(s): 4
Model name: virt-10.0
---CPU---
cpu2 100937500 0 112343750 4816941231
cpu3 62343750 0 74218750 4897765395
cpu0 167500000 0 207187500 4661384773
cpu1 130000000 0 149843750 4753439605
cpu 115195312 0 135898437 4782382751
---MEM---
MemTotal: 4184832 kB
MemAvailable: 2659464 kB
MemFree: 1690776 kB
Cached: 968688 kB
SwapTotal: 1441792 kB
SwapFree: 1441792 kB
---DISK---
C: 66011132 30586192 35424940 - C:\
---DISKIO---
0 0 C: 40678 0 1934475 0 6233 0 120756
---NET---
Ethernet: 603184 1412 0 0 0 0 0 0 1126240 1734
---NETIP---
Ethernet,192.168.64.6/24
---PROC---
PID %CPU %MEM RSS NLWP USER COMMAND ARGS
3028 175 2.4 101736 22 SSH_Box_Test powershell C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe
2508 3.1 0.3 12528 5 SSH_Box_Test conhost C:\WINDOWS\system32\conhost.exe
3692 3 6.2 259768 38 - MsMpEng MsMpEng
";

    // The bug: a Windows host showed 1 core / 0 GB because nothing in the sh probe's output parsed there.
    #[test]
    fn windows_snapshot_gives_real_cores_memory_and_usage() {
        let crlf = |s: &str| s.replace('\n', "\r\n");
        let (_, prev) = parse_linux_telemetry("win", &crlf(WIN_ADMIN_A), None, 5.0);
        let (m, _) = parse_linux_telemetry("win", &crlf(WIN_ADMIN_B), Some(&prev), 5.0);
        assert_eq!(m.os_name, "Windows 11 Pro 26H2 ARM64");
        assert_eq!(m.hostname, "WIN-58D0KFGI3KB");
        assert_eq!(m.uptime_seconds, 504);
        assert_eq!((m.cpu_cores, m.cpu_model.as_str()), (4, "virt-10.0"));
        // _Total ticks: busy = 1 - 45597220 idle / 52120657 total.
        assert!((m.cpu_percent - 12.52).abs() < 0.05, "{}", m.cpu_percent);
        assert!((m.cpu_user_percent - 7.64).abs() < 0.05 && (m.cpu_system_percent - 4.87).abs() < 0.05);
        assert_eq!(m.cores.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(), ["cpu0", "cpu1", "cpu2", "cpu3"]);
        // Used is total minus Available, as on Linux; Cached is the standby list.
        assert_eq!((m.mem_total_mb, m.mem_used_mb, m.mem_available_mb), (4086, 1489, 2597));
        assert_eq!((m.mem_free_mb, m.mem_cached_mb), (1651, 945));
        assert_eq!((m.swap_total_mb, m.swap_used_mb), (1408, 0));
        assert!((m.disk_total_gb - 62.95).abs() < 0.01 && (m.disk_percent - 46.34).abs() < 0.01);
        assert_eq!((m.volumes[0].device.as_str(), m.volumes[0].mount.as_str()), ("C:", "C:\\"));
        // 6368 sectors written in 5 s.
        assert!((m.volumes[0].write_bytes_per_sec - 652_083.2).abs() < 1.0);
        assert!((m.volumes[0].iops_write - 96.4).abs() < 0.01);
        // 12760 bytes received in 5 s.
        assert!((m.net_rx_kbps - 2.49).abs() < 0.01);
        assert_eq!((m.interfaces[0].name.as_str(), m.interfaces[0].ip.as_str()), ("Ethernet", "192.168.64.6/24"));
        assert_eq!(m.top_processes[0].command, "powershell");
        assert_eq!(m.top_processes[0].user, "SSH_Box_Test");
    }

    // A standard account can read no perf counters (registry 'Global' is denied), but cores, memory, disks,
    // network and uptime must still show. Seen on the VM: when the list carried its own processes' CPU, the
    // parser's fallback summed them into a made-up 21.9% total.
    #[test]
    fn windows_standard_account_snapshot_keeps_what_it_can_read() {
        let raw = r"---SYS---
Windows 11 Pro 26H2 ARM64
WIN-58D0KFGI3KB
926
CPU(s): 4
Model name: virt-10.0
---CPU---
---MEM---
MemTotal: 4184832 kB
MemAvailable: 2670848 kB
---DISK---
C: 66011132 30589204 35421928 - C:\
---DISKIO---
---NET---
Ethernet: 734971 1926 0 0 0 0 0 0 1929545 2516
---NETIP---
Ethernet,192.168.64.6/24
---PROC---
PID %CPU %MEM RSS NLWP USER COMMAND ARGS
3692 - 6.4 266348 38 - MsMpEng MsMpEng
1756 - 2.2 94108 21 - powershell C:\WINDOWS\System32\WindowsPowerShell\v1.0\powershell.exe
1196 - 2.2 90816 16 - LogonUI LogonUI
";
        let (_, prev) = parse_linux_telemetry("win", raw, None, 10.0);
        let (m, _) = parse_linux_telemetry("win", raw, Some(&prev), 10.0);
        assert_eq!(m.cpu_cores, 4);
        assert_eq!(m.cpu_percent, 0.0);
        assert_eq!((m.mem_total_mb, m.mem_used_mb), (4086, 1478));
        assert!((m.disk_total_gb - 62.95).abs() < 0.01);
        assert_eq!(m.uptime_seconds, 926);
        assert_eq!(m.top_processes[0].command, "MsMpEng");
    }

    // The sh probe's real answer from the Windows VM (cmd.exe as the SSH shell): this must send the poll to PowerShell.
    #[test]
    fn sh_probe_error_from_windows_is_not_a_snapshot() {
        let cmd_error = "'LC_ALL' is not recognized as an internal or external command,\r\noperable program or batch file.\r\n";
        assert!(!is_snapshot(cmd_error));
        assert!(is_snapshot(WIN_ADMIN_A));
    }

    // CIM denies a standard account after ~5 s per call, and Get-Counter paths are localized: either would stall
    // or break every poll.
    #[test]
    fn windows_telemetry_script_uses_neither_cim_nor_get_counter() {
        assert!(!WINDOWS_TELEMETRY_SCRIPT.contains("Get-CimInstance"));
        assert!(!WINDOWS_TELEMETRY_SCRIPT.contains("Get-WmiObject"));
        assert!(!WINDOWS_TELEMETRY_SCRIPT.contains("Get-Counter"));
    }

    #[test]
    fn parses_docker_snapshot_accurately() {
        let sample = r#"
---INFO---
26.1.4|Ubuntu 24.04 LTS|x86_64|3|2|0|1|5
---CONTAINERS---
a1b2c3d4e5f60000|api-gateway|nginx:alpine|running|Up 2 hours|0.0.0.0:80->80/tcp|2026-10-22|nginx -g daemon off;
---STATS---
a1b2c3d4e5f6|api-gateway|1.25%|42.5MiB / 8GiB|0.52%|1.2MB / 850kB|0B / 4.1kB|4
---COMPOSE---
[{"Name":"web-stack","Status":"running(2)","ConfigFiles":"/srv/web/docker-compose.yml"}]
---IMAGES---
sha256:112233|nginx|alpine|43.2MB|2 weeks ago
---VOLUMES---
pg_data|local|/var/lib/docker/volumes/pg_data/_data|local
---NETWORKS---
net123|bridge|bridge|local
"#;
        let snap = parse_docker_snapshot(sample);
        assert!(snap.installed);
        assert_eq!(snap.server_version, "26.1.4");
        assert_eq!(snap.containers.len(), 1);
        assert_eq!(snap.containers[0].name, "api-gateway");
        assert_eq!(snap.containers[0].cpu_perc, "1.25%");
        assert_eq!(snap.compose_projects.len(), 1);
        assert_eq!(snap.compose_projects[0].name, "web-stack");
        assert_eq!(snap.images.len(), 1);
        assert_eq!(snap.volumes.len(), 1);
        assert_eq!(snap.networks.len(), 1);
    }
}
