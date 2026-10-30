# Mở rộng Cấu hình VPS sang Windows và macOS — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Cho Cấu hình VPS làm việc với máy Windows và macOS mà app SSH vào. Mỗi tab trên mỗi hệ ở một trong ba mức Đầy đủ / Chỉ xem / Ẩn, do backend quyết định. Hành vi trên Linux giữ nguyên.

**Architecture:**
- **Nhận diện hệ máy.** Probe gửi riêng `uname -s` trước. Kết quả `Linux` / `Darwin` thì chạy probe Unix sẵn có (thêm nhánh cho macOS). Kết quả khác thì chạy probe Windows qua một bộ chạy PowerShell mới, bộ này nhận toàn bộ script qua stdin.
- **Bảng mức hỗ trợ** (`VpsSupport`) được tính bằng một hàm Rust thuần từ `VpsProbe` rồi gửi kèm probe. Frontend chỉ hiển thị theo bảng.
- **Code theo hệ máy nằm trong file riêng:** `vps_macos.rs`, `vps_windows.rs`. `vps_config.rs` chỉ rẽ nhánh theo `os_family`.

**Tech Stack:** Rust (Tauri v2, `russh`, `serde`, `serde_json`, `base64 0.22`, `tokio`), React 19 + TypeScript, Tailwind CSS v4, PowerShell 5.1 phía Windows. Không thêm dependency mới.

**Spec:** `docs/superpowers/specs/2026-10-30-server-config-multi-os-design.md`. Spec này dựa trên `docs/superpowers/specs/2026-10-30-vps-server-center-design.md` (spec gốc). Người thực hiện đọc cả hai spec cùng plan này.

## Global Constraints

- Mọi quy định của spec gốc vẫn áp dụng, trừ chỗ spec mới ghi khác.
- **Linux không đổi hành vi.** Mọi test Linux sẵn có phải qua. Ma trận Linux (`src-tauri/tests/vps_matrix.rs` trên `u24`, `d13`, `r9`) phải qua lại ở Task 11.
- Ba mức: Đầy đủ / Chỉ xem / Ẩn. Backend quyết định qua `compute_support`; frontend không tự suy luận theo hệ máy.
- **Quy tắc cứng:** thay đổi SSH chỉ ở mức Đầy đủ trên một hệ khi cơ chế tự hoàn tác của hệ đó đã qua ma trận §7.3. Firewall được bảo vệ bằng bộ chặn như Linux.
- **Theo khả năng của máy, không theo danh sách phiên bản** (spec §3.1).
  - macOS SSH ghi cần dòng `Include …sshd_config.d…` đứng đầu.
  - Windows SSH ghi cần dịch vụ `sshd` đang chạy và module `ScheduledTasks`.
  - Windows Firewall ghi cần module `NetSecurity`.
- **Bí mật (mật khẩu, nội dung file) chỉ đi qua stdin**, không bao giờ nằm trong dòng lệnh.
- **PowerShell 5.1 không tự báo lỗi khi lệnh ngoài (`sshd.exe`, …) trả mã khác 0.** Mỗi lần gọi phải kiểm `$LASTEXITCODE` rồi `throw`.
- Lỗi backend có dạng `MÃ: chi tiết`. Mã mới nằm trong `ERR_UNSUPPORTED` với các lý do: `unsupported_os`, `windows_not_admin`, `windows_no_sshd`, `windows_ssh_read_only`, `macos_no_dropin`, `read_only_os`, `hidden_setting`, `windows_rule_not_managed`.
- **Giao diện:** theo khuôn spec gốc §3. Rộng tối thiểu 360px. Không có thông tin chỉ hiện qua `title` / `group-hover`. Chạy được trên 5 nền tảng app.
- **Chữ:** mọi chuỗi lấy từ `getI18n(lang)`, có đủ VI và EN. Không dùng `isVi ? … : …`.
- **Kiểm thử:** chỉ trên máy ảo dùng một lần (UTM: Windows 11 ARM, macOS; Lima: Linux), không bao giờ trên máy thật. Trước khi tải bất cứ thứ gì phải hỏi người dùng, kèm tên file, nguồn và dung lượng.
- **Cách làm:** mỗi task tối đa khoảng 400 dòng code, làm tuần tự Task 1 → 11. Commit sau mỗi task theo kiểu `feat(vps): …`. Chỉ push khi người dùng yêu cầu.
- **Nơi làm việc:** worktree `~/ssh-box/.worktrees/server-config-multi-os`, nhánh `feat/server-config-multi-os`. Không `git checkout` trong thư mục chính.

## Review Focus

1. **Máy Windows có shell mặc định là PowerShell** (đặt `DefaultShell` trong registry) **thay vì `cmd`.** Probe và bộ chạy lệnh phải chạy như nhau với cả hai. → Task 2 `powershell_command_is_plain_ascii_safe_in_cmd_and_powershell`; Task 4 bước đổi `DefaultShell`; Task 11 dòng ma trận.
2. **Máy Windows có Git for Windows trong PATH.** Khi đó `uname -s` trả `MINGW64_NT-…` dù shell là `cmd`. Probe phải chuyển sang probe Windows, không được báo "không hỗ trợ". → Task 1 `uname_from_git_for_windows_falls_through_to_windows_probe`.
3. **Tên user hoặc tên rule có dấu tiếng Việt trên Windows.** Output phải tới dạng UTF-8 nguyên vẹn. → Task 2 `windows_probe_keeps_vietnamese_user_name`; Task 8 `windows_firewall_parses_unicode_display_name`.
4. **`sshd_config` của Windows từng được sửa bằng Notepad** (có BOM UTF-8 và CRLF). Phải giữ nguyên BOM và CRLF, và chèn directive trước `Match`. → Task 10 `windows_render_keeps_bom_and_crlf_and_inserts_before_match`.
5. **App vào máy macOS/Windows bằng IPv6.** `SSH_CONNECTION` có địa chỉ IPv6, và bộ chặn phải nhận đúng IP. → Task 2 `windows_probe_parses_ipv6_ssh_connection`; Task 1 `macos_probe_parses_ipv6_ssh_connection`.

---

## Cấu trúc file

| File | Trách nhiệm |
|---|---|
| `src-tauri/src/vps_support.rs` (mới) | `OsFamily`, `SupportLevel`, `VpsSupport`, `OsCaps`, `compute_support`, `check_hidden_rows`. Hàm thuần, không I/O |
| `src-tauri/src/vps_windows.rs` (mới) | Bộ chạy PowerShell, probe Windows, script đọc/ghi SSH, Firewall, Users & Keys của Windows, parser JSON |
| `src-tauri/src/vps_macos.rs` (mới) | Script apply/rollback SSH qua `launchd`, script đọc user của macOS |
| `src-tauri/src/vps_config.rs` | `VpsProbe` thêm trường; `probe()` nhận diện hệ máy; các hàm công khai rẽ nhánh theo `os_family` |
| `src-tauri/src/vps_guard.rs` | Tách `check_sshd_auth`; thêm `check_firewall_delete_windows` |
| `src-tauri/src/lib.rs` | `pub mod vps_support; pub mod vps_windows; pub mod vps_macos;` (lệnh Tauri không đổi tên) |
| `src/types.ts`, `src/i18n.ts` | Kiểu dữ liệu mới, chuỗi VI/EN |
| `src/components/VpsConfigWorkspace.tsx`, `src/components/vps/{SshHardening,Firewall,AccessKeys}Studio.tsx` | Hiển thị theo `probe.support` |
| `src-tauri/tests/vps_matrix.rs` | Thêm `MX_DISTRO=macos` và `MX_DISTRO=windows` |

**Lệnh kiểm tra dùng chung** (chạy trong worktree):
- **Unit:** `CARGO_TARGET_DIR=~/ssh-box/src-tauri/target cargo test --manifest-path src-tauri/Cargo.toml vps`
- **Frontend:** `~/ssh-box/node_modules/.bin/tsc --noEmit -p . && ~/ssh-box/node_modules/.bin/vite build`
- **Xem giao diện:** làm theo memory dự án.
  - Thêm một entry tạm vào `~/ssh-box/.claude/launch.json`: `~/ssh-box/node_modules/.bin/vite <worktree> --port 1430 --strictPort`.
  - Tạo một trang HTML stub định nghĩa `window.__TAURI_INTERNALS__`, trả `VpsProbe` mẫu cho từng hệ máy.
  - Kiểm ở 360×800 và 1280×800, theme `zinc-dark` và `zinc-light`, VI và EN.
  - Xong thì xoá entry và stub tạm.

---

### Task 1: Nhận diện hệ máy và bảng mức hỗ trợ (spec §3, §5.1)

**Files:**
- Create `src-tauri/src/vps_support.rs`.
- Modify `src-tauri/src/vps_config.rs`:
  - `PrivilegeLevel`, `FirewallEngine`, `VpsProbe`, `PROBE_SCRIPT`, `parse_probe_output`, `probe`, `engine_name`, `apply_preflight`;
  - tách `parse_ssh_connection` ra khỏi `parse_probe_output`.
- Modify `src-tauri/src/lib.rs` (khai báo module).
- Test: `mod tests` trong `vps_support.rs` và trong `vps_config.rs`.

**Interfaces — Produces:**
```rust
// vps_support.rs — mọi type: #[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)], #[serde(rename_all = "camelCase")]
pub enum OsFamily { Linux, Macos, Windows }
pub enum SupportLevel { Full, ReadOnly, Hidden }
pub struct TabSupport { pub level: SupportLevel, pub reason: Option<String> } // reason = khoá trong t.vpsUnsupportedReasons
pub struct SshRows { pub port: bool, pub permit_root_login: bool, pub allow_deny_users: bool } // true = hiện
pub struct VpsSupport { pub ssh: TabSupport, pub ssh_rows: SshRows, pub firewall: TabSupport, pub access: TabSupport, pub password_coupled: bool }
#[derive(Default)] pub struct OsCaps { pub sshd_dropin: bool, pub sshd_running: bool, pub has_scheduled_tasks: bool, pub has_net_security: bool }
pub fn compute_support(p: &VpsProbe) -> VpsSupport;
pub fn check_hidden_rows(rows: &SshRows, current: &SshdSettings, desired: &SshdSettings) -> Result<(), String>; // UNSUPPORTED: hidden_setting
pub fn unix_family(uname_stdout: Option<&str>) -> Option<OsFamily>; // None = thử probe Windows

// vps_config.rs
pub enum PrivilegeLevel { Root, SudoNoPassword, SudoPassword, Admin, None }   // thêm Admin
pub enum FirewallEngine { Ufw, Firewalld, Iptables, Windows, None }            // thêm Windows
pub struct VpsProbe { /* các trường cũ */ pub os_family: OsFamily, pub support: VpsSupport, #[serde(skip)] pub caps: OsCaps }
pub(crate) fn parse_ssh_connection(conn: &str) -> Result<(String, String, u16), String>; // (client_ip, server_ip, server_port)
```
- **`unix_family`:** `Some("Linux")` → `Linux`, `Some("Darwin")` → `Macos`. Các trường hợp còn lại, kể cả `None` (lệnh lỗi) và `MINGW…` / `CYGWIN…` / `MSYS…`, đều ra `None`.
- **`probe()`:**
  1. `find_remote_host`.
  2. `pool.get_or_connect(&host, None).await?` để lỗi kết nối được trả ra nguyên văn.
  3. `pool.exec_bytes_on_host(&host, "uname -s", None)`. `Ok` thì lấy stdout đã trim, `Err` thì coi như `None`.
  4. `unix_family` ra `Some` thì chạy `PROBE_SCRIPT` như cũ. Ra `None` thì tạm trả `UNSUPPORTED: unsupported_os`; Task 2 thay chỗ này bằng probe Windows.
  5. Cuối cùng gán `probe.support = compute_support(&probe)`.
- **`PROBE_SCRIPT`** thêm phần cuối, chỉ chạy khi `uname -s` là `Darwin`:
  - `echo "MAC_VERSION:$(sw_vers -productVersion)"`;
  - `echo ---SSHD_MAIN---`;
  - `cat /etc/ssh/sshd_config`. File này mọi user đọc được trên macOS.
- **`parse_probe_output`:**
  - Chấp nhận `UNAME:Darwin`, gán `os_id = "macOS"`, `os_version = MAC_VERSION`, `caps.sshd_dropin = sshd_write_target(<phần sau ---SSHD_MAIN--->) == WriteTarget::DropIn`.
  - `UNAME` không phải `Linux` hay `Darwin` → `UNSUPPORTED: unsupported_os`. Lý do cũ `linux_only` chỉ còn dùng cho host `local`.
- **`compute_support` trong task này:**
  - **Linux** (giữ đúng hành vi hiện tại):
    - `ssh`: `Full`; không có systemd thì `ReadOnly` + `no_systemd`.
    - `firewall`: `Ufw` / `Firewalld` → `Full`; `Iptables` → `ReadOnly` + `firewall_read_only`; `None` → `ReadOnly` không kèm lý do.
    - `access`: `Full`.
    - Mọi dòng của `ssh_rows` đều hiện; `password_coupled = false`.
  - **macOS và Windows:** mọi tab `Hidden`. Các task sau mở dần từng ô.
- **`check_hidden_rows`:** trả lỗi nếu một dòng đang ẩn có giá trị khác nhau giữa `current` và `desired`:
  - `port`;
  - `permit_root_login`;
  - `allow_users` hoặc `deny_users` (cả hai cùng thuộc dòng `allow_deny_users`).
- **`apply_preflight`:** thay `if !probe.has_systemd` bằng `if probe.support.ssh.level != SupportLevel::Full`, trả `UNSUPPORTED: <reason hoặc "read_only_os">`. Ngay sau đó gọi `check_hidden_rows`.

- [ ] **Step 1: Viết test.** Mỗi test có một dòng comment nói nó bảo vệ điều gì.
```rust
// vps_support.rs
fn linux_support_reproduces_current_behaviour()   // 4 probe Linux: có/không systemd × ufw/iptables/none → đúng như danh sách trên
fn macos_and_windows_start_hidden()                // mọi tab Hidden
fn uname_from_git_for_windows_falls_through_to_windows_probe() // unix_family(Some("MINGW64_NT-10.0-22631")) == None
fn uname_linux_and_darwin_are_unix()              // Some("Linux")→Linux, Some("Darwin")→Macos, None→None
fn hidden_row_change_is_rejected()                 // rows.port=false, desired.port 2222 ≠ 22 → Err bắt đầu "UNSUPPORTED: hidden_setting"
fn visible_row_change_passes()
// vps_config.rs
fn macos_probe_parses_version_and_dropin()        // UNAME:Darwin, MAC_VERSION:15.1, ---SSHD_MAIN--- "Include /etc/ssh/sshd_config.d/*\n..." → os_family Macos, os_id "macOS", caps.sshd_dropin
fn macos_probe_without_include_has_no_dropin()
fn macos_probe_parses_ipv6_ssh_connection()       // "2001:db8::7 51000 2001:db8::2 22"
fn probe_rejects_unknown_unix()                    // UNAME:FreeBSD → Err bắt đầu "UNSUPPORTED: unsupported_os"
fn preflight_blocks_when_ssh_not_full()           // support.ssh = ReadOnly + "no_systemd" → Err "UNSUPPORTED: no_systemd" (đúng như cũ)
```
Sửa test cũ `probe_rejects_non_linux` thành `probe_rejects_unknown_unix`. Lý do: `Darwin` giờ đã hợp lệ.
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL (chưa có `vps_support`).
- [ ] **Step 3:** Viết code theo Interfaces.
- [ ] **Step 4:** Chạy lệnh Unit. Kỳ vọng: PASS, và mọi test Linux cũ vẫn qua.
- [ ] **Step 5:** Commit `feat(vps): detect host OS family and compute per-tab support levels`.

---

### Task 2: Bộ chạy PowerShell và probe Windows (spec §5.1, §5.2)

**Files:** Create `src-tauri/src/vps_windows.rs`; Modify `src-tauri/src/vps_config.rs` (nhánh `None` của `probe()`). Test trong `mod tests` của `vps_windows.rs`.

**Interfaces:**
- **Consumes:** `parse_ssh_connection`, `classify_privileged_output`, `find_remote_host`, `compute_support` (Task 1).
- **Produces:**
```rust
pub const PS_BOOTSTRAP: &str;
pub const PS_SECRET_SEP: &str = "\nSSHBOX_SECRET_BEGIN\n";
pub fn build_powershell_command() -> String;  // "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand <base64(UTF-16LE(PS_BOOTSTRAP))>"
pub fn powershell_stdin(script: &str, secret: Option<&str>) -> Vec<u8>; // UTF-8: script + PS_SECRET_SEP + secret
pub fn ps_quote(s: &str) -> String;            // '…' và nhân đôi dấu '
pub async fn run_windows_script(host_id: &str, pool: &SshPool, script: &str, secret: Option<&str>) -> Result<String, String>;
pub const WINDOWS_PROBE_SCRIPT: &str;
pub fn parse_windows_probe(raw: &str, auth: AuthUsed) -> Result<VpsProbe, String>;
pub async fn probe_windows(host: &HostEntry, pool: &SshPool) -> Result<VpsProbe, String>;
```
- **Lý do script đi qua stdin, không qua dòng lệnh:** `cmd.exe` giới hạn dòng lệnh ở 8191 ký tự, mà một script dạng base64 UTF-16 dài gấp khoảng 2,7 lần bản gốc. Vì vậy `-EncodedCommand` chỉ chở `PS_BOOTSTRAP` cố định và ngắn. `PS_BOOTSTRAP` đọc toàn bộ stdin, tách script và bí mật, rồi chạy script. Đây là thuật toán mà chữ ký hàm không tự quyết được:
```powershell
$ErrorActionPreference = 'Stop'; $ProgressPreference = 'SilentlyContinue'
[Console]::InputEncoding = New-Object Text.UTF8Encoding $false
[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false
$in = [Console]::In.ReadToEnd(); $sep = "`nSSHBOX_SECRET_BEGIN`n"; $i = $in.IndexOf($sep)
$SSHBOX_SECRET = $in.Substring($i + $sep.Length)
try { & ([ScriptBlock]::Create($in.Substring(0, $i))) 2>&1 | Out-String -Width 4096 }
catch { ($_ | Out-String -Width 4096); 'SSHBOX_EXIT_CODE:1' }
```
- **`run_windows_script`:** dùng `find_remote_host` và `pool.exec_bytes_on_host(&host, &build_powershell_command(), Some(&powershell_stdin(..)))`, rồi đưa output cho `classify_privileged_output`.
- **`WINDOWS_PROBE_SCRIPT`** in mỗi dòng một trường, theo dạng `KHOÁ:giá trị`:
  - `OS_FAMILY:windows`
  - `OS_CAPTION:` lấy từ `(Get-CimInstance Win32_OperatingSystem).Caption`
  - `OS_DISPLAY_VERSION:` lấy từ registry `HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion`, giá trị `DisplayVersion`
  - `USER:` lấy từ `[Environment]::UserName`
  - `ADMIN:1|0` theo `WindowsPrincipal.IsInRole(Administrator)`
  - `OPENSSH:` là `ProductVersion` của `$env:WINDIR\System32\OpenSSH\sshd.exe`
  - `SSHD_SERVICE:` là `(Get-Service sshd -ErrorAction SilentlyContinue).Status`
  - `SSH_CONNECTION:` là `$env:SSH_CONNECTION`
  - `SCHEDTASKS:1|0` và `NETSECURITY:1|0`, theo `Get-Module -ListAvailable`
- **`parse_windows_probe`:**
  - Không có `OS_FAMILY:windows` → `UNSUPPORTED: unsupported_os`.
  - `os_id = "Windows"`. `os_version` = `OS_CAPTION` bỏ tiền tố `Microsoft Windows `, nối thêm một dấu cách và `OS_DISPLAY_VERSION`, rồi trim.
  - `privilege`: `Admin` hoặc `None`.
  - `firewall_engine`: `Windows` nếu `NETSECURITY:1`, ngược lại `None`.
  - `has_systemd`, `ssh_socket_active`, `selinux_enforcing` đều `false`.
  - `caps`: `sshd_running` (`SSHD_SERVICE` là `Running`), `has_scheduled_tasks`, `has_net_security`.
  - `context` dùng `parse_ssh_connection`. Thiếu `SSH_CONNECTION` thì trả `UNSUPPORTED: no_ssh_connection`, giống Linux.
- **`probe()`:** nhánh `None` gọi `vps_windows::probe_windows`. Bảng mức hỗ trợ của Windows vẫn `Hidden` hết.

- [ ] **Step 1: Viết test**
```rust
// Script nằm trong lệnh thì vượt giới hạn 8191 ký tự của cmd, và bí mật lộ trong danh sách tiến trình.
fn powershell_command_is_constant_and_carries_no_script()   // build_powershell_command() giống nhau với mọi script; len < 8191; không chứa "SSHBOX_SECRET_BEGIN" ở dạng chữ thường
fn powershell_command_is_plain_ascii_safe_in_cmd_and_powershell() // phần sau "-EncodedCommand " chỉ gồm [A-Za-z0-9+/=]; cả lệnh không chứa % ^ & | < > " ` $
fn encoded_command_decodes_to_bootstrap()                   // base64 → UTF-16LE → == PS_BOOTSTRAP
fn stdin_carries_script_then_secret_utf8()                  // powershell_stdin("Write-Output 'Đạt'", Some("p'a$s")) == "Write-Output 'Đạt'\nSSHBOX_SECRET_BEGIN\np'a$s" dạng bytes UTF-8
fn ps_quote_doubles_single_quotes()                         // ps_quote("a'b") == "'a''b'"
fn windows_probe_admin_with_ssh_running()                   // privilege Admin, os_version "11 Pro 24H2", caps.sshd_running, firewall_engine Windows
fn windows_probe_non_admin()                                // ADMIN:0 → None
fn windows_probe_keeps_vietnamese_user_name()               // USER:Đạt → context.login_user == "Đạt"
fn windows_probe_parses_ipv6_ssh_connection()
fn windows_probe_without_marker_is_unsupported()            // output của cmd "'powershell.exe' is not recognized…" → Err bắt đầu "UNSUPPORTED: unsupported_os"
```
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces.
- [ ] **Step 4:** Chạy lệnh Unit. Kỳ vọng: PASS.
- [ ] **Step 5:** Commit `feat(vps): add stdin-fed PowerShell runner and Windows probe`.

---

### Task 3: Giao diện theo bảng mức hỗ trợ (spec §3, §5.3)

**Files:**
- Modify `src/types.ts`, `src/i18n.ts` (cả khối VI lẫn EN);
- Modify `src/components/VpsConfigWorkspace.tsx`;
- Modify `src/components/vps/SshHardeningStudio.tsx`, `FirewallStudio.tsx`, `AccessKeysStudio.tsx`.

**Interfaces:**
- **Consumes:** dạng JSON của `VpsProbe` sau Task 1–2.
- **Produces:**
```ts
export type OsFamily = "linux" | "macos" | "windows";
export type SupportLevel = "full" | "readOnly" | "hidden";
export interface TabSupport { level: SupportLevel; reason: string | null }
export interface VpsSupport { ssh: TabSupport; sshRows: { port: boolean; permitRootLogin: boolean; allowDenyUsers: boolean };
  firewall: TabSupport; access: TabSupport; passwordCoupled: boolean }
export type PrivilegeLevel = "root" | "sudoNoPassword" | "sudoPassword" | "admin" | "none";
export type FirewallEngine = "ufw" | "firewalld" | "iptables" | "windows" | "none";
// VpsProbe thêm: osFamily: OsFamily; support: VpsSupport
```

**Hành vi:**
- **`VpsConfigWorkspace`:**
  - Chưa có probe thì không vẽ thanh tab.
  - Có probe thì thanh tab chỉ gồm các tab có `level !== "hidden"`. Nếu tab đang mở bị ẩn thì chuyển sang tab đầu tiên còn hiện.
  - Studio chỉ được render khi tab của nó không ẩn.
  - `PRIVILEGE_TONE.admin = "ok"`, nhãn lấy từ `t.vpsPrivAdmin`.
  - Nút mở khoá sudo chỉ hiện khi `privilege === "none" && osFamily !== "windows"`.
- **Mỗi studio** đọc `probe.support.<tab>`.
  - `readOnly` khi `level === "readOnly"`. Riêng SSH còn thêm điều kiện `rollbackPending`, như cũ.
  - Có `reason` thì hiện `<Notice tone="warn">{t.vpsUnsupportedReasons[reason]}</Notice>`.
  - Bỏ các điều kiện cũ dựa trên `probe.hasSystemd` và `engine === "iptables"`.
- **`SshHardeningStudio`:** ẩn dòng Port khi `!sshRows.port`. Ẩn dòng PermitRootLogin, cùng mục kiểm tra `row-root`, khi `!sshRows.permitRootLogin`. Ẩn hai dòng AllowUsers/DenyUsers khi `!sshRows.allowDenyUsers`.
- **`FirewallStudio`:** `writable = support.firewall.level === "full"`. Nhãn mức phơi ra ngoài thêm `unknown`, màu `neutral`.
- **`AccessKeysStudio`:** ở mức `readOnly` thì ẩn form tạo user và mọi `ConfirmButton`.

**Chuỗi mới, chép nguyên văn (VI / EN):**

| Khoá | VI | EN |
|---|---|---|
| `vpsPrivAdmin` | Quản trị viên | Administrator |
| `vpsUnsupportedReasons.linux_only` (sửa) | Cấu hình VPS không dùng cho máy local | Server Config is not available for the local machine |
| `.unsupported_os` | Hệ điều hành của máy này chưa được hỗ trợ | This machine's operating system is not supported |
| `.windows_not_admin` | Tài khoản này không có quyền quản trị nên chỉ xem được. | This account is not an administrator, so settings are read only. |
| `.windows_no_sshd` | Dịch vụ OpenSSH Server (sshd) không chạy nên chỉ xem được. | The OpenSSH Server service (sshd) is not running, so settings are read only. |
| `.windows_ssh_read_only` | Trên Windows, cấu hình SSH hiện chỉ xem được. | On Windows, SSH settings are read only for now. |
| `.macos_no_dropin` | Bản macOS này không có thư mục sshd_config.d nên chỉ xem được. | This macOS version has no sshd_config.d folder, so settings are read only. |
| `.read_only_os` | Trên hệ điều hành này, mục này chỉ xem được. | On this operating system, this section is read only. |
| `.hidden_setting` | Cài đặt này không sửa được trên hệ điều hành này. | This setting cannot be changed on this operating system. |
| `.windows_rule_not_managed` | Chỉ xoá được rule do SSH Box tạo. | Only rules created by SSH Box can be deleted. |
| `vpsFwExposure.unknown` | Không xác định | Unknown |

- [ ] **Step 1:** Sửa `types.ts` và `i18n.ts`. Chạy lệnh Frontend. Kỳ vọng: FAIL ở các studio đang dùng `hasSystemd` / `PrivilegeLevel` (TypeScript báo thiếu nhánh `admin`).
- [ ] **Step 2:** Sửa `VpsConfigWorkspace` và 3 studio theo phần Hành vi.
- [ ] **Step 3:** Chạy lệnh Frontend. Kỳ vọng: PASS.
- [ ] **Step 4: Xem giao diện** bằng stub với 4 probe mẫu. Mỗi probe kiểm ở 360px và 1280px, theme sáng và tối, VI và EN. Kỳ vọng:
  - (a) Linux ufw có systemd: 3 tab, giống hệt bản trên `develop`.
  - (b) Linux không có systemd: tab SSH có notice `no_systemd`.
  - (c) macOS, mọi tab `hidden`: không có tab nào, thanh máy hiện `macOS 15.1`.
  - (d) Windows `admin`, SSH và Users ở `readOnly`, Firewall `hidden`, `sshRows` ẩn root/allow: chỉ có 2 tab, không có nút sudo, nhãn "Quản trị viên".
  
  Chụp màn hình (b), (c), (d). Xoá stub và entry tạm.
- [ ] **Step 5:** Commit `feat(vps): render Server Config tabs from backend support levels`.

---

### Task 4: Dựng máy ảo và thử nghiệm spec §8 (thủ công, không có code sản phẩm)

**Files:** Modify spec `docs/superpowers/specs/2026-10-30-server-config-multi-os-design.md`, thêm mục "§8.1 Kết quả thử nghiệm (ngày)". Khoá thử tạo trong scratchpad, không đặt trong repo.

- [ ] **Step 1: Hỏi người dùng trước khi tải** và chờ đồng ý. Liệt kê kèm dung lượng thật lấy từ nguồn:
  - UTM (`brew info --cask utm`);
  - file ISO Windows 11 ARM64 từ trang `https://www.microsoft.com/software-download/windows11arm64`. Nếu trang bắt chấp nhận điều khoản thì để người dùng tự tải;
  - file phục hồi macOS mà UTM tải về.
- [ ] **Step 2: Dựng máy ảo Windows 11 ARM.** 4 GB RAM, ổ 64 GB, mạng Shared.
  - Đi qua màn hình cài đặt. Người dùng tự làm, hoặc Claude làm bằng computer-use sau khi xin quyền cho UTM.
  - Tạo user admin `sshbox` và user thường `sshbox-std`.
  - Bật OpenSSH Server: `Add-WindowsCapability -Online -Name OpenSSH.Server~~~~0.0.1.0; Start-Service sshd; Set-Service sshd -StartupType Automatic`.
  - Đưa public key thử vào `C:\ProgramData\ssh\administrators_authorized_keys` (cho `sshbox`) và vào `C:\Users\sshbox-std\.ssh\authorized_keys`.
- [ ] **Step 3: Dựng máy ảo macOS.** 4 GB RAM, ổ 64 GB. Tạo user admin, chạy `sudo systemsetup -setremotelogin on`, đưa key vào `~/.ssh/authorized_keys`.
- [ ] **Step 4: Thử 6 điểm của spec §8**, cộng một điểm thứ 7. Mỗi điểm ghi lệnh đã chạy và kết quả.
  1. Chạy `Restart-Service sshd` từ một phiên SSH: phiên đó có bị rớt không?
  2. SSH bằng `sshbox`: `ADMIN:1` không? SSH bằng `sshbox-std`: `ADMIN:0` không?
  3. Phiên SSH trên Windows có `$env:SSH_CONNECTION` không?
  4. Gửi lệnh `build_powershell_command()` qua `ssh`, kèm stdin có chữ `Đạt`: chữ có in ra đúng không? Thử với cả `DefaultShell` là cmd (mặc định) và PowerShell (`New-ItemProperty HKLM:\SOFTWARE\OpenSSH -Name DefaultShell -Value C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe`).
  5. Trên macOS, nạp một LaunchDaemon `sleep 60; touch /tmp/x` bằng `launchctl bootstrap system`, rồi đóng phiên SSH: sau 60 giây file có xuất hiện không? Thử thêm `launchctl bootout` có huỷ được không.
  6. File `/etc/ssh/sshd_config` của máy ảo macOS có dòng `Include` đứng đầu không?
  7. Trên cả hai máy: đặt `PasswordAuthentication no` nhưng để nguyên `KbdInteractiveAuthentication`. Lệnh `ssh -o PreferredAuthentications=keyboard-interactive` còn đăng nhập bằng mật khẩu được không?
- [ ] **Step 5: Ghi kết quả vào spec.** Kết luận theo đúng các luật sau:
  - Điểm 1 **hoặc** 3 không đạt → **dừng, báo người dùng** trước khi làm Task 7–10.
  - Điểm 7 trên Windows là "còn đăng nhập được" → Task 10 đặt `password_coupled = true` cho Windows.
  - Điểm 5 không đạt → Task 5 dừng lại và báo người dùng.
- [ ] **Step 6:** Commit `docs(vps): record Windows/macOS spike results`. Dừng các máy ảo.

---

### Task 5: macOS — SSH (spec §6.1)

**Files:**
- Create `src-tauri/src/vps_macos.rs`.
- Modify `src-tauri/src/vps_config.rs`: `SshdSettings`, `parse_sshd_t`, `sshd_directives`, `get_sshd_view`, `apply_sshd_settings`, `finish_sshd_apply`, `revert_sshd_settings`, `sshd_cleanup_script`, `sshd_rollback_now_script`.
- Modify `src-tauri/src/vps_support.rs`, `src/types.ts`, `src/components/vps/SshHardeningStudio.tsx`.
- Test: chạy script thật bằng `sh` cục bộ, dùng `tests::shell` (`fake_bin`, `run_local`, `temp_dir`) trong `mod tests` của `vps_config.rs`.

**Interfaces — Produces:**
```rust
// vps_config.rs
pub struct SshdSettings { /* … */ pub kbd_interactive_authentication: bool } // mặc định true
// vps_macos.rs
pub const MACOS_ROLLBACK_LABEL: &str = "com.sshbox.rollback";
pub const MACOS_ROLLBACK_PLIST: &str = "/Library/LaunchDaemons/com.sshbox.rollback.plist";
pub fn build_sshd_apply_script_macos(file: &str, rollback_path: &str, plist_path: &str) -> String;
pub fn build_sshd_rollback_script_macos(file: &str, plist_path: &str) -> String;
// vps_config.rs
fn rollback_pending_cmd(os: OsFamily) -> String;   // Linux: lệnh systemctl hiện có; macOS: `launchctl print system/com.sshbox.rollback >/dev/null 2>&1 && echo active || true`
fn stop_rollback_timer_cmd(os: OsFamily) -> String; // Linux: `systemctl stop sshbox-rollback.timer …`; macOS: `launchctl bootout system/com.sshbox.rollback 2>/dev/null || true; rm -f <plist>`
fn sshd_cleanup_script(rollback: &str, files: [&str; 2], os: OsFamily) -> String;
fn sshd_rollback_now_script(os: OsFamily) -> String;
```
- **`parse_sshd_t`** đọc cả `kbdinteractiveauthentication` lẫn `challengeresponseauthentication` vào `kbd_interactive_authentication`. `sshd_directives` thêm dòng `KbdInteractiveAuthentication yes|no`. Với Linux, giá trị này không đổi trừ khi giao diện đổi nó, nên Linux không bao giờ bị ghi thêm dòng này.
- **Script apply của macOS** giống bản Linux, chỉ khác mấy chỗ sau:
  - không có `semanage`, không nạp lại `sshd`, không chạy `ss`;
  - kiểm tra đang có hoàn tác chờ bằng `rollback_pending_cmd`;
  - sau `sshd -t`, ghi script hoàn tác, rồi ghi một plist có `RunAtLoad` chạy `/bin/sh -c 'sleep 60; exec /bin/sh <rollback_path>'`;
  - `launchctl bootstrap system <plist>`, rồi in `NOW:` / `ROLLBACK_AT:`.
- **Script hoàn tác của macOS:** gọi `sshbox_restore` (dùng lại `sshd_restore_fn`), `rm -f <plist>`, `rm -f "$0"`, cuối cùng mới `launchctl bootout system/com.sshbox.rollback`. Bootout phải đứng cuối vì lệnh này dừng luôn chính tiến trình đang chạy script.
- **Các hàm công khai** gọi `probe()` để biết `os_family`, rồi chọn script tương ứng. `finish_sshd_apply` cũng gọi `probe()` ở đầu hàm.
- **`compute_support` cho macOS:**
  - `ssh`: `Full` nếu `caps.sshd_dropin`, ngược lại `ReadOnly` + `macos_no_dropin`;
  - `ssh_rows.port = false`;
  - `password_coupled = true`.
- **Giao diện:** khi `support.passwordCoupled`:
  - công tắc mật khẩu bật nếu `passwordAuthentication || kbdInteractiveAuthentication` đang bật;
  - đổi công tắc thì đặt cả hai trường cùng giá trị;
  - mục kiểm tra `row-password` chỉ đạt khi cả hai đều tắt.

- [ ] **Step 1: Viết test**
```rust
// macOS: chỉ tắt PasswordAuthentication thì vẫn đăng nhập bằng mật khẩu được qua keyboard-interactive.
fn parses_kbd_interactive_from_both_names()          // "kbdinteractiveauthentication no" và "challengeresponseauthentication no" → false
fn linux_password_toggle_does_not_touch_kbd()          // changed_directives khi chỉ đổi password → đúng 1 dòng PasswordAuthentication
fn macos_support_full_with_dropin_readonly_without()   // ReadOnly kèm reason "macos_no_dropin"; ssh_rows.port == false; password_coupled
fn macos_plan_rejects_port_change()                    // Err bắt đầu "UNSUPPORTED: hidden_setting"
// Chạy thật bằng sh với sshd/launchctl giả, file đích và plist nằm trong thư mục tạm:
fn macos_apply_backs_up_writes_and_arms_launchd()      // log launchctl chứa "bootstrap system <plist>"; plist chứa "sleep 60"; file đích = nội dung mới; có .sshbox.bak; output có NOW: và ROLLBACK_AT:
fn macos_apply_invalid_config_restores_and_never_arms()// sshd giả: `-t` → exit 1 ⇒ file đích về như cũ, log launchctl rỗng, output có SSHBOX_SSHD_INVALID
fn macos_rollback_restores_and_removes_plist()         // chạy script hoàn tác: file về như cũ, plist bị xoá, launchctl giả nhận "bootout system/com.sshbox.rollback"
fn macos_apply_refuses_while_rollback_pending()        // launchctl giả: `print` → exit 0 ⇒ output có SSHBOX_ROLLBACK_PENDING, file đích không đổi
```
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces.
- [ ] **Step 4:** Chạy lệnh Unit, lệnh Frontend, và test tích hợp `env -u SSH_AUTH_SOCK CARGO_TARGET_DIR=~/ssh-box/src-tauri/target cargo test --manifest-path src-tauri/Cargo.toml --test vps_sshd -- --ignored --test-threads=1`. File `vps_sshd.rs` có dựng `SshdSettings` nên phải thêm trường mới vào đó. Kỳ vọng: PASS.
- [ ] **Step 5: Thử trên máy ảo macOS** qua `vps_matrix.rs` hoặc app debug với HOME giả. Tắt password khi app đăng nhập bằng key, rồi kiểm tra kết nối mới thành công. Áp dụng xong ngắt app ngay: sau 60 giây cấu hình cũ phải trở lại.
- [ ] **Step 6:** Commit `feat(vps): edit sshd on macOS with a launchd rollback timer`.

---

### Task 6: macOS — Users & Keys chỉ xem (spec §6.1)

**Files:** Modify `src-tauri/src/vps_macos.rs`, `src-tauri/src/vps_config.rs` (`get_access_keys_snapshot`; các hàm ghi user/key trả `UNSUPPORTED: read_only_os` khi `os_family != Linux`), `src-tauri/src/vps_support.rs`.

**Interfaces — Produces:** `pub const MACOS_ACCOUNTS_READ_SCRIPT: &str`. Output cùng định dạng với `ACCOUNTS_READ_SCRIPT`, gồm các phần `---SUDOERS---`, `---ACCOUNT---`, `PASSWD:`, `GROUPS:`, `SHADOW:`, `---AUTHORIZED_KEYS---`, để dùng lại `parse_access_keys_snapshot`.
- `PASSWD:` được dựng từ `dscl . -read /Users/<u> UniqueID PrimaryGroupID NFSHomeDirectory UserShell` theo dạng `name:*:uid:gid::home:shell`.
- Lấy các user có UID ≥ 501, cộng thêm `root`. Bỏ những tên bắt đầu bằng `_`.
- Để trống `SHADOW:`.
- **`compute_support` cho macOS:** `access = ReadOnly` + `read_only_os`.

- [ ] **Step 1: Viết test**
```rust
fn macos_accounts_parse_admin_as_sudo()        // output mẫu: GROUPS chứa "admin", SUDOERS có "%admin ALL = (ALL) ALL" → has_sudo; user thường → !has_sudo; key được parse
fn macos_access_writes_are_refused()           // create_linux_user / set_user_lock / authorize_ssh_key với probe macOS → Err "UNSUPPORTED: read_only_os"
fn macos_support_access_read_only()
```
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL. Nếu `parse_access_keys_snapshot` chưa hiểu dòng `%admin` trong sudoers thì sửa tối thiểu cho nó hiểu, kèm test.
- [ ] **Step 3:** Viết code. Chạy lệnh Unit. Kỳ vọng: PASS.
- [ ] **Step 4:** Thử trên máy ảo macOS: tab Users & Keys hiện danh sách user, có notice, không có nút sửa.
- [ ] **Step 5:** Commit `feat(vps): show macOS users and keys read-only`.

---

### Task 7: Windows — xem SSH và Users & Keys (spec §6.2)

**Files:** Modify `src-tauri/src/vps_windows.rs`, `src-tauri/src/vps_config.rs` (rẽ nhánh `get_sshd_view`, `get_access_keys_snapshot`, `LinuxAccountItem`), `src-tauri/src/vps_support.rs`, `src/types.ts`, `src/components/vps/AccessKeysStudio.tsx`, `src/components/vps/SshHardeningStudio.tsx`.

**Interfaces — Produces:**
```rust
pub const WINDOWS_SSHD_CONFIG: &str = r"C:\ProgramData\ssh\sshd_config";
pub fn windows_sshd_view_script(spec: &str) -> String; // in 4 phần cách nhau bởi SSHD_PART: sshd.exe -T -C <spec> | nội dung WINDOWS_SSHD_CONFIG | (trống) | "active" nếu Get-ScheduledTask sshbox-rollback tồn tại
pub const WINDOWS_ACCOUNTS_READ_SCRIPT: &str;          // in JSON: [{ name, enabled, isAdmin, keysFile, keys: [dòng…] }] (ConvertTo-Json -InputObject @(…) -Depth 4)
pub fn parse_windows_accounts(json: &str) -> Result<AccessKeysSnapshot, String>;
// LinuxAccountItem thêm: #[serde(default)] pub keys_file: String   (TS: keysFile?: string)
// vps_support.rs
pub const WINDOWS_SSH_WRITE_VERIFIED: bool = false; // chỉ Task 10 được đổi thành true
```
- **`get_sshd_view` trên Windows:**
  - Dùng `run_windows_script` và `parse_sshd_view`, sau đó ép `write_target = MainFile`.
  - Quyền `None` thì trả `UNSUPPORTED: windows_not_admin` mà không chạy gì: `sshd -T` cần đọc host key, mà chỉ admin đọc được.
- **`keysFile`:**
  - User admin dùng `C:\ProgramData\ssh\administrators_authorized_keys`, nếu `sshd -T` của user đó cho `authorizedkeysfile` trỏ tới file này.
  - User khác dùng `C:\Users\<name>\.ssh\authorized_keys`.
  - File nào không đọc được thì để `keys` rỗng, không làm hỏng cả script (bọc `try/catch` từng file).
- **Ánh xạ sang `LinuxAccountItem`:** `uid = gid = 0`, `home_dir` và `shell` rỗng, `locked = !enabled`, `groups = ["Administrators"]` nếu là admin, `has_sudo = isAdmin`. Mỗi dòng key được parse bằng `parse_authorized_key_line`.
- **Giao diện:**
  - `AccessKeysStudio` hiện `keysFile` bằng font mono dưới danh sách key, khi giá trị không rỗng.
  - `SshHardeningStudio` không gọi `vps_get_sshd_settings` khi `probe.support.ssh.reason === "windows_not_admin"`, chỉ hiện notice. Nếu gọi thì sẽ hiện thêm một hộp lỗi trùng nội dung.
- **`compute_support` cho Windows:** `access = ReadOnly` + `read_only_os`. `ssh = ReadOnly`, với lý do theo thứ tự ưu tiên `windows_not_admin` → `windows_no_sshd` → `windows_ssh_read_only`. `ssh_rows`: `permit_root_login = false`, `allow_deny_users = false`.

- [ ] **Step 1: Viết test**
```rust
fn windows_sshd_view_parses_and_forces_main_file()     // raw mẫu có "Match Group administrators" → settings đúng, write_target MainFile
fn windows_view_non_admin_is_refused_without_running() // probe privilege None → Err "UNSUPPORTED: windows_not_admin"
fn windows_accounts_map_admin_keys_file_and_disabled() // JSON 2 user (admin có 1 key, std bị tắt) → has_sudo, keys_file, locked
fn windows_accounts_accept_single_object_json()        // PS 5.1 bỏ ngoặc mảng khi chỉ có 1 phần tử → vẫn parse được
fn windows_support_reasons_in_priority_order()
```
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code. Chạy lệnh Unit và lệnh Frontend. Kỳ vọng: PASS.
- [ ] **Step 4:** Thử trên máy ảo Windows bằng user `sshbox` và `sshbox-std`. Kỳ vọng: đúng tab, đúng notice, danh sách user và key khớp với máy.
- [ ] **Step 5:** Commit `feat(vps): read Windows sshd settings and accounts`.

---

### Task 8: Windows — xem Firewall và luật "block thắng" (spec §6.2)

**Files:** Modify `src-tauri/src/vps_windows.rs`, `src-tauri/src/vps_config.rs` (`get_firewall_snapshot` rẽ nhánh, `engine_name`), `src-tauri/src/vps_guard.rs`, `src-tauri/src/vps_support.rs`.

**Interfaces — Produces:**
```rust
pub const WINDOWS_RULE_GROUP: &str = "ssh-box";
pub fn windows_firewall_read_script() -> String;   // JSON: { profiles:[{name,enabled,defaultInbound}], active:[name], rules:[{displayName,group,action,localPort,protocol,remoteAddress}], sockets:[{protocol,addr,port,pid,process}] }
pub fn parse_windows_firewall(json: &str) -> Result<FirewallSnapshot, String>;
pub fn compute_socket_exposure_windows(socket: &ListeningSocketItem, enabled: bool, rules: &[FirewallRuleItem]) -> String; // exposed|restricted|blocked|loopback|unknown
// vps_guard.rs
pub fn check_firewall_delete_windows(ctx: &ConnectionContext, rules: &[FirewallRuleItem], target: &FirewallRuleItem) -> Result<(), String>;
```
- **Script đọc:**
  - Chỉ lấy rule inbound đang bật có lọc theo cổng.
  - Dùng `Get-NetFirewallPortFilter -All` và `Get-NetFirewallAddressFilter -All` một lần, rồi ghép theo `InstanceID`. Gọi từng rule một thì quá chậm trên PS 5.1.
  - Socket lấy từ `Get-NetTCPConnection -State Listen` và `Get-NetUDPEndpoint`, tên tiến trình lấy từ `Get-Process`.
- **Ánh xạ sang `FirewallRuleItem`:**
  - `action`: `Allow` → `ALLOW`, `Block` → `DENY`.
  - `protocol`: `TCP` → `tcp`, `UDP` → `udp`, giá trị khác → `any`.
  - `from_source`: `Any` → `Anywhere`.
  - `comment = group`, `raw_spec = displayName`.
  - Rule có nguồn `Any` sinh hai mục `v6 = false` / `true` cùng `raw_spec`, giống cách đang làm với firewalld.
- **`FirewallSnapshot`:** `enabled` = có ít nhất một profile đang dùng được bật. `default_incoming` = `deny` nếu `defaultInbound` của profile đang dùng là `Block`. `ipv6_filtered = true`.
- **`compute_socket_exposure_windows`:**
  - Không public thì `loopback`. Firewall tắt thì `exposed`.
  - Có rule block khớp cổng với nguồn mọi nơi → `blocked`.
  - Có rule allow khớp cổng với nguồn mọi nơi, và không có rule block khớp → `exposed`.
  - Chỉ có allow theo nguồn cụ thể → `restricted`.
  - Không có rule theo cổng nào khớp → `unknown`.
- **`check_firewall_delete_windows`:** giống `check_firewall_delete`, nhưng một rule allow chỉ được tính là "còn đường vào" khi không có rule block nào còn lại cũng khớp `server_port` từ `client_ip`.
- **`compute_support` cho Windows:**
  - `firewall`: `ReadOnly` (Task 9 mới mở ghi) nếu có `has_net_security`, ngược lại `Hidden`.
  - Quyền `None` thì `ReadOnly` + `windows_not_admin`.

- [ ] **Step 1: Viết test**
```rust
fn windows_firewall_parses_rules_profiles_and_sockets()   // JSON mẫu → engine "windows", rule Any thành 2 mục, comment "ssh-box"
fn windows_firewall_parses_unicode_display_name()         // displayName "Cho phép Đạt" còn nguyên
// ufw: rule khớp đầu tiên thắng. Windows: block luôn thắng.
fn allow_then_block_same_port_differs_between_ufw_and_windows() // compute_socket_exposure → "exposed"; compute_socket_exposure_windows → "blocked"
fn port_without_port_rule_is_unknown_on_windows()
fn windows_delete_ignores_allow_shadowed_by_block()      // rules: allowA (của tôi), allowB (của tôi), blockC khớp tôi; xoá allowA → Err "LOCKOUT_BLOCKED: only_ssh_rule"; với check_firewall_delete (Linux) → Ok
```
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code. Chạy lệnh Unit. Kỳ vọng: PASS.
- [ ] **Step 4:** Thử trên máy ảo Windows. Kỳ vọng: rule `OpenSSH-Server-In-TCP` hiện ra, cổng 22 ở trạng thái `exposed`, thời gian đọc dưới 5 giây.
- [ ] **Step 5:** Commit `feat(vps): read Windows Defender Firewall with block-wins exposure`.

---

### Task 9: Windows — ghi Firewall (spec §6.2)

**Files:** Modify `src-tauri/src/vps_windows.rs`; `src-tauri/src/vps_config.rs` (`add_firewall_rule`, `delete_firewall_rule`, `set_firewall_state`, `writable_engine`); `src-tauri/src/vps_support.rs`; `src/components/vps/FirewallStudio.tsx`.

**Interfaces — Produces:**
```rust
pub fn build_windows_add_rule_script(action: &str, source: &str, port: &str, proto: &str) -> Result<String, String>;
pub fn build_windows_delete_rule_script(rule: &FirewallRuleItem) -> Result<String, String>; // comment != "ssh-box" → UNSUPPORTED: windows_rule_not_managed
pub fn build_windows_firewall_state_script(enable: bool, ensure_port: Option<u16>) -> String;
```
- **Kiểm tra đầu vào** bằng `validate_rule_args` có sẵn.
- **Thêm rule:** `New-NetFirewallRule -DisplayName <ps_quote("ssh-box: <ACTION> <port>/<proto> from <source>")> -Group 'ssh-box' -Direction Inbound -Action Allow|Block -RemoteAddress <source|Any>`. Có cổng mà `proto = any` thì tạo hai rule (TCP và UDP) cùng `DisplayName`.
- **Xoá rule:** `Get-NetFirewallRule -Group 'ssh-box' -DisplayName <dn> | Remove-NetFirewallRule`. Không tìm thấy rule nào thì `throw`.
- **Bật/tắt:** `Set-NetFirewallProfile -Profile Domain,Private,Public -Enabled True|False`. Khi bật mà có `ensure_port`, tạo rule allow `ensure_port`/TCP từ mọi nơi trong nhóm `ssh-box` **trước khi** bật.
- **Sau mỗi lần ghi,** cùng script đó in lại snapshot (`windows_firewall_read_script`).
- **Bộ chặn:** `check_firewall_add` dùng chung với Linux. Engine `Windows` thì xoá dùng `check_firewall_delete_windows`. `ssh_allow_needed` dùng chung.
- **Giao diện:** với engine `windows`, nút xoá chỉ hiện ở rule có `comment === "ssh-box"`.
- **`compute_support`:** `firewall = Full` khi quyền là `Admin` và có `has_net_security`.

- [ ] **Step 1: Viết test**
```rust
fn windows_add_rule_quotes_and_groups()            // script chứa "-Group 'ssh-box'", "-Action Block", nguồn đã quote; port "22;calc" → Err (validate_rule_args)
fn windows_add_any_proto_creates_tcp_and_udp()
fn windows_delete_refuses_unmanaged_rule()         // comment "" → Err "UNSUPPORTED: windows_rule_not_managed"
fn windows_enable_adds_ssh_allow_before_enabling() // vị trí "New-NetFirewallRule" < vị trí "Set-NetFirewallProfile"
fn windows_block_covering_me_is_blocked()          // add DENY Anywhere cổng server_port → Err "LOCKOUT_BLOCKED: closes_ssh_for_everyone"
```
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code. Chạy lệnh Unit và lệnh Frontend. Kỳ vọng: PASS.
- [ ] **Step 4:** Thử trên máy ảo Windows: thêm một rule rồi xoá nó; thử xoá rule `OpenSSH-Server-In-TCP` (phải bị từ chối); bật firewall rồi khởi động lại máy, kiểm tra SSH vẫn vào được.
- [ ] **Step 5:** Commit `feat(vps): edit Windows Firewall rules created by ssh-box`.

---

### Task 10: Windows — ghi SSH (spec §6.2; chỉ làm khi Task 4 kết luận được phép)

**Files:** Modify `src-tauri/src/vps_windows.rs`; `src-tauri/src/vps_config.rs` (rẽ nhánh apply / finish / revert; `render_sshd_config_preserving`); `src-tauri/src/vps_guard.rs`; `src-tauri/src/vps_support.rs`.

**Interfaces — Produces:**
```rust
pub const WINDOWS_ROLLBACK_TASK: &str = "sshbox-rollback";
pub const WINDOWS_ROLLBACK_SCRIPT: &str = r"C:\ProgramData\ssh\sshbox-rollback.ps1";
// vps_support.rs: WINDOWS_SSH_WRITE_VERIFIED (Task 7) đổi thành true
pub fn render_sshd_config_preserving(base_raw: &str, managed: &[(&'static str, Option<String>)]) -> String; // bỏ BOM → render_sshd_config → gắn lại BOM; base có "\r\n" thì đổi mọi "\n" thành "\r\n"
pub fn build_sshd_apply_script_windows(new_port: Option<u16>, port: u16) -> String;
pub fn build_sshd_rollback_script_windows(new_port: Option<u16>) -> String;
// vps_guard.rs
pub fn check_sshd_auth(ctx: &ConnectionContext, next: &SshdSettings) -> Result<(), String>; // both_auth_off, no_sessions, password_in_use, pubkey_in_use; check_sshd gọi nó trước
```
- **Script apply của Windows.** Nội dung file mới nằm trong `$SSHBOX_SECRET`. Các bước:
  1. `Get-ScheduledTask sshbox-rollback` có tồn tại → in `SSHBOX_ROLLBACK_PENDING`, `throw`.
  2. Sao lưu sang `<file>.sshbox.bak`.
  3. Ghi file bằng `[IO.File]::WriteAllText`. Nội dung đã gồm cả BOM nếu có, vì Rust đã gắn lại từ trước.
  4. Chạy `sshd.exe -t`; `$LASTEXITCODE` khác 0 thì khôi phục, in `SSHBOX_SSHD_INVALID`, `throw`.
  5. Ghi `WINDOWS_ROLLBACK_SCRIPT`.
  6. `Register-ScheduledTask sshbox-rollback`: chạy một lần sau 60 giây, `-User SYSTEM -RunLevel Highest`, chạy `powershell.exe -NoProfile -ExecutionPolicy Bypass -File <script>`.
  7. In `NOW:` / `ROLLBACK_AT:` (`[DateTimeOffset]::UtcNow.ToUnixTimeSeconds()`).
  8. Có port mới thì thêm rule allow port mới/TCP vào nhóm `ssh-box`.
  9. `Restart-Service sshd`. Nếu Task 4 điểm 1 cho thấy lệnh này làm rớt phiên, thay bằng một task `sshbox-restart` chạy sau 2 giây, và bỏ bước 10.
  10. Kiểm tra `Get-NetTCPConnection -State Listen -LocalPort <port>`.
- **Script hoàn tác:** khôi phục bản sao lưu (hoặc xoá file nếu có dấu `.sshbox.absent`), `Restart-Service sshd`, xoá rule port mới, `Unregister-ScheduledTask sshbox-rollback -Confirm:$false`, xoá chính file script.
- **`finish` / cleanup:** gỡ task. Nếu không còn file script thì in `SSHBOX_ALREADY_ROLLED_BACK`, đúng như luồng Linux.
- **Bộ chặn trên Windows:** gọi `check_sshd_auth` và `check_hidden_rows`, không gọi `check_sshd`, vì `AllowUsers` và root đang bị ẩn.
- **`compute_support`:**
  - `ssh = Full` khi `WINDOWS_SSH_WRITE_VERIFIED`, quyền `Admin`, `caps.sshd_running` và `caps.has_scheduled_tasks`.
  - `password_coupled` cho Windows lấy theo kết luận điểm 7 của Task 4.

- [ ] **Step 1: Viết test**
```rust
// Notepad lưu file kèm BOM và CRLF; directive đặt sau "Match Group administrators" chỉ áp dụng cho admin.
fn windows_render_keeps_bom_and_crlf_and_inserts_before_match()
fn windows_apply_checks_exit_code_and_restores_on_invalid() // script chứa "$LASTEXITCODE", "SSHBOX_SSHD_INVALID", và lệnh khôi phục nằm trước throw
fn windows_apply_arms_task_before_restart()                // vị trí "Register-ScheduledTask" < vị trí "Restart-Service"
fn windows_rollback_unregisters_task_and_removes_port_rule()
fn windows_guard_ignores_hidden_allow_users()              // AllowUsers hiện tại "DOMAIN\\bob", desired chỉ đổi MaxAuthTries → Ok
fn linux_check_sshd_unchanged_after_split()                // các test check_sshd cũ vẫn qua
```
- [ ] **Step 2:** Chạy lệnh Unit. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code. Chạy lệnh Unit. Kỳ vọng: PASS.
- [ ] **Step 4:** Thử trên máy ảo Windows:
  - đổi `MaxAuthTries` → kết nối thử thành công và thay đổi được giữ;
  - đổi port sang một port bị chặn → tự hoàn tác;
  - áp dụng rồi ngắt app → sau 60 giây cấu hình cũ trở lại.
- [ ] **Step 5:** Commit `feat(vps): edit sshd on Windows with a scheduled-task rollback`.

Nếu Task 4 kết luận không được phép: bỏ qua task này, giữ `WINDOWS_SSH_WRITE_VERIFIED = false` (SSH Windows ở mức `ReadOnly` + `windows_ssh_read_only`), và ghi lý do vào mô tả MR.

---

### Task 11: Ma trận kiểm thử (spec §7.3, §7.4)

**Files:** Modify `src-tauri/tests/vps_matrix.rs`: thêm nhánh `MX_DISTRO=macos` và `MX_DISTRO=windows`, chạy các kịch bản của spec §7.3 qua chính các hàm công khai của `vps_config`.

- [ ] **Step 1:** Thêm kịch bản cho macOS và Windows. Mỗi kịch bản là một hàm, đặt tên theo dòng của bảng §7.3. Kịch bản Windows bỏ qua các dòng ghi SSH khi `WINDOWS_SSH_WRITE_VERIFIED == false`.
- [ ] **Step 2:** Chạy trên máy ảo macOS, rồi Windows (cả `sshbox` lẫn `sshbox-std`; cả `DefaultShell` cmd lẫn PowerShell). Kỳ vọng: mọi kịch bản PASS.
- [ ] **Step 3:** Chạy lại ma trận Linux trên `u24`, `d13`, `r9`. Kỳ vọng: 7/7 PASS trên mỗi máy, như ở MR !50.
- [ ] **Step 4:** Kiểm tra app macOS (bản debug, HOME giả) với cả 3 hệ máy: theme sáng và tối, VI và EN, rộng 360px.
- [ ] **Step 5:** Chạy lần cuối lệnh Unit, lệnh Frontend, và test tích hợp `vps_sshd` (lệnh ở Task 5 Step 4). Kỳ vọng: PASS.
- [ ] **Step 6:** Viết phần kết quả cho mô tả MR:
  - bảng đạt/không đạt;
  - mức cuối cùng của từng ô trong bảng §4;
  - những bản hệ điều hành đã thử thật;
  - phần còn mở: app trên Windows, iOS, Android.
  
  Commit `test(vps): run Server Config matrix on macOS and Windows VMs`. Chỉ tạo MR khi người dùng đồng ý.
