use crate::frame_batcher::{batch_stream_to_sink, ChannelReader};
use crate::host_store::HostEntry;
use crate::vault;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use russh::client::{self, Handle};
use russh::{ChannelMsg, Disconnect};
use russh_keys::key::PublicKey;
use russh_keys::PublicKeyBase64;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

pub fn expand_tilde_path(raw: &str) -> PathBuf {
    if let Some(stripped) = raw.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(stripped);
        }
    }
    PathBuf::from(raw)
}

pub fn default_known_hosts_path() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".ssh")
        .join("known_hosts")
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

        let secret_opt = password_override.or_else(|| vault::get_host_secret(&host.id).ok().flatten());
        let authenticated = Self::authenticate_session(&mut handle, host, secret_opt.as_deref()).await?;
        if !authenticated {
            return Err(format!(
                "Authentication failed for {}@{}:{} (tried SSH Agent, IdentityFile, and Keychain/Password)",
                host.user, host.hostname, host.port
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
        secret: Option<&str>,
    ) -> Result<bool, String> {
        // 1. Try SSH Agent (SSH_AUTH_SOCK)
        if let Ok(sock) = std::env::var("SSH_AUTH_SOCK") {
            if let Ok(stream) = tokio::net::UnixStream::connect(&sock).await {
                let mut agent = russh_keys::agent::client::AgentClient::connect(stream);
                if let Ok(identities) = agent.request_identities().await {
                    for pubkey in identities {
                        let (returned_agent, res) =
                            handle.authenticate_future(&host.user, pubkey, agent).await;
                        agent = returned_agent;
                        if res.unwrap_or(false) {
                            return Ok(true);
                        }
                    }
                }
            }
        }

        // 2. Try Private Keys (~/.ssh/id_* or configured IdentityFile)
        let mut candidate_paths = Vec::new();
        if let Some(ref custom_id) = host.identity_file {
            candidate_paths.push(expand_tilde_path(custom_id));
        }
        if let Some(home) = dirs::home_dir() {
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
                    .authenticate_publickey(&host.user, Arc::new(key_pair))
                    .await
                {
                    return Ok(true);
                }
            }
        }

        // 3. Try Password
        if let Some(pw) = secret {
            if !pw.is_empty() {
                if let Ok(true) = handle.authenticate_password(&host.user, pw).await {
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

        channel
            .request_shell(false)
            .await
            .map_err(|e| format!("Failed to request shell: {e}"))?;

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
                ChannelMsg::Data { data } => out.extend_from_slice(&data),
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }
        Ok(String::from_utf8_lossy(&out).to_string())
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
    fn expands_tilde_to_user_home() {
        let expanded = expand_tilde_path("~/.ssh/id_ed25519");
        assert!(!expanded.to_string_lossy().starts_with("~/"));
        assert!(expanded.to_string_lossy().ends_with(".ssh/id_ed25519"));
    }
}
