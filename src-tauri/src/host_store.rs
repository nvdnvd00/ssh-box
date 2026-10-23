use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

static APP_DATA_DIR_OVERRIDE: OnceLock<PathBuf> = OnceLock::new();

pub fn set_app_data_dir(dir: PathBuf) {
    let _ = std::fs::create_dir_all(&dir);
    let _ = APP_DATA_DIR_OVERRIDE.set(dir);
}

pub fn app_home_dir() -> PathBuf {
    if let Some(dir) = APP_DATA_DIR_OVERRIDE.get() {
        let home = dir.join("home");
        let _ = std::fs::create_dir_all(&home);
        if cfg!(any(target_os = "android", target_os = "ios")) {
            return home;
        }
    }
    dirs::home_dir().unwrap_or_else(|| {
        let fallback = std::env::temp_dir().join("ssh-box-home");
        let _ = std::fs::create_dir_all(&fallback);
        fallback
    })
}

pub fn app_config_dir() -> PathBuf {
    if let Some(dir) = APP_DATA_DIR_OVERRIDE.get() {
        return dir.clone();
    }
    if let Some(cfg) = dirs::config_dir() {
        return cfg.join("ssh-box");
    }
    app_home_dir().join(".config").join("ssh-box")
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HostEntry {
    pub id: String,
    pub alias: String,
    pub hostname: String,
    pub user: String,
    pub port: u16,
    #[serde(default)]
    pub identity_file: Option<String>,
    #[serde(default)]
    pub group: String,
    pub source: HostSource,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "default_auth_method")]
    pub auth_method: String,
    #[serde(default)]
    pub identity_id: Option<String>,
    #[serde(default)]
    pub ssh_key_id: Option<String>,
    #[serde(default)]
    pub proxy_profile_id: Option<String>,
    #[serde(default)]
    pub jump_host_id: Option<String>,
    #[serde(default = "default_true")]
    pub show_status: bool,
    #[serde(default)]
    pub default_sftp_path: String,
    #[serde(default)]
    pub selected_network_interface: Option<String>,
    #[serde(default)]
    pub selected_volume_device: Option<String>,
    #[serde(default)]
    pub created_date: u64,
}

fn default_auth_method() -> String {
    "passwordOrKey".to_string()
}

fn default_true() -> bool {
    true
}

impl Default for HostEntry {
    fn default() -> Self {
        Self {
            id: String::new(),
            alias: String::new(),
            hostname: String::new(),
            user: "root".to_string(),
            port: 22,
            identity_file: None,
            group: "Production".to_string(),
            source: HostSource::Custom,
            remark: String::new(),
            tags: Vec::new(),
            auth_method: default_auth_method(),
            identity_id: None,
            ssh_key_id: None,
            proxy_profile_id: None,
            jump_host_id: None,
            show_status: true,
            default_sftp_path: String::new(),
            selected_network_interface: None,
            selected_volume_device: None,
            created_date: 0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum HostSource {
    SshConfig,
    Custom,
}

pub fn parse_ssh_config_str(input: &str) -> Vec<HostEntry> {
    let mut hosts = Vec::new();
    let mut current_aliases: Vec<String> = Vec::new();
    let mut fields: BTreeMap<String, String> = BTreeMap::new();

    let flush = |aliases: &mut Vec<String>,
                 map: &mut BTreeMap<String, String>,
                 out: &mut Vec<HostEntry>| {
        for alias in aliases.drain(..) {
            if alias.contains('*') || alias.contains('?') {
                continue;
            }
            let hostname = map
                .get("hostname")
                .cloned()
                .unwrap_or_else(|| alias.clone());
            let user = map
                .get("user")
                .cloned()
                .unwrap_or_else(|| {
                    std::env::var("USER")
                        .or_else(|_| std::env::var("USERNAME"))
                        .unwrap_or_else(|_| "root".into())
                });
            let port = map
                .get("port")
                .and_then(|p| p.parse::<u16>().ok())
                .unwrap_or(22);
            let identity_file = map.get("identityfile").cloned();
            out.push(HostEntry {
                id: format!("sshcfg:{alias}"),
                alias,
                hostname,
                user,
                port,
                identity_file,
                group: "~/.ssh/config".into(),
                source: HostSource::SshConfig,
                ..Default::default()
            });
        }
        map.clear();
    };

    for raw_line in input.lines() {
        let line = raw_line.split('#').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, |c: char| c.is_whitespace() || c == '=');
        let key = parts.next().unwrap_or("").trim().to_ascii_lowercase();
        let val = parts
            .next()
            .unwrap_or("")
            .trim()
            .trim_start_matches('=')
            .trim()
            .trim_matches('"');
        if key == "host" {
            flush(&mut current_aliases, &mut fields, &mut hosts);
            current_aliases = val.split_whitespace().map(|s| s.to_string()).collect();
        } else if !current_aliases.is_empty() && !val.is_empty() {
            fields.entry(key).or_insert_with(|| val.to_string());
        }
    }
    flush(&mut current_aliases, &mut fields, &mut hosts);
    hosts
}

pub fn load_merged_hosts(ssh_config_path: &Path, custom_json_path: &Path) -> Vec<HostEntry> {
    let mut result = Vec::new();
    if let Ok(raw) = std::fs::read_to_string(ssh_config_path) {
        result.extend(parse_ssh_config_str(&raw));
    }
    if let Ok(raw) = std::fs::read_to_string(custom_json_path) {
        if let Ok(custom) = serde_json::from_str::<Vec<HostEntry>>(&raw) {
            result.extend(custom);
        }
    }
    result
}

fn read_custom_hosts(custom_json_path: &Path) -> Vec<HostEntry> {
    std::fs::read_to_string(custom_json_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Vec<HostEntry>>(&raw).ok())
        .unwrap_or_default()
}

fn write_custom_hosts(custom_json_path: &Path, list: &[HostEntry]) -> Result<(), String> {
    if let Some(parent) = custom_json_path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(list).map_err(|e| e.to_string())?;
    std::fs::write(custom_json_path, json).map_err(|e| e.to_string())
}

pub fn upsert_custom_host_at(
    custom_json_path: &Path,
    mut entry: HostEntry,
) -> Result<Vec<HostEntry>, String> {
    entry.source = HostSource::Custom;
    if entry.id.trim().is_empty() {
        entry.id = format!("custom:{}", uuid::Uuid::new_v4());
    }
    if entry.group.trim().is_empty() {
        entry.group = "Custom Hosts".into();
    }
    if entry.created_date == 0 {
        entry.created_date = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
    }
    let mut list = read_custom_hosts(custom_json_path);
    if let Some(existing) = list.iter_mut().find(|h| h.id == entry.id) {
        *existing = entry;
    } else {
        list.push(entry);
    }
    write_custom_hosts(custom_json_path, &list)?;
    Ok(list)
}

pub fn delete_custom_host_at(
    custom_json_path: &Path,
    host_id: &str,
) -> Result<Vec<HostEntry>, String> {
    let mut list = read_custom_hosts(custom_json_path);
    list.retain(|h| h.id != host_id);
    write_custom_hosts(custom_json_path, &list)?;
    Ok(list)
}

pub fn default_paths() -> (PathBuf, PathBuf) {
    let home = app_home_dir();
    let ssh_cfg = home.join(".ssh").join("config");
    let custom_cfg = app_config_dir().join("hosts.json");
    (ssh_cfg, custom_cfg)
}

pub fn load_all_hosts() -> Vec<HostEntry> {
    let (ssh_cfg, custom_cfg) = default_paths();
    load_merged_hosts(&ssh_cfg, &custom_cfg)
}

pub fn upsert_custom_host(entry: HostEntry) -> Result<Vec<HostEntry>, String> {
    let (ssh_cfg, custom_cfg) = default_paths();
    upsert_custom_host_at(&custom_cfg, entry)?;
    Ok(load_merged_hosts(&ssh_cfg, &custom_cfg))
}

pub fn delete_custom_host(host_id: &str) -> Result<Vec<HostEntry>, String> {
    let (ssh_cfg, custom_cfg) = default_paths();
    delete_custom_host_at(&custom_cfg, host_id)?;
    Ok(load_merged_hosts(&ssh_cfg, &custom_cfg))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hosts_and_skips_wildcards() {
        let sample = r#"
            Host *
                ServerAliveInterval 60

            Host prod-api
                HostName 10.0.1.25
                User ubuntu
                Port 2222
                IdentityFile ~/.ssh/id_ed25519

            Host staging-db
                HostName db.internal
        "#;
        let parsed = parse_ssh_config_str(sample);
        assert_eq!(parsed.len(), 2);
        assert_eq!(parsed[0].alias, "prod-api");
        assert_eq!(parsed[0].hostname, "10.0.1.25");
        assert_eq!(parsed[0].user, "ubuntu");
        assert_eq!(parsed[0].port, 2222);
        assert_eq!(
            parsed[0].identity_file.as_deref(),
            Some("~/.ssh/id_ed25519")
        );
        assert_eq!(parsed[1].alias, "staging-db");
        assert_eq!(parsed[1].port, 22);
    }

    #[test]
    fn merges_ssh_config_and_upserts_custom_hosts_in_json() {
        let tmp_dir = std::env::temp_dir().join(format!("ssh-box-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let ssh_cfg = tmp_dir.join("config");
        let custom_json = tmp_dir.join("hosts.json");

        std::fs::write(
            &ssh_cfg,
            "Host edge-node\n  HostName 192.168.1.10\n  User admin\n",
        )
        .unwrap();

        let custom_host = HostEntry {
            id: "custom:1".into(),
            alias: "my-vps".into(),
            hostname: "203.0.113.5".into(),
            user: "root".into(),
            port: 22022,
            identity_file: None,
            group: "Production".into(),
            source: HostSource::Custom,
            ..Default::default()
        };

        upsert_custom_host_at(&custom_json, custom_host.clone()).unwrap();
        let merged = load_merged_hosts(&ssh_cfg, &custom_json);
        assert_eq!(merged.len(), 2);
        assert_eq!(merged[0].alias, "edge-node");
        assert_eq!(merged[1].alias, "my-vps");

        delete_custom_host_at(&custom_json, "custom:1").unwrap();
        let after_delete = load_merged_hosts(&ssh_cfg, &custom_json);
        assert_eq!(after_delete.len(), 1);
        assert_eq!(after_delete[0].alias, "edge-node");

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
