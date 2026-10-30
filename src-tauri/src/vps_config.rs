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
}
