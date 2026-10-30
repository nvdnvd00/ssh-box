use crate::host_store::HostEntry;
use crate::ssh_pool::{AuthUsed, SshPool};
use crate::vps_config::{
    classify_privileged_output, find_remote_host, parse_ssh_connection, vps_err, ConnectionContext,
    FirewallEngine, PrivilegeLevel, VpsProbe, ERR_COMMAND_FAILED, ERR_UNSUPPORTED,
};
use crate::vps_support::{compute_support, OsCaps, OsFamily, VpsSupport};
use base64::Engine;

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
}
