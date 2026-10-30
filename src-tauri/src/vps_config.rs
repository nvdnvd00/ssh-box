use crate::ssh_pool::SshPool;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct VpsPrivilegeInfo {
    pub user: String,
    pub uid: u32,
    pub is_root: bool,
    pub passwordless_sudo: bool,
    pub sudo_unlocked: bool,
    pub os_pretty_name: String,
    pub detected_client_ip: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshdConfigSnapshot {
    pub port: u16,
    pub permit_root_login: String,
    pub password_authentication: bool,
    pub pubkey_authentication: bool,
    pub permit_empty_passwords: bool,
    pub max_auth_tries: u32,
    pub login_grace_time: u32,
    pub client_alive_interval: u32,
    pub client_alive_count_max: u32,
    pub allow_users: Vec<String>,
    pub x11_forwarding: bool,
    pub allow_tcp_forwarding: bool,
    pub allow_agent_forwarding: bool,
    pub address_family: String,
    pub authorized_keys_count: u32,
    pub security_score: u32,
    pub raw_config: String,
    pub rollback_pending: bool,
}

impl Default for SshdConfigSnapshot {
    fn default() -> Self {
        Self {
            port: 22,
            permit_root_login: "prohibit-password".to_string(),
            password_authentication: true,
            pubkey_authentication: true,
            permit_empty_passwords: false,
            max_auth_tries: 6,
            login_grace_time: 120,
            client_alive_interval: 0,
            client_alive_count_max: 3,
            allow_users: Vec::new(),
            x11_forwarding: false,
            allow_tcp_forwarding: true,
            allow_agent_forwarding: true,
            address_family: "any".to_string(),
            authorized_keys_count: 0,
            security_score: 0,
            raw_config: String::new(),
            rollback_pending: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshdConfigUpdate {
    pub port: u16,
    pub permit_root_login: String,
    pub password_authentication: bool,
    pub pubkey_authentication: bool,
    pub permit_empty_passwords: bool,
    pub max_auth_tries: u32,
    pub login_grace_time: u32,
    pub client_alive_interval: u32,
    pub client_alive_count_max: u32,
    pub allow_users: Vec<String>,
    pub x11_forwarding: bool,
    pub allow_tcp_forwarding: bool,
    pub allow_agent_forwarding: bool,
    pub address_family: String,
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
    pub detected_client_ip: String,
    pub ssh_port: u16,
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
            detected_client_ip: String::new(),
            ssh_port: 22,
            rules: Vec::new(),
            listening_sockets: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Fail2banJailItem {
    pub name: String,
    pub currently_failed: u32,
    pub total_failed: u32,
    pub currently_banned: u32,
    pub total_banned: u32,
    pub banned_ips: Vec<String>,
    pub findtime_secs: u32,
    pub bantime_secs: i64,
    pub maxretry: u32,
}

impl Default for Fail2banJailItem {
    fn default() -> Self {
        Self {
            name: "sshd".to_string(),
            currently_failed: 0,
            total_failed: 0,
            currently_banned: 0,
            total_banned: 0,
            banned_ips: Vec::new(),
            findtime_secs: 600,
            bantime_secs: 3600,
            maxretry: 5,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct SshAuthAuditEvent {
    pub timestamp: String,
    pub event_type: String,
    pub username: String,
    pub ip: String,
    pub method: String,
    pub port: u16,
    pub raw_line: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AttackerIpSummary {
    pub ip: String,
    pub failed_count: u32,
    pub last_target_user: String,
    pub is_currently_banned: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Fail2banSnapshot {
    pub installed: bool,
    pub running: bool,
    pub detected_client_ip: String,
    pub jails: Vec<Fail2banJailItem>,
    pub ignore_ips: Vec<String>,
    pub auth_events: Vec<SshAuthAuditEvent>,
    pub top_attacker_ips: Vec<AttackerIpSummary>,
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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KernelSysctlSnapshot {
    pub kernel_release: String,
    pub available_congestion_controls: Vec<String>,
    pub bbr_supported: bool,
    pub bbr_active: bool,
    pub congestion_control: String,
    pub default_qdisc: String,
    pub swappiness: u32,
    pub overcommit_memory: u32,
    pub vfs_cache_pressure: u32,
    pub file_max: u64,
    pub somaxconn: u32,
    pub tcp_max_syn_backlog: u32,
    pub tcp_tw_reuse: bool,
    pub tcp_fin_timeout: u32,
    pub tcp_syncookies: bool,
    pub icmp_echo_ignore_all: bool,
    pub rp_filter: bool,
    pub accept_redirects: bool,
    pub accept_source_route: bool,
    pub ip_forward: bool,
    pub persisted_config: String,
}

impl Default for KernelSysctlSnapshot {
    fn default() -> Self {
        Self {
            kernel_release: "Linux".to_string(),
            available_congestion_controls: vec!["cubic".to_string(), "reno".to_string()],
            bbr_supported: false,
            bbr_active: false,
            congestion_control: "cubic".to_string(),
            default_qdisc: "fq_codel".to_string(),
            swappiness: 60,
            overcommit_memory: 0,
            vfs_cache_pressure: 100,
            file_max: 524288,
            somaxconn: 4096,
            tcp_max_syn_backlog: 4096,
            tcp_tw_reuse: false,
            tcp_fin_timeout: 60,
            tcp_syncookies: true,
            icmp_echo_ignore_all: false,
            rp_filter: true,
            accept_redirects: false,
            accept_source_route: false,
            ip_forward: false,
            persisted_config: String::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct KernelSysctlUpdate {
    pub congestion_control: String,
    pub default_qdisc: String,
    pub swappiness: u32,
    pub overcommit_memory: u32,
    pub vfs_cache_pressure: u32,
    pub file_max: u64,
    pub somaxconn: u32,
    pub tcp_max_syn_backlog: u32,
    pub tcp_tw_reuse: bool,
    pub tcp_fin_timeout: u32,
    pub tcp_syncookies: bool,
    pub icmp_echo_ignore_all: bool,
    pub rp_filter: bool,
    pub accept_redirects: bool,
    pub accept_source_route: bool,
    pub ip_forward: bool,
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

fn rule_port_matches(rule_to_port: &str, rule_proto: &str, sock_port: u16, sock_proto: &str) -> bool {
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

fn is_anywhere_source(src: &str) -> bool {
    let s = src.trim().to_ascii_lowercase();
    s.is_empty() || s == "anywhere" || s == "any" || s == "0.0.0.0/0" || s == "::/0" || s == "*"
}

pub fn compute_socket_exposure(
    socket: &ListeningSocketItem,
    firewall_enabled: bool,
    default_incoming: &str,
    rules: &[FirewallRuleItem],
) -> String {
    if !socket.is_public {
        return "loopback".to_string();
    }
    if !firewall_enabled {
        return "exposed".to_string();
    }

    let matching: Vec<&FirewallRuleItem> = rules
        .iter()
        .filter(|r| rule_port_matches(&r.to_port, &r.protocol, socket.port, &socket.protocol))
        .collect();

    let mut has_anywhere_allow = false;
    let mut has_restricted_allow = false;
    let mut has_deny = false;

    for r in &matching {
        let act = r.action.to_ascii_uppercase();
        if act == "ALLOW" || act == "LIMIT" {
            if is_anywhere_source(&r.from_source) {
                has_anywhere_allow = true;
            } else {
                has_restricted_allow = true;
            }
        } else if act == "DENY" || act == "REJECT" {
            has_deny = true;
        }
    }

    if has_anywhere_allow {
        "exposed".to_string()
    } else if has_restricted_allow {
        "restricted".to_string()
    } else if has_deny
        || default_incoming.eq_ignore_ascii_case("deny")
        || default_incoming.eq_ignore_ascii_case("reject")
    {
        "blocked".to_string()
    } else {
        "exposed".to_string()
    }
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
                } else if let Some(v) = trimmed.strip_prefix("CLIENT_IP:") {
                    snap.detected_client_ip = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("SSH_PORT:") {
                    if let Ok(p) = v.trim().parse::<u16>() {
                        if p > 0 {
                            snap.ssh_port = p;
                        }
                    }
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
        sock.exposure =
            compute_socket_exposure(sock, snap.enabled, &snap.default_incoming, &snap.rules);
    }

    snap
}

pub async fn get_firewall_snapshot(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    let script = r#"
engine="none"
enabled=0
def_in="allow"
def_out="allow"
if command -v ufw >/dev/null 2>&1; then
  engine="ufw"
  ufw_verbose=$(ufw status verbose 2>/dev/null || true)
  if printf '%s\n' "$ufw_verbose" | grep -qi '^Status:[[:space:]]*active'; then
    enabled=1
  fi
  if printf '%s\n' "$ufw_verbose" | grep -qi 'deny (incoming)'; then
    def_in="deny"
  elif printf '%s\n' "$ufw_verbose" | grep -qi 'reject (incoming)'; then
    def_in="reject"
  elif printf '%s\n' "$ufw_verbose" | grep -qi 'allow (incoming)'; then
    def_in="allow"
  elif [ -f /etc/default/ufw ]; then
    if grep -qi '^DEFAULT_INPUT_POLICY="DROP"' /etc/default/ufw 2>/dev/null; then
      def_in="deny"
    elif grep -qi '^DEFAULT_INPUT_POLICY="REJECT"' /etc/default/ufw 2>/dev/null; then
      def_in="reject"
    fi
  fi
  if printf '%s\n' "$ufw_verbose" | grep -qi 'deny (outgoing)'; then
    def_out="deny"
  fi
elif command -v firewall-cmd >/dev/null 2>&1 && firewall-cmd --state >/dev/null 2>&1; then
  engine="firewalld"
  enabled=1
  def_in="deny"
elif command -v iptables >/dev/null 2>&1; then
  engine="iptables"
  ipt_pol=$(iptables -S INPUT 2>/dev/null | head -n 1 || true)
  if printf '%s\n' "$ipt_pol" | grep -q -- '-P INPUT DROP'; then
    enabled=1
    def_in="deny"
  elif printf '%s\n' "$ipt_pol" | grep -q -- '-P INPUT REJECT'; then
    enabled=1
    def_in="reject"
  else
    rcount=$(iptables -S INPUT 2>/dev/null | grep -c '^-A INPUT' || true)
    if [ "${rcount:-0}" -gt 0 ] 2>/dev/null; then
      enabled=1
    fi
  fi
fi
ssh_p=$(grep -iE '^[[:space:]]*Port[[:space:]]+[0-9]+' /etc/ssh/sshd_config 2>/dev/null | head -n 1 | awk '{print $2}')
[ -z "$ssh_p" ] && ssh_p=22
cip=$(echo "${SSH_CLIENT%% *}")
[ -z "$cip" ] && cip=$(echo "${SSH_CONNECTION%% *}")
if [ -z "$cip" ] && command -v ss >/dev/null 2>&1; then
  cip=$(ss -tnH state established "( sport = :$ssh_p or sport = :22 )" 2>/dev/null | awk 'NR==1 {print $4}' | sed 's/^\[//;s/\]:[0-9]*$//;s/:[0-9]*$//')
fi
echo "---META---"
echo "ENGINE:$engine"
echo "ENABLED:$enabled"
echo "DEFAULT_INCOMING:$def_in"
echo "DEFAULT_OUTGOING:$def_out"
echo "CLIENT_IP:$cip"
echo "SSH_PORT:$ssh_p"
echo "---UFW---"
if [ "$engine" = "ufw" ]; then
  ufw status numbered 2>/dev/null || true
fi
echo "---IPTABLES---"
if [ "$engine" != "ufw" ] && command -v iptables >/dev/null 2>&1; then
  iptables -S INPUT 2>/dev/null || true
fi
echo "---SOCKETS---"
if command -v ss >/dev/null 2>&1; then
  ss -tulpnH 2>/dev/null || ss -tulpn 2>/dev/null || true
elif command -v netstat >/dev/null 2>&1; then
  netstat -tulpn 2>/dev/null || true
fi
"#;
    let raw = run_privileged_script(host_id, pool, sudo_password, script).await?;
    Ok(parse_firewall_snapshot(&raw))
}

pub fn build_add_firewall_rule_script(
    action: &str,
    port_spec: &str,
    protocol: &str,
    source_ip: &str,
    comment: &str,
) -> Result<String, String> {
    let act_lower = match action.trim().to_ascii_lowercase().as_str() {
        "allow" => "allow",
        "limit" => "limit",
        "deny" => "deny",
        "reject" => "reject",
        other => return Err(format!("Unsupported firewall action: {other}")),
    };

    let proto_lower = match protocol.trim().to_ascii_lowercase().as_str() {
        "tcp" => "tcp",
        "udp" => "udp",
        _ => "any",
    };

    let clean_port = {
        let p = port_spec
            .trim()
            .trim_end_matches("/tcp")
            .trim_end_matches("/udp")
            .replace('-', ":");
        if p.is_empty() || p.eq_ignore_ascii_case("any") || p.eq_ignore_ascii_case("anywhere") {
            "any".to_string()
        } else {
            if !p
                .chars()
                .all(|c| c.is_ascii_digit() || c == ',' || c == ':')
            {
                return Err(format!("Invalid port specification: {port_spec}"));
            }
            p
        }
    };

    let clean_source = {
        let s = source_ip.trim();
        if is_anywhere_source(s) {
            "any".to_string()
        } else {
            if !s
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':' || c == '/')
            {
                return Err(format!("Invalid source IP / CIDR: {source_ip}"));
            }
            s.to_string()
        }
    };

    let clean_comment = comment.trim().replace('\n', " ").replace('\r', " ");
    let comment_ufw_arg = if clean_comment.is_empty() {
        String::new()
    } else {
        format!(" comment {}", shell_single_quote(&clean_comment))
    };

    // Build UFW command(s)
    // Note: UFW requires explicit `proto tcp` or `proto udp` when port_spec has `,` or `:`.
    let ufw_protos: Vec<&str> =
        if clean_port != "any" && (clean_port.contains(',') || clean_port.contains(':')) && proto_lower == "any" {
            vec!["tcp", "udp"]
        } else {
            vec![proto_lower]
        };

    let mut ufw_cmds: Vec<String> = Vec::new();
    for pr in ufw_protos {
        let base_cmd = if clean_port == "any" {
            if pr == "any" {
                format!("ufw {act_lower} from {clean_source} to any")
            } else {
                format!("ufw {act_lower} from {clean_source} to any proto {pr}")
            }
        } else if pr == "any" {
            format!("ufw {act_lower} from {clean_source} to any port {clean_port}")
        } else {
            format!("ufw {act_lower} from {clean_source} to any port {clean_port} proto {pr}")
        };
        if comment_ufw_arg.is_empty() {
            ufw_cmds.push(base_cmd);
        } else {
            ufw_cmds.push(format!("{base_cmd}{comment_ufw_arg} || {base_cmd}"));
        }
    }
    let ufw_block = ufw_cmds.join("\n  ");

    // Build iptables fallback command
    let ipt_target = match act_lower {
        "allow" | "limit" => "ACCEPT",
        "deny" => "DROP",
        "reject" => "REJECT",
        _ => "ACCEPT",
    };
    let ipt_src = if clean_source == "any" {
        String::new()
    } else {
        format!(" -s {clean_source}")
    };
    let ipt_proto = if proto_lower == "any" {
        if clean_port != "any" {
            " -p tcp".to_string()
        } else {
            String::new()
        }
    } else {
        format!(" -p {proto_lower}")
    };
    let ipt_port = if clean_port == "any" {
        String::new()
    } else if clean_port.contains(',') {
        format!(" -m multiport --dports {clean_port}")
    } else {
        format!(" --dport {clean_port}")
    };
    let ipt_comment = if clean_comment.is_empty() {
        String::new()
    } else {
        format!(" -m comment --comment {}", shell_single_quote(&clean_comment))
    };

    Ok(format!(
        r#"if command -v ufw >/dev/null 2>&1; then
  {ufw_block}
elif command -v iptables >/dev/null 2>&1; then
  iptables -A INPUT{ipt_src}{ipt_proto}{ipt_port}{ipt_comment} -j {ipt_target}
else
  echo "Neither ufw nor iptables found on this server."
  exit 1
fi"#
    ))
}

pub fn build_delete_firewall_rule_script(rule_index: u32, raw_spec: &str) -> String {
    let safe_spec: String = raw_spec
        .chars()
        .filter(|c| *c != '\n' && *c != '\r' && *c != ';' && *c != '`' && *c != '$')
        .collect();
    format!(
        r#"if command -v ufw >/dev/null 2>&1; then
  yes | ufw --force delete {rule_index}
elif command -v iptables >/dev/null 2>&1; then
  if [ -n {quoted_spec} ]; then
    iptables -D INPUT {safe_spec} 2>/dev/null || iptables -D INPUT {rule_index}
  else
    iptables -D INPUT {rule_index}
  fi
else
  echo "Neither ufw nor iptables found on this server."
  exit 1
fi"#,
        quoted_spec = shell_single_quote(&safe_spec)
    )
}

pub fn build_set_firewall_state_script(
    enabled: bool,
    default_incoming: &str,
    ssh_port: u16,
) -> String {
    let safe_port = if ssh_port == 0 { 22 } else { ssh_port };
    let norm_incoming = match default_incoming.trim().to_ascii_lowercase().as_str() {
        "allow" => "allow",
        "reject" => "reject",
        _ => "deny",
    };
    let extra_ssh_22 = if safe_port != 22 {
        "\n  ufw allow 22/tcp >/dev/null 2>&1 || true"
    } else {
        ""
    };
    let ufw_toggle_cmd = if enabled {
        "ufw --force enable"
    } else {
        "ufw disable"
    };
    let ipt_policy = if enabled && (norm_incoming == "deny" || norm_incoming == "reject") {
        "DROP"
    } else {
        "ACCEPT"
    };

    format!(
        r#"if command -v ufw >/dev/null 2>&1; then
  ufw allow {safe_port}/tcp >/dev/null 2>&1 || true{extra_ssh_22}
  ufw default {norm_incoming} incoming
  ufw default allow outgoing
  {ufw_toggle_cmd}
elif command -v iptables >/dev/null 2>&1; then
  iptables -C INPUT -i lo -j ACCEPT 2>/dev/null || iptables -I INPUT 1 -i lo -j ACCEPT
  iptables -C INPUT -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || iptables -I INPUT 2 -m conntrack --ctstate RELATED,ESTABLISHED -j ACCEPT
  iptables -C INPUT -p tcp --dport {safe_port} -j ACCEPT 2>/dev/null || iptables -I INPUT 3 -p tcp --dport {safe_port} -j ACCEPT
  iptables -P INPUT {ipt_policy}
else
  echo "Neither ufw nor iptables found on this server."
  exit 1
fi"#
    )
}

pub async fn add_firewall_rule(
    host_id: &str,
    pool: &SshPool,
    action: String,
    port_spec: String,
    protocol: String,
    source_ip: String,
    comment: String,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    let script =
        build_add_firewall_rule_script(&action, &port_spec, &protocol, &source_ip, &comment)?;
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_firewall_snapshot(host_id, pool, sudo_password).await
}

pub async fn delete_firewall_rule(
    host_id: &str,
    pool: &SshPool,
    rule_index: u32,
    raw_spec: String,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    let script = build_delete_firewall_rule_script(rule_index, &raw_spec);
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_firewall_snapshot(host_id, pool, sudo_password).await
}

pub async fn set_firewall_state(
    host_id: &str,
    pool: &SshPool,
    enabled: bool,
    default_incoming: String,
    ssh_port: u16,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    let script = build_set_firewall_state_script(enabled, &default_incoming, ssh_port);
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_firewall_snapshot(host_id, pool, sudo_password).await
}

pub fn shell_single_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\"'\"'"))
}

pub fn resolve_sudo_password(host_id: &str, sudo_password: Option<String>) -> Option<String> {
    if let Some(pw) = sudo_password {
        if !pw.is_empty() {
            return Some(pw);
        }
    }
    let hosts = crate::host_store::load_all_hosts();
    if let Some(host) = hosts.iter().find(|h| h.id == host_id) {
        let store = crate::resources::load_resources();
        let creds = crate::resources::resolve_host_credentials_from(&store, host, None);
        if let Some(sec) = creds.secret {
            if !sec.is_empty() {
                return Some(sec);
            }
        }
    }
    if let Ok(Some(sec)) = crate::vault::get_host_secret(host_id) {
        if !sec.is_empty() {
            return Some(sec);
        }
    }
    None
}

pub fn build_privileged_command(script: &str, sudo_password: Option<&str>) -> String {
    let pw_quoted = shell_single_quote(sudo_password.unwrap_or(""));
    format!(
        r#"LC_ALL=C
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH"
if [ "$(id -u)" = "0" ]; then
  sh -s << 'SSHBOX_EOF_9912' 2>&1
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH"
{script}
:
SSHBOX_EOF_9912
  rc=$?
elif sudo -n true 2>/dev/null; then
  sudo -n sh -s << 'SSHBOX_EOF_9912' 2>&1
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH"
{script}
:
SSHBOX_EOF_9912
  rc=$?
elif [ -n {pw_quoted} ]; then
  {{ printf '%s\n' {pw_quoted}; cat << 'SSHBOX_EOF_9912'
export PATH="/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin:$PATH"
{script}
:
SSHBOX_EOF_9912
}} | sudo -S -p '' sh -s 2>&1
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

pub async fn run_privileged_script(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
    script: &str,
) -> Result<String, String> {
    let resolved_pw = resolve_sudo_password(host_id, sudo_password);
    let wrapped_cmd = build_privileged_command(script, resolved_pw.as_deref());
    let raw = crate::telemetry::run_host_or_local_cmd(host_id, pool, &wrapped_cmd).await?;

    if raw.contains("SSHBOX_ERR_SUDO_PASSWORD_REQUIRED") {
        return Err(
            "Sudo password required. Unlock sudo in the top bar to manage system configuration."
                .to_string(),
        );
    }

    if let Some(idx) = raw.rfind("SSHBOX_EXIT_CODE:") {
        let body = raw[..idx].trim();
        let lower = body.to_lowercase();
        if lower.contains("sorry, try again")
            || lower.contains("incorrect password")
            || lower.contains("a password is required")
        {
            return Err("Sudo authentication failed: incorrect sudo password.".to_string());
        }
        let msg = if body.is_empty() {
            "Privileged command failed on remote host.".to_string()
        } else {
            body.to_string()
        };
        return Err(msg);
    }

    Ok(raw)
}

pub fn parse_privilege_probe(raw: &str) -> VpsPrivilegeInfo {
    let mut info = VpsPrivilegeInfo {
        user: "root".to_string(),
        uid: 0,
        is_root: false,
        passwordless_sudo: false,
        sudo_unlocked: false,
        os_pretty_name: "Linux".to_string(),
        detected_client_ip: String::new(),
    };

    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("USER:") {
            info.user = v.trim().to_string();
        } else if let Some(v) = trimmed.strip_prefix("UID:") {
            info.uid = v.trim().parse::<u32>().unwrap_or(1000);
        } else if let Some(v) = trimmed.strip_prefix("NOPASSWD:") {
            info.passwordless_sudo = v.trim() == "1";
        } else if let Some(v) = trimmed.strip_prefix("SUDO_OK:") {
            info.sudo_unlocked = v.trim() == "1";
        } else if let Some(v) = trimmed.strip_prefix("OS:") {
            let os = v.trim().trim_matches('"');
            if !os.is_empty() {
                info.os_pretty_name = os.to_string();
            }
        } else if let Some(v) = trimmed.strip_prefix("CLIENT_IP:") {
            info.detected_client_ip = v.trim().to_string();
        }
    }

    info.is_root = info.uid == 0;
    if info.is_root || info.passwordless_sudo {
        info.sudo_unlocked = true;
    }
    info
}

pub async fn probe_privilege(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<VpsPrivilegeInfo, String> {
    let resolved_pw = resolve_sudo_password(host_id, sudo_password);
    let pw_quoted = shell_single_quote(resolved_pw.as_deref().unwrap_or(""));
    let cmd = format!(
        r#"LC_ALL=C sh -c '
uid=$(id -u 2>/dev/null || echo 1000)
usr=$(id -un 2>/dev/null || whoami 2>/dev/null || echo user)
echo "USER:$usr"
echo "UID:$uid"
if [ "$uid" = "0" ]; then
  echo "NOPASSWD:1"
  echo "SUDO_OK:1"
elif sudo -n true 2>/dev/null; then
  echo "NOPASSWD:1"
  echo "SUDO_OK:1"
else
  echo "NOPASSWD:0"
  if [ -n {pw_quoted} ] && printf "%s\n" {pw_quoted} | sudo -S -p "" true 2>/dev/null; then
    echo "SUDO_OK:1"
  else
    echo "SUDO_OK:0"
  fi
fi
os_name=""
if [ -f /etc/os-release ]; then
  os_name=$(. /etc/os-release 2>/dev/null && echo "$PRETTY_NAME")
fi
if [ -z "$os_name" ]; then
  os_name=$(uname -sr 2>/dev/null || echo "Linux")
fi
echo "OS:$os_name"
cip=$(echo "${{SSH_CLIENT%% *}}")
if [ -z "$cip" ]; then
  cip=$(echo "${{SSH_CONNECTION%% *}}")
fi
echo "CLIENT_IP:$cip"
'"#
    );

    let raw = crate::telemetry::run_host_or_local_cmd(host_id, pool, &cmd).await?;
    Ok(parse_privilege_probe(&raw))
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
        "no" | "forced-commands-only" => "no".to_string(),
        "prohibit-password" | "without-password" => "prohibit-password".to_string(),
        _ => "prohibit-password".to_string(),
    }
}

fn normalize_address_family(val: &str) -> String {
    match val.trim().to_ascii_lowercase().as_str() {
        "inet" => "inet".to_string(),
        "inet6" => "inet6".to_string(),
        _ => "any".to_string(),
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

pub fn compute_ssh_security_score(cfg: &SshdConfigSnapshot) -> u32 {
    let mut score = 0u32;

    if !cfg.permit_empty_passwords {
        score += 15;
    }
    if cfg.pubkey_authentication {
        score += 15;
    }
    if !cfg.password_authentication {
        score += 25;
    }
    match cfg.permit_root_login.as_str() {
        "no" => score += 20,
        "prohibit-password" => score += 12,
        _ => {}
    }
    if cfg.port != 22 {
        score += 10;
    }
    if cfg.max_auth_tries <= 4 {
        score += 5;
    }
    if !cfg.x11_forwarding {
        score += 5;
    }
    if (cfg.client_alive_interval > 0 && cfg.client_alive_interval <= 600)
        || !cfg.allow_users.is_empty()
        || cfg.login_grace_time <= 60
    {
        score += 5;
    }

    score.min(100)
}

pub fn parse_sshd_config(
    raw: &str,
    authorized_keys_count: u32,
    rollback_pending: bool,
) -> SshdConfigSnapshot {
    let mut snap = SshdConfigSnapshot {
        authorized_keys_count,
        raw_config: raw.to_string(),
        rollback_pending,
        ..Default::default()
    };

    let mut allow_users_collected: Vec<String> = Vec::new();

    for line in raw.lines() {
        let Some((key, val)) = split_sshd_directive(line) else {
            continue;
        };
        let key_lower = key.to_ascii_lowercase();
        if key_lower == "match" {
            // Stop parsing global directives once a Match block starts.
            break;
        }
        match key_lower.as_str() {
            "port" => {
                if let Ok(p) = val.split_whitespace().next().unwrap_or("22").parse::<u16>() {
                    if p > 0 {
                        snap.port = p;
                    }
                }
            }
            "permitrootlogin" => {
                snap.permit_root_login = normalize_permit_root_login(val);
            }
            "passwordauthentication" => {
                snap.password_authentication = parse_yes_no(val, snap.password_authentication);
            }
            "pubkeyauthentication" => {
                snap.pubkey_authentication = parse_yes_no(val, snap.pubkey_authentication);
            }
            "permitemptypasswords" => {
                snap.permit_empty_passwords = parse_yes_no(val, snap.permit_empty_passwords);
            }
            "maxauthtries" => {
                if let Ok(n) = val.parse::<u32>() {
                    snap.max_auth_tries = n.max(1);
                }
            }
            "logingracetime" => {
                snap.login_grace_time = parse_time_seconds(val, snap.login_grace_time);
            }
            "clientaliveinterval" => {
                snap.client_alive_interval = parse_time_seconds(val, snap.client_alive_interval);
            }
            "clientalivecountmax" => {
                if let Ok(n) = val.parse::<u32>() {
                    snap.client_alive_count_max = n;
                }
            }
            "allowusers" => {
                for u in val.split_whitespace() {
                    let clean = u.trim();
                    if !clean.is_empty() && !allow_users_collected.iter().any(|x| x == clean) {
                        allow_users_collected.push(clean.to_string());
                    }
                }
            }
            "x11forwarding" => {
                snap.x11_forwarding = parse_yes_no(val, snap.x11_forwarding);
            }
            "allowtcpforwarding" => {
                snap.allow_tcp_forwarding = parse_yes_no(val, snap.allow_tcp_forwarding);
            }
            "allowagentforwarding" => {
                snap.allow_agent_forwarding = parse_yes_no(val, snap.allow_agent_forwarding);
            }
            "addressfamily" => {
                snap.address_family = normalize_address_family(val);
            }
            _ => {}
        }
    }

    snap.allow_users = allow_users_collected;
    snap.security_score = compute_ssh_security_score(&snap);
    snap
}

pub fn render_sshd_config(base_raw: &str, update: &SshdConfigUpdate) -> String {
    let yes_no = |b: bool| if b { "yes" } else { "no" };
    let clean_users: Vec<String> = update
        .allow_users
        .iter()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
        .collect();

    let managed_entries: Vec<(&str, Option<String>)> = vec![
        ("port", Some(format!("Port {}", update.port.max(1)))),
        (
            "addressfamily",
            Some(format!(
                "AddressFamily {}",
                normalize_address_family(&update.address_family)
            )),
        ),
        (
            "permitrootlogin",
            Some(format!(
                "PermitRootLogin {}",
                normalize_permit_root_login(&update.permit_root_login)
            )),
        ),
        (
            "pubkeyauthentication",
            Some(format!(
                "PubkeyAuthentication {}",
                yes_no(update.pubkey_authentication)
            )),
        ),
        (
            "passwordauthentication",
            Some(format!(
                "PasswordAuthentication {}",
                yes_no(update.password_authentication)
            )),
        ),
        (
            "permitemptypasswords",
            Some(format!(
                "PermitEmptyPasswords {}",
                yes_no(update.permit_empty_passwords)
            )),
        ),
        (
            "maxauthtries",
            Some(format!("MaxAuthTries {}", update.max_auth_tries.max(1))),
        ),
        (
            "logingracetime",
            Some(format!("LoginGraceTime {}", update.login_grace_time)),
        ),
        (
            "clientaliveinterval",
            Some(format!(
                "ClientAliveInterval {}",
                update.client_alive_interval
            )),
        ),
        (
            "clientalivecountmax",
            Some(format!(
                "ClientAliveCountMax {}",
                update.client_alive_count_max
            )),
        ),
        (
            "allowusers",
            if clean_users.is_empty() {
                None
            } else {
                Some(format!("AllowUsers {}", clean_users.join(" ")))
            },
        ),
        (
            "x11forwarding",
            Some(format!("X11Forwarding {}", yes_no(update.x11_forwarding))),
        ),
        (
            "allowtcpforwarding",
            Some(format!(
                "AllowTcpForwarding {}",
                yes_no(update.allow_tcp_forwarding)
            )),
        ),
        (
            "allowagentforwarding",
            Some(format!(
                "AllowAgentForwarding {}",
                yes_no(update.allow_agent_forwarding)
            )),
        ),
    ];

    let mut emitted_keys = std::collections::HashSet::new();
    let mut pre_match_lines: Vec<String> = Vec::new();
    let mut match_and_after_lines: Vec<String> = Vec::new();
    let mut in_match_block = false;

    for line in base_raw.lines() {
        if !in_match_block {
            if let Some((key, _)) = split_sshd_directive(line) {
                let key_lower = key.to_ascii_lowercase();
                if key_lower == "match" {
                    in_match_block = true;
                    match_and_after_lines.push(line.to_string());
                    continue;
                }
                if let Some((_, replacement)) =
                    managed_entries.iter().find(|(k, _)| *k == key_lower)
                {
                    if emitted_keys.insert(key_lower) {
                        if let Some(rep_line) = replacement {
                            pre_match_lines.push(rep_line.clone());
                        }
                    }
                    continue;
                }
            }
            pre_match_lines.push(line.to_string());
        } else {
            match_and_after_lines.push(line.to_string());
        }
    }

    let mut missing_lines: Vec<String> = Vec::new();
    for (key, replacement) in &managed_entries {
        if !emitted_keys.contains(*key) {
            if let Some(rep_line) = replacement {
                missing_lines.push(rep_line.clone());
            }
        }
    }

    if !missing_lines.is_empty() {
        if !pre_match_lines.is_empty()
            && !pre_match_lines.last().unwrap().trim().is_empty()
        {
            pre_match_lines.push(String::new());
        }
        pre_match_lines.push("# --- Managed by ssh-box Hardening Studio ---".to_string());
        pre_match_lines.extend(missing_lines);
    }

    if !match_and_after_lines.is_empty() {
        if !pre_match_lines.last().map(|s| s.trim().is_empty()).unwrap_or(true) {
            pre_match_lines.push(String::new());
        }
        pre_match_lines.extend(match_and_after_lines);
    }

    let mut out = pre_match_lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn parse_sshd_probe_output(raw: &str) -> SshdConfigSnapshot {
    let mut ak_count = 0u32;
    let mut rollback_pending = false;
    let mut config_lines: Vec<&str> = Vec::new();
    let mut in_config = false;

    for line in raw.lines() {
        if in_config {
            config_lines.push(line);
            continue;
        }
        let trimmed = line.trim();
        if let Some(v) = trimmed.strip_prefix("AK_COUNT:") {
            ak_count = v.trim().parse::<u32>().unwrap_or(0);
        } else if let Some(v) = trimmed.strip_prefix("ROLLBACK_PENDING:") {
            rollback_pending = v.trim() == "1";
        } else if trimmed == "---SSHD_CONFIG_START---" {
            in_config = true;
        }
    }

    let raw_cfg = config_lines.join("\n");
    parse_sshd_config(&raw_cfg, ak_count, rollback_pending)
}

pub async fn get_sshd_config(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<SshdConfigSnapshot, String> {
    let script = r#"
target_user="${SUDO_USER:-$(id -un)}"
user_home=$(getent passwd "$target_user" 2>/dev/null | cut -d: -f6)
[ -z "$user_home" ] && user_home="$HOME"
ak_count=0
for ak in "$user_home/.ssh/authorized_keys" "/root/.ssh/authorized_keys"; do
  if [ -f "$ak" ]; then
    c=$(grep -vE '^[[:space:]]*(#|$)' "$ak" 2>/dev/null | wc -l | tr -d ' ')
    if [ "${c:-0}" -gt "$ak_count" ] 2>/dev/null; then
      ak_count="$c"
    fi
  fi
done
echo "AK_COUNT:$ak_count"
rb_pending=0
if [ -f /tmp/sshbox-sshd-rollback.pid ]; then
  pid=$(cat /tmp/sshbox-sshd-rollback.pid 2>/dev/null | tr -d '[:space:]')
  if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
    rb_pending=1
  else
    rm -f /tmp/sshbox-sshd-rollback.pid
  fi
fi
echo "ROLLBACK_PENDING:$rb_pending"
echo "---SSHD_CONFIG_START---"
cat /etc/ssh/sshd_config 2>/dev/null || true
"#;
    let raw = run_privileged_script(host_id, pool, sudo_password, script).await?;
    Ok(parse_sshd_probe_output(&raw))
}

pub async fn apply_sshd_config(
    host_id: &str,
    pool: &SshPool,
    update: SshdConfigUpdate,
    sudo_password: Option<String>,
    with_auto_rollback: bool,
) -> Result<SshdConfigSnapshot, String> {
    let current = get_sshd_config(host_id, pool, sudo_password.clone()).await?;
    let rendered = render_sshd_config(&current.raw_config, &update);
    let port = update.port.max(1);
    let ufw_step = if port != 22 {
        format!(
            r#"if command -v ufw >/dev/null 2>&1 && ufw status 2>/dev/null | grep -qi "Status: active"; then
  ufw allow {port}/tcp >/dev/null 2>&1 || true
fi"#
        )
    } else {
        String::new()
    };

    let rollback_step = if with_auto_rollback {
        r#"nohup sh -c 'sleep 30 && cp /etc/ssh/sshd_config.sshbox.bak /etc/ssh/sshd_config && (systemctl reload sshd 2>/dev/null || systemctl reload ssh 2>/dev/null || service sshd reload 2>/dev/null || service ssh reload 2>/dev/null)' >/dev/null 2>&1 </dev/null &
echo $! > /tmp/sshbox-sshd-rollback.pid"#
    } else {
        "rm -f /tmp/sshbox-sshd-rollback.pid"
    };

    let script = format!(
        r#"
if [ -f /tmp/sshbox-sshd-rollback.pid ]; then
  old_pid=$(cat /tmp/sshbox-sshd-rollback.pid 2>/dev/null | tr -d '[:space:]')
  if [ -n "$old_pid" ]; then
    pkill -P "$old_pid" 2>/dev/null || true
    kill -9 "$old_pid" 2>/dev/null || true
  fi
  rm -f /tmp/sshbox-sshd-rollback.pid
fi

if [ -f /etc/ssh/sshd_config ]; then
  cp -p /etc/ssh/sshd_config /etc/ssh/sshd_config.sshbox.bak
fi

cat << 'SSHBOX_NEW_SSHD_EOF' > /tmp/sshd_config.sshbox.new
{rendered}SSHBOX_NEW_SSHD_EOF

mkdir -p /run/sshd /var/run/sshd 2>/dev/null || true
if ! (sshd -t -f /tmp/sshd_config.sshbox.new 2>&1 || /usr/sbin/sshd -t -f /tmp/sshd_config.sshbox.new 2>&1); then
  rm -f /tmp/sshd_config.sshbox.new
  exit 1
fi

{ufw_step}

mv /tmp/sshd_config.sshbox.new /etc/ssh/sshd_config
chmod 600 /etc/ssh/sshd_config 2>/dev/null || true

if ! (systemctl reload sshd 2>/dev/null || systemctl reload ssh 2>/dev/null || service sshd reload 2>/dev/null || service ssh reload 2>/dev/null); then
  cp /etc/ssh/sshd_config.sshbox.bak /etc/ssh/sshd_config 2>/dev/null || true
  echo "Failed to reload sshd daemon; restored backup."
  exit 1
fi

{rollback_step}
"#
    );

    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_sshd_config(host_id, pool, sudo_password).await
}

pub async fn confirm_sshd_config(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<SshdConfigSnapshot, String> {
    let script = r#"
if [ -f /tmp/sshbox-sshd-rollback.pid ]; then
  pid=$(cat /tmp/sshbox-sshd-rollback.pid 2>/dev/null | tr -d '[:space:]')
  if [ -n "$pid" ]; then
    pkill -P "$pid" 2>/dev/null || true
    kill -9 "$pid" 2>/dev/null || true
  fi
  rm -f /tmp/sshbox-sshd-rollback.pid
fi
"#;
    run_privileged_script(host_id, pool, sudo_password.clone(), script).await?;
    get_sshd_config(host_id, pool, sudo_password).await
}

pub async fn revert_sshd_config(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<SshdConfigSnapshot, String> {
    let script = r#"
if [ -f /tmp/sshbox-sshd-rollback.pid ]; then
  pid=$(cat /tmp/sshbox-sshd-rollback.pid 2>/dev/null | tr -d '[:space:]')
  if [ -n "$pid" ]; then
    pkill -P "$pid" 2>/dev/null || true
    kill -9 "$pid" 2>/dev/null || true
  fi
  rm -f /tmp/sshbox-sshd-rollback.pid
fi
if [ -f /etc/ssh/sshd_config.sshbox.bak ]; then
  cp -p /etc/ssh/sshd_config.sshbox.bak /etc/ssh/sshd_config
  systemctl reload sshd 2>/dev/null || systemctl reload ssh 2>/dev/null || service sshd reload 2>/dev/null || service ssh reload 2>/dev/null || true
else
  echo "No backup /etc/ssh/sshd_config.sshbox.bak found on server."
  exit 1
fi
"#;
    run_privileged_script(host_id, pool, sudo_password.clone(), script).await?;
    get_sshd_config(host_id, pool, sudo_password).await
}

fn parse_auth_log_line(line: &str) -> Option<SshAuthAuditEvent> {
    let trimmed = line.trim();
    if trimmed.is_empty() {
        return None;
    }

    let (timestamp, msg) = if let Some(sshd_idx) = trimmed.find("sshd") {
        let before = trimmed[..sshd_idx].trim();
        let ts = match before.rsplit_once(char::is_whitespace) {
            Some((prefix, _hostname)) if !prefix.trim().is_empty() => prefix.trim().to_string(),
            _ => before.to_string(),
        };
        let after_sshd = &trimmed[sshd_idx + 4..];
        let body = match after_sshd.find(':') {
            Some(colon_idx) => after_sshd[colon_idx + 1..].trim(),
            None => after_sshd.trim(),
        };
        (ts, body)
    } else {
        (String::new(), trimmed)
    };

    let extract_from_and_port = |tokens: &[&str], start_idx: usize| -> Option<(String, u16)> {
        let from_rel = tokens[start_idx..].iter().position(|&t| t == "from")?;
        let from_idx = start_idx + from_rel;
        let ip = (*tokens.get(from_idx + 1)?).trim().to_string();
        if ip.is_empty() {
            return None;
        }
        let port = tokens[from_idx + 2..]
            .iter()
            .position(|&t| t == "port")
            .and_then(|p_rel| tokens.get(from_idx + 2 + p_rel + 1))
            .and_then(|p_str| p_str.parse::<u16>().ok())
            .unwrap_or(22);
        Some((ip, port))
    };

    if let Some(rest) = msg.strip_prefix("Accepted ") {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() < 5 || tokens.get(1) != Some(&"for") {
            return None;
        }
        let raw_method = tokens[0].to_ascii_lowercase();
        let method = if raw_method == "publickey" {
            "publickey".to_string()
        } else if raw_method == "password" || raw_method.starts_with("keyboard-interactive") {
            "password".to_string()
        } else {
            "unknown".to_string()
        };
        let username = tokens[2].to_string();
        let (ip, port) = extract_from_and_port(&tokens, 3)?;
        return Some(SshAuthAuditEvent {
            timestamp,
            event_type: "accepted".to_string(),
            username,
            ip,
            method,
            port,
            raw_line: trimmed.to_string(),
        });
    }

    if let Some(rest) = msg.strip_prefix("Failed ") {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.len() < 5 || tokens.get(1) != Some(&"for") {
            return None;
        }
        let raw_method = tokens[0].to_ascii_lowercase();
        let method = if raw_method == "publickey" {
            "publickey".to_string()
        } else if raw_method == "password" || raw_method.starts_with("keyboard-interactive") {
            "password".to_string()
        } else {
            "unknown".to_string()
        };
        let (username, search_start) =
            if tokens.get(2) == Some(&"invalid") && tokens.get(3) == Some(&"user") {
                let u = (*tokens.get(4)?).to_string();
                (u, 5)
            } else {
                (tokens[2].to_string(), 3)
            };
        let (ip, port) = extract_from_and_port(&tokens, search_start)?;
        return Some(SshAuthAuditEvent {
            timestamp,
            event_type: "failed".to_string(),
            username,
            ip,
            method,
            port,
            raw_line: trimmed.to_string(),
        });
    }

    let inv_rest = msg
        .strip_prefix("Invalid user ")
        .or_else(|| msg.strip_prefix("invalid user "));
    if let Some(rest) = inv_rest {
        let tokens: Vec<&str> = rest.split_whitespace().collect();
        if tokens.is_empty() {
            return None;
        }
        let (username, search_start) = if tokens[0] == "from" {
            ("unknown".to_string(), 0)
        } else {
            (tokens[0].to_string(), 1)
        };
        let (ip, port) = extract_from_and_port(&tokens, search_start)?;
        return Some(SshAuthAuditEvent {
            timestamp,
            event_type: "invalid_user".to_string(),
            username,
            ip,
            method: "invalid_user".to_string(),
            port,
            raw_line: trimmed.to_string(),
        });
    }

    None
}

pub fn parse_fail2ban_and_auth_snapshot(raw: &str) -> Fail2banSnapshot {
    let mut snap = Fail2banSnapshot::default();
    let mut section = "";
    let mut current_jail: Option<Fail2banJailItem> = None;

    for line in raw.lines() {
        let trimmed = line.trim();
        match trimmed {
            "---F2B_META---" => {
                if let Some(j) = current_jail.take() {
                    snap.jails.push(j);
                }
                section = "meta";
                continue;
            }
            "---F2B_JAIL---" => {
                if let Some(j) = current_jail.take() {
                    snap.jails.push(j);
                }
                current_jail = Some(Fail2banJailItem::default());
                section = "jail";
                continue;
            }
            "---AUTH_LOG---" => {
                if let Some(j) = current_jail.take() {
                    snap.jails.push(j);
                }
                section = "auth";
                continue;
            }
            _ => {}
        }

        match section {
            "meta" => {
                if let Some(v) = trimmed.strip_prefix("INSTALLED:") {
                    snap.installed = parse_yes_no(v, false);
                } else if let Some(v) = trimmed.strip_prefix("RUNNING:") {
                    snap.running = parse_yes_no(v, false);
                } else if let Some(v) = trimmed.strip_prefix("CLIENT_IP:") {
                    snap.detected_client_ip = v.trim().to_string();
                } else if let Some(v) = trimmed.strip_prefix("IGNORE_IPS:") {
                    for tok in v.split(|c: char| c.is_ascii_whitespace() || c == ',') {
                        let clean = tok.trim();
                        if !clean.is_empty()
                            && clean != "127.0.0.1/8"
                            && clean != "127.0.0.1"
                            && clean != "::1"
                            && !snap.ignore_ips.iter().any(|x| x == clean)
                        {
                            snap.ignore_ips.push(clean.to_string());
                        }
                    }
                }
            }
            "jail" => {
                let jail = current_jail.get_or_insert_with(Fail2banJailItem::default);
                let clean = trimmed
                    .trim_start_matches(|c: char| c == '|' || c == '`' || c == '-' || c.is_whitespace());
                if let Some(v) = clean.strip_prefix("JAIL_NAME:") {
                    let name = v.trim();
                    if !name.is_empty() {
                        jail.name = name.to_string();
                    }
                } else if let Some(v) = clean.strip_prefix("Status for the jail:") {
                    let name = v.trim();
                    if !name.is_empty() {
                        jail.name = name.to_string();
                    }
                } else if let Some(v) = clean.strip_prefix("Currently failed:") {
                    jail.currently_failed = v.trim().parse::<u32>().unwrap_or(0);
                } else if let Some(v) = clean.strip_prefix("Total failed:") {
                    jail.total_failed = v.trim().parse::<u32>().unwrap_or(0);
                } else if let Some(v) = clean.strip_prefix("Currently banned:") {
                    jail.currently_banned = v.trim().parse::<u32>().unwrap_or(0);
                } else if let Some(v) = clean.strip_prefix("Total banned:") {
                    jail.total_banned = v.trim().parse::<u32>().unwrap_or(0);
                } else if let Some(v) = clean.strip_prefix("Banned IP list:") {
                    jail.banned_ips = v
                        .split_whitespace()
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                } else if let Some(v) = clean.strip_prefix("MAXRETRY:") {
                    if let Ok(n) = v.trim().parse::<u32>() {
                        jail.maxretry = n.max(1);
                    }
                } else if let Some(v) = clean.strip_prefix("FINDTIME:") {
                    jail.findtime_secs = parse_time_seconds(v, jail.findtime_secs);
                } else if let Some(v) = clean.strip_prefix("BANTIME:") {
                    if let Ok(n) = v.trim().parse::<i64>() {
                        jail.bantime_secs = n;
                    }
                }
            }
            "auth" => {
                if let Some(ev) = parse_auth_log_line(trimmed) {
                    snap.auth_events.push(ev);
                }
            }
            _ => {}
        }
    }

    if let Some(j) = current_jail.take() {
        snap.jails.push(j);
    }

    let mut banned_set: std::collections::HashSet<String> = std::collections::HashSet::new();
    for j in &snap.jails {
        for ip in &j.banned_ips {
            banned_set.insert(ip.clone());
        }
    }

    let mut attacker_map: std::collections::HashMap<String, (u32, String)> =
        std::collections::HashMap::new();
    for ev in &snap.auth_events {
        if ev.event_type == "failed" || ev.event_type == "invalid_user" {
            let entry = attacker_map
                .entry(ev.ip.clone())
                .or_insert_with(|| (0, ev.username.clone()));
            entry.0 += 1;
            entry.1 = ev.username.clone();
        }
    }

    let mut attackers: Vec<AttackerIpSummary> = attacker_map
        .into_iter()
        .map(|(ip, (failed_count, last_target_user))| {
            let is_currently_banned = banned_set.contains(&ip);
            AttackerIpSummary {
                ip,
                failed_count,
                last_target_user,
                is_currently_banned,
            }
        })
        .collect();

    attackers.sort_by(|a, b| {
        b.failed_count
            .cmp(&a.failed_count)
            .then_with(|| a.ip.cmp(&b.ip))
    });
    snap.top_attacker_ips = attackers;

    snap
}

pub async fn get_fail2ban_snapshot(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<Fail2banSnapshot, String> {
    let script = r#"
installed=0
running=0
if command -v fail2ban-client >/dev/null 2>&1 || command -v fail2ban-server >/dev/null 2>&1; then
  installed=1
  if fail2ban-client ping >/dev/null 2>&1 || systemctl is-active --quiet fail2ban 2>/dev/null; then
    running=1
  fi
fi
ssh_p=$(grep -iE '^[[:space:]]*Port[[:space:]]+[0-9]+' /etc/ssh/sshd_config 2>/dev/null | head -n 1 | awk '{print $2}')
[ -z "$ssh_p" ] && ssh_p=22
cip=$(echo "${SSH_CLIENT%% *}")
[ -z "$cip" ] && cip=$(echo "${SSH_CONNECTION%% *}")
if [ -z "$cip" ] && command -v ss >/dev/null 2>&1; then
  cip=$(ss -tnH state established "( sport = :$ssh_p or sport = :22 )" 2>/dev/null | awk 'NR==1 {print $4}' | sed 's/^\[//;s/\]:[0-9]*$//;s/:[0-9]*$//')
fi
ignore_ips=""
if [ -f /etc/fail2ban/jail.d/99-sshbox.local ]; then
  ignore_ips=$(grep -iE '^[[:space:]]*ignoreip[[:space:]]*=' /etc/fail2ban/jail.d/99-sshbox.local 2>/dev/null | tail -n 1 | cut -d= -f2-)
elif [ -f /etc/fail2ban/jail.local ]; then
  ignore_ips=$(grep -iE '^[[:space:]]*ignoreip[[:space:]]*=' /etc/fail2ban/jail.local 2>/dev/null | tail -n 1 | cut -d= -f2-)
fi
if [ -z "$ignore_ips" ] && [ "$running" = "1" ]; then
  ignore_ips=$(fail2ban-client get sshd ignoreip 2>/dev/null | tr '\n' ' ' | sed 's/[|`-]//g')
fi
echo "---F2B_META---"
echo "INSTALLED:$installed"
echo "RUNNING:$running"
echo "CLIENT_IP:$cip"
echo "IGNORE_IPS:$ignore_ips"
if [ "$running" = "1" ]; then
  jails=$(fail2ban-client status 2>/dev/null | sed -n 's/.*Jail list:[[:space:]]*//p' | tr ',' ' ')
  for j in $jails; do
    echo "---F2B_JAIL---"
    echo "JAIL_NAME:$j"
    fail2ban-client status "$j" 2>/dev/null || true
    mr=$(fail2ban-client get "$j" maxretry 2>/dev/null | tr -d '[:space:]')
    ft=$(fail2ban-client get "$j" findtime 2>/dev/null | tr -d '[:space:]')
    bt=$(fail2ban-client get "$j" bantime 2>/dev/null | tr -d '[:space:]')
    echo "MAXRETRY:${mr:-5}"
    echo "FINDTIME:${ft:-600}"
    echo "BANTIME:${bt:-3600}"
  done
fi
echo "---AUTH_LOG---"
if [ -s /var/log/auth.log ]; then
  grep -E 'sshd\[[0-9]+\]: (Accepted|Failed|Invalid user)' /var/log/auth.log 2>/dev/null | tail -n 150 || true
elif [ -s /var/log/secure ]; then
  grep -E 'sshd\[[0-9]+\]: (Accepted|Failed|Invalid user)' /var/log/secure 2>/dev/null | tail -n 150 || true
elif command -v journalctl >/dev/null 2>&1; then
  journalctl -u ssh -u sshd -n 200 --no-pager 2>/dev/null | grep -E 'sshd\[[0-9]+\]: (Accepted|Failed|Invalid user)' | tail -n 150 || true
fi
"#;
    let raw = run_privileged_script(host_id, pool, sudo_password, script).await?;
    Ok(parse_fail2ban_and_auth_snapshot(&raw))
}

pub fn build_fail2ban_ban_action_script(
    jail: &str,
    ip: &str,
    action: &str,
) -> Result<String, String> {
    let clean_jail = jail.trim();
    if clean_jail.is_empty()
        || !clean_jail
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_')
    {
        return Err(format!("Invalid Fail2ban jail name: {jail}"));
    }

    let clean_ip = ip.trim();
    if clean_ip.is_empty()
        || !clean_ip
            .chars()
            .all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':' || c == '/')
    {
        return Err(format!("Invalid IP address: {ip}"));
    }

    let subcmd = match action.trim().to_ascii_lowercase().as_str() {
        "ban" | "banip" => "banip",
        "unban" | "unbanip" => "unbanip",
        other => return Err(format!("Unsupported Fail2ban action: {other}")),
    };

    Ok(format!(
        "fail2ban-client set {} {} {}",
        shell_single_quote(clean_jail),
        subcmd,
        shell_single_quote(clean_ip)
    ))
}

pub fn build_update_fail2ban_jail_script(
    maxretry: u32,
    findtime_secs: u32,
    bantime_secs: i64,
    ignore_ips: &[String],
) -> Result<String, String> {
    let safe_maxretry = maxretry.clamp(1, 100);
    let safe_findtime = findtime_secs.max(10);
    let safe_bantime = if bantime_secs < 0 {
        -1
    } else {
        bantime_secs.max(60)
    };

    let mut extra_ips: Vec<String> = Vec::new();
    for raw_ip in ignore_ips {
        for tok in raw_ip.split(|c: char| c.is_ascii_whitespace() || c == ',') {
            let clean = tok.trim();
            if clean.is_empty()
                || clean == "127.0.0.1/8"
                || clean == "127.0.0.1"
                || clean == "::1"
            {
                continue;
            }
            if !clean
                .chars()
                .all(|c| c.is_ascii_hexdigit() || c == '.' || c == ':' || c == '/')
            {
                return Err(format!("Invalid ignoreip entry: {clean}"));
            }
            if !extra_ips.iter().any(|x| x == clean) {
                extra_ips.push(clean.to_string());
            }
        }
    }

    let ignore_line = if extra_ips.is_empty() {
        "ignoreip = 127.0.0.1/8 ::1".to_string()
    } else {
        format!("ignoreip = 127.0.0.1/8 ::1 {}", extra_ips.join(" "))
    };

    Ok(format!(
        r#"mkdir -p /etc/fail2ban/jail.d
cat << 'SSHBOX_F2B_EOF' > /etc/fail2ban/jail.d/99-sshbox.local
[DEFAULT]
{ignore_line}
bantime = {safe_bantime}
findtime = {safe_findtime}
maxretry = {safe_maxretry}

[sshd]
enabled = true
SSHBOX_F2B_EOF
fail2ban-client reload 2>/dev/null || systemctl restart fail2ban 2>/dev/null || service fail2ban restart"#
    ))
}

pub async fn fail2ban_ban_action(
    host_id: &str,
    pool: &SshPool,
    jail: String,
    ip: String,
    action: String,
    sudo_password: Option<String>,
) -> Result<Fail2banSnapshot, String> {
    let script = build_fail2ban_ban_action_script(&jail, &ip, &action)?;
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_fail2ban_snapshot(host_id, pool, sudo_password).await
}

pub async fn update_fail2ban_jail(
    host_id: &str,
    pool: &SshPool,
    maxretry: u32,
    findtime_secs: u32,
    bantime_secs: i64,
    ignore_ips: Vec<String>,
    sudo_password: Option<String>,
) -> Result<Fail2banSnapshot, String> {
    let script =
        build_update_fail2ban_jail_script(maxretry, findtime_secs, bantime_secs, &ignore_ips)?;
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_fail2ban_snapshot(host_id, pool, sudo_password).await
}

pub async fn install_fail2ban(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<Fail2banSnapshot, String> {
    let script = r#"
if ! command -v fail2ban-client >/dev/null 2>&1; then
  if command -v apt-get >/dev/null 2>&1; then
    DEBIAN_FRONTEND=noninteractive apt-get update -y && DEBIAN_FRONTEND=noninteractive apt-get install -y fail2ban
  elif command -v dnf >/dev/null 2>&1; then
    dnf install -y epel-release 2>/dev/null || true
    dnf install -y fail2ban
  elif command -v yum >/dev/null 2>&1; then
    yum install -y epel-release 2>/dev/null || true
    yum install -y fail2ban
  elif command -v apk >/dev/null 2>&1; then
    apk add --no-cache fail2ban
  else
    echo "No supported package manager (apt-get, dnf, yum, apk) found to install fail2ban."
    exit 1
  fi
fi
mkdir -p /etc/fail2ban/jail.d
if [ ! -f /etc/fail2ban/jail.d/99-sshbox.local ]; then
  cat << 'SSHBOX_F2B_EOF' > /etc/fail2ban/jail.d/99-sshbox.local
[DEFAULT]
ignoreip = 127.0.0.1/8 ::1
bantime = 3600
findtime = 600
maxretry = 5

[sshd]
enabled = true
SSHBOX_F2B_EOF
fi
systemctl enable fail2ban 2>/dev/null || rc-update add fail2ban default 2>/dev/null || true
systemctl restart fail2ban 2>/dev/null || service fail2ban restart 2>/dev/null || fail2ban-client reload 2>/dev/null || fail2ban-client -x start
"#;
    run_privileged_script(host_id, pool, sudo_password.clone(), script).await?;
    get_fail2ban_snapshot(host_id, pool, sudo_password).await
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
    let raw = run_privileged_script(host_id, pool, sudo_password, script).await?;
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
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
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
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
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
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
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
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_access_keys_snapshot(host_id, pool, sudo_password).await
}

fn parse_sysctl_bool(val: &str, default: bool) -> bool {
    let s = val.trim().to_ascii_lowercase();
    match s.as_str() {
        "1" | "2" | "yes" | "true" => true,
        "0" | "no" | "false" => false,
        _ => default,
    }
}

fn kernel_release_supports_bbr(release: &str) -> bool {
    let clean = release.trim();
    let mut parts = clean.split(|c: char| !c.is_ascii_digit());
    let Some(maj_s) = parts.next() else {
        return false;
    };
    let Some(min_s) = parts.next() else {
        return false;
    };
    if let (Ok(maj), Ok(min)) = (maj_s.parse::<u32>(), min_s.parse::<u32>()) {
        return maj > 4 || (maj == 4 && min >= 9);
    }
    false
}

fn sanitize_sysctl_token(raw: &str, fallback: &str) -> String {
    let clean: String = raw
        .trim()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .collect();
    if clean.is_empty() {
        fallback.to_string()
    } else {
        clean.to_ascii_lowercase()
    }
}

pub fn parse_kernel_sysctl_snapshot(raw: &str) -> KernelSysctlSnapshot {
    let mut snap = KernelSysctlSnapshot::default();
    let mut section = "";
    let mut bbr_mod_available = false;
    let mut persisted_lines: Vec<String> = Vec::new();

    for line in raw.lines() {
        let trimmed = line.trim();
        match trimmed {
            "---KERNEL_META---" => {
                section = "meta";
                continue;
            }
            "---SYSCTL---" => {
                section = "sysctl";
                continue;
            }
            "---PERSISTED---" => {
                section = "persisted";
                continue;
            }
            _ => {}
        }

        if section == "persisted" {
            persisted_lines.push(line.to_string());
            continue;
        }

        if trimmed.is_empty() || trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }

        if let Some(v) = trimmed.strip_prefix("KERNEL_RELEASE:") {
            let rel = v.trim();
            if !rel.is_empty() {
                snap.kernel_release = rel.to_string();
            }
            continue;
        }
        if let Some(v) = trimmed.strip_prefix("AVAILABLE_CC:") {
            let list: Vec<String> = v
                .split_whitespace()
                .map(|s| s.trim().to_ascii_lowercase())
                .filter(|s| !s.is_empty())
                .collect();
            if !list.is_empty() {
                snap.available_congestion_controls = list;
            }
            continue;
        }
        if let Some(v) = trimmed.strip_prefix("BBR_MOD_AVAILABLE:") {
            bbr_mod_available = parse_yes_no(v, false);
            continue;
        }

        let without_comment = trimmed.split('#').next().unwrap_or("").trim();
        if let Some((k, v)) = without_comment.split_once('=') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "net.core.default_qdisc" => {
                    let q = sanitize_sysctl_token(val, &snap.default_qdisc);
                    if !q.is_empty() {
                        snap.default_qdisc = q;
                    }
                }
                "net.ipv4.tcp_congestion_control" => {
                    let cc = sanitize_sysctl_token(val, &snap.congestion_control);
                    if !cc.is_empty() {
                        snap.congestion_control = cc;
                    }
                }
                "net.ipv4.tcp_available_congestion_control" => {
                    let list: Vec<String> = val
                        .split_whitespace()
                        .map(|s| s.trim().to_ascii_lowercase())
                        .filter(|s| !s.is_empty())
                        .collect();
                    if !list.is_empty() {
                        snap.available_congestion_controls = list;
                    }
                }
                "vm.swappiness" => {
                    if let Ok(n) = val.parse::<u32>() {
                        snap.swappiness = n.min(200);
                    }
                }
                "vm.overcommit_memory" => {
                    if let Ok(n) = val.parse::<u32>() {
                        snap.overcommit_memory = n.min(2);
                    }
                }
                "vm.vfs_cache_pressure" => {
                    if let Ok(n) = val.parse::<u32>() {
                        snap.vfs_cache_pressure = n.clamp(1, 10000);
                    }
                }
                "fs.file-max" => {
                    if let Ok(n) = val.parse::<u64>() {
                        if n > 0 {
                            snap.file_max = n;
                        }
                    }
                }
                "net.core.somaxconn" => {
                    if let Ok(n) = val.parse::<u32>() {
                        if n > 0 {
                            snap.somaxconn = n;
                        }
                    }
                }
                "net.ipv4.tcp_max_syn_backlog" => {
                    if let Ok(n) = val.parse::<u32>() {
                        if n > 0 {
                            snap.tcp_max_syn_backlog = n;
                        }
                    }
                }
                "net.ipv4.tcp_tw_reuse" => {
                    snap.tcp_tw_reuse = parse_sysctl_bool(val, snap.tcp_tw_reuse);
                }
                "net.ipv4.tcp_fin_timeout" => {
                    if let Ok(n) = val.parse::<u32>() {
                        if n > 0 {
                            snap.tcp_fin_timeout = n;
                        }
                    }
                }
                "net.ipv4.tcp_syncookies" => {
                    snap.tcp_syncookies = parse_sysctl_bool(val, snap.tcp_syncookies);
                }
                "net.ipv4.icmp_echo_ignore_all" => {
                    snap.icmp_echo_ignore_all = parse_sysctl_bool(val, snap.icmp_echo_ignore_all);
                }
                "net.ipv4.conf.all.rp_filter" => {
                    snap.rp_filter = parse_sysctl_bool(val, snap.rp_filter);
                }
                "net.ipv4.conf.all.accept_redirects" => {
                    snap.accept_redirects = parse_sysctl_bool(val, snap.accept_redirects);
                }
                "net.ipv4.conf.all.accept_source_route" => {
                    snap.accept_source_route = parse_sysctl_bool(val, snap.accept_source_route);
                }
                "net.ipv4.ip_forward" => {
                    snap.ip_forward = parse_sysctl_bool(val, snap.ip_forward);
                }
                _ => {}
            }
        }
    }

    snap.persisted_config = persisted_lines.join("\n").trim().to_string();
    snap.bbr_active = snap.congestion_control.eq_ignore_ascii_case("bbr");

    let has_bbr_in_list = snap
        .available_congestion_controls
        .iter()
        .any(|c| c.eq_ignore_ascii_case("bbr"));

    snap.bbr_supported = has_bbr_in_list
        || bbr_mod_available
        || snap.bbr_active
        || kernel_release_supports_bbr(&snap.kernel_release);

    if snap.bbr_supported && !has_bbr_in_list {
        snap.available_congestion_controls.push("bbr".to_string());
    }

    snap
}

pub fn render_sysctl_conf(update: &KernelSysctlUpdate) -> String {
    let qdisc = sanitize_sysctl_token(&update.default_qdisc, "fq");
    let cc = sanitize_sysctl_token(&update.congestion_control, "bbr");
    let swappiness = update.swappiness.min(100);
    let overcommit = update.overcommit_memory.min(2);
    let vfs_pressure = update.vfs_cache_pressure.clamp(1, 500);
    let file_max = update.file_max.max(65536);
    let somaxconn = update.somaxconn.max(128);
    let syn_backlog = update.tcp_max_syn_backlog.max(128);
    let tw_reuse = u8::from(update.tcp_tw_reuse);
    let fin_timeout = update.tcp_fin_timeout.clamp(5, 120);
    let syncookies = u8::from(update.tcp_syncookies);
    let icmp_ignore = u8::from(update.icmp_echo_ignore_all);
    let rp_filter = u8::from(update.rp_filter);
    let accept_redirects = u8::from(update.accept_redirects);
    let accept_source_route = u8::from(update.accept_source_route);
    let ip_forward = u8::from(update.ip_forward);

    format!(
        r#"# /etc/sysctl.d/99-sshbox.conf — Managed by SSH-Box Kernel & Network Tuning Studio
# Congestion Control & Packet Scheduler
net.core.default_qdisc = {qdisc}
net.ipv4.tcp_congestion_control = {cc}

# Virtual Memory & Swap Behavior
vm.swappiness = {swappiness}
vm.overcommit_memory = {overcommit}
vm.vfs_cache_pressure = {vfs_pressure}

# Filesystem & High-Concurrency Socket Backlogs
fs.file-max = {file_max}
net.core.somaxconn = {somaxconn}
net.ipv4.tcp_max_syn_backlog = {syn_backlog}
net.ipv4.tcp_tw_reuse = {tw_reuse}
net.ipv4.tcp_fin_timeout = {fin_timeout}

# Network Fortress, Anti-Spoofing & Routing
net.ipv4.tcp_syncookies = {syncookies}
net.ipv4.icmp_echo_ignore_all = {icmp_ignore}
net.ipv4.conf.all.rp_filter = {rp_filter}
net.ipv4.conf.default.rp_filter = {rp_filter}
net.ipv4.conf.all.accept_redirects = {accept_redirects}
net.ipv4.conf.default.accept_redirects = {accept_redirects}
net.ipv4.conf.all.accept_source_route = {accept_source_route}
net.ipv4.conf.default.accept_source_route = {accept_source_route}
net.ipv4.ip_forward = {ip_forward}
"#
    )
}

pub fn build_apply_kernel_tuning_script(update: &KernelSysctlUpdate) -> String {
    let rendered = render_sysctl_conf(update);
    let modprobe_step = if update.congestion_control.trim().eq_ignore_ascii_case("bbr") {
        "modprobe tcp_bbr 2>/dev/null || true\n"
    } else {
        ""
    };

    format!(
        r#"{modprobe_step}mkdir -p /etc/sysctl.d
cat << 'SSHBOX_SYSCTL_EOF' > /etc/sysctl.d/99-sshbox.conf
{rendered}SSHBOX_SYSCTL_EOF
chmod 0644 /etc/sysctl.d/99-sshbox.conf
sysctl -e -p /etc/sysctl.d/99-sshbox.conf >/dev/null 2>&1 || sysctl --system >/dev/null 2>&1 || true"#
    )
}

pub async fn get_kernel_snapshot(
    host_id: &str,
    pool: &SshPool,
    sudo_password: Option<String>,
) -> Result<KernelSysctlSnapshot, String> {
    let script = r#"
echo "---KERNEL_META---"
krel=$(uname -r 2>/dev/null || echo "Linux")
echo "KERNEL_RELEASE:$krel"
avail_cc=$(cat /proc/sys/net/ipv4/tcp_available_congestion_control 2>/dev/null || sysctl -n net.ipv4.tcp_available_congestion_control 2>/dev/null || echo "cubic reno")
echo "AVAILABLE_CC:$avail_cc"
bbr_mod=0
if printf '%s\n' "$avail_cc" | grep -qw bbr; then
  bbr_mod=1
elif modinfo tcp_bbr >/dev/null 2>&1 || find "/lib/modules/$krel" -name '*bbr*' 2>/dev/null | grep -q bbr; then
  bbr_mod=1
elif grep -q CONFIG_TCP_CONG_BBR "/boot/config-$krel" 2>/dev/null; then
  bbr_mod=1
fi
echo "BBR_MOD_AVAILABLE:$bbr_mod"
echo "---SYSCTL---"
sysctl -e \
  net.core.default_qdisc \
  net.ipv4.tcp_congestion_control \
  net.ipv4.tcp_available_congestion_control \
  vm.swappiness \
  vm.overcommit_memory \
  vm.vfs_cache_pressure \
  fs.file-max \
  net.core.somaxconn \
  net.ipv4.tcp_max_syn_backlog \
  net.ipv4.tcp_tw_reuse \
  net.ipv4.tcp_fin_timeout \
  net.ipv4.tcp_syncookies \
  net.ipv4.icmp_echo_ignore_all \
  net.ipv4.conf.all.rp_filter \
  net.ipv4.conf.all.accept_redirects \
  net.ipv4.conf.all.accept_source_route \
  net.ipv4.ip_forward 2>/dev/null || true
echo "---PERSISTED---"
if [ -f /etc/sysctl.d/99-sshbox.conf ]; then
  cat /etc/sysctl.d/99-sshbox.conf 2>/dev/null || true
fi
"#;
    let raw = run_privileged_script(host_id, pool, sudo_password, script).await?;
    Ok(parse_kernel_sysctl_snapshot(&raw))
}

pub async fn apply_kernel_tuning(
    host_id: &str,
    pool: &SshPool,
    update: KernelSysctlUpdate,
    sudo_password: Option<String>,
) -> Result<KernelSysctlSnapshot, String> {
    let script = build_apply_kernel_tuning_script(&update);
    run_privileged_script(host_id, pool, sudo_password.clone(), &script).await?;
    get_kernel_snapshot(host_id, pool, sudo_password).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_default_and_custom_sshd_config_accurately() {
        let sample = r#"
# OpenBSD Secure Shell server-wide configuration
Include /etc/ssh/sshd_config.d/*.conf

Port 2244
AddressFamily inet
PermitRootLogin without-password
MaxAuthTries 3
LoginGraceTime 1m
PubkeyAuthentication yes
PasswordAuthentication no
PermitEmptyPasswords no
ClientAliveInterval 300
ClientAliveCountMax 2
AllowUsers deploy admin@10.0.0.5
X11Forwarding no
AllowTcpForwarding no
AllowAgentForwarding no

# Example of overriding settings on a per-user basis
Match User anoncvs
    X11Forwarding yes
    AllowTcpForwarding yes
    PermitRootLogin yes
"#;

        let snap = parse_sshd_config(sample, 2, true);
        assert_eq!(snap.port, 2244);
        assert_eq!(snap.address_family, "inet");
        assert_eq!(snap.permit_root_login, "prohibit-password");
        assert_eq!(snap.max_auth_tries, 3);
        assert_eq!(snap.login_grace_time, 60);
        assert!(snap.pubkey_authentication);
        assert!(!snap.password_authentication);
        assert!(!snap.permit_empty_passwords);
        assert_eq!(snap.client_alive_interval, 300);
        assert_eq!(snap.client_alive_count_max, 2);
        assert_eq!(
            snap.allow_users,
            vec!["deploy".to_string(), "admin@10.0.0.5".to_string()]
        );
        assert!(!snap.x11_forwarding);
        assert!(!snap.allow_tcp_forwarding);
        assert!(!snap.allow_agent_forwarding);
        assert_eq!(snap.authorized_keys_count, 2);
        assert!(snap.rollback_pending);
        // Score should be 92 (prohibit-password gives 12 instead of 20 for 'no')
        assert_eq!(snap.security_score, 92);
    }

    #[test]
    fn computes_security_score_across_weak_and_hardened_profiles() {
        let weak_raw = r#"
Port 22
PermitRootLogin yes
PasswordAuthentication yes
PubkeyAuthentication yes
PermitEmptyPasswords yes
MaxAuthTries 6
X11Forwarding yes
LoginGraceTime 120
ClientAliveInterval 0
"#;
        let weak = parse_sshd_config(weak_raw, 0, false);
        assert_eq!(weak.security_score, 15);

        let fortress_raw = r#"
Port 49222
PermitRootLogin no
PasswordAuthentication no
PubkeyAuthentication yes
PermitEmptyPasswords no
MaxAuthTries 3
X11Forwarding no
LoginGraceTime 45
ClientAliveInterval 300
"#;
        let fortress = parse_sshd_config(fortress_raw, 3, false);
        assert_eq!(fortress.security_score, 100);
    }

    #[test]
    fn renders_sshd_config_preserving_comments_and_before_match_blocks() {
        let base_raw = r#"# Ubuntu default sshd_config
Include /etc/ssh/sshd_config.d/*.conf

#Port 22
PermitRootLogin yes
PasswordAuthentication yes
UsePAM yes
Subsystem sftp /usr/lib/openssh/sftp-server

Match User git
    AllowTcpForwarding no
"#;

        let update = SshdConfigUpdate {
            port: 32022,
            permit_root_login: "no".to_string(),
            password_authentication: false,
            pubkey_authentication: true,
            permit_empty_passwords: false,
            max_auth_tries: 3,
            login_grace_time: 60,
            client_alive_interval: 300,
            client_alive_count_max: 2,
            allow_users: vec!["ubuntu".to_string(), "ops@192.168.1.10".to_string()],
            x11_forwarding: false,
            allow_tcp_forwarding: true,
            allow_agent_forwarding: false,
            address_family: "inet".to_string(),
        };

        let rendered = render_sshd_config(base_raw, &update);
        assert!(rendered.contains("# Ubuntu default sshd_config"));
        assert!(rendered.contains("Include /etc/ssh/sshd_config.d/*.conf"));
        assert!(rendered.contains("UsePAM yes"));
        assert!(rendered.contains("Subsystem sftp /usr/lib/openssh/sftp-server"));
        assert!(rendered.contains("PermitRootLogin no"));
        assert!(rendered.contains("PasswordAuthentication no"));
        assert!(rendered.contains("Port 32022"));
        assert!(rendered.contains("AllowUsers ubuntu ops@192.168.1.10"));

        // Ensure Match User git remains at the end after managed directives
        let port_pos = rendered.find("Port 32022").unwrap();
        let match_pos = rendered.find("Match User git").unwrap();
        assert!(port_pos < match_pos);

        // Round-trip parse verification
        let reparsed = parse_sshd_config(&rendered, 1, false);
        assert_eq!(reparsed.port, 32022);
        assert_eq!(reparsed.permit_root_login, "no");
        assert!(!reparsed.password_authentication);
        assert!(reparsed.pubkey_authentication);
        assert!(!reparsed.permit_empty_passwords);
        assert_eq!(reparsed.max_auth_tries, 3);
        assert_eq!(reparsed.login_grace_time, 60);
        assert_eq!(reparsed.client_alive_interval, 300);
        assert_eq!(reparsed.client_alive_count_max, 2);
        assert_eq!(
            reparsed.allow_users,
            vec!["ubuntu".to_string(), "ops@192.168.1.10".to_string()]
        );
        assert!(!reparsed.x11_forwarding);
        assert!(reparsed.allow_tcp_forwarding);
        assert!(!reparsed.allow_agent_forwarding);
        assert_eq!(reparsed.address_family, "inet");
        assert_eq!(reparsed.security_score, 100);
    }

    #[test]
    fn parses_privilege_probe_output() {
        let probe = r#"
USER:ubuntu
UID:1000
NOPASSWD:0
SUDO_OK:1
OS:Ubuntu 24.04.1 LTS
CLIENT_IP:203.0.113.42
"#;
        let info = parse_privilege_probe(probe);
        assert_eq!(info.user, "ubuntu");
        assert_eq!(info.uid, 1000);
        assert!(!info.is_root);
        assert!(!info.passwordless_sudo);
        assert!(info.sudo_unlocked);
        assert_eq!(info.os_pretty_name, "Ubuntu 24.04.1 LTS");
        assert_eq!(info.detected_client_ip, "203.0.113.42");
    }

    #[test]
    fn parses_ufw_firewall_snapshot_and_cross_references_socket_exposure() {
        let raw = r#"
---META---
ENGINE:ufw
ENABLED:1
DEFAULT_INCOMING:deny
DEFAULT_OUTGOING:allow
CLIENT_IP:203.0.113.42
SSH_PORT:2222
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
        assert_eq!(snap.detected_client_ip, "203.0.113.42");
        assert_eq!(snap.ssh_port, 2222);
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

    #[test]
    fn enforces_anti_lockout_ssh_rules_before_ufw_enable_and_builds_rule_commands() {
        let script = build_set_firewall_state_script(true, "deny", 2244);
        let allow_custom_pos = script.find("ufw allow 2244/tcp").unwrap();
        let allow_22_pos = script.find("ufw allow 22/tcp").unwrap();
        let default_deny_pos = script.find("ufw default deny incoming").unwrap();
        let enable_pos = script.find("ufw --force enable").unwrap();
        assert!(allow_custom_pos < default_deny_pos);
        assert!(allow_22_pos < default_deny_pos);
        assert!(default_deny_pos < enable_pos);

        let add_ip_rule = build_add_firewall_rule_script(
            "ALLOW",
            "5432",
            "tcp",
            "203.0.113.42",
            "Postgres Admin",
        )
        .unwrap();
        assert!(add_ip_rule.contains("from 203.0.113.42 to any port 5432 proto tcp"));
        assert!(add_ip_rule.contains("Postgres Admin"));

        let whitelist_all = build_add_firewall_rule_script(
            "ALLOW",
            "any",
            "any",
            "203.0.113.42",
            "Admin Whitelist",
        )
        .unwrap();
        assert!(whitelist_all.contains("ufw allow from 203.0.113.42 to any"));
    }

    #[test]
    fn parses_fail2ban_jails_and_ssh_auth_audit_timeline_with_attacker_aggregation() {
        let raw = r#"
---F2B_META---
INSTALLED:1
RUNNING:1
CLIENT_IP:203.0.113.42
IGNORE_IPS:127.0.0.1/8 ::1 203.0.113.42 10.8.0.0/24
---F2B_JAIL---
JAIL_NAME:sshd
Status for the jail: sshd
|- Filter
|  |- Currently failed:	4
|  |- Total failed:	128
|  `- File list:	/var/log/auth.log
`- Actions
   |- Currently banned:	2
   |- Total banned:	19
   `- Banned IP list:	185.220.101.5 45.155.205.233
MAXRETRY:5
FINDTIME:600
BANTIME:3600
---AUTH_LOG---
Oct 30 06:10:12 prod-vps sshd[1010]: Accepted publickey for ubuntu from 203.0.113.42 port 54322 ssh2: ED25519 SHA256:abcd
Oct 30 06:11:01 prod-vps sshd[1015]: Failed password for root from 185.220.101.5 port 48120 ssh2
Oct 30 06:11:04 prod-vps sshd[1016]: Failed password for root from 185.220.101.5 port 48124 ssh2
Oct 30 06:11:09 prod-vps sshd[1017]: Failed password for invalid user admin from 185.220.101.5 port 48130 ssh2
Oct 30 06:12:00 prod-vps sshd[1020]: Invalid user postgres from 91.240.118.172 port 39100
Oct 30 06:12:03 prod-vps sshd[1021]: Failed password for invalid user deploy from 91.240.118.172 port 39104 ssh2
Oct 30 06:13:15 prod-vps sshd[1030]: Accepted password for deploy from 198.51.100.25 port 51234 ssh2
"#;

        let snap = parse_fail2ban_and_auth_snapshot(raw);
        assert!(snap.installed);
        assert!(snap.running);
        assert_eq!(snap.detected_client_ip, "203.0.113.42");
        assert_eq!(
            snap.ignore_ips,
            vec!["203.0.113.42".to_string(), "10.8.0.0/24".to_string()]
        );
        assert_eq!(snap.jails.len(), 1);

        let sshd = &snap.jails[0];
        assert_eq!(sshd.name, "sshd");
        assert_eq!(sshd.currently_failed, 4);
        assert_eq!(sshd.total_failed, 128);
        assert_eq!(sshd.currently_banned, 2);
        assert_eq!(sshd.total_banned, 19);
        assert_eq!(
            sshd.banned_ips,
            vec!["185.220.101.5".to_string(), "45.155.205.233".to_string()]
        );
        assert_eq!(sshd.maxretry, 5);
        assert_eq!(sshd.findtime_secs, 600);
        assert_eq!(sshd.bantime_secs, 3600);

        assert_eq!(snap.auth_events.len(), 7);
        assert_eq!(snap.auth_events[0].timestamp, "Oct 30 06:10:12");
        assert_eq!(snap.auth_events[0].event_type, "accepted");
        assert_eq!(snap.auth_events[0].method, "publickey");
        assert_eq!(snap.auth_events[0].username, "ubuntu");
        assert_eq!(snap.auth_events[0].ip, "203.0.113.42");
        assert_eq!(snap.auth_events[0].port, 54322);

        assert_eq!(snap.auth_events[3].event_type, "failed");
        assert_eq!(snap.auth_events[3].method, "password");
        assert_eq!(snap.auth_events[3].username, "admin");
        assert_eq!(snap.auth_events[3].ip, "185.220.101.5");
        assert_eq!(snap.auth_events[3].port, 48130);

        assert_eq!(snap.auth_events[4].event_type, "invalid_user");
        assert_eq!(snap.auth_events[4].method, "invalid_user");
        assert_eq!(snap.auth_events[4].username, "postgres");
        assert_eq!(snap.auth_events[4].ip, "91.240.118.172");
        assert_eq!(snap.auth_events[4].port, 39100);

        assert_eq!(snap.top_attacker_ips.len(), 2);
        assert_eq!(snap.top_attacker_ips[0].ip, "185.220.101.5");
        assert_eq!(snap.top_attacker_ips[0].failed_count, 3);
        assert_eq!(snap.top_attacker_ips[0].last_target_user, "admin");
        assert!(snap.top_attacker_ips[0].is_currently_banned);

        assert_eq!(snap.top_attacker_ips[1].ip, "91.240.118.172");
        assert_eq!(snap.top_attacker_ips[1].failed_count, 2);
        assert_eq!(snap.top_attacker_ips[1].last_target_user, "deploy");
        assert!(!snap.top_attacker_ips[1].is_currently_banned);
    }

    #[test]
    fn builds_fail2ban_policy_and_ban_action_scripts_safely() {
        let policy_script = build_update_fail2ban_jail_script(
            3,
            900,
            -1,
            &["203.0.113.42".to_string(), "127.0.0.1/8".to_string()],
        )
        .unwrap();
        assert!(policy_script.contains("ignoreip = 127.0.0.1/8 ::1 203.0.113.42"));
        assert!(policy_script.contains("bantime = -1"));
        assert!(policy_script.contains("findtime = 900"));
        assert!(policy_script.contains("maxretry = 3"));
        assert!(policy_script.contains("[sshd]\nenabled = true"));

        let ban_script = build_fail2ban_ban_action_script("sshd", "185.220.101.5", "ban").unwrap();
        assert!(ban_script.contains("fail2ban-client set 'sshd' banip '185.220.101.5'"));

        let unban_script =
            build_fail2ban_ban_action_script("sshd", "185.220.101.5", "unban").unwrap();
        assert!(unban_script.contains("fail2ban-client set 'sshd' unbanip '185.220.101.5'"));
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

    #[test]
    fn parses_kernel_sysctl_snapshot_with_bbr_and_persisted_config() {
        let raw = r#"
---KERNEL_META---
KERNEL_RELEASE:6.8.0-45-generic
AVAILABLE_CC:reno cubic bbr
BBR_MOD_AVAILABLE:1
---SYSCTL---
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
net.ipv4.tcp_available_congestion_control = reno cubic bbr
vm.swappiness = 10
vm.overcommit_memory = 1
vm.vfs_cache_pressure = 50
fs.file-max = 2097152
net.core.somaxconn = 65535
net.ipv4.tcp_max_syn_backlog = 16384
net.ipv4.tcp_tw_reuse = 1
net.ipv4.tcp_fin_timeout = 15
net.ipv4.tcp_syncookies = 1
net.ipv4.icmp_echo_ignore_all = 1
net.ipv4.conf.all.rp_filter = 1
net.ipv4.conf.all.accept_redirects = 0
net.ipv4.conf.all.accept_source_route = 0
net.ipv4.ip_forward = 1
---PERSISTED---
# Managed by SSH-Box
net.core.default_qdisc = fq
net.ipv4.tcp_congestion_control = bbr
"#;

        let snap = parse_kernel_sysctl_snapshot(raw);
        assert_eq!(snap.kernel_release, "6.8.0-45-generic");
        assert_eq!(
            snap.available_congestion_controls,
            vec!["reno".to_string(), "cubic".to_string(), "bbr".to_string()]
        );
        assert!(snap.bbr_supported);
        assert!(snap.bbr_active);
        assert_eq!(snap.congestion_control, "bbr");
        assert_eq!(snap.default_qdisc, "fq");
        assert_eq!(snap.swappiness, 10);
        assert_eq!(snap.overcommit_memory, 1);
        assert_eq!(snap.vfs_cache_pressure, 50);
        assert_eq!(snap.file_max, 2097152);
        assert_eq!(snap.somaxconn, 65535);
        assert_eq!(snap.tcp_max_syn_backlog, 16384);
        assert!(snap.tcp_tw_reuse);
        assert_eq!(snap.tcp_fin_timeout, 15);
        assert!(snap.tcp_syncookies);
        assert!(snap.icmp_echo_ignore_all);
        assert!(snap.rp_filter);
        assert!(!snap.accept_redirects);
        assert!(!snap.accept_source_route);
        assert!(snap.ip_forward);
        assert!(snap.persisted_config.contains("net.ipv4.tcp_congestion_control = bbr"));
    }

    #[test]
    fn renders_sysctl_conf_and_roundtrips_through_parser() {
        let update = KernelSysctlUpdate {
            congestion_control: "bbr".to_string(),
            default_qdisc: "fq".to_string(),
            swappiness: 10,
            overcommit_memory: 1,
            vfs_cache_pressure: 50,
            file_max: 2097152,
            somaxconn: 65535,
            tcp_max_syn_backlog: 16384,
            tcp_tw_reuse: true,
            tcp_fin_timeout: 15,
            tcp_syncookies: true,
            icmp_echo_ignore_all: true,
            rp_filter: true,
            accept_redirects: false,
            accept_source_route: false,
            ip_forward: true,
        };

        let rendered = render_sysctl_conf(&update);
        assert!(rendered.contains("net.core.default_qdisc = fq"));
        assert!(rendered.contains("net.ipv4.tcp_congestion_control = bbr"));
        assert!(rendered.contains("vm.swappiness = 10"));
        assert!(rendered.contains("vm.overcommit_memory = 1"));
        assert!(rendered.contains("vm.vfs_cache_pressure = 50"));
        assert!(rendered.contains("fs.file-max = 2097152"));
        assert!(rendered.contains("net.core.somaxconn = 65535"));
        assert!(rendered.contains("net.ipv4.tcp_max_syn_backlog = 16384"));
        assert!(rendered.contains("net.ipv4.tcp_tw_reuse = 1"));
        assert!(rendered.contains("net.ipv4.tcp_fin_timeout = 15"));
        assert!(rendered.contains("net.ipv4.tcp_syncookies = 1"));
        assert!(rendered.contains("net.ipv4.icmp_echo_ignore_all = 1"));
        assert!(rendered.contains("net.ipv4.conf.all.rp_filter = 1"));
        assert!(rendered.contains("net.ipv4.conf.all.accept_redirects = 0"));
        assert!(rendered.contains("net.ipv4.conf.all.accept_source_route = 0"));
        assert!(rendered.contains("net.ipv4.ip_forward = 1"));

        let reparsed = parse_kernel_sysctl_snapshot(&rendered);
        assert_eq!(reparsed.congestion_control, "bbr");
        assert_eq!(reparsed.default_qdisc, "fq");
        assert!(reparsed.bbr_active);
        assert!(reparsed.bbr_supported);
        assert_eq!(reparsed.swappiness, 10);
        assert_eq!(reparsed.overcommit_memory, 1);
        assert_eq!(reparsed.vfs_cache_pressure, 50);
        assert_eq!(reparsed.file_max, 2097152);
        assert_eq!(reparsed.somaxconn, 65535);
        assert_eq!(reparsed.tcp_max_syn_backlog, 16384);
        assert!(reparsed.tcp_tw_reuse);
        assert_eq!(reparsed.tcp_fin_timeout, 15);
        assert!(reparsed.tcp_syncookies);
        assert!(reparsed.icmp_echo_ignore_all);
        assert!(reparsed.rp_filter);
        assert!(!reparsed.accept_redirects);
        assert!(!reparsed.accept_source_route);
        assert!(reparsed.ip_forward);

        let apply_script = build_apply_kernel_tuning_script(&update);
        assert!(apply_script.contains("modprobe tcp_bbr 2>/dev/null || true"));
        assert!(apply_script.contains("/etc/sysctl.d/99-sshbox.conf"));
        assert!(apply_script.contains("sysctl -e -p /etc/sysctl.d/99-sshbox.conf"));
    }
}
