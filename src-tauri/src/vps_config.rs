use crate::ssh_pool::{AuthUsed, SshPool};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum PrivilegeLevel {
    Root,
    SudoNoPassword,
    SudoPassword,
    None,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum FirewallEngine {
    Ufw,
    Firewalld,
    Iptables,
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
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum WriteTarget {
    DropIn,
    MainFile,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshdView {
    pub settings: SshdSettings,
    pub write_target: WriteTarget,
    pub rollback_pending: bool,
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
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AccessKeysSnapshot {
    pub accounts: Vec<LinuxAccountItem>,
}

fn is_public_bind_address(bind_addr: &str) -> bool {
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
        raw_spec: after_bracket.to_string(),
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
        raw_spec: spec.to_string(),
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
        FirewallEngine::None => "echo ENABLED:0\necho DEFAULT_INCOMING:allow\necho IPV6:0",
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
    let raw = run_privileged_script(host_id, pool, sudo_password, &firewall_read_script(probe.firewall_engine), None).await?;
    Ok(snapshot_after(&raw, probe.firewall_engine))
}

struct UfwRuleArgs {
    action: &'static str,
    source: String,
    port: String,
    proto: &'static str,
}

fn validate_rule_args(action: &str, source: &str, port: &str, proto: &str) -> Result<UfwRuleArgs, String> {
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

// deny/reject go to the top (`ufw prepend` handles IPv4 and IPv6 positions); allow appends.
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
    let verb = if matches!(a.action, "deny" | "reject") { "prepend " } else { "" };
    Ok(protos
        .iter()
        .map(|p| {
            let proto = if *p == "any" { String::new() } else { format!(" proto {p}") };
            format!("ufw {verb}{} {target}{proto}\n", a.action)
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
                script.push_str(&format!("ufw allow {p}/tcp\n"));
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
        FirewallEngine::Iptables | FirewallEngine::None => {
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
        FirewallEngine::Ufw | FirewallEngine::Firewalld => Ok(()),
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
    let ensure = crate::vps_guard::ssh_allow_needed(&probe.context, &current.rules).then_some(probe.context.server_port);
    let script = build_set_firewall_state_script(probe.firewall_engine, enable, ensure)?;
    run_firewall_write(host_id, pool, sudo_password, probe.firewall_engine, &script).await
}

// firewalld writes arrive in Task 10.
pub fn build_firewalld_add_rule_script(_action: &str, _source: &str, _port: &str, _proto: &str) -> Result<String, String> {
    Err(vps_err(ERR_UNSUPPORTED, "firewall_read_only"))
}

pub fn build_firewalld_delete_rule_script(_rule: &FirewallRuleItem) -> String {
    "echo SSHBOX_RULE_NOT_FOUND; exit 3\n".to_string()
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
elif command -v iptables >/dev/null 2>&1; then fw=iptables
else fw=none
fi
echo "FW:$fw"
"#;

pub fn parse_probe_output(raw: &str, auth: AuthUsed) -> Result<VpsProbe, String> {
    let field = |key: &str| {
        raw.lines()
            .find_map(|l| l.trim().strip_prefix(key).map(|v| v.trim().to_string()))
            .unwrap_or_default()
    };
    if field("UNAME:") != "Linux" {
        return Err(vps_err(ERR_UNSUPPORTED, "linux_only"));
    }
    let privilege = match field("SUDO:").as_str() {
        "root" => PrivilegeLevel::Root,
        "nopasswd" => PrivilegeLevel::SudoNoPassword,
        "password_ok" => PrivilegeLevel::SudoPassword,
        "password_bad" => return Err(vps_err(ERR_SUDO_DENIED, "incorrect sudo password")),
        _ => PrivilegeLevel::None,
    };
    // $SSH_CONNECTION = "client_ip client_port server_ip server_port"
    let conn = field("SSH_CONNECTION:");
    let parts: Vec<&str> = conn.split_whitespace().collect();
    let [client_ip, _, server_ip, port] = parts.as_slice() else {
        return Err(vps_err(ERR_UNSUPPORTED, "no_ssh_connection"));
    };
    let server_port = port
        .parse::<u16>()
        .map_err(|_| vps_err(ERR_UNSUPPORTED, "no_ssh_connection"))?;
    let firewall_engine = match field("FW:").as_str() {
        "ufw" => FirewallEngine::Ufw,
        "firewalld" => FirewallEngine::Firewalld,
        "iptables" => FirewallEngine::Iptables,
        _ => FirewallEngine::None,
    };
    Ok(VpsProbe {
        privilege,
        context: ConnectionContext {
            login_user: field("USER:"),
            auth,
            client_ip: client_ip.to_string(),
            server_ip: server_ip.to_string(),
            server_port,
        },
        os_id: field("OS_ID:"),
        os_version: field("OS_VERSION:"),
        openssh_version: field("OPENSSH:"),
        has_systemd: field("SYSTEMD:") == "1",
        ssh_socket_active: field("SSH_SOCKET:") == "active",
        selinux_enforcing: field("SELINUX:").eq_ignore_ascii_case("enforcing"),
        firewall_engine,
    })
}

pub async fn probe(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<VpsProbe, String> {
    let host = find_remote_host(host_id)?;
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

pub fn render_dropin(s: &SshdSettings) -> String {
    let mut out = String::from("# Managed by ssh-box. Changes here are overwritten by Server Config.\n");
    for line in sshd_directives(s).into_iter().filter_map(|(_, l)| l) {
        out.push_str(&line);
        out.push('\n');
    }
    out
}

// Used only when sshd_config has no leading Include: replaces managed directives in place,
// keeps comments, and appends missing ones before the first Match block.
pub fn render_sshd_config(base_raw: &str, s: &SshdSettings) -> String {
    let managed = sshd_directives(s);
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
        before.push("# --- Managed by ssh-box ---".to_string());
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

const SSHD_PART: &str = "---SSHBOX-PART---";

pub fn parse_sshd_view(raw: &str) -> Result<SshdView, String> {
    let parts: Vec<&str> = raw.split(SSHD_PART).collect();
    let [effective, main_cfg, pending] = parts.as_slice() else {
        return Err(vps_err(ERR_COMMAND_FAILED, raw.trim()));
    };
    Ok(SshdView {
        settings: parse_sshd_t(effective),
        write_target: sshd_write_target(main_cfg),
        rollback_pending: pending.trim() == "active",
    })
}

pub async fn get_sshd_view(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<SshdView, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    let c = &probe.context;
    // -C applies Match blocks for this exact connection, so the result is what the app gets.
    let spec = format!(
        "user={},host={},addr={},laddr={},lport={}",
        c.login_user, c.client_ip, c.client_ip, c.server_ip, c.server_port
    );
    let script = format!(
        "sshd -T -C {spec} || exit 1\necho {SSHD_PART}\ncat {SSHD_MAIN_CONFIG}\necho {SSHD_PART}\n\
systemctl is-active sshbox-rollback.timer 2>/dev/null || true\n",
        spec = shell_single_quote(&spec),
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
    pub new_port: Option<u16>,
    pub firewall_engine: FirewallEngine,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshdApplyStarted {
    pub rollback_at: i64,
    pub server_now: i64,
    pub new_port: u16,
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

fn sshd_restore_fn(file: &str) -> String {
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
    let semanage = match plan.new_port {
        Some(port) if plan.selinux_enforcing && port != 22 => format!(
            "semanage port -a -t ssh_port_t -p tcp {port} 2>/dev/null || semanage port -m -t ssh_port_t -p tcp {port}\n"
        ),
        _ => String::new(),
    };
    let port_check = plan
        .new_port
        .map(|p| p.to_string())
        .unwrap_or_else(|| "$(sed -n 's/^Port \\([0-9]*\\).*/\\1/p' \"$F\" | head -n 1)".to_string());
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
cat > "$R" << 'SSHBOX_ROLLBACK_EOF'
{rollback}SSHBOX_ROLLBACK_EOF
chmod 700 "$R"
trap 'rc=$?; [ "$rc" -eq 0 ] || /bin/sh "$R"; exit "$rc"' EXIT
systemd-run --on-active=60 --unit=sshbox-rollback /bin/sh "$R" >/dev/null
echo "NOW:$(date +%s)"
echo "ROLLBACK_AT:$(( $(date +%s) + 60 ))"
{semanage}{firewall}{reload}
P={port_check}
ss -tlnpH "sport = :$P" | grep -Eq 'sshd|systemd' || {{ echo SSHBOX_PORT_NOT_LISTENING; exit 4; }}
"#,
        restore = sshd_restore_fn(file),
        r = shell_single_quote(rollback_path),
        rollback = build_sshd_rollback_script(file, plan),
        firewall = sshd_firewall_cmd(plan, true),
        reload = sshd_reload_cmd(plan, false),
    )
}

pub fn apply_preflight(probe: &VpsProbe, view: &SshdView, desired: &SshdSettings) -> Result<(), String> {
    if !probe.has_systemd {
        return Err(vps_err(ERR_UNSUPPORTED, "no_systemd"));
    }
    if view.rollback_pending {
        return Err(vps_err(ERR_COMMAND_FAILED, "a previous change is still waiting for verification"));
    }
    crate::vps_guard::check_sshd(&probe.context, desired)
}

pub fn parse_apply_started(raw: &str, new_port: u16) -> Result<SshdApplyStarted, String> {
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

pub async fn apply_sshd_settings(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    desired: SshdSettings,
) -> Result<SshdApplyStarted, String> {
    let probe = probe(host_id, pool, sudo_password.clone()).await?;
    let view = get_sshd_view(host_id, pool, sudo_password.clone()).await?;
    apply_preflight(&probe, &view, &desired)?;
    let plan = ReloadPlan {
        ssh_socket_active: probe.ssh_socket_active,
        selinux_enforcing: probe.selinux_enforcing,
        new_port: (desired.port != view.settings.port).then_some(desired.port),
        firewall_engine: probe.firewall_engine,
    };
    let (file, content) = match view.write_target {
        WriteTarget::DropIn => (SSHD_DROPIN, render_dropin(&desired)),
        WriteTarget::MainFile => {
            let main = run_privileged_script(host_id, pool, sudo_password.clone(), &format!("cat {SSHD_MAIN_CONFIG}"), None).await?;
            (SSHD_MAIN_CONFIG, render_sshd_config(&main, &desired))
        }
    };
    let script = build_sshd_apply_script(file, SSHD_ROLLBACK_SCRIPT, &plan);
    let raw = run_privileged_script(host_id, pool, sudo_password, &script, Some(&content)).await?;
    parse_apply_started(&raw, desired.port)
}

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
    let reason = match pool.test_connection(&target, None).await {
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

fn sshd_cleanup_script() -> String {
    format!(
        "systemctl stop sshbox-rollback.timer 2>/dev/null || true\n\
rm -f {d}.sshbox.bak {d}.sshbox.absent {m}.sshbox.bak {m}.sshbox.absent {r}\n",
        d = SSHD_DROPIN,
        m = SSHD_MAIN_CONFIG,
        r = SSHD_ROLLBACK_SCRIPT
    )
}

fn sshd_rollback_now_script() -> String {
    format!(
        "if [ -e {r} ]; then /bin/sh {r}; fi\nsystemctl stop sshbox-rollback.timer 2>/dev/null || true\n",
        r = SSHD_ROLLBACK_SCRIPT
    )
}

// Step 8–9: verify with a new connection, then keep the change or roll it back now.
pub async fn finish_sshd_apply(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    new_port: u16,
) -> Result<SshdView, String> {
    let host = find_remote_host(host_id)?;
    let expected_fp = pool
        .session_fingerprint(host_id)
        .await
        .ok_or_else(|| vps_err(ERR_COMMAND_FAILED, "no SSH session for this host"))?;
    if let Err(e) = verify_new_connection(pool, &host, new_port, &expected_fp).await {
        run_privileged_script(host_id, pool, sudo_password, &sshd_rollback_now_script(), None).await?;
        return Err(e);
    }
    run_privileged_script(host_id, pool, sudo_password.clone(), &sshd_cleanup_script(), None).await?;
    get_sshd_view(host_id, pool, sudo_password).await
}

pub async fn revert_sshd_settings(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<SshdView, String> {
    run_privileged_script(host_id, pool, sudo_password.clone(), &sshd_rollback_now_script(), None).await?;
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
    }
}

pub fn parse_access_keys_snapshot(raw: &str) -> AccessKeysSnapshot {
    let mut snap = AccessKeysSnapshot::default();
    let mut sudoers_lines: Vec<String> = Vec::new();
    let mut current_acc: Option<LinuxAccountItem> = None;
    let mut section = "";

    let push_if_eligible = |acc: LinuxAccountItem, out: &mut Vec<LinuxAccountItem>| {
        if !acc.username.is_empty() && (acc.uid == 0 || (acc.uid >= 1000 && acc.uid < 65534)) {
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
                } else if let Some(v) = trimmed.strip_prefix("PASSWD_S:") {
                    let toks: Vec<&str> = v.split_whitespace().collect();
                    let st = if toks.len() >= 2 {
                        toks[1]
                    } else {
                        toks.first().copied().unwrap_or("")
                    };
                    if st == "L" || st == "LK" {
                        acc.locked = true;
                    } else if st == "P" || st == "PS" || st == "NP" {
                        acc.locked = false;
                    }
                } else if let Some(v) = trimmed
                    .strip_prefix("SHADOW:")
                    .or_else(|| trimmed.strip_prefix("SHADOW_PW:"))
                {
                    let pw_field = if v.contains(':') {
                        v.split(':').nth(1).unwrap_or("").trim()
                    } else {
                        v.trim()
                    };
                    if pw_field.starts_with('!') || pw_field.starts_with('*') {
                        acc.locked = true;
                    } else if !pw_field.is_empty() {
                        acc.locked = false;
                    }
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

pub async fn get_access_keys_snapshot(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    let script = r#"
echo "---SUDOERS---"
if [ -f /etc/sudoers ]; then
  grep -vE '^[[:space:]]*(#|$)' /etc/sudoers 2>/dev/null || true
fi
if [ -d /etc/sudoers.d ]; then
  for sf in /etc/sudoers.d/*; do
    if [ -f "$sf" ]; then
      grep -vE '^[[:space:]]*(#|$)' "$sf" 2>/dev/null || true
    fi
  done
fi
awk -F: '$3 == 0 || ($3 >= 1000 && $3 < 65534) { print $0 }' /etc/passwd 2>/dev/null | while IFS= read -r pwline; do
  usr=$(printf '%s\n' "$pwline" | cut -d: -f1)
  home=$(printf '%s\n' "$pwline" | cut -d: -f6)
  [ -z "$usr" ] && continue
  echo "---ACCOUNT---"
  echo "PASSWD:$pwline"
  grps=$(id -Gn "$usr" 2>/dev/null | tr '\n' ' ')
  echo "GROUPS:$grps"
  pstat=$(passwd -S "$usr" 2>/dev/null | head -n 1)
  if [ -n "$pstat" ]; then
    echo "PASSWD_S:$pstat"
  else
    shad=$(grep -E "^${usr}:" /etc/shadow 2>/dev/null | head -n 1 | cut -d: -f2)
    echo "SHADOW:$shad"
  fi
  echo "---AUTHORIZED_KEYS---"
  if [ -n "$home" ] && [ -f "$home/.ssh/authorized_keys" ]; then
    cat "$home/.ssh/authorized_keys" 2>/dev/null || true
  fi
done
"#;
    let raw = run_privileged_script(host_id, pool, sudo_password, script, None).await?;
    Ok(parse_access_keys_snapshot(&raw))
}

pub fn build_authorize_ssh_key_script(
    username: &str,
    public_key: &str,
) -> Result<String, String> {
    let clean_user = username.trim();
    if !is_valid_linux_username(clean_user) {
        return Err(format!("Invalid Linux username: {username}"));
    }

    let clean_line = public_key
        .lines()
        .map(|l| l.trim())
        .find(|l| !l.is_empty() && !l.starts_with('#'))
        .ok_or_else(|| "Public key cannot be empty.".to_string())?;

    let parsed = parse_authorized_key_line(clean_line, 1).ok_or_else(|| {
        "Invalid SSH public key format. Expected 'ssh-ed25519 AAAA... [comment]' or 'ssh-rsa AAAA...'."
            .to_string()
    })?;

    let user_q = shell_single_quote(clean_user);
    let body_q = shell_single_quote(&parsed.public_key_body);
    let line_q = shell_single_quote(&parsed.raw_line);

    Ok(format!(
        r#"home_dir=$(getent passwd {user_q} | cut -d: -f6)
if [ -z "$home_dir" ]; then
  echo "Could not resolve home directory for user {clean_user}."
  exit 1
fi
mkdir -p "$home_dir/.ssh"
chmod 700 "$home_dir/.ssh"
touch "$home_dir/.ssh/authorized_keys"
if ! grep -Fq {body_q} "$home_dir/.ssh/authorized_keys" 2>/dev/null; then
  printf '%s\n' {line_q} >> "$home_dir/.ssh/authorized_keys"
fi
chmod 600 "$home_dir/.ssh/authorized_keys"
chown -R {user_q}:{user_q} "$home_dir/.ssh" || chown -R {user_q} "$home_dir/.ssh""#
    ))
}

pub fn build_revoke_ssh_key_script(
    username: &str,
    key_body: &str,
) -> Result<String, String> {
    let clean_user = username.trim();
    if !is_valid_linux_username(clean_user) {
        return Err(format!("Invalid Linux username: {username}"));
    }
    let clean_body = key_body.trim();
    if clean_body.is_empty() || clean_body.chars().any(|c| c.is_whitespace()) {
        return Err("Invalid SSH key body to revoke.".to_string());
    }

    let user_q = shell_single_quote(clean_user);
    let body_q = shell_single_quote(clean_body);

    Ok(format!(
        r#"home_dir=$(getent passwd {user_q} | cut -d: -f6)
if [ -z "$home_dir" ]; then
  echo "Could not resolve home directory for user {clean_user}."
  exit 1
fi
ak_file="$home_dir/.ssh/authorized_keys"
if [ -f "$ak_file" ]; then
  grep -Fv {body_q} "$ak_file" > "${{ak_file}}.sshbox.tmp" || true
  mv "${{ak_file}}.sshbox.tmp" "$ak_file"
  chmod 600 "$ak_file"
  chown -R {user_q}:{user_q} "$home_dir/.ssh" || chown -R {user_q} "$home_dir/.ssh"
fi"#
    ))
}

pub fn build_create_linux_user_script(
    username: &str,
    shell: &str,
    sudo_access: bool,
    nopasswd_sudo: bool,
    docker_access: bool,
    initial_public_key: Option<&str>,
) -> Result<String, String> {
    let clean_user = username.trim();
    if !is_valid_linux_username(clean_user) {
        return Err(format!(
            "Invalid username '{username}'. Must match ^[a-z_][a-z0-9_-]{{0,30}}$."
        ));
    }
    if clean_user == "root" {
        return Err("Account 'root' already exists.".to_string());
    }

    let clean_shell = {
        let s = shell.trim();
        if s.is_empty() {
            "/bin/bash"
        } else {
            if !s.starts_with('/')
                || !s
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '/' || c == '_' || c == '-' || c == '.')
            {
                return Err(format!("Invalid login shell path: {shell}"));
            }
            s
        }
    };

    let user_q = shell_single_quote(clean_user);
    let shell_q = shell_single_quote(clean_shell);
    let sudoers_file = format!("/etc/sudoers.d/90-sshbox-{clean_user}");
    let sudoers_file_q = shell_single_quote(&sudoers_file);
    let sudoers_rule_q = shell_single_quote(&format!("{clean_user} ALL=(ALL) NOPASSWD:ALL"));

    let mut steps: Vec<String> = vec![format!(
        r#"if id -u {user_q} >/dev/null 2>&1; then
  echo "User {clean_user} already exists on this server."
  exit 1
fi
useradd -m -s {shell_q} {user_q}
usermod -p '*' {user_q} 2>/dev/null || true"#
    )];

    if sudo_access || nopasswd_sudo {
        steps.push(format!(
            r#"if getent group sudo >/dev/null 2>&1; then
  usermod -aG sudo {user_q}
elif getent group wheel >/dev/null 2>&1; then
  usermod -aG wheel {user_q}
fi"#
        ));
    }

    if docker_access {
        steps.push(format!(
            r#"if getent group docker >/dev/null 2>&1; then
  usermod -aG docker {user_q}
fi"#
        ));
    }

    if nopasswd_sudo {
        steps.push(format!(
            r#"mkdir -p /etc/sudoers.d
printf '%s\n' {sudoers_rule_q} > {sudoers_file_q}
chmod 0440 {sudoers_file_q}
if command -v visudo >/dev/null 2>&1; then
  if ! visudo -cf {sudoers_file_q} >/dev/null 2>&1; then
    rm -f {sudoers_file_q}
    echo "visudo validation failed for {sudoers_file}"
    exit 1
  fi
fi"#
        ));
    }

    if let Some(pk) = initial_public_key {
        if !pk.trim().is_empty() {
            let auth_step = build_authorize_ssh_key_script(clean_user, pk)?;
            steps.push(auth_step);
        }
    }

    Ok(steps.join("\n"))
}

pub fn build_update_linux_user_privilege_script(
    username: &str,
    sudo_access: bool,
    nopasswd_sudo: bool,
    locked: bool,
) -> Result<String, String> {
    let clean_user = username.trim();
    if !is_valid_linux_username(clean_user) {
        return Err(format!("Invalid Linux username: {username}"));
    }
    if clean_user == "root" {
        return Err("Cannot modify privilege or lock state of the root account.".to_string());
    }

    let user_q = shell_single_quote(clean_user);
    let sudoers_file = format!("/etc/sudoers.d/90-sshbox-{clean_user}");
    let sudoers_file_q = shell_single_quote(&sudoers_file);
    let sudoers_rule_q = shell_single_quote(&format!("{clean_user} ALL=(ALL) NOPASSWD:ALL"));

    let sudo_group_step = if sudo_access || nopasswd_sudo {
        format!(
            r#"if getent group sudo >/dev/null 2>&1; then
  usermod -aG sudo {user_q}
elif getent group wheel >/dev/null 2>&1; then
  usermod -aG wheel {user_q}
fi"#
        )
    } else {
        format!(
            r#"if getent group sudo >/dev/null 2>&1; then
  gpasswd -d {user_q} sudo >/dev/null 2>&1 || deluser {user_q} sudo >/dev/null 2>&1 || true
fi
if getent group wheel >/dev/null 2>&1; then
  gpasswd -d {user_q} wheel >/dev/null 2>&1 || deluser {user_q} wheel >/dev/null 2>&1 || true
fi"#
        )
    };

    let nopasswd_step = if nopasswd_sudo {
        format!(
            r#"mkdir -p /etc/sudoers.d
printf '%s\n' {sudoers_rule_q} > {sudoers_file_q}
chmod 0440 {sudoers_file_q}
if command -v visudo >/dev/null 2>&1; then
  if ! visudo -cf {sudoers_file_q} >/dev/null 2>&1; then
    rm -f {sudoers_file_q}
    echo "visudo validation failed for {sudoers_file}"
    exit 1
  fi
fi"#
        )
    } else {
        format!("rm -f {sudoers_file_q}")
    };

    let lock_step = if locked {
        format!("usermod -L {user_q}")
    } else {
        format!("usermod -U {user_q} 2>/dev/null || usermod -p '*' {user_q} 2>/dev/null || true")
    };

    Ok(format!("{sudo_group_step}\n{nopasswd_step}\n{lock_step}"))
}

pub async fn authorize_ssh_key(
    host_id: &str,
    pool: &SshPool,
    username: String,
    public_key: String,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    let script = build_authorize_ssh_key_script(&username, &public_key)?;
    run_privileged_script(host_id, pool, sudo_password.clone(), &script, None).await?;
    get_access_keys_snapshot(host_id, pool, sudo_password).await
}

pub async fn revoke_ssh_key(
    host_id: &str,
    pool: &SshPool,
    username: String,
    key_body: String,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    let script = build_revoke_ssh_key_script(&username, &key_body)?;
    run_privileged_script(host_id, pool, sudo_password.clone(), &script, None).await?;
    get_access_keys_snapshot(host_id, pool, sudo_password).await
}

pub async fn create_linux_user(
    host_id: &str,
    pool: &SshPool,
    username: String,
    shell: String,
    sudo_access: bool,
    nopasswd_sudo: bool,
    docker_access: bool,
    initial_public_key: Option<String>,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    let script = build_create_linux_user_script(
        &username,
        &shell,
        sudo_access,
        nopasswd_sudo,
        docker_access,
        initial_public_key.as_deref(),
    )?;
    run_privileged_script(host_id, pool, sudo_password.clone(), &script, None).await?;
    get_access_keys_snapshot(host_id, pool, sudo_password).await
}

pub async fn update_linux_user_privilege(
    host_id: &str,
    pool: &SshPool,
    username: String,
    sudo_access: bool,
    nopasswd_sudo: bool,
    locked: bool,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    let script =
        build_update_linux_user_privilege_script(&username, sudo_access, nopasswd_sudo, locked)?;
    run_privileged_script(host_id, pool, sudo_password.clone(), &script, None).await?;
    get_access_keys_snapshot(host_id, pool, sudo_password).await
}

#[cfg(test)]
mod tests {
    use super::*;

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
        let out = render_dropin(&sample_settings());
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
        let rendered = render_sshd_config(base_raw, &sample_settings());
        for kept in ["# Ubuntu default sshd_config", "UsePAM yes", "Subsystem sftp /usr/lib/openssh/sftp-server"] {
            assert!(rendered.contains(kept), "{kept}");
        }
        assert!(rendered.find("Port 32022").unwrap() < rendered.find("Match User git").unwrap());
        let before_match = &rendered[..rendered.find("Match User git").unwrap()];
        assert_eq!(parse_sshd_t(before_match), sample_settings());
        let forced = SshdSettings { permit_root_login: "forced-commands-only".to_string(), ..sample_settings() };
        assert!(render_sshd_config(base_raw, &forced).contains("PermitRootLogin forced-commands-only"));
    }

    #[test]
    fn parses_sshd_view_parts() {
        let raw = "port 2222\n---SSHBOX-PART---\nInclude /etc/ssh/sshd_config.d/*.conf\n---SSHBOX-PART---\nactive\n";
        let v = parse_sshd_view(raw).unwrap();
        assert_eq!(v.settings.port, 2222);
        assert_eq!(v.write_target, WriteTarget::DropIn);
        assert!(v.rollback_pending);
        let v = parse_sshd_view("port 22\n---SSHBOX-PART---\nPort 22\n---SSHBOX-PART---\ninactive\n").unwrap();
        assert!(!v.rollback_pending && v.write_target == WriteTarget::MainFile);
        assert!(parse_sshd_view("garbage").unwrap_err().starts_with("COMMAND_FAILED"));
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
        ReloadPlan { ssh_socket_active: socket, selinux_enforcing: selinux, new_port, firewall_engine: engine }
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

    fn probe_for(ctx_user: &str, has_systemd: bool) -> VpsProbe {
        let mut p = parse_probe_output(PROBE_UBUNTU, key_auth()).unwrap();
        p.context.login_user = ctx_user.to_string();
        p.has_systemd = has_systemd;
        p
    }

    #[test]
    fn apply_refuses_change_blocked_by_guard() {
        let view = SshdView { settings: SshdSettings::default(), write_target: WriteTarget::DropIn, rollback_pending: false };
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
        let s = parse_apply_started("noise\nNOW:1000\nROLLBACK_AT:1060\n", 2222).unwrap();
        assert_eq!((s.server_now, s.rollback_at, s.new_port), (1000, 1060, 2222));
        assert!(parse_apply_started("nothing", 22).unwrap_err().starts_with("COMMAND_FAILED"));
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

    #[test]
    fn probe_rejects_non_linux() {
        let raw = PROBE_UBUNTU.replace("UNAME:Linux", "UNAME:Darwin");
        assert!(parse_probe_output(&raw, key_auth()).unwrap_err().starts_with("UNSUPPORTED"));
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
            raw_spec: format!("{to} {action} IN {from}"),
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

    // ufw evaluates rules top-down; an appended deny below an allow never applies.
    #[cfg(unix)]
    #[test]
    fn deny_rules_are_inserted_first() {
        let script = build_ufw_add_rule_script("deny", "198.51.100.9", "80", "tcp").unwrap();
        let (_, log) = run_with_fakes(&script, &[("ufw", "")]);
        assert!(log.contains("ufw prepend deny from 198.51.100.9 to any port 80 proto tcp"), "{log}");
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

    // Enabling ufw with incoming deny must open the app's SSH port first.
    #[cfg(unix)]
    #[test]
    fn enable_adds_ssh_allow_first_when_missing() {
        let script = build_set_firewall_state_script(FirewallEngine::Ufw, true, Some(2222)).unwrap();
        let (_, log) = run_with_fakes(&script, &[("ufw", "")]);
        let allow = log.find("ufw allow 2222/tcp").expect(&log);
        assert!(allow < log.find("ufw --force enable").expect(&log), "{log}");
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
    fn validates_usernames_and_builds_key_and_user_privilege_scripts() {
        assert!(is_valid_linux_username("ubuntu"));
        assert!(is_valid_linux_username("deploy_01-prod"));
        assert!(!is_valid_linux_username(""));
        assert!(!is_valid_linux_username("1invalid"));
        assert!(!is_valid_linux_username("UpperCase"));
        assert!(!is_valid_linux_username("bad;rm"));

        let auth_script = build_authorize_ssh_key_script(
            "ubuntu",
            "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q== macbook@local",
        )
        .unwrap();
        assert!(auth_script.contains("getent passwd 'ubuntu' | cut -d: -f6"));
        assert!(auth_script.contains("chmod 700"));
        assert!(auth_script.contains("chmod 600"));
        assert!(auth_script.contains("chown -R 'ubuntu':'ubuntu'"));
        assert!(auth_script.contains("AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q=="));

        let revoke_script = build_revoke_ssh_key_script(
            "ubuntu",
            "AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q==",
        )
        .unwrap();
        assert!(revoke_script.contains("grep -Fv 'AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q=='"));

        let create_script = build_create_linux_user_script(
            "deploy",
            "/bin/bash",
            true,
            true,
            true,
            Some("ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIGbXz8J9R2vL3mK8pQ7wX1yZ4nB5vC6mD7eF8gH99x2Q== key1"),
        )
        .unwrap();
        assert!(create_script.contains("useradd -m -s '/bin/bash' 'deploy'"));
        assert!(create_script.contains("/etc/sudoers.d/90-sshbox-deploy"));
        assert!(create_script.contains("deploy ALL=(ALL) NOPASSWD:ALL"));
        assert!(create_script.contains("visudo -cf"));
        assert!(create_script.contains("usermod -aG docker 'deploy'"));

        let priv_script = build_update_linux_user_privilege_script("deploy", false, false, true).unwrap();
        assert!(priv_script.contains("rm -f '/etc/sudoers.d/90-sshbox-deploy'"));
        assert!(priv_script.contains("usermod -L 'deploy'"));
    }
}
