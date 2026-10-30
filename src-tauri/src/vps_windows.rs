use crate::host_store::HostEntry;
use crate::ssh_pool::{AuthUsed, SshPool};
use crate::vps_config::{
    classify_privileged_output, find_remote_host, parse_authorized_key_line, parse_ssh_connection, parse_sshd_view,
    compute_socket_exposure_windows, is_anywhere_source, is_public_bind_address, vps_err, AccessKeysSnapshot,
    ConnectionContext, FirewallEngine, FirewallRuleItem, FirewallSnapshot, LinuxAccountItem, ListeningSocketItem,
    PrivilegeLevel, SshdView, VpsProbe, WriteTarget, ERR_COMMAND_FAILED, ERR_UNSUPPORTED, SSHD_PART,
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

pub const WINDOWS_RULE_GROUP: &str = "ssh-box";

// $ErrorActionPreference is Stop (bootstrap), so a failing cmdlet fails the read instead of showing a false "safe" state.
// Only the process-name lookup is lenient: a process can exit between listing the socket and naming it.
const WINDOWS_FIREWALL_READ_SCRIPT: &str = r#"$profiles = @(Get-NetFirewallProfile | ForEach-Object { [ordered]@{ name = [string]$_.Name; enabled = ([string]$_.Enabled -eq 'True'); defaultInbound = [string]$_.DefaultInboundAction } })
$active = @(Get-NetConnectionProfile | ForEach-Object { [string]$_.NetworkCategory -replace 'Authenticated','' } | Select-Object -Unique)
$ports = @{}; Get-NetFirewallPortFilter -All | ForEach-Object { $ports[$_.InstanceID] = $_ }
$addrs = @{}; Get-NetFirewallAddressFilter -All | ForEach-Object { $addrs[$_.InstanceID] = $_ }
$apps = @{}; Get-NetFirewallApplicationFilter -All | ForEach-Object { $apps[$_.InstanceID] = $_ }
$svcs = @{}; Get-NetFirewallServiceFilter -All | ForEach-Object { $svcs[$_.InstanceID] = $_ }
$ifs = @{}; Get-NetFirewallInterfaceTypeFilter -All | ForEach-Object { $ifs[$_.InstanceID] = $_ }
$rules = @(Get-NetFirewallRule -Direction Inbound -Enabled True | ForEach-Object {
  $p = $ports[$_.InstanceID]; $a = $addrs[$_.InstanceID]
  if ($p -and $a -and (@($p.LocalPort) -join ',') -ne 'Any') {
    $scoped = ([string]$apps[$_.InstanceID].Program -ne 'Any') -or ([string]$svcs[$_.InstanceID].Service -ne 'Any') -or ((@($a.LocalAddress) -join ',') -ne 'Any') -or ([string]$ifs[$_.InstanceID].InterfaceType -ne 'Any')
    [ordered]@{ displayName = [string]$_.DisplayName; group = [string]$_.Group; profile = [string]$_.Profile; scoped = [bool]$scoped; action = [string]$_.Action; localPort = (@($p.LocalPort) -join ','); protocol = [string]$p.Protocol; remoteAddress = (@($a.RemoteAddress) -join ',') }
  }
})
$names = @{}; Get-Process -ErrorAction SilentlyContinue | ForEach-Object { $names[[int]$_.Id] = $_.ProcessName }
$sockets = @(
  Get-NetTCPConnection -State Listen | ForEach-Object { [ordered]@{ protocol = 'tcp'; addr = [string]$_.LocalAddress; port = [int]$_.LocalPort; pid = [int]$_.OwningProcess; process = [string]$names[[int]$_.OwningProcess] } }
  Get-NetUDPEndpoint | ForEach-Object { [ordered]@{ protocol = 'udp'; addr = [string]$_.LocalAddress; port = [int]$_.LocalPort; pid = [int]$_.OwningProcess; process = [string]$names[[int]$_.OwningProcess] } }
)
ConvertTo-Json -InputObject ([ordered]@{ profiles = $profiles; active = $active; rules = $rules; sockets = $sockets }) -Depth 5 -Compress
"#;

pub fn windows_firewall_read_script() -> String {
    WINDOWS_FIREWALL_READ_SCRIPT.to_string()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WinProfile {
    name: String,
    enabled: bool,
    #[serde(default)]
    default_inbound: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WinRule {
    display_name: String,
    #[serde(default)]
    group: String,
    #[serde(default)]
    profile: String,
    // Program, service, local-address or interface-type scoped: not a plain port rule.
    #[serde(default)]
    scoped: bool,
    action: String,
    local_port: String,
    #[serde(default)]
    protocol: String,
    #[serde(default)]
    remote_address: String,
}

#[derive(Deserialize)]
struct WinSocket {
    protocol: String,
    addr: String,
    port: u16,
    pid: Option<u32>,
    #[serde(default)]
    process: String,
}

#[derive(Deserialize)]
struct WinFirewall {
    profiles: Vec<WinProfile>,
    #[serde(default)]
    active: Vec<String>,
    #[serde(default)]
    rules: Vec<WinRule>,
    #[serde(default)]
    sockets: Vec<WinSocket>,
}

pub fn parse_windows_firewall(json: &str) -> Result<FirewallSnapshot, String> {
    let w: WinFirewall = serde_json::from_str(json.trim())
        .map_err(|e| vps_err(ERR_COMMAND_FAILED, format!("cannot read the firewall: {e}")))?;
    // Without any detectable active profile, all profiles count rather than none.
    let in_use = |p: &&WinProfile| w.active.is_empty() || w.active.iter().any(|a| a.eq_ignore_ascii_case(&p.name));
    let enabled = w.profiles.iter().filter(in_use).any(|p| p.enabled);
    let blocks = w.profiles.iter().filter(in_use).any(|p| p.default_inbound.eq_ignore_ascii_case("block"));
    let applies = |profile: &str| {
        let p = profile.trim();
        w.active.is_empty()
            || p.is_empty()
            || p.eq_ignore_ascii_case("any")
            || p.split(',').any(|x| w.active.iter().any(|a| a.eq_ignore_ascii_case(x.trim())))
    };
    // Every rule is listed; only plain port rules of an active profile take part in the exposure, the rest leave the port "unknown".
    let mut rules = Vec::new();
    for r in &w.rules {
        let protocol = match r.protocol.to_ascii_lowercase().as_str() {
            "tcp" => "tcp",
            "udp" => "udp",
            _ => "any",
        };
        // One item per address (as firewalld twins) so the lockout guard sees single addresses/CIDRs and the right family.
        let mut sources: Vec<(String, bool)> = Vec::new();
        for tok in r.remote_address.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            if is_anywhere_source(tok) {
                sources.extend([("Anywhere".to_string(), false), ("Anywhere".to_string(), true)]);
            } else {
                sources.push((tok.to_string(), tok.contains(':')));
            }
        }
        if sources.is_empty() {
            sources.extend([("Anywhere".to_string(), false), ("Anywhere".to_string(), true)]);
        }
        for (from_source, v6) in sources {
            let item = FirewallRuleItem {
                index: rules.len() as u32 + 1,
                to_port: r.local_port.clone(),
                protocol: protocol.to_string(),
                action: if r.action.eq_ignore_ascii_case("allow") { "ALLOW" } else { "DENY" }.to_string(),
                from_source,
                v6,
                comment: r.group.clone(),
                raw_spec: r.display_name.clone(),
                effective: !r.scoped && applies(&r.profile),
            };
            rules.push(item);
        }
    }
    let counted: Vec<FirewallRuleItem> = rules.iter().filter(|r| r.effective).cloned().collect();
    let listening_sockets = w
        .sockets
        .iter()
        .map(|s| {
            let mut item = ListeningSocketItem {
                protocol: s.protocol.to_ascii_lowercase(),
                bind_addr: s.addr.clone(),
                port: s.port,
                is_public: is_public_bind_address(&s.addr),
                process_name: s.process.clone(),
                pid: s.pid,
                exposure: String::new(),
            };
            item.exposure = compute_socket_exposure_windows(&item, enabled, &counted);
            item
        })
        .collect();
    Ok(FirewallSnapshot {
        engine: "windows".to_string(),
        enabled,
        default_incoming: if blocks { "deny" } else { "allow" }.to_string(),
        default_outgoing: "allow".to_string(),
        ipv6_filtered: true,
        rules,
        listening_sockets,
    })
}

// Every rule ssh-box writes carries the group, so deletes can be limited to rules it created.
fn new_rule_cmd(action: &str, source: &str, port: &str, name_proto: &str, proto: &str) -> String {
    let src = if source == "any" { "Any".to_string() } else { ps_quote(source) };
    let name = format!("{WINDOWS_RULE_GROUP}: {} {port}/{name_proto} from {source}", action.to_ascii_uppercase());
    let ports = if port == "any" {
        String::new()
    } else {
        let list: Vec<String> = port.split(',').map(|p| ps_quote(&p.replace(':', "-"))).collect();
        format!(" -LocalPort @({})", list.join(","))
    };
    let proto_arg = if port == "any" && proto == "any" { "Any" } else { &proto.to_ascii_uppercase() };
    let act = if action == "allow" { "Allow" } else { "Block" };
    format!(
        "New-NetFirewallRule -DisplayName {} -Group {} -Direction Inbound -Action {act} -RemoteAddress {src} -Protocol {proto_arg}{ports} | Out-Null\n",
        ps_quote(&name),
        ps_quote(WINDOWS_RULE_GROUP)
    )
}

pub fn build_windows_add_rule_script(action: &str, source: &str, port: &str, proto: &str) -> Result<String, String> {
    let a = crate::vps_config::validate_rule_args(action, source, port, proto)?;
    if a.port == "any" {
        return Err(vps_err(ERR_UNSUPPORTED, "windows_port_required"));
    }
    let act = match a.action {
        "allow" => "allow",
        "deny" | "reject" => "deny",
        _ => return Err(vps_err(ERR_UNSUPPORTED, "windows_action_unsupported")),
    };
    // Ports need a concrete protocol: "any" becomes one rule per protocol, sharing the DisplayName.
    let protos: &[&str] = if a.proto == "any" && a.port != "any" { &["tcp", "udp"] } else { &[a.proto] };
    Ok(protos.iter().map(|p| new_rule_cmd(act, &a.source, &a.port, a.proto, p)).collect())
}

// Deletes by group + DisplayName; "found nothing" must throw because a native non-zero exit does not stop PowerShell 5.1.
pub fn build_windows_delete_rule_script(rule: &FirewallRuleItem) -> Result<String, String> {
    if rule.comment != WINDOWS_RULE_GROUP {
        return Err(vps_err(ERR_UNSUPPORTED, "windows_rule_not_managed"));
    }
    Ok(format!(
        "$r = @(Get-NetFirewallRule -Group {} -ErrorAction Stop | Where-Object {{ $_.DisplayName -ceq {} }})\nif ($r.Count -eq 0) {{ throw 'rule not found' }}\n$r | Remove-NetFirewallRule\n",
        ps_quote(WINDOWS_RULE_GROUP),
        ps_quote(&rule.raw_spec)
    ))
}

// Enabling first creates the SSH allow, otherwise the profile switch can cut the current session.
// Only rules that really apply count; an ineffective allow would suppress the auto-allow and enabling would lock us out.
pub fn windows_ensure_port(ctx: &ConnectionContext, rules: &[FirewallRuleItem]) -> Option<u16> {
    let effective: Vec<FirewallRuleItem> = rules.iter().filter(|r| r.effective).cloned().collect();
    crate::vps_guard::ssh_allow_needed(ctx, &effective).then_some(ctx.server_port)
}

pub fn build_windows_firewall_state_script(enable: bool, ensure_port: Option<u16>) -> String {
    let mut s = String::new();
    if let (true, Some(p)) = (enable, ensure_port) {
        s.push_str(&new_rule_cmd("allow", "any", &p.to_string(), "tcp", "tcp"));
    }
    s.push_str(&format!(
        "Set-NetFirewallProfile -Profile Domain,Private,Public -Enabled {}\n",
        if enable { "True" } else { "False" }
    ));
    s
}

// Write, then the fresh snapshot in the same run (same marker as the Linux path).
pub async fn run_windows_firewall_write(host_id: &str, pool: &SshPool, write: &str) -> Result<FirewallSnapshot, String> {
    let script = format!("{write}Write-Output 'SSHBOX_SNAPSHOT_BEGIN'\n{}", windows_firewall_read_script());
    let raw = run_windows_script(host_id, pool, &script, None).await?;
    parse_windows_firewall(raw.rsplit("SSHBOX_SNAPSHOT_BEGIN").next().unwrap_or(&raw))
}

pub async fn get_windows_firewall(host_id: &str, pool: &SshPool, probe: &VpsProbe) -> Result<FirewallSnapshot, String> {
    check_windows_readable(probe)?;
    parse_windows_firewall(&run_windows_script(host_id, pool, &windows_firewall_read_script(), None).await?)
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

    const FW: &str = r#"{"profiles":[{"name":"Domain","enabled":false,"defaultInbound":"Block"},{"name":"Private","enabled":true,"defaultInbound":"Block"}],"active":["Private"],"rules":[{"displayName":"OpenSSH-Server-In-TCP","group":"ssh-box","action":"Allow","localPort":"22","protocol":"TCP","remoteAddress":"Any"},{"displayName":"Cho phép Đạt","group":"","action":"Block","localPort":"8080","protocol":"UDP","remoteAddress":"10.0.0.0/8"}],"sockets":[{"protocol":"tcp","addr":"::","port":22,"pid":1234,"process":"sshd"},{"protocol":"tcp","addr":"127.0.0.1","port":5432,"pid":null,"process":""}]}"#;

    // Rule "Any" phải thành hai mục v4/v6, tên rule và nhóm phải qua được để Task 9 xoá theo tên.
    #[test]
    fn windows_firewall_parses_rules_profiles_and_sockets() {
        let s = parse_windows_firewall(FW).unwrap();
        assert_eq!((s.engine.as_str(), s.enabled, s.default_incoming.as_str(), s.ipv6_filtered), ("windows", true, "deny", true));
        let ssh: Vec<_> = s.rules.iter().filter(|r| r.raw_spec == "OpenSSH-Server-In-TCP").collect();
        assert_eq!(ssh.len(), 2);
        assert!(ssh[0].v6 != ssh[1].v6 && ssh[0].action == "ALLOW" && ssh[0].protocol == "tcp");
        assert_eq!((ssh[0].comment.as_str(), ssh[0].from_source.as_str(), ssh[0].to_port.as_str()), (WINDOWS_RULE_GROUP, "Anywhere", "22"));
        let blk = s.rules.iter().find(|r| r.action == "DENY").unwrap();
        assert_eq!((blk.protocol.as_str(), blk.from_source.as_str()), ("udp", "10.0.0.0/8"));
        assert_eq!(s.listening_sockets[0].exposure, "exposed");
        assert_eq!(s.listening_sockets[1].exposure, "loopback");
    }

    // Máy Windows tiếng Việt có tên rule có dấu; mất dấu thì không xoá/đối chiếu được.
    #[test]
    fn windows_firewall_parses_unicode_display_name() {
        let s = parse_windows_firewall(FW).unwrap();
        assert!(s.rules.iter().any(|r| r.raw_spec == "Cho phép Đạt"));
    }

    // Profile không dùng mà bật không được làm firewall "bật"; tất cả profile đang dùng tắt thì phơi ra ngoài.
    #[test]
    fn windows_firewall_enabled_only_counts_active_profiles() {
        let off = FW.replace(r#""enabled":true"#, r#""enabled":false"#).replace(r#""name":"Domain","enabled":false"#, r#""name":"Domain","enabled":true"#);
        let s = parse_windows_firewall(&off).unwrap();
        assert!(!s.enabled);
        assert_eq!(s.listening_sockets[0].exposure, "exposed");
    }

    fn sock(port: u16) -> ListeningSocketItem {
        ListeningSocketItem { protocol: "tcp".into(), bind_addr: "0.0.0.0".into(), port, is_public: true, process_name: String::new(), pid: None, exposure: String::new() }
    }
    fn wrule(action: &str, port: &str, from: &str) -> FirewallRuleItem {
        FirewallRuleItem { index: 1, to_port: port.into(), protocol: "tcp".into(), action: action.into(), from_source: from.into(), v6: false, comment: String::new(), raw_spec: format!("{action} {port}"), effective: true }
    }

    // ufw: luật khớp đầu tiên thắng. Windows: block luôn thắng dù đứng sau allow.
    #[test]
    fn allow_then_block_same_port_differs_between_ufw_and_windows() {
        let rules = [wrule("ALLOW", "80", "Anywhere"), wrule("DENY", "80", "Anywhere")];
        assert_eq!(crate::vps_config::compute_socket_exposure(&sock(80), true, "deny", true, &rules), "exposed");
        assert_eq!(compute_socket_exposure_windows(&sock(80), true, &rules), "blocked");
    }

    // Cổng không có rule theo cổng nào thì không được báo "blocked" sai; phải là unknown. Nguồn cụ thể thì restricted.
    #[test]
    fn port_without_port_rule_is_unknown_on_windows() {
        assert_eq!(compute_socket_exposure_windows(&sock(9000), true, &[wrule("ALLOW", "80", "Anywhere")]), "unknown");
        assert_eq!(compute_socket_exposure_windows(&sock(80), true, &[wrule("ALLOW", "80", "10.0.0.0/8")]), "restricted");
        assert_eq!(compute_socket_exposure_windows(&sock(80), false, &[]), "exposed");
    }

    // Script đọc phải ghép bộ lọc một lần (PS 5.1 chậm) và không nuốt lỗi cả danh mục.
    #[test]
    fn windows_firewall_script_joins_filters_once_and_fails_loudly() {
        let s = windows_firewall_read_script();
        assert!(s.contains("Get-NetFirewallApplicationFilter -All") && s.contains("Get-NetFirewallInterfaceTypeFilter -All") && s.contains("profile = [string]$_.Profile"));
        assert!(s.contains("Get-NetFirewallPortFilter -All") && s.contains("Get-NetFirewallAddressFilter -All"));
        assert!(s.contains("Get-NetTCPConnection -State Listen") && s.contains("Get-NetUDPEndpoint"));
        assert!(s.contains("ConvertTo-Json") && s.contains("-Compress"));
        assert!(!s.contains("Get-NetFirewallRule -ErrorAction SilentlyContinue"));
        assert_eq!(s.matches("SilentlyContinue").count(), 1); // chỉ tra tên tiến trình
    }

    // JSON hỏng phải ra COMMAND_FAILED, không phải một danh sách rỗng trông như "an toàn".
    #[test]
    fn windows_firewall_parse_error_is_command_failed() {
        assert!(parse_windows_firewall("not json").unwrap_err().starts_with("COMMAND_FAILED"));
    }

    // Luật Block của profile Public không chặn cổng khi máy đang dùng Private; không được báo "blocked".
    #[test]
    fn windows_rule_of_inactive_profile_does_not_count() {
        let j = FW.replace(r#""remoteAddress":"Any"}"#, r#""remoteAddress":"Any"},{"displayName":"B","group":"","profile":"Public","action":"Block","localPort":"22","protocol":"TCP","remoteAddress":"Any"}"#);
        assert_eq!(parse_windows_firewall(&j).unwrap().listening_sockets[0].exposure, "exposed");
        let j = j.replace(r#""profile":"Public""#, r#""profile":"Private""#);
        assert_eq!(parse_windows_firewall(&j).unwrap().listening_sockets[0].exposure, "blocked");
    }

    // Luật gắn với chương trình không phải luật cổng thuần; không được tạo "blocked" mà cổng suy biến về unknown.
    #[test]
    fn windows_program_scoped_rule_degrades_to_unknown() {
        let j = FW.replace(r#""displayName":"OpenSSH-Server-In-TCP""#, r#""displayName":"OpenSSH-Server-In-TCP","scoped":true"#).replace(r#""action":"Allow""#, r#""action":"Block""#);
        let s = parse_windows_firewall(&j).unwrap();
        assert_eq!(s.listening_sockets[0].exposure, "unknown");
        assert!(s.rules.iter().any(|r| r.raw_spec == "OpenSSH-Server-In-TCP"));
    }

    // Mỗi địa chỉ nguồn phải thành mục riêng đúng họ v4/v6, nếu không guard khoá không thấy allow của máy khách IPv6.
    #[test]
    fn windows_remote_addresses_split_per_address_and_family() {
        let j = FW.replace(r#""remoteAddress":"10.0.0.0/8""#, r#""remoteAddress":"10.0.0.0/8,2001:db8::/64,LocalSubnet""#);
        let s = parse_windows_firewall(&j).unwrap();
        let got: Vec<_> = s.rules.iter().filter(|r| r.raw_spec == "Cho phép Đạt").map(|r| (r.from_source.as_str(), r.v6)).collect();
        assert_eq!(got, vec![("10.0.0.0/8", false), ("2001:db8::/64", true), ("LocalSubnet", false)]);
    }

    // Allow từ mọi nơi cộng block một nguồn: mọi người trừ nguồn đó vẫn vào được, nên phải là exposed.
    #[test]
    fn windows_allow_any_with_one_blocked_source_is_exposed() {
        let rules = [wrule("ALLOW", "80", "Anywhere"), wrule("DENY", "80", "198.51.100.9")];
        assert_eq!(compute_socket_exposure_windows(&sock(80), true, &rules), "exposed");
    }

    fn win_rule(comment: &str) -> FirewallRuleItem {
        FirewallRuleItem {
            index: 1, to_port: "22".into(), protocol: "tcp".into(), action: "ALLOW".into(), from_source: "Anywhere".into(),
            v6: false, comment: comment.into(), raw_spec: "ssh-box: ALLOW 22/tcp from any".into(), effective: true,
        }
    }

    // Mọi giá trị người dùng đưa vào PowerShell phải được quote và rule phải nằm trong nhóm ssh-box (để xoá được sau này).
    #[test]
    fn windows_add_rule_quotes_and_groups() {
        let s = build_windows_add_rule_script("deny", "10.0.0.1'; calc", "22", "tcp");
        assert!(s.is_err(), "source with quote must be rejected by validation");
        let s = build_windows_add_rule_script("deny", "10.0.0.0/8", "22", "tcp").unwrap();
        assert!(s.contains("-Group 'ssh-box'") && s.contains("-Action Block") && s.contains("-RemoteAddress '10.0.0.0/8'"));
        assert!(build_windows_add_rule_script("allow", "any", "22;calc", "tcp").is_err());
    }

    // Port mà proto = any phải mở cả TCP lẫn UDP, nếu không rule chỉ chặn/mở một nửa.
    #[test]
    fn windows_add_any_proto_creates_tcp_and_udp() {
        let s = build_windows_add_rule_script("allow", "any", "53", "any").unwrap();
        assert_eq!(s.matches("New-NetFirewallRule").count(), 2);
        assert!(s.contains("-Protocol TCP") && s.contains("-Protocol UDP"));
    }

    // Chỉ xoá rule do ssh-box tạo; rule hệ thống như OpenSSH-Server-In-TCP không được đụng tới.
    #[test]
    fn windows_delete_refuses_unmanaged_rule() {
        assert_eq!(build_windows_delete_rule_script(&win_rule("")), Err("UNSUPPORTED: windows_rule_not_managed".to_string()));
        let s = build_windows_delete_rule_script(&win_rule("ssh-box")).unwrap();
        assert!(s.contains("throw") && s.contains("-Group 'ssh-box'") && s.contains("-ceq 'ssh-box: ALLOW 22/tcp from any'"));
        assert!(!s.contains("-DisplayName"), "DisplayName parameter set is wildcard and cannot mix with -Group");
    }

    // Bật firewall mà chưa có rule allow SSH sẽ tự cắt kết nối; rule phải có trước lệnh bật.
    #[test]
    fn windows_enable_adds_ssh_allow_before_enabling() {
        let s = build_windows_firewall_state_script(true, Some(22));
        assert!(s.find("New-NetFirewallRule").unwrap() < s.find("Set-NetFirewallProfile").unwrap());
        assert!(s.contains("-Enabled True"));
        let off = build_windows_firewall_state_script(false, Some(22));
        assert!(!off.contains("New-NetFirewallRule") && off.contains("-Enabled False"));
    }

    // Chặn mọi nguồn trên cổng SSH đang dùng là tự khoá mình; phải bị bộ chặn từ chối trước khi tới PowerShell.
    #[test]
    fn windows_block_covering_me_is_blocked() {
        let ctx = parse_windows_probe(&raw("1", "admin", CONN), key_auth()).unwrap().context;
        assert_eq!(
            crate::vps_guard::check_firewall_add(&ctx, "deny", "Anywhere", "22"),
            Err("LOCKOUT_BLOCKED: closes_ssh_for_everyone".to_string())
        );
    }

    // Rule without a port is invisible to the read script, so it could open every port unseen; Windows must demand a port.
    #[test]
    fn windows_add_requires_a_port() {
        for port in ["", "any"] {
            assert_eq!(build_windows_add_rule_script("allow", "any", port, "tcp"), Err("UNSUPPORTED: windows_port_required".to_string()));
        }
    }

    // An allow that does not apply (other profile) must not suppress the auto-allow, or enabling cuts SSH.
    #[test]
    fn windows_enable_ignores_ineffective_ssh_allow() {
        let ctx = parse_windows_probe(&raw("1", "admin", CONN), key_auth()).unwrap().context;
        let mut r = win_rule("");
        r.effective = false;
        assert_eq!(windows_ensure_port(&ctx, &[r.clone()]), Some(22));
        r.effective = true;
        assert_eq!(windows_ensure_port(&ctx, &[r]), None);
    }
}
