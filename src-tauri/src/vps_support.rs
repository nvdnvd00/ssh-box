// Which Server Config tabs and sshd rows each host OS gets (spec §3). Pure: no I/O.
use crate::vps_config::{FirewallEngine, PrivilegeLevel, SshdSettings, VpsProbe, ERR_UNSUPPORTED};
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

// Flipped only once the Windows write path is verified on a real host (Task 10).
pub const WINDOWS_SSH_WRITE_VERIFIED: bool = true;

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
            access: tab(ReadOnly, Some("read_only_os")),
            password_coupled: true,
        },
        OsFamily::Windows => VpsSupport {
            ssh: match (p.privilege == PrivilegeLevel::Admin, p.caps.sshd_running) {
                (false, _) => tab(ReadOnly, Some("windows_not_admin")),
                (_, false) => tab(ReadOnly, Some("windows_no_sshd")),
                _ if WINDOWS_SSH_WRITE_VERIFIED && p.caps.has_scheduled_tasks => tab(Full, None),
                _ => tab(ReadOnly, Some("windows_ssh_read_only")),
            },
            ssh_rows: SshRows { permit_root_login: false, allow_deny_users: false, ..all_rows },
            firewall: match (p.caps.has_net_security, p.privilege == PrivilegeLevel::Admin) {
                (false, _) => tab(Hidden, None),
                (_, false) => tab(ReadOnly, Some("windows_not_admin")),
                _ => tab(Full, None),
            },
            access: tab(ReadOnly, Some("read_only_os")),
            // Windows sshd keeps offering keyboard-interactive when only PasswordAuthentication is off.
            password_coupled: true,
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

    // macOS Users & Keys is view-only, with the reason the UI shows as a notice.
    #[test]
    fn macos_support_access_read_only() {
        let mut p = linux(true, "none");
        p.os_family = OsFamily::Macos;
        assert_eq!(compute_support(&p).access, tab(SupportLevel::ReadOnly, Some("read_only_os")));
    }

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

    // Windows ssh is writable only with admin + running sshd + scheduled tasks (the rollback needs them); password is coupled to keyboard-interactive.
    #[test]
    fn windows_ssh_is_full_only_with_rollback_prerequisites() {
        let mut p = linux(true, "ufw");
        p.os_family = OsFamily::Windows;
        p.privilege = PrivilegeLevel::Admin;
        p.caps.sshd_running = true;
        assert_eq!(compute_support(&p).ssh.reason.as_deref(), Some("windows_ssh_read_only"));
        p.caps.has_scheduled_tasks = true;
        let s = compute_support(&p);
        assert_eq!((s.ssh, s.access.level, s.firewall.level), (tab(SupportLevel::Full, None), SupportLevel::ReadOnly, SupportLevel::Hidden));
        assert!(!s.ssh_rows.permit_root_login && !s.ssh_rows.allow_deny_users && s.ssh_rows.port && s.password_coupled);
    }

    // The notice must name the real blocker first: no admin, then no sshd, then the generic read-only note.
    #[test]
    fn windows_support_reasons_in_priority_order() {
        let mut p = linux(true, "ufw");
        p.os_family = OsFamily::Windows;
        let why = |p: &VpsProbe| compute_support(p).ssh.reason.unwrap();
        assert_eq!(why(&p), "windows_not_admin");
        p.privilege = PrivilegeLevel::Admin;
        assert_eq!(why(&p), "windows_no_sshd");
        p.caps.sshd_running = true;
        assert_eq!(why(&p), "windows_ssh_read_only");
        assert_eq!(compute_support(&p).access, tab(SupportLevel::ReadOnly, Some("read_only_os")));
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

    // Windows firewall is writable for admins with NetSecurity; without NetSecurity it is hidden, without admin the reason is named.
    #[test]
    fn windows_firewall_is_full_for_admin_with_net_security() {
        let mut p = linux(true, "ufw");
        p.os_family = OsFamily::Windows;
        p.privilege = PrivilegeLevel::Admin;
        assert_eq!(compute_support(&p).firewall.level, SupportLevel::Hidden);
        p.caps.has_net_security = true;
        assert_eq!(compute_support(&p).firewall, tab(SupportLevel::Full, None));
        p.privilege = PrivilegeLevel::None;
        assert_eq!(compute_support(&p).firewall, tab(SupportLevel::ReadOnly, Some("windows_not_admin")));
    }
}
