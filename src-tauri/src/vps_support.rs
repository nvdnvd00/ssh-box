// Which Server Config tabs and sshd rows each host OS gets (spec §3). Pure: no I/O.
use crate::vps_config::{FirewallEngine, SshdSettings, VpsProbe, ERR_UNSUPPORTED};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum OsFamily {
    Linux,
    Macos,
    Windows,
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SupportLevel {
    Full,
    ReadOnly,
    #[default]
    Hidden,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TabSupport {
    pub level: SupportLevel,
    // Key in t.vpsUnsupportedReasons.
    pub reason: Option<String>,
}

// true = the row is shown.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshRows {
    pub port: bool,
    pub permit_root_login: bool,
    pub allow_deny_users: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VpsSupport {
    pub ssh: TabSupport,
    pub ssh_rows: SshRows,
    pub firewall: TabSupport,
    pub access: TabSupport,
    pub password_coupled: bool,
}

// What the OS probe found beyond the shared fields; backend-only.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct OsCaps {
    pub sshd_dropin: bool,
    pub sshd_running: bool,
    pub has_scheduled_tasks: bool,
    pub has_net_security: bool,
}

fn tab(level: SupportLevel, reason: Option<&str>) -> TabSupport {
    TabSupport { level, reason: reason.map(str::to_string) }
}

pub fn compute_support(p: &VpsProbe) -> VpsSupport {
    use SupportLevel::*;
    let all_rows = SshRows { port: true, permit_root_login: true, allow_deny_users: true };
    match p.os_family {
        OsFamily::Linux => VpsSupport {
            ssh: if p.has_systemd { tab(Full, None) } else { tab(ReadOnly, Some("no_systemd")) },
            ssh_rows: all_rows,
            firewall: match p.firewall_engine {
                FirewallEngine::Ufw | FirewallEngine::Firewalld => tab(Full, None),
                FirewallEngine::Iptables => tab(ReadOnly, Some("firewall_read_only")),
                _ => tab(ReadOnly, None),
            },
            access: tab(Full, None),
            password_coupled: false,
        },
        OsFamily::Macos => VpsSupport {
            ssh: if p.caps.sshd_dropin { tab(Full, None) } else { tab(ReadOnly, Some("macos_no_dropin")) },
            ssh_rows: SshRows { port: false, ..all_rows },
            firewall: tab(Hidden, None),
            access: tab(Hidden, None),
            password_coupled: true,
        },
        // Later tasks open these tab by tab.
        OsFamily::Windows => VpsSupport {
            ssh: tab(Hidden, None),
            ssh_rows: all_rows,
            firewall: tab(Hidden, None),
            access: tab(Hidden, None),
            password_coupled: false,
        },
    }
}

// A hidden row cannot be edited, so a differing value must be refused rather than silently written.
pub fn check_hidden_rows(rows: &SshRows, current: &SshdSettings, desired: &SshdSettings) -> Result<(), String> {
    let changed = (!rows.port && current.port != desired.port)
        || (!rows.permit_root_login && current.permit_root_login != desired.permit_root_login)
        || (!rows.allow_deny_users
            && (current.allow_users != desired.allow_users || current.deny_users != desired.deny_users));
    if changed {
        return Err(crate::vps_config::vps_err(ERR_UNSUPPORTED, "hidden_setting"));
    }
    Ok(())
}

// None = not a Linux/macOS shell (uname failed or Git-for-Windows style): try the Windows probe.
pub fn unix_family(uname_stdout: Option<&str>) -> Option<OsFamily> {
    match uname_stdout?.trim() {
        "Linux" => Some(OsFamily::Linux),
        "Darwin" => Some(OsFamily::Macos),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_pool::AuthUsed;
    use crate::vps_config::parse_probe_output;

    fn linux(systemd: bool, fw: &str) -> VpsProbe {
        let raw = format!(
            "UNAME:Linux\nUSER:u\nUID:1000\nSUDO:nopasswd\nSSH_CONNECTION:1.1.1.1 5 2.2.2.2 22\nSYSTEMD:{}\nFW:{fw}\n",
            if systemd { 1 } else { 0 }
        );
        parse_probe_output(&raw, AuthUsed::Key { public_key_body: "k".into() }).unwrap()
    }

    // Linux users must see exactly what they saw before multi-OS support.
    #[test]
    fn linux_support_reproduces_current_behaviour() {
        let s = compute_support(&linux(true, "ufw"));
        assert_eq!((s.ssh, s.firewall, s.access), (tab(SupportLevel::Full, None), tab(SupportLevel::Full, None), tab(SupportLevel::Full, None)));
        assert!(s.ssh_rows.port && s.ssh_rows.permit_root_login && s.ssh_rows.allow_deny_users && !s.password_coupled);
        let s = compute_support(&linux(false, "iptables"));
        assert_eq!(s.ssh, tab(SupportLevel::ReadOnly, Some("no_systemd")));
        assert_eq!(s.firewall, tab(SupportLevel::ReadOnly, Some("firewall_read_only")));
        assert_eq!(compute_support(&linux(true, "none")).firewall, tab(SupportLevel::ReadOnly, None));
        assert_eq!(compute_support(&linux(true, "firewalld")).firewall.level, SupportLevel::Full);
    }

    // Until a later task adds a real probe, a Windows host must not expose any write path.
    #[test]
    fn windows_starts_hidden() {
        let mut p = linux(true, "ufw");
        p.os_family = OsFamily::Windows;
        let s = compute_support(&p);
        assert!([s.ssh.level, s.firewall.level, s.access.level].iter().all(|l| *l == SupportLevel::Hidden));
    }

    // Git for Windows answers `uname`; it must still reach the Windows probe, not be treated as unix.
    #[test]
    fn uname_from_git_for_windows_falls_through_to_windows_probe() {
        assert_eq!(unix_family(Some("MINGW64_NT-10.0-22631")), None);
        assert_eq!(unix_family(Some("CYGWIN_NT-10.0")), None);
    }

    #[test]
    fn uname_linux_and_darwin_are_unix() {
        assert_eq!(unix_family(Some("Linux\n")), Some(OsFamily::Linux));
        assert_eq!(unix_family(Some("Darwin")), Some(OsFamily::Macos));
        assert_eq!(unix_family(None), None);
    }

    // A row the user cannot see must not be changed behind their back.
    #[test]
    fn hidden_row_change_is_rejected() {
        let rows = SshRows { port: false, permit_root_login: true, allow_deny_users: true };
        let desired = SshdSettings { port: 2222, ..SshdSettings::default() };
        let e = check_hidden_rows(&rows, &SshdSettings::default(), &desired).unwrap_err();
        assert!(e.starts_with("UNSUPPORTED: hidden_setting"), "{e}");
        let hide_users = SshRows { allow_deny_users: false, ..rows };
        let desired = SshdSettings { deny_users: vec!["x".into()], ..SshdSettings::default() };
        assert!(check_hidden_rows(&hide_users, &SshdSettings::default(), &desired).is_err());
    }

    #[test]
    fn visible_row_change_passes() {
        let rows = SshRows { port: true, permit_root_login: true, allow_deny_users: false };
        let desired = SshdSettings { port: 2222, ..SshdSettings::default() };
        assert_eq!(check_hidden_rows(&rows, &SshdSettings::default(), &desired), Ok(()));
    }
}
