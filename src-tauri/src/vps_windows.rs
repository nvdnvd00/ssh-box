use crate::host_store::HostEntry;
use crate::ssh_pool::{AuthUsed, SshPool};
use crate::vps_config::{
    classify_privileged_output, find_remote_host, parse_authorized_key_line, parse_ssh_connection, parse_sshd_view, rule_port_matches,
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

// The longest script (sshd apply with its 15s sshd -t bound, restarts and rollback) needs well under this.
const WINDOWS_EXEC_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(120);

// Seen once on the VM matrix: the bootstrap sat on stdin for 10+ minutes before running anything. Never wait forever.
async fn bounded<T>(run: impl std::future::Future<Output = Result<T, String>>, limit: std::time::Duration) -> Result<T, String> {
    tokio::time::timeout(limit, run).await.unwrap_or_else(|_| {
        Err(vps_err(
            ERR_COMMAND_FAILED,
            format!("the Windows host did not answer within {}s; if a change was being applied it is rolled back automatically within 60s", limit.as_secs()),
        ))
    })
}

async fn exec_windows(pool: &SshPool, host: &HostEntry, script: &str, secret: Option<&str>) -> Result<Vec<u8>, String> {
    let stdin = powershell_stdin(script, secret);
    bounded(pool.exec_bytes_on_host(host, &build_powershell_command(), Some(&stdin)), WINDOWS_EXEC_TIMEOUT).await
}

pub async fn run_windows_script(
    host_id: &str,
    pool: &SshPool,
    script: &str,
    secret: Option<&str>,
) -> Result<String, String> {
    let host = find_remote_host(host_id)?;
    let out = exec_windows(pool, &host, script, secret).await?;
    classify_privileged_output(&String::from_utf8_lossy(&out))
}

pub const WINDOWS_PROBE_SCRIPT: &str = r#"Write-Output 'OS_FAMILY:windows'
# A standard account gets "Access denied" from CIM; OS_PRODUCT then names the OS.
Write-Output ('OS_CAPTION:' + (Get-CimInstance Win32_OperatingSystem -ErrorAction SilentlyContinue).Caption)
$cv = Get-ItemProperty 'HKLM:\SOFTWARE\Microsoft\Windows NT\CurrentVersion'
Write-Output ('OS_DISPLAY_VERSION:' + $cv.DisplayVersion)
Write-Output ('OS_PRODUCT:' + $cv.ProductName)
Write-Output ('OS_BUILD:' + $cv.CurrentBuildNumber)
Write-Output ('USER:' + [Environment]::UserName)
$admin = ([Security.Principal.WindowsPrincipal][Security.Principal.WindowsIdentity]::GetCurrent()).IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)
Write-Output ('ADMIN:' + [int]$admin)
Write-Output ('OPENSSH:' + (Get-Item "$env:WINDIR\System32\OpenSSH\sshd.exe" -ErrorAction SilentlyContinue).VersionInfo.ProductVersion)
Write-Output ('SSHD_SERVICE:' + (Get-Service sshd -ErrorAction SilentlyContinue).Status)
Write-Output ('SSHD_EXE:' + [int](Test-Path "$env:WINDIR\System32\OpenSSH\sshd.exe"))
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
    let mut caption = field("OS_CAPTION:");
    // A standard account reads no CIM Caption; ProductName says "Windows 10" on 11 too, so build 22000+ names 11.
    if caption.is_empty() {
        caption = field("OS_PRODUCT:");
        if field("OS_BUILD:").parse::<u32>().is_ok_and(|b| b >= 22000) {
            caption = caption.replacen("Windows 10", "Windows 11", 1);
        }
    }
    let name = caption.strip_prefix("Microsoft ").unwrap_or(&caption);
    let os_version = format!("{} {}", name.strip_prefix("Windows ").unwrap_or(name), field("OS_DISPLAY_VERSION:"));
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
            // Every SSH script runs the inbox sshd.exe, so a running service elsewhere does not count.
            sshd_running: field("SSHD_SERVICE:") == "Running" && field("SSHD_EXE:") == "1",
            has_scheduled_tasks: field("SCHEDTASKS:") == "1",
            has_net_security: net_security,
            ..Default::default()
        },
    };
    probe.support = compute_support(&probe);
    Ok(probe)
}

pub const WINDOWS_SSHD_CONFIG: &str = r"C:\ProgramData\ssh\sshd_config";
// This OpenSSH build never exits on `sshd -t` of an invalid config, so every sshd.exe call is bounded and returns {Code, Out, Err}.
const SSHD_HELPER: &str = r#"function sshbox_sshd([string]$argline) {
  $psi = New-Object Diagnostics.ProcessStartInfo
  $psi.FileName = "$env:WINDIR\System32\OpenSSH\sshd.exe"; $psi.Arguments = $argline
  $psi.UseShellExecute = $false; $psi.CreateNoWindow = $true; $psi.RedirectStandardOutput = $true; $psi.RedirectStandardError = $true
  $p = [Diagnostics.Process]::Start($psi)
  $o = $p.StandardOutput.ReadToEndAsync(); $e = $p.StandardError.ReadToEndAsync()
  if (-not $p.WaitForExit(15000)) { try { $p.Kill() } catch { }; [void]$p.WaitForExit(3000); $why = if ($e.Wait(1000)) { $e.Result.Trim() } else { '' }; return [pscustomobject]@{ Code = -1; Out = ''; Err = ($why + "`nsshd timed out after 15s").Trim() } }
  [pscustomobject]@{ Code = $p.ExitCode; Out = $o.Result; Err = $e.Result }
}
"#;

// PS 5.1 does not throw when a native command fails, so $LASTEXITCODE is checked and thrown by hand.
pub fn windows_sshd_view_script(spec: &str) -> String {
    format!(
        r#"{SSHD_HELPER}$r = sshbox_sshd ('-T -C "' + {spec} + '"')
if ($r.Code -ne 0) {{ throw "sshd -T failed with exit code $($r.Code): $($r.Err)" }}
$r.Out
'{SSHD_PART}'
Get-Content -Raw -Encoding UTF8 -LiteralPath '{WINDOWS_SSHD_CONFIG}'
'{SSHD_PART}'
'{SSHD_PART}'
if ((Get-Command Get-ScheduledTask -ErrorAction SilentlyContinue) -and (Get-ScheduledTask -TaskName sshbox-rollback -ErrorAction SilentlyContinue)) {{ 'active' }} else {{ 'inactive' }}
"#,
        spec = ps_quote(spec)
    )
}

pub const WINDOWS_ROLLBACK_TASK: &str = "sshbox-rollback";
pub const WINDOWS_ROLLBACK_SCRIPT: &str = r"C:\ProgramData\ssh\sshbox-rollback.ps1";

// Same marker files as Linux: .bak is moved back over the config, .absent means "delete it".
fn windows_restore_fn() -> String {
    format!(
        r#"$F = '{WINDOWS_SSHD_CONFIG}'
function sshbox_restore {{
  if (Test-Path -LiteralPath "$F.sshbox.bak") {{ Move-Item -LiteralPath "$F.sshbox.bak" -Destination $F -Force }}
  elseif (Test-Path -LiteralPath "$F.sshbox.absent") {{ Remove-Item -LiteralPath $F, "$F.sshbox.absent" -Force -ErrorAction SilentlyContinue }}
}}
"#
    )
}

// The child's stderr must not abort the parent under EAP=Stop, so judge it by exit code.
fn windows_run_rollback_cmd() -> String {
    format!(
        "$ea = $ErrorActionPreference; $ErrorActionPreference = 'Continue'; & powershell.exe -NoProfile -ExecutionPolicy Bypass -File '{WINDOWS_ROLLBACK_SCRIPT}' 2>&1 | Out-Null; $rbrc = $LASTEXITCODE; $ErrorActionPreference = $ea; if ($rbrc -ne 0) {{ 'SSHBOX_ROLLBACK_FAILED' }}"
    )
}

// Runs as SYSTEM 60s after arming (or by "Roll back now"). Unregistering is last: it may end the task running this script.
pub fn build_sshd_rollback_script_windows(new_port: Option<u16>) -> String {
    // Same DisplayName that new_rule_cmd gives an "allow <port>/tcp from any" rule.
    let rule = new_port.map_or(String::new(), |p| {
        format!(
            "Get-NetFirewallRule -Group {} -ErrorAction SilentlyContinue | Where-Object {{ $_.DisplayName -ceq {} }} | Remove-NetFirewallRule\n",
            ps_quote(WINDOWS_RULE_GROUP),
            ps_quote(&format!("{WINDOWS_RULE_GROUP}: ALLOW {p}/tcp from any"))
        )
    });
    format!(
        "$ErrorActionPreference = 'Stop'\n$ok = $true\n{}try {{ sshbox_restore }} catch {{ $ok = $false }}\nRemove-Item -LiteralPath \"$F.sshbox.new\" -Force -ErrorAction SilentlyContinue\ntry {{ Restart-Service sshd }} catch {{ $ok = $false }}\ntry {{ {rule}}} catch {{ $ok = $false }}\nRemove-Item -LiteralPath '{WINDOWS_ROLLBACK_SCRIPT}' -Force -ErrorAction SilentlyContinue\nUnregister-ScheduledTask -TaskName {WINDOWS_ROLLBACK_TASK} -Confirm:$false -ErrorAction SilentlyContinue\nif (-not $ok) {{ exit 1 }}\n",
        windows_restore_fn()
    )
}

// The new config arrives in $SSHBOX_SECRET (LF, no BOM). BOM and line endings of the existing file are put back here (the only layer that does it).
// Order matters: validate a side file first (sshd -t can hang on a bad config), arm the rollback, only then replace the live file.
pub fn build_sshd_apply_script_windows(new_port: Option<u16>, port: u16) -> String {
    let fw = new_port.map_or(String::new(), |p| new_rule_cmd("allow", "any", &p.to_string(), "tcp", "tcp"));
    format!(
        r#"{restore}{SSHD_HELPER}$R = '{WINDOWS_ROLLBACK_SCRIPT}'
$N = "$F.sshbox.new"
if (Get-ScheduledTask -TaskName {WINDOWS_ROLLBACK_TASK} -ErrorAction SilentlyContinue) {{ 'SSHBOX_ROLLBACK_PENDING'; throw 'a previous change is still waiting for verification' }}
$old = @()
if (Test-Path -LiteralPath $F) {{ $old = [IO.File]::ReadAllBytes($F) }}
$bom = $old.Length -ge 3 -and $old[0] -eq 0xEF -and $old[1] -eq 0xBB -and $old[2] -eq 0xBF
$eol = if ([Text.Encoding]::UTF8.GetString($old).Contains("`r`n")) {{ "`r`n" }} else {{ "`n" }}
$text = ($SSHBOX_SECRET.TrimStart([char]0xFEFF) -replace '\r?\n', $eol)
if ($bom) {{ $text = [string][char]0xFEFF + $text }}
[IO.File]::WriteAllText($N, $text, (New-Object Text.UTF8Encoding $false))
$v = sshbox_sshd ('-t -f "' + $N + '"')
if ($v.Code -ne 0) {{ Remove-Item -LiteralPath $N -Force -ErrorAction SilentlyContinue; 'SSHBOX_SSHD_INVALID'; ($v.Err -split "`r?`n")[0]; throw "sshd -t failed with exit code $($v.Code)" }}
try {{
  if (Test-Path -LiteralPath $F) {{ Copy-Item -LiteralPath $F -Destination "$F.sshbox.bak" -Force; Remove-Item -LiteralPath "$F.sshbox.absent" -Force -ErrorAction SilentlyContinue }}
  else {{ New-Item -ItemType File -Path "$F.sshbox.absent" -Force | Out-Null; Remove-Item -LiteralPath "$F.sshbox.bak" -Force -ErrorAction SilentlyContinue }}
  [IO.File]::WriteAllText($R, @'
{rollback}'@)
  $act = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument '-NoProfile -ExecutionPolicy Bypass -File "{WINDOWS_ROLLBACK_SCRIPT}"'
  $trg = New-ScheduledTaskTrigger -Once -At (Get-Date).AddSeconds(60)
  $set = New-ScheduledTaskSettingsSet -AllowStartIfOnBatteries -DontStopIfGoingOnBatteries -StartWhenAvailable
  Register-ScheduledTask -TaskName {WINDOWS_ROLLBACK_TASK} -Action $act -Trigger $trg -Settings $set -User SYSTEM -RunLevel Highest -Force | Out-Null
}} catch {{
  Remove-Item -LiteralPath $N, $R, "$F.sshbox.bak", "$F.sshbox.absent" -Force -ErrorAction SilentlyContinue
  Unregister-ScheduledTask -TaskName {WINDOWS_ROLLBACK_TASK} -Confirm:$false -ErrorAction SilentlyContinue
  throw
}}
try {{
  $now = [DateTimeOffset]::UtcNow.ToUnixTimeSeconds()
  "NOW:$now"
  "ROLLBACK_AT:$($now + 60)"
  Move-Item -LiteralPath $N -Destination $F -Force
  {fw}Restart-Service sshd
  $up = $false
  for ($i = 0; $i -lt 10 -and -not $up; $i++) {{ Start-Sleep -Milliseconds 500; $up = [bool](Get-NetTCPConnection -State Listen -LocalPort {port} -ErrorAction SilentlyContinue) }}
  if (-not $up) {{ throw 'SSHBOX_PORT_NOT_LISTENING' }}
}} catch {{ {run_rollback}; throw }}
"#,
        restore = windows_restore_fn(),
        rollback = build_sshd_rollback_script_windows(new_port),
        run_rollback = windows_run_rollback_cmd(),
    )
}

// Keeps the change: a missing script means the rollback already ran.
pub fn windows_cleanup_script() -> String {
    format!(
        r#"{restore}Unregister-ScheduledTask -TaskName {WINDOWS_ROLLBACK_TASK} -Confirm:$false -ErrorAction SilentlyContinue
if (-not (Test-Path -LiteralPath '{WINDOWS_ROLLBACK_SCRIPT}')) {{ 'SSHBOX_ALREADY_ROLLED_BACK'; throw 'the change was already rolled back' }}
Remove-Item -LiteralPath "$F.sshbox.bak", "$F.sshbox.absent", '{WINDOWS_ROLLBACK_SCRIPT}' -Force -ErrorAction SilentlyContinue
"#,
        restore = windows_restore_fn()
    )
}

pub fn windows_rollback_now_script() -> String {
    format!(
        "if (Test-Path -LiteralPath '{WINDOWS_ROLLBACK_SCRIPT}') {{ {}; if ($rbrc -ne 0) {{ throw 'the rollback script failed' }} }}\nUnregister-ScheduledTask -TaskName {WINDOWS_ROLLBACK_TASK} -Confirm:$false -ErrorAction SilentlyContinue\n",
        windows_run_rollback_cmd()
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
    // Out-String adds its own newline after the file text; it is not part of the file.
    if let Some(text) = view.main_raw.strip_suffix('\n') {
        view.main_raw = text.to_string();
    }
    Ok(view)
}

pub async fn get_windows_sshd_view(host_id: &str, pool: &SshPool, probe: &VpsProbe) -> Result<SshdView, String> {
    check_windows_readable(probe)?;
    let c = &probe.context;
    let spec = format!("user={},host={},addr={},laddr={},lport={}", c.login_user, c.client_ip, c.client_ip, c.server_ip, c.server_port);
    parse_windows_sshd_view(&run_windows_script(host_id, pool, &windows_sshd_view_script(&spec), None).await?)
}

const WINDOWS_ACCOUNTS_BODY: &str = r#"$admins = @(Get-LocalGroupMember -SID S-1-5-32-544 -ErrorAction SilentlyContinue | ForEach-Object { $_.Name.Split('\')[-1] })
$rows = foreach ($u in Get-LocalUser) {
  $isAdmin = $admins -contains $u.Name
  $file = "C:\Users\$($u.Name)\.ssh\authorized_keys"
  # Only enabled admins need it (disabled built-ins make sshd hang); a failure keeps the default file for that row.
  if ($isAdmin -and $u.Enabled) {
    try {
      $r = sshbox_sshd ('-T -C "user=' + $u.Name + ',host=localhost,addr=127.0.0.1"')
      if ($r.Code -eq 0 -and (@($r.Out -split "`r?`n") -match '^authorizedkeysfile .*administrators_authorized_keys')) { $file = 'C:\ProgramData\ssh\administrators_authorized_keys' }
    } catch { }
  }
  $keys = @()
  # Plain strings: PS 5.1 would serialize each line with its PSPath/PSDrive NoteProperties.
  try { $keys = @(Get-Content -LiteralPath $file -ErrorAction Stop | ForEach-Object { [string]$_ }) } catch { }
  [ordered]@{ name = $u.Name; enabled = [bool]$u.Enabled; isAdmin = [bool]$isAdmin; keysFile = $file; keys = $keys }
}
ConvertTo-Json -InputObject @($rows) -Depth 4 -Compress
"#;

pub fn windows_accounts_read_script() -> String {
    format!("{SSHD_HELPER}{WINDOWS_ACCOUNTS_BODY}")
}

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
    keys: Vec<KeyLine>,
}

// A Get-Content line can arrive as {"value": "...", "PSPath": ...} instead of a string.
#[derive(Deserialize)]
#[serde(untagged)]
enum KeyLine {
    Text(String),
    Object { value: String },
}

impl KeyLine {
    fn text(&self) -> &str {
        match self {
            KeyLine::Text(s) | KeyLine::Object { value: s } => s,
        }
    }
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
            authorized_keys: a.keys.iter().enumerate().filter_map(|(i, l)| parse_authorized_key_line(l.text(), i as u32)).collect(),
            keys_file: a.keys_file,
        })
        .collect();
    Ok(AccessKeysSnapshot { accounts })
}

pub async fn get_windows_accounts(host_id: &str, pool: &SshPool) -> Result<AccessKeysSnapshot, String> {
    parse_windows_accounts(&run_windows_script(host_id, pool, &windows_accounts_read_script(), None).await?)
}

pub const WINDOWS_RULE_GROUP: &str = "ssh-box";

// Reads the effective policy (ActiveStore, Group Policy included) so a GPO-overridden setting shows as it really is;
// writes go to the default persistent store, and the re-read after a write shows whether it took effect.
// $ErrorActionPreference is Stop (bootstrap), so a failing cmdlet fails the read instead of showing a false "safe" state.
// Only the process-name lookup is lenient: a process can exit between listing the socket and naming it.
// The SSH interface's profile lookup is lenient too: unknown ('') only narrows which rules count.
const WINDOWS_FIREWALL_READ_SCRIPT: &str = r#"$profiles = @(Get-NetFirewallProfile -PolicyStore ActiveStore | ForEach-Object { [ordered]@{ name = [string]$_.Name; enabled = ([string]$_.Enabled -eq 'True'); defaultInbound = [string]$_.DefaultInboundAction } })
$active = @(Get-NetConnectionProfile | ForEach-Object { [string]$_.NetworkCategory -replace 'Authenticated','' } | Select-Object -Unique)
$sshProfile = ''
if ($sshIp) { try { $ifx = @(Get-NetIPAddress -IPAddress $sshIp -ErrorAction Stop)[0].InterfaceIndex; $sshProfile = ([string]@(Get-NetConnectionProfile -InterfaceIndex $ifx -ErrorAction Stop)[0].NetworkCategory) -replace 'Authenticated','' } catch { $sshProfile = '' } }
$ports = @{}; Get-NetFirewallPortFilter -All -PolicyStore ActiveStore | ForEach-Object { $ports[$_.InstanceID] = $_ }
$addrs = @{}; Get-NetFirewallAddressFilter -All -PolicyStore ActiveStore | ForEach-Object { $addrs[$_.InstanceID] = $_ }
$apps = @{}; Get-NetFirewallApplicationFilter -All -PolicyStore ActiveStore | ForEach-Object { $apps[$_.InstanceID] = $_ }
$svcs = @{}; Get-NetFirewallServiceFilter -All -PolicyStore ActiveStore | ForEach-Object { $svcs[$_.InstanceID] = $_ }
$ifs = @{}; Get-NetFirewallInterfaceTypeFilter -All -PolicyStore ActiveStore | ForEach-Object { $ifs[$_.InstanceID] = $_ }
$rules = @(Get-NetFirewallRule -PolicyStore ActiveStore -Direction Inbound -Enabled True | ForEach-Object {
  $p = $ports[$_.InstanceID]; $a = $addrs[$_.InstanceID]
  # Port rules are listed. A block without a port filter is returned too: it is not listed, but it beats any allow.
  if ($p -and $a -and (((@($p.LocalPort) -join ',') -ne 'Any') -or ([string]$_.Action -eq 'Block'))) {
    # No application filter read: '' (treated as scoped). Otherwise 'Any' or the program path with %SystemRoot% etc. expanded.
    $app = $apps[$_.InstanceID]; $prog = if ($app) { [Environment]::ExpandEnvironmentVariables([string]$app.Program) } else { '' }
    $scoped = ([string]$svcs[$_.InstanceID].Service -ne 'Any') -or ((@($a.LocalAddress) -join ',') -ne 'Any') -or ([string]$ifs[$_.InstanceID].InterfaceType -ne 'Any')
    [ordered]@{ displayName = [string]$_.DisplayName; group = [string]$_.Group; profile = [string]$_.Profile; program = $prog; scoped = [bool]$scoped; action = [string]$_.Action; localPort = (@($p.LocalPort) -join ','); protocol = [string]$p.Protocol; remoteAddress = (@($a.RemoteAddress) -join ',') }
  }
})
$names = @{}; $paths = @{}; Get-Process -ErrorAction SilentlyContinue | ForEach-Object { $names[[int]$_.Id] = $_.ProcessName; $paths[[int]$_.Id] = [string]$_.Path }
$sockets = @(
  Get-NetTCPConnection -State Listen | ForEach-Object { [ordered]@{ protocol = 'tcp'; addr = [string]$_.LocalAddress; port = [int]$_.LocalPort; pid = [int]$_.OwningProcess; process = [string]$names[[int]$_.OwningProcess]; path = [string]$paths[[int]$_.OwningProcess] } }
  Get-NetUDPEndpoint | ForEach-Object { [ordered]@{ protocol = 'udp'; addr = [string]$_.LocalAddress; port = [int]$_.LocalPort; pid = [int]$_.OwningProcess; process = [string]$names[[int]$_.OwningProcess]; path = [string]$paths[[int]$_.OwningProcess] } }
)
ConvertTo-Json -InputObject ([ordered]@{ profiles = $profiles; active = $active; sshProfile = $sshProfile; rules = $rules; sockets = $sockets }) -Depth 5 -Compress
"#;

// server_ip names the interface the SSH session arrives on; anything but a plain address leaves its profile unknown.
pub fn windows_firewall_read_script(server_ip: &str) -> String {
    let ip = server_ip.trim().parse::<std::net::IpAddr>().map(|ip| ip.to_string()).unwrap_or_default();
    format!("$sshIp = {}\n{WINDOWS_FIREWALL_READ_SCRIPT}", ps_quote(&ip))
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
    // "Any", the expanded program path the rule is limited to, or "" when no application filter was read (scoped).
    #[serde(default)]
    program: String,
    // Service, local-address or interface-type scoped: not a plain port rule.
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
    // Full image path of the owning process; empty when unknown (e.g. PID 4 "System").
    #[serde(default)]
    path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WinFirewall {
    profiles: Vec<WinProfile>,
    #[serde(default)]
    active: Vec<String>,
    // Profile of the interface that owns the SSH server address; "" when it could not be read.
    #[serde(default)]
    ssh_profile: String,
    #[serde(default)]
    rules: Vec<WinRule>,
    #[serde(default)]
    sockets: Vec<WinSocket>,
}

// Windows lists IPv4 subnets as "net/255.255.255.0"; the guard and the UI speak CIDR. Non-contiguous masks stay as they are.
fn netmask_to_prefix(tok: &str) -> String {
    match tok.split_once('/').map(|(n, m)| (n, m.parse::<std::net::Ipv4Addr>())) {
        Some((net, Ok(mask))) if u32::from(mask).leading_ones() + u32::from(mask).trailing_zeros() == 32 => {
            format!("{net}/{}", u32::from(mask).leading_ones())
        }
        _ => tok.to_string(),
    }
}

pub fn parse_windows_firewall(json: &str) -> Result<FirewallSnapshot, String> {
    let w: WinFirewall = serde_json::from_str(json.trim())
        .map_err(|e| vps_err(ERR_COMMAND_FAILED, format!("cannot read the firewall: {e}")))?;
    // Without any detectable active profile, all profiles count rather than none.
    let in_use = |p: &&WinProfile| w.active.is_empty() || w.active.iter().any(|a| a.eq_ignore_ascii_case(&p.name));
    let enabled = w.profiles.iter().filter(in_use).any(|p| p.enabled);
    let blocks = w.profiles.iter().filter(in_use).any(|p| p.default_inbound.eq_ignore_ascii_case("block"));
    // A rule carries (or blocks) this session only on the profile of the interface SSH arrives on: on a multi-NIC host
    // another active profile says nothing about it. Unknown profile: only rules for every profile count.
    let ssh_profile = w.ssh_profile.trim();
    let applies = |profile: &str| {
        let p = profile.trim();
        p.is_empty()
            || p.eq_ignore_ascii_case("any")
            || (!ssh_profile.is_empty() && p.split(',').any(|x| x.trim().eq_ignore_ascii_case(ssh_profile)))
    };
    // A program-scoped rule (the stock OpenSSH one is limited to sshd.exe) applies only while exactly that file owns every
    // socket on its port: another sshd.exe (zip/MSI install, Cygwin) does not, and an unknown listener path never does.
    let program_owns = |r: &WinRule, protocol: &str| {
        let prog = r.program.trim();
        if prog.eq_ignore_ascii_case("any") {
            return true;
        }
        let mut on_port = w.sockets.iter().filter(|s| rule_port_matches(&r.local_port, protocol, s.port, &s.protocol)).peekable();
        !prog.is_empty() && on_port.peek().is_some() && on_port.all(|s| !s.path.is_empty() && s.path.eq_ignore_ascii_case(prog))
    };
    // Every port rule is listed; only plain port rules of the SSH profile take part in the exposure, the rest leave the port "unknown".
    let (mut rules, mut unlisted_blocks) = (Vec::new(), Vec::new());
    for r in &w.rules {
        // ICMPv4/ICMPv6/protocol-number rules (only port-less blocks reach here) never touch a TCP/UDP port: skip them.
        let protocol = match r.protocol.to_ascii_lowercase().as_str() {
            "tcp" => "tcp",
            "udp" => "udp",
            "any" => "any",
            _ => continue,
        };
        // One item per address (as firewalld twins) so the lockout guard sees single addresses/CIDRs and the right family.
        let mut sources: Vec<(String, bool)> = Vec::new();
        for tok in r.remote_address.split(',').map(str::trim).filter(|t| !t.is_empty()) {
            if is_anywhere_source(tok) {
                sources.extend([("Anywhere".to_string(), false), ("Anywhere".to_string(), true)]);
            } else {
                sources.push((netmask_to_prefix(tok), tok.contains(':')));
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
                effective: !r.scoped && applies(&r.profile) && program_owns(r, protocol),
            };
            if r.local_port.trim().eq_ignore_ascii_case("any") {
                unlisted_blocks.push(item);
            } else {
                rules.push(item);
            }
        }
    }
    let counted: Vec<FirewallRuleItem> = rules.iter().chain(&unlisted_blocks).filter(|r| r.effective).cloned().collect();
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
        unlisted_blocks,
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

// Listed rules plus the unlisted any-port blocks: everything that decides whether this session still gets in.
fn guard_rules(snap: &FirewallSnapshot) -> Vec<FirewallRuleItem> {
    snap.rules.iter().chain(&snap.unlisted_blocks).cloned().collect()
}

// Enabling makes every saved block live, and a block beats the auto-allow: refuse while one covers this session.
pub fn build_windows_set_state(ctx: &ConnectionContext, snap: &FirewallSnapshot, enable: bool) -> Result<String, String> {
    if enable {
        crate::vps_guard::check_firewall_enable_windows(ctx, &guard_rules(snap))?;
    }
    Ok(build_windows_firewall_state_script(enable, windows_ensure_port(ctx, &snap.rules)))
}

// Trust only the fresh rows (never the client's flags): find the rule by name, then guard and delete it.
pub fn build_windows_delete(ctx: &ConnectionContext, snap: &FirewallSnapshot, raw_spec: &str) -> Result<String, String> {
    let fresh: Vec<&FirewallRuleItem> = snap.rules.iter().filter(|r| r.raw_spec == raw_spec).collect();
    let Some(target) = fresh.first() else {
        return Err(vps_err(ERR_COMMAND_FAILED, "rule not found"));
    };
    if fresh.iter().any(|r| r.comment != WINDOWS_RULE_GROUP) {
        return Err(vps_err(ERR_UNSUPPORTED, "windows_rule_not_managed"));
    }
    crate::vps_guard::check_firewall_delete_windows(ctx, &guard_rules(snap), target)?;
    build_windows_delete_rule_script(target)
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
pub async fn run_windows_firewall_write(host_id: &str, pool: &SshPool, server_ip: &str, write: &str) -> Result<FirewallSnapshot, String> {
    let script = format!("{write}Write-Output 'SSHBOX_SNAPSHOT_BEGIN'\n{}", windows_firewall_read_script(server_ip));
    let raw = run_windows_script(host_id, pool, &script, None).await?;
    parse_windows_firewall(raw.rsplit("SSHBOX_SNAPSHOT_BEGIN").next().unwrap_or(&raw))
}

pub async fn get_windows_firewall(host_id: &str, pool: &SshPool, probe: &VpsProbe) -> Result<FirewallSnapshot, String> {
    check_windows_readable(probe)?;
    parse_windows_firewall(&run_windows_script(host_id, pool, &windows_firewall_read_script(&probe.context.server_ip), None).await?)
}

// The exec fails when the host has no powershell.exe (e.g. cmd.exe: "not recognized"): not a Windows we can manage.
// A timeout is not that: it keeps its own message.
fn exec_failure(detail: &str) -> String {
    if detail.starts_with(ERR_COMMAND_FAILED) {
        return detail.to_string();
    }
    vps_err(ERR_UNSUPPORTED, "unsupported_os")
}

pub async fn probe_windows(host: &HostEntry, pool: &SshPool) -> Result<VpsProbe, String> {
    let out = exec_windows(pool, host, WINDOWS_PROBE_SCRIPT, None).await.map_err(|e| exec_failure(&e))?;
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
            "OS_FAMILY:windows\nOS_CAPTION:Microsoft Windows 11 Pro\nOS_DISPLAY_VERSION:24H2\nUSER:{user}\nADMIN:{admin}\nOPENSSH:OpenSSH_9.5p2 for Windows\nSSHD_SERVICE:Running\nSSHD_EXE:1\nSSH_CONNECTION:{conn}\nSCHEDTASKS:1\nNETSECURITY:1\n"
        )
    }
    const CONN: &str = "10.0.0.5 51000 10.0.0.9 22";

    // A script on the command line would exceed cmd's 8191-char limit and expose secrets in the process list.
    #[test]
    fn powershell_command_is_constant_and_carries_no_script() {
        let c = build_powershell_command();
        assert_eq!(c, build_powershell_command());
        assert!(c.len() < 8191);
        assert!(!c.contains("sshbox_secret_begin"));
    }

    // cmd.exe and PowerShell interpret special characters; the command may only carry plain base64.
    #[test]
    fn powershell_command_is_plain_ascii_safe_in_cmd_and_powershell() {
        let c = build_powershell_command();
        let b64 = c.split("-EncodedCommand ").nth(1).unwrap();
        assert!(b64.chars().all(|ch| ch.is_ascii_alphanumeric() || "+/=".contains(ch)));
        assert!(!c.chars().any(|ch| "%^&|<>\"`$".contains(ch)));
    }

    // A wrongly encoded bootstrap makes PowerShell run the wrong script, or nothing.
    #[test]
    fn encoded_command_decodes_to_bootstrap() {
        let c = build_powershell_command();
        let b64 = c.split("-EncodedCommand ").nth(1).unwrap();
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64).unwrap();
        let units: Vec<u16> = bytes.chunks(2).map(|p| u16::from_le_bytes([p[0], p[1]])).collect();
        assert_eq!(String::from_utf16(&units).unwrap(), PS_BOOTSTRAP);
    }

    // The bootstrap splits script and secret at exactly this separator, in UTF-8.
    #[test]
    fn stdin_carries_script_then_secret_utf8() {
        let got = powershell_stdin("Write-Output 'Đạt'", Some("p'a$s"));
        assert_eq!(got, "Write-Output 'Đạt'\nSSHBOX_SECRET_BEGIN\np'a$s".as_bytes());
    }

    // A value placed in a script must not escape its single-quoted string.
    #[test]
    fn ps_quote_doubles_single_quotes() {
        assert_eq!(ps_quote("a'b"), "'a''b'");
    }

    // The support table and firewall rely on these fields; wrong values show the wrong features.
    #[test]
    fn windows_probe_admin_with_ssh_running() {
        let p = parse_windows_probe(&raw("1", "admin", CONN), key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::Admin);
        assert_eq!(p.os_version, "11 Pro 24H2");
        assert!(p.caps.sshd_running);
        assert_eq!(p.firewall_engine, FirewallEngine::Windows);
        assert_eq!(p.os_family, OsFamily::Windows);
    }

    // A standard account gets an empty CIM Caption, and the registry ProductName still says "Windows 10" on 11.
    #[test]
    fn windows_probe_names_the_os_without_cim_caption() {
        let no_cim = |product: &str, build: &str| {
            let r = raw("0", "bob", CONN).replace("OS_CAPTION:Microsoft Windows 11 Pro\n", &format!("OS_CAPTION:\nOS_PRODUCT:{product}\nOS_BUILD:{build}\n"));
            parse_windows_probe(&r, key_auth()).unwrap().os_version
        };
        assert_eq!(no_cim("Windows 10 Pro", "26300"), "11 Pro 24H2");
        assert_eq!(no_cim("Windows 10 Pro", "19045"), "10 Pro 24H2");
        assert_eq!(no_cim("Windows Server 2025 Datacenter", "26100"), "Server 2025 Datacenter 24H2");
        assert!(WINDOWS_PROBE_SCRIPT.contains("OS_PRODUCT:") && WINDOWS_PROBE_SCRIPT.contains("OS_BUILD:"));
    }

    // CIM denies a standard account; under EAP=Stop that aborted the whole probe and the host showed "unsupported OS".
    #[test]
    fn windows_probe_script_survives_cim_access_denied() {
        let cim = WINDOWS_PROBE_SCRIPT.lines().find(|l| l.contains("Get-CimInstance")).unwrap();
        assert!(cim.contains("-ErrorAction SilentlyContinue"), "{cim}");
    }

    // Spec §3.1: every SSH write runs the inbox sshd.exe; a running service without that file must not unlock writes.
    #[test]
    fn windows_ssh_needs_the_inbox_sshd_exe() {
        let p = parse_windows_probe(&raw("1", "admin", CONN).replace("SSHD_EXE:1", "SSHD_EXE:0"), key_auth()).unwrap();
        assert!(!p.caps.sshd_running);
        assert_eq!(p.support.ssh.reason.as_deref(), Some("windows_no_sshd"));
        let exe = WINDOWS_PROBE_SCRIPT.lines().find(|l| l.contains("SSHD_EXE:")).unwrap();
        assert!(exe.contains(r#"Test-Path "$env:WINDIR\System32\OpenSSH\sshd.exe""#), "{exe}");
    }

    // A user who is not an Administrator must not be treated as able to write.
    #[test]
    fn windows_probe_non_admin() {
        let p = parse_windows_probe(&raw("0", "bob", CONN), key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::None);
    }

    // Accented user names are real on Vietnamese Windows hosts; they must arrive intact.
    #[test]
    fn windows_probe_keeps_vietnamese_user_name() {
        let p = parse_windows_probe(&raw("1", "Đạt", CONN), key_auth()).unwrap();
        assert_eq!(p.context.login_user, "Đạt");
    }

    // The lockout guard compares client/server IPs; IPv6 must pass through intact.
    #[test]
    fn windows_probe_parses_ipv6_ssh_connection() {
        let p = parse_windows_probe(&raw("1", "a", "fe80::1 51000 fe80::2 2222"), key_auth()).unwrap();
        assert_eq!(p.context.client_ip, "fe80::1");
        assert_eq!(p.context.server_port, 2222);
    }

    // A host without PowerShell (cmd reports an error) must read as "unsupported", not as an odd error.
    #[test]
    fn windows_probe_without_marker_is_unsupported() {
        let e = parse_windows_probe("'powershell.exe' is not recognized as an internal or external command", key_auth()).unwrap_err();
        assert!(e.starts_with("UNSUPPORTED: unsupported_os"));
    }

    // On the VM a bootstrap once sat on stdin for 10+ minutes before running anything; the app must give up, not hang, and not call it "unsupported OS".
    #[tokio::test]
    async fn windows_exec_gives_up_on_a_silent_host() {
        let e = bounded(std::future::pending::<Result<Vec<u8>, String>>(), std::time::Duration::from_millis(20)).await.unwrap_err();
        // The user must know a half-applied sshd change is not left behind: the armed task reverts it.
        assert_eq!(e, "COMMAND_FAILED: the Windows host did not answer within 0s; if a change was being applied it is rolled back automatically within 60s");
        assert_eq!(exec_failure(&e), e);
    }

    // An exec failure (no powershell.exe) becomes unsupported_os; connection errors were already reported by get_or_connect.
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

    // Out-String ends the file text with its own newline; kept, every apply grew sshd_config by one trailing blank line.
    #[test]
    fn windows_view_drops_the_out_string_newline() {
        let raw = SSHD_RAW.replace("administrators_authorized_keys\r\n---", "administrators_authorized_keys\r\n\r\n---");
        let v = parse_windows_sshd_view(&raw).unwrap();
        assert!(v.main_raw.ends_with("administrators_authorized_keys\n"), "{:?}", v.main_raw);
        let applied = crate::vps_config::render_sshd_config(&v.main_raw, &[("maxauthtries", Some("MaxAuthTries 5".into()))]);
        assert!(applied.ends_with("administrators_authorized_keys\n"), "{applied:?}");
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
        assert!(s.contains("+ 'user=o''x,host=h' +"));
        assert!(s.contains("$r.Code -ne 0") && s.contains("throw") && s.contains("-Encoding UTF8"));
        assert_eq!(s.matches("---SSHBOX-PART---").count(), 3);
        assert!(s.contains(WINDOWS_SSHD_CONFIG) && s.contains("sshbox-rollback"));
    }

    // The accounts script must keep one-element arrays as arrays and survive an unreadable keys file.
    #[test]
    fn windows_accounts_script_is_array_safe_and_per_file_guarded() {
        let s = windows_accounts_read_script();
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

    // PS 5.1 serializes Get-Content lines as objects ({"value":..,"PSPath":..}); one key must not break the whole tab.
    #[test]
    fn windows_accounts_accept_key_lines_serialized_as_objects() {
        let obj = ACC.replace(
            r#"["ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHkeyone me@pc"]"#,
            r#"[{"value":"ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHkeyone me@pc","PSPath":"C:\\x","ReadCount":1}]"#,
        );
        let s = parse_windows_accounts(&format!("[{obj},{STD}]")).unwrap();
        assert_eq!(s.accounts[0].authorized_keys[0].public_key_body, "AAAAC3NzaC1lZDI1NTE5AAAAIHkeyone");
        assert!(windows_accounts_read_script().contains("ForEach-Object { [string]$_ }"));
    }

    // PS 5.1 drops the array brackets for a single account; the list must still load.
    #[test]
    fn windows_accounts_accept_single_object_json() {
        assert_eq!(parse_windows_accounts(ACC).unwrap().accounts.len(), 1);
        assert!(parse_windows_accounts("").unwrap().accounts.is_empty());
        assert!(parse_windows_accounts("not json").unwrap_err().starts_with("COMMAND_FAILED"));
    }

    const FW: &str = r#"{"profiles":[{"name":"Domain","enabled":false,"defaultInbound":"Block"},{"name":"Private","enabled":true,"defaultInbound":"Block"}],"active":["Private"],"sshProfile":"Private","rules":[{"displayName":"OpenSSH-Server-In-TCP","group":"ssh-box","program":"Any","action":"Allow","localPort":"22","protocol":"TCP","remoteAddress":"Any"},{"displayName":"Cho phép Đạt","group":"","program":"Any","action":"Block","localPort":"8080","protocol":"UDP","remoteAddress":"10.0.0.0/8"}],"sockets":[{"protocol":"tcp","addr":"::","port":22,"pid":1234,"process":"sshd","path":"C:\\Windows\\System32\\OpenSSH\\sshd.exe"},{"protocol":"tcp","addr":"127.0.0.1","port":5432,"pid":null,"process":"","path":""}]}"#;

    // An "Any" rule becomes a v4 and a v6 item; name and group must survive so deletes can find the rule by name.
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

    // Vietnamese Windows hosts have accented rule names; losing the accents breaks delete and matching.
    #[test]
    fn windows_firewall_parses_unicode_display_name() {
        let s = parse_windows_firewall(FW).unwrap();
        assert!(s.rules.iter().any(|r| r.raw_spec == "Cho phép Đạt"));
    }

    // An enabled but unused profile must not make the firewall "on"; with every active profile off, ports are exposed.
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

    // ufw: the first matching rule wins. Windows: a block always wins, even after an allow.
    #[test]
    fn allow_then_block_same_port_differs_between_ufw_and_windows() {
        let rules = [wrule("ALLOW", "80", "Anywhere"), wrule("DENY", "80", "Anywhere")];
        assert_eq!(crate::vps_config::compute_socket_exposure(&sock(80), true, "deny", true, &rules), "exposed");
        assert_eq!(compute_socket_exposure_windows(&sock(80), true, &rules), "blocked");
    }

    // A port without any port rule must not be reported "blocked"; it is unknown. A specific source means restricted.
    #[test]
    fn port_without_port_rule_is_unknown_on_windows() {
        assert_eq!(compute_socket_exposure_windows(&sock(9000), true, &[wrule("ALLOW", "80", "Anywhere")]), "unknown");
        assert_eq!(compute_socket_exposure_windows(&sock(80), true, &[wrule("ALLOW", "80", "10.0.0.0/8")]), "restricted");
        assert_eq!(compute_socket_exposure_windows(&sock(80), false, &[]), "exposed");
    }

    // The read script joins the filters once (PS 5.1 is slow) and never swallows a failing cmdlet.
    #[test]
    fn windows_firewall_script_joins_filters_once_and_fails_loudly() {
        let s = windows_firewall_read_script("10.0.0.9");
        assert!(s.contains("Get-NetFirewallApplicationFilter -All") && s.contains("Get-NetFirewallInterfaceTypeFilter -All") && s.contains("profile = [string]$_.Profile"));
        assert!(s.contains("Get-NetFirewallPortFilter -All") && s.contains("Get-NetFirewallAddressFilter -All"));
        assert!(s.contains("Get-NetTCPConnection -State Listen") && s.contains("Get-NetUDPEndpoint"));
        assert!(s.contains("ConvertTo-Json") && s.contains("-Compress"));
        assert!(!s.contains("Get-NetFirewallRule -ErrorAction SilentlyContinue"));
        assert_eq!(s.matches("SilentlyContinue").count(), 1); // only the process-name lookup
    }

    // Group Policy can override local firewall settings: display and guard must read the effective policy (ActiveStore),
    // while writes stay in the persistent store where a local change can live.
    #[test]
    fn windows_firewall_reads_active_store_and_writes_persistent_store() {
        let s = windows_firewall_read_script("10.0.0.9");
        assert_eq!(s.matches("Get-NetFirewall").count(), 7);
        assert_eq!(s.matches("-PolicyStore ActiveStore").count(), 7, "{s}");
        let writes = [
            build_windows_firewall_state_script(true, Some(22)),
            build_windows_add_rule_script("allow", "any", "8080", "tcp").unwrap(),
            build_windows_delete_rule_script(&win_rule("ssh-box")).unwrap(),
        ];
        assert!(writes.iter().all(|w| !w.contains("PolicyStore")));
    }

    // Broken JSON must be COMMAND_FAILED, not an empty list that looks "safe".
    #[test]
    fn windows_firewall_parse_error_is_command_failed() {
        assert!(parse_windows_firewall("not json").unwrap_err().starts_with("COMMAND_FAILED"));
    }

    // A Public-profile block does not block the port while SSH arrives on Private; it must not read "blocked".
    #[test]
    fn windows_rule_of_inactive_profile_does_not_count() {
        let j = FW.replace(r#""remoteAddress":"Any"}"#, r#""remoteAddress":"Any"},{"displayName":"B","group":"","program":"Any","profile":"Public","action":"Block","localPort":"22","protocol":"TCP","remoteAddress":"Any"}"#);
        assert_eq!(parse_windows_firewall(&j).unwrap().listening_sockets[0].exposure, "exposed");
        let j = j.replace(r#""profile":"Public""#, r#""profile":"Private""#);
        assert_eq!(parse_windows_firewall(&j).unwrap().listening_sockets[0].exposure, "blocked");
    }

    // A program-scoped rule is not a plain port rule; it must not create "blocked", the port degrades to unknown.
    #[test]
    fn windows_program_scoped_rule_degrades_to_unknown() {
        let j = FW.replace(r#""displayName":"OpenSSH-Server-In-TCP""#, r#""displayName":"OpenSSH-Server-In-TCP","scoped":true"#).replace(r#""action":"Allow""#, r#""action":"Block""#);
        let s = parse_windows_firewall(&j).unwrap();
        assert_eq!(s.listening_sockets[0].exposure, "unknown");
        let other = j.replace(r#","scoped":true"#, "").replace(r#""program":"Any","action":"Block","localPort":"22""#, r#""program":"C:\\tools\\other.exe","action":"Block","localPort":"22""#);
        assert_eq!(parse_windows_firewall(&other).unwrap().listening_sockets[0].exposure, "unknown");
        assert!(s.rules.iter().any(|r| r.raw_spec == "OpenSSH-Server-In-TCP"));
    }

    // The stock "OpenSSH SSH Server (sshd)" rule is limited to sshd.exe: it carries SSH while exactly that program owns the port.
    // Ignoring it showed port 22 "unknown" and made enabling add an allow for every profile (Public included).
    // A stale rule for another sshd.exe (zip/MSI install, Cygwin) must not count: it would suppress that auto-allow and pass the delete guard.
    #[test]
    fn windows_program_rule_counts_only_for_the_exact_program_path() {
        let with = |program: &str, path: &str| {
            let j = FW
                .replace(r#""group":"ssh-box","program":"Any""#, &format!(r#""group":"ssh-box","program":{}"#, serde_json::to_string(program).unwrap()))
                .replace(r#""path":"C:\\Windows\\System32\\OpenSSH\\sshd.exe""#, &format!(r#""path":{}"#, serde_json::to_string(path).unwrap()));
            parse_windows_firewall(&j).unwrap()
        };
        let counts = |s: &FirewallSnapshot| s.rules.iter().filter(|r| r.raw_spec == "OpenSSH-Server-In-TCP").all(|r| r.effective);
        let ctx = parse_windows_probe(&raw("1", "admin", CONN), key_auth()).unwrap().context;
        let inbox = r"C:\WINDOWS\system32\OpenSSH\sshd.exe"; // %SystemRoot% already expanded by the script; case differs
        let s = with(inbox, r"C:\Windows\System32\OpenSSH\sshd.exe");
        assert!(counts(&s) && s.listening_sockets[0].exposure == "exposed");
        assert_eq!(windows_ensure_port(&ctx, &s.rules), None);
        let s = with(inbox, r"C:\Program Files\OpenSSH\sshd.exe");
        assert!(!counts(&s) && s.listening_sockets[0].exposure == "unknown");
        assert_eq!(windows_ensure_port(&ctx, &s.rules), Some(22));
        assert!(!counts(&with(inbox, "")), "an unreadable path (PID 4 System) never owns a port");
        let script = windows_firewall_read_script("10.0.0.9");
        assert!(script.contains("[Environment]::ExpandEnvironmentVariables(") && script.contains("$paths[[int]$_.Id] = [string]$_.Path"));
    }

    // A rule whose application filter was not read is scoped, not "any program": counting it could hide the need for an SSH allow.
    #[test]
    fn windows_rule_without_application_filter_is_not_effective() {
        let s = parse_windows_firewall(&FW.replace(r#""group":"ssh-box","program":"Any","#, r#""group":"ssh-box","#)).unwrap();
        assert!(s.rules.iter().filter(|r| r.raw_spec == "OpenSSH-Server-In-TCP").all(|r| !r.effective));
        assert!(windows_firewall_read_script("10.0.0.9").contains("$prog = if ($app) {"));
    }

    // Each source address becomes its own item of the right family, or the guard misses an IPv6 client's allow.
    #[test]
    fn windows_remote_addresses_split_per_address_and_family() {
        let j = FW.replace(r#""remoteAddress":"10.0.0.0/8""#, r#""remoteAddress":"10.0.0.0/8,2001:db8::/64,LocalSubnet""#);
        let s = parse_windows_firewall(&j).unwrap();
        let got: Vec<_> = s.rules.iter().filter(|r| r.raw_spec == "Cho phép Đạt").map(|r| (r.from_source.as_str(), r.v6)).collect();
        assert_eq!(got, vec![("10.0.0.0/8", false), ("2001:db8::/64", true), ("LocalSubnet", false)]);
    }

    // Windows reports "10.0.0.0/8" as "10.0.0.0/255.0.0.0"; read as a host, a block covering the client would not count for the lockout guard.
    #[test]
    fn windows_netmask_sources_become_prefixes() {
        let j = FW.replace(r#""remoteAddress":"10.0.0.0/8""#, r#""remoteAddress":"192.168.65.0/255.255.255.0,10.0.0.0/255.0.255.0""#);
        let s = parse_windows_firewall(&j).unwrap();
        let got: Vec<_> = s.rules.iter().filter(|r| r.raw_spec == "Cho phép Đạt").map(|r| r.from_source.as_str()).collect();
        assert_eq!(got, vec!["192.168.65.0/24", "10.0.0.0/255.0.255.0"]);
        assert!(crate::vps_guard::source_covers(got[0], "192.168.65.1"));
    }

    // Allow from anywhere plus a block for one source: everyone but that source gets in, so it is exposed.
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

    // Every user value sent to PowerShell is quoted, and the rule joins the ssh-box group (so it can be deleted later).
    #[test]
    fn windows_add_rule_quotes_and_groups() {
        let s = build_windows_add_rule_script("deny", "10.0.0.1'; calc", "22", "tcp");
        assert!(s.is_err(), "source with quote must be rejected by validation");
        let s = build_windows_add_rule_script("deny", "10.0.0.0/8", "22", "tcp").unwrap();
        assert!(s.contains("-Group 'ssh-box'") && s.contains("-Action Block") && s.contains("-RemoteAddress '10.0.0.0/8'"));
        assert!(build_windows_add_rule_script("allow", "any", "22;calc", "tcp").is_err());
    }

    // A port with proto any must cover both TCP and UDP, or the rule only half applies.
    #[test]
    fn windows_add_any_proto_creates_tcp_and_udp() {
        let s = build_windows_add_rule_script("allow", "any", "53", "any").unwrap();
        assert_eq!(s.matches("New-NetFirewallRule").count(), 2);
        assert!(s.contains("-Protocol TCP") && s.contains("-Protocol UDP"));
    }

    // Only rules ssh-box created can be deleted; system rules such as OpenSSH-Server-In-TCP stay untouched.
    #[test]
    fn windows_delete_refuses_unmanaged_rule() {
        assert_eq!(build_windows_delete_rule_script(&win_rule("")), Err("UNSUPPORTED: windows_rule_not_managed".to_string()));
        let s = build_windows_delete_rule_script(&win_rule("ssh-box")).unwrap();
        assert!(s.contains("throw") && s.contains("-Group 'ssh-box'") && s.contains("-ceq 'ssh-box: ALLOW 22/tcp from any'"));
        assert!(!s.contains("-DisplayName"), "DisplayName parameter set is wildcard and cannot mix with -Group");
    }

    // Enabling the firewall without an SSH allow cuts the session; the rule must exist before the enable command.
    #[test]
    fn windows_enable_adds_ssh_allow_before_enabling() {
        let s = build_windows_firewall_state_script(true, Some(22));
        assert!(s.find("New-NetFirewallRule").unwrap() < s.find("Set-NetFirewallProfile").unwrap());
        assert!(s.contains("-Enabled True"));
        let off = build_windows_firewall_state_script(false, Some(22));
        assert!(!off.contains("New-NetFirewallRule") && off.contains("-Enabled False"));
    }

    // Blocking every source on the SSH port in use is a lockout; the guard must refuse it before PowerShell runs.
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

    fn admin_ctx() -> ConnectionContext {
        parse_windows_probe(&raw("1", "admin", CONN), key_auth()).unwrap().context
    }

    // Firewall off, SSH arrives on the Private NIC, the stock Private-only allow, plus EXTRA rules.
    const FW_OFF: &str = r#"{"profiles":[{"name":"Private","enabled":false,"defaultInbound":"Block"}],"active":["Private"],"sshProfile":"Private","rules":[{"displayName":"OpenSSH-Server-In-TCP","group":"","program":"Any","profile":"Private","action":"Allow","localPort":"22","protocol":"TCP","remoteAddress":"Any"}EXTRA],"sockets":[{"protocol":"tcp","addr":"0.0.0.0","port":22,"pid":1,"process":"sshd","path":"C:\\Windows\\System32\\OpenSSH\\sshd.exe"}]}"#;
    const ANY_PORT_BLOCK: &str = r#",{"displayName":"block subnet","group":"tmp","program":"Any","profile":"Any","action":"Block","localPort":"Any","protocol":"Any","remoteAddress":"10.0.0.0/255.255.255.0"}"#;
    const MY_ALLOW: &str = r#",{"displayName":"ssh-box: ALLOW 22/tcp from any","group":"ssh-box","program":"Any","profile":"Any","action":"Allow","localPort":"22","protocol":"TCP","remoteAddress":"Any"}"#;
    fn fw_off(extra: &str) -> FirewallSnapshot {
        parse_windows_firewall(&FW_OFF.replace("EXTRA", extra)).unwrap()
    }

    // A saved "block every port from my subnet" beats the auto-allow once the firewall is on: enabling would cut SSH.
    #[test]
    fn windows_enable_refused_when_any_port_block_covers_me() {
        let snap = fw_off(ANY_PORT_BLOCK);
        assert_eq!(build_windows_set_state(&admin_ctx(), &snap, true), Err("LOCKOUT_BLOCKED: windows_block_covers_me".to_string()));
        assert!(build_windows_set_state(&admin_ctx(), &snap, false).is_ok(), "turning the firewall off never locks anyone out");
        // Hidden from the UI list, yet the read script must still return port-less blocks.
        assert!(!snap.rules.iter().any(|r| r.raw_spec == "block subnet"));
        assert!(windows_firewall_read_script("10.0.0.9").contains("-or ([string]$_.Action -eq 'Block')"));
    }

    // A port-less block for another subnet does not touch this session, so enabling must still work.
    #[test]
    fn windows_enable_allowed_when_any_port_block_is_for_another_subnet() {
        let snap = fw_off(&ANY_PORT_BLOCK.replace("10.0.0.0/255.255.255.0", "192.0.2.0/255.255.255.0"));
        assert!(build_windows_set_state(&admin_ctx(), &snap, true).unwrap().contains("-Enabled True"));
    }

    // Block wins: with a port-less block on my subnet, deleting my ssh-box allow leaves no way in although the stock allow stays.
    #[test]
    fn windows_delete_counts_any_port_block_covering_me() {
        let snap = fw_off(&format!("{MY_ALLOW}{ANY_PORT_BLOCK}"));
        assert!(!snap.rules.iter().any(|r| r.raw_spec == "block subnet"));
        assert_eq!(build_windows_delete(&admin_ctx(), &snap, "ssh-box: ALLOW 22/tcp from any"), Err("LOCKOUT_BLOCKED: only_ssh_rule".to_string()));
        assert!(build_windows_delete(&admin_ctx(), &fw_off(MY_ALLOW), "ssh-box: ALLOW 22/tcp from any").is_ok());
    }

    // An ICMP (or protocol-number) block never touches TCP: read as "any protocol" it refused enabling, showed SSH "blocked" and refused deletes.
    #[test]
    fn windows_icmp_block_does_not_count_for_ssh() {
        let icmp = r#",{"displayName":"Block ICMPv4 from Any","group":"tmp","program":"Any","profile":"Any","action":"Block","localPort":"Any","protocol":"ICMPv4","remoteAddress":"Any"},{"displayName":"gre","group":"tmp","program":"Any","profile":"Any","action":"Block","localPort":"Any","protocol":"47","remoteAddress":"Any"}"#;
        assert!(build_windows_set_state(&admin_ctx(), &fw_off(icmp), true).unwrap().contains("-Enabled True"));
        let on = parse_windows_firewall(&FW_OFF.replace(r#""enabled":false"#, r#""enabled":true"#).replace("EXTRA", icmp)).unwrap();
        assert_eq!(on.listening_sockets[0].exposure, "exposed");
        assert!(build_windows_delete(&admin_ctx(), &fw_off(&format!("{MY_ALLOW}{icmp}")), "ssh-box: ALLOW 22/tcp from any").is_ok());
    }

    // Multi-NIC host: SSH arrives on a Public NIC while another NIC is Private. The Private-only stock rule does not carry
    // this session, so enabling must add the SSH allow first.
    #[test]
    fn windows_rule_counts_only_on_the_ssh_interface_profile() {
        let public = FW_OFF.replace(r#""active":["Private"],"sshProfile":"Private""#, r#""active":["Private","Public"],"sshProfile":"Public""#).replace("EXTRA", "");
        let snap = parse_windows_firewall(&public).unwrap();
        assert!(snap.rules.iter().all(|r| !r.effective));
        assert_eq!(windows_ensure_port(&admin_ctx(), &snap.rules), Some(22));
        assert!(build_windows_set_state(&admin_ctx(), &snap, true).unwrap().contains("New-NetFirewallRule"));
        let both = parse_windows_firewall(&public.replace(r#""profile":"Private""#, r#""profile":"Private, Public""#)).unwrap();
        assert_eq!(windows_ensure_port(&admin_ctx(), &both.rules), None);
        let private = parse_windows_firewall(&public.replace(r#""sshProfile":"Public""#, r#""sshProfile":"Private""#)).unwrap();
        assert_eq!(windows_ensure_port(&admin_ctx(), &private.rules), None);
    }

    // When the SSH interface's profile cannot be read, only rules for every profile ("Any") may count.
    #[test]
    fn windows_unknown_ssh_profile_counts_only_any_profile_rules() {
        let unknown = FW_OFF.replace(r#","sshProfile":"Private""#, "").replace("EXTRA", "");
        assert!(parse_windows_firewall(&unknown).unwrap().rules.iter().all(|r| !r.effective));
        let any = parse_windows_firewall(&unknown.replace(r#""profile":"Private""#, r#""profile":"Any""#)).unwrap();
        assert!(any.rules.iter().all(|r| r.effective));
        let s = windows_firewall_read_script("10.0.0.9");
        assert!(s.starts_with("$sshIp = '10.0.0.9'\n"), "{s}");
        assert!(s.contains("Get-NetIPAddress -IPAddress $sshIp") && s.contains("Get-NetConnectionProfile -InterfaceIndex") && s.contains("sshProfile = $sshProfile"));
        assert!(windows_firewall_read_script("10.0.0.9'; calc").starts_with("$sshIp = ''\n"));
    }

    // Arming the 60s rollback before restarting sshd is what makes a bad config recoverable.
    #[test]
    fn windows_apply_arms_task_before_restart() {
        let s = build_sshd_apply_script_windows(Some(2222), 2222);
        assert!(s.find("Register-ScheduledTask -TaskName").unwrap() < s.rfind("Restart-Service sshd").unwrap());
        assert!(s.contains("-User SYSTEM -RunLevel Highest") && s.contains("AddSeconds(60)"));
        assert!(s.contains("ALLOW 2222/tcp from any") && s.contains("-LocalPort 2222"));
        assert!(s.contains("NOW:") && s.contains("ROLLBACK_AT:") && s.contains("SSHBOX_ROLLBACK_PENDING"));
    }

    // This sshd hangs on `-t` of a bad config: the candidate is validated as a side file, before backup, arming or touching the live file.
    #[test]
    fn windows_apply_validates_side_file_before_touching_live_config() {
        let s = build_sshd_apply_script_windows(None, 22);
        let at = |n: &str| s.find(n).unwrap_or_else(|| panic!("missing {n}"));
        let validate = at("sshbox_sshd ('-t -f \"' + $N");
        assert!(at("WriteAllText($N") < validate);
        for later in ["Copy-Item -LiteralPath $F", "Register-ScheduledTask -TaskName", "Move-Item -LiteralPath $N"] {
            assert!(validate < at(later), "{later}");
        }
        assert!(at("Register-ScheduledTask -TaskName") < at("Move-Item -LiteralPath $N"));
        let bad = &s[at("$v.Code -ne 0")..];
        assert!(bad.find("Remove-Item").unwrap() < bad.find("SSHBOX_SSHD_INVALID").unwrap());
        assert!(bad.find("SSHBOX_SSHD_INVALID").unwrap() < bad.find("throw").unwrap());
    }

    // A hung sshd.exe must never hang the app: no Windows script may call sshd.exe except through the bounded helper.
    #[test]
    fn windows_scripts_only_call_sshd_through_the_timeout_helper() {
        let scripts = [
            windows_sshd_view_script("user=a"),
            windows_accounts_read_script(),
            build_sshd_apply_script_windows(Some(2222), 2222),
        ];
        for s in &scripts {
            assert!(s.contains("WaitForExit(15000)") && s.contains(".Kill()"));
            assert!(!s.contains("& $sshd") && !s.contains("& \"$env:WINDIR"), "{s}");
            assert_eq!(s.matches("sshd.exe").count(), 1, "only the helper names sshd.exe");
        }
    }

    // A failure between backup and arming must restore and clean up; a failure after arming runs the rollback.
    #[test]
    fn windows_apply_cleans_up_when_arming_fails_and_rolls_back_after() {
        let s = build_sshd_apply_script_windows(None, 22);
        let c = s.find("Remove-Item -LiteralPath $N, $R, \"$F.sshbox.bak\"").unwrap();
        assert!(c < s.find("Move-Item -LiteralPath $N").unwrap());
        assert!(!s[c - 20..c].contains("sshbox_restore"), "the live file is untouched before arming");
        assert!(s.contains("-StartWhenAvailable"));
        let tail = &s[s.find("Move-Item -LiteralPath $N").unwrap()..];
        assert!(tail.contains("sshbox-rollback.ps1' 2>&1 | Out-Null; $rbrc = $LASTEXITCODE"));
        assert!(tail.contains("$ErrorActionPreference = 'Continue'"));
    }

    // Blank lines are real config: the EOL rewrite must be the regex \r?\n, not two literal newlines.
    #[test]
    fn windows_apply_eol_rewrite_emits_exact_regex() {
        let s = build_sshd_apply_script_windows(None, 22);
        assert!(s.contains(r"-replace '\r?\n', $eol"), "{s}");
        assert!(s.contains("Contains(\"`r`n\")"));
        assert!(!s.contains("-replace \"\n"));
    }

    // Config text must travel via stdin, never the command line or the script body.
    #[test]
    fn windows_apply_reads_new_config_from_secret_only() {
        let s = build_sshd_apply_script_windows(None, 22);
        assert!(s.contains("$SSHBOX_SECRET") && s.contains("0xEF") && s.contains("`r`n"));
    }

    // A rolled-back port change must not leave its firewall hole, and the task and script must not linger.
    #[test]
    fn windows_rollback_unregisters_task_and_removes_port_rule() {
        let s = build_sshd_rollback_script_windows(Some(2222));
        assert!(s.contains("Remove-NetFirewallRule") && s.contains("ALLOW 2222/tcp from any"));
        assert!(s.find("Restart-Service sshd").unwrap() < s.find("Remove-NetFirewallRule").unwrap());
        assert!(s.find("Remove-Item -LiteralPath 'C:\\ProgramData\\ssh\\sshbox-rollback.ps1'").unwrap() < s.find("Unregister-ScheduledTask -TaskName sshbox-rollback").unwrap());
        assert!(!build_sshd_rollback_script_windows(None).contains("Remove-NetFirewallRule"));
    }

    // Keeping a change after the rollback already ran must fail loudly, same as Linux.
    #[test]
    fn windows_cleanup_reports_already_rolled_back() {
        let s = windows_cleanup_script();
        assert!(s.find("Unregister-ScheduledTask").unwrap() < s.find("SSHBOX_ALREADY_ROLLED_BACK").unwrap());
        assert!(s.contains("throw"));
    }

    // One disabled built-in admin (sshd hangs on it) or a failing sshd -T must degrade a row, never fail the whole tab.
    #[test]
    fn windows_accounts_skip_sshd_for_disabled_and_non_admin_and_never_throw() {
        let s = windows_accounts_read_script();
        assert!(s.contains("if ($isAdmin -and $u.Enabled)"));
        let body = &s[s.find("$isAdmin -and $u.Enabled").unwrap()..s.find("$keys = @()").unwrap()];
        assert!(body.contains("try {") && body.contains("catch { }") && !body.contains("throw"));
        assert_eq!(s.matches("sshbox_sshd (").count(), 1);
    }

    // On this build every bad config ends in a timeout, so the real sshd error must still reach the user (first line).
    #[test]
    fn windows_sshd_helper_keeps_stderr_on_timeout() {
        assert!(SSHD_HELPER.contains("$e.Wait(1000)") && SSHD_HELPER.contains("timed out after 15s"));
        assert!(SSHD_HELPER.find("$why +").unwrap() < SSHD_HELPER.find("timed out after 15s").unwrap());
    }

    // "Roll back now" must not claim success when a restore step failed, yet every step and the task removal still run.
    #[test]
    fn windows_rollback_reports_failure_but_still_unregisters() {
        let s = build_sshd_rollback_script_windows(Some(2222));
        assert!(s.contains("try { sshbox_restore } catch { $ok = $false }") && s.contains("try { Restart-Service sshd } catch { $ok = $false }"));
        assert!(s.contains("sshd_config.sshbox.new") || s.contains("$F.sshbox.new"));
        assert!(s.find("Unregister-ScheduledTask").unwrap() < s.find("if (-not $ok) { exit 1 }").unwrap());
    }
}
