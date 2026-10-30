use crate::ssh_pool::{AuthUsed, SshPool};
use crate::vps_macos::{build_sshd_apply_script_macos, MACOS_ROLLBACK_LABEL, MACOS_ROLLBACK_PLIST};
use crate::vps_support::{check_hidden_rows, compute_support, unix_family, OsCaps, OsFamily, SupportLevel, VpsSupport};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PrivilegeLevel {
    Root,
    SudoNoPassword,
    SudoPassword,
    Admin,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FirewallEngine {
    Ufw,
    Firewalld,
    Iptables,
    Windows,
    None,
}

// What the app is using to reach this host right now (spec §4.1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionContext {
    pub login_user: String,
    pub auth: AuthUsed,
    pub client_ip: String,
    pub server_ip: String,
    pub server_port: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VpsProbe {
    pub privilege: PrivilegeLevel,
    pub context: ConnectionContext,
    pub os_id: String,
    pub os_version: String,
    pub openssh_version: String,
    pub has_systemd: bool,
    pub ssh_socket_active: bool,
    pub selinux_enforcing: bool,
    pub firewall_engine: FirewallEngine,
    pub os_family: OsFamily,
    pub support: VpsSupport,
    #[serde(skip)]
    pub caps: OsCaps,
}

// The sshd directives the SSH studio reads and writes (spec §1.1).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshdSettings {
    pub port: u16,
    pub permit_root_login: String,
    pub password_authentication: bool,
    pub pubkey_authentication: bool,
    pub allow_users: Vec<String>,
    pub deny_users: Vec<String>,
    pub max_auth_tries: u32,
    pub max_sessions: u32,
    pub login_grace_time: u32,
    pub client_alive_interval: u32,
    pub client_alive_count_max: u32,
    // macOS: password login also goes through keyboard-interactive, so the UI toggles both together.
    pub kbd_interactive_authentication: bool,
}

// OpenSSH defaults, used when `sshd -T` omits a directive.
impl Default for SshdSettings {
    fn default() -> Self {
        Self {
            port: 22,
            permit_root_login: "prohibit-password".to_string(),
            password_authentication: true,
            pubkey_authentication: true,
            allow_users: Vec::new(),
            deny_users: Vec::new(),
            max_auth_tries: 6,
            max_sessions: 10,
            login_grace_time: 120,
            client_alive_interval: 0,
            client_alive_count_max: 3,
            kbd_interactive_authentication: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WriteTarget {
    DropIn,
    #[default]
    MainFile,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshdView {
    pub settings: SshdSettings,
    pub write_target: WriteTarget,
    pub rollback_pending: bool,
    // Server-side inputs for planning a change; not sent to the UI.
    #[serde(skip)]
    pub main_raw: String,
    #[serde(skip)]
    pub dropin_raw: String,
    #[serde(skip)]
    pub port_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ListeningSocketItem {
    pub protocol: String,
    pub bind_addr: String,
    pub port: u16,
    pub is_public: bool,
    pub process_name: String,
    pub pid: Option<u32>,
    pub exposure: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirewallRuleItem {
    pub index: u32,
    pub to_port: String,
    pub protocol: String,
    pub action: String,
    pub from_source: String,
    pub v6: bool,
    pub comment: String,
    pub raw_spec: String,
    // false: listed, but not a plain port rule of an active profile (Windows only); true everywhere else.
    #[serde(default = "default_effective")]
    pub effective: bool,
}

fn default_effective() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct FirewallSnapshot {
    pub engine: String,
    pub enabled: bool,
    pub default_incoming: String,
    pub default_outgoing: String,
    pub ipv6_filtered: bool,
    pub rules: Vec<FirewallRuleItem>,
    pub listening_sockets: Vec<ListeningSocketItem>,
}

impl Default for FirewallSnapshot {
    fn default() -> Self {
        Self {
            engine: "ufw".to_string(),
            enabled: false,
            default_incoming: "deny".to_string(),
            default_outgoing: "allow".to_string(),
            ipv6_filtered: true,
            rules: Vec::new(),
            listening_sockets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RemoteAuthorizedKey {
    pub index: u32,
    pub key_type: String,
    pub public_key_body: String,
    pub comment: String,
    pub short_preview: String,
    pub raw_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct LinuxAccountItem {
    pub username: String,
    pub uid: u32,
    pub gid: u32,
    pub home_dir: String,
    pub shell: String,
    pub locked: bool,
    pub groups: Vec<String>,
    pub has_sudo: bool,
    pub nopasswd_sudo: bool,
    pub authorized_keys: Vec<RemoteAuthorizedKey>,
    // Windows only: the file the keys were read from; empty elsewhere.
    #[serde(default)]
    pub keys_file: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccessKeysSnapshot {
    pub accounts: Vec<LinuxAccountItem>,
}

pub(crate) fn is_public_bind_address(bind_addr: &str) -> bool {
    let clean = bind_addr
        .trim()
        .trim_matches(|c| c == '[' || c == ']')
        .to_ascii_lowercase();
    let host_part = clean.split('%').next().unwrap_or(&clean);
    !(host_part.starts_with("127.")
        || host_part == "::1"
        || host_part == "localhost"
        || host_part.starts_with("::ffff:127."))
}

pub(crate) fn rule_port_matches(rule_to_port: &str, rule_proto: &str, sock_port: u16, sock_proto: &str) -> bool {
    if rule_proto != "any" && !rule_proto.eq_ignore_ascii_case(sock_proto) {
        return false;
    }
    let clean_to = rule_to_port.trim();
    if clean_to.eq_ignore_ascii_case("anywhere") || clean_to.eq_ignore_ascii_case("any") {
        return true;
    }
    let port_part = clean_to
        .split('/')
        .next()
        .unwrap_or(clean_to)
        .trim();
    if port_part.eq_ignore_ascii_case("anywhere") || port_part.eq_ignore_ascii_case("any") {
        return true;
    }
    for token in port_part.split(',') {
        let tok = token.trim();
        if let Some((start_s, end_s)) = tok.split_once(':').or_else(|| tok.split_once('-')) {
            if let (Ok(start), Ok(end)) = (start_s.trim().parse::<u16>(), end_s.trim().parse::<u16>())
            {
                if sock_port >= start && sock_port <= end {
                    return true;
                }
            }
        } else if let Ok(p) = tok.parse::<u16>() {
            if p == sock_port {
                return true;
            }
        }
    }
    false
}

pub(crate) fn is_anywhere_source(src: &str) -> bool {
    let s = src.trim().to_ascii_lowercase();
    s.is_empty() || s == "anywhere" || s == "any" || s == "0.0.0.0/0" || s == "::/0" || s == "*"
}

// Rules are evaluated top-down per address family; the first rule matching a source decides.
pub fn compute_socket_exposure(
    socket: &ListeningSocketItem,
    firewall_enabled: bool,
    default_incoming: &str,
    ipv6_filtered: bool,
    rules: &[FirewallRuleItem],
) -> String {
    if !socket.is_public {
        return "loopback".to_string();
    }
    if !firewall_enabled {
        return "exposed".to_string();
    }
    let bind = socket.bind_addr.trim_matches(|c| c == '[' || c == ']');
    // "::" and "*" are dual-stack on Linux; a specific IPv6 address is IPv6 only.
    let families: &[bool] = match bind {
        "*" | "::" => &[false, true],
        b if b.contains(':') => &[true],
        _ => &[false],
    };
    let default_blocks = matches!(default_incoming.to_ascii_lowercase().as_str(), "deny" | "reject" | "drop");
    let family = |v6: bool| -> u8 {
        if v6 && !ipv6_filtered {
            return 2;
        }
        let mut restricted = false;
        for r in rules.iter().filter(|r| {
            r.v6 == v6 && rule_port_matches(&r.to_port, &r.protocol, socket.port, &socket.protocol)
        }) {
            let allow = matches!(r.action.to_ascii_uppercase().as_str(), "ALLOW" | "LIMIT" | "ACCEPT");
            match (is_anywhere_source(&r.from_source), allow) {
                (true, true) => return 2,
                (true, false) => return u8::from(restricted),
                (false, true) => restricted = true,
                (false, false) => {}
            }
        }
        if !default_blocks {
            2
        } else {
            u8::from(restricted)
        }
    };
    match families.iter().map(|v6| family(*v6)).max().unwrap_or(2) {
        2 => "exposed",
        1 => "restricted",
        _ => "blocked",
    }
    .to_string()
}

// Windows Firewall: a block rule beats any allow regardless of order, so there is no first-match walk.
pub fn compute_socket_exposure_windows(
    socket: &ListeningSocketItem,
    firewall_enabled: bool,
    rules: &[FirewallRuleItem],
) -> String {
    if !socket.is_public {
        return "loopback".to_string();
    }
    if !firewall_enabled {
        return "exposed".to_string();
    }
    let (mut allow_any, mut allow_src, mut block_any, mut block_src) = (false, false, false, false);
    for r in rules.iter().filter(|r| rule_port_matches(&r.to_port, &r.protocol, socket.port, &socket.protocol)) {
        let allow = matches!(r.action.to_ascii_uppercase().as_str(), "ALLOW" | "LIMIT" | "ACCEPT");
        match (allow, is_anywhere_source(&r.from_source)) {
            (true, true) => allow_any = true,
            (true, false) => allow_src = true,
            (false, true) => block_any = true,
            (false, false) => block_src = true,
        }
    }
    // No rule for the port says nothing about the default: never claim "blocked" from silence.
    match (block_any, allow_any, allow_src, block_src) {
        (true, ..) => "blocked",
        // An allow-from-anywhere stays open to everyone except the one blocked source.
        (_, true, ..) => "exposed",
        (_, false, true, _) => "restricted",
        _ => "unknown",
    }
    .to_string()
}

fn parse_ufw_numbered_line(line: &str) -> Option<FirewallRuleItem> {
    let trimmed = line.trim();
    if !trimmed.starts_with('[') {
        return None;
    }
    let close_bracket = trimmed.find(']')?;
    let idx_str = trimmed[1..close_bracket].trim();
    let index = idx_str.parse::<u32>().ok()?;

    let after_bracket = trimmed[close_bracket + 1..].trim();
    let (main_part, comment) = match after_bracket.split_once('#') {
        Some((m, c)) => (m.trim(), c.trim().to_string()),
        None => (after_bracket, String::new()),
    };

    // Locate action token (ALLOW, LIMIT, DENY, REJECT)
    let action_keywords = ["ALLOW", "LIMIT", "DENY", "REJECT"];
    let mut found_action: Option<(&str, usize, usize)> = None;
    let tokens: Vec<&str> = main_part.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }

    // Find byte offset of the action keyword in main_part
    let mut search_from = 0usize;
    for tok in &tokens {
        let pos = main_part[search_from..].find(tok)? + search_from;
        let end_pos = pos + tok.len();
        if let Some(&kw) = action_keywords.iter().find(|&&k| k == *tok) {
            found_action = Some((kw, pos, end_pos));
            break;
        }
        search_from = end_pos;
    }

    let (action_kw, act_start, mut act_end) = found_action?;
    let to_raw = main_part[..act_start].trim();
    if to_raw.is_empty() {
        return None;
    }

    let rest_after_act = main_part[act_end..].trim_start();
    // Skip optional direction keyword: IN, OUT, FWD
    for dir_kw in ["IN", "OUT", "FWD"] {
        if let Some(stripped) = rest_after_act.strip_prefix(dir_kw) {
            if stripped.is_empty() || stripped.starts_with(char::is_whitespace) {
                act_end = main_part.len() - stripped.len();
                break;
            }
        }
    }

    let from_raw = main_part[act_end..].trim();
    let v6 = to_raw.contains("(v6)") || from_raw.contains("(v6)");
    let to_clean = to_raw.replace("(v6)", "").trim().to_string();
    let from_clean = {
        let s = from_raw.replace("(v6)", "").trim().to_string();
        if s.is_empty() {
            "Anywhere".to_string()
        } else {
            s
        }
    };

    let protocol = if to_clean.ends_with("/tcp") {
        "tcp".to_string()
    } else if to_clean.ends_with("/udp") {
        "udp".to_string()
    } else {
        "any".to_string()
    };

    Some(FirewallRuleItem {
        index,
        to_port: to_clean,
        protocol,
        action: action_kw.to_string(),
        from_source: from_clean,
        v6,
        comment,
        raw_spec: after_bracket.to_string(), effective: true,
    })
}

fn parse_iptables_input_line(line: &str, next_index: u32) -> Option<FirewallRuleItem> {
    let trimmed = line.trim();
    let spec = trimmed.strip_prefix("-A INPUT ")?.trim();
    // Skip loopback and stateful conntrack boilerplate rules
    if spec.contains("-i lo") || spec.contains("RELATED,ESTABLISHED") || spec.contains("ESTABLISHED") {
        return None;
    }

    let mut from_source = "Anywhere".to_string();
    let mut protocol = "any".to_string();
    let mut dport: Option<String> = None;
    let mut target: Option<String> = None;
    let mut comment = String::new();

    // Extract --comment "..." if quoted
    if let Some(c_idx) = spec.find("--comment ") {
        let after_c = spec[c_idx + "--comment ".len()..].trim_start();
        if let Some(stripped) = after_c.strip_prefix('"') {
            if let Some(end_q) = stripped.find('"') {
                comment = stripped[..end_q].to_string();
            }
        } else if let Some(tok) = after_c.split_whitespace().next() {
            comment = tok.trim_matches('"').to_string();
        }
    }

    let tokens: Vec<&str> = spec.split_whitespace().collect();
    let mut i = 0;
    while i < tokens.len() {
        match tokens[i] {
            "-s" if i + 1 < tokens.len() => {
                let raw_src = tokens[i + 1];
                from_source = raw_src
                    .strip_suffix("/32")
                    .unwrap_or(raw_src)
                    .to_string();
                i += 2;
            }
            "-p" if i + 1 < tokens.len() => {
                protocol = tokens[i + 1].to_ascii_lowercase();
                i += 2;
            }
            "--dport" | "--dports" if i + 1 < tokens.len() => {
                dport = Some(tokens[i + 1].to_string());
                i += 2;
            }
            "-j" if i + 1 < tokens.len() => {
                target = Some(tokens[i + 1].to_ascii_uppercase());
                i += 2;
            }
            _ => {
                i += 1;
            }
        }
    }

    let action = match target.as_deref()? {
        "ACCEPT" => "ALLOW",
        "DROP" => "DENY",
        "REJECT" => "REJECT",
        "LIMIT" => "LIMIT",
        _ => return None,
    }
    .to_string();

    let to_port = match dport {
        Some(p) => {
            if protocol == "tcp" || protocol == "udp" {
                format!("{p}/{protocol}")
            } else {
                p
            }
        }
        None => "Anywhere".to_string(),
    };

    Some(FirewallRuleItem {
        index: next_index,
        to_port,
        protocol,
        action,
        from_source,
        v6: false,
        comment,
        raw_spec: spec.to_string(), effective: true,
    })
}

fn extract_ss_process(line: &str) -> (String, Option<u32>) {
    if let Some(u_idx) = line.find("users:((") {
        let rest = &line[u_idx + "users:((".len()..];
        let proc_name = if let Some(stripped) = rest.strip_prefix('"') {
            stripped
                .split('"')
                .next()
                .unwrap_or("unknown")
                .to_string()
        } else {
            "unknown".to_string()
        };
        let pid = rest.find("pid=").and_then(|p_idx| {
            let after_pid = &rest[p_idx + 4..];
            let digits: String = after_pid
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            digits.parse::<u32>().ok()
        });
        return (proc_name, pid);
    }
    ("system".to_string(), None)
}

fn parse_socket_line(line: &str) -> Option<ListeningSocketItem> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() < 5 {
        return None;
    }
    let proto_tok = tokens[0].to_ascii_lowercase();
    let protocol = if proto_tok.starts_with("tcp") {
        "tcp".to_string()
    } else if proto_tok.starts_with("udp") {
        "udp".to_string()
    } else {
        return None;
    };

    // Determine whether this is `ss -tulpn` or `netstat -tulpn`
    let is_netstat = tokens[1].chars().all(|c| c.is_ascii_digit());
    let local_addr_port: &str;
    let mut process_name = "system".to_string();
    let mut pid: Option<u32> = None;

    if is_netstat {
        // netstat: Proto Recv-Q Send-Q Local-Address Foreign-Address [State] [PID/Program]
        local_addr_port = tokens[3];
        if protocol == "tcp" && !tokens.iter().any(|t| *t == "LISTEN") {
            return None;
        }
        if let Some(last_tok) = tokens.last() {
            if let Some((pid_s, name_s)) = last_tok.split_once('/') {
                if let Ok(p) = pid_s.parse::<u32>() {
                    pid = Some(p);
                    process_name = name_s.trim_end_matches(':').to_string();
                }
            }
        }
    } else {
        // ss: Netid State Recv-Q Send-Q Local-Address:Port Peer-Address:Port [Process]
        let state = tokens[1].to_ascii_uppercase();
        if protocol == "tcp" && state != "LISTEN" {
            return None;
        }
        local_addr_port = tokens[4];
        let (pname, p_id) = extract_ss_process(trimmed);
        process_name = pname;
        pid = p_id;
    }

    let (raw_addr, port_str) = local_addr_port.rsplit_once(':')?;
    let port = port_str.parse::<u16>().ok()?;
    let bind_addr = match raw_addr.trim() {
        "*" => "0.0.0.0".to_string(),
        "[::]" | "::" => "::".to_string(),
        "[::1]" => "::1".to_string(),
        other => other.trim_matches(|c| c == '[' || c == ']').to_string(),
    };
    let is_public = is_public_bind_address(&bind_addr);

    Some(ListeningSocketItem {
        protocol,
        bind_addr,
        port,
        is_public,
        process_name,
        pid,
        exposure: String::new(),
    })
}

pub fn parse_firewall_snapshot(raw: &str) -> FirewallSnapshot {
    let mut snap = FirewallSnapshot::default();
    let mut section = "";
    let mut firewalld_text = String::new();
    let mut ipt_idx = 1u32;

    for line in raw.lines() {
        let trimmed = line.trim();
        match trimmed {
            "---META---" => {
                section = "meta";
                continue;
            }
            "---UFW---" => {
                section = "ufw";
                continue;
            }
            "---IPTABLES---" => {
                section = "iptables";
                continue;
            }
            "---FIREWALLD---" => {
                section = "firewalld";
                continue;
            }
            "---SOCKETS---" => {
                section = "sockets";
                continue;
            }
            _ => {}
        }

        match section {
            "meta" => {
                if let Some(v) = trimmed.strip_prefix("ENGINE:") {
                    let e = v.trim().to_ascii_lowercase();
                    if !e.is_empty() {
                        snap.engine = e;
                    }
                } else if let Some(v) = trimmed.strip_prefix("ENABLED:") {
                    snap.enabled = parse_yes_no(v, false);
                } else if let Some(v) = trimmed.strip_prefix("DEFAULT_INCOMING:") {
                    let d = v.trim().to_ascii_lowercase();
                    if !d.is_empty() {
                        snap.default_incoming = d;
                    }
                } else if let Some(v) = trimmed.strip_prefix("DEFAULT_OUTGOING:") {
                    let d = v.trim().to_ascii_lowercase();
                    if !d.is_empty() {
                        snap.default_outgoing = d;
                    }
                } else if let Some(v) = trimmed.strip_prefix("IPV6:") {
                    snap.ipv6_filtered = parse_yes_no(v, true);
                }
            }
            "ufw" => {
                if let Some(st) = trimmed.strip_prefix("Status:") {
                    let st_lower = st.trim().to_ascii_lowercase();
                    if st_lower.starts_with("active") {
                        snap.enabled = true;
                    } else if st_lower.starts_with("inactive") {
                        snap.enabled = false;
                    }
                } else if let Some(rule) = parse_ufw_numbered_line(trimmed) {
                    snap.rules.push(rule);
                }
            }
            "iptables" => {
                if snap.engine != "ufw" {
                    if let Some(rule) = parse_iptables_input_line(trimmed, ipt_idx) {
                        ipt_idx += 1;
                        snap.rules.push(rule);
                    }
                }
            }
            "firewalld" => {
                firewalld_text.push_str(line);
                firewalld_text.push('\n');
            }
            "sockets" => {
                if let Some(mut sock) = parse_socket_line(trimmed) {
                    let exists = snap.listening_sockets.iter().any(|s| {
                        s.protocol == sock.protocol
                            && s.bind_addr == sock.bind_addr
                            && s.port == sock.port
                    });
                    if !exists {
                        sock.exposure = compute_socket_exposure(
                            &sock,
                            snap.enabled,
                            &snap.default_incoming,
                            snap.ipv6_filtered,
                            &snap.rules,
                        );
                        snap.listening_sockets.push(sock);
                    }
                }
            }
            _ => {}
        }
    }

    snap.rules.extend(parse_firewalld_list_all(&firewalld_text));
    for sock in &mut snap.listening_sockets {
        sock.exposure = compute_socket_exposure(
            sock,
            snap.enabled,
            &snap.default_incoming,
            snap.ipv6_filtered,
            &snap.rules,
        );
    }

    snap
}

// Read-only snapshot script for the engine the probe found.
fn firewall_read_script(engine: FirewallEngine) -> String {
    let meta = match engine {
        FirewallEngine::Ufw => r#"v=$(ufw status verbose 2>/dev/null || true)
if printf '%s\n' "$v" | grep -qi '^Status:[[:space:]]*active'; then echo ENABLED:1; else echo ENABLED:0; fi
if printf '%s\n' "$v" | grep -qi 'allow (incoming)'; then echo DEFAULT_INCOMING:allow; else echo DEFAULT_INCOMING:deny; fi
if grep -q '^IPV6=yes' /etc/default/ufw 2>/dev/null; then echo IPV6:1; else echo IPV6:0; fi
echo ---UFW---
ufw status numbered 2>/dev/null || true"#,
        FirewallEngine::Firewalld => r#"if firewall-cmd --state >/dev/null 2>&1; then echo ENABLED:1; else echo ENABLED:0; fi
echo DEFAULT_INCOMING:deny
echo IPV6:1
echo ---FIREWALLD---
firewall-cmd --list-all 2>/dev/null || firewall-offline-cmd --list-all 2>/dev/null || true"#,
        FirewallEngine::Iptables => r#"if iptables -S INPUT 2>/dev/null | grep -qE -- '-P INPUT (DROP|REJECT)|^-A INPUT'; then echo ENABLED:1; else echo ENABLED:0; fi
if iptables -S INPUT 2>/dev/null | grep -qE -- '-P INPUT (DROP|REJECT)'; then echo DEFAULT_INCOMING:deny; else echo DEFAULT_INCOMING:allow; fi
if ip6tables -S INPUT 2>/dev/null | grep -qE -- '-P INPUT (DROP|REJECT)|^-A INPUT'; then echo IPV6:1; else echo IPV6:0; fi
echo ---IPTABLES---
iptables -S INPUT 2>/dev/null || true"#,
        FirewallEngine::None | FirewallEngine::Windows => "echo ENABLED:0\necho DEFAULT_INCOMING:allow\necho IPV6:0",
    };
    format!(
        "echo ---META---\n{meta}\necho ---SOCKETS---\nss -tulpnH 2>/dev/null || ss -tulpn 2>/dev/null || true\n"
    )
}

fn engine_name(engine: FirewallEngine) -> &'static str {
    match engine {
        FirewallEngine::Ufw => "ufw",
        FirewallEngine::Firewalld => "firewalld",
        FirewallEngine::Iptables => "iptables",
        FirewallEngine::Windows => "windows",
        FirewallEngine::None => "none",
    }
}

fn snapshot_after(raw: &str, engine: FirewallEngine) -> FirewallSnapshot {
    let body = raw.rsplit("SSHBOX_SNAPSHOT_BEGIN").next().unwrap_or(raw);
    let mut snap = parse_firewall_snapshot(body);
    snap.engine = engine_name(engine).to_string();
    snap
}

pub async fn get_firewall_snapshot(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    if probe.firewall_engine == FirewallEngine::Windows {
        return crate::vps_windows::get_windows_firewall(host_id, pool, &probe).await;
    }
    let raw = run_privileged_script(host_id, pool, sudo_password, &firewall_read_script(probe.firewall_engine), None).await?;
    Ok(snapshot_after(&raw, probe.firewall_engine))
}

pub(crate) struct UfwRuleArgs {
    pub action: &'static str,
    pub source: String,
    pub port: String,
    pub proto: &'static str,
}

pub(crate) fn validate_rule_args(action: &str, source: &str, port: &str, proto: &str) -> Result<UfwRuleArgs, String> {
    let action = match action.trim().to_ascii_lowercase().as_str() {
        "allow" => "allow",
        "limit" => "limit",
        "deny" => "deny",
        "reject" => "reject",
        other => return Err(vps_err(ERR_COMMAND_FAILED, format!("unsupported action {other}"))),
    };
    let proto = match proto.trim().to_ascii_lowercase().as_str() {
        "tcp" => "tcp",
        "udp" => "udp",
        _ => "any",
    };
    let port = port.trim().trim_end_matches("/tcp").trim_end_matches("/udp").replace('-', ":");
    let port = if port.is_empty() || port.eq_ignore_ascii_case("any") { "any".to_string() } else { port };
    if port != "any" && !port.chars().all(|c| c.is_ascii_digit() || c == ',' || c == ':') {
        return Err(vps_err(ERR_COMMAND_FAILED, format!("invalid port {port}")));
    }
    let source = source.trim();
    let source = if is_anywhere_source(source) { "any".to_string() } else { source.to_string() };
    if source != "any" && !source.chars().all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':' || c == '/') {
        return Err(vps_err(ERR_COMMAND_FAILED, format!("invalid source {source}")));
    }
    Ok(UfwRuleArgs { action, source, port, proto })
}

// Puts a rule at the top. `ufw prepend` handles IPv4/IPv6 positions but fails on an empty list,
// where a plain add is already on top. `ufw show added` also lists rules while ufw is off.
fn ufw_on_top(rule: &str) -> String {
    format!("if ufw show added 2>/dev/null | grep -q '^ufw '; then ufw prepend {rule}; else ufw {rule}; fi\n")
}

// deny/reject go to the top; allow appends.
pub fn build_ufw_add_rule_script(action: &str, source: &str, port: &str, proto: &str) -> Result<String, String> {
    let a = validate_rule_args(action, source, port, proto)?;
    let mut target = format!("from {} to any", a.source);
    if a.port != "any" {
        target.push_str(&format!(" port {}", a.port));
    }
    // ufw needs an explicit proto for port lists and ranges.
    let protos: Vec<&str> = match a.proto {
        "any" if a.port.contains(',') || a.port.contains(':') => vec!["tcp", "udp"],
        p => vec![p],
    };
    let on_top = matches!(a.action, "deny" | "reject");
    Ok(protos
        .iter()
        .map(|p| {
            let proto = if *p == "any" { String::new() } else { format!(" proto {p}") };
            let rule = format!("{} {target}{proto}", a.action);
            if on_top { ufw_on_top(&rule) } else { format!("ufw {rule}\n") }
        })
        .collect())
}

fn normalize_spaces(s: &str) -> String {
    s.split_whitespace().collect::<Vec<_>>().join(" ")
}

// Looks the rule up by its text in the current numbered list, so shifted numbers cannot hit another rule.
pub fn build_ufw_delete_rule_script(rule: &FirewallRuleItem) -> String {
    format!(
        r#"want={want}
n=$(ufw status numbered | awk -v want="$want" '/^\[ *[0-9]+\]/ {{
  num = $0; sub(/^\[ */, "", num); sub(/\].*/, "", num)
  rest = $0; sub(/^\[ *[0-9]+\] */, "", rest); gsub(/[ \t]+/, " ", rest); sub(/ $/, "", rest)
  if (rest == want) {{ print num; exit }}
}}')
if [ -z "$n" ]; then echo SSHBOX_RULE_NOT_FOUND; exit 3; fi
ufw --force delete "$n"
"#,
        want = shell_single_quote(&normalize_spaces(&rule.raw_spec))
    )
}

pub fn build_set_firewall_state_script(
    engine: FirewallEngine,
    enable: bool,
    ensure_ssh_port: Option<u16>,
) -> Result<String, String> {
    let mut script = String::new();
    match engine {
        FirewallEngine::Ufw => {
            if let (true, Some(p)) = (enable, ensure_ssh_port) {
                // Above any saved deny the guard could not see while ufw was off.
                script.push_str(&ufw_on_top(&format!("allow {p}/tcp")));
            }
            script.push_str(if enable { "ufw --force enable\n" } else { "ufw disable\n" });
        }
        FirewallEngine::Firewalld => {
            if let (true, Some(p)) = (enable, ensure_ssh_port) {
                script.push_str(&format!("firewall-offline-cmd --add-port={p}/tcp\n"));
            }
            script.push_str(if enable {
                "systemctl enable --now firewalld\n"
            } else {
                "systemctl disable --now firewalld\n"
            });
        }
        FirewallEngine::Iptables | FirewallEngine::Windows | FirewallEngine::None => {
            return Err(vps_err(ERR_UNSUPPORTED, "firewall_read_only"))
        }
    }
    Ok(script)
}

// Runs a write and returns the fresh snapshot from the same script run (spec §5).
async fn run_firewall_write(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    engine: FirewallEngine,
    write: &str,
) -> Result<FirewallSnapshot, String> {
    let script = format!(
        "set -eu\n{write}set +eu\necho SSHBOX_SNAPSHOT_BEGIN\n{}",
        firewall_read_script(engine)
    );
    let raw = run_privileged_script(host_id, pool, sudo_password, &script, None).await?;
    Ok(snapshot_after(&raw, engine))
}

fn writable_engine(engine: FirewallEngine) -> Result<(), String> {
    match engine {
        FirewallEngine::Ufw | FirewallEngine::Firewalld | FirewallEngine::Windows => Ok(()),
        _ => Err(vps_err(ERR_UNSUPPORTED, "firewall_read_only")),
    }
}

pub async fn add_firewall_rule(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    action: String,
    source: String,
    port: String,
    proto: String,
) -> Result<FirewallSnapshot, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    writable_engine(probe.firewall_engine)?;
    crate::vps_guard::check_firewall_add(&probe.context, &action, &source, &port)?;
    if probe.firewall_engine == FirewallEngine::Windows {
        let script = crate::vps_windows::build_windows_add_rule_script(&action, &source, &port, &proto)?;
        return crate::vps_windows::run_windows_firewall_write(host_id, pool, &script).await;
    }
    let script = match probe.firewall_engine {
        FirewallEngine::Firewalld => build_firewalld_add_rule_script(&action, &source, &port, &proto)?,
        _ => build_ufw_add_rule_script(&action, &source, &port, &proto)?,
    };
    run_firewall_write(host_id, pool, sudo_password, probe.firewall_engine, &script).await
}

pub async fn delete_firewall_rule(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    rule: FirewallRuleItem,
) -> Result<FirewallSnapshot, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    writable_engine(probe.firewall_engine)?;
    let current = get_firewall_snapshot(host_id, pool, sudo_password.clone()).await?;
    if probe.firewall_engine == FirewallEngine::Windows {
        // Trust only the fresh rows (never the client's flags): find the rule by name, then guard and delete it.
        let fresh: Vec<&FirewallRuleItem> = current.rules.iter().filter(|r| r.raw_spec == rule.raw_spec).collect();
        let Some(target) = fresh.first() else {
            return Err(vps_err(ERR_COMMAND_FAILED, "rule not found"));
        };
        if fresh.iter().any(|r| r.comment != crate::vps_windows::WINDOWS_RULE_GROUP) {
            return Err(vps_err(ERR_UNSUPPORTED, "windows_rule_not_managed"));
        }
        crate::vps_guard::check_firewall_delete_windows(&probe.context, &current.rules, target)?;
        let script = crate::vps_windows::build_windows_delete_rule_script(target)?;
        return crate::vps_windows::run_windows_firewall_write(host_id, pool, &script).await;
    }
    crate::vps_guard::check_firewall_delete(&probe.context, &current.rules, &rule)?;
    let script = match probe.firewall_engine {
        FirewallEngine::Firewalld => build_firewalld_delete_rule_script(&rule),
        _ => build_ufw_delete_rule_script(&rule),
    };
    run_firewall_write(host_id, pool, sudo_password, probe.firewall_engine, &script).await
}

pub async fn set_firewall_state(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    enable: bool,
) -> Result<FirewallSnapshot, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    writable_engine(probe.firewall_engine)?;
    let current = get_firewall_snapshot(host_id, pool, sudo_password.clone()).await?;
    if probe.firewall_engine == FirewallEngine::Windows {
        let ensure = crate::vps_windows::windows_ensure_port(&probe.context, &current.rules);
        let script = crate::vps_windows::build_windows_firewall_state_script(enable, ensure);
        return crate::vps_windows::run_windows_firewall_write(host_id, pool, &script).await;
    }
    let ensure = crate::vps_guard::ssh_allow_needed(&probe.context, &current.rules).then_some(probe.context.server_port);
    let script = build_set_firewall_state_script(probe.firewall_engine, enable, ensure)?;
    run_firewall_write(host_id, pool, sudo_password, probe.firewall_engine, &script).await
}

fn firewalld_attr<'a>(rule: &'a str, name: &str) -> Option<&'a str> {
    let key = format!("{name}=\"");
    let start = rule.find(&key)? + key.len();
    rule[start..].split('"').next()
}

// Parses `firewall-cmd --list-all`. Order follows firewalld's chains: negative-priority rich
// rules, then rich drop/reject, then accepts (services, ports, rich accept), then positive priority.
pub fn parse_firewalld_list_all(raw: &str) -> Vec<FirewallRuleItem> {
    let known_service = |name: &str| match name {
        "ssh" => Some("22/tcp"),
        "http" => Some("80/tcp"),
        "https" => Some("443/tcp"),
        _ => None,
    };
    type Keyed = Vec<((u8, i32), FirewallRuleItem)>;
    let mut keyed: Keyed = Vec::new();
    // Services and ports have no family, so they apply to IPv4 and IPv6.
    fn push_both(keyed: &mut Keyed, to: String, raw_spec: String) {
        for v6 in [false, true] {
            let protocol = to.rsplit_once('/').map(|(_, p)| p.to_string()).unwrap_or_else(|| "any".to_string());
            keyed.push(((2, 0), FirewallRuleItem {
                index: 0, to_port: to.clone(), protocol, action: "ALLOW".to_string(),
                from_source: "Anywhere".to_string(), v6, comment: String::new(), raw_spec: raw_spec.clone(), effective: true,
            }));
        }
    }
    let mut in_rich = false;
    for line in raw.lines() {
        let t = line.trim();
        if let Some(v) = t.strip_prefix("services:") {
            for svc in v.split_whitespace() {
                push_both(&mut keyed, known_service(svc).unwrap_or(svc).to_string(), format!("service:{svc}"));
            }
        } else if let Some(v) = t.strip_prefix("ports:") {
            for port in v.split_whitespace() {
                push_both(&mut keyed, port.replace('-', ":"), format!("port:{port}"));
            }
        } else if t.starts_with("rich rules:") {
            in_rich = true;
        } else if in_rich && t.starts_with("rule ") {
            let priority: i32 = firewalld_attr(t, "priority").and_then(|p| p.parse().ok()).unwrap_or(0);
            let deny = t.ends_with(" drop") || t.ends_with(" reject") || t.contains(" reject type=");
            let to_port = match (firewalld_attr(t, "port"), firewalld_attr(t, "protocol")) {
                (Some(p), Some(proto)) => format!("{}/{proto}", p.replace('-', ":")),
                (Some(p), None) => p.replace('-', ":"),
                _ => "Anywhere".to_string(),
            };
            let protocol = firewalld_attr(t, "protocol").unwrap_or("any").to_string();
            let source = firewalld_attr(t, "address").unwrap_or("Anywhere").to_string();
            let families: Vec<bool> = match firewalld_attr(t, "family") {
                Some("ipv6") => vec![true],
                Some("ipv4") => vec![false],
                _ => vec![false, true],
            };
            let key = match (priority, deny) {
                (p, _) if p < 0 => (0, p),
                (p, _) if p > 0 => (3, p),
                (_, true) => (1, 0),
                (_, false) => (2, 0),
            };
            for v6 in families {
                keyed.push((key, FirewallRuleItem {
                    index: 0, to_port: to_port.clone(), protocol: protocol.clone(),
                    action: if deny { "DENY" } else { "ALLOW" }.to_string(),
                    from_source: source.clone(), v6, comment: String::new(), raw_spec: t.to_string(), effective: true,
                }));
            }
        } else if !t.is_empty() && t.contains(':') {
            in_rich = false;
        }
    }
    keyed.sort_by_key(|(k, _)| *k);
    keyed
        .into_iter()
        .enumerate()
        .map(|(i, (_, mut r))| {
            r.index = i as u32 + 1;
            r
        })
        .collect()
}

fn firewalld_rule_body(a: &UfwRuleArgs, port: Option<&str>, proto: Option<&str>) -> String {
    let mut body = String::from("rule");
    if matches!(a.action, "deny" | "reject") {
        body.push_str(" priority=\"-100\"");
    }
    if a.source != "any" {
        let family = if a.source.contains(':') { "ipv6" } else { "ipv4" };
        body.push_str(&format!(" family=\"{family}\" source address=\"{}\"", a.source));
    }
    if let (Some(p), Some(pr)) = (port, proto) {
        body.push_str(&format!(" port port=\"{p}\" protocol=\"{pr}\""));
    }
    body.push_str(match a.action {
        "deny" => " drop",
        "reject" => " reject",
        _ => " accept",
    });
    body
}

pub fn build_firewalld_add_rule_script(action: &str, source: &str, port: &str, proto: &str) -> Result<String, String> {
    let a = validate_rule_args(action, source, port, proto)?;
    let protos: Vec<&str> = if a.proto == "any" { vec!["tcp", "udp"] } else { vec![a.proto] };
    let ports: Vec<String> = a.port.split(',').map(|p| p.replace(':', "-")).collect();
    let mut cmds: Vec<String> = Vec::new();
    if a.port == "any" {
        if a.source == "any" && !matches!(a.action, "deny" | "reject") {
            return Err(vps_err(ERR_COMMAND_FAILED, "allowing every port from anywhere is not supported"));
        }
        cmds.push(format!("--add-rich-rule={}", shell_single_quote(&firewalld_rule_body(&a, None, None))));
    } else {
        for p in &ports {
            for pr in &protos {
                if a.source == "any" && !matches!(a.action, "deny" | "reject") {
                    cmds.push(format!("--add-port={p}/{pr}"));
                } else {
                    cmds.push(format!("--add-rich-rule={}", shell_single_quote(&firewalld_rule_body(&a, Some(p), Some(pr)))));
                }
            }
        }
    }
    let mut script: String = cmds.iter().map(|c| format!("firewall-cmd --permanent {c}\n")).collect();
    script.push_str("firewall-cmd --reload\n");
    Ok(script)
}

// raw_spec is "service:<name>", "port:<p>/<proto>" or the exact rich rule text.
pub fn build_firewalld_delete_rule_script(rule: &FirewallRuleItem) -> String {
    let (kind, value) = if let Some(svc) = rule.raw_spec.strip_prefix("service:") {
        ("service", svc.to_string())
    } else if let Some(port) = rule.raw_spec.strip_prefix("port:") {
        ("port", port.to_string())
    } else {
        ("rich-rule", rule.raw_spec.clone())
    };
    let v = shell_single_quote(&value);
    format!(
        "if ! firewall-cmd --permanent --query-{kind}={v} >/dev/null 2>&1; then echo SSHBOX_RULE_NOT_FOUND; exit 3; fi\n\
firewall-cmd --permanent --remove-{kind}={v}\nfirewall-cmd --reload\n"
    )
}

pub fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

pub const ERR_SUDO_REQUIRED: &str = "SUDO_REQUIRED";
pub const ERR_SUDO_DENIED: &str = "SUDO_DENIED";
pub const ERR_LOCKOUT_BLOCKED: &str = "LOCKOUT_BLOCKED";
pub const ERR_COMMAND_FAILED: &str = "COMMAND_FAILED";
pub const ERR_VERIFY_FAILED: &str = "VERIFY_FAILED";
pub const ERR_UNSUPPORTED: &str = "UNSUPPORTED";

// Errors cross to the UI as "CODE: detail"; the frontend translates CODE.
pub fn vps_err(code: &str, detail: impl std::fmt::Display) -> String {
    format!("{code}: {detail}")
}

// stdin protocol: line 1 is the sudo password, everything after it is the script's secret.
pub fn privileged_stdin(sudo_password: Option<&str>, secret: Option<&str>) -> Result<Vec<u8>, String> {
    let pw = sudo_password.unwrap_or("");
    if pw.contains('\n') || pw.contains('\r') {
        return Err(vps_err(ERR_SUDO_DENIED, "sudo password must not contain a line break"));
    }
    Ok(format!("{pw}\n{}", secret.unwrap_or("")).into_bytes())
}

// Passwords and secrets arrive on stdin, never in this command line (visible in `ps` on the server).
// System paths go AFTER $PATH so tests can stand in fake sudo/id binaries.
pub fn build_privileged_command(script: &str) -> String {
    format!(
        r#"LC_ALL=C
PATH="$PATH:/usr/local/sbin:/usr/sbin:/sbin"
IFS= read -r SSHBOX_PW || SSHBOX_PW=
SSHBOX_SECRET_FILE=$(mktemp)
chmod 600 "$SSHBOX_SECRET_FILE"
cat > "$SSHBOX_SECRET_FILE"
trap 'rm -f "$SSHBOX_SECRET_FILE"' EXIT
sshbox_body() {{
  printf 'SSHBOX_SECRET_FILE=%s\nexport PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH"\n' "$SSHBOX_SECRET_FILE"
  cat << 'SSHBOX_EOF_9912'
{script}
SSHBOX_EOF_9912
}}
if [ "$(id -u)" = "0" ]; then
  sshbox_body | sh -s 2>&1
  rc=$?
elif sudo -n true 2>/dev/null; then
  sshbox_body | sudo -n sh -s 2>&1
  rc=$?
elif [ -n "$SSHBOX_PW" ]; then
  {{ printf '%s\n' "$SSHBOX_PW"; sshbox_body; }} | sudo -S -p '' sh -s 2>&1
  rc=$?
else
  echo "SSHBOX_ERR_SUDO_PASSWORD_REQUIRED"
  rc=126
fi
if [ "$rc" -ne 0 ]; then
  echo "SSHBOX_EXIT_CODE:$rc"
fi
"#
    )
}

pub fn classify_privileged_output(raw: &str) -> Result<String, String> {
    if raw.contains("SSHBOX_ERR_SUDO_PASSWORD_REQUIRED") {
        return Err(vps_err(ERR_SUDO_REQUIRED, "sudo password required"));
    }
    if let Some(idx) = raw.rfind("SSHBOX_EXIT_CODE:") {
        let body = raw[..idx].trim();
        let lower = body.to_lowercase();
        if lower.contains("sorry, try again")
            || lower.contains("incorrect password")
            || lower.contains("a password is required")
        {
            return Err(vps_err(ERR_SUDO_DENIED, "incorrect sudo password"));
        }
        let detail = if body.is_empty() { raw[idx..].trim() } else { body };
        return Err(vps_err(ERR_COMMAND_FAILED, detail));
    }
    Ok(raw.to_string())
}

pub(crate) fn find_remote_host(host_id: &str) -> Result<crate::host_store::HostEntry, String> {
    if host_id == "local" || host_id.trim().is_empty() {
        return Err(vps_err(ERR_UNSUPPORTED, "linux_only"));
    }
    crate::host_store::load_all_hosts()
        .into_iter()
        .find(|h| h.id == host_id)
        .ok_or_else(|| vps_err(ERR_COMMAND_FAILED, format!("unknown host {host_id}")))
}

// Write scripts start with `set -eu` themselves; read-only scripts may tolerate failing probes.
pub async fn run_privileged_script(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    script: &str,
    secret: Option<&str>,
) -> Result<String, String> {
    let host = find_remote_host(host_id)?;
    let stdin = privileged_stdin(sudo_password.as_deref(), secret)?;
    let out = pool
        .exec_bytes_on_host(&host, &build_privileged_command(script), Some(&stdin))
        .await?;
    classify_privileged_output(&String::from_utf8_lossy(&out))
}

// Runs WITHOUT sudo: sudo's env_reset would strip SSH_CONNECTION. Line 1 of stdin is the sudo password.
pub const PROBE_SCRIPT: &str = r#"LC_ALL=C
PATH="$PATH:/usr/local/sbin:/usr/sbin:/sbin"
IFS= read -r pw || pw=
echo "UNAME:$(uname -s 2>/dev/null)"
echo "USER:$(id -un 2>/dev/null)"
uid=$(id -u 2>/dev/null)
echo "UID:$uid"
if [ "$uid" = "0" ]; then s=root
elif sudo -n true 2>/dev/null; then s=nopasswd
elif [ -z "$pw" ]; then s=none
elif printf '%s\n' "$pw" | sudo -S -p '' true >/dev/null 2>&1; then s=password_ok
else s=password_bad
fi
echo "SUDO:$s"
echo "OS_ID:$( (. /etc/os-release && echo "$ID") 2>/dev/null)"
echo "OS_VERSION:$( (. /etc/os-release && echo "$VERSION_ID") 2>/dev/null)"
echo "OPENSSH:$(ssh -V 2>&1 | head -n 1)"
echo "SSH_CONNECTION:${SSH_CONNECTION:-}"
if command -v systemctl >/dev/null 2>&1; then echo "SYSTEMD:1"; else echo "SYSTEMD:0"; fi
echo "SSH_SOCKET:$(systemctl is-active ssh.socket 2>/dev/null)"
echo "SELINUX:$(getenforce 2>/dev/null)"
if systemctl is-active --quiet firewalld 2>/dev/null; then fw=firewalld
elif command -v ufw >/dev/null 2>&1; then fw=ufw
elif command -v firewall-cmd >/dev/null 2>&1; then fw=firewalld
elif command -v iptables >/dev/null 2>&1; then fw=iptables
else fw=none
fi
echo "FW:$fw"
if [ "$(uname -s 2>/dev/null)" = Darwin ]; then
echo "MAC_VERSION:$(sw_vers -productVersion 2>/dev/null)"
echo ---SSHD_MAIN---
cat /etc/ssh/sshd_config 2>/dev/null
fi
"#;

pub fn parse_probe_output(raw: &str, auth: AuthUsed) -> Result<VpsProbe, String> {
    let field = |key: &str| {
        raw.lines()
            .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
            .unwrap_or_default()
    };
    let os_family = match field("UNAME:").as_str() {
        "Linux" => OsFamily::Linux,
        "Darwin" => OsFamily::Macos,
        _ => return Err(vps_err(ERR_UNSUPPORTED, "unsupported_os")),
    };
    let privilege = match field("SUDO:").as_str() {
        "root" => PrivilegeLevel::Root,
        "nopasswd" => PrivilegeLevel::SudoNoPassword,
        "password_ok" => PrivilegeLevel::SudoPassword,
        "password_bad" => return Err(vps_err(ERR_SUDO_DENIED, "incorrect sudo password")),
        _ => PrivilegeLevel::None,
    };
    let (client_ip, server_ip, server_port) = parse_ssh_connection(&field("SSH_CONNECTION:"))?;
    let firewall_engine = match field("FW:").as_str() {
        "ufw" => FirewallEngine::Ufw,
        "firewalld" => FirewallEngine::Firewalld,
        "iptables" => FirewallEngine::Iptables,
        _ => FirewallEngine::None,
    };
    let macos = os_family == OsFamily::Macos;
    let sshd_main = raw.split_once("---SSHD_MAIN---\n").map_or("", |(_, rest)| rest);
    let mut probe = VpsProbe {
        privilege,
        context: ConnectionContext {
            login_user: field("USER:"),
            auth,
            client_ip,
            server_ip,
            server_port,
        },
        os_id: if macos { "macOS".to_string() } else { field("OS_ID:") },
        os_version: if macos { field("MAC_VERSION:") } else { field("OS_VERSION:") },
        openssh_version: field("OPENSSH:"),
        has_systemd: field("SYSTEMD:") == "1",
        ssh_socket_active: field("SSH_SOCKET:") == "active",
        selinux_enforcing: field("SELINUX:").eq_ignore_ascii_case("enforcing"),
        firewall_engine,
        os_family,
        support: VpsSupport::default(),
        caps: OsCaps { sshd_dropin: macos && sshd_write_target(sshd_main) == WriteTarget::DropIn, ..Default::default() },
    };
    probe.support = compute_support(&probe);
    Ok(probe)
}

// $SSH_CONNECTION = "client_ip client_port server_ip server_port" -> (client_ip, server_ip, server_port)
pub(crate) fn parse_ssh_connection(conn: &str) -> Result<(String, String, u16), String> {
    let parts: Vec<&str> = conn.split_whitespace().collect();
    let [client_ip, _, server_ip, port] = parts.as_slice() else {
        return Err(vps_err(ERR_UNSUPPORTED, "no_ssh_connection"));
    };
    let server_port = port
        .parse::<u16>()
        .map_err(|_| vps_err(ERR_UNSUPPORTED, "no_ssh_connection"))?;
    Ok((client_ip.to_string(), server_ip.to_string(), server_port))
}

pub async fn probe(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<VpsProbe, String> {
    let host = find_remote_host(host_id)?;
    pool.get_or_connect(&host, None).await?;
    let uname = pool
        .exec_bytes_on_host(&host, "uname -s", None)
        .await
        .ok()
        .map(|o| String::from_utf8_lossy(&o).trim().to_string());
    if unix_family(uname.as_deref()).is_none() {
        return crate::vps_windows::probe_windows(&host, pool).await;
    }
    let stdin = privileged_stdin(sudo_password.as_deref(), None)?;
    let out = pool.exec_bytes_on_host(&host, PROBE_SCRIPT, Some(&stdin)).await?;
    let auth = pool
        .session_auth(host_id)
        .await
        .ok_or_else(|| vps_err(ERR_COMMAND_FAILED, "no SSH session for this host"))?;
    parse_probe_output(&String::from_utf8_lossy(&out), auth)
}

fn parse_yes_no(val: &str, default: bool) -> bool {
    match val.trim().to_ascii_lowercase().as_str() {
        "yes" | "true" | "1" => true,
        "no" | "false" | "0" => false,
        _ => default,
    }
}

fn parse_time_seconds(val: &str, default: u32) -> u32 {
    let s = val.trim().to_ascii_lowercase();
    if let Ok(n) = s.parse::<u32>() {
        return n;
    }
    if let Some(num) = s.strip_suffix('s') {
        if let Ok(n) = num.trim().parse::<u32>() {
            return n;
        }
    }
    if let Some(num) = s.strip_suffix('m') {
        if let Ok(n) = num.trim().parse::<u32>() {
            return n.saturating_mul(60);
        }
    }
    if let Some(num) = s.strip_suffix('h') {
        if let Ok(n) = num.trim().parse::<u32>() {
            return n.saturating_mul(3600);
        }
    }
    default
}

fn normalize_permit_root_login(val: &str) -> String {
    match val.trim().to_ascii_lowercase().as_str() {
        "yes" => "yes".to_string(),
        "no" => "no".to_string(),
        "forced-commands-only" => "forced-commands-only".to_string(),
        "prohibit-password" | "without-password" => "prohibit-password".to_string(),
        _ => "prohibit-password".to_string(),
    }
}

fn split_sshd_directive(line: &str) -> Option<(&str, &str)> {
    let without_comment = line.split('#').next().unwrap_or("").trim();
    if without_comment.is_empty() {
        return None;
    }
    if let Some((k, v)) = without_comment.split_once(|c: char| c.is_ascii_whitespace() || c == '=')
    {
        let key = k.trim();
        let val = v.trim_start_matches(|c: char| c.is_ascii_whitespace() || c == '=').trim();
        if !key.is_empty() && !val.is_empty() {
            return Some((key, val));
        }
    }
    None
}

pub const SSHD_MAIN_CONFIG: &str = "/etc/ssh/sshd_config";
pub const SSHD_DROPIN: &str = "/etc/ssh/sshd_config.d/00-sshbox.conf";

// Reads `sshd -T` output (lowercase keys); also parses a plain config file's directives.
pub fn parse_sshd_t(raw: &str) -> SshdSettings {
    let mut s = SshdSettings::default();
    let (mut port_seen, mut allow, mut deny) = (false, Vec::new(), Vec::new());
    for line in raw.lines() {
        let Some((key, val)) = split_sshd_directive(line) else {
            continue;
        };
        match key.to_ascii_lowercase().as_str() {
            "port" if !port_seen => {
                if let Ok(p) = val.parse::<u16>() {
                    s.port = p;
                    port_seen = true;
                }
            }
            "permitrootlogin" => s.permit_root_login = normalize_permit_root_login(val),
            "passwordauthentication" => s.password_authentication = parse_yes_no(val, true),
            "kbdinteractiveauthentication" | "challengeresponseauthentication" => {
                s.kbd_interactive_authentication = parse_yes_no(val, true)
            }
            "pubkeyauthentication" => s.pubkey_authentication = parse_yes_no(val, true),
            "allowusers" => allow.extend(val.split_whitespace().map(str::to_string)),
            "denyusers" => deny.extend(val.split_whitespace().map(str::to_string)),
            "maxauthtries" => s.max_auth_tries = val.parse().unwrap_or(s.max_auth_tries),
            "maxsessions" => s.max_sessions = val.parse().unwrap_or(s.max_sessions),
            "logingracetime" => s.login_grace_time = parse_time_seconds(val, s.login_grace_time),
            "clientaliveinterval" => s.client_alive_interval = parse_time_seconds(val, s.client_alive_interval),
            "clientalivecountmax" => s.client_alive_count_max = val.parse().unwrap_or(s.client_alive_count_max),
            _ => {}
        }
    }
    s.allow_users = allow;
    s.deny_users = deny;
    s
}

// DropIn only if `Include …sshd_config.d…` is the first directive: sshd keeps the first value it reads.
pub fn sshd_write_target(main_cfg: &str) -> WriteTarget {
    match main_cfg.lines().find_map(split_sshd_directive) {
        Some((key, val)) if key.eq_ignore_ascii_case("include") && val.contains("sshd_config.d") => {
            WriteTarget::DropIn
        }
        _ => WriteTarget::MainFile,
    }
}

// (lowercase key, full line or None to drop the directive)
fn sshd_directives(s: &SshdSettings) -> Vec<(&'static str, Option<String>)> {
    let yes_no = |b: bool| if b { "yes" } else { "no" };
    let list = |name: &str, users: &[String]| {
        let users: Vec<&str> = users.iter().map(|u| u.trim()).filter(|u| !u.is_empty()).collect();
        (!users.is_empty()).then(|| format!("{name} {}", users.join(" ")))
    };
    vec![
        ("port", Some(format!("Port {}", s.port.max(1)))),
        ("permitrootlogin", Some(format!("PermitRootLogin {}", normalize_permit_root_login(&s.permit_root_login)))),
        ("passwordauthentication", Some(format!("PasswordAuthentication {}", yes_no(s.password_authentication)))),
        ("kbdinteractiveauthentication", Some(format!("KbdInteractiveAuthentication {}", yes_no(s.kbd_interactive_authentication)))),
        ("pubkeyauthentication", Some(format!("PubkeyAuthentication {}", yes_no(s.pubkey_authentication)))),
        ("maxauthtries", Some(format!("MaxAuthTries {}", s.max_auth_tries.max(1)))),
        ("maxsessions", Some(format!("MaxSessions {}", s.max_sessions))),
        ("logingracetime", Some(format!("LoginGraceTime {}", s.login_grace_time))),
        ("clientaliveinterval", Some(format!("ClientAliveInterval {}", s.client_alive_interval))),
        ("clientalivecountmax", Some(format!("ClientAliveCountMax {}", s.client_alive_count_max))),
        ("allowusers", list("AllowUsers", &s.allow_users)),
        ("denyusers", list("DenyUsers", &s.deny_users)),
    ]
}

// The directives whose rendered line differs between two settings (None = remove the line).
pub fn changed_directives(current: &SshdSettings, desired: &SshdSettings) -> Vec<(&'static str, Option<String>)> {
    sshd_directives(desired)
        .into_iter()
        .zip(sshd_directives(current))
        .filter(|((_, want), (_, have))| want != have)
        .map(|(d, _)| d)
        .collect()
}

const DROPIN_HEADER: &str = "# Managed by ssh-box. Changes here are overwritten by Server Config.\n";

// Only the given directives change; whatever the drop-in already holds stays.
pub fn render_dropin(existing: &str, directives: &[(&'static str, Option<String>)]) -> String {
    let base = if existing.trim().is_empty() { DROPIN_HEADER } else { existing };
    render_sshd_config(base, directives)
}

// Replaces the given directives in place, keeps everything else (comments included), and
// appends missing ones before the first Match block.
pub fn render_sshd_config(base_raw: &str, managed: &[(&'static str, Option<String>)]) -> String {
    let mut emitted = std::collections::HashSet::new();
    let mut before: Vec<String> = Vec::new();
    let mut from_match: Vec<String> = Vec::new();
    for line in base_raw.lines() {
        if from_match.is_empty() {
            if let Some((key, _)) = split_sshd_directive(line) {
                let key = key.to_ascii_lowercase();
                if key == "match" {
                    from_match.push(line.to_string());
                    continue;
                }
                if let Some((_, replacement)) = managed.iter().find(|(k, _)| *k == key) {
                    if emitted.insert(key) {
                        before.extend(replacement.clone());
                    }
                    continue;
                }
            }
            before.push(line.to_string());
        } else {
            from_match.push(line.to_string());
        }
    }
    let missing: Vec<String> = managed
        .iter()
        .filter(|(k, _)| !emitted.contains(*k))
        .filter_map(|(_, l)| l.clone())
        .collect();
    if !missing.is_empty() {
        if before.last().is_some_and(|l| !l.trim().is_empty()) {
            before.push(String::new());
        }
        if !base_raw.contains("Managed by ssh-box") {
            before.push("# --- Managed by ssh-box ---".to_string());
        }
        before.extend(missing);
    }
    if !from_match.is_empty() {
        if before.last().is_some_and(|l| !l.trim().is_empty()) {
            before.push(String::new());
        }
        before.extend(from_match);
    }
    let mut out = before.join("\n");
    out.push('\n');
    out
}

pub(crate) fn rollback_pending_cmd(os: OsFamily) -> String {
    match os {
        OsFamily::Macos => format!("launchctl print system/{MACOS_ROLLBACK_LABEL} >/dev/null 2>&1 && echo active || true"),
        OsFamily::Linux => "systemctl is-active sshbox-rollback.timer 2>/dev/null || true".to_string(),
        OsFamily::Windows => unreachable!("Windows reads the pending rollback in its own view script"),
    }
}

fn stop_rollback_timer_cmd(os: OsFamily) -> String {
    match os {
        OsFamily::Macos => format!("launchctl bootout system/{MACOS_ROLLBACK_LABEL} 2>/dev/null || true; rm -f {MACOS_ROLLBACK_PLIST}"),
        OsFamily::Linux => "systemctl stop sshbox-rollback.timer 2>/dev/null || true".to_string(),
        OsFamily::Windows => unreachable!("Windows has its own cleanup scripts"),
    }
}

pub(crate) const SSHD_PART: &str = "---SSHBOX-PART---";

pub fn parse_sshd_view(raw: &str) -> Result<SshdView, String> {
    let parts: Vec<&str> = raw.split(SSHD_PART).collect();
    let [effective, main_cfg, dropin, pending] = parts.as_slice() else {
        return Err(vps_err(ERR_COMMAND_FAILED, raw.trim()));
    };
    Ok(SshdView {
        settings: parse_sshd_t(effective),
        write_target: sshd_write_target(main_cfg),
        rollback_pending: pending.trim() == "active",
        main_raw: main_cfg.trim_start_matches('\n').to_string(),
        dropin_raw: dropin.trim_start_matches('\n').to_string(),
        port_count: effective
            .lines()
            .filter(|l| split_sshd_directive(l).is_some_and(|(k, _)| k.eq_ignore_ascii_case("port")))
            .count(),
    })
}

pub async fn get_sshd_view(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<SshdView, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    let c = &probe.context;
    if probe.os_family == OsFamily::Windows {
        return crate::vps_windows::get_windows_sshd_view(host_id, pool, &probe).await;
    }
    // -C applies Match blocks for this exact connection, so the result is what the app gets.
    let spec = format!(
        "user={},host={},addr={},laddr={},lport={}",
        c.login_user, c.client_ip, c.client_ip, c.server_ip, c.server_port
    );
    let script = format!(
        "sshd -T -C {spec} || exit 1\necho {SSHD_PART}\ncat {SSHD_MAIN_CONFIG}\necho {SSHD_PART}\n\
cat {SSHD_DROPIN} 2>/dev/null || true\necho {SSHD_PART}\n{pending}\n",
        spec = shell_single_quote(&spec),
        pending = rollback_pending_cmd(probe.os_family),
    );
    let raw = run_privileged_script(host_id, pool, sudo_password, &script, None).await?;
    parse_sshd_view(&raw)
}

pub const SSHD_ROLLBACK_SCRIPT: &str = "/etc/ssh/sshbox-rollback.sh";

// What the apply script must do besides writing the file (from the probe and the port change).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReloadPlan {
    pub ssh_socket_active: bool,
    pub selinux_enforcing: bool,
    pub port: u16,
    pub new_port: Option<u16>,
    pub firewall_engine: FirewallEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshdApplyStarted {
    pub rollback_at: i64,
    pub server_now: i64,
    // Some only when the port changed; the UI saves the host port only then.
    pub new_port: Option<u16>,
}

fn sshd_reload_cmd(plan: &ReloadPlan, restart: bool) -> &'static str {
    match (plan.ssh_socket_active, restart) {
        (true, _) => "systemctl daemon-reload\nsystemctl restart ssh.socket",
        (false, false) => "systemctl reload-or-restart ssh 2>/dev/null || systemctl reload-or-restart sshd",
        (false, true) => "systemctl restart ssh 2>/dev/null || systemctl restart sshd",
    }
}

// Opens (or, on rollback, closes) the new SSH port in the active firewall.
fn sshd_firewall_cmd(plan: &ReloadPlan, open: bool) -> String {
    let Some(port) = plan.new_port else {
        return String::new();
    };
    match (plan.firewall_engine, open) {
        (FirewallEngine::Ufw, true) => format!(
            "if ufw status 2>/dev/null | grep -q '^Status: active'; then ufw allow {port}/tcp; fi\n"
        ),
        (FirewallEngine::Ufw, false) => format!("ufw delete allow {port}/tcp 2>/dev/null || true\n"),
        (FirewallEngine::Firewalld, true) => format!(
            "if firewall-cmd --state >/dev/null 2>&1; then firewall-cmd --permanent --add-port={port}/tcp && firewall-cmd --reload; fi\n"
        ),
        (FirewallEngine::Firewalld, false) => format!(
            "firewall-cmd --permanent --remove-port={port}/tcp 2>/dev/null && firewall-cmd --reload 2>/dev/null || true\n"
        ),
        _ => String::new(),
    }
}

pub(crate) fn sshd_restore_fn(file: &str) -> String {
    format!(
        "F={f}\nsshbox_restore() {{\n  if [ -e \"$F.sshbox.bak\" ]; then mv -f \"$F.sshbox.bak\" \"$F\";\n  \
elif [ -e \"$F.sshbox.absent\" ]; then rm -f \"$F\" \"$F.sshbox.absent\"; fi\n}}\n",
        f = shell_single_quote(file)
    )
}

// Run by the systemd timer after 60s, on any failure after arming, or by "Roll back now".
pub fn build_sshd_rollback_script(file: &str, plan: &ReloadPlan) -> String {
    format!(
        "#!/bin/sh\n{restore}sshbox_restore\n{reload}\n{firewall}systemctl stop sshbox-rollback.timer 2>/dev/null || true\nrm -f \"$0\"\n",
        restore = sshd_restore_fn(file),
        reload = sshd_reload_cmd(plan, true),
        firewall = sshd_firewall_cmd(plan, false),
    )
}

// Spec §4.2 steps 3–7. The new file content comes from $SSHBOX_SECRET_FILE.
pub fn build_sshd_apply_script(file: &str, rollback_path: &str, plan: &ReloadPlan) -> String {
    // semanage is slow on RHEL, so it runs before the 60s timer starts.
    let semanage = match plan.new_port {
        Some(port) if plan.selinux_enforcing && port != 22 => format!(
            "if ! {{ semanage port -a -t ssh_port_t -p tcp {port} 2>/dev/null || semanage port -m -t ssh_port_t -p tcp {port}; }}; then \
sshbox_restore; echo SSHBOX_SEMANAGE_FAILED; exit 6; fi\n"
        ),
        _ => String::new(),
    };
    format!(
        r#"set -eu
{restore}R={r}
if systemctl is-active --quiet sshbox-rollback.timer 2>/dev/null; then echo SSHBOX_ROLLBACK_PENDING; exit 3; fi
if [ -e "$F" ]; then cp -p "$F" "$F.sshbox.bak"; rm -f "$F.sshbox.absent"; else : > "$F.sshbox.absent"; rm -f "$F.sshbox.bak"; fi
T=$(mktemp)
cat "$SSHBOX_SECRET_FILE" > "$T"
install -m 644 "$T" "$F"
rm -f "$T"
if ! sshd -t 2>&1; then sshbox_restore; echo SSHBOX_SSHD_INVALID; exit 1; fi
{semanage}cat > "$R" << 'SSHBOX_ROLLBACK_EOF'
{rollback}SSHBOX_ROLLBACK_EOF
chmod 700 "$R"
trap 'rc=$?; [ "$rc" -eq 0 ] || /bin/sh "$R"; exit "$rc"' EXIT
systemd-run --on-active=60 --unit=sshbox-rollback /bin/sh "$R" >/dev/null
echo "NOW:$(date +%s)"
echo "ROLLBACK_AT:$(( $(date +%s) + 60 ))"
{firewall}{reload}
P={port}
ss -tlnpH "sport = :$P" | grep -Eq 'sshd|systemd' || {{ echo SSHBOX_PORT_NOT_LISTENING; exit 4; }}
"#,
        restore = sshd_restore_fn(file),
        r = shell_single_quote(rollback_path),
        rollback = build_sshd_rollback_script(file, plan),
        firewall = sshd_firewall_cmd(plan, true),
        reload = sshd_reload_cmd(plan, false),
        port = plan.port,
    )
}

pub fn apply_preflight(probe: &VpsProbe, view: &SshdView, desired: &SshdSettings) -> Result<(), String> {
    if probe.support.ssh.level != SupportLevel::Full {
        return Err(vps_err(
            ERR_UNSUPPORTED,
            probe.support.ssh.reason.as_deref().unwrap_or("read_only_os"),
        ));
    }
    check_hidden_rows(&probe.support.ssh_rows, &view.settings, desired)?;
    if view.rollback_pending {
        return Err(vps_err(ERR_COMMAND_FAILED, "a previous change is still waiting for verification"));
    }
    match probe.os_family {
        // AllowUsers / DenyUsers / PermitRootLogin are hidden on Windows, so only the auth rules apply.
        OsFamily::Windows => crate::vps_guard::check_sshd_auth(&probe.context, desired),
        OsFamily::Linux | OsFamily::Macos => crate::vps_guard::check_sshd(&probe.context, desired),
    }
}

pub fn parse_apply_started(raw: &str, new_port: Option<u16>) -> Result<SshdApplyStarted, String> {
    let num = |key: &str| {
        raw.lines()
            .find_map(|l| l.trim().strip_prefix(key))
            .and_then(|v| v.trim().parse::<i64>().ok())
    };
    match (num("NOW:"), num("ROLLBACK_AT:")) {
        (Some(server_now), Some(rollback_at)) => Ok(SshdApplyStarted { rollback_at, server_now, new_port }),
        _ => Err(vps_err(ERR_COMMAND_FAILED, raw.trim())),
    }
}

#[derive(Debug)]
pub struct ApplyPlan {
    pub file: &'static str,
    pub content: String,
    pub reload: ReloadPlan,
}

// Directives that accumulate across files instead of "first value wins".
const ACCUMULATING: [&str; 3] = ["port", "allowusers", "denyusers"];

fn main_file_keys(main_raw: &str) -> Vec<String> {
    main_raw
        .lines()
        .filter_map(split_sshd_directive)
        .map(|(k, _)| k.to_ascii_lowercase())
        .take_while(|k| k != "match")
        .collect()
}

// Writes only what changed, so Match-specific values read for the app's own connection are
// never copied into the global config.
pub fn plan_sshd_apply(probe: &VpsProbe, view: &SshdView, desired: &SshdSettings) -> Result<ApplyPlan, String> {
    apply_preflight(probe, view, desired)?;
    let changes = changed_directives(&view.settings, desired);
    if changes.is_empty() {
        return Err(vps_err(ERR_COMMAND_FAILED, "nothing to change"));
    }
    let port_changed = desired.port != view.settings.port;
    if port_changed && view.port_count > 1 {
        return Err(vps_err(ERR_UNSUPPORTED, "multiple_ports"));
    }
    if view.write_target == WriteTarget::DropIn {
        let in_main = main_file_keys(&view.main_raw);
        if changes.iter().any(|(k, _)| ACCUMULATING.contains(k) && in_main.iter().any(|m| m == k)) {
            return Err(vps_err(ERR_UNSUPPORTED, "set_in_main_file"));
        }
    }
    let (file, content) = match view.write_target {
        WriteTarget::DropIn => (SSHD_DROPIN, render_dropin(&view.dropin_raw, &changes)),
        WriteTarget::MainFile => (SSHD_MAIN_CONFIG, render_sshd_config(&view.main_raw, &changes)),
    };
    Ok(ApplyPlan {
        file,
        content,
        reload: ReloadPlan {
            ssh_socket_active: probe.ssh_socket_active,
            selinux_enforcing: probe.selinux_enforcing,
            port: desired.port,
            new_port: port_changed.then_some(desired.port),
            firewall_engine: probe.firewall_engine,
        },
    })
}

pub async fn apply_sshd_settings(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    desired: SshdSettings,
) -> Result<SshdApplyStarted, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    let view = get_sshd_view(host_id, pool, sudo_password.clone()).await?;
    let plan = plan_sshd_apply(&probe, &view, &desired)?;
    let script = match probe.os_family {
        OsFamily::Macos => build_sshd_apply_script_macos(plan.file, SSHD_ROLLBACK_SCRIPT, MACOS_ROLLBACK_PLIST),
        OsFamily::Linux => build_sshd_apply_script(plan.file, SSHD_ROLLBACK_SCRIPT, &plan.reload),
        OsFamily::Windows => crate::vps_windows::build_sshd_apply_script_windows(plan.reload.new_port, plan.reload.port),
    };
    let raw = run_sshd_script(host_id, pool, sudo_password, probe.os_family, &script, Some(&plan.content)).await?;
    parse_apply_started(&raw, plan.reload.new_port)
}

async fn run_sshd_script(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    os: OsFamily,
    script: &str,
    secret: Option<&str>,
) -> Result<String, String> {
    match os {
        OsFamily::Windows => crate::vps_windows::run_windows_script(host_id, pool, script, secret).await,
        OsFamily::Linux | OsFamily::Macos => run_privileged_script(host_id, pool, sudo_password, script, secret).await,
    }
}

// Runs in sh, cmd.exe and PowerShell alike (a Windows login shell has no `true`).
const VERIFY_COMMAND: &str = "exit 0";

// Opens a brand-new SSH connection on `new_port`; the live session proves nothing (spec §4.1).
pub async fn verify_new_connection(
    pool: &SshPool,
    host: &crate::host_store::HostEntry,
    new_port: u16,
    expected_fp: &str,
) -> Result<(), String> {
    let known_hosts = crate::ssh_pool::default_known_hosts_path();
    let before = crate::ssh_pool::list_known_hosts_at(&known_hosts);
    let mut target = host.clone();
    target.port = new_port;
    let reason = match pool.test_exec(&target, VERIFY_COMMAND).await {
        Ok(fp) if fp == expected_fp => return Ok(()),
        Ok(fp) => format!("host key changed on port {new_port} ({fp})"),
        Err(e) => e,
    };
    // test_connection trusts a first-seen key (TOFU); forget anything it just recorded.
    for entry in crate::ssh_pool::list_known_hosts_at(&known_hosts) {
        let existed = before
            .iter()
            .any(|b| b.host_pattern == entry.host_pattern && b.public_key_base64 == entry.public_key_base64);
        if !existed {
            let _ = crate::ssh_pool::delete_known_host_at(&known_hosts, &entry.host_pattern, &entry.key_type);
        }
    }
    Err(vps_err(ERR_VERIFY_FAILED, reason))
}

// Keeps the change. Fails if the rollback already ran (timer fired or "Roll back now").
fn sshd_cleanup_script(rollback: &str, files: [&str; 2], os: OsFamily) -> String {
    if os == OsFamily::Windows {
        return crate::vps_windows::windows_cleanup_script();
    }
    let r = shell_single_quote(rollback);
    let markers: Vec<String> = files
        .iter()
        .flat_map(|f| [format!("{f}.sshbox.bak"), format!("{f}.sshbox.absent")])
        .map(|m| shell_single_quote(&m))
        .collect();
    format!(
        "{}\nif [ ! -e {r} ]; then echo SSHBOX_ALREADY_ROLLED_BACK; exit 5; fi\nrm -f {} {r}\n",
        stop_rollback_timer_cmd(os),
        markers.join(" ")
    )
}

fn sshd_rollback_now_script(os: OsFamily) -> String {
    if os == OsFamily::Windows {
        return crate::vps_windows::windows_rollback_now_script();
    }
    format!(
        "if [ -e {r} ]; then /bin/sh {r}; fi\n{}\n",
        stop_rollback_timer_cmd(os),
        r = SSHD_ROLLBACK_SCRIPT
    )
}

// Step 8–9: verify with a new connection, then keep the change or roll it back now.
pub async fn finish_sshd_apply(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    new_port: Option<u16>,
) -> Result<SshdView, String> {
    let host = find_remote_host(host_id)?;
    let os = probe(host_id, pool, sudo_password.clone()).await?.os_family;
    let expected_fp = pool
        .session_fingerprint(host_id)
        .await
        .ok_or_else(|| vps_err(ERR_COMMAND_FAILED, "no SSH session for this host"))?;
    // Unchanged port: verify where the app already connects (may differ from sshd's port behind NAT).
    if let Err(e) = verify_new_connection(pool, &host, new_port.unwrap_or(host.port), &expected_fp).await {
        run_sshd_script(host_id, pool, sudo_password, os, &sshd_rollback_now_script(os), None).await?;
        return Err(e);
    }
    let cleanup = sshd_cleanup_script(SSHD_ROLLBACK_SCRIPT, [SSHD_DROPIN, SSHD_MAIN_CONFIG], os);
    run_sshd_script(host_id, pool, sudo_password.clone(), os, &cleanup, None)
        .await
        .map_err(|_| vps_err(ERR_VERIFY_FAILED, "the change was rolled back before it could be kept"))?;
    get_sshd_view(host_id, pool, sudo_password).await
}

pub async fn revert_sshd_settings(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<SshdView, String> {
    let os = probe(host_id, pool, sudo_password.clone()).await?.os_family;
    run_sshd_script(host_id, pool, sudo_password.clone(), os, &sshd_rollback_now_script(os), None).await?;
    get_sshd_view(host_id, pool, sudo_password).await
}

pub fn is_valid_linux_username(username: &str) -> bool {
    let mut chars = username.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    if !(first.is_ascii_lowercase() || first == '_') {
        return false;
    }
    let mut count = 1usize;
    for c in chars {
        count += 1;
        if count > 31 {
            return false;
        }
        if !(c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-') {
            return false;
        }
    }
    true
}

fn is_ssh_key_type(tok: &str) -> bool {
    tok.starts_with("ssh-")
        || tok.starts_with("ecdsa-sha2-")
        || tok.starts_with("sk-ssh-")
        || tok.starts_with("sk-ecdsa-")
}

fn format_key_short_preview(body: &str) -> String {
    let clean = body.trim();
    if clean.len() > 18 {
        let prefix = &clean[..10];
        let suffix = &clean[clean.len() - 6..];
        format!("{prefix}...{suffix}")
    } else {
        clean.to_string()
    }
}

pub fn parse_authorized_key_line(line: &str, index: u32) -> Option<RemoteAuthorizedKey> {
    let trimmed = line.trim();
    if trimmed.is_empty() || trimmed.starts_with('#') {
        return None;
    }
    let tokens: Vec<&str> = trimmed.split_whitespace().collect();
    if tokens.len() < 2 {
        return None;
    }
    let k_idx = tokens.iter().position(|t| is_ssh_key_type(t))?;
    let key_type = tokens[k_idx].to_string();
    let body = (*tokens.get(k_idx + 1)?).trim();
    if body.is_empty()
        || !body.chars().all(|c| {
            c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=' || c == '_' || c == '-'
        })
    {
        return None;
    }
    let public_key_body = body.to_string();
    let comment = if k_idx + 2 < tokens.len() {
        tokens[k_idx + 2..].join(" ")
    } else {
        String::new()
    };
    let short_preview = format_key_short_preview(&public_key_body);
    Some(RemoteAuthorizedKey {
        index,
        key_type,
        public_key_body,
        comment,
        short_preview,
        raw_line: trimmed.to_string(),
    })
}

fn default_linux_account() -> LinuxAccountItem {
    LinuxAccountItem {
        username: String::new(),
        uid: u32::MAX,
        gid: 0,
        home_dir: String::new(),
        shell: "/bin/sh".to_string(),
        locked: false,
        groups: Vec::new(),
        has_sudo: false,
        nopasswd_sudo: false,
        authorized_keys: Vec::new(),
        keys_file: String::new(),
    }
}

pub fn parse_access_keys_snapshot(raw: &str) -> AccessKeysSnapshot {
    let mut snap = AccessKeysSnapshot::default();
    let mut sudoers_lines: Vec<String> = Vec::new();
    let mut current_acc: Option<LinuxAccountItem> = None;
    let mut section = "";
    let today: Option<i64> = raw
        .lines()
        .find_map(|l| l.trim().strip_prefix("TODAY:"))
        .and_then(|v| v.trim().parse().ok());

    // macOS regular users start at 501; Linux scripts omit the line and keep 1000.
    let min_uid: u32 = raw
        .lines()
        .find_map(|l| l.trim().strip_prefix("MIN_UID:"))
        .and_then(|v| v.trim().parse().ok())
        .unwrap_or(1000);

    let push_if_eligible = |acc: LinuxAccountItem, out: &mut Vec<LinuxAccountItem>| {
        if !acc.username.is_empty() && (acc.uid == 0 || (acc.uid >= min_uid && acc.uid < 65534)) {
            out.push(acc);
        }
    };

    for line in raw.lines() {
        let trimmed = line.trim();
        match trimmed {
            "---SUDOERS---" => {
                if let Some(acc) = current_acc.take() {
                    push_if_eligible(acc, &mut snap.accounts);
                }
                section = "sudoers";
                continue;
            }
            "---ACCOUNT---" => {
                if let Some(acc) = current_acc.take() {
                    push_if_eligible(acc, &mut snap.accounts);
                }
                current_acc = Some(default_linux_account());
                section = "account";
                continue;
            }
            "---AUTHORIZED_KEYS---" => {
                section = "authorized_keys";
                continue;
            }
            _ => {}
        }

        match section {
            "sudoers" => {
                if !trimmed.is_empty() && !trimmed.starts_with('#') {
                    sudoers_lines.push(trimmed.to_string());
                }
            }
            "account" => {
                let acc = current_acc.get_or_insert_with(default_linux_account);
                if let Some(v) = trimmed.strip_prefix("PASSWD:") {
                    let parts: Vec<&str> = v.split(':').collect();
                    if parts.len() >= 7 {
                        acc.username = parts[0].trim().to_string();
                        acc.uid = parts[2].trim().parse::<u32>().unwrap_or(u32::MAX);
                        acc.gid = parts[3].trim().parse::<u32>().unwrap_or(0);
                        acc.home_dir = parts[5].trim().to_string();
                        acc.shell = parts[6].trim().to_string();
                    } else if parts.len() == 5 {
                        acc.username = parts[0].trim().to_string();
                        acc.uid = parts[1].trim().parse::<u32>().unwrap_or(u32::MAX);
                        acc.gid = parts[2].trim().parse::<u32>().unwrap_or(0);
                        acc.home_dir = parts[3].trim().to_string();
                        acc.shell = parts[4].trim().to_string();
                    }
                } else if let Some(v) = trimmed.strip_prefix("GROUPS:") {
                    for tok in v.split(|c: char| c.is_ascii_whitespace() || c == ',') {
                        let g = tok.trim();
                        if !g.is_empty() && !acc.groups.iter().any(|x| x == g) {
                            acc.groups.push(g.to_string());
                        }
                    }
                } else if let Some(v) = trimmed.strip_prefix("SHADOW:") {
                    // getent shadow: name:pw:lastchg:min:max:warn:inactive:expire:
                    let f: Vec<&str> = v.split(':').collect();
                    // Also accepts just the password field (older snapshots).
                    let pw = if f.len() > 1 { f[1] } else { f[0] }.trim();
                    let expire = f.get(7).and_then(|e| e.trim().parse::<i64>().ok());
                    // "!<hash>" is a locked password; bare "!"/"!!" is useradd's "no password yet",
                    // which still allows key login. Our lock also expires the account.
                    let locked_hash = pw.strip_prefix('!').is_some_and(|h| h.starts_with('$'));
                    acc.locked = locked_hash || matches!((expire, today), (Some(e), Some(t)) if e <= t);
                } else if let Some(v) = trimmed.strip_prefix("LOCKED:") {
                    acc.locked = parse_yes_no(v, acc.locked);
                } else if let Some(v) = trimmed.strip_prefix("HAS_SUDO:") {
                    acc.has_sudo = parse_yes_no(v, acc.has_sudo);
                } else if let Some(v) = trimmed.strip_prefix("NOPASSWD:") {
                    acc.nopasswd_sudo = parse_yes_no(v, acc.nopasswd_sudo);
                }
            }
            "authorized_keys" => {
                if let Some(acc) = current_acc.as_mut() {
                    let next_idx = (acc.authorized_keys.len() as u32) + 1;
                    if let Some(key_item) = parse_authorized_key_line(trimmed, next_idx) {
                        acc.authorized_keys.push(key_item);
                    }
                }
            }
            _ => {}
        }
    }

    if let Some(acc) = current_acc.take() {
        push_if_eligible(acc, &mut snap.accounts);
    }

    for acc in &mut snap.accounts {
        if acc.uid == 0 {
            acc.has_sudo = true;
            acc.nopasswd_sudo = true;
        }
        if acc
            .groups
            .iter()
            .any(|g| g == "sudo" || g == "wheel" || g == "admin")
        {
            acc.has_sudo = true;
        }
        for sline in &sudoers_lines {
            let clean_sline = sline.split('#').next().unwrap_or("").trim();
            let mut s_tokens = clean_sline.split_whitespace();
            let Some(principal) = s_tokens.next() else {
                continue;
            };
            if principal.starts_with("Defaults")
                || principal == "User_Alias"
                || principal == "Runas_Alias"
                || principal == "Host_Alias"
                || principal == "Cmnd_Alias"
            {
                continue;
            }
            let matched = if let Some(grp) = principal.strip_prefix('%') {
                acc.groups.iter().any(|g| g == grp)
            } else {
                principal == acc.username
            };
            if matched {
                acc.has_sudo = true;
                if clean_sline.contains("NOPASSWD") {
                    acc.nopasswd_sudo = true;
                }
            }
        }
        if acc.nopasswd_sudo {
            acc.has_sudo = true;
        }
    }

    snap.accounts.sort_by(|a, b| {
        (a.uid != 0)
            .cmp(&(b.uid != 0))
            .then_with(|| a.uid.cmp(&b.uid))
            .then_with(|| a.username.cmp(&b.username))
    });

    snap
}

const ACCOUNTS_READ_SCRIPT: &str = r#"echo "TODAY:$(( $(date +%s) / 86400 ))"
echo "---SUDOERS---"
grep -vhE '^[[:space:]]*(#|$)' /etc/sudoers /etc/sudoers.d/* 2>/dev/null || true
awk -F: '$3 == 0 || ($3 >= 1000 && $3 < 65534) { print $0 }' /etc/passwd 2>/dev/null | while IFS= read -r pwline; do
  usr=$(printf '%s\n' "$pwline" | cut -d: -f1)
  home=$(printf '%s\n' "$pwline" | cut -d: -f6)
  [ -z "$usr" ] && continue
  echo "---ACCOUNT---"
  echo "PASSWD:$pwline"
  echo "GROUPS:$(id -Gn "$usr" 2>/dev/null | tr '\n' ' ')"
  echo "SHADOW:$(getent shadow "$usr" 2>/dev/null)"
  echo "---AUTHORIZED_KEYS---"
  if [ -n "$home" ] && [ -f "$home/.ssh/authorized_keys" ]; then cat "$home/.ssh/authorized_keys" 2>/dev/null || true; fi
done
"#;

pub async fn get_access_keys_snapshot(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    let os = probe(host_id, pool, sudo_password.clone()).await?.os_family;
    if os == OsFamily::Windows {
        return crate::vps_windows::get_windows_accounts(host_id, pool).await;
    }
    let script = if os == OsFamily::Macos { crate::vps_macos::MACOS_ACCOUNTS_READ_SCRIPT } else { ACCOUNTS_READ_SCRIPT };
    let raw = run_privileged_script(host_id, pool, sudo_password, script, None).await?;
    Ok(parse_access_keys_snapshot(&raw))
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SudoMode {
    None,
    NoPassword,
    WithPassword,
}

fn checked_user(username: &str) -> Result<String, String> {
    let u = username.trim();
    if !is_valid_linux_username(u) || u == "root" {
        return Err(vps_err(ERR_COMMAND_FAILED, format!("invalid username {u}")));
    }
    Ok(u.to_string())
}

pub fn validate_user_password(username: &str, password: Option<&str>, sudo: &SudoMode) -> Result<(), String> {
    checked_user(username)?;
    if *sudo == SudoMode::WithPassword {
        let pw = password.unwrap_or("");
        if pw.is_empty() {
            return Err(vps_err(ERR_COMMAND_FAILED, "a password is required for sudo with password"));
        }
        if pw.contains('\n') || pw.contains('\r') {
            return Err(vps_err(ERR_COMMAND_FAILED, "the password must not contain a line break"));
        }
    }
    Ok(())
}

const SUDO_GROUP: &str = "g=sudo; getent group sudo >/dev/null || g=wheel\n";

fn sudoers_install(u: &str) -> String {
    format!(
        "T=$(mktemp)\nprintf '%s ALL=(ALL) NOPASSWD:ALL\\n' {q} > \"$T\"\nvisudo -cf \"$T\" >/dev/null\n\
install -m 0440 \"$T\" {f}\nrm -f \"$T\"\n",
        q = shell_single_quote(u),
        f = shell_single_quote(&format!("/etc/sudoers.d/90-sshbox-{u}"))
    )
}

// The password, if any, arrives as "user:password" in $SSHBOX_SECRET_FILE.
pub fn build_create_user_script(username: &str, sudo: SudoMode) -> Result<String, String> {
    let u = checked_user(username)?;
    let q = shell_single_quote(&u);
    let mut s = format!(
        "set -eu\nif id -u {q} >/dev/null 2>&1; then echo SSHBOX_USER_EXISTS; exit 1; fi\nuseradd -m -s /bin/bash {q}\n"
    );
    // '*' (not '!') keeps key login working even with UsePAM no.
    s.push_str(&match sudo {
        SudoMode::WithPassword => "chpasswd < \"$SSHBOX_SECRET_FILE\"\n".to_string(),
        _ => format!("usermod -p '*' {q}\n"),
    });
    if sudo != SudoMode::None {
        s.push_str(&format!("{SUDO_GROUP}usermod -aG \"$g\" {q}\n"));
    }
    if sudo == SudoMode::NoPassword {
        s.push_str(&sudoers_install(&u));
    }
    Ok(s)
}

pub fn build_set_user_sudo_script(username: &str, sudo: SudoMode) -> Result<String, String> {
    let u = checked_user(username)?;
    let q = shell_single_quote(&u);
    let file = shell_single_quote(&format!("/etc/sudoers.d/90-sshbox-{u}"));
    Ok(match sudo {
        SudoMode::None => format!(
            "set -eu\nfor g in sudo wheel; do gpasswd -d {q} \"$g\" >/dev/null 2>&1 || true; done\nrm -f {file}\n"
        ),
        SudoMode::NoPassword => format!("set -eu\n{SUDO_GROUP}usermod -aG \"$g\" {q}\n{}", sudoers_install(&u)),
        SudoMode::WithPassword => format!(
            "set -eu\n{SUDO_GROUP}usermod -aG \"$g\" {q}\nrm -f {file}\nchpasswd < \"$SSHBOX_SECRET_FILE\"\n"
        ),
    })
}

// -e 1 expires the account: with UsePAM yes, -L alone does not stop SSH key login.
pub fn build_set_user_lock_script(username: &str, lock: bool) -> Result<String, String> {
    let q = shell_single_quote(&checked_user(username)?);
    Ok(if lock {
        format!("set -eu\nusermod -L -e 1 {q}\n")
    } else {
        format!("set -eu\nusermod -U -e '' {q}\n")
    })
}

fn checked_key_body(body: &str) -> Result<String, String> {
    let b = body.trim();
    if b.is_empty() || !b.chars().all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=') {
        return Err(vps_err(ERR_COMMAND_FAILED, "invalid SSH key"));
    }
    Ok(b.to_string())
}

// Everything inside ~/.ssh runs as the user (runuser), never as root, so symlinks cannot redirect writes.
fn user_ssh_script(username: &str, key_body: &str, awk: &str, inner: &str, stdin_secret: bool) -> Result<String, String> {
    let u = shell_single_quote(&checked_user(username)?);
    let body = shell_single_quote(&checked_key_body(key_body)?);
    let redirect = if stdin_secret { " < \"$SSHBOX_SECRET_FILE\"" } else { "" };
    Ok(format!(
        "set -eu\nhome=$(getent passwd {u} | cut -d: -f6)\nif [ -z \"$home\" ]; then echo SSHBOX_NO_HOME; exit 1; fi\n\
AWK={awk}\nINNER={inner}\nrunuser -u {u} -- sh -c \"$INNER\" sh \"$home\" {body} \"$AWK\"{redirect}\n",
        awk = shell_single_quote(awk),
        inner = shell_single_quote(inner),
    ))
}

// Matches the key body field exactly (the token after the key type), never a substring.
const KEY_MATCH_AWK: &str =
    "/^[ \\t]*#/ { next } { for (i = 1; i < NF; i++) if ($i ~ /^(ssh-|ecdsa-|sk-)/ && $(i + 1) == k) { found = 1 } } END { exit !found }";
const KEY_REMOVE_AWK: &str = "/^[ \\t]*#/ { print; next } { keep = 1; for (i = 1; i < NF; i++) if ($i ~ /^(ssh-|ecdsa-|sk-)/ && $(i + 1) == k) keep = 0; if (keep) print }";

// The key line arrives in $SSHBOX_SECRET_FILE.
pub fn build_authorize_key_script(username: &str, key_body: &str) -> Result<String, String> {
    user_ssh_script(
        username,
        key_body,
        KEY_MATCH_AWK,
        "umask 077; mkdir -p \"$1/.ssh\"; f=\"$1/.ssh/authorized_keys\"; touch \"$f\"; if ! awk -v k=\"$2\" \"$3\" \"$f\"; then \
if [ -s \"$f\" ] && [ -n \"$(tail -c 1 \"$f\")\" ]; then echo >> \"$f\"; fi; cat >> \"$f\"; fi",
        true,
    )
}

pub fn build_revoke_key_script(username: &str, key_body: &str) -> Result<String, String> {
    user_ssh_script(
        username,
        key_body,
        KEY_REMOVE_AWK,
        "umask 077; f=\"$1/.ssh/authorized_keys\"; [ -f \"$f\" ] || exit 0; t=$(mktemp \"$1/.ssh/ak.XXXXXX\"); awk -v k=\"$2\" \"$3\" \"$f\" > \"$t\"; mv \"$t\" \"$f\"",
        false,
    )
}

// Users & Keys can only be edited on Linux; macOS/Windows are read-only.
fn check_access_writable(os: OsFamily) -> Result<(), String> {
    if os == OsFamily::Linux {
        Ok(())
    } else {
        Err(vps_err(ERR_UNSUPPORTED, "read_only_os"))
    }
}

async fn require_linux(host_id: &str, pool: &SshPool, sudo_password: Option<String>) -> Result<VpsProbe, String> {
    let p = probe(host_id, pool, sudo_password).await?;
    check_access_writable(p.os_family)?;
    Ok(p)
}

async fn run_access_write(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    write: &str,
    secret: Option<&str>,
) -> Result<AccessKeysSnapshot, String> {
    let script = format!("{write}set +eu\necho SSHBOX_SNAPSHOT_BEGIN\n{ACCOUNTS_READ_SCRIPT}");
    let raw = run_privileged_script(host_id, pool, sudo_password, &script, secret).await?;
    Ok(parse_access_keys_snapshot(raw.rsplit("SSHBOX_SNAPSHOT_BEGIN").next().unwrap_or(&raw)))
}

pub async fn create_linux_user(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    username: String,
    sudo: SudoMode,
    password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    require_linux(host_id, pool, sudo_password.clone()).await?;
    validate_user_password(&username, password.as_deref(), &sudo)?;
    let script = build_create_user_script(&username, sudo)?;
    let secret = password.filter(|_| sudo == SudoMode::WithPassword).map(|p| format!("{}:{p}\n", username.trim()));
    run_access_write(host_id, pool, sudo_password, &script, secret.as_deref()).await
}

pub async fn set_user_lock(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    username: String,
    lock: bool,
) -> Result<AccessKeysSnapshot, String> {
    let probe = require_linux(host_id, pool, sudo_password.clone()).await?;
    if lock {
        let current = get_access_keys_snapshot(host_id, pool, sudo_password.clone()).await?;
        crate::vps_guard::check_user_change(&probe.context, &current.accounts, &username, crate::vps_guard::UserChange::Lock)?;
    }
    let script = build_set_user_lock_script(&username, lock)?;
    run_access_write(host_id, pool, sudo_password, &script, None).await
}

pub async fn set_user_sudo(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    username: String,
    sudo: SudoMode,
    password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    let probe = require_linux(host_id, pool, sudo_password.clone()).await?;
    validate_user_password(&username, password.as_deref(), &sudo)?;
    if sudo == SudoMode::None {
        let current = get_access_keys_snapshot(host_id, pool, sudo_password.clone()).await?;
        crate::vps_guard::check_user_change(&probe.context, &current.accounts, &username, crate::vps_guard::UserChange::RemoveSudo)?;
    }
    let script = build_set_user_sudo_script(&username, sudo)?;
    let secret = password.filter(|_| sudo == SudoMode::WithPassword).map(|p| format!("{}:{p}\n", username.trim()));
    run_access_write(host_id, pool, sudo_password, &script, secret.as_deref()).await
}

pub async fn authorize_ssh_key(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    username: String,
    key_line: String,
) -> Result<AccessKeysSnapshot, String> {
    require_linux(host_id, pool, sudo_password.clone()).await?;
    let line = key_line
        .lines()
        .map(str::trim)
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .and_then(|l| parse_authorized_key_line(l, 1))
        .ok_or_else(|| vps_err(ERR_COMMAND_FAILED, "invalid SSH public key"))?;
    let script = build_authorize_key_script(&username, &line.public_key_body)?;
    run_access_write(host_id, pool, sudo_password, &script, Some(&format!("{}\n", line.raw_line))).await
}

pub async fn revoke_ssh_key(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    username: String,
    key_body: String,
) -> Result<AccessKeysSnapshot, String> {
    let probe = require_linux(host_id, pool, sudo_password.clone()).await?;
    crate::vps_guard::check_key_revoke(&probe.context, &username, &key_body)?;
    let script = build_revoke_key_script(&username, &key_body)?;
    run_access_write(host_id, pool, sudo_password, &script, None).await
}

#[cfg(test)]
mod tests {
    use super::*;

    // cmd.exe and PowerShell have no `true`: with it every Windows change failed its new-connection check and rolled back.
    #[test]
    fn verify_command_is_a_builtin_of_sh_cmd_and_powershell() {
        assert_eq!(VERIFY_COMMAND, "exit 0");
        let ok = std::process::Command::new("/bin/sh").args(["-c", VERIFY_COMMAND]).env_clear().status().unwrap();
        assert!(ok.success());
    }

    // Runs shell snippets locally against fake `sudo`/`id` binaries placed first in PATH.
    #[cfg(unix)]
    pub(crate) mod shell {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::path::{Path, PathBuf};
        use std::process::{Command, Stdio};
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NEXT: AtomicUsize = AtomicUsize::new(0);

        pub const FAKE_SUDO: &str = r#"case "$1" in -n) exit 1;; esac
IFS= read -r pw
printf '%s' "$pw" > "$FAKE_SUDO_LOG"
[ "$pw" = "$FAKE_SUDO_OK" ] || { echo 'Sorry, try again.'; exit 1; }
exec sh -s"#;

        pub fn temp_dir() -> PathBuf {
            let n = NEXT.fetch_add(1, Ordering::SeqCst);
            let d = std::env::temp_dir().join(format!("sshbox-test-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&d);
            std::fs::create_dir_all(&d).unwrap();
            d
        }

        pub fn fake_bin(dir: &Path, name: &str, body: &str) {
            std::fs::create_dir_all(dir).unwrap();
            let p = dir.join(name);
            std::fs::write(&p, format!("#!/bin/sh\n{body}\n")).unwrap();
            std::fs::set_permissions(&p, std::fs::Permissions::from_mode(0o755)).unwrap();
        }

        // Non-root `id` plus a sudo that logs the password it read and checks it against FAKE_SUDO_OK.
        pub fn fake_sudo_env(dir: &Path) -> PathBuf {
            let bin = dir.join("bin");
            fake_bin(&bin, "id", "echo 1000");
            fake_bin(&bin, "sudo", FAKE_SUDO);
            bin
        }

        pub fn run_local(cmd: &str, stdin: &[u8], bin: &Path, envs: &[(&str, &str)], cwd: &Path) -> String {
            let mut child = Command::new("/bin/sh")
                .arg("-c")
                .arg(cmd)
                .env("PATH", format!("{}:/usr/bin:/bin", bin.display()))
                .envs(envs.iter().copied())
                .current_dir(cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            child.stdin.take().unwrap().write_all(stdin).unwrap();
            let out = child.wait_with_output().unwrap();
            format!(
                "{}{}",
                String::from_utf8_lossy(&out.stdout),
                String::from_utf8_lossy(&out.stderr)
            )
        }
    }

    #[cfg(unix)]
    fn run_wrapped(script: &str, pw: Option<&str>, secret: Option<&str>, sudo_ok: &str) -> (String, std::path::PathBuf) {
        let dir = shell::temp_dir();
        let bin = shell::fake_sudo_env(&dir);
        let log = dir.join("sudo.log");
        let tmp = dir.join("tmp");
        std::fs::create_dir_all(&tmp).unwrap();
        let stdin = privileged_stdin(pw, secret).unwrap();
        let out = shell::run_local(
            &build_privileged_command(script),
            &stdin,
            &bin,
            &[
                ("FAKE_SUDO_LOG", log.to_str().unwrap()),
                ("FAKE_SUDO_OK", sudo_ok),
                ("TMPDIR", tmp.to_str().unwrap()),
            ],
            &dir,
        );
        (out, dir)
    }

    // The old probe nested the password inside sh -c '…': a quote or ; broke the command or ran another one.
    #[cfg(unix)]
    #[test]
    fn sudo_password_reaches_sudo_intact_and_runs_nothing_else() {
        for pw in [
            "p@ss word",
            "a'b",
            "a\"b",
            "abc$HOME",
            "a;touch INJECTED",
            "a`touch INJECTED`b",
            "$(touch INJECTED)",
            "a\\b",
        ] {
            assert!(!build_privileged_command("echo SCRIPT_RAN").contains(pw));
            let (out, dir) = run_wrapped("echo SCRIPT_RAN", Some(pw), None, pw);
            assert_eq!(std::fs::read_to_string(dir.join("sudo.log")).unwrap(), pw, "{out}");
            assert!(!dir.join("INJECTED").exists(), "password {pw:?} ran a command");
            assert!(classify_privileged_output(&out).unwrap().contains("SCRIPT_RAN"), "{out}");
        }
    }

    // New-user passwords and key lines must reach the script without being part of the command line.
    #[cfg(unix)]
    #[test]
    fn secret_reaches_script_and_temp_file_is_removed() {
        let secret = "u:p'a$s;`x`\nsecond line";
        let (out, dir) = run_wrapped("cat \"$SSHBOX_SECRET_FILE\"", Some("pw"), Some(secret), "pw");
        assert!(out.contains(secret), "{out}");
        assert_eq!(std::fs::read_dir(dir.join("tmp")).unwrap().count(), 0);
    }

    // The old wrapper ended every script with `:`, so a failed command still reported success.
    #[cfg(unix)]
    #[test]
    fn failing_command_mid_script_returns_command_failed() {
        let (out, _) = run_wrapped("set -eu\nfalse\necho AFTER", Some("pw"), None, "pw");
        let err = classify_privileged_output(&out).unwrap_err();
        assert!(err.starts_with("COMMAND_FAILED"), "{err}");
        assert!(!out.contains("AFTER"), "{out}");

        let (out, _) = run_wrapped("true\nfalse", Some("pw"), None, "pw");
        assert!(classify_privileged_output(&out).unwrap_err().starts_with("COMMAND_FAILED"), "{out}");
    }

    // A stopped firewalld must still be found, or the app can never turn it back on.
    #[cfg(unix)]
    #[test]
    fn probe_finds_installed_but_stopped_firewalld() {
        let dir = shell::temp_dir();
        let bin = shell::fake_sudo_env(&dir);
        shell::fake_bin(&bin, "firewall-cmd", "exit 252");
        let out = shell::run_local(
            PROBE_SCRIPT,
            b"\n",
            &bin,
            &[("SSH_CONNECTION", "203.0.113.7 51000 198.51.100.2 22")],
            &dir,
        );
        assert!(out.contains("FW:firewalld"), "{out}");
    }

    // The UI asks for a password on SUDO_REQUIRED and shows an error on SUDO_DENIED.
    #[cfg(unix)]
    #[test]
    fn missing_and_wrong_sudo_password_map_to_codes() {
        let (out, _) = run_wrapped("echo X", None, None, "right");
        assert!(classify_privileged_output(&out).unwrap_err().starts_with("SUDO_REQUIRED"), "{out}");
        let (out, _) = run_wrapped("echo X", Some("wrong"), None, "right");
        assert!(classify_privileged_output(&out).unwrap_err().starts_with("SUDO_DENIED"), "{out}");
    }

    // Line 1 of stdin is the sudo password; a newline inside it would feed the rest to the script.
    #[test]
    fn newline_in_password_is_rejected_before_sending() {
        assert!(privileged_stdin(Some("a\nb"), None).unwrap_err().starts_with("SUDO_DENIED"));
        assert!(privileged_stdin(Some("a\rb"), None).unwrap_err().starts_with("SUDO_DENIED"));
        assert_eq!(privileged_stdin(Some("pw"), Some("s")).unwrap(), b"pw\ns".to_vec());
    }

    fn sample_settings() -> SshdSettings {
        SshdSettings {
            port: 32022,
            permit_root_login: "no".to_string(),
            password_authentication: false,
            pubkey_authentication: true,
            allow_users: vec!["ubuntu".to_string(), "ops@192.168.1.10".to_string()],
            deny_users: vec!["guest".to_string()],
            max_auth_tries: 3,
            max_sessions: 4,
            login_grace_time: 60,
            client_alive_interval: 300,
            client_alive_count_max: 2,
            kbd_interactive_authentication: true,
        }
    }

    // sshd -T prints the alias without-password and one allowusers line per user.
    #[test]
    fn parses_sshd_t_alias_and_repeated_allowusers() {
        let raw = "port 2222\nport 2223\npermitrootlogin without-password\npasswordauthentication no\n\
pubkeyauthentication yes\nallowusers alice\nallowusers bob@10.0.0.0/8\ndenyusers guest\nmaxauthtries 3\n\
maxsessions 4\nlogingracetime 60\nclientaliveinterval 300\nclientalivecountmax 2\nusepam yes\n";
        let s = parse_sshd_t(raw);
        assert_eq!(s.port, 2222);
        assert_eq!(s.permit_root_login, "prohibit-password");
        assert!(!s.password_authentication && s.pubkey_authentication);
        assert_eq!(s.allow_users, vec!["alice".to_string(), "bob@10.0.0.0/8".to_string()]);
        assert_eq!(s.deny_users, vec!["guest".to_string()]);
        assert_eq!((s.max_auth_tries, s.max_sessions, s.login_grace_time), (3, 4, 60));
        assert_eq!((s.client_alive_interval, s.client_alive_count_max), (300, 2));
    }

    // Missing directives fall back to OpenSSH defaults, not zero.
    #[test]
    fn parse_sshd_t_uses_openssh_defaults() {
        let s = parse_sshd_t("port 22\n");
        assert_eq!(s.permit_root_login, "prohibit-password");
        assert!(s.password_authentication && s.pubkey_authentication);
        assert_eq!((s.max_auth_tries, s.max_sessions, s.login_grace_time), (6, 10, 120));
    }

    // 00-sshbox.conf only wins when sshd reads the drop-in dir before any main-file directive.
    #[test]
    fn write_target_dropin_only_when_include_precedes_directives() {
        let first = "# comment\n\nInclude /etc/ssh/sshd_config.d/*.conf\nPort 22\n";
        assert_eq!(sshd_write_target(first), WriteTarget::DropIn);
        let late = "Port 22\nInclude /etc/ssh/sshd_config.d/*.conf\n";
        assert_eq!(sshd_write_target(late), WriteTarget::MainFile);
        assert_eq!(sshd_write_target("Port 22\nUsePAM yes\n"), WriteTarget::MainFile);
    }

    #[test]
    fn render_dropin_has_every_managed_directive() {
        let out = render_dropin("", &sshd_directives(&sample_settings()));
        assert!(out.starts_with("# Managed by ssh-box"));
        assert_eq!(parse_sshd_t(&out), sample_settings());
        for key in ["Port ", "PermitRootLogin ", "PasswordAuthentication ", "PubkeyAuthentication ", "MaxAuthTries ",
            "MaxSessions ", "LoginGraceTime ", "ClientAliveInterval ", "ClientAliveCountMax ", "AllowUsers ", "DenyUsers "] {
            assert_eq!(out.lines().filter(|l| l.starts_with(key)).count(), 1, "{key}");
        }
    }

    // The old renderer rewrote forced-commands-only to no.
    #[test]
    fn renders_sshd_config_preserving_comments_and_before_match_blocks() {
        let base_raw = "# Ubuntu default sshd_config\n#Port 22\nPermitRootLogin yes\nPasswordAuthentication yes\n\
UsePAM yes\nSubsystem sftp /usr/lib/openssh/sftp-server\n\nMatch User git\n    AllowTcpForwarding no\n";
        let rendered = render_sshd_config(base_raw, &sshd_directives(&sample_settings()));
        for kept in ["# Ubuntu default sshd_config", "UsePAM yes", "Subsystem sftp /usr/lib/openssh/sftp-server"] {
            assert!(rendered.contains(kept), "{kept}");
        }
        assert!(rendered.find("Port 32022").unwrap() < rendered.find("Match User git").unwrap());
        let before_match = &rendered[..rendered.find("Match User git").unwrap()];
        assert_eq!(parse_sshd_t(before_match), sample_settings());
        let forced = SshdSettings { permit_root_login: "forced-commands-only".to_string(), ..sample_settings() };
        assert!(render_sshd_config(base_raw, &sshd_directives(&forced)).contains("PermitRootLogin forced-commands-only"));
    }

    #[test]
    fn parses_sshd_view_parts() {
        let raw = "port 2222\nport 22\n---SSHBOX-PART---\nInclude /etc/ssh/sshd_config.d/*.conf\n---SSHBOX-PART---\nMaxAuthTries 3\n---SSHBOX-PART---\nactive\n";
        let v = parse_sshd_view(raw).unwrap();
        assert_eq!((v.settings.port, v.port_count), (2222, 2));
        assert_eq!(v.write_target, WriteTarget::DropIn);
        assert_eq!(v.dropin_raw.trim(), "MaxAuthTries 3");
        assert!(v.rollback_pending);
        let v = parse_sshd_view("port 22\n---SSHBOX-PART---\nPort 22\n---SSHBOX-PART---\n---SSHBOX-PART---\ninactive\n").unwrap();
        assert!(!v.rollback_pending && v.write_target == WriteTarget::MainFile && v.dropin_raw.is_empty());
        assert!(parse_sshd_view("garbage").unwrap_err().starts_with("COMMAND_FAILED"));
    }

    fn dropin_view() -> SshdView {
        SshdView {
            settings: SshdSettings::default(),
            write_target: WriteTarget::DropIn,
            dropin_raw: "# Managed by ssh-box\nPasswordAuthentication no\n".to_string(),
            main_raw: "Include /etc/ssh/sshd_config.d/*.conf\nUsePAM yes\n".to_string(),
            port_count: 1,
            ..Default::default()
        }
    }

    // Writing all 11 directives (read with this connection's Match context) changed other users' login.
    #[test]
    fn plans_only_changed_directives_and_keeps_existing_dropin() {
        let desired = SshdSettings { max_auth_tries: 3, ..SshdSettings::default() };
        let plan = plan_sshd_apply(&probe_for("deploy", true), &dropin_view(), &desired).unwrap();
        assert_eq!(plan.file, SSHD_DROPIN);
        assert!(plan.content.contains("PasswordAuthentication no") && plan.content.contains("MaxAuthTries 3"), "{}", plan.content);
        assert!(!plan.content.contains("Port ") && !plan.content.contains("PubkeyAuthentication"), "{}", plan.content);
        assert_eq!(plan.reload.new_port, None);
        let moved = SshdSettings { port: 2222, ..desired };
        assert_eq!(plan_sshd_apply(&probe_for("deploy", true), &dropin_view(), &moved).unwrap().reload.new_port, Some(2222));
    }

    // Port/AllowUsers/DenyUsers add up across files: the old port would stay open.
    #[test]
    fn apply_refuses_directive_also_set_in_main_file() {
        let view = SshdView { main_raw: "Include /etc/ssh/sshd_config.d/*.conf\nPort 22\n".to_string(), ..dropin_view() };
        let moved = SshdSettings { port: 2222, ..SshdSettings::default() };
        let err = plan_sshd_apply(&probe_for("deploy", true), &view, &moved).unwrap_err();
        assert_eq!(err, "UNSUPPORTED: set_in_main_file");
        let other = SshdSettings { max_auth_tries: 3, ..SshdSettings::default() };
        assert!(plan_sshd_apply(&probe_for("deploy", true), &view, &other).is_ok());
    }

    #[test]
    fn apply_refuses_port_change_with_multiple_ports() {
        let view = SshdView { port_count: 2, ..dropin_view() };
        let moved = SshdSettings { port: 2222, ..SshdSettings::default() };
        assert_eq!(plan_sshd_apply(&probe_for("deploy", true), &view, &moved).unwrap_err(), "UNSUPPORTED: multiple_ports");
    }

    // If the timer (or "Roll back now") already restored the old config, keeping the change must fail.
    #[cfg(unix)]
    #[test]
    fn cleanup_fails_when_rollback_already_ran() {
        let dir = shell::temp_dir();
        let bin = dir.join("bin");
        shell::fake_bin(&bin, "systemctl", "exit 0");
        let rollback = dir.join("rollback.sh");
        let files = [dir.join("a.conf"), dir.join("b.conf")];
        let script = sshd_cleanup_script(rollback.to_str().unwrap(), [files[0].to_str().unwrap(), files[1].to_str().unwrap()], OsFamily::Linux);
        let out = shell::run_local(&script, b"", &bin, &[], &dir);
        assert!(out.contains("SSHBOX_ALREADY_ROLLED_BACK"), "{out}");
        std::fs::write(&rollback, "").unwrap();
        std::fs::write(dir.join("a.conf.sshbox.bak"), "").unwrap();
        let out = shell::run_local(&script, b"", &bin, &[], &dir);
        assert!(!out.contains("SSHBOX_ALREADY_ROLLED_BACK"), "{out}");
        assert!(!rollback.exists() && !dir.join("a.conf.sshbox.bak").exists());
    }

    // Runs the apply script locally with fake systemctl/sshd/ss/semanage/ufw that log each call in order.
    #[cfg(unix)]
    struct ApplyRun {
        out: String,
        log: String,
        file: std::path::PathBuf,
        rollback: std::path::PathBuf,
        bin: std::path::PathBuf,
        dir: std::path::PathBuf,
    }

    #[cfg(unix)]
    fn plan(socket: bool, selinux: bool, new_port: Option<u16>, engine: FirewallEngine) -> ReloadPlan {
        ReloadPlan { ssh_socket_active: socket, selinux_enforcing: selinux, port: new_port.unwrap_or(2222), new_port, firewall_engine: engine }
    }

    #[cfg(unix)]
    fn run_apply(plan: &ReloadPlan, existing: Option<&str>, envs: &[(&str, &str)]) -> ApplyRun {
        let dir = shell::temp_dir();
        let bin = dir.join("bin");
        let log = dir.join("calls.log");
        let logger = |name: &str, extra: &str| {
            shell::fake_bin(&bin, name, &format!("echo \"{name} $*\" >> \"$FAKE_LOG\"\n{extra}\nexit 0"));
        };
        logger("systemctl", r#"case "$*" in *"is-active --quiet sshbox-rollback.timer"*) exit "${FAKE_PENDING:-1}";; *is-active*) exit 1;; esac"#);
        logger("sshd", r#"[ "$1" = "-t" ] && exit "${FAKE_SSHD_T:-0}""#);
        logger("ss", r#"[ -n "${FAKE_SS_EMPTY:-}" ] || echo 'LISTEN 0 128 0.0.0.0:2222 0.0.0.0:* users:(("sshd",pid=7,fd=3))'"#);
        logger("systemd-run", "");
        logger("semanage", "");
        logger("ufw", r#"[ "$1" = "status" ] && echo 'Status: active'"#);
        logger("firewall-cmd", r#"[ "$1" = "--state" ] && echo running"#);
        let file = dir.join("00-sshbox.conf");
        if let Some(content) = existing {
            std::fs::write(&file, content).unwrap();
        }
        let secret = dir.join("secret");
        std::fs::write(&secret, "Port 2222\n").unwrap();
        let rollback = dir.join("sshbox-rollback.sh");
        let script = build_sshd_apply_script(file.to_str().unwrap(), rollback.to_str().unwrap(), plan);
        let mut all: Vec<(&str, &str)> = vec![
            ("FAKE_LOG", log.to_str().unwrap()),
            ("SSHBOX_SECRET_FILE", secret.to_str().unwrap()),
        ];
        all.extend_from_slice(envs);
        let out = shell::run_local(&script, b"", &bin, &all, &dir);
        let log = std::fs::read_to_string(&log).unwrap_or_default();
        ApplyRun { out, log, file, rollback, bin, dir }
    }

    #[cfg(unix)]
    fn pos(log: &str, needle: &str) -> usize {
        log.find(needle).unwrap_or_else(|| panic!("{needle:?} not called:\n{log}"))
    }

    // The rollback timer must exist before sshd reloads, or a bad reload has nothing to undo it.
    #[cfg(unix)]
    #[test]
    fn apply_script_arms_rollback_before_reload() {
        let r = run_apply(&plan(false, false, None, FirewallEngine::Ufw), Some("Port 22\n"), &[]);
        assert!(r.out.contains("ROLLBACK_AT:") && r.out.contains("NOW:"), "{}", r.out);
        assert!(pos(&r.log, "systemd-run") < pos(&r.log, "systemctl reload-or-restart"), "{}", r.log);
        assert!(pos(&r.log, "systemctl reload-or-restart") < pos(&r.log, "ss "), "{}", r.log);
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 2222\n");
        assert!(r.rollback.exists());
    }

    #[cfg(unix)]
    #[test]
    fn apply_script_handles_socket_selinux_and_firewall() {
        let r = run_apply(&plan(true, true, Some(2222), FirewallEngine::Ufw), Some("Port 22\n"), &[]);
        assert!(pos(&r.log, "semanage port") < pos(&r.log, "systemctl restart ssh.socket"), "{}", r.log);
        // semanage is slow on RHEL; it runs before the 60s timer starts counting.
        assert!(pos(&r.log, "semanage port") < pos(&r.log, "systemd-run"), "{}", r.log);
        assert!(pos(&r.log, "ufw allow 2222/tcp") < pos(&r.log, "systemctl restart ssh.socket"), "{}", r.log);
        assert!(!r.log.contains("reload-or-restart"), "{}", r.log);
        let r = run_apply(&plan(false, true, Some(22), FirewallEngine::Firewalld), Some("Port 2222\n"), &[]);
        assert!(!r.log.contains("semanage"), "{}", r.log);
        assert!(pos(&r.log, "firewall-cmd --permanent --add-port=22/tcp") < pos(&r.log, "reload-or-restart"), "{}", r.log);
    }

    // A second apply while one is unverified would overwrite the only good backup.
    #[cfg(unix)]
    #[test]
    fn apply_refuses_while_rollback_pending() {
        let r = run_apply(&plan(false, false, None, FirewallEngine::None), Some("Port 22\n"), &[("FAKE_PENDING", "0")]);
        assert!(r.out.contains("SSHBOX_ROLLBACK_PENDING"), "{}", r.out);
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n");
        assert!(!r.log.contains("systemd-run"));
    }

    #[cfg(unix)]
    #[test]
    fn invalid_config_restores_previous_file() {
        let r = run_apply(&plan(false, false, None, FirewallEngine::None), Some("Port 22\n"), &[("FAKE_SSHD_T", "1")]);
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n");
        assert!(!r.log.contains("systemd-run") && !r.log.contains("reload"), "{}", r.log);
    }

    // Any failure after the timer is armed rolls back at once instead of waiting 60 seconds.
    #[cfg(unix)]
    #[test]
    fn failure_after_arming_rolls_back_immediately() {
        let r = run_apply(&plan(false, false, None, FirewallEngine::None), Some("Port 22\n"), &[("FAKE_SS_EMPTY", "1")]);
        assert!(r.out.contains("SSHBOX_PORT_NOT_LISTENING"), "{}", r.out);
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n");
        assert!(r.log.contains("systemctl restart ssh"), "{}", r.log);
        assert!(!r.rollback.exists());
    }

    #[cfg(unix)]
    fn run_rollback(r: &ApplyRun) -> String {
        let log = r.dir.join("calls.log");
        shell::run_local(
            &format!("/bin/sh {}", r.rollback.display()),
            b"",
            &r.bin,
            &[("FAKE_LOG", log.to_str().unwrap())],
            &r.dir,
        );
        std::fs::read_to_string(&log).unwrap()
    }

    #[cfg(unix)]
    #[test]
    fn rollback_restores_backup_and_restarts() {
        let r = run_apply(&plan(false, false, Some(2222), FirewallEngine::Ufw), Some("Port 22\n"), &[]);
        let log = run_rollback(&r);
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n");
        assert!(log[r.log.len()..].contains("systemctl restart ssh"), "{log}");
        assert!(log[r.log.len()..].contains("ufw delete allow 2222/tcp"), "{log}");
        assert!(!r.rollback.exists());
    }

    #[cfg(unix)]
    #[test]
    fn rollback_removes_dropin_that_did_not_exist() {
        let r = run_apply(&plan(false, false, None, FirewallEngine::None), None, &[]);
        assert!(r.file.exists());
        run_rollback(&r);
        assert!(!r.file.exists());
        assert!(!r.dir.join("00-sshbox.conf.sshbox.absent").exists());
    }

    // macOS: turning off only PasswordAuthentication still allows password login via keyboard-interactive.
    #[test]
    fn parses_kbd_interactive_from_both_names() {
        assert!(!parse_sshd_t("kbdinteractiveauthentication no\n").kbd_interactive_authentication);
        assert!(!parse_sshd_t("challengeresponseauthentication no\n").kbd_interactive_authentication);
        assert!(parse_sshd_t("port 22\n").kbd_interactive_authentication);
    }

    // Linux must never get an extra KbdInteractiveAuthentication line written behind the user's back.
    #[test]
    fn linux_password_toggle_does_not_touch_kbd() {
        let cur = SshdSettings::default();
        let want = SshdSettings { password_authentication: false, ..cur.clone() };
        let d = changed_directives(&cur, &want);
        assert_eq!(d, vec![("passwordauthentication", Some("PasswordAuthentication no".to_string()))]);
    }

    fn macos_probe(include: bool) -> VpsProbe {
        let raw = format!("{PROBE_MAC}{}Port 22\n", if include { "Include /etc/ssh/sshd_config.d/*\n" } else { "" });
        parse_probe_output(&raw, key_auth()).unwrap()
    }

    // Without a drop-in folder macOS would need main-file edits, which we do not offer.
    #[test]
    fn macos_support_full_with_dropin_readonly_without() {
        let s = macos_probe(true).support;
        assert_eq!(s.ssh.level, SupportLevel::Full);
        assert!(!s.ssh_rows.port && s.password_coupled);
        let s = macos_probe(false).support;
        assert_eq!((s.ssh.level, s.ssh.reason.as_deref()), (SupportLevel::ReadOnly, Some("macos_no_dropin")));
    }

    // The Port row is hidden on macOS, so a port change must be refused, not written.
    #[test]
    fn macos_plan_rejects_port_change() {
        let view = SshdView { write_target: WriteTarget::DropIn, ..Default::default() };
        let moved = SshdSettings { port: 2222, ..SshdSettings::default() };
        let e = plan_sshd_apply(&macos_probe(true), &view, &moved).unwrap_err();
        assert!(e.starts_with("UNSUPPORTED: hidden_setting"), "{e}");
    }

    #[cfg(unix)]
    struct MacRun {
        out: String,
        log: String,
        file: std::path::PathBuf,
        plist: std::path::PathBuf,
        rollback: std::path::PathBuf,
        bin: std::path::PathBuf,
        dir: std::path::PathBuf,
    }

    // Runs the macOS apply script locally with fake sshd/launchctl; `launchctl print` exits FAKE_PENDING (default 1).
    #[cfg(unix)]
    fn run_mac_apply(existing: Option<&str>, envs: &[(&str, &str)]) -> MacRun {
        run_mac_apply_at("rb.plist", existing, envs)
    }

    #[cfg(unix)]
    fn run_mac_apply_at(plist_rel: &str, existing: Option<&str>, envs: &[(&str, &str)]) -> MacRun {
        let dir = shell::temp_dir();
        let bin = dir.join("bin");
        let log = dir.join("calls.log");
        shell::fake_bin(&bin, "sshd", r#"echo "sshd $*" >> "$FAKE_LOG"; [ "$1" = "-t" ] && exit "${FAKE_SSHD_T:-0}"; exit 0"#);
        shell::fake_bin(&bin, "launchctl", r#"[ "$1" = print ] && exit "${FAKE_PENDING:-1}"; echo "launchctl $*" >> "$FAKE_LOG"; [ "$1" = bootstrap ] && exit "${FAKE_BOOTSTRAP:-0}"; exit 0"#);
        let file = dir.join("00-sshbox.conf");
        if let Some(c) = existing {
            std::fs::write(&file, c).unwrap();
        }
        let secret = dir.join("secret");
        std::fs::write(&secret, "PasswordAuthentication no\n").unwrap();
        let (plist, rollback) = (dir.join(plist_rel), dir.join("sshbox-rollback.sh"));
        let script = crate::vps_macos::build_sshd_apply_script_macos(
            file.to_str().unwrap(), rollback.to_str().unwrap(), plist.to_str().unwrap());
        let mut all = vec![("FAKE_LOG", log.to_str().unwrap()), ("SSHBOX_SECRET_FILE", secret.to_str().unwrap())];
        all.extend_from_slice(envs);
        let out = shell::run_local(&script, b"", &bin, &all, &dir);
        let log = std::fs::read_to_string(&log).unwrap_or_default();
        MacRun { out, log, file, plist, rollback, bin, dir }
    }

    // The launchd job is the only thing that can undo a bad change after the SSH session dies.
    #[cfg(unix)]
    #[test]
    fn macos_apply_backs_up_writes_and_arms_launchd() {
        let r = run_mac_apply(Some("Port 22\n"), &[]);
        assert!(r.out.contains("NOW:") && r.out.contains("ROLLBACK_AT:"), "{}", r.out);
        assert!(r.log.contains(&format!("launchctl bootstrap system {}", r.plist.display())), "{}", r.log);
        assert!(std::fs::read_to_string(&r.plist).unwrap().contains("sleep 60"));
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "PasswordAuthentication no\n");
        assert_eq!(std::fs::read_to_string(r.dir.join("00-sshbox.conf.sshbox.bak")).unwrap(), "Port 22\n");
        assert!(!r.log.contains("reload") && !r.log.contains("kill"), "{}", r.log);
    }

    // A config sshd rejects must be undone before any timer exists.
    #[cfg(unix)]
    #[test]
    fn macos_apply_invalid_config_restores_and_never_arms() {
        let r = run_mac_apply(Some("Port 22\n"), &[("FAKE_SSHD_T", "1")]);
        assert!(r.out.contains("SSHBOX_SSHD_INVALID"), "{}", r.out);
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n");
        assert!(!r.log.contains("launchctl"), "{}", r.log);
    }

    // If the timer cannot be armed (plist unwritable or bootstrap fails), the new config must not stay.
    #[cfg(unix)]
    #[test]
    fn macos_apply_restores_when_timer_cannot_be_armed() {
        let cases = [("no-such-dir/rb.plist", "0"), ("rb.plist", "1")];
        for (plist, bootstrap) in cases {
            let r = run_mac_apply_at(plist, Some("Port 22\n"), &[("FAKE_BOOTSTRAP", bootstrap)]);
            assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n", "{plist}: {}", r.out);
            assert!(!r.plist.exists() && !r.rollback.exists(), "{plist}");
            assert!(!r.out.contains("ROLLBACK_AT:"), "{plist}: {}", r.out);
        }
    }

    // Rolling back must restore the file and leave no plist that would re-fire at the next boot.
    #[cfg(unix)]
    #[test]
    fn macos_rollback_restores_and_removes_plist() {
        let r = run_mac_apply(Some("Port 22\n"), &[]);
        let log = r.dir.join("calls.log");
        shell::run_local(&format!("/bin/sh {}", r.rollback.display()), b"", &r.bin, &[("FAKE_LOG", log.to_str().unwrap())], &r.dir);
        let log = std::fs::read_to_string(&log).unwrap();
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n");
        assert!(!r.plist.exists() && !r.rollback.exists());
        assert!(log.trim_end().ends_with("launchctl bootout system/com.sshbox.rollback"), "{log}");
    }

    // bootout kills the process running the script, so anything after it would never run.
    #[test]
    fn macos_rollback_script_ends_with_bootout() {
        let s = crate::vps_macos::build_sshd_rollback_script_macos("/f", "/p.plist");
        assert!(s.trim_end().ends_with("launchctl bootout system/com.sshbox.rollback"), "{s}");
    }

    // A second apply while one is unverified would overwrite the only good backup.
    #[cfg(unix)]
    #[test]
    fn macos_apply_refuses_while_rollback_pending() {
        let r = run_mac_apply(Some("Port 22\n"), &[("FAKE_PENDING", "0")]);
        assert!(r.out.contains("SSHBOX_ROLLBACK_PENDING"), "{}", r.out);
        assert_eq!(std::fs::read_to_string(&r.file).unwrap(), "Port 22\n");
    }

    fn probe_for(ctx_user: &str, has_systemd: bool) -> VpsProbe {
        let mut p = parse_probe_output(PROBE_UBUNTU, key_auth()).unwrap();
        p.context.login_user = ctx_user.to_string();
        p.has_systemd = has_systemd;
        p.support = compute_support(&p);
        p
    }

    #[test]
    fn apply_refuses_change_blocked_by_guard() {
        let view = SshdView { write_target: WriteTarget::DropIn, ..Default::default() };
        let no_pubkey = SshdSettings { pubkey_authentication: false, ..SshdSettings::default() };
        assert!(apply_preflight(&probe_for("deploy", true), &view, &no_pubkey).unwrap_err().starts_with("LOCKOUT_BLOCKED"));
        let fine = SshdSettings { max_auth_tries: 3, ..SshdSettings::default() };
        assert_eq!(apply_preflight(&probe_for("deploy", true), &view, &fine), Ok(()));
        assert!(apply_preflight(&probe_for("deploy", false), &view, &fine).unwrap_err().starts_with("UNSUPPORTED"));
        let pending = SshdView { rollback_pending: true, ..view };
        assert!(apply_preflight(&probe_for("deploy", true), &pending, &fine).unwrap_err().starts_with("COMMAND_FAILED"));
    }

    #[test]
    fn parses_apply_started_times() {
        let s = parse_apply_started("noise\nNOW:1000\nROLLBACK_AT:1060\n", Some(2222)).unwrap();
        assert_eq!((s.server_now, s.rollback_at, s.new_port), (1000, 1060, Some(2222)));
        assert!(parse_apply_started("nothing", None).unwrap_err().starts_with("COMMAND_FAILED"));
    }

    const PROBE_UBUNTU: &str = "UNAME:Linux\nUSER:deploy\nUID:1000\nSUDO:password_ok\nOS_ID:ubuntu\nOS_VERSION:24.04\n\
OPENSSH:OpenSSH_9.6p1 Ubuntu-3ubuntu13, OpenSSL 3.0.13 30 Jan 2024\n\
SSH_CONNECTION:203.0.113.7 51000 198.51.100.2 2222\nSYSTEMD:1\nSSH_SOCKET:active\nSELINUX:\nFW:ufw\n";

    fn key_auth() -> AuthUsed {
        AuthUsed::Key { public_key_body: "AAAAC3NzaC1lZDI1NTE5".to_string() }
    }

    // The lockout guard compares against these exact values; a wrong field locks the user out.
    #[test]
    fn probe_parses_ubuntu_socket_sudo_password() {
        let p = parse_probe_output(PROBE_UBUNTU, key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::SudoPassword);
        assert_eq!(
            p.context,
            ConnectionContext {
                login_user: "deploy".to_string(),
                auth: key_auth(),
                client_ip: "203.0.113.7".to_string(),
                server_ip: "198.51.100.2".to_string(),
                server_port: 2222,
            }
        );
        assert_eq!((p.os_id.as_str(), p.os_version.as_str()), ("ubuntu", "24.04"));
        assert!(p.openssh_version.starts_with("OpenSSH_9.6"));
        assert!(p.has_systemd && p.ssh_socket_active && !p.selinux_enforcing);
        assert_eq!(p.firewall_engine, FirewallEngine::Ufw);
    }

    #[test]
    fn probe_maps_privilege_selinux_and_engine() {
        let with = |from: &str, to: &str| PROBE_UBUNTU.replace(from, to);
        let p = parse_probe_output(&with("SUDO:password_ok", "SUDO:root"), key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::Root);
        let p = parse_probe_output(&with("SUDO:password_ok", "SUDO:nopasswd"), key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::SudoNoPassword);
        let p = parse_probe_output(&with("SUDO:password_ok", "SUDO:none"), key_auth()).unwrap();
        assert_eq!(p.privilege, PrivilegeLevel::None);
        let raw = with("SELINUX:\nFW:ufw", "SELINUX:Enforcing\nFW:firewalld").replace("SSH_SOCKET:active", "SSH_SOCKET:inactive");
        let p = parse_probe_output(&raw, key_auth()).unwrap();
        assert!(p.selinux_enforcing && !p.ssh_socket_active);
        assert_eq!(p.firewall_engine, FirewallEngine::Firewalld);
    }

    #[test]
    fn probe_parses_ipv6_ssh_connection() {
        let raw = PROBE_UBUNTU.replace("203.0.113.7 51000 198.51.100.2 2222", "2001:db8::7 51000 2001:db8::2 22");
        let p = parse_probe_output(&raw, key_auth()).unwrap();
        assert_eq!(p.context.client_ip, "2001:db8::7");
        assert_eq!(p.context.server_ip, "2001:db8::2");
        assert_eq!(p.context.server_port, 22);
    }

    // FreeBSD and friends must be refused: only Linux and macOS have probes.
    #[test]
    fn probe_rejects_unknown_unix() {
        let raw = PROBE_UBUNTU.replace("UNAME:Linux", "UNAME:FreeBSD");
        assert!(parse_probe_output(&raw, key_auth()).unwrap_err().starts_with("UNSUPPORTED: unsupported_os"));
    }

    // macOS users have UID 501+; the parser must keep them, treat the admin group as sudo, and still drop _service accounts.
    #[test]
    fn macos_accounts_parse_admin_as_sudo() {
        let raw = "MIN_UID:501\n---SUDOERS---\n%admin ALL = (ALL) ALL\nroot ALL = (ALL) ALL\n\
---ACCOUNT---\nPASSWD:me:*:501:20::/Users/me:/bin/zsh\nGROUPS:staff admin\nSHADOW:\n---AUTHORIZED_KEYS---\nssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIHkeyone me@mac\n\
---ACCOUNT---\nPASSWD:guest:*:502:20::/Users/guest:/bin/zsh\nGROUPS:staff\nSHADOW:\n---AUTHORIZED_KEYS---\n\
---ACCOUNT---\nPASSWD:_spotlight:*:89:89::/var/empty:/usr/bin/false\nGROUPS:\nSHADOW:\n---AUTHORIZED_KEYS---\n";
        let s = parse_access_keys_snapshot(raw);
        let by = |n: &str| s.accounts.iter().find(|a| a.username == n).unwrap();
        assert!(by("me").has_sudo && !by("guest").has_sudo);
        assert_eq!(by("me").authorized_keys.len(), 1);
        assert!(s.accounts.iter().all(|a| a.username != "_spotlight"));
    }

    // Without MIN_UID (Linux) a UID 501 system-range account must stay hidden.
    #[test]
    fn linux_snapshot_still_hides_uid_501() {
        let raw = "---SUDOERS---\n---ACCOUNT---\nPASSWD:svc:x:501:501::/home/svc:/bin/sh\nGROUPS:svc\nSHADOW:\n---AUTHORIZED_KEYS---\n";
        assert!(parse_access_keys_snapshot(raw).accounts.is_empty());
    }

    // Writes would corrupt a macOS/Windows host; every non-Linux OS must be refused with the code the UI translates.
    #[test]
    fn macos_access_writes_are_refused() {
        for os in [OsFamily::Macos, OsFamily::Windows] {
            assert_eq!(check_access_writable(os).unwrap_err(), "UNSUPPORTED: read_only_os");
        }
        assert!(check_access_writable(OsFamily::Linux).is_ok());
    }

    // The script is what feeds the shared parser; it must emit the agreed sections and skip service accounts.
    #[test]
    fn macos_accounts_script_emits_parser_sections() {
        let s = crate::vps_macos::MACOS_ACCOUNTS_READ_SCRIPT;
        for m in ["MIN_UID:501", "---SUDOERS---", "---ACCOUNT---", "PASSWD:", "GROUPS:", "SHADOW:", "---AUTHORIZED_KEYS---", "dscl", "_*)"] {
            assert!(s.contains(m), "{m}");
        }
    }

    // dscl returns two space-separated homes for root; the PASSWD home must be the single real one so keys are found.
    #[cfg(target_os = "macos")]
    #[test]
    fn macos_script_reports_root_home_exactly() {
        let out = std::process::Command::new("sh").arg("-c").arg(crate::vps_macos::MACOS_ACCOUNTS_READ_SCRIPT).output().unwrap();
        let text = String::from_utf8_lossy(&out.stdout);
        let line = text.lines().find(|l| l.starts_with("PASSWD:root:")).expect("root account line");
        assert_eq!(line.split(':').nth(6), Some("/var/root"), "{line}");
    }

    const PROBE_MAC: &str = "UNAME:Darwin\nUSER:me\nUID:501\nSUDO:none\nSSH_CONNECTION:203.0.113.7 51000 198.51.100.2 22\nSYSTEMD:0\nFW:none\nMAC_VERSION:15.1\n---SSHD_MAIN---\n";

    // The drop-in decision on macOS decides which file we write; wrong means edits that never take effect.
    #[test]
    fn macos_probe_parses_version_and_dropin() {
        let raw = format!("{PROBE_MAC}Include /etc/ssh/sshd_config.d/*\nPort 22\n");
        let p = parse_probe_output(&raw, key_auth()).unwrap();
        assert_eq!(p.os_family, OsFamily::Macos);
        assert_eq!((p.os_id.as_str(), p.os_version.as_str()), ("macOS", "15.1"));
        assert!(p.caps.sshd_dropin);
    }

    #[test]
    fn macos_probe_without_include_has_no_dropin() {
        let p = parse_probe_output(&format!("{PROBE_MAC}Port 22\n"), key_auth()).unwrap();
        assert!(!p.caps.sshd_dropin);
    }

    #[test]
    fn macos_probe_parses_ipv6_ssh_connection() {
        let raw = PROBE_MAC.replace("203.0.113.7 51000 198.51.100.2 22", "2001:db8::7 51000 2001:db8::2 22");
        let p = parse_probe_output(&raw, key_auth()).unwrap();
        assert_eq!((p.context.client_ip.as_str(), p.context.server_ip.as_str(), p.context.server_port), ("2001:db8::7", "2001:db8::2", 22));
    }

    // Any tab the OS cannot fully write must block the apply, with the reason the UI shows.
    #[test]
    fn preflight_blocks_when_ssh_not_full() {
        let view = SshdView { write_target: WriteTarget::DropIn, ..Default::default() };
        let mut p = probe_for("deploy", true);
        p.support.ssh = crate::vps_support::TabSupport { level: SupportLevel::ReadOnly, reason: Some("no_systemd".into()) };
        assert_eq!(apply_preflight(&p, &view, &SshdSettings::default()), Err("UNSUPPORTED: no_systemd".to_string()));
    }

    #[test]
    fn probe_wrong_sudo_password_is_sudo_denied() {
        let raw = PROBE_UBUNTU.replace("SUDO:password_ok", "SUDO:password_bad");
        assert!(parse_probe_output(&raw, key_auth()).unwrap_err().starts_with("SUDO_DENIED"));
    }

    // Without SSH_CONNECTION the guard cannot know the client IP or port, so nothing may be edited.
    #[test]
    fn probe_without_ssh_connection_is_unsupported() {
        let raw = PROBE_UBUNTU.replace("SSH_CONNECTION:203.0.113.7 51000 198.51.100.2 2222", "SSH_CONNECTION:");
        assert!(parse_probe_output(&raw, key_auth()).unwrap_err().starts_with("UNSUPPORTED"));
    }

    // The old probe nested the password in sh -c '…', so "a;touch INJECTED" ran a command.
    #[cfg(unix)]
    #[test]
    fn probe_script_passes_password_via_stdin_only() {
        for pw in ["a;touch INJECTED'\"", "$(touch INJECTED)"] {
            let dir = shell::temp_dir();
            let bin = shell::fake_sudo_env(&dir);
            let log = dir.join("sudo.log");
            let out = shell::run_local(
                PROBE_SCRIPT,
                &privileged_stdin(Some(pw), None).unwrap(),
                &bin,
                &[
                    ("FAKE_SUDO_LOG", log.to_str().unwrap()),
                    ("FAKE_SUDO_OK", pw),
                    ("SSH_CONNECTION", "203.0.113.7 51000 198.51.100.2 22"),
                ],
                &dir,
            );
            assert!(out.contains("SUDO:password_ok"), "{out}");
            assert!(out.contains("SSH_CONNECTION:203.0.113.7 51000 198.51.100.2 22"), "{out}");
            assert_eq!(std::fs::read_to_string(&log).unwrap(), pw);
            assert!(!dir.join("INJECTED").exists(), "password {pw:?} ran a command");
        }
    }

    #[test]
    fn parses_ufw_firewall_snapshot_and_cross_references_socket_exposure() {
        let raw = r#"
---META---
ENGINE:ufw
ENABLED:1
DEFAULT_INCOMING:deny
DEFAULT_OUTGOING:allow
IPV6:1
---UFW---
Status: active

     To                         Action      From
     --                         ------      ----
[ 1] 2222/tcp                   LIMIT IN    Anywhere                   # SSH Anti-Brute
[ 2] 80,443/tcp                 ALLOW IN    Anywhere                   # Web HTTP/HTTPS
[ 3] 5432/tcp                   ALLOW IN    203.0.113.42               # Postgres Admin
[ 4] 6379/tcp                   DENY IN     Anywhere                   # Block Redis
[ 5] 2222/tcp (v6)              LIMIT IN    Anywhere (v6)              # SSH Anti-Brute
---IPTABLES---
---SOCKETS---
tcp   LISTEN 0      128          0.0.0.0:2222        0.0.0.0:*    users:(("sshd",pid=842,fd=3))
tcp   LISTEN 0      511             [::]:443            [::]:*    users:(("nginx",pid=1042,fd=6))
tcp   LISTEN 0      244          0.0.0.0:5432        0.0.0.0:*    users:(("postgres",pid=915,fd=5))
tcp   LISTEN 0      511          0.0.0.0:6379        0.0.0.0:*    users:(("redis-server",pid=880,fd=6))
tcp   LISTEN 0      128          0.0.0.0:3000        0.0.0.0:*    users:(("node",pid=2100,fd=18))
tcp   LISTEN 0      128        127.0.0.1:9000        0.0.0.0:*    users:(("php-fpm",pid=1500,fd=9))
udp   UNCONN 0      0      127.0.0.53%lo:53          0.0.0.0:*    users:(("systemd-resolve",pid=412,fd=13))
"#;

        let snap = parse_firewall_snapshot(raw);
        assert_eq!(snap.engine, "ufw");
        assert!(snap.enabled);
        assert_eq!(snap.default_incoming, "deny");
        assert_eq!(snap.default_outgoing, "allow");
        assert!(snap.ipv6_filtered);
        assert_eq!(snap.rules.len(), 5);

        assert_eq!(snap.rules[0].index, 1);
        assert_eq!(snap.rules[0].to_port, "2222/tcp");
        assert_eq!(snap.rules[0].protocol, "tcp");
        assert_eq!(snap.rules[0].action, "LIMIT");
        assert_eq!(snap.rules[0].from_source, "Anywhere");
        assert!(!snap.rules[0].v6);
        assert_eq!(snap.rules[0].comment, "SSH Anti-Brute");

        assert_eq!(snap.rules[2].index, 3);
        assert_eq!(snap.rules[2].to_port, "5432/tcp");
        assert_eq!(snap.rules[2].action, "ALLOW");
        assert_eq!(snap.rules[2].from_source, "203.0.113.42");
        assert_eq!(snap.rules[2].comment, "Postgres Admin");

        assert_eq!(snap.rules[4].index, 5);
        assert!(snap.rules[4].v6);
        assert_eq!(snap.rules[4].to_port, "2222/tcp");
        assert_eq!(snap.rules[4].from_source, "Anywhere");

        assert_eq!(snap.listening_sockets.len(), 7);

        let ssh_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 2222)
            .unwrap();
        assert!(ssh_sock.is_public);
        assert_eq!(ssh_sock.process_name, "sshd");
        assert_eq!(ssh_sock.pid, Some(842));
        assert_eq!(ssh_sock.exposure, "exposed");

        let https_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 443)
            .unwrap();
        assert!(https_sock.is_public);
        assert_eq!(https_sock.bind_addr, "::");
        assert_eq!(https_sock.process_name, "nginx");
        assert_eq!(https_sock.exposure, "exposed");

        let pg_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 5432)
            .unwrap();
        assert!(pg_sock.is_public);
        assert_eq!(pg_sock.exposure, "restricted");

        let redis_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 6379)
            .unwrap();
        assert!(redis_sock.is_public);
        assert_eq!(redis_sock.exposure, "blocked");

        let node_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 3000)
            .unwrap();
        assert!(node_sock.is_public);
        // Blocked by default_incoming == "deny"
        assert_eq!(node_sock.exposure, "blocked");

        let fpm_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 9000)
            .unwrap();
        assert!(!fpm_sock.is_public);
        assert_eq!(fpm_sock.exposure, "loopback");

        let dns_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 53)
            .unwrap();
        assert!(!dns_sock.is_public);
        assert_eq!(dns_sock.protocol, "udp");
        assert_eq!(dns_sock.exposure, "loopback");
    }

    #[test]
    fn parses_iptables_fallback_and_disabled_firewall_exposure() {
        let raw = r#"
---META---
ENGINE:iptables
ENABLED:0
DEFAULT_INCOMING:allow
DEFAULT_OUTGOING:allow
CLIENT_IP:198.51.100.7
SSH_PORT:22
---UFW---
---IPTABLES---
-P INPUT ACCEPT
-A INPUT -i lo -j ACCEPT
-A INPUT -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
-A INPUT -s 198.51.100.7/32 -p tcp -m tcp --dport 3306 -m comment --comment "MySQL Admin" -j ACCEPT
-A INPUT -p tcp -m tcp --dport 6379 -j DROP
---SOCKETS---
tcp        0      0 0.0.0.0:3306            0.0.0.0:*               LISTEN      1120/mysqld
tcp        0      0 127.0.0.1:6379          0.0.0.0:*               LISTEN      880/redis-server
"#;

        let snap = parse_firewall_snapshot(raw);
        assert_eq!(snap.engine, "iptables");
        assert!(!snap.enabled);
        assert_eq!(snap.rules.len(), 2);
        assert_eq!(snap.rules[0].to_port, "3306/tcp");
        assert_eq!(snap.rules[0].from_source, "198.51.100.7");
        assert_eq!(snap.rules[0].action, "ALLOW");
        assert_eq!(snap.rules[0].comment, "MySQL Admin");
        assert_eq!(snap.rules[1].to_port, "6379/tcp");
        assert_eq!(snap.rules[1].action, "DENY");

        // When firewall is disabled (enabled == false), public sockets are exposed
        let mysql_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 3306)
            .unwrap();
        assert!(mysql_sock.is_public);
        assert_eq!(mysql_sock.process_name, "mysqld");
        assert_eq!(mysql_sock.pid, Some(1120));
        assert_eq!(mysql_sock.exposure, "exposed");

        let redis_sock = snap
            .listening_sockets
            .iter()
            .find(|s| s.port == 6379)
            .unwrap();
        assert!(!redis_sock.is_public);
        assert_eq!(redis_sock.exposure, "loopback");
    }

    // Runs `script` with fake binaries that log "<name> <args>" in call order.
    #[cfg(unix)]
    fn run_with_fakes(script: &str, fakes: &[(&str, &str)]) -> (String, String) {
        let dir = shell::temp_dir();
        let bin = dir.join("bin");
        let log = dir.join("calls.log");
        for (name, extra) in fakes {
            shell::fake_bin(&bin, name, &format!("echo \"{name} $*\" >> \"$FAKE_LOG\"\n{extra}\nexit 0"));
        }
        let out = shell::run_local(script, b"", &bin, &[("FAKE_LOG", log.to_str().unwrap())], &dir);
        (out, std::fs::read_to_string(&log).unwrap_or_default())
    }

    fn fw_rule(index: u32, action: &str, to: &str, from: &str, v6: bool) -> FirewallRuleItem {
        FirewallRuleItem {
            index,
            to_port: to.to_string(),
            protocol: if to.ends_with("/tcp") { "tcp" } else { "any" }.to_string(),
            action: action.to_string(),
            from_source: from.to_string(),
            v6,
            comment: String::new(),
            raw_spec: format!("{to} {action} IN {from}"), effective: true,
        }
    }

    const UFW_SHIFTED: &str = r#"if [ "$1" = "status" ]; then cat <<'UFW'
Status: active

     To                         Action      From
     --                         ------      ----
[ 1] 2222/tcp                   LIMIT IN    Anywhere
[ 2] 5432/tcp                   ALLOW IN    203.0.113.42               # Postgres Admin
[ 3] 6379/tcp                   DENY IN     Anywhere
UFW
fi"#;

    // Rule numbers shift when another rule is added or deleted; deleting by number hits the wrong rule.
    #[cfg(unix)]
    #[test]
    fn ufw_delete_matches_content_after_numbers_shift() {
        let mut target = fw_rule(3, "ALLOW", "5432/tcp", "203.0.113.42", false);
        target.raw_spec = "5432/tcp   ALLOW IN    203.0.113.42     # Postgres Admin".to_string();
        let (out, log) = run_with_fakes(&build_ufw_delete_rule_script(&target), &[("ufw", UFW_SHIFTED)]);
        assert!(log.contains("ufw --force delete 2"), "{out}\n{log}");
        assert!(!log.contains("delete 3"), "{log}");
    }

    #[cfg(unix)]
    #[test]
    fn ufw_delete_missing_rule_deletes_nothing() {
        let gone = fw_rule(9, "ALLOW", "8080/tcp", "Anywhere", false);
        let (out, log) = run_with_fakes(&build_ufw_delete_rule_script(&gone), &[("ufw", UFW_SHIFTED)]);
        assert!(out.contains("SSHBOX_RULE_NOT_FOUND"), "{out}");
        assert!(!log.contains("delete"), "{log}");
    }

    // `ufw show added` lists saved rules even while ufw is off; FAKE_UFW_RULES=1 simulates some.
    const FAKE_UFW_SAVED: &str = r#"if [ "$1 $2" = "show added" ] && [ -n "${FAKE_UFW_RULES:-}" ]; then printf 'Added user rules:\nufw deny from 203.0.113.7\n'; fi"#;

    // ufw evaluates rules top-down; an appended deny below an allow never applies.
    #[cfg(unix)]
    #[test]
    fn deny_rules_are_inserted_first() {
        let script = build_ufw_add_rule_script("deny", "198.51.100.9", "80", "tcp").unwrap();
        let (_, log) = run_with_fakes(&format!("FAKE_UFW_RULES=1; export FAKE_UFW_RULES\n{script}"), &[("ufw", FAKE_UFW_SAVED)]);
        assert!(log.contains("ufw prepend deny from 198.51.100.9 to any port 80 proto tcp"), "{log}");
        // ufw prepend fails on an empty list; there a plain add is already on top.
        let (_, log) = run_with_fakes(&script, &[("ufw", FAKE_UFW_SAVED)]);
        assert!(log.contains("ufw deny from 198.51.100.9 to any port 80 proto tcp"), "{log}");
        let script = build_ufw_add_rule_script("allow", "203.0.113.42", "5432", "tcp").unwrap();
        let (_, log) = run_with_fakes(&script, &[("ufw", "")]);
        assert!(log.contains("ufw allow from 203.0.113.42 to any port 5432 proto tcp"), "{log}");
        assert!(build_ufw_add_rule_script("allow", "1.2.3.4; rm -rf /", "22", "tcp").is_err());
    }

    #[test]
    fn exposure_is_first_match_per_family() {
        let sock = |bind: &str| ListeningSocketItem {
            protocol: "tcp".to_string(),
            bind_addr: bind.to_string(),
            port: 3306,
            is_public: true,
            process_name: "mysqld".to_string(),
            pid: None,
            exposure: String::new(),
        };
        let deny = fw_rule(1, "DENY", "3306/tcp", "Anywhere", false);
        let allow = fw_rule(2, "ALLOW", "3306/tcp", "Anywhere", false);
        let v4 = sock("0.0.0.0");
        assert_eq!(compute_socket_exposure(&v4, true, "deny", true, &[deny.clone(), allow.clone()]), "blocked");
        assert_eq!(compute_socket_exposure(&v4, true, "deny", true, &[allow.clone(), deny.clone()]), "exposed");
        let only_me = fw_rule(1, "ALLOW", "3306/tcp", "203.0.113.42", false);
        assert_eq!(compute_socket_exposure(&v4, true, "deny", true, &[only_me]), "restricted");
        // An IPv4-only allow does not cover an IPv6-only socket; unfiltered IPv6 is exposed.
        let v6 = sock("2001:db8::1");
        assert_eq!(compute_socket_exposure(&v6, true, "deny", true, &[allow.clone()]), "blocked");
        assert_eq!(compute_socket_exposure(&v6, true, "deny", false, &[allow]), "exposed");
    }

    #[test]
    fn iptables_engine_is_read_only() {
        for engine in [FirewallEngine::Iptables, FirewallEngine::None] {
            let err = build_set_firewall_state_script(engine, true, None).unwrap_err();
            assert!(err.starts_with("UNSUPPORTED"), "{err}");
        }
    }

    // While ufw is off the guard cannot see saved rules, so the SSH allow must go above them.
    #[cfg(unix)]
    #[test]
    fn enable_adds_ssh_allow_first_when_missing() {
        let script = build_set_firewall_state_script(FirewallEngine::Ufw, true, Some(2222)).unwrap();
        let (_, log) = run_with_fakes(&format!("FAKE_UFW_RULES=1; export FAKE_UFW_RULES\n{script}"), &[("ufw", FAKE_UFW_SAVED)]);
        let allow = log.find("ufw prepend allow 2222/tcp").expect(&log);
        assert!(allow < log.find("ufw --force enable").expect(&log), "{log}");
        let (_, log) = run_with_fakes(&script, &[("ufw", FAKE_UFW_SAVED)]);
        assert!(log.contains("ufw allow 2222/tcp"), "{log}");
        let script = build_set_firewall_state_script(FirewallEngine::Ufw, true, None).unwrap();
        let (_, log) = run_with_fakes(&script, &[("ufw", "")]);
        assert!(!log.contains("allow") && log.contains("ufw --force enable"), "{log}");
    }

    #[test]
    fn ufw_ipv6_disabled_is_reported() {
        let raw = "---META---\nENGINE:ufw\nENABLED:1\nDEFAULT_INCOMING:deny\nIPV6:0\n---UFW---\n";
        assert!(!parse_firewall_snapshot(raw).ipv6_filtered);
        assert!(parse_firewall_snapshot(&raw.replace("IPV6:0", "IPV6:1")).ipv6_filtered);
    }

    const FIREWALLD_LIST_ALL: &str = "public (active)
  target: default
  services: dhcpv6-client ssh
  ports: 2222/tcp 8000-8010/udp
  rich rules: 
	rule family=\"ipv4\" source address=\"203.0.113.42\" port port=\"5432\" protocol=\"tcp\" accept
	rule priority=\"-100\" family=\"ipv4\" source address=\"198.51.100.0/24\" drop
";

    #[test]
    fn parses_firewalld_ports_services_and_rich_rules() {
        let rules = parse_firewalld_list_all(FIREWALLD_LIST_ALL);
        let v4: Vec<&FirewallRuleItem> = rules.iter().filter(|r| !r.v6).collect();
        // Negative-priority rich rules run first, then denies, then allows (firewalld chain order).
        assert_eq!(v4[0].action, "DENY");
        assert_eq!(v4[0].from_source, "198.51.100.0/24");
        let pg = v4.iter().find(|r| r.to_port == "5432/tcp").unwrap();
        assert_eq!((pg.action.as_str(), pg.from_source.as_str()), ("ALLOW", "203.0.113.42"));
        assert!(pg.raw_spec.starts_with("rule family=\"ipv4\" source address=\"203.0.113.42\""));
        let ssh = v4.iter().find(|r| r.raw_spec == "service:ssh").unwrap();
        assert_eq!((ssh.to_port.as_str(), ssh.from_source.as_str()), ("22/tcp", "Anywhere"));
        assert!(v4.iter().any(|r| r.raw_spec == "port:2222/tcp" && r.to_port == "2222/tcp"));
        assert!(v4.iter().any(|r| r.raw_spec == "port:8000-8010/udp" && r.to_port == "8000:8010/udp"));
        // Services and ports apply to both families; ipv4-only rich rules do not.
        assert!(rules.iter().any(|r| r.v6 && r.raw_spec == "service:ssh"));
        assert!(!rules.iter().any(|r| r.v6 && r.from_source == "203.0.113.42"));
    }

    const FAKE_FIREWALLD: &str = r#"case "$*" in *--query-*) exit "${FAKE_QUERY:-0}";; esac"#;

    // Runtime-only changes vanish on reload/reboot; every change is --permanent then reloaded.
    #[cfg(unix)]
    #[test]
    fn firewalld_rules_are_permanent_then_reloaded() {
        let scripts = [
            build_firewalld_add_rule_script("allow", "any", "8080", "tcp").unwrap(),
            build_firewalld_add_rule_script("allow", "203.0.113.42", "5432", "tcp").unwrap(),
            build_firewalld_add_rule_script("deny", "198.51.100.9", "any", "any").unwrap(),
            build_firewalld_delete_rule_script(&fw_rule(1, "ALLOW", "2222/tcp", "Anywhere", false)),
        ];
        for script in scripts {
            let (out, log) = run_with_fakes(&script, &[("firewall-cmd", FAKE_FIREWALLD)]);
            let calls: Vec<&str> = log.lines().filter(|l| !l.contains("--query-")).collect();
            assert!(!calls.is_empty(), "{out}");
            assert_eq!(calls.last().unwrap().trim(), "firewall-cmd --reload", "{log}");
            assert!(calls[..calls.len() - 1].iter().all(|c| c.contains("--permanent")), "{log}");
        }
        let (_, log) = run_with_fakes(&build_firewalld_add_rule_script("allow", "any", "8080", "tcp").unwrap(), &[("firewall-cmd", "")]);
        assert!(log.contains("--add-port=8080/tcp"), "{log}");
    }

    #[cfg(unix)]
    #[test]
    fn firewalld_deny_uses_negative_priority() {
        let script = build_firewalld_add_rule_script("deny", "2001:db8::/32", "22", "tcp").unwrap();
        let (_, log) = run_with_fakes(&script, &[("firewall-cmd", "")]);
        assert!(
            log.contains(r#"--add-rich-rule=rule priority="-100" family="ipv6" source address="2001:db8::/32" port port="22" protocol="tcp" drop"#),
            "{log}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn firewalld_delete_uses_exact_rich_rule() {
        let mut rich = fw_rule(1, "ALLOW", "5432/tcp", "203.0.113.42", false);
        rich.raw_spec = r#"rule family="ipv4" source address="203.0.113.42" port port="5432" protocol="tcp" accept"#.to_string();
        let (_, log) = run_with_fakes(&build_firewalld_delete_rule_script(&rich), &[("firewall-cmd", FAKE_FIREWALLD)]);
        assert!(log.contains(&format!("--permanent --remove-rich-rule={}", rich.raw_spec)), "{log}");
        let mut svc = fw_rule(2, "ALLOW", "22/tcp", "Anywhere", false);
        svc.raw_spec = "service:ssh".to_string();
        let (_, log) = run_with_fakes(&build_firewalld_delete_rule_script(&svc), &[("firewall-cmd", FAKE_FIREWALLD)]);
        assert!(log.contains("--permanent --remove-service=ssh"), "{log}");
        let script = format!("FAKE_QUERY=1\nexport FAKE_QUERY\n{}", build_firewalld_delete_rule_script(&svc));
        let (out, log) = run_with_fakes(&script, &[("firewall-cmd", FAKE_FIREWALLD)]);
        assert!(out.contains("SSHBOX_RULE_NOT_FOUND") && !log.contains("--remove"), "{out}\n{log}");
    }

    // The SSH port must be in the permanent config before firewalld starts with its default zone.
    #[cfg(unix)]
    #[test]
    fn firewalld_enable_opens_ssh_port_before_start() {
        let script = build_set_firewall_state_script(FirewallEngine::Firewalld, true, Some(2222)).unwrap();
        let (_, log) = run_with_fakes(&script, &[("firewall-offline-cmd", ""), ("systemctl", "")]);
        let open = log.find("firewall-offline-cmd --add-port=2222/tcp").expect(&log);
        assert!(open < log.find("systemctl enable --now firewalld").expect(&log), "{log}");
    }

    #[test]
    fn parses_access_keys_snapshot_with_sudoers_lock_state_and_authorized_keys() {
        let raw = r#"
---SUDOERS---
# Defaults specification
Defaults env_reset
root ALL=(ALL:ALL) ALL
%sudo ALL=(ALL:ALL) ALL
deploy ALL=(ALL) NOPASSWD:ALL
---ACCOUNT---
PASSWD:root:x:0:0:root:/root:/bin/bash
GROUPS:root
PASSWD_S:root P 01/01/2025 0 99999 7 -1
---AUTHORIZED_KEYS---
# Root emergency key
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q== macbook-pro@local

---ACCOUNT---
PASSWD:sshd:x:105:65534::/run/sshd:/usr/sbin/nologin
GROUPS:nogroup
PASSWD_S:sshd L 01/01/2025 0 99999 7 -1
---AUTHORIZED_KEYS---
---ACCOUNT---
PASSWD:ubuntu:x:1000:1000:Ubuntu:/home/ubuntu:/bin/bash
GROUPS:ubuntu adm sudo docker
PASSWD_S:ubuntu P 01/01/2025 0 99999 7 -1
---AUTHORIZED_KEYS---
ssh-rsa AAAAB3NzaC1yc2EAAAADAQABAAABAQC7vbqajDhA1234567890abcdefghijklmnop== legacy-ci-runner
from="10.0.0.0/8",no-agent-forwarding ecdsa-sha2-nistp256 AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBF1234567890xyz= bastion-key
---ACCOUNT---
PASSWD:deploy:x:1001:1001:Deploy Bot:/home/deploy:/bin/zsh
GROUPS:deploy docker
SHADOW:!$6$rounds=4096$saltsalt$hashedpasswordvalue
---AUTHORIZED_KEYS---
ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q==
---ACCOUNT---
PASSWD:nobody:x:65534:65534:nobody:/nonexistent:/usr/sbin/nologin
GROUPS:nogroup
SHADOW:*
---AUTHORIZED_KEYS---
"#;

        let snap = parse_access_keys_snapshot(raw);
        // Should only include root (uid 0), ubuntu (uid 1000), and deploy (uid 1001); filtering out sshd (105) and nobody (65534)
        assert_eq!(snap.accounts.len(), 3);

        let root = &snap.accounts[0];
        assert_eq!(root.username, "root");
        assert_eq!(root.uid, 0);
        assert_eq!(root.gid, 0);
        assert_eq!(root.home_dir, "/root");
        assert_eq!(root.shell, "/bin/bash");
        assert!(!root.locked);
        assert!(root.has_sudo);
        assert!(root.nopasswd_sudo);
        assert_eq!(root.authorized_keys.len(), 1);
        assert_eq!(root.authorized_keys[0].index, 1);
        assert_eq!(root.authorized_keys[0].key_type, "ssh-ed25519");
        assert_eq!(
            root.authorized_keys[0].public_key_body,
            "AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q=="
        );
        assert_eq!(root.authorized_keys[0].comment, "macbook-pro@local");
        assert_eq!(root.authorized_keys[0].short_preview, "AAAAC3NzaC...9x2Q==");

        let ubuntu = &snap.accounts[1];
        assert_eq!(ubuntu.username, "ubuntu");
        assert_eq!(ubuntu.uid, 1000);
        assert_eq!(ubuntu.gid, 1000);
        assert_eq!(ubuntu.home_dir, "/home/ubuntu");
        assert_eq!(ubuntu.shell, "/bin/bash");
        assert!(!ubuntu.locked);
        assert_eq!(
            ubuntu.groups,
            vec!["ubuntu", "adm", "sudo", "docker"]
        );
        assert!(ubuntu.has_sudo);
        assert!(!ubuntu.nopasswd_sudo);
        assert_eq!(ubuntu.authorized_keys.len(), 2);
        assert_eq!(ubuntu.authorized_keys[0].index, 1);
        assert_eq!(ubuntu.authorized_keys[0].key_type, "ssh-rsa");
        assert_eq!(ubuntu.authorized_keys[0].comment, "legacy-ci-runner");
        assert_eq!(ubuntu.authorized_keys[1].index, 2);
        assert_eq!(ubuntu.authorized_keys[1].key_type, "ecdsa-sha2-nistp256");
        assert_eq!(
            ubuntu.authorized_keys[1].public_key_body,
            "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIbmlzdHAyNTYAAABBBF1234567890xyz="
        );
        assert_eq!(ubuntu.authorized_keys[1].comment, "bastion-key");

        let deploy = &snap.accounts[2];
        assert_eq!(deploy.username, "deploy");
        assert_eq!(deploy.uid, 1001);
        assert_eq!(deploy.shell, "/bin/zsh");
        assert!(deploy.locked);
        assert!(deploy.has_sudo);
        assert!(deploy.nopasswd_sudo);
        assert_eq!(deploy.authorized_keys.len(), 1);
        assert_eq!(deploy.authorized_keys[0].comment, "");
    }

    #[test]
    fn validates_usernames() {
        assert!(is_valid_linux_username("ubuntu"));
        assert!(is_valid_linux_username("deploy_01-prod"));
        assert!(!is_valid_linux_username(""));
        assert!(!is_valid_linux_username("1invalid"));
        assert!(!is_valid_linux_username("UpperCase"));
        assert!(!is_valid_linux_username("bad;rm"));
    }

    // Fake getent/runuser/usermod/visudo/install that log calls; runuser drops "-u USER --" and runs the rest.
    #[cfg(unix)]
    fn user_fakes(home: &std::path::Path) -> Vec<(&'static str, String)> {
        vec![
            ("getent", format!(r#"case "$1" in passwd) echo "$2:x:1001:1001::{}:/bin/bash";; group) [ "$2" = sudo ];; esac"#, home.display())),
            ("runuser", "shift 3\n\"$@\"".to_string()),
            ("usermod", String::new()),
            ("useradd", String::new()),
            ("gpasswd", String::new()),
            ("chpasswd", "cat > \"$FAKE_LOG.chpasswd\"".to_string()),
            ("visudo", "exit \"${FAKE_VISUDO:-0}\"".to_string()),
            ("install", String::new()),
            ("id", "exit 1".to_string()),
        ]
    }

    #[cfg(unix)]
    fn run_user_script(script: &str, secret: &str, envs: &[(&str, &str)]) -> (String, String, std::path::PathBuf) {
        let dir = shell::temp_dir();
        let home = dir.join("home");
        std::fs::create_dir_all(&home).unwrap();
        let bin = dir.join("bin");
        let log = dir.join("calls.log");
        for (name, extra) in user_fakes(&home) {
            // runuser must keep the caller's exit status, so it does not end with exit 0.
            let tail = if name == "runuser" || name == "visudo" || name == "id" || name == "getent" { "" } else { "\nexit 0" };
            shell::fake_bin(&bin, name, &format!("echo \"{name} $*\" >> \"$FAKE_LOG\"\n{extra}{tail}"));
        }
        let secret_file = dir.join("secret");
        std::fs::write(&secret_file, secret).unwrap();
        let mut all = vec![("FAKE_LOG", log.to_str().unwrap()), ("SSHBOX_SECRET_FILE", secret_file.to_str().unwrap())];
        all.extend_from_slice(envs);
        let out = shell::run_local(script, b"", &bin, &all, &dir);
        (out, std::fs::read_to_string(&log).unwrap_or_default(), home)
    }

    // With UsePAM yes, "usermod -L" alone still lets SSH keys in; expiring the account stops them.
    #[cfg(unix)]
    #[test]
    fn lock_expires_account_so_keys_stop_working() {
        let (_, log, _) = run_user_script(&build_set_user_lock_script("deploy", true).unwrap(), "", &[]);
        assert!(log.contains("usermod -L -e 1 deploy"), "{log}");
        let (_, log, _) = run_user_script(&build_set_user_lock_script("deploy", false).unwrap(), "", &[]);
        assert!(log.contains("usermod -U -e  deploy"), "{log}");
        assert!(build_set_user_lock_script("root", true).is_err());
    }

    // Writing ~/.ssh as root lets a user symlink authorized_keys onto any root-owned file.
    #[cfg(unix)]
    #[test]
    fn key_file_ops_run_as_the_target_user() {
        let line = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAKEY alice@laptop\n";
        let script = build_authorize_key_script("alice", "AAAAC3NzaC1lZDI1NTE5AAAAKEY").unwrap();
        assert!(!script.contains(">> \"$home"), "{script}");
        let (out, log, home) = run_user_script(&script, line, &[]);
        assert!(log.contains("runuser -u alice --"), "{out}\n{log}");
        let ak = home.join(".ssh/authorized_keys");
        assert_eq!(std::fs::read_to_string(&ak).unwrap(), line);
    }

    #[cfg(unix)]
    #[test]
    fn authorize_does_not_duplicate_existing_key() {
        let line = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAKEY alice@laptop\n";
        let script = build_authorize_key_script("alice", "AAAAC3NzaC1lZDI1NTE5AAAAKEY").unwrap();
        let (_, _, home) = run_user_script(&script, line, &[]);
        let ak = home.join(".ssh/authorized_keys");
        // Re-run against the same home by pointing a fresh run at it.
        let dir = home.parent().unwrap().to_path_buf();
        let bin = dir.join("bin");
        let secret = dir.join("secret");
        let log = dir.join("calls.log");
        shell::run_local(&script, b"", &bin, &[("FAKE_LOG", log.to_str().unwrap()), ("SSHBOX_SECRET_FILE", secret.to_str().unwrap())], &dir);
        assert_eq!(std::fs::read_to_string(&ak).unwrap().matches("AAAAKEY").count(), 1);
    }

    // A last line without "\n" would glue the new key onto the old one and break both.
    #[cfg(unix)]
    #[test]
    fn authorize_appends_on_a_new_line_when_file_lacks_newline() {
        let dir = shell::temp_dir();
        let home = dir.join("home");
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        let ak = home.join(".ssh/authorized_keys");
        std::fs::write(&ak, "ssh-ed25519 AAAAOLD").unwrap();
        let bin = dir.join("bin");
        let log = dir.join("calls.log");
        for (name, extra) in user_fakes(&home) {
            shell::fake_bin(&bin, name, &format!("echo \"{name} $*\" >> \"$FAKE_LOG\"\n{extra}"));
        }
        let secret = dir.join("secret");
        std::fs::write(&secret, "ssh-ed25519 AAAANEW alice@new\n").unwrap();
        let script = build_authorize_key_script("alice", "AAAANEW").unwrap();
        let out = shell::run_local(
            &script,
            b"",
            &bin,
            &[("FAKE_LOG", log.to_str().unwrap()), ("SSHBOX_SECRET_FILE", secret.to_str().unwrap())],
            &dir,
        );
        assert_eq!(std::fs::read_to_string(&ak).unwrap(), "ssh-ed25519 AAAAOLD\nssh-ed25519 AAAANEW alice@new\n", "{out}");
    }

    // The old revoke used grep -Fv on a substring and also removed keys that merely contained it.
    #[cfg(unix)]
    #[test]
    fn revoke_matches_key_field_exactly() {
        let script = build_revoke_key_script("alice", "AAAAX").unwrap();
        let dir = shell::temp_dir();
        let home = dir.join("home");
        std::fs::create_dir_all(home.join(".ssh")).unwrap();
        let ak = home.join(".ssh/authorized_keys");
        std::fs::write(
            &ak,
            "# laptop ssh-ed25519 AAAAX kept as comment\nfrom=\"10.0.0.1\" ssh-ed25519 AAAAX alice@old\nssh-ed25519 AAAAXY alice@new\nssh-rsa AAAAB3 ops\n",
        )
        .unwrap();
        let bin = dir.join("bin");
        let log = dir.join("calls.log");
        for (name, extra) in user_fakes(&home) {
            shell::fake_bin(&bin, name, &format!("echo \"{name} $*\" >> \"$FAKE_LOG\"\n{extra}"));
        }
        let out = shell::run_local(&script, b"", &bin, &[("FAKE_LOG", log.to_str().unwrap())], &dir);
        assert_eq!(
            std::fs::read_to_string(&ak).unwrap(),
            "# laptop ssh-ed25519 AAAAX kept as comment\nssh-ed25519 AAAAXY alice@new\nssh-rsa AAAAB3 ops\n",
            "{out}"
        );
        assert!(build_revoke_key_script("alice", "AAAA X; rm").is_err());
    }

    // "Sudo with password" is useless without a password (the old code set '*').
    #[test]
    fn sudo_with_password_requires_password() {
        assert!(validate_user_password("deploy", None, &SudoMode::WithPassword).is_err());
        assert!(validate_user_password("deploy", Some(""), &SudoMode::WithPassword).is_err());
        assert_eq!(validate_user_password("deploy", Some("s3cret:pw"), &SudoMode::WithPassword), Ok(()));
        assert_eq!(validate_user_password("deploy", None, &SudoMode::NoPassword), Ok(()));
        assert!(validate_user_password("root", None, &SudoMode::None).is_err());
    }

    // chpasswd reads "user:password" lines; a newline would set a second user's password.
    #[test]
    fn user_password_with_newline_is_rejected() {
        assert!(validate_user_password("deploy", Some("a\nroot:x"), &SudoMode::WithPassword).is_err());
        assert!(validate_user_password("deploy", Some("a\rb"), &SudoMode::WithPassword).is_err());
    }

    // A broken sudoers file can disable sudo for everyone; it must be checked before it is installed.
    #[cfg(unix)]
    #[test]
    fn nopasswd_sudoers_checked_with_visudo_before_install() {
        let script = build_create_user_script("deploy", SudoMode::NoPassword).unwrap();
        let (out, log, _) = run_user_script(&script, "", &[]);
        let visudo = log.find("visudo -cf").expect(&format!("{out}\n{log}"));
        assert!(visudo < log.find("install -m 0440").expect(&log), "{log}");
        assert!(log.contains("usermod -aG sudo deploy"), "{log}");
        let (_, log, _) = run_user_script(&script, "", &[("FAKE_VISUDO", "1")]);
        assert!(!log.contains("install -m 0440"), "{log}");
    }

    #[cfg(unix)]
    #[test]
    fn create_user_with_password_feeds_chpasswd_from_secret() {
        let script = build_create_user_script("deploy", SudoMode::WithPassword).unwrap();
        let (out, log, home) = run_user_script(&script, "deploy:s3cret\n", &[]);
        let fed = std::fs::read_to_string(home.parent().unwrap().join("calls.log.chpasswd")).unwrap_or_default();
        assert_eq!(fed, "deploy:s3cret\n", "{out}\n{log}");
        assert!(!log.contains("usermod -p"), "{log}");
    }

    #[test]
    fn parses_locked_by_expiry() {
        let raw = "TODAY:20000\n---ACCOUNT---\nPASSWD:deploy:x:1001:1001::/home/deploy:/bin/bash\n\
SHADOW:deploy:*:19000:0:99999:7::1:\n---ACCOUNT---\nPASSWD:alice:x:1002:1002::/home/alice:/bin/bash\n\
SHADOW:alice:*:19000:0:99999:7:::\n---ACCOUNT---\nPASSWD:bob:x:1003:1003::/home/bob:/bin/bash\n\
SHADOW:bob:!$6$hash:19000:0:99999:7:::\n---ACCOUNT---\nPASSWD:carol:x:1004:1004::/home/carol:/bin/bash\n\
SHADOW:carol:!:19000:0:99999:7:::\n---ACCOUNT---\nPASSWD:dave:x:1005:1005::/home/dave:/bin/bash\n\
SHADOW:dave:!!:19000:0:99999:7:::\n";
        let snap = parse_access_keys_snapshot(raw);
        let locked = |u: &str| snap.accounts.iter().find(|a| a.username == u).unwrap().locked;
        assert!(locked("deploy"), "expired account");
        assert!(!locked("alice"), "'*' means no password, not locked");
        assert!(locked("bob"), "'!' before a password hash");
        // useradd leaves '!' (Debian/Ubuntu) or '!!' (RHEL): no password, but SSH keys still work.
        assert!(!locked("carol") && !locked("dave"));
    }
}
