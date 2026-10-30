// Tool Installer catalog and core types (spec §6). Pure: no I/O.
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use crate::vps_config::shell_single_quote as shq;
use crate::vps_windows::ps_quote;

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
        && s.chars().all(|c| c.is_ascii_alphanumeric() || "@/._+:-".contains(c))
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum ToolStatus {
    NotInstalled,
    Installed { via: Vec<String> },
    External,
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
fn bin_key(t: &CatalogTool, os: HostOs) -> Option<&str> {
    Some(t.bin.as_deref().unwrap_or(&t.id))
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
            "$w = winget list --source winget 2>$null | Out-String\nforeach ($id in @({})) {{ if ($w -match [regex]::Escape($id)) {{ ('PKG:winget:' + $id) }} }}\n",
            ids.join(",")
        );
        s += "npm ls -g --depth=0 --parseable 2>$null | ForEach-Object { if ($_ -match 'node_modules[\\\\/](.+)$') { 'PKG:npm:' + ($Matches[1] -replace '\\\\','/') } }\n";
        for t in tools() {
            if let Some(k) = bin_key(t, os) {
                s += &format!("{} + [int][bool](Get-Command {} -ErrorAction SilentlyContinue)\n", ps_quote(&format!("BIN:{k}:")), ps_quote(k));
            }
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
        if let Some(k) = bin_key(t, os) {
            s += &format!("if command -v {q} >/dev/null 2>&1; then echo {y}; else echo {n}; fi\n", q = shq(k), y = shq(&format!("BIN:{k}:1")), n = shq(&format!("BIN:{k}:0")));
        }
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
            let via: Vec<String> = os_methods(t, os)
                .filter(|m| pkgs.contains(&(m.manager.as_str(), m.package.as_str())))
                .map(|m| m.manager.clone())
                .collect();
            let status = if !via.is_empty() {
                ToolStatus::Installed { via }
            } else if bin_key(t, os).is_some_and(|k| bins.contains(k)) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_parses_and_ids_unique() {
        let c = catalog();
        assert!(!c.is_empty());
        let mut ids: Vec<&str> = c.iter().map(|t| t.id.as_str()).collect();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), c.len());
        for t in c {
            for m in &t.methods {
                assert!(managers_for(m.os).contains(&m.manager.as_str()), "{} {}", t.id, m.manager);
                assert!(valid_name(&m.package));
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
        for bad in ["a;b", "a b", "$(x)", ""] {
            assert!(!valid_name(bad), "{bad}");
        }
        assert!(!valid_name(&"a".repeat(129)));
    }

    fn scan_tool<'a>(s: &'a InstallerScan, id: &str) -> &'a ToolView {
        s.tools.iter().find(|t| t.id == id).unwrap()
    }

    #[test]
    fn parse_scan_linux_owner_detection() {
        let s = parse_scan(HostOs::Linux, "MGR:apt:1\nPKG:apt:docker.io\nSVC:docker:1:0\n");
        let d = scan_tool(&s, "docker");
        assert_eq!(d.status, ToolStatus::Installed { via: vec!["apt".into()] });
        assert_eq!(d.service_state, Some(ServiceState { enabled: true, active: false }));
    }

    #[test]
    fn parse_scan_two_owners() {
        // docker is the catalog tool that has both an apt and a snap method.
        let s = parse_scan(HostOs::Linux, "MGR:apt:1\nMGR:snap:1\nPKG:apt:docker.io\nPKG:snap:docker\n");
        assert_eq!(scan_tool(&s, "docker").status, ToolStatus::Installed { via: vec!["apt".into(), "snap".into()] });
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
        assert_eq!(scan_tool(&s, "claude-code").status, ToolStatus::Installed { via: vec!["npm".into()] });
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
}
