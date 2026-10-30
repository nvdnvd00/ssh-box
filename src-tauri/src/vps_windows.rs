use crate::host_store::HostEntry;
use crate::ssh_pool::{AuthUsed, SshPool};
use crate::vps_config::{
    classify_privileged_output, find_remote_host, parse_authorized_key_line, parse_ssh_connection, parse_sshd_view,
    vps_err, AccessKeysSnapshot, ConnectionContext, FirewallEngine, LinuxAccountItem, PrivilegeLevel, SshdView,
    VpsProbe, WriteTarget, ERR_COMMAND_FAILED, ERR_UNSUPPORTED, SSHD_PART,
};
use crate::vps_support::{compute_support, OsCaps, OsFamily, VpsSupport};
use base64::Engine;
use serde::Deserialize;

// Fixed and short: cmd.exe caps a command line at 8191 chars, so the real script travels on stdin.
pub const PS_BOOTSTRAP: &str = r#"$ErrorActionPreference = 'Stop'; $ProgressPreference = 'SilentlyContinue'
[Console]::InputEncoding = New-Object Text.UTF8Encoding $false
[Console]::OutputEncoding = New-Object Text.UTF8Encoding $false
$in = [Console]::In.ReadToEnd(); $sep = "`nSSHBOX_SECRET_BEGIN`n"; $i = $in.IndexOf($sep)
$SSHBOX_SECRET = $in.Substring($i + $sep.Length)
try { & ([ScriptBlock]::Create($in.Substring(0, $i))) 2>&1 | Out-String -Width 4096 }
catch { ($_ | Out-String -Width 4096); 'SSHBOX_EXIT_CODE:1' }
"#;
pub const PS_SECRET_SEP: &str = "\nSSHBOX_SECRET_BEGIN\n";

pub fn build_powershell_command() -> String {
    let utf16: Vec<u8> = PS_BOOTSTRAP.encode_utf16().flat_map(u16::to_le_bytes).collect();
    format!(
        "powershell.exe -NoProfile -NonInteractive -ExecutionPolicy Bypass -EncodedCommand {}",
        base64::engine::general_purpose::STANDARD.encode(utf16)
    )
}

pub fn powershell_stdin(script: &str, secret: Option<&str>) -> Vec<u8> {
    format!("{script}{PS_SECRET_SEP}{}", secret.unwrap_or("")).into_bytes()
}

pub fn ps_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

pub async fn run_windows_script(
    host_id: &str,
    pool: &SshPool,
    script: &str,
    secret: Option<&str>,
) -> Result<String, String> {
    let host = find_remote_host(host_id)?;
    let out = pool
        .exec_bytes_on_host(&host, &build_powershell_command(), Some(&powershell_stdin(script, secret)))
        .await?;
    classify_privileged_output(&String::from_utf8_lossy(&out))
}

pub const WINDOWS_PROBE_SCRIPT: &str = r#"Write-Output 'OS_FAMILY:windows'
Write-Output ('OS_CAPTION:' + (Get-CimInstance Win32_OperatingSystem).Caption)
Write-Output ('OS_DISPLAY_VERSION:' + (Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion').DisplayVersion)
Write-Output ('USER:' + [Environment]::UserName)
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Output ('ADMIN:' + [int]$admin)
Write-Output ('OPENSSH:' + (Get-Item "$env:WINDIR\System32\OpenSSH\sshd.exe" -ErrorAction SilentlyContinue).VersionInfo.ProductVersion)
Write-Output ('SSHD_SERVICE:' + (Get-Service sshd -ErrorAction SilentlyContinue).Status)
Write-Output ('SSH_CONNECTION:' + $env:SSH_CONNECTION)
Write-Output ('SCHEDTASKS:' + [int][bool](Get-Module -ListAvailable ScheduledTasks))
Write-Output ('NETSECURITY:' + [int][bool](Get-Module -ListAvailable NetSecurity))
"#;

pub fn parse_windows_probe(raw: &str, auth: AuthUsed) -> Result<VpsProbe, String> {
    let field = |key: &str| {
        raw.lines()
            .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
            .unwrap_or_default()
    };
    if field("OS_FAMILY:") != "windows" {
        return Err(vps_err(ERR_UNSUPPORTED, "unsupported_os"));
    }
    let (client_ip, server_ip, server_port) = parse_ssh_connection(&field("SSH_CONNECTION:"))?;
    let caption = field("OS_CAPTION:");
    let os_version = format!(
        "{} {}",
        caption.strip_prefix("Microsoft Windows ").unwrap_or(&caption),
        field("OS_DISPLAY_VERSION:")
    );
    let net_security = field("NETSECURITY:") == "1";
    let mut probe = VpsProbe {
        privilege: if field("ADMIN:") == "1" { PrivilegeLevel::Admin } else { PrivilegeLevel::None },
        context: ConnectionContext { login_user: field("USER:"), auth, client_ip, server_ip, server_port },
        os_id: "Windows".to_string(),
        os_version: os_version.trim().to_string(),
        openssh_version: field("OPENSSH:"),
        has_systemd: false,
        ssh_socket_active: false,
        selinux_enforcing: false,
        firewall_engine: if net_security { FirewallEngine::Windows } else { FirewallEngine::None },
        os_family: OsFamily::Windows,
        support: VpsSupport::default(),
        caps: OsCaps {
            sshd_running: field("SSHD_SERVICE:") == "Running",
            has_scheduled_tasks: field("SCHEDTASKS:") == "1",
            has_net_security: net_security,
            ..Default::default()
        },
    };
    probe.support = compute_support(&probe);
    Ok(probe)
}

pub const WINDOWS_SSHD_CONFIG: &str = r"C:\ProgramData\ssh\sshd_config";
const SSHD_EXE: &str = r#""$env:WINDIR\System32\OpenSSH\sshd.exe""#;

// PS 5.1 does not throw when a native command fails, so $LASTEXITCODE is checked and thrown by hand.
pub fn windows_sshd_view_script(spec: &str) -> String {
    format!(
        r#"$t = & {SSHD_EXE} -T -C {spec}
if ($LASTEXITCODE -ne 0) {{ throw "sshd -T failed with exit code $LASTEXITCODE" }}
$t
'{SSHD_PART}'
Get-Content -Raw -LiteralPath '{WINDOWS_SSHD_CONFIG}'
'{SSHD_PART}'
'{SSHD_PART}'
if ((Get-Command Get-ScheduledTask -ErrorAction SilentlyContinue) -and (Get-ScheduledTask -TaskName sshbox-rollback -ErrorAction SilentlyContinue)) {{ 'active' }} else {{ 'inactive' }}
"#,
        spec = ps_quote(spec)
    )
}

// sshd -T needs the host keys, which only an administrator can read: refuse before running anything.
fn check_windows_readable(p: &VpsProbe) -> Result<(), String> {
    if p.privilege != PrivilegeLevel::Admin {
        return Err(vps_err(ERR_UNSUPPORTED, "windows_not_admin"));
    }
    Ok(())
}

pub fn parse_windows_sshd_view(raw: &str) -> Result<SshdView, String> {
    let mut view = parse_sshd_view(&raw.replace("\r\n", "\n"))?;
    // No drop-in directory is managed on Windows.
    view.write_target = WriteTarget::MainFile;
    Ok(view)
}

pub async fn get_windows_sshd_view(host_id: &str, pool: &SshPool, probe: &VpsProbe) -> Result<SshdView, String> {
    check_windows_readable(probe)?;
    let c = &probe.context;
    let spec = format!("user={},host={},addr={},laddr={},lport={}", c.login_user, c.client_ip, c.client_ip, c.server_ip, c.server_port);
    parse_windows_sshd_view(&run_windows_script(host_id, pool, &windows_sshd_view_script(&spec), None).await?)
}

pub const WINDOWS_ACCOUNTS_READ_SCRIPT: &str = r#"$sshd = "$env:WINDIR\System32\OpenSSH\sshd.exe"
$admins = @(Get-LocalGroupMember -SID S-1-5-32-544 -ErrorAction SilentlyContinue | ForEach-Object { $_.Name.Split('\')[-1] })
$rows = foreach ($u in Get-LocalUser) {
  $isAdmin = $admins -contains $u.Name
  $file = "C:\Users\$($u.Name)\.ssh\authorized_keys"
  if ($isAdmin) {
    try {
      $eff = & $sshd -T -C "user=$($u.Name),host=localhost,addr=127.0.0.1" 2>$null | Where-Object { $_ -match '^authorizedkeysfile ' }
      if ($eff -match 'administrators_authorized_keys') { $file = 'C:\ProgramData\ssh\administrators_authorized_keys' }
    } catch { }
  }
  $keys = @()
  try { $keys = @(Get-Content -LiteralPath $file -ErrorAction Stop) } catch { }
  [ordered]@{ name = $u.Name; enabled = [bool]$u.Enabled; isAdmin = [bool]$isAdmin; keysFile = $file; keys = $keys }
}
ConvertTo-Json -InputObject @($rows) -Depth 4 -Compress
"#;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WinAccount {
    name: String,
    enabled: bool,
    #[serde(default)]
    is_admin: bool,
    #[serde(default)]
    keys_file: String,
    #[serde(default)]
    keys: Vec<String>,
}

pub fn parse_windows_accounts(json: &str) -> Result<AccessKeysSnapshot, String> {
    let json = json.trim();
    // PS 5.1 can still unwrap a one-element array into a bare object.
    let rows: Vec<WinAccount> = if json.is_empty() {
        Ok(Vec::new())
    } else if json.starts_with('[') {
        serde_json::from_str(json)
    } else {
        serde_json::from_str(json).map(|a| vec![a])
    }
    .map_err(|e| vps_err(ERR_COMMAND_FAILED, format!("cannot read the account list: {e}")))?;
    let accounts = rows
        .into_iter()
        .map(|a| LinuxAccountItem {
            username: a.name,
            uid: 0,
            gid: 0,
            home_dir: String::new(),
            shell: String::new(),
            locked: !a.enabled,
            groups: if a.is_admin { vec!["Administrators".to_string()] } else { Vec::new() },
            has_sudo: a.is_admin,
            nopasswd_sudo: false,
            authorized_keys: a.keys.iter().enumerate().filter_map(|(i, l)| parse_authorized_key_line(l, i as u32)).collect(),
            keys_file: a.keys_file,
        })
        .collect();
    Ok(AccessKeysSnapshot { accounts })
}

pub async fn get_windows_accounts(host_id: &str, pool: &SshPool) -> Result<AccessKeysSnapshot, String> {
    parse_windows_accounts(&run_windows_script(host_id, pool, WINDOWS_ACCOUNTS_READ_SCRIPT, None).await?)
}

// The exec fails when the host has no powershell.exe (e.g. cmd.exe: "not recognized"): not a Windows we can manage.
fn exec_failure(_detail: &str) -> String {
    vps_err(ERR_UNSUPPORTED, "unsupported_os")
}

pub async fn probe_windows(host: &HostEntry, pool: &SshPool) -> Result<VpsProbe, String> {
    let out = pool
        .exec_bytes_on_host(host, &build_powershell_command(), Some(&powershell_stdin(WINDOWS_PROBE_SCRIPT, None)))
        .await
        .map_err(|e| exec_failure(&e))?;
    let auth = pool
        .session_auth(&host.id)
        .await
        .ok_or_else(|| vps_err(ERR_COMMAND_FAILED, "no SSH session for this host"))?;
    parse_windows_probe(&String::from_utf8_lossy(&out), auth)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn key_auth() -> AuthUsed {
        AuthUsed::Key { public_key_body: "AAAAC3NzaC1lZDI1NTE5".to_string() }
    }

    fn raw(admin: &str, user: &str, conn: &str) -> String {
        format!(
            "OS_FAMILY:windows\nOS_CAPTION:Microsoft Windows 11 Pro\nOS_DISPLAY_VERSION:24H2\nUSER:{user}\nADMIN:{admin}\nOPENSSH:9.5.0.0\nSSHD_SERVICE:Running\nSSH_CONNECTION:{conn}\nSCHEDTASKS:1\nNETSECURITY:1\n"
        )
    }
    const CONN: &str = "10.0.0.5 51000 10.0.0.9 22";

    // Script nằm trong lệnh thì vượt giới hạn 8191 ký tự của cmd, và bí mật lộ trong danh sách tiến trình.
    #[test]
    fn powershell_command_is_constant_and_carries_no_script() {
        let c = build_powershell_command();
        assert_eq!(c, build_powershell_command());
        assert!(c.len() < 8191);
        assert!(!c.contains("sshbox_secret_begin"));
    }

    // cmd.exe và PowerShell diễn giải các ký tự đặc biệt; lệnh chỉ được chứa base64 thuần.
    #[test]
    fn powershell_command_is_plain_ascii_safe_in_cmd_and_powershell() {
        let c = build_powershell_command();
        let b64 = c.split("-EncodedCommand ").nth(1).unwrap();
        assert!(b64.chars().all(|ch| ch.is_ascii_alphanumeric() || "+/=".contains(ch)));
        assert!(!c.chars().any(|ch| "%^&|<>\"`$".contains(ch)));
    }

    // Bootstrap sai mã hoá thì PowerShell chạy sai script, hoặc không chạy gì.
    #[test]
    fn encoded_command_decodes_to_bootstrap() {
        let c = build_powershell_command();
        let b64 = c.split("-EncodedCommand ").nth(1).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        let units: Vec<u16> = bytes.chunks(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
        assert_eq!(String::from_utf16(&units).unwrap(), PS_BOOTSTRAP);
    }

    // Bootstrap tách script và bí mật theo đúng dấu phân cách này, bằng UTF-8.
    #[test]
    fn stdin_carries_script_then_secret_utf8() {
        let got = powershell_stdin("Write-Output 'Đạt'", Some("p'a$s"));
        assert_eq!(got, "Write-Output 'Đạt'\nSSHBOX_SECRET_BEGIN\np'a$s".as_bytes());
    }

    // Giá trị chèn vào script phải không thoát được khỏi chuỗi trong dấu nháy đơn.
    #[test]
    fn ps_quote_doubles_single_quotes() {
        assert_eq!(ps_quote("a'b"), "'a''b'");
    }

    // Lưới quyền và firewall dựa vào các trường này; sai là hiện nhầm tính năng.
    #[test]
    fn windows_probe_admin_with_ssh_running() {
        let p = parse_windows_probe(&raw("1", "admin", CONN), key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::Admin);
        assert_eq!(p.os_version, "11 Pro 24H2");
        assert!(p.caps.sshd_running);
        assert_eq!(p.firewall_engine, FirewallEngine::Windows);
        assert_eq!(p.os_family, OsFamily::Windows);
    }

    // Người dùng không phải Administrator không được coi là có quyền ghi.
    #[test]
    fn windows_probe_non_admin() {
        let p = parse_windows_probe(&raw("0", "bob", CONN), key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::None);
    }

    // Tên người dùng có dấu là thật ở máy Windows tiếng Việt; không được làm hỏng.
    #[test]
    fn windows_probe_keeps_vietnamese_user_name() {
        let p = parse_windows_probe(&raw("1", "Đạt", CONN), key_auth()).unwrap();
        assert_eq!(p.context.login_user, "Đạt");
    }

    // Bộ chặn khoá cổng so sánh IP máy khách/máy chủ; IPv6 phải đi qua nguyên vẹn.
    #[test]
    fn windows_probe_parses_ipv6_ssh_connection() {
        let p = parse_windows_probe(&raw("1", "a", "fe80::1 51000 fe80::2 2222"), key_auth()).unwrap();
        assert_eq!(p.context.client_ip, "fe80::1");
        assert_eq!(p.context.server_port, 2222);
    }

    // Máy không có PowerShell (cmd báo lỗi) phải ra "không hỗ trợ", không phải lỗi lạ.
    #[test]
    fn windows_probe_without_marker_is_unsupported() {
        let e = parse_windows_probe("'powershell.exe' is not recognized as an internal or external command", key_auth()).unwrap_err();
        assert!(e.starts_with("UNSUPPORTED: unsupported_os"));
    }

    // Lỗi chạy lệnh (không có powershell.exe) được đổi thành unsupported_os, (lỗi kết nối đã được get_or_connect báo trước đó).
    #[test]
    fn exec_failure_maps_to_unsupported_os() {
        assert!(exec_failure("exit 1: 'powershell.exe' is not recognized").starts_with("UNSUPPORTED: unsupported_os"));
    }

    const SSHD_RAW: &str = "port 22\r\npermitrootlogin no\r\n---SSHBOX-PART---\r\nPort 22\r\nMatch Group administrators\r\n       AuthorizedKeysFile __PROGRAMDATA__/ssh/administrators_authorized_keys\r\n---SSHBOX-PART---\r\n---SSHBOX-PART---\r\ninactive\r\n";

    // Windows has no drop-in directory, so the write target must always be the main file even if the file looks like Include.
    #[test]
    fn windows_sshd_view_parses_and_forces_main_file() {
        let v = parse_windows_sshd_view(SSHD_RAW).unwrap();
        assert_eq!(v.settings.port, 22);
        assert_eq!(v.write_target, WriteTarget::MainFile);
        assert!(!v.rollback_pending && v.main_raw.contains("Match Group administrators"));
        let inc = SSHD_RAW.replace("Port 22\r\n", "Include sshd_config.d/*.conf\r\n");
        assert_eq!(parse_windows_sshd_view(&inc).unwrap().write_target, WriteTarget::MainFile);
    }

    // sshd -T cannot read host keys without admin; running it would only produce a confusing error box.
    #[test]
    fn windows_view_non_admin_is_refused_without_running() {
        let p = parse_windows_probe(&raw("0", "bob", CONN), key_auth()).unwrap();
        assert_eq!(check_windows_readable(&p).unwrap_err(), "UNSUPPORTED: windows_not_admin");
        let a = parse_windows_probe(&raw("1", "adm", CONN), key_auth()).unwrap();
        assert!(check_windows_readable(&a).is_ok());
    }

    // PS 5.1 ignores native exit codes; without the throw a failed sshd -T would parse as empty settings.
    #[test]
    fn windows_view_script_checks_exit_code_and_quotes_spec() {
        let s = windows_sshd_view_script("user=o'x,host=h");
        assert!(s.contains("-T -C 'user=o''x,host=h'"));
        assert!(s.contains("$LASTEXITCODE -ne 0") && s.contains("throw"));
        assert_eq!(s.matches("---SSHBOX-PART---").count(), 3);
        assert!(s.contains(WINDOWS_SSHD_CONFIG) && s.contains("sshbox-rollback"));
    }

    // The accounts script must keep one-element arrays as arrays and survive an unreadable keys file.
    #[test]
    fn windows_accounts_script_is_array_safe_and_per_file_guarded() {
        let s = WINDOWS_ACCOUNTS_READ_SCRIPT;
        assert!(s.contains("ConvertTo-Json -InputObject @($rows) -Depth 4"));
        assert!(s.contains("catch { }") && s.contains("administrators_authorized_keys"));
    }

    const ACC: &str = r#"{"name":"sshbox","enabled":true,"isAdmin":true,"keysFile":"C:\\ProgramData\\ssh\\administrators_authorized_keys","keys":["ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHkeyone me@pc"]}"#;
    const STD: &str = r#"{"name":"sshbox-std","enabled":false,"isAdmin":false,"keysFile":"C:\\Users\\sshbox-std\\.ssh\\authorized_keys","keys":[]}"#;

    // The UI shows Windows admins as sudo, disabled accounts as locked, and where the keys came from.
    #[test]
    fn windows_accounts_map_admin_keys_file_and_disabled() {
        let s = parse_windows_accounts(&format!("[{ACC},{STD}]")).unwrap();
        let (a, b) = (&s.accounts[0], &s.accounts[1]);
        assert!(a.has_sudo && !a.locked && a.keys_file.ends_with("administrators_authorized_keys"));
        assert_eq!((a.authorized_keys.len(), a.groups.clone()), (1, vec!["Administrators".to_string()]));
        assert!(!b.has_sudo && b.locked && b.authorized_keys.is_empty() && b.keys_file.contains(r"sshbox-std\.ssh"));
    }

    // PS 5.1 drops the array brackets for a single account; the list must still load.
    #[test]
    fn windows_accounts_accept_single_object_json() {
        assert_eq!(parse_windows_accounts(ACC).unwrap().accounts.len(), 1);
        assert!(parse_windows_accounts("").unwrap().accounts.is_empty());
        assert!(parse_windows_accounts("not json").unwrap_err().starts_with("COMMAND_FAILED"));
    }
}
