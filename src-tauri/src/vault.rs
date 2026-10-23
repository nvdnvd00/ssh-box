use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
use std::collections::BTreeMap;
use std::path::PathBuf;

const SERVICE_NAME: &str = "com.darien.sshbox";

fn fallback_vault_path() -> PathBuf {
    crate::host_store::app_config_dir().join("vault.json")
}

fn read_fallback_map() -> BTreeMap<String, String> {
    let path = fallback_vault_path();
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<BTreeMap<String, String>>(&raw).ok())
        .unwrap_or_default()
}

fn write_fallback_map(map: &BTreeMap<String, String>) -> Result<(), String> {
    let path = fallback_vault_path();
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(map).map_err(|e| e.to_string())?;
    std::fs::write(&path, json).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600));
    }
    Ok(())
}

pub fn set_host_secret(host_id: &str, secret: &str) -> Result<(), String> {
    if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, host_id) {
        if entry.set_password(secret).is_ok() {
            return Ok(());
        }
    }
    let mut map = read_fallback_map();
    map.insert(host_id.to_string(), BASE64.encode(secret.as_bytes()));
    write_fallback_map(&map)
}

pub fn get_host_secret(host_id: &str) -> Result<Option<String>, String> {
    if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, host_id) {
        match entry.get_password() {
            Ok(pw) => return Ok(Some(pw)),
            Err(keyring::Error::NoEntry) => {}
            Err(_) => {}
        }
    }
    let map = read_fallback_map();
    if let Some(encoded) = map.get(host_id) {
        if let Ok(bytes) = BASE64.decode(encoded) {
            if let Ok(decoded) = String::from_utf8(bytes) {
                return Ok(Some(decoded));
            }
        }
    }
    Ok(None)
}

pub fn delete_host_secret(host_id: &str) -> Result<(), String> {
    if let Ok(entry) = keyring::Entry::new(SERVICE_NAME, host_id) {
        let _ = entry.delete_credential();
    }
    let mut map = read_fallback_map();
    if map.remove(host_id).is_some() {
        let _ = write_fallback_map(&map);
    }
    Ok(())
}

pub fn set_secret(host_id: &str, secret: &str) -> Result<(), String> {
    set_host_secret(host_id, secret)
}

pub fn get_secret(host_id: &str) -> Result<Option<String>, String> {
    get_host_secret(host_id)
}

pub fn delete_secret(host_id: &str) -> Result<(), String> {
    delete_host_secret(host_id)
}
