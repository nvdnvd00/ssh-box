use crate::frame_batcher::{batch_stream_to_sink, ChannelReader};
use crate::host_store::HostEntry;
use crate::resources::{self, PortForwardingRule};
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use russh::client::{self, Handle};
use russh::{ChannelMsg, Disconnect};
use russh_keys::key::PublicKey;
use russh_keys::PublicKeyBase64;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::sync::mpsc;

pub fn expand_tilde_path_with(raw: &str, home: &Path) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/") {
        return home.join(stripped);
    }
    PathBuf::from(raw)
}

pub fn expand_tilde_path(raw: &str) -> PathBuf {
    expand_tilde_path_with(raw, &crate::host_store::app_home_dir())
}

pub fn resolve_agent_sockets(
    host: &HostEntry,
    env_sock: Option<&str>,
    home: &Path,
) -> Vec<PathBuf> {
    if let Some(ref ia) = host.identity_agent {
        if ia.trim().eq_ignore_ascii_case("none") {
            return Vec::new();
        }
    }

    let mut sockets = Vec::new();
    let mut push_unique = |p: PathBuf| {
        if !p.as_os_str().is_empty() && !sockets.contains(&p) {
            sockets.push(p);
        }
    };

    if let Some(ref ia) = host.identity_agent {
        let trimmed = ia.trim();
        if !trimmed.is_empty() && !trimmed.eq_ignore_ascii_case("SSH_AUTH_SOCK") {
            push_unique(expand_tilde_path_with(trimmed, home));
        }
    } else if let Ok(raw_cfg) = std::fs::read_to_string(home.join(".ssh").join("config")) {
        for line in raw_cfg.lines() {
            let clean = line.split('#').next().unwrap_or("").trim();
            let mut parts = clean.splitn(2, |c: char| c.is_whitespace() || c == '=');
            if parts
                .next()
                .unwrap_or("")
                .trim()
                .eq_ignore_ascii_case("identityagent")
            {
                let val = parts
                    .next()
                    .unwrap_or("")
                    .trim()
                    .trim_start_matches('=')
                    .trim()
                    .trim_matches('"');
                if !val.is_empty()
                    && !val.eq_ignore_ascii_case("none")
                    && !val.eq_ignore_ascii_case("SSH_AUTH_SOCK")
                {
                    push_unique(expand_tilde_path_with(val, home));
                }
            }
        }
    }

    let is_apple_launchd = env_sock
        .map(|s| s.contains("com.apple.launchd"))
        .unwrap_or(false);

    if !is_apple_launchd {
        if let Some(s) = env_sock {
            push_unique(expand_tilde_path_with(s.trim(), home));
        }
    }

    for rel in [
        "Library/Containers/com.bitwarden.desktop/Data/.bitwarden-ssh-agent.sock",
        ".bitwarden-ssh-agent.sock",
        "Library/Group Containers/2BUA8C4S2C.com.1password/t/agent.sock",
        ".1password/agent.sock",
    ] {
        let candidate = home.join(rel);
        if candidate.exists() {
            push_unique(candidate);
        }
    }

    if is_apple_launchd {
        if let Some(s) = env_sock {
            push_unique(expand_tilde_path_with(s.trim(), home));
        }
    }

    sockets
}

pub fn default_known_hosts_path() -> PathBuf {
    crate::host_store::app_home_dir()
        .join(".ssh")
        .join("known_hosts")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct KnownHostEntry {
    pub id: String,
    pub host_pattern: String,
    pub key_type: String,
    pub fingerprint: String,
    pub public_key_base64: String,
    pub line_index: usize,
}

pub fn compute_base64_fingerprint(key_base64: &str) -> String {
    let raw_bytes = BASE64
        .decode(key_base64.trim())
        .unwrap_or_else(|_| key_base64.as_bytes().to_vec());
    let digest = Sha256::digest(&raw_bytes);
    format!("SHA256:{}", BASE64.encode(digest).trim_end_matches('='))
}

pub fn list_known_hosts_at(known_hosts_path: &Path) -> Vec<KnownHostEntry> {
    let mut entries = Vec::new();
    let Ok(raw) = std::fs::read_to_string(known_hosts_path) else {
        return entries;
    };
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 {
            let host_pattern = parts[0].to_string();
            let key_type = parts[1].to_string();
            let public_key_base64 = parts[2].to_string();
            let fingerprint = compute_base64_fingerprint(&public_key_base64);
            entries.push(KnownHostEntry {
                id: format!("kh:{idx}:{host_pattern}:{key_type}"),
                host_pattern,
                key_type,
                fingerprint,
                public_key_base64,
                line_index: idx,
            });
        }
    }
    entries
}

pub fn delete_known_host_at(
    known_hosts_path: &Path,
    host_pattern: &str,
    key_type: &str,
) -> Result<Vec<KnownHostEntry>, String> {
    if !known_hosts_path.exists() {
        return Ok(Vec::new());
    }
    let raw = std::fs::read_to_string(known_hosts_path).map_err(|e| e.to_string())?;
    let mut kept_lines = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            kept_lines.push(line);
            continue;
        }
        let parts: Vec<&str> = trimmed.split_whitespace().collect();
        if parts.len() >= 3 && parts[0] == host_pattern && parts[1] == key_type {
            continue;
        }
        kept_lines.push(line);
    }
    let updated = if kept_lines.is_empty() {
        String::new()
    } else {
        format!("{}\n", kept_lines.join("\n"))
    };
    std::fs::write(known_hosts_path, updated).map_err(|e| e.to_string())?;
    Ok(list_known_hosts_at(known_hosts_path))
}

pub fn clear_known_hosts_at(known_hosts_path: &Path) -> Result<Vec<KnownHostEntry>, String> {
    if known_hosts_path.exists() {
        std::fs::write(known_hosts_path, "").map_err(|e| e.to_string())?;
    }
    Ok(Vec::new())
}

pub fn parse_env_vars_pairs(raw: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let clean = trimmed.strip_prefix("export ").unwrap_or(trimmed).trim();
        if let Some((k, v)) = clean.split_once('=') {
            let key = k.trim();
            let val = v.trim().trim_matches('"').trim_matches('\'');
            if !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_alphanumeric() || c == '_')
            {
                out.push((key.to_string(), val.to_string()));
            }
        }
    }
    out
}

pub fn build_shell_init_line(env_vars: &str, startup_command: &str) -> Option<String> {
    let mut parts = Vec::new();
    for (k, v) in parse_env_vars_pairs(env_vars) {
        let quoted_v = format!("'{}'", v.replace('\'', "'\\''"));
        parts.push(format!("export {k}={quoted_v}"));
    }
    let cmd = startup_command.trim();
    if !cmd.is_empty() {
        parts.push(cmd.to_string());
    }
    if parts.is_empty() {
        None
    } else {
        Some(format!("{}\n", parts.join(" && ")))
    }
}

pub fn verify_tofu_host_key(
    known_hosts_path: &Path,
    host_port_id: &str,
    key_type: &str,
    key_base64: &str,
) -> Result<String, String> {
    let raw_bytes = BASE64
        .decode(key_base64.trim())
        .unwrap_or_else(|_| key_base64.as_bytes().to_vec());
    let digest = Sha256::digest(&raw_bytes);
    let fingerprint = format!("SHA256:{}", BASE64.encode(digest).trim_end_matches('='));

    if let Ok(existing) = std::fs::read_to_string(known_hosts_path) {
        for line in existing.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() || trimmed.starts_with('#') {
                continue;
            }
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 3 {
                let hosts_col = parts[0];
                let recorded_type = parts[1];
                let recorded_b64 = parts[2];
                let host_matches = hosts_col.split(',').any(|h| h == host_port_id);
                if host_matches && recorded_type == key_type {
                    if recorded_b64 == key_base64.trim() {
                        return Ok(fingerprint);
                    } else {
                        return Err(format!(
                            "REMOTE HOST IDENTIFICATION HAS CHANGED for {host_port_id}! Expected key does not match {fingerprint}"
                        ));
                    }
                }
            }
        }
    }

    if let Some(parent) = known_hosts_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(known_hosts_path)
        .map_err(|e| e.to_string())?;
    writeln!(file, "{} {} {}", host_port_id, key_type, key_base64.trim())
        .map_err(|e| e.to_string())?;

    Ok(fingerprint)
}

pub struct SshClientHandler {
    host_port_id: String,
    fingerprint_out: Arc<Mutex<Option<String>>>,
    host_key_err: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl client::Handler for SshClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        server_public_key: &PublicKey,
    ) -> Result<bool, Self::Error> {
        let key_type = server_public_key.name();
        let key_b64 = server_public_key.public_key_base64();
        let kh_path = default_known_hosts_path();
        match verify_tofu_host_key(&kh_path, &self.host_port_id, key_type, &key_b64) {
            Ok(fp) => {
                *self.fingerprint_out.lock().unwrap() = Some(fp);
                Ok(true)
            }
            Err(err_msg) => {
                *self.host_key_err.lock().unwrap() = Some(err_msg);
                Ok(false)
            }
        }
    }
}

enum SshPaneCommand {
    Input(Vec<u8>),
    Resize(u16, u16),
    Close,
}

pub struct MultiplexedSession {
    pub host_id: String,
    pub fingerprint: String,
    pub handle: tokio::sync::Mutex<Handle<SshClientHandler>>,
}

#[derive(Default, Clone)]
pub struct SshPool {
    sessions: Arc<tokio::sync::Mutex<HashMap<String, Arc<MultiplexedSession>>>>,
    pane_txs: Arc<Mutex<HashMap<String, (String, mpsc::UnboundedSender<SshPaneCommand>)>>>,
    forward_stops: Arc<Mutex<HashMap<String, mpsc::Sender<()>>>>,
}

impl SshPool {
    pub async fn get_or_connect(
        &self,
        host: &HostEntry,
        password_override: Option<String>,
    ) -> Result<Arc<MultiplexedSession>, String> {
        {
            let map = self.sessions.lock().await;
            if let Some(existing) = map.get(&host.id) {
                let h = existing.handle.lock().await;
                if !h.is_closed() {
                    drop(h);
                    return Ok(existing.clone());
                }
            }
        }

        let config = Arc::new(client::Config {
            keepalive_interval: Some(Duration::from_secs(15)),
            keepalive_max: 3,
            ..Default::default()
        });

        let host_port_id = if host.port == 22 {
            host.hostname.clone()
        } else {
            format!("[{}]:{}", host.hostname, host.port)
        };

        let fingerprint_out = Arc::new(Mutex::new(None));
        let host_key_err = Arc::new(Mutex::new(None));
        let handler = SshClientHandler {
            host_port_id,
            fingerprint_out: fingerprint_out.clone(),
            host_key_err: host_key_err.clone(),
        };

        let mut handle = tokio::time::timeout(
            Duration::from_secs(12),
            client::connect(config, (host.hostname.as_str(), host.port), handler),
        )
        .await
        .map_err(|_| format!("Connection timed out to {}:{}", host.hostname, host.port))?
        .map_err(|e| {
            if let Some(hk_err) = host_key_err.lock().unwrap().clone() {
                hk_err
            } else {
                format!("SSH connection failed: {e}")
            }
        })?;

        let store = resources::load_resources();
        let resolved = resources::resolve_host_credentials_from(&store, host, password_override);
        let authenticated = Self::authenticate_session(&mut handle, host, &resolved).await?;
        if !authenticated {
            return Err(format!(
                "Authentication failed for {}@{}:{} (tried SSH Agent, Identity/SSHKey, IdentityFile, and Keychain/Password)",
                resolved.user, host.hostname, host.port
            ));
        }

        let fp = fingerprint_out
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "SHA256:verified".to_string());

        let session = Arc::new(MultiplexedSession {
            host_id: host.id.clone(),
            fingerprint: fp,
            handle: tokio::sync::Mutex::new(handle),
        });

        self.sessions
            .lock()
            .await
            .insert(host.id.clone(), session.clone());
        Ok(session)
    }

    async fn authenticate_session(
        handle: &mut Handle<SshClientHandler>,
        host: &HostEntry,
        resolved: &resources::ResolvedHostCredentials,
    ) -> Result<bool, String> {
        let auth_user = &resolved.user;
        let secret = resolved.secret.as_deref();

        // 1. Try Managed SSH Keys (from Identity or Host sshKeyId)
        for (key_path, key_pass) in &resolved.key_paths {
            if !key_path.exists() {
                continue;
            }
            let key_res = russh_keys::load_secret_key(key_path, key_pass.as_deref())
                .or_else(|_| russh_keys::load_secret_key(key_path, secret));
            if let Ok(key_pair) = key_res {
                if let Ok(true) = handle
                    .authenticate_publickey(auth_user, Arc::new(key_pair))
                    .await
                {
                    return Ok(true);
                }
            }
        }

        // 2. Try SSH Agent (IdentityAgent, Bitwarden/1Password sockets, and SSH_AUTH_SOCK on Unix)
        #[cfg(unix)]
        {
            let env_sock = std::env::var("SSH_AUTH_SOCK").ok();
            let home = crate::host_store::app_home_dir();
            for sock in resolve_agent_sockets(host, env_sock.as_deref(), &home) {
                if let Ok(stream) = tokio::net::UnixStream::connect(&sock).await {
                    let mut agent = russh_keys::agent::client::AgentClient::connect(stream);
                    if let Ok(identities) = agent.request_identities().await {
                        for pubkey in identities {
                            let (returned_agent, res) =
                                handle.authenticate_future(auth_user, pubkey, agent).await;
                            agent = returned_agent;
                            if res.unwrap_or(false) {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }

        // 3. Try Private Keys (~/.ssh/id_* or configured IdentityFile)
        let mut candidate_paths = Vec::new();
        if let Some(ref custom_id) = host.identity_file {
            candidate_paths.push(expand_tilde_path(custom_id));
        }
        let home = crate::host_store::app_home_dir();
        if home.exists() {
            let ssh_dir = home.join(".ssh");
            for name in ["id_ed25519", "id_rsa", "id_ecdsa"] {
                let p = ssh_dir.join(name);
                if !candidate_paths.contains(&p) {
                    candidate_paths.push(p);
                }
            }
        }

        for key_path in candidate_paths {
            if !key_path.exists() {
                continue;
            }
            let key_res = russh_keys::load_secret_key(&key_path, None)
                .or_else(|_| russh_keys::load_secret_key(&key_path, secret));
            if let Ok(key_pair) = key_res {
                if let Ok(true) = handle
                    .authenticate_publickey(auth_user, Arc::new(key_pair))
                    .await
                {
                    return Ok(true);
                }
            }
        }

        // 4. Try Password
        if let Some(pw) = secret {
            if !pw.is_empty() {
                if let Ok(true) = handle.authenticate_password(auth_user, pw).await {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    pub async fn open_pty_pane<F>(
        &self,
        pane_id: String,
        host: &HostEntry,
        cols: u16,
        rows: u16,
        password_override: Option<String>,
        mut on_output: F,
    ) -> Result<String, String>
    where
        F: FnMut(Vec<u8>) -> bool + Send + 'static,
    {
        let session = self.get_or_connect(host, password_override).await?;
        let fingerprint = session.fingerprint.clone();

        let mut channel = {
            let handle = session.handle.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|e| format!("Failed to open multiplexed SSH channel: {e}"))?
        };

        channel
            .request_pty(
                false,
                "xterm-256color",
                cols as u32,
                rows as u32,
                0,
                0,
                &[],
            )
            .await
            .map_err(|e| format!("Failed to request PTY: {e}"))?;

        if host.agent_forwarding {
            let _ = channel.agent_forward(false).await;
        }

        let _ = channel.set_env(false, "COLORTERM", "truecolor").await;
        let _ = channel.set_env(false, "CLICOLOR", "1").await;
        let _ = channel.set_env(false, "LANG", "en_US.UTF-8").await;
        for (k, v) in parse_env_vars_pairs(&host.env_vars) {
            let _ = channel.set_env(false, &k, &v).await;
        }

        channel
            .request_shell(false)
            .await
            .map_err(|e| format!("Failed to request shell: {e}"))?;

        if let Some(init_line) = build_shell_init_line(&host.env_vars, &host.startup_command) {
            let _ = channel.data(init_line.as_bytes()).await;
        }

        let (cmd_tx, mut cmd_rx) = mpsc::unbounded_channel::<SshPaneCommand>();
        let (bytes_tx, bytes_rx) = mpsc::channel::<Vec<u8>>(64);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

        self.pane_txs
            .lock()
            .unwrap()
            .insert(pane_id.clone(), (host.id.clone(), cmd_tx));

        tokio::spawn(async move {
            let stream_reader = ChannelReader::new(bytes_rx);
            let _ = batch_stream_to_sink(stream_reader, move |chunk| on_output(chunk), shutdown_rx)
                .await;
        });

        let pane_txs_ref = self.pane_txs.clone();
        let pane_id_clone = pane_id.clone();
        tokio::spawn(async move {
            loop {
                tokio::select! {
                    biased;
                    some_cmd = cmd_rx.recv() => {
                        match some_cmd {
                            Some(SshPaneCommand::Input(data)) => {
                                if channel.data(&data[..]).await.is_err() {
                                    break;
                                }
                            }
                            Some(SshPaneCommand::Resize(c, r)) => {
                                let _ = channel.window_change(c as u32, r as u32, 0, 0).await;
                            }
                            Some(SshPaneCommand::Close) | None => {
                                let _ = channel.close().await;
                                break;
                            }
                        }
                    }
                    msg = channel.wait() => {
                        match msg {
                            Some(ChannelMsg::Data { data }) => {
                                if bytes_tx.send(data.to_vec()).await.is_err() {
                                    break;
                                }
                            }
                            Some(ChannelMsg::ExtendedData { data, .. }) => {
                                if bytes_tx.send(data.to_vec()).await.is_err() {
                                    break;
                                }
                            }
                            Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => {
                                let _ = bytes_tx.send(b"\r\n\x1b[33m[SSH channel closed. Press Enter to reconnect]\x1b[0m\r\n".to_vec()).await;
                                break;
                            }
                            _ => {}
                        }
                    }
                }
            }
            let _ = shutdown_tx.send(()).await;
            pane_txs_ref.lock().unwrap().remove(&pane_id_clone);
        });

        Ok(fingerprint)
    }

    pub fn write_input(&self, pane_id: &str, data: &[u8]) -> Result<(), String> {
        let map = self.pane_txs.lock().unwrap();
        let (_, tx) = map.get(pane_id).ok_or("SSH pane not found")?;
        tx.send(SshPaneCommand::Input(data.to_vec()))
            .map_err(|e| e.to_string())
    }

    pub fn resize(&self, pane_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let map = self.pane_txs.lock().unwrap();
        let (_, tx) = map.get(pane_id).ok_or("SSH pane not found")?;
        tx.send(SshPaneCommand::Resize(cols, rows))
            .map_err(|e| e.to_string())
    }

    pub async fn close_pane(&self, pane_id: &str) -> bool {
        let removed = self.pane_txs.lock().unwrap().remove(pane_id);
        if let Some((host_id, tx)) = removed {
            let _ = tx.send(SshPaneCommand::Close);
            let still_has_panes = self
                .pane_txs
                .lock()
                .unwrap()
                .values()
                .any(|(h, _)| h == &host_id);
            if !still_has_panes {
                if let Some(sess) = self.sessions.lock().await.remove(&host_id) {
                    let handle = sess.handle.lock().await;
                    let _ = handle
                        .disconnect(Disconnect::ByApplication, "All panes closed", "en")
                        .await;
                }
            }
            true
        } else {
            false
        }
    }

    pub async fn has_active_session(&self, host_id: &str) -> bool {
        let map = self.sessions.lock().await;
        if let Some(sess) = map.get(host_id) {
            let h = sess.handle.lock().await;
            !h.is_closed()
        } else {
            false
        }
    }

    pub async fn exec_command(&self, host_id: &str, command: &str) -> Result<String, String> {
        let session = {
            let map = self.sessions.lock().await;
            map.get(host_id)
                .cloned()
                .ok_or_else(|| format!("No active SSH session for {host_id}"))?
        };

        let mut channel = {
            let handle = session.handle.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|e| format!("Failed to open exec channel: {e}"))?
        };

        channel
            .exec(true, command)
            .await
            .map_err(|e| format!("Failed to exec telemetry command: {e}"))?;

        let mut out = Vec::new();
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } | ChannelMsg::ExtendedData { data, .. } => {
                    out.extend_from_slice(&data)
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }
        Ok(String::from_utf8_lossy(&out).to_string())
    }

    pub async fn exec_on_host(&self, host: &HostEntry, command: &str) -> Result<String, String> {
        let _ = self.get_or_connect(host, None).await?;
        self.exec_command(&host.id, command).await
    }

    pub async fn open_exec_channel(
        &self,
        host: &HostEntry,
        command: &str,
    ) -> Result<russh::Channel<client::Msg>, String> {
        let session = self.get_or_connect(host, None).await?;
        let channel = {
            let handle = session.handle.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|e| format!("Failed to open SSH channel: {e}"))?
        };
        channel
            .exec(true, command)
            .await
            .map_err(|e| format!("Failed to exec SSH command: {e}"))?;
        Ok(channel)
    }

    pub async fn export_public_key_to_host(
        &self,
        host: &HostEntry,
        public_key: &str,
        password_override: Option<String>,
    ) -> Result<(), String> {
        let clean_pub = public_key.trim();
        if clean_pub.is_empty() {
            return Err("Public key is empty".to_string());
        }
        let session = self.get_or_connect(host, password_override).await?;
        let mut channel = {
            let handle = session.handle.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|e| format!("Failed to open SSH channel: {e}"))?
        };
        let quoted_pub = format!("'{}'", clean_pub.replace('\'', "'\\''"));
        let cmd = format!(
            "umask 077; mkdir -p ~/.ssh && chmod 700 ~/.ssh && touch ~/.ssh/authorized_keys && chmod 600 ~/.ssh/authorized_keys && (grep -qxF {quoted_pub} ~/.ssh/authorized_keys || printf '%s\\n' {quoted_pub} >> ~/.ssh/authorized_keys)"
        );
        channel
            .exec(true, cmd.as_str())
            .await
            .map_err(|e| format!("Failed to install public key on host: {e}"))?;

        let mut stderr_buf = Vec::new();
        let mut exit_code: Option<u32> = None;
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::ExtendedData { data, .. } => stderr_buf.extend_from_slice(&data),
                ChannelMsg::ExitStatus { exit_status } => exit_code = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }
        if let Some(code) = exit_code {
            if code != 0 {
                let err_msg = String::from_utf8_lossy(&stderr_buf).trim().to_string();
                return Err(if err_msg.is_empty() {
                    format!("Remote ssh-copy-id exited with code {code}")
                } else {
                    err_msg
                });
            }
        }
        Ok(())
    }

    pub async fn exec_bytes_on_host(
        &self,
        host: &HostEntry,
        command: &str,
        stdin_data: Option<&[u8]>,
    ) -> Result<Vec<u8>, String> {
        let session = self.get_or_connect(host, None).await?;
        let mut channel = {
            let handle = session.handle.lock().await;
            handle
                .channel_open_session()
                .await
                .map_err(|e| format!("Failed to open SSH channel: {e}"))?
        };

        channel
            .exec(true, command)
            .await
            .map_err(|e| format!("Failed to exec SSH command: {e}"))?;

        if let Some(input) = stdin_data {
            for chunk in input.chunks(32768) {
                channel
                    .data(chunk)
                    .await
                    .map_err(|e| format!("Failed to stream data to remote host: {e}"))?;
            }
            let _ = channel.eof().await;
        }

        let mut stdout_buf = Vec::new();
        let mut stderr_buf = Vec::new();
        let mut exit_code: Option<u32> = None;

        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => stdout_buf.extend_from_slice(&data),
                ChannelMsg::ExtendedData { data, .. } => stderr_buf.extend_from_slice(&data),
                ChannelMsg::ExitStatus { exit_status } => exit_code = Some(exit_status),
                ChannelMsg::Close => break,
                _ => {}
            }
        }

        if let Some(code) = exit_code {
            if code != 0 {
                let err_text = String::from_utf8_lossy(&stderr_buf).trim().to_string();
                let out_text = String::from_utf8_lossy(&stdout_buf).trim().to_string();
                let detail = if !err_text.is_empty() {
                    err_text
                } else if !out_text.is_empty() {
                    out_text
                } else {
                    format!("Remote command exited with status {code}")
                };
                return Err(detail);
            }
        }

        Ok(stdout_buf)
    }

    pub async fn start_port_forward(
        &self,
        rule: &PortForwardingRule,
        host: &HostEntry,
    ) -> Result<(), String> {
        self.stop_port_forward(&rule.id);
        let session = self.get_or_connect(host, None).await?;
        let bind_addr = format!(
            "{}:{}",
            if rule.local_host.trim().is_empty() {
                "127.0.0.1"
            } else {
                rule.local_host.trim()
            },
            rule.local_port
        );
        let listener = tokio::net::TcpListener::bind(&bind_addr)
            .await
            .map_err(|e| format!("Failed to bind {bind_addr}: {e}"))?;

        let (stop_tx, mut stop_rx) = mpsc::channel::<()>(1);
        self.forward_stops
            .lock()
            .unwrap()
            .insert(rule.id.clone(), stop_tx);

        let mode = rule.mode.clone();
        let target_host = if rule.target_host.trim().is_empty() {
            "127.0.0.1".to_string()
        } else {
            rule.target_host.trim().to_string()
        };
        let target_port = if rule.target_port == 0 {
            rule.remote_port
        } else {
            rule.target_port
        };

        tokio::spawn(async move {
            loop {
                tokio::select! {
                    _ = stop_rx.recv() => break,
                    accept_res = listener.accept() => {
                        let Ok((mut tcp_stream, peer_addr)) = accept_res else { break };
                        let sess = session.clone();
                        let mode_cl = mode.clone();
                        let thost = target_host.clone();
                        let tport = target_port;
                        tokio::spawn(async move {
                            let (dest_host, dest_port) = if mode_cl == "dynamic" {
                                // Minimal SOCKS5 handshake
                                let mut greeting = [0u8; 2];
                                if tcp_stream.read_exact(&mut greeting).await.is_err() || greeting[0] != 5 {
                                    return;
                                }
                                let nmethods = greeting[1] as usize;
                                let mut methods = vec![0u8; nmethods];
                                if tcp_stream.read_exact(&mut methods).await.is_err() {
                                    return;
                                }
                                let _ = tcp_stream.write_all(&[5, 0]).await;
                                let mut req_head = [0u8; 4];
                                if tcp_stream.read_exact(&mut req_head).await.is_err() || req_head[1] != 1 {
                                    return;
                                }
                                let dhost = match req_head[3] {
                                    1 => {
                                        let mut ip = [0u8; 4];
                                        if tcp_stream.read_exact(&mut ip).await.is_err() { return; }
                                        format!("{}.{}.{}.{}", ip[0], ip[1], ip[2], ip[3])
                                    }
                                    3 => {
                                        let mut len = [0u8; 1];
                                        if tcp_stream.read_exact(&mut len).await.is_err() { return; }
                                        let mut dom = vec![0u8; len[0] as usize];
                                        if tcp_stream.read_exact(&mut dom).await.is_err() { return; }
                                        String::from_utf8_lossy(&dom).to_string()
                                    }
                                    _ => return,
                                };
                                let mut port_buf = [0u8; 2];
                                if tcp_stream.read_exact(&mut port_buf).await.is_err() { return; }
                                let dport = u16::from_be_bytes(port_buf);
                                let _ = tcp_stream.write_all(&[5, 0, 0, 1, 0, 0, 0, 0, 0, 0]).await;
                                (dhost, dport)
                            } else {
                                (thost, tport)
                            };

                            let channel_res = {
                                let h = sess.handle.lock().await;
                                h.channel_open_direct_tcpip(
                                    &dest_host,
                                    dest_port as u32,
                                    &peer_addr.ip().to_string(),
                                    peer_addr.port() as u32,
                                ).await
                            };
                            let Ok(mut channel) = channel_res else { return };
                            let mut buf = vec![0u8; 16384];
                            loop {
                                tokio::select! {
                                    n_res = tcp_stream.read(&mut buf) => {
                                        match n_res {
                                            Ok(0) | Err(_) => {
                                                let _ = channel.eof().await;
                                                break;
                                            }
                                            Ok(n) => {
                                                if channel.data(&buf[..n]).await.is_err() {
                                                    break;
                                                }
                                            }
                                        }
                                    }
                                    msg = channel.wait() => {
                                        match msg {
                                            Some(ChannelMsg::Data { data }) => {
                                                if tcp_stream.write_all(&data).await.is_err() {
                                                    break;
                                                }
                                            }
                                            Some(ChannelMsg::Eof) | Some(ChannelMsg::Close) | None => break,
                                            _ => {}
                                        }
                                    }
                                }
                            }
                            let _ = channel.close().await;
                        });
                    }
                }
            }
        });

        Ok(())
    }

    pub fn stop_port_forward(&self, rule_id: &str) -> bool {
        if let Some(tx) = self.forward_stops.lock().unwrap().remove(rule_id) {
            let _ = tx.try_send(());
            true
        } else {
            false
        }
    }

    pub fn is_port_forward_active(&self, rule_id: &str) -> bool {
        self.forward_stops.lock().unwrap().contains_key(rule_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tofu_records_new_host_key_and_rejects_mismatch() {
        let tmp_dir =
            std::env::temp_dir().join(format!("ssh-box-tofu-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let kh_path = tmp_dir.join("known_hosts");

        let fp1 = verify_tofu_host_key(
            &kh_path,
            "[10.0.1.25]:2222",
            "ssh-ed25519",
            "AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyDataOne",
        )
        .expect("first connection should succeed via TOFU");
        assert!(fp1.starts_with("SHA256:"));

        let fp2 = verify_tofu_host_key(
            &kh_path,
            "[10.0.1.25]:2222",
            "ssh-ed25519",
            "AAAAC3NzaC1lZDI1NTE5AAAAIExampleKeyDataOne",
        )
        .expect("subsequent connection with same key should succeed");
        assert_eq!(fp1, fp2);

        let err = verify_tofu_host_key(
            &kh_path,
            "[10.0.1.25]:2222",
            "ssh-ed25519",
            "AAAAC3NzaC1lZDI1NTE5AAAAIDifferentAttackerKey",
        )
        .expect_err("changed host key must be rejected");
        assert!(err.contains("IDENTIFICATION HAS CHANGED"));

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn manages_known_hosts_list_delete_and_clear() {
        let tmp_dir =
            std::env::temp_dir().join(format!("ssh-box-kh-mgr-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let kh_path = tmp_dir.join("known_hosts");

        let _ = verify_tofu_host_key(
            &kh_path,
            "192.168.1.10",
            "ssh-ed25519",
            "AAAAC3NzaC1lZDI1NTE5AAAAIKeyOne",
        )
        .unwrap();
        let _ = verify_tofu_host_key(
            &kh_path,
            "[10.0.0.5]:2222",
            "ecdsa-sha2-nistp256",
            "AAAAE2VjZHNhLXNoYTItbmlzdHAyNTYAAAAIKeyTwo",
        )
        .unwrap();

        let list = list_known_hosts_at(&kh_path);
        assert_eq!(list.len(), 2);
        assert_eq!(list[0].host_pattern, "192.168.1.10");
        assert_eq!(list[1].host_pattern, "[10.0.0.5]:2222");

        let after_del = delete_known_host_at(&kh_path, "192.168.1.10", "ssh-ed25519").unwrap();
        assert_eq!(after_del.len(), 1);
        assert_eq!(after_del[0].host_pattern, "[10.0.0.5]:2222");

        let after_clear = clear_known_hosts_at(&kh_path).unwrap();
        assert!(after_clear.is_empty());

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn builds_shell_init_line_from_env_vars_and_startup_command() {
        let env_raw = "APP_ENV=production\nexport TZ=\"Asia/Ho_Chi_Minh\"\n# comment";
        let cmd = "tmux attach -t main || tmux new -s main";
        let init = build_shell_init_line(env_raw, cmd).unwrap();
        assert_eq!(
            init,
            "export APP_ENV='production' && export TZ='Asia/Ho_Chi_Minh' && tmux attach -t main || tmux new -s main\n"
        );
        assert!(build_shell_init_line("", "   ").is_none());
    }

    #[test]
    fn expands_tilde_to_user_home() {
        let expanded = expand_tilde_path("~/.ssh/id_ed25519");
        assert!(!expanded.to_string_lossy().starts_with("~/"));
        assert!(expanded.to_string_lossy().ends_with(".ssh/id_ed25519"));
    }

    #[test]
    fn resolves_identity_agent_and_bitwarden_sockets_alongside_launchd() {
        let tmp_home =
            std::env::temp_dir().join(format!("ssh-box-agent-test-{}", uuid::Uuid::new_v4()));
        let bw_dir = tmp_home.join("Library/Containers/com.bitwarden.desktop/Data");
        std::fs::create_dir_all(&bw_dir).unwrap();
        let bw_sock = bw_dir.join(".bitwarden-ssh-agent.sock");
        std::fs::write(&bw_sock, b"").unwrap();

        let host = HostEntry {
            alias: "example-vps".into(),
            hostname: "203.0.113.10".into(),
            user: "root".into(),
            identity_agent: Some(
                "~/Library/Containers/com.bitwarden.desktop/Data/.bitwarden-ssh-agent.sock".into(),
            ),
            ..Default::default()
        };

        let sockets = resolve_agent_sockets(
            &host,
            Some("/var/run/com.apple.launchd.test/Listeners"),
            &tmp_home,
        );
        assert_eq!(sockets.first(), Some(&bw_sock));
        assert!(sockets.contains(&PathBuf::from("/var/run/com.apple.launchd.test/Listeners")));

        let custom_host = HostEntry::default();
        let fallback_sockets = resolve_agent_sockets(
            &custom_host,
            Some("/var/run/com.apple.launchd.test/Listeners"),
            &tmp_home,
        );
        assert!(fallback_sockets.contains(&bw_sock));

        let _ = std::fs::remove_dir_all(&tmp_home);
    }
}
