# SSH-Box Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Xây dựng ứng dụng quản lý Server & Terminal hiệu năng cao `SSH-Box` trên macOS bằng Rust + Tauri v2 + WebGL2 (`@xterm/addon-webgl`), hỗ trợ tự động đọc `~/.ssh/config`, macOS Keychain, SSH Multiplexing, Local PTY (`zsh`/`bash`), Split Panes đệ quy 120fps và Telemetry Dashboard kiểu Server Box.

**Architecture:** Toàn bộ tầng kết nối SSH (`russh`), Local Shell (`portable-pty`), đọc cấu hình (`~/.ssh/config` + `hosts.json`), lưu trữ bí mật (`keyring` macOS Keychain) và thu thập thông số Server (`TelemetryPoller`) được đặt trong Rust Async Core (`tokio`). Luồng xuất dữ liệu Terminal từ cả SSH và Local PTY đi qua bộ gom gói `FrameBatcher` (`8ms` / `64KB`) và truyền nhị phân trực tiếp qua `tauri::ipc::Channel` lên trình vẽ GPU WebGL2 (`xterm.js` + `@xterm/addon-webgl`).

**Architecture Diagram:**

```mermaid
graph TD
    subgraph "Frontend (React + TypeScript + WebGL2)"
        UI[AppShell & HostSidebar] --> SW[SplitWorkspace Tree]
        SW --> TP[TerminalPane WebGL2]
        UI --> TD[TelemetryDrawer & StatusBar]
    end

    subgraph "Rust Core (src-tauri / Tokio)"
        HS[host_store.rs]
        VT[vault.rs]
        FB[frame_batcher.rs]
        LP[local_pty.rs]
        SP[ssh_pool.rs]
        TM[telemetry.rs]
    end

    UI -->|invoke: list_hosts / save_host| HS
    HS --> VT
    TP <-->|Binary Channel 8ms + write/resize| FB
    FB <--> LP
    FB <--> SP
    TD <--|ServerMetrics Channel 2s| TM
    TM <-->|Exec Channel| SP
```

**Tech Stack:**
- **Rust Backend (`src-tauri`):** `tauri` v2, `tokio` v1, `portable-pty` v0.8, `russh` v0.45, `russh-keys` v0.45, `keyring` v3 (`apple-native`), `bytes` v1, `serde` v1, `serde_json` v1, `dirs` v5, `uuid` v1, `thiserror` v2.
- **Frontend (`src`):** React 18, TypeScript, Vite, Tailwind CSS, `@xterm/xterm` v5.5, `@xterm/addon-webgl` v0.18, `@xterm/addon-fit` v0.10, `@xterm/addon-unicode11` v0.8, `lucide-react`.

---

### Task 1: Khởi tạo Workspace Rust + Tauri v2 & Cấu hình Dependencies

**Files:**
- Create: `~/ssh-box/package.json`
- Create: `~/ssh-box/tsconfig.json`
- Create: `~/ssh-box/vite.config.ts`
- Create: `~/ssh-box/index.html`
- Create: `~/ssh-box/src-tauri/Cargo.toml`
- Create: `~/ssh-box/src-tauri/build.rs`
- Create: `~/ssh-box/src-tauri/tauri.conf.json`
- Create: `~/ssh-box/src-tauri/src/lib.rs`
- Create: `~/ssh-box/src-tauri/src/main.rs`

- [ ] **Step 1: Tạo `src-tauri/Cargo.toml`, `build.rs`, `tauri.conf.json` và entry tối thiểu**

```toml
# ~/ssh-box/src-tauri/Cargo.toml
[package]
name = "ssh-box"
version = "0.1.0"
description = "High-performance Rust SSH & Terminal Manager inspired by Server Box & iTerm2"
authors = ["darien"]
edition = "2021"

[lib]
name = "ssh_box_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }

[dependencies]
tauri = { version = "2", features = ["macos-private-api"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
bytes = "1"
portable-pty = "0.8"
russh = "0.45"
russh-keys = "0.45"
async-trait = "0.1"
keyring = { version = "3", features = ["apple-native"] }
dirs = "5"
uuid = { version = "1", features = ["v4", "serde"] }
thiserror = "2"
```

```rust
// ~/ssh-box/src-tauri/build.rs
fn main() {
    tauri_build::build()
}
```

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "SSH Box",
  "version": "0.1.0",
  "identifier": "com.darien.sshbox",
  "build": {
    "beforeDevCommand": "npm run dev",
    "devUrl": "http://localhost:1420",
    "beforeBuildCommand": "npm run build",
    "frontendDist": "../dist"
  },
  "app": {
    "macOSPrivateApi": true,
    "windows": [
      {
        "title": "SSH Box",
        "width": 1360,
        "height": 860,
        "minWidth": 960,
        "minHeight": 600,
        "titleBarStyle": "Overlay",
        "hiddenTitle": true,
        "transparent": true
      }
    ],
    "security": {
      "csp": null
    }
  }
}
```

- [ ] **Step 2: Kiểm tra biên dịch Rust workspace (`cargo check`)**

Run: `cd ~/ssh-box/src-tauri && cargo check`
Expected: `Finished dev profile`

- [ ] **Step 3: Commit cấu trúc nền tảng**

```bash
cd ~/ssh-box
git add .
git commit -m "chore: scaffold tauri v2 and rust async core workspace"
```

---

### Task 2: Bộ gom gói 120fps `frame_batcher.rs` (Zero-Jank Terminal Pipeline)

**Files:**
- Create: `~/ssh-box/src-tauri/src/frame_batcher.rs`
- Modify: `~/ssh-box/src-tauri/src/lib.rs`

- [ ] **Step 1: Viết unit test kiểm thử gom gói 8ms / 64KB trong `frame_batcher.rs`**

```rust
// ~/ssh-box/src-tauri/src/frame_batcher.rs
use bytes::BytesMut;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio::sync::mpsc;

pub const FRAME_WINDOW: Duration = Duration::from_millis(8);
pub const MAX_BATCH_BYTES: usize = 64 * 1024;

pub async fn batch_stream_to_sink<R, F>(
    mut reader: R,
    mut on_batch: F,
    mut shutdown_rx: mpsc::Receiver<()>,
) -> std::io::Result<()>
where
    R: AsyncRead + Unpin,
    F: FnMut(Vec<u8>) -> bool,
{
    let mut read_buf = [0u8; 16 * 1024];
    let mut batch = BytesMut::with_capacity(MAX_BATCH_BYTES);
    let sleep = tokio::time::sleep(FRAME_WINDOW);
    tokio::pin!(sleep);
    let mut timer_armed = false;

    loop {
        tokio::select! {
            biased;
            _ = shutdown_rx.recv() => {
                if !batch.is_empty() {
                    let _ = on_batch(batch.split().to_vec());
                }
                return Ok(());
            }
            _ = &mut sleep, if timer_armed => {
                if !batch.is_empty() && !on_batch(batch.split().to_vec()) {
                    return Ok(());
                }
                timer_armed = false;
            }
            res = reader.read(&mut read_buf) => {
                match res {
                    Ok(0) => {
                        if !batch.is_empty() {
                            let _ = on_batch(batch.split().to_vec());
                        }
                        return Ok(());
                    }
                    Ok(n) => {
                        batch.extend_from_slice(&read_buf[..n]);
                        if batch.len() >= MAX_BATCH_BYTES {
                            if !on_batch(batch.split().to_vec()) {
                                return Ok(());
                            }
                            timer_armed = false;
                        } else if !timer_armed {
                            sleep.as_mut().reset(tokio::time::Instant::now() + FRAME_WINDOW);
                            timer_armed = true;
                        }
                    }
                    Err(e) => return Err(e),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[tokio::test]
    async fn coalesces_large_flood_without_dropping_bytes() {
        let payload: Vec<u8> = (0..(256 * 1024)).map(|i| (i % 251) as u8).collect();
        let reader = Cursor::new(payload.clone());
        let (_tx, rx) = mpsc::channel(1);
        let mut received = Vec::new();
        let mut batch_count = 0usize;

        batch_stream_to_sink(
            reader,
            |chunk| {
                assert!(chunk.len() <= MAX_BATCH_BYTES + 16 * 1024);
                batch_count += 1;
                received.extend_from_slice(&chunk);
                true
            },
            rx,
        )
        .await
        .unwrap();

        assert_eq!(received, payload);
        assert!(batch_count >= 4);
    }
}
```

- [ ] **Step 2: Chạy `cargo test frame_batcher`**

Run: `cd ~/ssh-box/src-tauri && cargo test frame_batcher -- --nocapture`
Expected: `test frame_batcher::tests::coalesces_large_flood_without_dropping_bytes ... ok`

- [ ] **Step 3: Commit `frame_batcher.rs`**

```bash
cd ~/ssh-box
git add src-tauri/src/frame_batcher.rs src-tauri/src/lib.rs
git commit -m "feat(terminal): add 8ms/120Hz frame-batched binary stream coalescer"
```

---

### Task 3: Bộ đọc `~/.ssh/config` + Quản lý Host (`host_store.rs`) & macOS Keychain (`vault.rs`)

**Files:**
- Create: `~/ssh-box/src-tauri/src/host_store.rs`
- Create: `~/ssh-box/src-tauri/src/vault.rs`
- Modify: `~/ssh-box/src-tauri/src/lib.rs`

- [ ] **Step 1: Viết `host_store.rs` kèm unit tests parse `~/.ssh/config` và hợp nhất `hosts.json`**

```rust
// ~/ssh-box/src-tauri/src/host_store.rs
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub id: String,
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    pub identity_file: Option<String>,
    pub group: String,
    pub source: HostSource,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HostSource {
    SshConfig,
    Custom,
}

pub fn parse_ssh_config_str(input: &str) -> Vec<HostEntry> {
    let mut hosts = Vec::new();
    let mut current_aliases: Vec<String> = Vec::new();
    let mut fields: BTreeMap<String, String> = BTreeMap::new();

    let flush = |aliases: &mut Vec<String>,
                 map: &mut BTreeMap<String, String>,
                 out: &mut Vec<HostEntry>| {
        for alias in aliases.drain(..) {
            if alias.contains('*') || alias.contains('?') {
                continue;
            }
            let hostname = map
                .get("hostname")
                .cloned()
                .unwrap_or_else(|| alias.clone());
            let user = map
                .get("user")
                .cloned()
                .unwrap_or_else(|| std::env::var("USER").unwrap_or_else(|_| "root".into()));
            let port = map
                .get("port")
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(22);
            let identity_file = map.get("identityfile").cloned();
            out.push(HostEntry {
                id: format!("sshcfg:{alias}"),
                alias,
                hostname,
                user,
                port,
                identity_file,
                group: "~/.ssh/config".into(),
                source: HostSource::SshConfig,
            });
        }
        map.clear();
    };

    for raw_line in input.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let key = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        let val = parts
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches('=')
            .trim()
            .trim_matches('"');
        if key == "host" {
            flush(&mut current_aliases, &mut fields, &mut hosts);
            current_aliases = val.split_whitespace().map(|s| s.to_string()).collect();
        } else if !current_aliases.is_empty() && !val.is_empty() {
            fields.entry(key).or_insert_with(|| val.to_string());
        }
    }
    flush(&mut current_aliases, &mut fields, &mut hosts);
    hosts
}

pub fn load_merged_hosts(ssh_config_path: &Path, custom_json_path: &Path) -> Vec<HostEntry> {
    let mut result = Vec::new();
    if let Ok(raw) = std::fs::read_to_string(ssh_config_path) {
        result.extend(parse_ssh_config_str(&raw));
    }
    if let Ok(raw) = std::fs::read_to_string(custom_json_path) {
        if let Ok(custom) = serde_json::from_str::<Vec<HostEntry>>(&raw) {
            result.extend(custom);
        }
    }
    result
}

pub fn default_paths() -> (PathBuf, PathBuf) {
    let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
    let ssh_cfg = home.join(".ssh").join("config");
    let custom_cfg = dirs::config_dir()
        .unwrap_or_else(|| home.join(".config"))
        .join("ssh-box")
        .join("hosts.json");
    (ssh_cfg, custom_cfg)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hosts_and_skips_wildcards() {
        let sample = r#"
            Host *
                ServerAliveInterval 60

            Host prod-api
                HostName 10.0.1.25
                User ubuntu
                Port 2222
                IdentityFile ~/.ssh/id_ed25519

            Host staging-db
                HostName db.internal
        "#;
        let parsed = parse_ssh_config_str(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].alias, "prod-api");
        assert_eq!(parsed[0].hostname, "10.0.1.25");
        assert_eq!(parsed[0].user, "ubuntu");
        assert_eq!(parsed[0].port, 2222);
        assert_eq!(
            parsed[0].identity_file.as_deref(),
            Some("~/.ssh/id_ed25519")
        );
        assert_eq!(parsed[1].alias, "staging-db");
        assert_eq!(parsed[1].port, 22);
    }
}
```

- [ ] **Step 2: Viết `vault.rs` tích hợp macOS Keychain**

```rust
// ~/ssh-box/src-tauri/src/vault.rs
const SERVICE_NAME: &str = "com.darien.sshbox";

pub fn set_host_secret(host_id: &str, secret: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_id).map_err(|e| e.to_string())?;
    entry.set_password(secret).map_err(|e| e.to_string())
}

pub fn get_host_secret(host_id: &str) -> Result<Option<String>, String> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_id).map_err(|e| e.to_string())?;
    match entry.get_password() {
        Ok(pw) => Ok(Some(pw)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(e) => Err(e.to_string()),
    }
}

pub fn delete_host_secret(host_id: &str) -> Result<(), String> {
    let entry = keyring::Entry::new(SERVICE_NAME, host_id).map_err(|e| e.to_string())?;
    match entry.delete_credential() {
        Ok(()) | Err(keyring::Error::NoEntry) => Ok(()),
        Err(e) => Err(e.to_string()),
    }
}
```

- [ ] **Step 3: Chạy `cargo test host_store`**

Run: `cd ~/ssh-box/src-tauri && cargo test host_store -- --nocapture`
Expected: `test host_store::tests::parses_hosts_and_skips_wildcards ... ok`

- [ ] **Step 4: Commit `host_store.rs` và `vault.rs`**

```bash
cd ~/ssh-box
git add src-tauri/src/host_store.rs src-tauri/src/vault.rs src-tauri/src/lib.rs
git commit -m "feat(hosts): add ~/.ssh/config parser and macOS Keychain vault"
```

---

### Task 4: Bộ phân tích Thông số Server kiểu Server Box (`telemetry.rs`)

**Files:**
- Create: `~/ssh-box/src-tauri/src/telemetry.rs`
- Modify: `~/ssh-box/src-tauri/src/lib.rs`

- [ ] **Step 1: Viết `telemetry.rs` kèm bộ tính delta CPU/Network và unit tests**

```rust
// ~/ssh-box/src-tauri/src/telemetry.rs
use serde::{Deserialize, Serialize};

pub const TELEMETRY_PROBE_CMD: &str = r#"LC_ALL=C sh -c 'echo "---LOAD---"; cat /proc/loadavg 2>/dev/null; echo "---CPU---"; head -n 1 /proc/stat 2>/dev/null; echo "---MEM---"; grep -E "^(MemTotal|MemAvailable|SwapTotal|SwapFree):" /proc/meminfo 2>/dev/null; echo "---NET---"; cat /proc/net/dev 2>/dev/null; echo "---DISK---"; df -kP / 2>/dev/null | tail -n 1; echo "---PROC---"; ps -eo pid,pcpu,pmem,comm --sort=-pcpu 2>/dev/null | head -n 6'"#;

#[derive(Debug, Clone, Default)]
pub struct RawCpuCounters {
    pub total: u64,
    pub idle: u64,
    pub rx_bytes: u64,
    pub tx_bytes: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProcessItem {
    pub pid: u32,
    pub cpu_pct: f32,
    pub mem_pct: f32,
    pub command: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
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
                if let Some(first) = trimmed.split_whitespace().next() {
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
                    cpu_idle = vals.get(3).copied().unwrap_or(0) + vals.get(4).copied().unwrap_or(0);
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
            .replace("eth0: 102400", "eth0: 307200");

        let (_, prev) = parse_linux_telemetry("prod", sample1, None, 2.0);
        let (m2, _) = parse_linux_telemetry("prod", &sample2, Some(&prev), 2.0);

        // Delta total = 11000 - 10000 = 1000, delta idle = 9000 - 8500 = 500 => 50% CPU
        assert!((m2.cpu_percent - 50.0).abs() < 0.1);
        assert_eq!(m2.mem_used_mb, 4000);
        assert_eq!(m2.mem_total_mb, 8000);
        // Delta RX = 204800 bytes = 200 KB over 2s => 100 KB/s
        assert!((m2.net_rx_kbps - 100.0).abs() < 0.1);
        assert_eq!(m2.top_processes.len(), 1);
        assert_eq!(m2.top_processes[0].command, "nginx");
    }
}
```

- [ ] **Step 2: Chạy `cargo test telemetry`**

Run: `cd ~/ssh-box/src-tauri && cargo test telemetry -- --nocapture`
Expected: `test telemetry::tests::computes_accurate_cpu_and_network_deltas ... ok`

- [ ] **Step 3: Commit `telemetry.rs`**

```bash
cd ~/ssh-box
git add src-tauri/src/telemetry.rs src-tauri/src/lib.rs
git commit -m "feat(telemetry): add Server Box style Linux metrics parser and delta calculator"
```

---

### Task 5: Quản lý Local PTY (`local_pty.rs`) & SSH Multiplexing (`ssh_pool.rs`)

**Files:**
- Create: `~/ssh-box/src-tauri/src/local_pty.rs`
- Create: `~/ssh-box/src-tauri/src/ssh_pool.rs`
- Modify: `~/ssh-box/src-tauri/src/lib.rs`

- [ ] **Step 1: Viết `local_pty.rs` quản lý phiên shell nội bộ (`zsh`/`bash`) kèm integration test**

```rust
// ~/ssh-box/src-tauri/src/local_pty.rs
use crate::frame_batcher::batch_stream_to_sink;
use portable_pty::{native_pty_system, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct LocalPaneHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    shutdown_tx: mpsc::Sender<()>,
}

#[derive(Default, Clone)]
pub struct LocalPtyManager {
    panes: Arc<Mutex<HashMap<String, LocalPaneHandle>>>,
}

impl LocalPtyManager {
    pub fn spawn_pane<F>(
        &self,
        pane_id: String,
        cols: u16,
        rows: u16,
        shell_override: Option<&str>,
        mut on_output: F,
    ) -> Result<(), String>
    where
        F: FnMut(Vec<u8>) -> bool + Send + 'static,
    {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let shell = shell_override
            .map(|s| s.to_string())
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/zsh".to_string());

        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        if let Some(home) = dirs::home_dir() {
            cmd.cwd(home);
        }

        let _child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        let (tx_bytes, rx_bytes) = tokio::sync::mpsc::channel::<Vec<u8>>(64);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        // Dedicated OS thread for blocking PTY read -> async Tokio bridge
        std::thread::spawn(move || {
            let mut buf = [0u8; 16 * 1024];
            while let Ok(n) = reader.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if tx_bytes.blocking_send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });

        tokio::spawn(async move {
            let stream_reader = tokio_util_channel_reader(rx_bytes);
            let _ = batch_stream_to_sink(stream_reader, move |chunk| on_output(chunk), shutdown_rx)
                .await;
        });

        self.panes.lock().unwrap().insert(
            pane_id,
            LocalPaneHandle {
                master: pair.master,
                writer,
                shutdown_tx,
            },
        );
        Ok(())
    }

    pub fn write_input(&self, pane_id: &str, data: &[u8]) -> Result<(), String> {
        let mut map = self.panes.lock().unwrap();
        let pane = map.get_mut(pane_id).ok_or("Pane not found")?;
        pane.writer.write_all(data).map_err(|e| e.to_string())?;
        pane.writer.flush().map_err(|e| e.to_string())
    }

    pub fn resize(&self, pane_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let map = self.panes.lock().unwrap();
        let pane = map.get(pane_id).ok_or("Pane not found")?;
        pane.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    }

    pub fn close(&self, pane_id: &str) {
        if let Some(pane) = self.panes.lock().unwrap().remove(pane_id) {
            let _ = pane.shutdown_tx.try_send(());
        }
    }
}
```

- [ ] **Step 2: Viết `ssh_pool.rs` hỗ trợ SSH Multiplexing (`russh`) cho nhiều PTY + Telemetry**

- [ ] **Step 3: Chạy `cargo test` xác minh toàn bộ unit + integration tests của Backend**

Run: `cd ~/ssh-box/src-tauri && cargo test -- --nocapture`
Expected: Tất cả tests trong `frame_batcher`, `host_store`, `telemetry`, `local_pty` đều `PASS`.

- [ ] **Step 4: Commit `local_pty.rs` và `ssh_pool.rs`**

```bash
cd ~/ssh-box
git add src-tauri/src/
git commit -m "feat(core): implement local PTY manager and multiplexed russh session pool"
```

---

### Task 6: Xây dựng Frontend WebGL2 Terminal, Split Panes & Server Box Telemetry Drawer

**Files:**
- Create: `~/ssh-box/src/types.ts`
- Create: `~/ssh-box/src/components/TerminalPane.tsx`
- Create: `~/ssh-box/src/components/SplitWorkspace.tsx`
- Create: `~/ssh-box/src/components/HostSidebar.tsx`
- Create: `~/ssh-box/src/components/TelemetryDrawer.tsx`
- Create: `~/ssh-box/src/App.tsx`

- [ ] **Step 1: Cài đặt `TerminalPane.tsx` tích hợp `@xterm/addon-webgl`, `@xterm/addon-fit`, `@xterm/addon-unicode11` và Binary Channel**
- [ ] **Step 2: Cài đặt `SplitWorkspace.tsx` hỗ trợ cây nhị phân đệ quy (`Cmd+D` chia dọc, `Cmd+Shift+D` chia ngang, `Cmd+W` đóng pane)**
- [ ] **Step 3: Cài đặt `HostSidebar.tsx` và `TelemetryDrawer.tsx` (hiển thị sparkline CPU, RAM, Swap, Disk, Network RX/TX, Top Processes)**
- [ ] **Step 4: Kiểm tra build toàn diện (`npm run build` & `cargo check`)**

Run: `cd ~/ssh-box && npm run build && cd src-tauri && cargo test`
Expected: Frontend bundle build thành công và toàn bộ Rust tests `PASS`.

- [ ] **Step 5: Commit giao diện hoàn chỉnh**

```bash
cd ~/ssh-box
git add .
git commit -m "feat(ui): add WebGL2 120fps split-pane terminal and Server Box telemetry drawer"
```
