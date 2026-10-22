pub mod frame_batcher;
pub mod host_store;
pub mod local_pty;
pub mod ssh_pool;
pub mod telemetry;
pub mod vault;

use host_store::HostEntry;
use local_pty::LocalPtyManager;
use serde::{Deserialize, Serialize};
use ssh_pool::SshPool;
use std::collections::HashMap;
use std::sync::Mutex;
use tauri::ipc::Channel;
use tauri::State;
use telemetry::ServerMetrics;
use tokio::sync::mpsc;

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

#[tauri::command]
fn list_hosts() -> Result<Vec<HostEntry>, String> {
    Ok(host_store::load_all_hosts())
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

#[tauri::command]
async fn spawn_local_pty(
    state: State<'_, AppState>,
    pane_id: String,
    cols: u16,
    rows: u16,
    on_data: Channel<Vec<u8>>,
) -> Result<(), String> {
    state
        .local_pty
        .spawn_pane(pane_id, cols, rows, None, move |chunk| {
            on_data.send(chunk).is_ok()
        })
}

#[tauri::command]
async fn connect_ssh_pane(
    state: State<'_, AppState>,
    pane_id: String,
    host: HostEntry,
    cols: u16,
    rows: u16,
    password: Option<String>,
    on_data: Channel<Vec<u8>>,
) -> Result<OpenSshPaneResponse, String> {
    let fingerprint = state
        .ssh_pool
        .open_pty_pane(pane_id, &host, cols, rows, password, move |chunk| {
            on_data.send(chunk).is_ok()
        })
        .await?;
    Ok(OpenSshPaneResponse { fingerprint })
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
    telemetry::start_monitoring(host_id, state.ssh_pool.clone(), on_metrics, stop_rx);
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            list_hosts,
            save_host,
            delete_host,
            spawn_local_pty,
            connect_ssh_pane,
            write_pane_input,
            resize_pane,
            close_pane,
            subscribe_telemetry,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
