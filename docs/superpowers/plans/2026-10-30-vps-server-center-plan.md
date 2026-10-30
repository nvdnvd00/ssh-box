# Cấu hình VPS (VPS Server Center) — Đợt 1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Làm đợt 1 của Cấu hình VPS (khung chung + SSH + Firewall + Users & Keys) theo spec bản sửa đổi 2, chống khoá bằng cách mở một kết nối SSH mới để xác minh.

**Architecture:** Backend giữ `src-tauri/src/vps_config.rs` (sau khi bỏ code đợt 2) và thêm `src-tauri/src/vps_guard.rs` gồm các hàm thuần của bộ chặn tự khoá. Mọi lệnh cần quyền root đi qua một bộ chạy duy nhất. Bộ chạy gửi mật khẩu sudo và dữ liệu bí mật qua stdin bằng `SshPool::exec_bytes_on_host` có sẵn. Frontend gồm `VpsConfigWorkspace.tsx` và 3 studio, tất cả ghép từ `src/components/vps/shared/`.

**Tech Stack:** Rust (Tauri v2, `russh`, `serde`, `tokio`), React 19 + TypeScript, Tailwind CSS v4. Không thêm dependency mới, kể cả dev-dependency.

**Spec:** `docs/superpowers/specs/2026-10-30-vps-server-center-design.md`. Người thực hiện đọc cả spec lẫn plan này.

## Global Constraints

- Thứ tự: làm tuần tự từ Task 0 đến Task 14, không chạy song song (spec §7). Mỗi task tối đa khoảng 400 dòng code; nếu vượt thì tách tiếp.
- App phải chạy được trên macOS, iOS, Windows, Linux, Android. Chỉ dùng HTML/CSS mà cả WebKit lẫn Chromium hỗ trợ mặc định. Màn hình rộng tối thiểu 360px. Không có thông tin chỉ hiện qua `title` hay `group-hover`.
- Server hỗ trợ: Ubuntu 22.04/24.04, Debian 12/13, Rocky/Alma 9. Host `local` hoặc `uname -s` khác `Linux` → `UNSUPPORTED`, UI hiện "Chỉ hỗ trợ máy chủ Linux" / "Linux servers only".
- Lỗi backend có dạng `MÃ: chi tiết`, kiểu `Result<T, String>`. Các mã: `SUDO_REQUIRED`, `SUDO_DENIED`, `LOCKOUT_BLOCKED`, `COMMAND_FAILED`, `VERIFY_FAILED`, `UNSUPPORTED`.
- Tên tính năng chỉ có một: `t.navVpsConfig`, VI "Cấu hình VPS", EN "Server Config". Mọi chuỗi lấy từ `getI18n(lang)`; không có `isVi ? … : …` trong JSX.
- Không dùng các từ "Fortress", "Radar", "Vault", "SOC", "SHIELDS ACTIVE". Không có đồng hồ điểm, không có preset "1 click". Không đặt `select-none` lên vùng nội dung. Không dùng thuộc tính `switch`.
- Đường dẫn và tên cố định:
  - drop-in: `/etc/ssh/sshd_config.d/00-sshbox.conf`;
  - bản sao lưu: `<file>.sshbox.bak`, kèm dấu `<file>.sshbox.absent` khi file chưa từng tồn tại;
  - hẹn giờ hoàn tác: unit `sshbox-rollback`, `--on-active=60`, script `/etc/ssh/sshbox-rollback.sh`;
  - sudoers: `/etc/sudoers.d/90-sshbox-<user>`.
- Mật khẩu (sudo hay mật khẩu user mới) chỉ đi qua stdin, không bao giờ nằm trong chuỗi lệnh. Không lấy mật khẩu từ vault để thử sudo. Mật khẩu sudo chỉ giữ trong state của `VpsConfigWorkspace`.
- Firewall chỉ được ghi qua `ufw` hoặc `firewalld`. iptables thuần chỉ được xem.
- Test không bao giờ dùng server thật, không dùng `~/.ssh` thật, không dùng `sudo` thật. Chỉ dùng binary giả trong thư mục tạm, hoặc `/usr/sbin/sshd` chạy trên `127.0.0.1`, cổng 22900–22999.
- Giữa Task 2 và Task 13, studio cũ có thể gọi tới lệnh Tauri đã bị đổi tên. Điều này chấp nhận được vì nhánh chưa merge. Mỗi studio phải chạy đúng khi xong task UI của nó (Task 8, 11, 13).
- Commit sau mỗi task theo kiểu `feat(vps): …`. Chỉ push khi người dùng yêu cầu.

## Review Focus

1. Lệnh `sshd -T` in `permitrootlogin without-password` (tên cũ của `prohibit-password`) và in mỗi user của `allowusers` trên một dòng riêng. Parser phải chuẩn hoá tên và gộp đủ các dòng. → Task 6, `parses_sshd_t_alias_and_repeated_allowusers`.
2. Trước khi áp dụng chưa có file drop-in, tức là không có gì để sao lưu. Khi hoàn tác, phải xoá file vừa tạo, không được để lại file hay báo lỗi. → Task 7, `rollback_removes_dropin_that_did_not_exist`.
3. Kết nối thử sang port mới gặp host key khác. Khi đó `test_connection` đã ghi key lạ vào `known_hosts` theo kiểu tin lần đầu (TOFU), nên phải xoá dòng đó đi. → Task 7, `verify_fails_on_different_host_key_and_forgets_it`.
4. Mật khẩu chứa ký tự xuống dòng sẽ làm lệch giao thức stdin (dòng đầu là mật khẩu sudo). Phải từ chối trước khi gửi. → Task 1, `newline_in_password_is_rejected_before_sending`; Task 12, `user_password_with_newline_is_rejected`.
5. App kết nối qua IPv6, hoặc rule firewall có nguồn là CIDR IPv6. Bộ chặn vẫn phải nhận ra rule đó có bao gồm IP của tôi. → Task 2, `probe_parses_ipv6_ssh_connection`; Task 3, `firewall_deny_covering_client_ipv6_is_blocked`.

---

## Cấu trúc file

| File | Trách nhiệm |
|---|---|
| `src-tauri/src/vps_config.rs` | Bộ chạy lệnh, probe, đọc/ghi SSH, Firewall, Users & Keys (sửa) |
| `src-tauri/src/vps_guard.rs` | Bộ chặn tự khoá: hàm thuần, không I/O (mới) |
| `src-tauri/src/ssh_pool.rs` | Ghi lại cách xác thực đã thành công: `AuthUsed` (sửa) |
| `src-tauri/src/lib.rs` | Đăng ký lệnh Tauri `vps_*` (sửa) |
| `src-tauri/tests/vps_sshd.rs` | Test tích hợp với sshd tạm, `#[ignore]` (mới) |
| `src/components/vps/shared/*` | `Switch`, `SettingRow`, `StatusBadge`, `ConfirmButton`, `ChangePreviewDialog`, `useVpsResource`, `errors.ts` (mới) |
| `src/components/VpsConfigWorkspace.tsx`, `src/components/vps/{SshHardening,Firewall,AccessKeys}Studio.tsx` | Viết lại theo khuôn §3 |
| `src/index.css`, `src/i18n.ts`, `src/types.ts`, `src/App.tsx`, `src/components/DashboardWorkspace.tsx` | Token màu, chuỗi, kiểu dữ liệu, điểm vào (sửa) |

**Lệnh kiểm tra dùng chung**
- Unit: `cargo test --manifest-path src-tauri/Cargo.toml vps`
- Tích hợp: `env -u SSH_AUTH_SOCK cargo test --manifest-path src-tauri/Cargo.toml --test vps_sshd -- --ignored --test-threads=1`
- Frontend: `npm run build`. Xem trước trong browser pane theo cách trong memory dự án: entry `vite` tạm trỏ vào worktree, cùng trang HTML stub `window.__TAURI_INTERNALS__` trả dữ liệu mẫu cho các lệnh `vps_*`. Kiểm ở 360×800 và 1280×800, theme `zinc-dark` và `zinc-light`, cả VI lẫn EN. Xong thì xoá entry và stub tạm.

---

### Task 0: Tách code đợt 2 sang nhánh riêng

**Files:**
- Delete: `src/components/vps/Fail2banStudio.tsx`, `KernelTuningStudio.tsx`, `SystemOpsStudio.tsx`
- Modify: `src-tauri/src/vps_config.rs`. Xoá:
  - các struct `Fail2banJailItem` … `Fail2banSnapshot` (138–200) và `KernelSysctlSnapshot` … `SystemOpsSnapshot` (233–377);
  - các hàm 1895–2396 (fail2ban, auth log) và 2990–3956 (kernel, swap, systemd, cron, host identity);
  - 5 test `parses_kernel_sysctl_…`, `renders_sysctl_conf_…`, `parses_system_ops_…`, `renders_crontab_…`, `builds_swapfile_…`.
- Modify: `src-tauri/src/lib.rs` (các lệnh `vps_*fail2ban*`, `vps_*kernel*`, `vps_*system_ops*`, `vps_configure_swapfile`, `vps_systemd_*`, `vps_save_cron_jobs`, `vps_update_host_identity`, cùng dòng đăng ký của chúng); `src/types.ts` (243–~395); `src/components/VpsConfigWorkspace.tsx` (tab `fail2ban`, `kernel`, `system`).

- [ ] **Step 1:** `git branch feat/vps-server-center-phase2 HEAD`. Nhánh này giữ toàn bộ code hiện tại. Chỉ chạy `git push origin feat/vps-server-center-phase2` khi người dùng đồng ý.
- [ ] **Step 2:** Xoá các phần liệt kê ở trên. Xoá luôn hàm phụ chỉ code đợt 2 dùng; dựa vào cảnh báo `never used` của compiler để tìm.
- [ ] **Step 3:** Chạy `cargo test --manifest-path src-tauri/Cargo.toml`. Kỳ vọng: PASS, số test = số cũ − 5, và `cargo build 2>&1 | grep -c 'never used'` không tăng so với trước Task 0.
- [ ] **Step 4:** Chạy `npm run build`. Kỳ vọng: PASS. Chạy `grep -rnE 'fail2ban|sysctl|swapfile|crontab|systemd_action|host_identity' src src-tauri/src` → không còn kết quả nào trong các file `vps`.
- [ ] **Step 5:** Commit `refactor(vps): move phase-2 studios to feat/vps-server-center-phase2`.

---

### Task 1: Bộ chạy lệnh đặc quyền (§5)

**Files:** Modify `src-tauri/src/vps_config.rs:1183-1284` (thay `resolve_sudo_password`, `build_privileged_command`, `run_privileged_script`); test trong `mod tests` của cùng file.

**Interfaces — Produces:**
- `pub const ERR_SUDO_REQUIRED/ERR_SUDO_DENIED/ERR_LOCKOUT_BLOCKED/ERR_COMMAND_FAILED/ERR_VERIFY_FAILED/ERR_UNSUPPORTED: &str` (giá trị là tên mã) và `pub fn vps_err(code: &str, detail: impl std::fmt::Display) -> String` → `"{code}: {detail}"`.
- `pub fn privileged_stdin(sudo_password: Option<&str>, secret: Option<&str>) -> Result<Vec<u8>, String>` → `"{pw}\n{secret}"`. Nếu `pw` chứa `\n` hoặc `\r` → `Err(vps_err(ERR_SUDO_DENIED, …))`.
- `pub fn build_privileged_command(script: &str) -> String`: không còn tham số mật khẩu.
- `pub fn classify_privileged_output(raw: &str) -> Result<String, String>`: phần phân loại tách ra từ `run_privileged_script` cũ.
  - `SSHBOX_ERR_SUDO_PASSWORD_REQUIRED` → `SUDO_REQUIRED`.
  - Có `SSHBOX_EXIT_CODE:` kèm "sorry, try again" / "incorrect password" → `SUDO_DENIED`.
  - Có `SSHBOX_EXIT_CODE:` trong các trường hợp còn lại → `COMMAND_FAILED: <output>`.
- `pub async fn run_privileged_script(host_id: &str, pool: &SshPool, sudo_password: Option<String>, script: &str, secret: Option<&str>) -> Result<String, String>`.
  - Tìm `HostEntry` bằng `host_store::load_all_hosts()`; `host_id == "local"` → `UNSUPPORTED`.
  - Gọi `pool.exec_bytes_on_host(&host, &cmd, Some(&stdin))`.
  - Các script có ghi thì tự bắt đầu bằng `set -eu`; script chỉ đọc thì không.

Khung lệnh (`build_privileged_command`). Đây là thuật toán mà chữ ký hàm không tự quyết được:
```sh
LC_ALL=C
PATH="$PATH:/usr/local/sbin:/usr/sbin:/sbin"          # nối SAU: test thay được sudo, id
IFS= read -r SSHBOX_PW || SSHBOX_PW=
SSHBOX_SECRET_FILE=$(mktemp); chmod 600 "$SSHBOX_SECRET_FILE"; cat > "$SSHBOX_SECRET_FILE"
trap 'rm -f "$SSHBOX_SECRET_FILE"' EXIT
body() { printf 'SSHBOX_SECRET_FILE=%s\nexport PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH"\n' "$SSHBOX_SECRET_FILE"
  cat <<'SSHBOX_EOF_9912'
{script}
SSHBOX_EOF_9912
}
if [ "$(id -u)" = 0 ]; then body | sh -s 2>&1; rc=$?
elif sudo -n true 2>/dev/null; then body | sudo -n sh -s 2>&1; rc=$?
elif [ -n "$SSHBOX_PW" ]; then { printf '%s\n' "$SSHBOX_PW"; body; } | sudo -S -p '' sh -s 2>&1; rc=$?
else echo SSHBOX_ERR_SUDO_PASSWORD_REQUIRED; rc=126; fi
[ "$rc" -eq 0 ] || echo "SSHBOX_EXIT_CODE:$rc"
```
Không còn dòng `:` cuối script.

**Công cụ test** (`#[cfg(unix)]`):
- `fn fake_bin(dir: &Path, name: &str, body: &str)`: ghi một script `#!/bin/sh` có quyền chạy.
- `fn run_local(cmd: &str, stdin: &[u8], path_prefix: &Path, envs: &[(&str, &str)], cwd: &Path) -> String`: chạy `sh -c cmd` với `PATH=<prefix>:/usr/bin:/bin`.
- `id` giả: `echo 1000`.
- `sudo` giả: `case "$1" in -n) exit 1;; esac; IFS= read -r pw; printf '%s' "$pw" > "$FAKE_SUDO_LOG"; [ "$pw" = "$FAKE_SUDO_OK" ] || { echo 'Sorry, try again.'; exit 1; }; exec sh -s`.
- Thư mục tạm: `std::env::temp_dir().join(format!("sshbox-test-{}-{}", std::process::id(), n))`.

- [ ] **Step 1: Viết test.** Mỗi test có một dòng comment nói nó bảo vệ điều gì.
```rust
// Bản cũ để mật khẩu nằm trong chuỗi lệnh: ký tự ' hoặc ; làm vỡ lệnh, hoặc chạy lệnh lạ.
fn sudo_password_reaches_sudo_intact_and_runs_nothing_else()
//   với từng pw trong ["p@ss word", "a'b", "a\"b", "abc$HOME", "a;touch INJECTED", "a`id`b", "a\\b"]:
//   assert_eq!(fs::read_to_string(log), pw); assert!(out.contains("SCRIPT_RAN"));
//   assert!(!cwd.join("INJECTED").exists()); assert!(!build_privileged_command("echo SCRIPT_RAN").contains(pw));
fn secret_reaches_script_and_temp_file_is_removed()
//   secret = "u:p'a$s;`x`", script `cat "$SSHBOX_SECRET_FILE"`, TMPDIR=<tmp>/t
//   assert!(out.contains(secret)); assert_eq!(fs::read_dir(tmp_t).count(), 0);
fn failing_command_mid_script_returns_command_failed()
//   "set -eu\nfalse\necho AFTER" → Err bắt đầu "COMMAND_FAILED" và không chứa "AFTER"
//   "true\nfalse" → Err  (bản cũ trả Ok vì có dòng `:` cuối)
fn missing_and_wrong_sudo_password_map_to_codes()
//   không có pw → Err bắt đầu "SUDO_REQUIRED"; pw sai → Err bắt đầu "SUDO_DENIED"
fn newline_in_password_is_rejected_before_sending()
//   privileged_stdin(Some("a\nb"), None) → Err bắt đầu "SUDO_DENIED"
```
- [ ] **Step 2:** Chạy `cargo test … vps`. Kỳ vọng: FAIL (chưa có hàm mới).
- [ ] **Step 3:** Viết code theo Interfaces và khung lệnh ở trên. Xoá `resolve_sudo_password`: chỉ có `run_privileged_script` và `probe_privilege` gọi nó. Thêm `None` cho tham số `secret` ở mọi nơi đang gọi.
- [ ] **Step 4:** Chạy `cargo test … vps`. Kỳ vọng: PASS.
- [ ] **Step 5: Chứng minh test bắt được lỗi cũ.** Tạm đặt lại khung lệnh cũ (`git show HEAD:src-tauri/src/vps_config.rs`, dòng 1211–1247) vào một hàm test tạm, rồi chạy `failing_command_mid_script_returns_command_failed` với nó. Kỳ vọng: FAIL. Ghi output vào phần thân commit message rồi xoá hàm tạm.
- [ ] **Step 6:** Commit `fix(vps): send sudo password via stdin and stop masking exit codes`.

---

### Task 2: Ngữ cảnh kết nối và probe (§4.1, §5)

**Files:**
- Modify `src-tauri/src/ssh_pool.rs`:
  - `MultiplexedSession` (425) có thêm `pub auth_used: AuthUsed`;
  - `authenticate_session` đổi kiểu trả về thành `Result<Option<AuthUsed>, String>`;
  - `connect_new` trả thêm `AuthUsed`.
- Modify `src-tauri/src/vps_config.rs`: thay `VpsPrivilegeInfo`, `parse_privilege_probe`, `probe_privilege`.
- Modify `src-tauri/src/lib.rs`: đổi `vps_probe_privilege` thành `vps_probe`.
- Modify `src/types.ts`.
- Create `src-tauri/tests/vps_sshd.rs`.

**Interfaces — Produces:**
```rust
// ssh_pool.rs
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum AuthUsed { Password, Key { public_key_body: String } }
impl SshPool {
    pub async fn session_auth(&self, host_id: &str) -> Option<AuthUsed>;
    pub async fn session_fingerprint(&self, host_id: &str) -> Option<String>;
}
// vps_config.rs  (mọi struct: #[serde(rename_all = "camelCase")])
pub enum PrivilegeLevel { Root, SudoNoPassword, SudoPassword, None }
pub enum FirewallEngine { Ufw, Firewalld, Iptables, None }
pub struct ConnectionContext { pub login_user: String, pub auth: AuthUsed, pub client_ip: String, pub server_ip: String, pub server_port: u16 }
pub struct VpsProbe { pub privilege: PrivilegeLevel, pub context: ConnectionContext, pub os_id: String, pub os_version: String,
    pub openssh_version: String, pub has_systemd: bool, pub ssh_socket_active: bool, pub selinux_enforcing: bool, pub firewall_engine: FirewallEngine }
pub const PROBE_SCRIPT: &str;
pub fn parse_probe_output(raw: &str, auth: AuthUsed) -> Result<VpsProbe, String>;
pub async fn probe(host_id: &str, pool: &SshPool, sudo_password: Option<String>) -> Result<VpsProbe, String>;
```
- Lấy `public_key_body`:
  - key từ file: `key_pair.clone_public_key()?.public_key_base64()`;
  - key từ agent: `pubkey.public_key_base64()`.
- `PROBE_SCRIPT` chạy **không qua sudo**. Script đọc dòng đầu stdin làm mật khẩu, nối `/usr/sbin:/sbin` vào cuối `PATH`, rồi in các dòng sau:
  - `UNAME:`, `USER:`, `UID:`;
  - `SUDO:root|nopasswd|password_ok|password_bad|none`. Mật khẩu được thử bằng `printf '%s\n' "$pw" | sudo -S -p '' true`;
  - `OS_ID:`, `OS_VERSION:` (từ `/etc/os-release`);
  - `OPENSSH:` (`ssh -V 2>&1`);
  - `SSH_CONNECTION:$SSH_CONNECTION`;
  - `SYSTEMD:1|0` (`command -v systemctl`);
  - `SSH_SOCKET:` (`systemctl is-active ssh.socket`);
  - `SELINUX:` (`getenforce`);
  - `FW:firewalld|ufw|iptables|none`, theo thứ tự ưu tiên: `systemctl is-active firewalld`, rồi `command -v ufw`, rồi `command -v iptables`.
- `parse_probe_output` xử lý lỗi như sau:
  - `UNAME` khác `Linux` → `UNSUPPORTED`;
  - `SUDO:password_bad` → `SUDO_DENIED`;
  - `SSH_CONNECTION` không đủ 4 trường → `UNSUPPORTED`.
- `probe()` chạy `PROBE_SCRIPT` qua `exec_bytes_on_host`, với stdin lấy từ `privileged_stdin`. Sau đó gọi `pool.session_auth(host_id)`; nếu không có thì trả `COMMAND_FAILED`.

- [ ] **Step 1: Viết unit test**
```rust
fn probe_parses_ubuntu_socket_sudo_password()   // SSH_CONNECTION:203.0.113.7 51000 198.51.100.2 2222
//   privilege == SudoPassword, context.client_ip == "203.0.113.7", server_ip == "198.51.100.2",
//   server_port == 2222, login_user == "deploy", os_version == "24.04", ssh_socket_active, firewall_engine == Ufw
fn probe_parses_ipv6_ssh_connection()            // "2001:db8::7 51000 2001:db8::2 22" → client_ip "2001:db8::7"
fn probe_rejects_non_linux()                     // UNAME:Darwin → Err bắt đầu "UNSUPPORTED"
fn probe_wrong_sudo_password_is_sudo_denied()
fn probe_without_ssh_connection_is_unsupported()
// Bản cũ lồng mật khẩu vào sh -c '…': "a;touch INJECTED" chạy được lệnh lạ.
fn probe_script_passes_password_via_stdin_only() // chạy PROBE_SCRIPT bằng sudo giả của Task 1, SSH_CONNECTION đặt sẵn
//   out chứa "SUDO:password_ok"; log == pw; !cwd.join("INJECTED").exists()
```
- [ ] **Step 2:** Chạy `cargo test … vps`. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces và cập nhật kiểu TS tương ứng trong `src/types.ts`.
- [ ] **Step 4: Dựng công cụ test tích hợp** trong `src-tauri/tests/vps_sshd.rs`:
  - `fn fake_home() -> PathBuf`: đặt `HOME` sang thư mục tạm và xoá `SSH_AUTH_SOCK`, chạy trước khi tạo runtime tokio;
  - `fn start_sshd(ports: &[u16], host_key: Option<&Path>) -> TestSshd { dir, pid, client_key, client_pub_body }`: dựng `sshd_config` theo memory dự án, chạy `/usr/sbin/sshd -f`;
  - `fn host_for(s: &TestSshd, port: u16) -> HostEntry`: `hostname: "127.0.0.1"`, user hiện tại, `identity_file`, `..Default::default()`;
  - `Drop` cho `TestSshd`: kill pid và xoá thư mục tạm.
  
  Test đầu tiên: `session_auth_reports_the_key_that_logged_in`. Gọi `get_or_connect`, rồi kiểm `session_auth == Some(AuthUsed::Key { public_key_body: client_pub_body })`.
- [ ] **Step 5:** Chạy lệnh test tích hợp. Kỳ vọng: PASS.
- [ ] **Step 6:** Commit `feat(vps): probe connection context and record auth method`.

---

### Task 3: Bộ chặn tự khoá (§4.3)

**Files:** Create `src-tauri/src/vps_guard.rs`, thêm `pub mod vps_guard;` vào `lib.rs`. Định nghĩa `SshdSettings` trong `vps_config.rs`.

**Interfaces:**
- Consumes: `ConnectionContext`, `AuthUsed` (Task 2); `FirewallRuleItem`, `LinuxAccountItem` (có sẵn).
- Produces:
```rust
// vps_config.rs
pub struct SshdSettings { pub port: u16, pub permit_root_login: String, pub password_authentication: bool,
    pub pubkey_authentication: bool, pub allow_users: Vec<String>, pub deny_users: Vec<String>, pub max_auth_tries: u32,
    pub max_sessions: u32, pub login_grace_time: u32, pub client_alive_interval: u32, pub client_alive_count_max: u32 }
// vps_guard.rs — mọi hàm trả Err(vps_err(ERR_LOCKOUT_BLOCKED, <lý do tiếng Anh ngắn>))
pub fn check_sshd(ctx: &ConnectionContext, next: &SshdSettings) -> Result<(), String>;
pub fn ssh_allow_needed(ctx: &ConnectionContext, rules: &[FirewallRuleItem]) -> bool;
pub fn check_firewall_delete(ctx: &ConnectionContext, rules: &[FirewallRuleItem], target: &FirewallRuleItem) -> Result<(), String>;
pub fn check_firewall_add(ctx: &ConnectionContext, action: &str, source: &str, port: &str) -> Result<(), String>;
pub enum UserChange { Lock, RemoveSudo }
pub fn check_user_change(ctx: &ConnectionContext, accounts: &[LinuxAccountItem], user: &str, change: UserChange) -> Result<(), String>;
pub fn check_key_revoke(ctx: &ConnectionContext, user: &str, key_body: &str) -> Result<(), String>;
pub fn source_covers(source: &str, ip: &str) -> bool; // "Anywhere"/"any"/"0.0.0.0/0"/"::/0", CIDR v4/v6, IP đơn; dùng std::net
```
- Cách so `AllowUsers`/`DenyUsers`:
  - mẫu có dạng `user[@host]`, phần user so bằng glob `*` `?`;
  - phần `@host` so với `client_ip` bằng glob, hoặc bằng `source_covers` nếu là CIDR.
- Chặn thêm (quyết định của plan, mở rộng dòng `PermitRootLogin no` của spec): `forced-commands-only` khi `login_user = root`, vì kết nối vẫn mở được nhưng không còn chạy lệnh được.

- [ ] **Step 1: Viết test**, mỗi dòng của bảng §4.3 có một test chặn và một test cho phép:

| Dòng §4.3 | Test chặn (Err `LOCKOUT_BLOCKED`) | Test cho phép (Ok) |
|---|---|---|
| Tắt password | `password_off_blocked_when_app_uses_password` | cùng thay đổi, `auth = Key` |
| Tắt pubkey | `pubkey_off_blocked_when_app_uses_key` | `auth = Password` |
| Tắt cả hai | `both_auth_off_always_blocked` | — |
| Root `no` / `forced-commands-only` | `root_login_off_blocked_for_root` | `login_user = "deploy"` |
| Root `prohibit-password` | `root_prohibit_password_blocked_for_root_with_password` | root + Key |
| `AllowUsers`/`DenyUsers` | `allow_users_excluding_me_blocked`, `deny_users_matching_me_blocked`, `allow_users_host_part_excluding_my_ip_blocked` | `["deploy@203.0.113.0/24"]` với `client_ip = 203.0.113.7` |
| Bật firewall | `ssh_allow_needed_when_no_rule_allows_server_port` | đã có `allow 2222/tcp Anywhere` → false |
| Xoá rule | `deleting_only_rule_for_my_ip_and_port_blocked` | còn rule `allow 2222 Anywhere` khác |
| Thêm deny | `deny_covering_client_ip_blocked`, `deny_server_port_from_anywhere_blocked`, `firewall_deny_covering_client_ipv6_is_blocked` (`2001:db8::/32`, client `2001:db8::7`) | deny `198.51.100.0/24` cổng 80 |
| Khoá / bỏ sudo chính mình | `lock_or_unsudo_login_user_blocked` | user khác |
| Bỏ sudo người cuối | `removing_last_sudo_user_blocked` (root không tính) | còn 2 user sudo |
| Thu hồi key đang dùng | `revoking_key_app_uses_blocked` | key khác, hoặc `auth = Password` |

- [ ] **Step 2:** Chạy `cargo test … vps_guard`. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces.
- [ ] **Step 4:** Chạy `cargo test … vps_guard`. Kỳ vọng: PASS.
- [ ] **Step 5:** Commit `feat(vps): add backend lockout guard`.

---

### Task 4: Thành phần UI dùng chung, token màu, chuỗi chung (§3.3, §3.4)

**Files:**
- Create trong `src/components/vps/shared/`: `Switch.tsx`, `SettingRow.tsx`, `StatusBadge.tsx`, `ConfirmButton.tsx`, `ChangePreviewDialog.tsx`, `useVpsResource.ts`, `errors.ts`.
- Modify: `src/index.css` (5 khối theme: `obsidian-dark`, `zinc-dark`, `stone-dark`, `midnight-dark`, `zinc-light` cùng khối `:root` mặc định; và `@theme inline`), `src/i18n.ts`.

**Interfaces — Produces:**
```ts
Switch({ checked, onChange, label, disabled }: { checked: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean })
SettingRow({ label, description, directive, effective, children }: { label: string; description?: string; directive?: string; effective?: string; children: ReactNode })
StatusBadge({ tone, children }: { tone: "ok" | "warn" | "danger" | "neutral"; children: ReactNode })
ConfirmButton({ label, confirmLabel, onConfirm, disabled }: { label: string; confirmLabel: string; onConfirm: () => void | Promise<void>; disabled?: boolean })
ChangePreviewDialog({ open, changes, blockedReason, applying, onCancel, onApply }:
  { open: boolean; changes: { label: string; from: string; to: string }[]; blockedReason?: string | null; applying: boolean; onCancel: () => void; onApply: () => void })
type VpsErrorCode = "SUDO_REQUIRED" | "SUDO_DENIED" | "LOCKOUT_BLOCKED" | "COMMAND_FAILED" | "VERIFY_FAILED" | "UNSUPPORTED";
parseVpsError(err: unknown): { code: VpsErrorCode | null; detail: string }
vpsErrorText(t: AppI18nStrings, err: unknown): string
interface VpsCtx { hostId: string; sudoPassword?: string; requestSudo: () => void; t: AppI18nStrings }
useVpsResource<T>(vps: VpsCtx, load: (sudoPassword?: string) => Promise<T>):
  { data: T | null; loading: boolean; error: string | null; refresh: () => Promise<void>; mutate: (run: (sudoPassword?: string) => Promise<T>) => Promise<boolean> }
```
- `Switch`: chép đúng class của toggle `role="switch"` ở `SettingsWorkspace.tsx:305-330`, dùng `aria-label={label}`. Không sửa `SettingsWorkspace.tsx` (ngoài phạm vi; ghi lại để dọn sau).
- `SettingRow`: `flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between` (`sm` = 640px). Tên directive dùng `font-mono text-xs text-muted-foreground`.
- `ConfirmButton`: theo mẫu `confirmReset` ở `SettingsWorkspace.tsx:162,446`. Bấm lần 1 thì hiện nút xác nhận cùng nút huỷ; tự huỷ sau 5 giây.
- `ChangePreviewDialog`: dùng `ui-modal-backdrop` + `ui-modal-content`. Khi có `blockedReason`, nút "Áp dụng" bị tắt.
- `useVpsResource`: khi gặp `SUDO_REQUIRED` thì gọi `vps.requestSudo()` thay vì hiện lỗi. Lỗi còn lại đi qua `vpsErrorText`.
- Token màu: thêm `--success`, `--warning`, `--danger` vào từng khối theme; thêm `--color-success: var(--success)` và tương tự vào `@theme inline`. Giá trị cho theme tối: `#34d399`, `#fbbf24`, `#f87171`; cho `zinc-light`: `#047857`, `#b45309`, `#b91c1c` (đọc được trên `--card` trắng).
- i18n: thêm khoá phẳng tiền tố `vpsCommon*` vào `AppI18nStrings` cùng hai khối VI/EN. Gồm:
  - các nút: áp dụng, bỏ, xem trước, hoàn tác ngay, xác nhận, huỷ;
  - câu "N thay đổi chưa áp dụng";
  - 6 câu cho 6 mã lỗi;
  - "Chỉ hỗ trợ máy chủ Linux" / "Linux servers only".
  
  Đổi `navVpsConfig` thành "Cấu hình VPS" / "Server Config".

- [ ] **Step 1:** Viết các thành phần theo Interfaces.
- [ ] **Step 2:** Chạy `npm run build`. Kỳ vọng: PASS.
- [ ] **Step 3:** Chạy `grep -rnE 'isVi|select-none|\bswitch\b=|text-(cyan|emerald|rose|amber|red|green)-[0-9]' src/components/vps/shared` → không có kết quả.
- [ ] **Step 4:** Commit `feat(vps): add shared VPS UI components and status tokens`.

---

### Task 5: Khung `VpsConfigWorkspace` và điểm vào (§3.1, §3.2)

**Files:**
- Rewrite `src/components/VpsConfigWorkspace.tsx`. Giữ nguyên `VpsConfigWorkspaceProps` (36–44).
- Modify `src/components/DashboardWorkspace.tsx` (632, 1573, 1757, 2028). Thêm prop `vpsConfigLabel: string`, vì file này chưa dùng i18n.
- Modify `src/App.tsx`: truyền `vpsConfigLabel={t.navVpsConfig}`.

**Interfaces:**
- Consumes: lệnh `vps_probe` (Task 2), `VpsCtx` và các thành phần dùng chung (Task 4).
- Produces: mỗi studio nhận `{ vps: VpsCtx; probe: VpsProbe; host: HostEntry; onSaveHost: VpsConfigWorkspaceProps["onSaveHost"] }`. Tab là `"ssh" | "firewall" | "access"`.

Bố cục theo §3.2:
- Thanh máy gồm: chọn máy, `user@host:port`, `osId osVersion`, `StatusBadge` quyền, nút tải lại.
- Có banner riêng cho các trường hợp `UNSUPPORTED`, đang xác minh, và chỉ xem.
- Mật khẩu sudo nhập trong modal và giữ trong state.
- Host `local` hiện banner `UNSUPPORTED` mà không gọi lệnh nào.

- [ ] **Step 1:** Viết lại workspace. Studio cũ để tạm sau tab, không đụng tới.
- [ ] **Step 2:** Chạy `npm run build`. Kỳ vọng: PASS.
- [ ] **Step 3:** Xem trước với stub `vps_probe`, 3 kịch bản: thành công, `UNSUPPORTED`, `SUDO_REQUIRED` (modal hiện ra, nhập mật khẩu thì gọi lại probe). Ở 360px không có thanh cuộn ngang toàn trang (`document.documentElement.scrollWidth <= 360`), tab cuộn ngang được. Theme sáng đọc được badge. Chế độ VI không còn chữ tiếng Anh, trừ tên directive.
- [ ] **Step 4:** Chạy `grep -rn 'VPS Hardening' src` → không còn kết quả.
- [ ] **Step 5:** Commit `feat(vps): rebuild Server Config shell on the shared layout`.

---

### Task 6: SSH — đọc cấu hình đang hiệu lực, chọn nơi ghi (§4.2 bước 1–2)

**Files:** Modify `src-tauri/src/vps_config.rs`:
- thay `parse_sshd_config`, `parse_sshd_probe_output`, `compute_ssh_security_score`, `SshdConfigSnapshot`, `SshdConfigUpdate` và các test của chúng;
- giữ `render_sshd_config` nhưng đổi kiểu tham số.

Test tích hợp nằm trong `src-tauri/tests/vps_sshd.rs`.

**Interfaces — Produces:**
```rust
pub enum WriteTarget { DropIn, MainFile }
pub struct SshdView { pub settings: SshdSettings, pub write_target: WriteTarget, pub rollback_pending: bool }
pub fn parse_sshd_t(raw: &str) -> SshdSettings;               // "without-password" → "prohibit-password"; gộp các dòng allowusers/denyusers
pub fn sshd_write_target(main_cfg: &str) -> WriteTarget;     // DropIn chỉ khi dòng Include …/sshd_config.d/*.conf đứng trước mọi directive
pub fn render_dropin(s: &SshdSettings) -> String;            // dòng đầu "# Managed by ssh-box", mỗi directive một dòng
pub fn render_sshd_config(base_raw: &str, s: &SshdSettings) -> String; // giữ comment, chèn trước Match như cũ
pub async fn get_sshd_view(host_id: &str, pool: &SshPool, sudo_password: Option<String>) -> Result<SshdView, String>;
```
- `get_sshd_view` gọi `probe()`, rồi chạy một script đặc quyền gồm:
  - `sshd -T -C user=<login_user>,host=<client_ip>,addr=<client_ip>,laddr=<server_ip>,lport=<server_port>`;
  - `cat /etc/ssh/sshd_config`;
  - `systemctl is-active sshbox-rollback.timer`.
  
  Các phần ngăn nhau bằng dòng `---SSHBOX-PART---`.
- Lệnh Tauri: `vps_get_sshd_settings(host_id, sudo_password) -> SshdView`. Xoá `vps_get_sshd_config`.

- [ ] **Step 1: Viết test**
```rust
fn parses_sshd_t_alias_and_repeated_allowusers()
//   "permitrootlogin without-password\nallowusers alice\nallowusers bob@10.0.0.0/8\nport 2222"
//   → permit_root_login == "prohibit-password", allow_users == ["alice","bob@10.0.0.0/8"], port == 2222
fn write_target_dropin_only_when_include_precedes_directives()
//   "# c\nInclude /etc/ssh/sshd_config.d/*.conf\nPort 22" → DropIn; "Port 22\nInclude …" → MainFile; không có Include → MainFile
fn render_dropin_has_every_managed_directive() // mỗi trường của SshdSettings có đúng một dòng
fn renders_sshd_config_preserving_comments_and_before_match_blocks() // test cũ, đổi sang SshdSettings
// tích hợp (#[ignore]): sshd thật quyết định thứ tự ưu tiên, không phải parser.
fn dropin_wins_over_cloud_init_and_main_file()
//   main: "Include <d>/*.conf\nPasswordAuthentication no\nHostKey <k>"; <d>/50-cloud-init.conf: "PasswordAuthentication yes"
//   parse_sshd_t(`/usr/sbin/sshd -T -f main`).password_authentication == true
//   ghi render_dropin(password=false) vào <d>/00-sshbox.conf → == false
```
- [ ] **Step 2:** Chạy cả unit và tích hợp. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces.
- [ ] **Step 4:** Chạy cả unit và tích hợp. Kỳ vọng: PASS.
- [ ] **Step 5:** Commit `feat(vps): read effective sshd config via sshd -T and write a drop-in`.

---

### Task 7: SSH — áp dụng, hẹn giờ hoàn tác, xác minh bằng kết nối mới (§4.2 bước 3–9)

**Files:** Modify `src-tauri/src/vps_config.rs`: thay `apply_sshd_config`, `confirm_sshd_config`, `revert_sshd_config`. Modify `src-tauri/src/lib.rs`. Thêm test vào `src-tauri/tests/vps_sshd.rs`.

**Interfaces:**
- Consumes:
  - `probe`, `SshPool::session_fingerprint` (Task 2);
  - `check_sshd`, `SshdSettings` (Task 3);
  - `get_sshd_view`, `render_*`, `WriteTarget` (Task 6);
  - `ssh_pool::{list_known_hosts_at, delete_known_host_at, default_known_hosts_path}`.
- Produces:
```rust
pub struct ReloadPlan { pub ssh_socket_active: bool, pub selinux_enforcing: bool, pub new_port: Option<u16>,
    pub firewall_engine: FirewallEngine, pub firewall_active: bool }
pub struct SshdApplyStarted { pub rollback_at: i64, pub server_now: i64, pub new_port: u16 }
pub fn build_sshd_rollback_script(file: &str, plan: &ReloadPlan) -> String;
pub fn build_sshd_apply_script(file: &str, plan: &ReloadPlan) -> String;  // nội dung file mới đọc từ $SSHBOX_SECRET_FILE
pub async fn apply_sshd_settings(host_id: &str, pool: &SshPool, sudo_password: Option<String>, desired: SshdSettings) -> Result<SshdApplyStarted, String>;
pub async fn verify_new_connection(pool: &SshPool, host: &HostEntry, new_port: u16, expected_fp: &str) -> Result<(), String>;
pub async fn finish_sshd_apply(host_id: &str, pool: &SshPool, sudo_password: Option<String>, new_port: u16) -> Result<SshdView, String>;
pub async fn revert_sshd_settings(host_id: &str, pool: &SshPool, sudo_password: Option<String>) -> Result<SshdView, String>;
```
- Thứ tự trong `build_sshd_apply_script`. Đây là thuật toán; chữ ký hàm không tự quyết được:
  1. `set -eu`.
  2. Nếu `systemctl is-active --quiet sshbox-rollback.timer` → in `SSHBOX_ROLLBACK_PENDING`, `exit 3`.
  3. Nếu `[ -e file ]` thì `cp -p file file.sshbox.bak`; nếu không thì `: > file.sshbox.absent`.
  4. Ghi file qua `mktemp` rồi `install -m 644`.
  5. `sshd -t || { khôi phục; exit 1; }`.
  6. Ghi `build_sshd_rollback_script` vào `/etc/ssh/sshbox-rollback.sh`, `chmod 700`.
  7. `systemd-run --on-active=60 --unit=sshbox-rollback /bin/sh /etc/ssh/sshbox-rollback.sh`.
  8. In `NOW:$(date +%s)` và `ROLLBACK_AT:$(( $(date +%s) + 60 ))`.
  9. Nếu `selinux_enforcing` và `new_port != 22`: `semanage port -a -t ssh_port_t -p tcp <port> || semanage port -m -t ssh_port_t -p tcp <port>`.
  10. Nếu `firewall_active` và port đổi: mở port mới (`ufw allow <p>/tcp` hoặc `firewall-cmd --permanent --add-port=<p>/tcp && firewall-cmd --reload`).
  11. Nạp lại: socket thì `systemctl daemon-reload && systemctl restart ssh.socket`; còn lại `systemctl reload-or-restart ssh 2>/dev/null || systemctl reload-or-restart sshd`.
  12. `ss -tlnH "sport = :<port>" | grep -q sshd || exit 4`.
- `build_sshd_rollback_script`:
  - có `.sshbox.bak` thì `mv` về file; có `.sshbox.absent` thì `rm -f file` và xoá dấu;
  - nạp lại bằng `restart` thay cho `reload`;
  - xoá rule firewall đã thêm cho port mới (nếu có);
  - `systemctl stop sshbox-rollback.timer 2>/dev/null || true`.
- `apply_sshd_settings`:
  1. `probe`. Nếu `!has_systemd` thì trả `UNSUPPORTED`, vì bước hẹn giờ hoàn tác cần `systemd-run` (spec §2.2: distro lạ chỉ được xem). Sau đó `check_sshd(&ctx, &desired)?`.
  2. Gọi `get_sshd_view`; nếu `rollback_pending` thì trả `COMMAND_FAILED: rollback pending`.
  3. Chọn file theo `write_target` và render nội dung; nội dung đi vào `secret`.
  4. Chạy script.
- `verify_new_connection`:
  - chụp lại `list_known_hosts_at` trước khi thử;
  - `test_connection(&host_with_new_port, None)`;
  - lỗi kết nối, hoặc fingerprint khác `expected_fp` → xoá các dòng known_hosts mới sinh ra trong lần thử, rồi trả `VERIFY_FAILED: …`.
- `finish_sshd_apply`:
  - `expected_fp = pool.session_fingerprint(host_id)`;
  - xác minh thành công → `systemctl stop sshbox-rollback.timer` và xoá các file `.sshbox.bak`, `.sshbox.absent`, `sshbox-rollback.sh`;
  - xác minh thất bại → chạy `/etc/ssh/sshbox-rollback.sh` ngay, trả `VERIFY_FAILED`.
- Lệnh Tauri: `vps_apply_sshd_settings`, `vps_finish_sshd_apply`, `vps_revert_sshd_settings`. Xoá `vps_apply_sshd_config`, `vps_confirm_sshd_config`, `vps_revert_sshd_config`.

- [ ] **Step 1: Viết unit test**
```rust
fn apply_script_arms_rollback_before_reload()
//   s.starts_with("set -eu"); idx("SSHBOX_ROLLBACK_PENDING") < idx("cp -p"); idx("systemd-run") < idx("reload-or-restart")
//   idx("reload-or-restart") < idx("ss -tlnH")
fn apply_script_handles_socket_selinux_and_firewall()
//   socket=true → chứa "restart ssh.socket", không chứa "reload-or-restart"
//   selinux=true,new_port=Some(2222) → idx("semanage port") < idx("restart ssh.socket"); new_port=Some(22) → không có "semanage"
//   firewall ufw active + port đổi → chứa "ufw allow 2222/tcp" trước lệnh nạp lại
fn rollback_restores_backup_and_restarts()       // chạy script trong thư mục tạm, systemctl giả ghi log tham số
//   file == nội dung .sshbox.bak; log chứa "restart"
fn rollback_removes_dropin_that_did_not_exist()  // có .sshbox.absent → file và dấu đều bị xoá
fn apply_refuses_change_blocked_by_guard()       // check_sshd Err → không gọi run_privileged_script (gọi qua hàm thuần apply_preflight)
```
`apply_preflight(ctx: &ConnectionContext, view: &SshdView, desired: &SshdSettings) -> Result<(), String>` là phần kiểm tra thuần mà `apply_sshd_settings` gọi; nó gồm bộ chặn và kiểm tra hoàn tác đang chờ.
- [ ] **Step 2: Viết test tích hợp** (§6.2):
```rust
fn verify_succeeds_on_new_port_with_same_host_key()  // sshd A Port 22901+22902; pool kết nối 22901 → verify(22902) Ok
fn verify_fails_on_closed_port()                     // verify(22909) → Err bắt đầu "VERIFY_FAILED"
fn verify_fails_on_different_host_key_and_forgets_it() // sshd B 22903 host key khác → Err; known_hosts không còn "[127.0.0.1]:22903"
// Phiên cũ vẫn sống sau reload: nếu code xác minh bằng phiên cũ thì test này phải FAIL.
fn old_session_is_not_proof()
//   pool kết nối 22901; kill pid tiến trình sshd cha (phiên con vẫn sống); pool.exec_on_host(&host, "echo alive") == Ok
//   verify_new_connection(&pool, &host, 22901, &fp) → Err
```
- [ ] **Step 3:** Chạy unit và tích hợp. Kỳ vọng: FAIL.
- [ ] **Step 4:** Viết code theo Interfaces.
- [ ] **Step 5:** Chạy unit và tích hợp. Kỳ vọng: PASS.
- [ ] **Step 6: Chứng minh test bắt lỗi.** Tạm sửa `verify_new_connection` để gọi `pool.exec_on_host` (tức là xác minh bằng phiên cũ). Chạy `old_session_is_not_proof`. Kỳ vọng: FAIL. Ghi lại kết quả, rồi khôi phục code.
- [ ] **Step 7:** Commit `feat(vps): apply sshd changes with server-side rollback and new-connection verification`.

---

### Task 8: SSH — giao diện (§3, §4.2 phần UI)

**Files:** Rewrite `src/components/vps/SshHardeningStudio.tsx`. Thêm chuỗi `vpsSsh*` vào `src/i18n.ts`. Thêm kiểu `SshdSettings`, `SshdView`, `SshdApplyStarted` vào `src/types.ts`.

**Interfaces — Consumes:** `vps_get_sshd_settings`, `vps_apply_sshd_settings`, `vps_finish_sshd_apply`, `vps_revert_sshd_settings` (Task 6–7); props của studio (Task 5); các thành phần dùng chung (Task 4).

Nội dung:
- **Tình trạng:** 3 mục đạt/chưa đạt, mỗi mục có nút "Sửa" cuộn tới dòng cài đặt tương ứng:
  - `!passwordAuthentication`;
  - `permitRootLogin !== "yes"`;
  - `maxAuthTries <= 4`.
- **Đăng nhập:** `Port` (`.ui-input` số 1–65535), `PasswordAuthentication`, `PubkeyAuthentication` (`Switch`), `PermitRootLogin` (`SelectField`: `yes`, `prohibit-password`, `no`).
- **Ai được vào:** `AllowUsers`, `DenyUsers` (ô nhập, các user cách nhau bằng dấu cách).
- **Giới hạn phiên:** `MaxAuthTries`, `MaxSessions`, `LoginGraceTime`, `ClientAliveInterval`, `ClientAliveCountMax`.
- **Luồng áp dụng:**
  1. Thanh "N thay đổi" → `ChangePreviewDialog`.
  2. Bấm Áp dụng → modal tiến trình 9 bước của §4.2. Đếm ngược tính bằng `rollbackAt − serverNow`, cộng với thời gian trôi qua tính theo đồng hồ app.
  3. Gọi `vps_finish_sshd_apply` ngay, không có nút "Xác nhận".
  4. Thành công và port đổi → `onSaveHost({ ...host, port: newPort })`.
  5. Thất bại → modal hiện bước lỗi và lý do; nút "Hoàn tác ngay" gọi `vps_revert_sshd_settings`.
- Khi `!probe.hasSystemd`: hiện banner chỉ xem và tắt mọi điều khiển.

- [ ] **Step 1:** Viết lại studio.
- [ ] **Step 2:** Chạy `npm run build`. Kỳ vọng: PASS.
- [ ] **Step 3:** Xem trước với stub cho 3 kịch bản:
  - đổi port thành công: `onSaveHost` được gọi đúng một lần, sau `vps_finish_sshd_apply`;
  - `VERIFY_FAILED`: `onSaveHost` không được gọi;
  - `LOCKOUT_BLOCKED`: dialog tắt nút Áp dụng và hiện câu giải thích.
  
  Kiểm thêm ở 360px, theme sáng, VI/EN.
- [ ] **Step 4:** Chạy `grep -nE 'isVi|select-none|Fortress|gauge|preset|\bswitch\b=' src/components/vps/SshHardeningStudio.tsx` → không có kết quả.
- [ ] **Step 5:** Commit `feat(vps): rebuild SSH studio on the shared layout`.

---

### Task 9: Firewall — ufw (§4.4)

**Files:** Modify `src-tauri/src/vps_config.rs`:
- `FirewallSnapshot` thêm `ipv6_filtered: bool`;
- sửa `compute_socket_exposure` (429), `build_add_firewall_rule_script` (937), `build_delete_firewall_rule_script` (1075), `build_set_firewall_state_script` (1097), `get_firewall_snapshot` (852).

**Interfaces — Produces:**
```rust
pub fn build_ufw_add_rule_script(action: &str, source: &str, port: &str, proto: &str) -> Result<String, String>; // deny/reject → "ufw insert 1 …"
pub fn build_ufw_delete_rule_script(rule: &FirewallRuleItem) -> String;  // xoá theo nội dung raw_spec
pub fn build_set_firewall_state_script(engine: FirewallEngine, enable: bool, ensure_ssh_port: Option<u16>) -> Result<String, String>;
pub async fn add_firewall_rule(host_id, pool, sudo_password, action, source, port, proto) -> Result<FirewallSnapshot, String>;
pub async fn delete_firewall_rule(host_id, pool, sudo_password, rule: FirewallRuleItem) -> Result<FirewallSnapshot, String>;
pub async fn set_firewall_state(host_id, pool, sudo_password, enable: bool) -> Result<FirewallSnapshot, String>;
```
- Mỗi hàm ghi đều làm theo cùng một mẫu:
  1. `probe` → hàm chặn tương ứng của Task 3;
  2. chạy `set -eu` + lệnh ghi + `echo SSHBOX_SNAPSHOT_BEGIN` + script đọc snapshot, trong cùng một lần chạy;
  3. parse phần nằm sau dấu `SSHBOX_SNAPSHOT_BEGIN`.
- `FirewallEngine::Iptables` hoặc `None` → `UNSUPPORTED: install ufw to edit rules`.
- Xoá theo nội dung: lấy số thứ tự từ `ufw status numbered`, ở dòng có phần sau `] ` (đã gộp khoảng trắng) bằng `raw_spec`. Không tìm thấy → `echo SSHBOX_RULE_NOT_FOUND; exit 3`, rồi chạy `ufw --force delete "$n"`.
- `ipv6_filtered`: `grep -q '^IPV6=yes' /etc/default/ufw`.
- Tính độ phơi của cổng: duyệt rule theo thứ tự, rule khớp đầu tiên quyết định. IPv4 và IPv6 tính riêng.

- [ ] **Step 1: Viết test.** Test dùng `ufw` giả: `status numbered` in một danh sách mẫu; `delete` ghi tham số ra log.
```rust
fn ufw_delete_matches_content_after_numbers_shift() // raw_spec là rule [3] cũ, danh sách hiện tại đã lệch thành [2] → log == "--force delete 2"
fn ufw_delete_missing_rule_deletes_nothing()        // output chứa SSHBOX_RULE_NOT_FOUND, log không có "delete"
fn deny_rules_are_inserted_first()                  // build_ufw_add_rule_script("deny", "198.51.100.9", "80", "tcp") chứa "ufw insert 1 deny"
fn exposure_is_first_match_per_family()             // [deny 3306 Anywhere, allow 3306 Anywhere] → blocked; đảo thứ tự → public
fn iptables_engine_is_read_only()                   // build_set_firewall_state_script(Iptables, …) → Err bắt đầu "UNSUPPORTED"
fn enable_adds_ssh_allow_first_when_missing()       // sửa từ test enforces_anti_lockout_… : idx("allow 2222/tcp") < idx("--force enable")
fn ufw_ipv6_disabled_is_reported()                  // parse snapshot có dòng IPV6=no → ipv6_filtered == false
```
- [ ] **Step 2:** Chạy `cargo test … vps`. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces.
- [ ] **Step 4:** Chạy `cargo test … vps`. Kỳ vọng: PASS.
- [ ] **Step 5:** Commit `fix(vps): ufw rules ordered, deleted by content, guarded`.

---

### Task 10: Firewall — firewalld (§4.4)

**Files:** Modify `src-tauri/src/vps_config.rs`.

**Interfaces — Produces:**
```rust
pub fn parse_firewalld_list_all(raw: &str) -> Vec<FirewallRuleItem>; // ports, services (ssh→22), rich rules (raw_spec = nguyên câu rule)
pub fn build_firewalld_add_rule_script(action: &str, source: &str, port: &str, proto: &str) -> Result<String, String>;
pub fn build_firewalld_delete_rule_script(rule: &FirewallRuleItem) -> String;
```
- Rule `allow` không có nguồn dùng `--permanent --add-port=<p>/<proto>`. Rule có nguồn hoặc là deny dùng rich rule, ví dụ: `rule family=ipv4 source address=X port port=P protocol=tcp accept` hoặc `rule priority=-100 family=… source address=X drop`. Chọn `family` theo kiểu địa chỉ.
- Mọi thay đổi dùng `--permanent` rồi `--reload`.
- Khi bật firewalld (`systemctl enable --now firewalld`): nếu `ssh_allow_needed` thì `--permanent --add-port=<server_port>/tcp` trước.
- `add_firewall_rule`, `delete_firewall_rule`, `set_firewall_state` của Task 9 rẽ nhánh theo `probe.firewall_engine`.

- [ ] **Step 1: Viết test**
```rust
fn parses_firewalld_ports_services_and_rich_rules()  // output mẫu từ `firewall-cmd --list-all` → 3 loại rule đúng port/nguồn
fn firewalld_rules_are_permanent_then_reloaded()     // mọi script add/delete chứa "--permanent" và kết thúc bằng "--reload"
fn firewalld_deny_uses_negative_priority()           // chứa "priority=-100" và "drop"
fn firewalld_delete_uses_exact_rich_rule()           // chứa --remove-rich-rule=<raw_spec đã quote bằng shell_single_quote>
```
- [ ] **Step 2:** Chạy test. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces.
- [ ] **Step 4:** Chạy test. Kỳ vọng: PASS.
- [ ] **Step 5:** Commit `feat(vps): write firewall rules through firewalld`.

---

### Task 11: Firewall — giao diện

**Files:** Rewrite `src/components/vps/FirewallStudio.tsx`. Thêm chuỗi `vpsFw*`. Cập nhật kiểu trong `src/types.ts`.

Nội dung:
- **Tình trạng:**
  - firewall bật/tắt (`Switch`);
  - engine đang dùng;
  - cảnh báo "IPv6 không được lọc" / "IPv6 traffic is not filtered" khi `!ipv6Filtered`;
  - với iptables: banner chỉ xem, kèm hướng dẫn cài `ufw`.
- **Cổng đang mở:** danh sách thẻ gồm cổng, tiến trình, `StatusBadge` độ phơi.
- **Rule:** danh sách thẻ, nút xoá dùng `ConfirmButton`. Form thêm rule gồm:
  - hành động (`SelectField`), nguồn, cổng, giao thức;
  - nút "IP của tôi" điền `probe.context.clientIp`.
  
  IP và port SSH chỉ lấy từ `probe.context`, không lấy từ snapshot hay `host.port`.

- [ ] **Step 1:** Viết lại studio.
- [ ] **Step 2:** Chạy `npm run build`. Kỳ vọng: PASS.
- [ ] **Step 3:** Xem trước với stub: ufw, firewalld, iptables chỉ xem, và `LOCKOUT_BLOCKED` khi xoá rule SSH duy nhất. Kiểm ở 360px, theme sáng, VI/EN.
- [ ] **Step 4:** Chạy lại lệnh grep của Task 8 cho file này. Kỳ vọng: không có kết quả.
- [ ] **Step 5:** Commit `feat(vps): rebuild Firewall studio on the shared layout`.

---

### Task 12: Users & Keys — backend (§4.5)

**Files:** Modify `src-tauri/src/vps_config.rs`: sửa `parse_access_keys_snapshot` (2487), `build_authorize_ssh_key_script` (2699), `build_revoke_ssh_key_script` (2740), `build_create_linux_user_script` (2772), `build_update_linux_user_privilege_script` (2864) cùng các hàm async tương ứng. Modify `src-tauri/src/lib.rs`.

**Interfaces — Produces:**
```rust
pub enum SudoMode { None, NoPassword, WithPassword }  // serde camelCase
pub fn build_create_user_script(username: &str, sudo: SudoMode) -> Result<String, String>; // mật khẩu (nếu có) đọc từ $SSHBOX_SECRET_FILE dạng "user:pw"
pub fn build_set_user_lock_script(username: &str, lock: bool) -> Result<String, String>;   // "usermod -L -e 1" / "usermod -U -e ''"
pub fn build_set_user_sudo_script(username: &str, sudo: SudoMode) -> Result<String, String>;
pub fn build_authorize_key_script(username: &str) -> Result<String, String>;              // dòng key đọc từ $SSHBOX_SECRET_FILE
pub fn build_revoke_key_script(username: &str, key_body: &str) -> Result<String, String>; // key_body chỉ gồm [A-Za-z0-9+/=]
pub async fn create_linux_user(host_id, pool, sudo_password, username: String, sudo: SudoMode, password: Option<String>) -> Result<AccessKeysSnapshot, String>;
pub async fn set_user_lock(host_id, pool, sudo_password, username: String, lock: bool) -> Result<AccessKeysSnapshot, String>;
pub async fn set_user_sudo(host_id, pool, sudo_password, username: String, sudo: SudoMode, password: Option<String>) -> Result<AccessKeysSnapshot, String>;
pub async fn authorize_ssh_key(host_id, pool, sudo_password, username: String, key_line: String) -> Result<AccessKeysSnapshot, String>;
pub async fn revoke_ssh_key(host_id, pool, sudo_password, username: String, key_body: String) -> Result<AccessKeysSnapshot, String>;
```
- Tạo user: `useradd -m -s /bin/bash`.
  - `None` và `NoPassword`: `usermod -p '*'`. Không để `!`, vì khi `UsePAM no` sshd sẽ từ chối cả đăng nhập bằng key.
  - `WithPassword`: `chpasswd < "$SSHBOX_SECRET_FILE"`; mật khẩu rỗng hoặc chứa `\n`/`:` → lỗi trước khi gửi.
- Nhóm sudo: `g=sudo; getent group sudo >/dev/null || g=wheel`.
- `NoPassword`: ghi qua `mktemp`, kiểm bằng `visudo -cf`, rồi `install -m 0440` vào `/etc/sudoers.d/90-sshbox-<user>`.
- Thao tác trong `~/.ssh` chạy bằng `home=$(getent passwd u | cut -d: -f6)` và `runuser -u <u> -- sh -c 'umask 077; …' sh "$home"`.
  - Authorize: `… < "$SSHBOX_SECRET_FILE"`.
  - Revoke: dùng `awk` tìm token là loại key (`ssh-*`, `ecdsa-*`, `sk-*`), so token kế tiếp bằng `==` với `key_body`, ghi ra `mktemp` trong `~/.ssh`, rồi `mv`.
- Snapshot: `locked = shadow_pw.starts_with('!') || (expire_days != "" && expire_days <= today_days)`; `today_days` lấy từ `$(( $(date +%s) / 86400 ))` in trong script.
- Mỗi hàm ghi gọi `probe` → `check_user_change` / `check_key_revoke` trước, và trả snapshot trong cùng lần chạy như Task 9.

- [ ] **Step 1: Viết test**
```rust
// Với UsePAM yes, chỉ "-L" không chặn được đăng nhập bằng key.
fn lock_expires_account_so_keys_stop_working()     // chứa "usermod -L -e 1"; unlock chứa "usermod -U -e ''"
fn key_file_ops_run_as_the_target_user()           // chứa "runuser -u 'alice' --" và "umask 077"; không có ">> /home"
fn revoke_matches_key_field_exactly()              // chạy script với runuser giả (`shift 3; exec "$@"`), HOME tạm, authorized_keys có:
//   dòng có options `from="10.0.0.1" ssh-ed25519 AAAAX alice`, key `AAAAXY` (có AAAAX là tiền tố), comment
//   revoke "AAAAX" → chỉ dòng AAAAX bị xoá; AAAAXY và dòng comment còn nguyên
fn sudo_with_password_requires_password()          // create_linux_user(..WithPassword, None) → Err trước khi chạy (qua hàm thuần validate_user_password)
fn user_password_with_newline_is_rejected()        // validate_user_password("alice", "a\nb") → Err
fn nopasswd_sudoers_checked_with_visudo_before_install() // idx("visudo -cf") < idx("install -m 0440")
fn parses_locked_by_expiry()                        // shadow "deploy:*:…:1:" với today=20000 → locked == true
```
`validate_user_password(username: &str, password: Option<&str>, sudo: &SudoMode) -> Result<(), String>` là hàm thuần mà `create_linux_user` và `set_user_sudo` gọi.
- [ ] **Step 2:** Chạy `cargo test … vps`. Kỳ vọng: FAIL.
- [ ] **Step 3:** Viết code theo Interfaces và đăng ký lệnh Tauri: `vps_create_linux_user`, `vps_set_user_lock`, `vps_set_user_sudo`, `vps_authorize_ssh_key`, `vps_revoke_ssh_key`. Xoá `vps_update_linux_user_privilege`.
- [ ] **Step 4:** Chạy `cargo test … vps`. Kỳ vọng: PASS.
- [ ] **Step 5:** Commit `fix(vps): safe user locking, sudo modes and per-user key file writes`.

---

### Task 13: Users & Keys — giao diện

**Files:** Rewrite `src/components/vps/AccessKeysStudio.tsx`. Giữ nguồn key hiện có: prop `sshKeys: SshKeyItem[]`. Thêm chuỗi `vpsUsers*`.

Nội dung:
- Danh sách user dạng thẻ: tên, `StatusBadge` (sudo / bị khoá), danh sách key kèm nút thu hồi (`ConfirmButton`).
- Mỗi user có:
  - nút Khoá/Mở khoá (`ConfirmButton`);
  - `SelectField` cho 3 chế độ sudo; chọn "có mật khẩu" thì hiện ô mật khẩu bắt buộc;
  - nút "Cấp key" chọn từ `sshKeys`.
- Form tạo user: tên, chế độ sudo, mật khẩu (chỉ khi chọn `withPassword`).

- [ ] **Step 1:** Viết lại studio.
- [ ] **Step 2:** Chạy `npm run build`. Kỳ vọng: PASS.
- [ ] **Step 3:** Xem trước với stub: tạo user ở cả 3 chế độ; `LOCKOUT_BLOCKED` khi tự khoá chính mình hoặc thu hồi key đang dùng. Kiểm ở 360px, theme sáng, VI/EN.
- [ ] **Step 4:** Chạy lại lệnh grep của Task 8 cho file này và cho cả `src/components/VpsConfigWorkspace.tsx`. Kỳ vọng: không có kết quả.
- [ ] **Step 5:** Commit `feat(vps): rebuild Users & Keys studio on the shared layout`.

---

### Task 14: Kiểm thử thủ công (§6.3, §6.4)

- [ ] **Step 1:** Chạy `cargo test --manifest-path src-tauri/Cargo.toml`, lệnh test tích hợp và `npm run build`. Kỳ vọng: tất cả PASS.
- [ ] **Step 2:** Chạy ma trận server của §6.3 trên máy ảo dùng một lần (Ubuntu 24.04, Debian 13, Rocky 9), đủ 6 kịch bản. Không dùng server thật.
- [ ] **Step 3:** Chạy ma trận app: macOS, Windows, iOS hoặc Android × theme tối/sáng × VI/EN, với 4 tiêu chí của §6.3.
- [ ] **Step 4:** Soạn bảng kết quả (đạt/không đạt cho từng ô). Chỉ cập nhật mô tả MR !50 khi người dùng đồng ý; nhớ thay luôn phần mô tả "6 studio" cũ.
- [ ] **Step 5:** Commit nếu có sửa lỗi. Báo cáo những ô chưa chạy được và lý do.
