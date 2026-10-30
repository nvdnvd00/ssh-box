// Tool Installer catalog and core types (spec §6). Pure: no I/O.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::vps_config::shell_single_quote as shq;
use crate::ssh_pool::SshPool;
use crate::vps_config::{
    classify_privileged_output, find_remote_host, run_privileged_script, vps_err, ERR_COMMAND_FAILED, ERR_SUDO_DENIED, ERR_SUDO_REQUIRED, ERR_UNSUPPORTED,
};
use crate::vps_windows::{ps_quote, run_windows_script};
#[cfg(target_os = "windows")]
use std::os::windows::process::CommandExt;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HostOs {
    Linux,
    Macos,
    Windows,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Method {
    pub os: HostOs,
    pub manager: String,
    pub package: String,
    pub service: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CatalogTool {
    pub id: String,
    pub name: String,
    pub category: String,
    pub recommended: bool,
    pub description_vi: String,
    pub description_en: String,
    pub bin: Option<String>,
    pub methods: Vec<Method>,
}

pub fn catalog() -> &'static [CatalogTool] {
    static CATALOG: OnceLock<Vec<CatalogTool>> = OnceLock::new();
    CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!("installer_catalog.json")).expect("installer_catalog.json")
    })
}

pub fn managers_for(os: HostOs) -> &'static [&'static str] {
    match os {
        HostOs::Linux => &["apt", "dnf", "snap", "npm"],
        HostOs::Macos => &["brew", "npm"],
        HostOs::Windows => &["winget", "npm"],
    }
}

pub fn pick_method<'a>(tool: &'a CatalogTool, os: HostOs, present: &[String]) -> Option<&'a Method> {
    tool.methods.iter().find(|m| m.os == os && present.contains(&m.manager))
}

// ^[A-Za-z0-9@/._+:-]{1,128}$ — the only names allowed into a generated script.
pub fn valid_name(s: &str) -> bool {
    (1..=128).contains(&s.len())
        && !s.starts_with('-')
        && s.chars().all(|c| c.is_ascii_alphanumeric() || "@/._+:-".contains(c))
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ToolStatus {
    NotInstalled,
    Installed { via: Vec<Owner> },
    External,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Owner {
    pub manager: String,
    pub package: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ServiceState {
    pub enabled: bool,
    pub active: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ManagerState {
    pub id: String,
    pub present: bool,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ToolView {
    pub id: String,
    pub name: String,
    pub category: String,
    pub recommended: bool,
    pub description_vi: String,
    pub description_en: String,
    pub status: ToolStatus,
    pub service: Option<String>,
    pub service_state: Option<ServiceState>,
    pub install: Option<Method>,
    pub blocked_by: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct InstallerScan {
    pub os: HostOs,
    pub managers: Vec<ManagerState>,
    pub tools: Vec<ToolView>,
}

fn os_methods(t: &CatalogTool, os: HostOs) -> impl Iterator<Item = &Method> {
    t.methods.iter().filter(move |m| m.os == os)
}

// Name used for the "installed outside a manager" check: the binary, else the tool id.
fn bin_key(t: &CatalogTool) -> &str {
    t.bin.as_deref().unwrap_or(&t.id)
}

pub fn scan_script(os: HostOs) -> String {
    let tools = || catalog().iter().filter(move |t| os_methods(t, os).next().is_some());
    let mut s = String::new();
    if os == HostOs::Windows {
        for m in managers_for(os) {
            s += &format!("{} + [int][bool](Get-Command {} -ErrorAction SilentlyContinue)\n", ps_quote(&format!("MGR:{m}:")), ps_quote(m));
        }
        let ids: Vec<String> = tools()
            .flat_map(|t| os_methods(t, os).filter(|m| m.manager == "winget").map(|m| ps_quote(&m.package)))
            .collect();
        s += &format!(
            "$w = winget list --source winget --accept-source-agreements --disable-interactivity 2>$null | Out-String\nforeach ($id in @({})) {{ if ($w -match ('(?m)(^|\\s)' + [regex]::Escape($id) + '(\\s|$)')) {{ ('PKG:winget:' + $id) }} }}\n",
            ids.join(",")
        );
        s += "npm ls -g --depth=0 --parseable 2>$null | ForEach-Object { if ($_ -match 'node_modules[\\\\/](.+)$') { 'PKG:npm:' + ($Matches[1] -replace '\\\\','/') } }\n";
        for t in tools() {
            let k = bin_key(t);
            s += &format!("{} + [int][bool](Get-Command {} -ErrorAction SilentlyContinue)\n", ps_quote(&format!("BIN:{k}:")), ps_quote(k));
            if let Some(v) = os_methods(t, os).find_map(|m| m.service.as_deref()) {
                s += &format!(
                    "$s = Get-Service -Name {q} -ErrorAction SilentlyContinue\nif ($s) {{ {l} + [int]($s.StartType -eq 'Automatic') + ':' + [int]($s.Status -eq 'Running') }}\n",
                    q = ps_quote(v), l = ps_quote(&format!("SVC:{v}:"))
                );
            }
        }
        return s;
    }
    for m in managers_for(os) {
        s += &format!("if command -v {q} >/dev/null 2>&1; then echo {r}; else echo {z}; fi\n", q = shq(m), r = shq(&format!("MGR:{m}:1")), z = shq(&format!("MGR:{m}:0")));
    }
    s += "dpkg-query -W -f='${Package}\\n' 2>/dev/null | sed 's/^/PKG:apt:/'\n";
    s += "rpm -qa --qf '%{NAME}\\n' 2>/dev/null | sed 's/^/PKG:dnf:/'\n";
    s += "snap list 2>/dev/null | awk 'NR>1{print \"PKG:snap:\"$1}'\n";
    s += "npm ls -g --depth=0 --parseable 2>/dev/null | sed -n 's|.*node_modules/|PKG:npm:|p'\n";
    s += "brew list --formula -1 2>/dev/null | sed 's/^/PKG:brew:/'\n";
    s += "brew list --cask -1 2>/dev/null | sed 's/^/PKG:brew:/'\n";
    for t in tools() {
        let k = bin_key(t);
        s += &format!("if command -v {q} >/dev/null 2>&1; then echo {y}; else echo {n}; fi\n", q = shq(k), y = shq(&format!("BIN:{k}:1")), n = shq(&format!("BIN:{k}:0")));
        if let Some(v) = os_methods(t, os).find_map(|m| m.service.as_deref()) {
            if os == HostOs::Linux {
                s += &format!(
                    "systemctl is-enabled {q} >/dev/null 2>&1 && e=1 || e=0\nsystemctl is-active --quiet {q} && a=1 || a=0\necho {p}\"$e\":\"$a\"\n",
                    q = shq(v), p = shq(&format!("SVC:{v}:"))
                );
            } else {
                s += &format!(
                    "brew services list 2>/dev/null | awk -v s={q} '$1==s{{print \"SVC:\"s\":\"($2==\"none\"?0:1)\":\"($2==\"started\"?1:0)}}'\n",
                    q = shq(v)
                );
            }
        }
    }
    s
}

pub fn parse_scan(os: HostOs, raw: &str) -> InstallerScan {
    let mut mgrs: HashSet<&str> = HashSet::new();
    let mut pkgs: HashSet<(&str, &str)> = HashSet::new();
    let mut bins: HashSet<&str> = HashSet::new();
    let mut svcs: HashMap<&str, ServiceState> = HashMap::new();
    for line in raw.lines().map(str::trim) {
        let p: Vec<&str> = line.splitn(4, ':').collect();
        match p.as_slice() {
            ["MGR", m, "1"] => { mgrs.insert(m); }
            ["PKG", m, n] => { pkgs.insert((m, n)); }
            ["BIN", b, "1"] => { bins.insert(b); }
            ["SVC", n, e, a] => { svcs.insert(n, ServiceState { enabled: *e == "1", active: *a == "1" }); }
            _ => {}
        }
    }
    let managers: Vec<ManagerState> = managers_for(os)
        .iter()
        .map(|m| ManagerState { id: m.to_string(), present: mgrs.contains(m) })
        .collect();
    let present: Vec<String> = managers.iter().filter(|m| m.present).map(|m| m.id.clone()).collect();
    let tools = catalog()
        .iter()
        .filter_map(|t| {
            let first = os_methods(t, os).next()?;
            let via: Vec<Owner> = os_methods(t, os)
                .filter(|m| pkgs.contains(&(m.manager.as_str(), m.package.as_str())))
                .map(|m| Owner { manager: m.manager.clone(), package: m.package.clone() })
                .collect();
            let status = if !via.is_empty() {
                ToolStatus::Installed { via }
            } else if bins.contains(bin_key(t)) {
                ToolStatus::External
            } else {
                ToolStatus::NotInstalled
            };
            let service = os_methods(t, os).find_map(|m| m.service.clone());
            let install = pick_method(t, os, &present).cloned();
            Some(ToolView {
                id: t.id.clone(),
                name: t.name.clone(),
                category: t.category.clone(),
                recommended: t.recommended,
                description_vi: t.description_vi.clone(),
                description_en: t.description_en.clone(),
                status,
                service_state: service.as_deref().and_then(|v| svcs.get(v).cloned()),
                service,
                blocked_by: if install.is_none() { Some(first.manager.clone()) } else { None },
                install,
            })
        })
        .collect();
    InstallerScan { os, managers, tools }
}

#[derive(Deserialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum InstallAction {
    Install { manager: String, package: String },
    Uninstall { manager: String, package: String },
    EnableService { service: String },
    InstallManager { manager: String },
    Refresh,
}

const WINGET_FLAGS: &str = "--silent --disable-interactivity --accept-package-agreements --accept-source-agreements";

fn checked<'a>(os: HostOs, manager: &str, name: &'a str) -> Result<&'a str, String> {
    if !managers_for(os).contains(&manager) {
        return Err(format!("manager {manager} not supported on this OS"));
    }
    if !valid_name(name) {
        return Err(format!("invalid name: {name}"));
    }
    Ok(name)
}

pub fn action_script(os: HostOs, a: &InstallAction) -> Result<String, String> {
    use InstallAction::*;
    match a {
        Install { manager, package } | Uninstall { manager, package } => {
            let add = matches!(a, Install { .. });
            let pkg = checked(os, manager, package)?;
            let (q, pq) = (shq(pkg), ps_quote(pkg));
            Ok(match (manager.as_str(), add) {
                ("apt", true) => format!("DEBIAN_FRONTEND=noninteractive apt-get install -y {q}"),
                ("apt", false) => format!("DEBIAN_FRONTEND=noninteractive apt-get remove -y {q}"),
                ("dnf", true) => format!("dnf install -y {q}"),
                ("dnf", false) => format!("dnf remove -y {q}"),
                ("snap", true) => format!("snap install {q}"),
                ("snap", false) => format!("snap remove {q}"),
                ("brew", true) => format!("NONINTERACTIVE=1 brew install {q}"),
                ("brew", false) => format!("NONINTERACTIVE=1 brew uninstall {q}"),
                ("winget", true) => format!("winget install --id {pq} -e {WINGET_FLAGS}"),
                ("winget", false) => format!("winget uninstall --id {pq} -e --silent --disable-interactivity"),
                ("npm", true) => format!("npm install -g {}", if os == HostOs::Windows { pq } else { q }),
                ("npm", _) => format!("npm uninstall -g {}", if os == HostOs::Windows { pq } else { q }),
                _ => return Err(format!("unsupported manager {manager}")),
            })
        }
        EnableService { service } => {
            if !valid_name(service) {
                return Err(format!("invalid name: {service}"));
            }
            Ok(match os {
                HostOs::Linux => format!("systemctl enable --now {}", shq(service)),
                HostOs::Macos => format!("brew services start {}", shq(service)),
                HostOs::Windows => {
                    let q = ps_quote(service);
                    format!("Set-Service -Name {q} -StartupType Automatic; Start-Service -Name {q}")
                }
            })
        }
        InstallManager { manager } => match (manager.as_str(), os) {
            ("brew", HostOs::Macos) => Ok("NONINTERACTIVE=1 /bin/bash -c \"$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)\"".into()),
            ("snap", HostOs::Linux) => Ok("DEBIAN_FRONTEND=noninteractive apt-get install -y snapd".into()),
            ("npm", HostOs::Linux) => Ok("if command -v apt-get >/dev/null 2>&1; then DEBIAN_FRONTEND=noninteractive apt-get install -y nodejs npm; else dnf install -y nodejs npm; fi".into()),
            ("npm", HostOs::Macos) => Ok("brew install node".into()),
            ("npm", HostOs::Windows) => Ok(format!("winget install --id OpenJS.NodeJS.LTS -e {WINGET_FLAGS}")),
            ("winget", HostOs::Windows) => Err("WINGET_MANUAL".into()),
            _ => Err(format!("cannot install manager {manager} on this OS")),
        },
        Refresh => match os {
            HostOs::Linux => Ok("if command -v apt-get >/dev/null 2>&1; then apt-get update -qq; else dnf makecache -q; fi".into()),
            _ => Err("refresh is only supported on Linux".into()),
        },
    }
}

pub fn needs_root(os: HostOs, a: &InstallAction) -> bool {
    use InstallAction::*;
    os == HostOs::Linux
        && match a {
            Install { manager, .. } | Uninstall { manager, .. } => // npm too: the distro's global prefix (/usr/lib) is root-owned.
                ["apt", "dnf", "snap", "npm"].contains(&manager.as_str()),
            EnableService { .. } | Refresh | InstallManager { .. } => true,
        }
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SearchHit {
    pub manager: String,
    pub package: String,
    pub description: String,
}

pub fn search_script(os: HostOs, present: &[String], query: &str) -> Result<String, String> {
    if !valid_name(query) {
        return Err(format!("invalid query: {query}"));
    }
    let q = shq(query);
    let mut s = String::new();
    for m in managers_for(os).iter().filter(|m| present.iter().any(|p| p == *m)) {
        s += &match (*m, os) {
            ("apt", _) => format!("apt-cache search --names-only {q} 2>/dev/null | head -n 20 | sed -E 's/^([^ ]+) - /HIT:apt:\\1:/'\n"),
            ("dnf", _) => format!("dnf search -q {q} 2>/dev/null | grep ' : ' | head -n 20 | sed -E 's/^([^ ]+) : /HIT:dnf:\\1:/'\n"),
            ("snap", _) => format!("snap find {q} 2>/dev/null | awk 'NR>1{{n=$1; $1=$2=$3=$4=\"\"; sub(/^ +/,\"\"); print \"HIT:snap:\" n \":\" $0}}' | head -n 20\n"),
            ("brew", _) => format!("brew search {q} 2>/dev/null | grep -v '^==>' | head -n 20 | sed 's/^/HIT:brew:/; s/$/:/'\n"),
            ("winget", _) => format!(
                "winget search --source winget --accept-source-agreements --disable-interactivity {} | Out-String -Stream | ForEach-Object {{ if ($_ -match '^(.+?)\\s{{2,}}([A-Za-z0-9_-]+(\\.[A-Za-z0-9_-]+)+)\\s') {{ 'HIT:winget:' + $Matches[2] + ':' + $Matches[1].Trim() }} }} | Select-Object -First 20\n",
                ps_quote(query)
            ),
            // npm search --parseable: name<TAB>description<TAB>...
            (_, HostOs::Windows) => format!("npm search --parseable {} | Select-Object -First 20 | ForEach-Object {{ $p = $_ -split [char]9; 'HIT:npm:' + $p[0] + ':' + $p[1] }}\n", ps_quote(query)),
            _ => format!("npm search --parseable {q} 2>/dev/null | head -n 20 | awk -F'\\t' '{{print \"HIT:npm:\" $1 \":\" $2}}'\n"),
        };
    }
    if s.is_empty() {
        return Err("no package manager available".into());
    }
    Ok(s)
}

pub fn parse_search(raw: &str) -> Vec<SearchHit> {
    raw.lines()
        .filter_map(|l| {
            let p: Vec<&str> = l.trim().splitn(4, ':').collect();
            match p.as_slice() {
                ["HIT", m, n, d] => Some(SearchHit { manager: m.to_string(), package: n.to_string(), description: d.trim().to_string() }),
                _ => None,
            }
        })
        .collect()
}

const TIMEOUT: &str = "TIMEOUT";

struct GroupKill(Option<u32>);

impl Drop for GroupKill {
    fn drop(&mut self) {
        #[cfg(unix)]
        if let Some(pid) = self.0 {
            let _ = std::process::Command::new("kill").args(["-TERM", &format!("-{pid}")]).output();
        }
    }
}

async fn pipe(mut cmd: tokio::process::Command, stdin: Vec<u8>) -> Result<String, String> {
    use std::process::Stdio;
    use tokio::io::AsyncWriteExt;
    cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true);
    #[cfg(unix)]
    cmd.process_group(0);
    let mut child = cmd.spawn().map_err(|e| vps_err(ERR_COMMAND_FAILED, e))?;
    // On timeout the future is dropped: SIGTERM the whole group so apt/brew grandchildren die too
    // (sudo relays it to its child). ponytail: remote SSH timeouts rely on the channel closing.
    let mut guard = GroupKill(child.id());
    let mut sin = child.stdin.take().ok_or_else(|| vps_err(ERR_COMMAND_FAILED, "no stdin"))?;
    // Ignore write errors: the child may exit before reading everything.
    let _ = sin.write_all(&stdin).await;
    drop(sin);
    let out = child.wait_with_output().await.map_err(|e| vps_err(ERR_COMMAND_FAILED, e))?;
    guard.0 = None; // finished: never signal a possibly reused pid
    let text = format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    if out.status.success() {
        Ok(text)
    } else {
        let low = text.to_lowercase();
        if low.contains("incorrect password") || low.contains("sorry, try again") {
            return Err(vps_err(ERR_SUDO_DENIED, "incorrect sudo password"));
        }
        Err(vps_err(ERR_COMMAND_FAILED, text.trim()))
    }
}

pub async fn run_local(
    os: HostOs,
    script: &str,
    sudo_password: Option<&str>,
    root: bool,
    elevated: bool,
) -> Result<String, String> {
    if os == HostOs::Windows {
        return run_local_windows(script, elevated).await;
    }
    let mut cmd = tokio::process::Command::new(if root { "sudo" } else { "sh" });
    if !root {
        cmd.arg("-s");
        return pipe(cmd, script.as_bytes().to_vec()).await;
    }
    match sudo_password.filter(|p| !p.is_empty()) {
        Some(pw) => {
            if pw.contains('\n') || pw.contains('\r') {
                return Err(vps_err(ERR_SUDO_DENIED, "sudo password must not contain a line break"));
            }
            // -k: ignore cached credentials so sudo always consumes line 1; otherwise sh would run the password.
            cmd.args(["-k", "-S", "-p", "", "sh", "-s"]);
            pipe(cmd, format!("{pw}\n{script}").into_bytes()).await
        }
        None => {
            // Passwordless sudo works without prompting; otherwise ask the UI for a password.
            let ok = tokio::process::Command::new("sudo")
                .args(["-n", "true"])
                .stdin(std::process::Stdio::null())
                .output()
                .await
                .map(|o| o.status.success())
                .unwrap_or(false);
            if !ok {
                return Err(vps_err(ERR_SUDO_REQUIRED, "sudo password required"));
            }
            cmd.args(["-n", "sh", "-s"]);
            pipe(cmd, script.as_bytes().to_vec()).await
        }
    }
}

#[cfg(target_os = "windows")]
async fn run_local_windows(script: &str, elevated: bool) -> Result<String, String> {
    const CREATE_NO_WINDOW: u32 = 0x0800_0000;
    let ps = |args: &[&str]| {
        let mut c = tokio::process::Command::new("powershell.exe");
        c.args(["-NoProfile", "-NonInteractive"]).args(args).creation_flags(CREATE_NO_WINDOW);
        c
    };
    if !elevated {
        return pipe(ps(&["-Command", "-"]), script.as_bytes().to_vec()).await;
    }
    // UAC prompt via RunAs; the elevated child cannot pipe back, so it writes to a temp file.
    let dir = std::env::temp_dir();
    let id = std::process::id();
    static SEQ: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = SEQ.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let (ps1, out) = (dir.join(format!("sshbox-inst-{id}-{n}.ps1")), dir.join(format!("sshbox-inst-{id}-{n}.txt")));
    struct Cleanup([std::path::PathBuf; 2]);
    impl Drop for Cleanup {
        fn drop(&mut self) {
            self.0.iter().for_each(|p| { let _ = std::fs::remove_file(p); });
        }
    }
    let _cleanup = Cleanup([ps1.clone(), out.clone()]); // also runs when a timeout drops this future
    let o = ps_quote(&out.to_string_lossy());
    std::fs::write(&ps1, format!("& {{\n{script}\n}} *> {o}\nif (-not $?) {{ Add-Content -Path {o} 'SSHBOX_EXIT_CODE:1' }}\n"))
        .map_err(|e| vps_err(ERR_COMMAND_FAILED, e))?;
    let outer = format!(
        "Start-Process powershell -Verb RunAs -Wait -WindowStyle Hidden -ArgumentList '-NoProfile','-ExecutionPolicy','Bypass','-File',{}",
        ps_quote(&ps1.to_string_lossy())
    );
    let r = pipe(ps(&["-Command", "-"]), outer.into_bytes()).await;
    let text = std::fs::read_to_string(&out).unwrap_or_default();
    r?;
    classify_privileged_output(&text)
}

#[cfg(not(target_os = "windows"))]
async fn run_local_windows(_script: &str, _elevated: bool) -> Result<String, String> {
    Err(vps_err(ERR_UNSUPPORTED, "windows_only"))
}

const UNIX_PATH: &str = r#"export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.npm-global/bin:$PATH""#;

#[allow(clippy::too_many_arguments)]
pub async fn run(
    host_id: &str,
    pool: &SshPool,
    os: HostOs,
    script: &str,
    sudo_password: Option<String>,
    root: bool,
    elevated: bool,
    secs: u64,
) -> Result<String, String> {
    // GUI-launched apps and non-login SSH miss Homebrew/npm bins; inside the script so it survives sudo.
    let script = &if os == HostOs::Windows { script.to_string() } else { format!("{UNIX_PATH}\n{script}") };
    let fut = async {
        if host_id == "local" {
            run_local(os, script, sudo_password.as_deref(), root, elevated).await
        } else if os == HostOs::Windows {
            run_windows_script(host_id, pool, script, None).await
        } else if root {
            run_privileged_script(host_id, pool, sudo_password, script, None).await
        } else {
            let host = find_remote_host(host_id)?;
            let out = pool.exec_bytes_on_host(&host, r#"sh -s 2>&1 || echo "SSHBOX_EXIT_CODE:$?""#, Some(script.as_bytes())).await?;
            classify_privileged_output(&String::from_utf8_lossy(&out))
        }
    };
    tokio::time::timeout(std::time::Duration::from_secs(secs), fut)
        .await
        .unwrap_or_else(|_| Err(vps_err(TIMEOUT, format!("exceeded {secs}s"))))
}

pub fn local_os() -> HostOs {
    if cfg!(target_os = "windows") {
        HostOs::Windows
    } else if cfg!(target_os = "macos") {
        HostOs::Macos
    } else {
        HostOs::Linux
    }
}

pub async fn scan(host_id: &str, pool: &SshPool, os: HostOs) -> Result<InstallerScan, String> {
    // Scan is read-only: no root needed.
    let raw = run(host_id, pool, os, &scan_script(os), None, false, false, 60).await?;
    Ok(parse_scan(os, &raw))
}

pub async fn search(host_id: &str, pool: &SshPool, os: HostOs, present: Vec<String>, query: String) -> Result<Vec<SearchHit>, String> {
    let script = search_script(os, &present, &query)?;
    Ok(parse_search(&run(host_id, pool, os, &script, None, false, false, 30).await?))
}

pub async fn run_action(host_id: &str, pool: &SshPool, os: HostOs, a: InstallAction, pw: Option<String>) -> Result<String, String> {
    let script = action_script(os, &a)?;
    let elevated = host_id == "local"
        && os == HostOs::Windows
        && matches!(a, InstallAction::Uninstall { .. } | InstallAction::EnableService { .. });
    run(host_id, pool, os, &script, pw, needs_root(os, &a), elevated, 600).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_ids_unique() {
        let c = catalog();
        assert!(!c.is_empty());
        for t in c {
            assert!(["security", "apps", "ai", "network", "utils"].contains(&t.category.as_str()), "{}", t.id);
        }
        let mut ids: Vec<&str> = c.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), c.len());
        for t in c {
            for m in &t.methods {
                assert!(managers_for(m.os).contains(&m.manager.as_str()), "{} {}", t.id, m.manager);
                assert!(valid_name(&m.package));
                assert!(m.service.as_deref().is_none_or(valid_name), "{}", t.id);
            }
        }
    }

    #[test]
    fn pick_method_prefers_first_present() {
        let t = catalog().iter().find(|t| t.id == "docker").unwrap();
        let p = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
        assert_eq!(pick_method(t, HostOs::Linux, &p(&["snap"])).unwrap().manager, "snap");
        assert_eq!(pick_method(t, HostOs::Linux, &p(&["apt", "snap"])).unwrap().manager, "apt");
        assert!(pick_method(t, HostOs::Linux, &[]).is_none());
    }

    #[test]
    fn valid_name_rejects_shell() {
        for ok in ["@anthropic-ai/claude-code", "Tailscale.Tailscale", "python3"] {
            assert!(valid_name(ok), "{ok}");
        }
        for bad in ["a;b", "a b", "$(x)", "", "--help", "-x"] {
            assert!(!valid_name(bad), "{bad}");
        }
        assert!(!valid_name(&"a".repeat(129)));
    }

    fn own(manager: &str, package: &str) -> Owner {
        Owner { manager: manager.into(), package: package.into() }
    }

    fn scan_tool<'a>(s: &'a InstallerScan, id: &str) -> &'a ToolView {
        s.tools.iter().find(|t| t.id == id).unwrap()
    }

    #[test]
    fn parse_scan_linux_owner_detection() {
        let s = parse_scan(HostOs::Linux, "MGR:apt:1\nPKG:apt:docker.io\nSVC:docker:1:0\n");
        let d = scan_tool(&s, "docker");
        assert_eq!(d.status, ToolStatus::Installed { via: vec![own("apt", "docker.io")] });
        assert_eq!(d.service_state, Some(ServiceState { enabled: true, active: false }));
    }

    #[test]
    fn parse_scan_two_owners() {
        // docker is the catalog tool that has both an apt and a snap method.
        let s = parse_scan(HostOs::Linux, "MGR:apt:1\nMGR:snap:1\nPKG:apt:docker.io\nPKG:snap:docker\n");
        assert_eq!(scan_tool(&s, "docker").status, ToolStatus::Installed { via: vec![own("apt", "docker.io"), own("snap", "docker")] });
    }

    #[test]
    fn parse_scan_external_has_no_owner() {
        let s = parse_scan(HostOs::Linux, "MGR:apt:1\nBIN:tailscale:1\n");
        assert_eq!(scan_tool(&s, "tailscale").status, ToolStatus::External);
    }

    #[test]
    fn parse_scan_blocked_by_missing_manager() {
        let s = parse_scan(HostOs::Macos, "MGR:brew:0\n");
        let t = scan_tool(&s, "tailscale");
        assert!(t.install.is_none());
        assert_eq!(t.blocked_by.as_deref(), Some("brew"));
    }

    #[test]
    fn windows_npm_paths_normalized_and_matched() {
        assert!(scan_script(HostOs::Windows).contains("-replace '\\\\','/'"));
        let s = parse_scan(HostOs::Windows, "MGR:npm:1\nPKG:npm:@anthropic-ai/claude-code\n");
        assert_eq!(scan_tool(&s, "claude-code").status, ToolStatus::Installed { via: vec![own("npm", "@anthropic-ai/claude-code")] });
    }

    #[test]
    fn windows_external_via_bin() {
        let s = parse_scan(HostOs::Windows, "BIN:tailscale:1\n");
        assert_eq!(scan_tool(&s, "tailscale").status, ToolStatus::External);
    }

    #[test]
    fn scan_script_quotes_catalog_names() {
        assert!(scan_script(HostOs::Windows).contains("'Tailscale.Tailscale'"));
        for os in [HostOs::Linux, HostOs::Macos] {
            assert!(!scan_script(os).contains("$("));
        }
    }

    fn act(k: &str, m: &str, p: &str) -> InstallAction {
        serde_json::from_str(&format!(r#"{{"kind":"{k}","manager":"{m}","package":"{p}"}}"#)).unwrap()
    }

    #[test]
    fn action_apt_install_noninteractive() {
        let s = action_script(HostOs::Linux, &act("install", "apt", "fail2ban")).unwrap();
        assert!(s.contains("DEBIAN_FRONTEND=noninteractive") && s.contains("apt-get install -y 'fail2ban'"));
    }

    #[test]
    fn action_rejects_bad_name() {
        assert!(action_script(HostOs::Linux, &act("install", "apt", "a;b")).is_err());
        assert!(action_script(HostOs::Linux, &act("install", "brew", "jq")).is_err());
        assert!(action_script(HostOs::Linux, &InstallAction::EnableService { service: "a b".into() }).is_err());
    }

    #[test]
    fn uninstall_uses_given_manager() {
        let s = action_script(HostOs::Macos, &act("uninstall", "brew", "jq")).unwrap();
        assert!(s.contains("brew uninstall") && !s.contains("npm"));
    }

    #[test]
    fn winget_flags_present() {
        let s = action_script(HostOs::Windows, &act("install", "winget", "Tailscale.Tailscale")).unwrap();
        let u = action_script(HostOs::Windows, &act("uninstall", "winget", "Tailscale.Tailscale")).unwrap();
        assert!(s.contains("--disable-interactivity") && u.contains("--disable-interactivity"));
        assert!(s.contains("--accept-package-agreements") && s.contains("--accept-source-agreements") && s.contains("'Tailscale.Tailscale'"));
        let scan = scan_script(HostOs::Windows);
        assert!(scan.contains("winget list --source winget --accept-source-agreements --disable-interactivity"));
        // whole-token: 'Git.Git' must not match 'Git.Git.LFS' or 'XGit.Git'
        assert!(scan.contains(r"-match ('(?m)(^|\s)' + [regex]::Escape($id) + '(\s|$)')"), "{scan}");
        let q = search_script(HostOs::Windows, &["winget".into()], "git").unwrap();
        assert!(q.contains("winget search --source winget --accept-source-agreements --disable-interactivity"));
        assert_eq!(action_script(HostOs::Windows, &InstallAction::InstallManager { manager: "winget".into() }), Err("WINGET_MANUAL".into()));
    }

    #[test]
    fn needs_root_matrix() {
        assert!(needs_root(HostOs::Linux, &act("install", "apt", "x")));
        assert!(needs_root(HostOs::Linux, &act("install", "npm", "x")), "distro npm prefix is root-owned");
        assert!(!needs_root(HostOs::Macos, &act("install", "brew", "x")));
        assert!(!needs_root(HostOs::Macos, &act("install", "npm", "x")));
        assert!(!needs_root(HostOs::Macos, &InstallAction::EnableService { service: "x".into() }));
        assert!(needs_root(HostOs::Linux, &InstallAction::Refresh));
        assert!(action_script(HostOs::Macos, &InstallAction::Refresh).is_err());
    }

    #[test]
    fn search_rejects_metachars() {
        assert!(search_script(HostOs::Linux, &["apt".into()], "a;rm -rf").is_err());
        let s = search_script(HostOs::Linux, &["apt".into(), "npm".into()], "jq").unwrap();
        assert!(s.contains("head -n 20") && s.contains("HIT:apt:") && s.contains("HIT:npm:"));
    }

    #[test]
    fn parse_search_splits_desc_with_colons() {
        let h = parse_search("junk\nHIT:apt:jq:JSON: processor\nHIT:brew:wget:\n");
        assert_eq!(h[0], SearchHit { manager: "apt".into(), package: "jq".into(), description: "JSON: processor".into() });
        assert_eq!(h[1].description, "");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_local_times_out() {
        let pool = crate::ssh_pool::SshPool::default();
        let f = std::env::temp_dir().join(format!("sshbox-timeout-{}", std::process::id()));
        let _ = std::fs::remove_file(&f);
        let script = format!("sh -c 'sleep 2; touch {}'", f.display());
        let e = run("local", &pool, HostOs::Linux, &script, None, false, false, 1).await.unwrap_err();
        assert!(e.contains("TIMEOUT"), "{e}");
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        assert!(!f.exists(), "grandchild survived the timeout");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_local_echo() {
        let out = run_local(HostOs::Linux, "echo hi", None, false, false).await.unwrap();
        assert_eq!(out.trim(), "hi");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_local_root_never_executes_password() {
        // Fake sudo that behaves like sudo with cached creds unless -k is given: it only reads
        // the password line when -k is present, then hands the rest of stdin to `sh -s`.
        let dir = std::env::temp_dir().join(format!("sshbox-fakesudo-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let sudo = dir.join("sudo");
        std::fs::write(&sudo, "#!/bin/sh\ncase \" $* \" in *\" -k \"*) IFS= read -r _pw;; esac\nexec sh -s\n").unwrap();
        std::fs::set_permissions(&sudo, std::os::unix::fs::PermissionsExt::from_mode(0o755)).unwrap();
        let old = std::env::var("PATH").unwrap();
        std::env::set_var("PATH", format!("{}:{old}", dir.display()));
        let r = run_local(HostOs::Linux, "echo ok", Some("echo LEAKED"), true, false).await;
        std::env::set_var("PATH", old);
        let _ = std::fs::remove_dir_all(&dir);
        let out = r.unwrap();
        assert!(!out.contains("LEAKED") && out.trim() == "ok", "{out}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn run_prepends_unix_path() {
        let pool = crate::ssh_pool::SshPool::default();
        let out = run("local", &pool, HostOs::Macos, "echo \"$PATH\"", None, false, false, 10).await.unwrap();
        assert!(out.starts_with("/opt/homebrew/bin:/usr/local/bin:"), "{out}");
        assert!(UNIX_PATH.starts_with(r#"export PATH="/opt/homebrew/bin:/usr/local/bin:$HOME/.npm-global/bin:$PATH""#));
    }
}
