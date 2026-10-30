pub mod frame_batcher;
pub mod host_store;
pub mod local_pty;
pub mod resources;
pub mod sftp;
pub mod ssh_pool;
pub mod telemetry;
pub mod vault;
pub mod vps_config;
pub mod vps_guard;

use host_store::HostEntry;
use local_pty::LocalPtyManager;
use resources::{
    ConnectionProxyProfile, IdentityItem, MachineGroup, MachineTag, MachinesExportBundle,
    PortForwardingRule, ResourcesStoreData, SnippetExecutionRecord, SnippetItem, SnippetPackage,
};
use serde::{Deserialize, Serialize};
use sftp::{SftpListResponse, SftpTransferProgress, SftpTransferResult};
use ssh_pool::{KnownHostEntry, SshPool};
use std::collections::{HashMap, HashSet};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use tauri::ipc::Channel;
use tauri::State;
use telemetry::{DockerFullSnapshot, ServerMetrics};
use tokio::sync::mpsc;
use vps_config::{
    AccessKeysSnapshot, FirewallRuleItem, FirewallSnapshot, SshdApplyStarted, SshdSettings, SshdView, SudoMode, VpsProbe,
};

pub struct AppState {
    pub local_pty: LocalPtyManager,
    pub ssh_pool: SshPool,
    pub telemetry_stops: Mutex<HashMap<String, mpsc::Sender<()>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            local_pty: LocalPtyManager::default(),
            ssh_pool: SshPool::default(),
            telemetry_stops: Mutex::new(HashMap::new()),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenSshPaneResponse {
    pub fingerprint: String,
}

fn enrich_resources(mut store: ResourcesStoreData, pool: &SshPool) -> ResourcesStoreData {
    for rule in &mut store.port_forwarding_rules {
        if pool.is_port_forward_active(&rule.id) {
            rule.status = "running".to_string();
        } else if rule.status == "running" {
            rule.status = "stopped".to_string();
        }
    }
    store
}

#[tauri::command]
fn list_hosts() -> Result<Vec<HostEntry>, String> {
    Ok(host_store::load_all_hosts())
}

#[tauri::command]
fn get_ui_settings() -> Option<serde_json::Value> {
    host_store::load_ui_settings_at(&host_store::default_settings_path())
}

#[tauri::command]
fn save_ui_settings(data: serde_json::Value) -> Result<(), String> {
    host_store::save_ui_settings_at(&host_store::default_settings_path(), &data)
}

#[tauri::command]
fn save_host(host: HostEntry, secret: Option<String>) -> Result<Vec<HostEntry>, String> {
    let custom_id = if host.id.trim().is_empty() {
        format!("custom:{}", uuid::Uuid::new_v4())
    } else {
        host.id.clone()
    };
    if let Some(pw) = secret {
        if !pw.trim().is_empty() {
            vault::set_host_secret(&custom_id, pw.trim())?;
        }
    }
    let mut updated = host;
    updated.id = custom_id;
    host_store::upsert_custom_host(updated)
}

#[tauri::command]
fn delete_host(host_id: String) -> Result<Vec<HostEntry>, String> {
    let _ = vault::delete_host_secret(&host_id);
    host_store::delete_custom_host(&host_id)
}

// Returns false when the user cancels the save dialog.
#[tauri::command]
async fn export_machines_bundle(
    app: tauri::AppHandle,
    hosts: Vec<HostEntry>,
    include_identities: bool,
    include_ssh_keys: bool,
    passphrase: Option<String>,
    file_name: String,
) -> Result<bool, String> {
    use std::io::Write;
    use tauri_plugin_dialog::DialogExt;
    use tauri_plugin_fs::FsExt;
    let bundle = resources::export_machines_bundle_at(
        &resources::default_resources_path(),
        hosts,
        include_identities,
        include_ssh_keys,
        passphrase,
    )?;
    let Some(picked) = app
        .dialog()
        .file()
        .set_file_name(file_name)
        .add_filter("JSON", &["json"])
        .blocking_save_file()
    else {
        return Ok(false);
    };
    let json = serde_json::to_string_pretty(&bundle).map_err(|e| e.to_string())?;
    // fs().open also takes the content:// URI the Android dialog returns.
    let mut opts = tauri_plugin_fs::OpenOptions::new();
    opts.read(false).write(true).create(true).truncate(true);
    let mut file = app
        .fs()
        .open(picked, opts)
        .map_err(|e| format!("Cannot save to this location: {e}"))?;
    file.write_all(json.as_bytes()).map_err(|e| e.to_string())?;
    Ok(true)
}

#[tauri::command]
async fn import_machines_bundle(
    bundle: MachinesExportBundle,
    selected_indices: Vec<usize>,
    include_identities: bool,
    include_ssh_keys: bool,
    passphrase: Option<String>,
) -> Result<(), String> {
    let (ssh_cfg, custom_cfg) = host_store::default_paths();
    resources::import_machines_bundle_at(
        &ssh_cfg,
        &custom_cfg,
        &resources::default_resources_path(),
        &resources::default_keys_dir(),
        bundle,
        selected_indices,
        include_identities,
        include_ssh_keys,
        passphrase,
    )
}

#[tauri::command]
async fn get_resources(state: State<'_, AppState>) -> Result<ResourcesStoreData, String> {
    let store = resources::load_resources();
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn save_machine_group(
    state: State<'_, AppState>,
    group: MachineGroup,
) -> Result<ResourcesStoreData, String> {
    let store = resources::upsert_group_at(&resources::default_resources_path(), group)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_machine_group(
    state: State<'_, AppState>,
    group_id: String,
) -> Result<ResourcesStoreData, String> {
    let store = resources::delete_group_at(&resources::default_resources_path(), &group_id)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn save_machine_tag(
    state: State<'_, AppState>,
    tag: MachineTag,
) -> Result<ResourcesStoreData, String> {
    let store = resources::upsert_tag_at(&resources::default_resources_path(), tag)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_machine_tag(
    state: State<'_, AppState>,
    tag_id: String,
) -> Result<ResourcesStoreData, String> {
    let path = resources::default_resources_path();
    // Machines reference tags by name, so drop it from them too.
    if let Some(tag) = resources::load_resources_at(&path)
        .tags
        .into_iter()
        .find(|t| t.id == tag_id)
    {
        host_store::remove_tag_from_hosts(&tag.name)?;
    }
    let store = resources::delete_tag_at(&path, &tag_id)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn save_identity(
    state: State<'_, AppState>,
    identity: IdentityItem,
    password: Option<String>,
) -> Result<ResourcesStoreData, String> {
    let store = resources::upsert_identity_at(
        &resources::default_resources_path(),
        identity,
        password,
    )?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_identity(
    state: State<'_, AppState>,
    identity_id: String,
) -> Result<ResourcesStoreData, String> {
    let store = resources::delete_identity_at(&resources::default_resources_path(), &identity_id)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn generate_ssh_key(
    state: State<'_, AppState>,
    name: String,
    remark: String,
    key_type: String,
    passphrase: Option<String>,
) -> Result<ResourcesStoreData, String> {
    let store = resources::generate_ssh_key_at(
        &resources::default_resources_path(),
        &resources::default_keys_dir(),
        name,
        remark,
        key_type,
        passphrase,
    )?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn import_ssh_key(
    state: State<'_, AppState>,
    key_id: Option<String>,
    name: String,
    remark: String,
    private_key: String,
    public_key: Option<String>,
    passphrase: Option<String>,
) -> Result<ResourcesStoreData, String> {
    let store = resources::import_ssh_key_at(
        &resources::default_resources_path(),
        &resources::default_keys_dir(),
        key_id,
        name,
        remark,
        private_key,
        public_key,
        passphrase,
    )?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_ssh_key(
    state: State<'_, AppState>,
    key_id: String,
) -> Result<ResourcesStoreData, String> {
    let store = resources::delete_ssh_key_at(&resources::default_resources_path(), &key_id)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn save_proxy_profile(
    state: State<'_, AppState>,
    profile: ConnectionProxyProfile,
    password: Option<String>,
) -> Result<ResourcesStoreData, String> {
    let store = resources::upsert_proxy_profile_at(
        &resources::default_resources_path(),
        profile,
        password,
    )?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_proxy_profile(
    state: State<'_, AppState>,
    profile_id: String,
) -> Result<ResourcesStoreData, String> {
    let store =
        resources::delete_proxy_profile_at(&resources::default_resources_path(), &profile_id)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn save_port_forwarding_rule(
    state: State<'_, AppState>,
    rule: PortForwardingRule,
) -> Result<ResourcesStoreData, String> {
    let store =
        resources::upsert_port_forwarding_rule_at(&resources::default_resources_path(), rule)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_port_forwarding_rule(
    state: State<'_, AppState>,
    rule_id: String,
) -> Result<ResourcesStoreData, String> {
    state.ssh_pool.stop_port_forward(&rule_id);
    let store = resources::delete_port_forwarding_rule_at(
        &resources::default_resources_path(),
        &rule_id,
    )?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn toggle_port_forwarding_rule(
    state: State<'_, AppState>,
    rule_id: String,
    active: bool,
) -> Result<ResourcesStoreData, String> {
    let path = resources::default_resources_path();
    let mut store = resources::load_resources_at(&path);
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    if let Some(rule) = store
        .port_forwarding_rules
        .iter_mut()
        .find(|r| r.id == rule_id)
    {
        if !active {
            state.ssh_pool.stop_port_forward(&rule_id);
            rule.status = "stopped".to_string();
            rule.error_message = None;
        } else {
            let hosts = host_store::load_all_hosts();
            let host = hosts
                .iter()
                .find(|h| h.id == rule.machine_id)
                .ok_or_else(|| format!("Target machine '{}' not found", rule.machine_id))?;
            match state.ssh_pool.start_port_forward(rule, host).await {
                Ok(_) => {
                    rule.status = "running".to_string();
                    rule.last_started_date = Some(now);
                    rule.error_message = None;
                }
                Err(err) => {
                    rule.status = "failed".to_string();
                    rule.error_message = Some(err.clone());
                    let _ = resources::save_resources_at(&path, &store);
                    return Err(err);
                }
            }
        }
        resources::save_resources_at(&path, &store)?;
    }
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn save_snippet_package(
    state: State<'_, AppState>,
    pkg: SnippetPackage,
) -> Result<ResourcesStoreData, String> {
    let store = resources::upsert_snippet_package_at(&resources::default_resources_path(), pkg)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_snippet_package(
    state: State<'_, AppState>,
    package_id: String,
) -> Result<ResourcesStoreData, String> {
    let store =
        resources::delete_snippet_package_at(&resources::default_resources_path(), &package_id)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn save_snippet(
    state: State<'_, AppState>,
    snippet: SnippetItem,
) -> Result<ResourcesStoreData, String> {
    let store = resources::upsert_snippet_at(&resources::default_resources_path(), snippet)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn delete_snippet(
    state: State<'_, AppState>,
    snippet_id: String,
) -> Result<ResourcesStoreData, String> {
    let store = resources::delete_snippet_at(&resources::default_resources_path(), &snippet_id)?;
    Ok(enrich_resources(store, &state.ssh_pool))
}

#[tauri::command]
async fn execute_snippet_batch(
    state: State<'_, AppState>,
    snippet_id: String,
    machine_ids: Vec<String>,
    group_names: Vec<String>,
) -> Result<ResourcesStoreData, String> {
    let path = resources::default_resources_path();
    let store = resources::load_resources_at(&path);
    let snippet = store
        .snippets
        .iter()
        .find(|s| s.id == snippet_id)
        .cloned()
        .ok_or_else(|| "Snippet not found".to_string())?;

    let all_hosts = host_store::load_all_hosts();
    let mut target_ids: Vec<String> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();

    for mid in machine_ids {
        if seen.insert(mid.clone()) {
            target_ids.push(mid);
        }
    }
    if !group_names.is_empty() {
        for h in &all_hosts {
            if group_names.iter().any(|g| g == &h.group) && seen.insert(h.id.clone()) {
                target_ids.push(h.id.clone());
            }
        }
    }

    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);

    for tid in target_ids {
        let machine_name = if tid == "local" {
            "Local Machine".to_string()
        } else {
            all_hosts
                .iter()
                .find(|h| h.id == tid)
                .map(|h| h.alias.clone())
                .unwrap_or_else(|| tid.clone())
        };

        let exec_res = telemetry::run_host_or_local_cmd(&tid, &state.ssh_pool, &snippet.code).await;
        let (status, result) = match exec_res {
            Ok(out) => (
                "finished".to_string(),
                if out.trim().is_empty() {
                    "(completed with no output)".to_string()
                } else {
                    out
                },
            ),
            Err(err) => ("failed".to_string(), err),
        };

        let record = SnippetExecutionRecord {
            id: format!("exec:{}", uuid::Uuid::new_v4()),
            snippet_id: snippet.id.clone(),
            snippet_name: snippet.name.clone(),
            machine_id: tid,
            machine_name,
            execution_date: now,
            status,
            result,
        };
        let _ = resources::record_snippet_execution_at(&path, record);
    }

    let updated = resources::load_resources_at(&path);
    Ok(enrich_resources(updated, &state.ssh_pool))
}

#[tauri::command]
async fn fetch_docker_snapshot(
    state: State<'_, AppState>,
    host_id: String,
) -> Result<DockerFullSnapshot, String> {
    let raw = telemetry::run_host_or_local_cmd(
        &host_id,
        &state.ssh_pool,
        telemetry::DOCKER_SNAPSHOT_PROBE_CMD,
    )
    .await?;
    Ok(telemetry::parse_docker_snapshot(&raw))
}

#[tauri::command]
async fn docker_action(
    state: State<'_, AppState>,
    host_id: String,
    scope: String,  // "container" | "compose" | "image" | "volume" | "network"
    action: String, // "start" | "stop" | "restart" | "rm" | "prune" | "up" | "down"
    target: String,
) -> Result<DockerFullSnapshot, String> {
    let safe_target = target.replace('\'', "'\\''");
    let subcmd = match (scope.as_str(), action.as_str()) {
        ("container", "start") => format!("docker container start '{safe_target}'"),
        ("container", "stop") => format!("docker container stop '{safe_target}'"),
        ("container", "restart") => format!("docker container restart '{safe_target}'"),
        ("container", "rm") => format!("docker container rm -f '{safe_target}'"),
        ("compose", "restart") => format!("docker compose -p '{safe_target}' restart"),
        ("compose", "stop") | ("compose", "down") => {
            format!("docker compose -p '{safe_target}' stop")
        }
        ("compose", "start") | ("compose", "up") => {
            format!("docker compose -p '{safe_target}' start")
        }
        ("image", "rm") => format!("docker image rm -f '{safe_target}'"),
        ("volume", "rm") => format!("docker volume rm -f '{safe_target}'"),
        ("volume", "prune") => "docker volume prune -f".to_string(),
        ("network", "rm") => format!("docker network rm '{safe_target}'"),
        _ => return Err(format!("Unsupported docker action: {scope}/{action}")),
    };
    let full_cmd = format!(
        "export PATH=\"$HOME/.docker/bin:/usr/local/bin:/opt/homebrew/bin:$PATH\"; {subcmd} 2>&1"
    );
    let _ = telemetry::run_host_or_local_cmd(&host_id, &state.ssh_pool, &full_cmd).await?;
    fetch_docker_snapshot(state, host_id).await
}

#[tauri::command]
async fn docker_logs(
    state: State<'_, AppState>,
    host_id: String,
    container_id: String,
    tail: Option<u32>,
) -> Result<String, String> {
    let safe_id = container_id.replace('\'', "'\\''");
    let lines = tail.unwrap_or(200);
    let cmd = format!(
        "export PATH=\"$HOME/.docker/bin:/usr/local/bin:/opt/homebrew/bin:$PATH\"; docker logs --tail {lines} '{safe_id}' 2>&1"
    );
    telemetry::run_host_or_local_cmd(&host_id, &state.ssh_pool, &cmd).await
}

#[tauri::command]
async fn kill_process(
    state: State<'_, AppState>,
    host_id: String,
    pid: u32,
    force: bool,
) -> Result<String, String> {
    let sig = if force { "-9" } else { "-15" };
    let cmd = format!("kill {sig} {pid} 2>&1");
    telemetry::run_host_or_local_cmd(&host_id, &state.ssh_pool, &cmd).await
}

#[tauri::command]
async fn spawn_local_pty(
    state: State<'_, AppState>,
    pane_id: String,
    cols: u16,
    rows: u16,
    shell: Option<String>,
    use_tmux: Option<bool>,
    on_data: Channel<Vec<u8>>,
) -> Result<(), String> {
    state.local_pty.spawn_pane(
        pane_id,
        cols,
        rows,
        shell.as_deref(),
        use_tmux.unwrap_or(false),
        move |chunk| on_data.send(chunk).is_ok(),
    )
}

#[tauri::command]
async fn list_local_shells() -> Vec<local_pty::LocalShellOption> {
    local_pty::list_local_shells().await
}

#[tauri::command]
async fn connect_ssh_pane(
    state: State<'_, AppState>,
    pane_id: String,
    host: HostEntry,
    cols: u16,
    rows: u16,
    password: Option<String>,
    use_tmux: Option<bool>,
    on_data: Channel<Vec<u8>>,
) -> Result<OpenSshPaneResponse, String> {
    let fingerprint = state
        .ssh_pool
        .open_pty_pane(
            pane_id,
            &host,
            cols,
            rows,
            password,
            use_tmux.unwrap_or(false),
            move |chunk| on_data.send(chunk).is_ok(),
        )
        .await?;
    Ok(OpenSshPaneResponse { fingerprint })
}

#[tauri::command]
async fn test_ssh_connection(
    state: State<'_, AppState>,
    host: HostEntry,
    secret: Option<String>,
) -> Result<String, String> {
    let password = secret
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    state.ssh_pool.test_connection(&host, password).await
}

#[tauri::command]
fn write_pane_input(
    state: State<'_, AppState>,
    pane_id: String,
    data: Vec<u8>,
) -> Result<(), String> {
    if state.local_pty.write_input(&pane_id, &data).is_ok() {
        return Ok(());
    }
    state.ssh_pool.write_input(&pane_id, &data)
}

#[tauri::command]
fn resize_pane(
    state: State<'_, AppState>,
    pane_id: String,
    cols: u16,
    rows: u16,
) -> Result<(), String> {
    if state.local_pty.resize(&pane_id, cols, rows).is_ok() {
        return Ok(());
    }
    state.ssh_pool.resize(&pane_id, cols, rows)
}

#[tauri::command]
async fn close_pane(state: State<'_, AppState>, pane_id: String) -> Result<(), String> {
    if !state.local_pty.close(&pane_id) {
        state.ssh_pool.close_pane(&pane_id).await;
    }
    Ok(())
}

#[tauri::command]
async fn subscribe_telemetry(
    state: State<'_, AppState>,
    host_id: String,
    interval_secs: Option<u64>,
    on_metrics: Channel<ServerMetrics>,
) -> Result<(), String> {
    let (stop_tx, stop_rx) = mpsc::channel::<()>(1);
    {
        let mut stops = state.telemetry_stops.lock().unwrap();
        if let Some(prev_stop) = stops.remove(&host_id) {
            let _ = prev_stop.try_send(());
        }
        stops.insert(host_id.clone(), stop_tx);
    }
    let secs = interval_secs.unwrap_or(2).clamp(1, 60);
    telemetry::start_monitoring(host_id, state.ssh_pool.clone(), secs, on_metrics, stop_rx);
    Ok(())
}

#[tauri::command]
fn unsubscribe_telemetry(state: State<'_, AppState>, host_id: String) -> Result<(), String> {
    let mut stops = state.telemetry_stops.lock().unwrap();
    if let Some(prev_stop) = stops.remove(&host_id) {
        let _ = prev_stop.try_send(());
    }
    Ok(())
}

#[tauri::command]
async fn sftp_list_dir(
    state: State<'_, AppState>,
    host_id: String,
    path: String,
) -> Result<SftpListResponse, String> {
    sftp::list_dir(&state.ssh_pool, &host_id, &path).await
}

#[tauri::command]
async fn sftp_mkdir(
    state: State<'_, AppState>,
    host_id: String,
    path: String,
) -> Result<(), String> {
    sftp::mkdir(&state.ssh_pool, &host_id, &path).await
}

#[tauri::command]
async fn sftp_rename(
    state: State<'_, AppState>,
    host_id: String,
    old_path: String,
    new_path: String,
) -> Result<(), String> {
    sftp::rename_entry(&state.ssh_pool, &host_id, &old_path, &new_path).await
}

#[tauri::command]
async fn sftp_delete(
    state: State<'_, AppState>,
    host_id: String,
    path: String,
) -> Result<(), String> {
    sftp::delete_entry(&state.ssh_pool, &host_id, &path).await
}

#[tauri::command]
async fn sftp_chmod(
    state: State<'_, AppState>,
    host_id: String,
    path: String,
    mode: String,
) -> Result<(), String> {
    sftp::chmod_entry(&state.ssh_pool, &host_id, &path, &mode).await
}

#[tauri::command]
async fn sftp_read_text(
    state: State<'_, AppState>,
    host_id: String,
    path: String,
) -> Result<String, String> {
    sftp::read_text_file(&state.ssh_pool, &host_id, &path).await
}

#[tauri::command]
async fn sftp_write_text(
    state: State<'_, AppState>,
    host_id: String,
    path: String,
    content: String,
) -> Result<(), String> {
    sftp::write_text_file(&state.ssh_pool, &host_id, &path, &content).await
}

#[tauri::command]
async fn sftp_transfer(
    state: State<'_, AppState>,
    source_host_id: String,
    source_path: String,
    target_host_id: String,
    target_dir: String,
    prefer_fast_sync: Option<bool>,
    on_progress: Channel<SftpTransferProgress>,
) -> Result<SftpTransferResult, String> {
    let fast_sync = prefer_fast_sync.unwrap_or(true);
    sftp::transfer_entry_with_progress(
        &state.ssh_pool,
        &source_host_id,
        &source_path,
        &target_host_id,
        &target_dir,
        fast_sync,
        move |ev| {
            let _ = on_progress.send(ev);
        },
    )
    .await
}

#[tauri::command]
fn list_known_hosts() -> Result<Vec<KnownHostEntry>, String> {
    Ok(ssh_pool::list_known_hosts_at(
        &ssh_pool::default_known_hosts_path(),
    ))
}

#[tauri::command]
fn delete_known_host(
    host_pattern: String,
    key_type: String,
) -> Result<Vec<KnownHostEntry>, String> {
    ssh_pool::delete_known_host_at(
        &ssh_pool::default_known_hosts_path(),
        &host_pattern,
        &key_type,
    )
}

#[tauri::command]
fn clear_known_hosts() -> Result<Vec<KnownHostEntry>, String> {
    ssh_pool::clear_known_hosts_at(&ssh_pool::default_known_hosts_path())
}

#[tauri::command]
async fn export_ssh_key_to_host(
    state: State<'_, AppState>,
    key_id: String,
    host_id: String,
    password: Option<String>,
) -> Result<Vec<HostEntry>, String> {
    let store = resources::load_resources();
    let key_item = store
        .ssh_keys
        .iter()
        .find(|k| k.id == key_id)
        .ok_or_else(|| format!("SSH Key '{key_id}' not found"))?;
    let all_hosts = host_store::load_all_hosts();
    let mut host = all_hosts
        .into_iter()
        .find(|h| h.id == host_id)
        .ok_or_else(|| format!("Host '{host_id}' not found"))?;

    state
        .ssh_pool
        .export_public_key_to_host(&host, &key_item.public_key, password)
        .await?;

    if host.source == host_store::HostSource::Custom {
        host.ssh_key_id = Some(key_id);
        let _ = host_store::upsert_custom_host(host);
    }
    Ok(host_store::load_all_hosts())
}

#[tauri::command]
fn save_session_log(
    host_label: String,
    content: String,
    append: Option<bool>,
    file_name: Option<String>,
) -> Result<String, String> {
    use std::io::Write;
    let logs_dir = host_store::app_config_dir().join("session-logs");
    std::fs::create_dir_all(&logs_dir).map_err(|e| e.to_string())?;
    let safe_label: String = host_label
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else {
                '_'
            }
        })
        .collect();
    let fname = file_name.unwrap_or_else(|| {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        format!("{safe_label}-{ts}.log")
    });
    let full_path = logs_dir.join(fname);
    if append.unwrap_or(false) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&full_path)
            .map_err(|e| e.to_string())?;
        f.write_all(content.as_bytes()).map_err(|e| e.to_string())?;
    } else {
        std::fs::write(&full_path, content.as_bytes()).map_err(|e| e.to_string())?;
    }
    Ok(full_path.to_string_lossy().to_string())
}

#[tauri::command]
async fn vps_probe(
    state: State<'_, AppState>,
    host_id: String,
    sudo_password: Option<String>,
) -> Result<VpsProbe, String> {
    vps_config::probe(&host_id, &state.ssh_pool, sudo_password).await
}

#[tauri::command]
async fn vps_get_sshd_settings(
    state: State<'_, AppState>,
    host_id: String,
    sudo_password: Option<String>,
) -> Result<SshdView, String> {
    vps_config::get_sshd_view(&host_id, &state.ssh_pool, sudo_password).await
}

#[tauri::command]
async fn vps_apply_sshd_settings(
    state: State<'_, AppState>,
    host_id: String,
    desired: SshdSettings,
    sudo_password: Option<String>,
) -> Result<SshdApplyStarted, String> {
    vps_config::apply_sshd_settings(&host_id, &state.ssh_pool, sudo_password, desired).await
}

#[tauri::command]
async fn vps_finish_sshd_apply(
    state: State<'_, AppState>,
    host_id: String,
    new_port: Option<u16>,
    sudo_password: Option<String>,
) -> Result<SshdView, String> {
    vps_config::finish_sshd_apply(&host_id, &state.ssh_pool, sudo_password, new_port).await
}

#[tauri::command]
async fn vps_revert_sshd_settings(
    state: State<'_, AppState>,
    host_id: String,
    sudo_password: Option<String>,
) -> Result<SshdView, String> {
    vps_config::revert_sshd_settings(&host_id, &state.ssh_pool, sudo_password).await
}

#[tauri::command]
async fn vps_get_firewall_snapshot(
    state: State<'_, AppState>,
    host_id: String,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    vps_config::get_firewall_snapshot(&host_id, &state.ssh_pool, sudo_password).await
}

#[tauri::command]
async fn vps_add_firewall_rule(
    state: State<'_, AppState>,
    host_id: String,
    action: String,
    source: String,
    port: String,
    proto: String,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    vps_config::add_firewall_rule(&host_id, &state.ssh_pool, sudo_password, action, source, port, proto).await
}

#[tauri::command]
async fn vps_delete_firewall_rule(
    state: State<'_, AppState>,
    host_id: String,
    rule: FirewallRuleItem,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    vps_config::delete_firewall_rule(&host_id, &state.ssh_pool, sudo_password, rule).await
}

#[tauri::command]
async fn vps_set_firewall_state(
    state: State<'_, AppState>,
    host_id: String,
    enable: bool,
    sudo_password: Option<String>,
) -> Result<FirewallSnapshot, String> {
    vps_config::set_firewall_state(&host_id, &state.ssh_pool, sudo_password, enable).await
}

#[tauri::command]
async fn vps_get_access_keys_snapshot(
    state: State<'_, AppState>,
    host_id: String,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    vps_config::get_access_keys_snapshot(&host_id, &state.ssh_pool, sudo_password).await
}

#[tauri::command]
async fn vps_authorize_ssh_key(
    state: State<'_, AppState>,
    host_id: String,
    username: String,
    key_line: String,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    vps_config::authorize_ssh_key(&host_id, &state.ssh_pool, sudo_password, username, key_line).await
}

#[tauri::command]
async fn vps_revoke_ssh_key(
    state: State<'_, AppState>,
    host_id: String,
    username: String,
    key_body: String,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    vps_config::revoke_ssh_key(&host_id, &state.ssh_pool, sudo_password, username, key_body).await
}

#[tauri::command]
async fn vps_create_linux_user(
    state: State<'_, AppState>,
    host_id: String,
    username: String,
    sudo: SudoMode,
    password: Option<String>,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    vps_config::create_linux_user(&host_id, &state.ssh_pool, sudo_password, username, sudo, password).await
}

#[tauri::command]
async fn vps_set_user_lock(
    state: State<'_, AppState>,
    host_id: String,
    username: String,
    lock: bool,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    vps_config::set_user_lock(&host_id, &state.ssh_pool, sudo_password, username, lock).await
}

#[tauri::command]
async fn vps_set_user_sudo(
    state: State<'_, AppState>,
    host_id: String,
    username: String,
    sudo: SudoMode,
    password: Option<String>,
    sudo_password: Option<String>,
) -> Result<AccessKeysSnapshot, String> {
    vps_config::set_user_sudo(&host_id, &state.ssh_pool, sudo_password, username, sudo, password).await
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            use tauri::Manager;
            if let Ok(app_dir) = app.path().app_data_dir() {
                host_store::set_app_data_dir(app_dir);
            }
            #[cfg(desktop)]
            app.handle().plugin(tauri_plugin_updater::Builder::new().build())?;
            Ok(())
        })
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_hosts,
            get_ui_settings,
            save_ui_settings,
            save_host,
            delete_host,
            export_machines_bundle,
            import_machines_bundle,
            get_resources,
            save_machine_group,
            delete_machine_group,
            save_machine_tag,
            delete_machine_tag,
            save_identity,
            delete_identity,
            generate_ssh_key,
            import_ssh_key,
            delete_ssh_key,
            save_proxy_profile,
            delete_proxy_profile,
            save_port_forwarding_rule,
            delete_port_forwarding_rule,
            toggle_port_forwarding_rule,
            save_snippet_package,
            delete_snippet_package,
            save_snippet,
            delete_snippet,
            execute_snippet_batch,
            fetch_docker_snapshot,
            docker_action,
            docker_logs,
            kill_process,
            spawn_local_pty,
            list_local_shells,
            connect_ssh_pane,
            test_ssh_connection,
            write_pane_input,
            resize_pane,
            close_pane,
            subscribe_telemetry,
            unsubscribe_telemetry,
            sftp_list_dir,
            sftp_mkdir,
            sftp_rename,
            sftp_delete,
            sftp_chmod,
            sftp_read_text,
            sftp_write_text,
            sftp_transfer,
            list_known_hosts,
            delete_known_host,
            clear_known_hosts,
            export_ssh_key_to_host,
            save_session_log,
            vps_probe,
            vps_get_sshd_settings,
            vps_apply_sshd_settings,
            vps_finish_sshd_apply,
            vps_revert_sshd_settings,
            vps_get_firewall_snapshot,
            vps_add_firewall_rule,
            vps_delete_firewall_rule,
            vps_set_firewall_state,
            vps_get_access_keys_snapshot,
            vps_authorize_ssh_key,
            vps_revoke_ssh_key,
            vps_create_linux_user,
            vps_set_user_lock,
            vps_set_user_sudo,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
