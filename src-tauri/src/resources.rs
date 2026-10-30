use crate::host_store::HostEntry;
use crate::vault;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn now_unix() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MachineGroup {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub created_date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MachineTag {
    pub id: String,
    pub name: String,
    pub color: String,
    #[serde(default)]
    pub created_date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IdentityItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub remark: String,
    pub user_name: String,
    #[serde(default)]
    pub ssh_key_id: Option<String>,
    #[serde(default)]
    pub has_password: bool,
    #[serde(default)]
    pub created_date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SshKeyItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub remark: String,
    pub key_type: String,
    pub public_key: String,
    pub private_key_path: String,
    #[serde(default)]
    pub fingerprint: String,
    #[serde(default)]
    pub has_passphrase: bool,
    #[serde(default)]
    pub created_date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ConnectionProxyProfile {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub remark: String,
    pub proxy_type: String, // "socks5" | "httpConnect"
    pub host: String,
    pub port: u16,
    #[serde(default = "default_proxy_auth")]
    pub authentication_method: String, // "none" | "usernamePassword"
    #[serde(default)]
    pub username: String,
    #[serde(default)]
    pub has_password: bool,
    #[serde(default)]
    pub created_date: u64,
}

fn default_proxy_auth() -> String {
    "none".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PortForwardingRule {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub remark: String,
    pub mode: String, // "local" | "remote" | "dynamic"
    pub machine_id: String,
    #[serde(default = "default_loopback")]
    pub local_host: String,
    pub local_port: u16,
    #[serde(default = "default_loopback")]
    pub remote_host: String,
    #[serde(default)]
    pub remote_port: u16,
    #[serde(default = "default_loopback")]
    pub target_host: String,
    #[serde(default)]
    pub target_port: u16,
    #[serde(default)]
    pub connection_proxy_profile_id: Option<String>,
    #[serde(default)]
    pub auto_start: bool,
    #[serde(default)]
    pub created_date: u64,
    #[serde(default)]
    pub last_started_date: Option<u64>,
    #[serde(default = "default_pf_status")]
    pub status: String, // "idle" | "running" | "stopped" | "failed"
    #[serde(default)]
    pub error_message: Option<String>,
}

fn default_loopback() -> String {
    "127.0.0.1".to_string()
}

fn default_pf_status() -> String {
    "idle".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetPackage {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub remark: String,
    #[serde(default)]
    pub created_date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetItem {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub remark: String,
    pub code: String,
    #[serde(default)]
    pub package_id: Option<String>,
    #[serde(default)]
    pub created_date: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SnippetExecutionRecord {
    pub id: String,
    pub snippet_id: String,
    pub snippet_name: String,
    pub machine_id: String,
    pub machine_name: String,
    pub execution_date: u64,
    pub status: String, // "finished" | "failed"
    pub result: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourcesStoreData {
    #[serde(default)]
    pub groups: Vec<MachineGroup>,
    #[serde(default)]
    pub tags: Vec<MachineTag>,
    #[serde(default)]
    pub identities: Vec<IdentityItem>,
    #[serde(default)]
    pub ssh_keys: Vec<SshKeyItem>,
    #[serde(default)]
    pub proxy_profiles: Vec<ConnectionProxyProfile>,
    #[serde(default)]
    pub port_forwarding_rules: Vec<PortForwardingRule>,
    #[serde(default)]
    pub snippet_packages: Vec<SnippetPackage>,
    #[serde(default)]
    pub snippets: Vec<SnippetItem>,
    #[serde(default)]
    pub snippet_history: Vec<SnippetExecutionRecord>,
}

pub fn default_resources_dir() -> PathBuf {
    crate::host_store::app_config_dir()
}

pub fn default_resources_path() -> PathBuf {
    default_resources_dir().join("resources.json")
}

pub fn default_keys_dir() -> PathBuf {
    default_resources_dir().join("keys")
}

pub fn load_resources_at(path: &Path) -> ResourcesStoreData {
    std::fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<ResourcesStoreData>(&raw).ok())
        .unwrap_or_default()
}

pub fn save_resources_at(path: &Path, data: &ResourcesStoreData) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
    }
    let json = serde_json::to_string_pretty(data).map_err(|e| e.to_string())?;
    std::fs::write(path, json).map_err(|e| e.to_string())
}

pub fn load_resources() -> ResourcesStoreData {
    load_resources_at(&default_resources_path())
}

pub fn upsert_group_at(path: &Path, mut group: MachineGroup) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    if group.id.trim().is_empty() {
        group.id = format!("grp:{}", uuid::Uuid::new_v4());
    }
    if group.created_date == 0 {
        group.created_date = now_unix();
    }
    if let Some(existing) = store.groups.iter_mut().find(|g| g.id == group.id) {
        *existing = group;
    } else {
        store.groups.push(group);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn delete_group_at(path: &Path, group_id: &str) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    store.groups.retain(|g| g.id != group_id);
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn upsert_tag_at(path: &Path, mut tag: MachineTag) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    if tag.id.trim().is_empty() {
        tag.id = format!("tag:{}", uuid::Uuid::new_v4());
    }
    if tag.created_date == 0 {
        tag.created_date = now_unix();
    }
    if let Some(existing) = store.tags.iter_mut().find(|t| t.id == tag.id) {
        *existing = tag;
    } else {
        store.tags.push(tag);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn delete_tag_at(path: &Path, tag_id: &str) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    store.tags.retain(|t| t.id != tag_id);
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn upsert_identity_at(
    path: &Path,
    mut identity: IdentityItem,
    password: Option<String>,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    if identity.id.trim().is_empty() {
        identity.id = format!("id:{}", uuid::Uuid::new_v4());
    }
    if identity.created_date == 0 {
        identity.created_date = now_unix();
    }
    if let Some(pw) = password {
        if !pw.trim().is_empty() {
            let _ = vault::set_secret(&format!("identity:{}", identity.id), pw.trim());
            identity.has_password = true;
        }
    }
    if let Some(existing) = store.identities.iter_mut().find(|i| i.id == identity.id) {
        if !identity.has_password && existing.has_password {
            identity.has_password = true;
        }
        *existing = identity;
    } else {
        store.identities.push(identity);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn delete_identity_at(path: &Path, identity_id: &str) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    let _ = vault::delete_secret(&format!("identity:{identity_id}"));
    store.identities.retain(|i| i.id != identity_id);
    save_resources_at(path, &store)?;
    Ok(store)
}

fn compute_ssh_key_fingerprint(pub_key_path: &Path) -> String {
    if let Ok(raw) = std::fs::read_to_string(pub_key_path) {
        if let Ok(pub_key) = ssh_key::PublicKey::from_openssh(raw.trim()) {
            return pub_key.fingerprint(ssh_key::HashAlg::Sha256).to_string();
        }
    }
    "SHA256:ssh-key".to_string()
}

pub fn generate_ssh_key_at(
    store_path: &Path,
    keys_dir: &Path,
    name: String,
    remark: String,
    key_type: String, // "ed25519" | "rsa" | "ecdsa"
    passphrase: Option<String>,
) -> Result<ResourcesStoreData, String> {
    std::fs::create_dir_all(keys_dir).map_err(|e| e.to_string())?;
    let id = format!("key:{}", uuid::Uuid::new_v4());
    let file_stem = id.replace(':', "_");
    let priv_path = keys_dir.join(&file_stem);
    let pub_path = keys_dir.join(format!("{file_stem}.pub"));

    let normalized_type = match key_type.to_lowercase().as_str() {
        "rsa" => "rsa",
        "ecdsa" => "ecdsa",
        _ => "ed25519",
    };

    let comment = if name.trim().is_empty() {
        "ssh-box"
    } else {
        name.trim()
    };

    let algorithm = match normalized_type {
        "rsa" => ssh_key::Algorithm::Rsa {
            hash: Some(ssh_key::HashAlg::Sha256),
        },
        "ecdsa" => ssh_key::Algorithm::Ecdsa {
            curve: ssh_key::EcdsaCurve::NistP256,
        },
        _ => ssh_key::Algorithm::Ed25519,
    };

    let mut rng = ssh_key::rand_core::OsRng;
    let mut private_key = ssh_key::PrivateKey::random(&mut rng, algorithm)
        .map_err(|e| format!("Failed to generate {normalized_type} SSH key: {e}"))?;
    private_key.set_comment(comment);

    let mut pub_key = private_key.public_key().clone();
    pub_key.set_comment(comment);
    let public_key = pub_key
        .to_openssh()
        .map_err(|e| format!("Failed to format public key: {e}"))?;
    let fingerprint = pub_key.fingerprint(ssh_key::HashAlg::Sha256).to_string();

    let pass_str = passphrase.as_deref().unwrap_or("").trim();
    let final_priv_key = if !pass_str.is_empty() {
        private_key
            .encrypt(&mut rng, pass_str.as_bytes())
            .map_err(|e| format!("Failed to encrypt private key: {e}"))?
    } else {
        private_key
    };

    let priv_openssh = final_priv_key
        .to_openssh(ssh_key::LineEnding::LF)
        .map_err(|e| format!("Failed to serialize private key: {e}"))?;

    std::fs::write(&priv_path, priv_openssh.as_str()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::write(&pub_path, format!("{public_key}\n")).map_err(|e| e.to_string())?;

    let has_passphrase = !pass_str.is_empty();
    if has_passphrase {
        let _ = vault::set_secret(&format!("sshkey:{id}"), pass_str);
    }

    let item = SshKeyItem {
        id,
        name: if name.trim().is_empty() {
            format!("{normalized_type}-key")
        } else {
            name.trim().to_string()
        },
        remark,
        key_type: normalized_type.to_string(),
        public_key,
        private_key_path: priv_path.to_string_lossy().to_string(),
        fingerprint,
        has_passphrase,
        created_date: now_unix(),
    };

    let mut store = load_resources_at(store_path);
    store.ssh_keys.push(item);
    save_resources_at(store_path, &store)?;
    Ok(store)
}

pub fn import_ssh_key_at(
    store_path: &Path,
    keys_dir: &Path,
    key_id: Option<String>,
    name: String,
    remark: String,
    private_key: String,
    public_key: Option<String>,
    passphrase: Option<String>,
) -> Result<ResourcesStoreData, String> {
    std::fs::create_dir_all(keys_dir).map_err(|e| e.to_string())?;
    let id = key_id
        .filter(|s| !s.trim().is_empty())
        .unwrap_or_else(|| format!("key:{}", uuid::Uuid::new_v4()));
    let file_stem = id.replace(':', "_");
    let priv_path = keys_dir.join(&file_stem);
    let pub_path = keys_dir.join(format!("{file_stem}.pub"));

    if !private_key.trim().is_empty() {
        let mut normalized_priv = private_key.replace("\r\n", "\n");
        if !normalized_priv.ends_with('\n') {
            normalized_priv.push('\n');
        }
        std::fs::write(&priv_path, &normalized_priv).map_err(|e| e.to_string())?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600));
        }
    }

    let derived_pub = if let Some(pk) = public_key.filter(|s| !s.trim().is_empty()) {
        std::fs::write(&pub_path, pk.trim()).ok();
        pk.trim().to_string()
    } else if let Ok(raw_priv) = std::fs::read_to_string(&priv_path) {
        if let Ok(parsed_priv) = ssh_key::PrivateKey::from_openssh(&raw_priv) {
            let maybe_decrypted = if parsed_priv.is_encrypted() {
                if let Some(pp) = passphrase.as_deref().filter(|s| !s.is_empty()) {
                    parsed_priv.decrypt(pp.as_bytes()).ok()
                } else {
                    Some(parsed_priv)
                }
            } else {
                Some(parsed_priv)
            };
            if let Some(pk_priv) = maybe_decrypted {
                if let Ok(s) = pk_priv.public_key().to_openssh() {
                    let _ = std::fs::write(&pub_path, &s);
                    s
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        } else {
            String::new()
        }
    } else {
        String::new()
    };

    let detected_type = if derived_pub.starts_with("ssh-ed25519") {
        "ed25519"
    } else if derived_pub.starts_with("ssh-rsa") {
        "rsa"
    } else if derived_pub.starts_with("ecdsa-") {
        "ecdsa"
    } else {
        "ed25519"
    };

    let fingerprint = if pub_path.exists() {
        compute_ssh_key_fingerprint(&pub_path)
    } else {
        "SHA256:imported".to_string()
    };

    let has_passphrase = passphrase
        .as_ref()
        .map(|s| !s.trim().is_empty())
        .unwrap_or(false);
    if let Some(pp) = passphrase {
        if !pp.trim().is_empty() {
            let _ = vault::set_secret(&format!("sshkey:{id}"), pp.trim());
        }
    }

    let mut store = load_resources_at(store_path);
    if let Some(existing) = store.ssh_keys.iter_mut().find(|k| k.id == id) {
        existing.name = name;
        existing.remark = remark;
        existing.private_key_path = priv_path.to_string_lossy().to_string();
        if !derived_pub.is_empty() {
            existing.public_key = derived_pub;
            existing.key_type = detected_type.to_string();
            existing.fingerprint = fingerprint;
        }
        if has_passphrase {
            existing.has_passphrase = true;
        }
    } else {
        store.ssh_keys.push(SshKeyItem {
            id,
            name,
            remark,
            key_type: detected_type.to_string(),
            public_key: derived_pub,
            private_key_path: priv_path.to_string_lossy().to_string(),
            fingerprint,
            has_passphrase,
            created_date: now_unix(),
        });
    }
    save_resources_at(store_path, &store)?;
    Ok(store)
}

pub fn delete_ssh_key_at(store_path: &Path, key_id: &str) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(store_path);
    if let Some(item) = store.ssh_keys.iter().find(|k| k.id == key_id) {
        let p = PathBuf::from(&item.private_key_path);
        let _ = std::fs::remove_file(&p);
        let _ = std::fs::remove_file(p.with_extension("pub"));
    }
    let _ = vault::delete_secret(&format!("sshkey:{key_id}"));
    store.ssh_keys.retain(|k| k.id != key_id);
    save_resources_at(store_path, &store)?;
    Ok(store)
}

pub fn upsert_proxy_profile_at(
    path: &Path,
    mut profile: ConnectionProxyProfile,
    password: Option<String>,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    if profile.id.trim().is_empty() {
        profile.id = format!("proxy:{}", uuid::Uuid::new_v4());
    }
    if profile.created_date == 0 {
        profile.created_date = now_unix();
    }
    if let Some(pw) = password {
        if !pw.trim().is_empty() {
            let _ = vault::set_secret(&format!("proxy:{}", profile.id), pw.trim());
            profile.has_password = true;
        }
    }
    if let Some(existing) = store.proxy_profiles.iter_mut().find(|p| p.id == profile.id) {
        if !profile.has_password && existing.has_password {
            profile.has_password = true;
        }
        *existing = profile;
    } else {
        store.proxy_profiles.push(profile);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn delete_proxy_profile_at(path: &Path, profile_id: &str) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    let _ = vault::delete_secret(&format!("proxy:{profile_id}"));
    store.proxy_profiles.retain(|p| p.id != profile_id);
    for rule in &mut store.port_forwarding_rules {
        if rule.connection_proxy_profile_id.as_deref() == Some(profile_id) {
            rule.connection_proxy_profile_id = None;
        }
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn upsert_port_forwarding_rule_at(
    path: &Path,
    mut rule: PortForwardingRule,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    if rule.id.trim().is_empty() {
        rule.id = format!("pf:{}", uuid::Uuid::new_v4());
    }
    if rule.created_date == 0 {
        rule.created_date = now_unix();
    }
    if rule.status.trim().is_empty() {
        rule.status = "idle".to_string();
    }
    if let Some(existing) = store
        .port_forwarding_rules
        .iter_mut()
        .find(|r| r.id == rule.id)
    {
        *existing = rule;
    } else {
        store.port_forwarding_rules.push(rule);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn delete_port_forwarding_rule_at(
    path: &Path,
    rule_id: &str,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    store.port_forwarding_rules.retain(|r| r.id != rule_id);
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn upsert_snippet_package_at(
    path: &Path,
    mut pkg: SnippetPackage,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    if pkg.id.trim().is_empty() {
        pkg.id = format!("spkg:{}", uuid::Uuid::new_v4());
    }
    if pkg.created_date == 0 {
        pkg.created_date = now_unix();
    }
    if let Some(existing) = store.snippet_packages.iter_mut().find(|p| p.id == pkg.id) {
        *existing = pkg;
    } else {
        store.snippet_packages.push(pkg);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn delete_snippet_package_at(
    path: &Path,
    package_id: &str,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    store.snippet_packages.retain(|p| p.id != package_id);
    // SwiftServer behavior: deleting a package does NOT delete snippets in it
    for snippet in &mut store.snippets {
        if snippet.package_id.as_deref() == Some(package_id) {
            snippet.package_id = None;
        }
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn upsert_snippet_at(
    path: &Path,
    mut snippet: SnippetItem,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    if snippet.id.trim().is_empty() {
        snippet.id = format!("snip:{}", uuid::Uuid::new_v4());
    }
    if snippet.created_date == 0 {
        snippet.created_date = now_unix();
    }
    if let Some(existing) = store.snippets.iter_mut().find(|s| s.id == snippet.id) {
        *existing = snippet;
    } else {
        store.snippets.push(snippet);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn delete_snippet_at(path: &Path, snippet_id: &str) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    store.snippets.retain(|s| s.id != snippet_id);
    save_resources_at(path, &store)?;
    Ok(store)
}

pub fn record_snippet_execution_at(
    path: &Path,
    record: SnippetExecutionRecord,
) -> Result<ResourcesStoreData, String> {
    let mut store = load_resources_at(path);
    store.snippet_history.insert(0, record);
    if store.snippet_history.len() > 200 {
        store.snippet_history.truncate(200);
    }
    save_resources_at(path, &store)?;
    Ok(store)
}

#[derive(Debug, Clone)]
pub struct ResolvedHostCredentials {
    pub user: String,
    pub secret: Option<String>,
    pub key_paths: Vec<(PathBuf, Option<String>)>,
}

pub fn resolve_host_credentials_from(
    store: &ResourcesStoreData,
    host: &HostEntry,
    password_override: Option<String>,
) -> ResolvedHostCredentials {
    let mut user = host.user.clone();
    let mut secret = password_override.or_else(|| vault::get_host_secret(&host.id).ok().flatten());
    let mut key_paths = Vec::new();

    if host.auth_method == "identity" {
        if let Some(ref id_ref) = host.identity_id {
            if let Some(identity) = store.identities.iter().find(|i| &i.id == id_ref) {
                if !identity.user_name.trim().is_empty() {
                    user = identity.user_name.clone();
                }
                if secret.is_none() {
                    secret = vault::get_secret(&format!("identity:{}", identity.id))
                        .ok()
                        .flatten();
                }
                if let Some(ref kid) = identity.ssh_key_id {
                    if let Some(kitem) = store.ssh_keys.iter().find(|k| &k.id == kid) {
                        let pass = vault::get_secret(&format!("sshkey:{}", kitem.id))
                            .ok()
                            .flatten();
                        key_paths.push((PathBuf::from(&kitem.private_key_path), pass));
                    }
                }
            }
        }
    }

    if let Some(ref kid) = host.ssh_key_id {
        if let Some(kitem) = store.ssh_keys.iter().find(|k| &k.id == kid) {
            let pass = vault::get_secret(&format!("sshkey:{}", kitem.id))
                .ok()
                .flatten();
            let p = PathBuf::from(&kitem.private_key_path);
            if !key_paths.iter().any(|(existing, _)| existing == &p) {
                key_paths.push((p, pass));
            }
        }
    }

    ResolvedHostCredentials {
        user,
        secret,
        key_paths,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedIdentityItem {
    pub identity: IdentityItem,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ExportedSshKeyItem {
    pub key: SshKeyItem,
    pub private_key: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub passphrase: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MachinesBundleSecrets {
    #[serde(default)]
    pub identities: Vec<ExportedIdentityItem>,
    #[serde(default)]
    pub ssh_keys: Vec<ExportedSshKeyItem>,
    #[serde(default)]
    pub host_secrets: std::collections::BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MachinesExportBundle {
    #[serde(rename = "type")]
    pub bundle_type: String,
    pub items: Vec<HostEntry>,
    #[serde(default)]
    pub has_identities: bool,
    #[serde(default)]
    pub has_ssh_keys: bool,
    #[serde(default)]
    pub encrypted: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secrets: Option<MachinesBundleSecrets>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_secrets: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportMachinesResult {
    pub hosts: Vec<HostEntry>,
    pub resources: ResourcesStoreData,
}

fn derive_backup_keys(passphrase: &str, salt: &[u8]) -> ([u8; 32], [u8; 32]) {
    use sha2::{Digest, Sha256};
    let mut key = [0u8; 32];
    let pass_bytes = passphrase.as_bytes();
    for round in 0..60_000u32 {
        let mut hasher = Sha256::new();
        hasher.update(key);
        hasher.update(salt);
        hasher.update(pass_bytes);
        hasher.update(round.to_le_bytes());
        key = hasher.finalize().into();
    }
    let enc_key: [u8; 32] = Sha256::new()
        .chain_update(key)
        .chain_update(b"ssh-box-enc-v1")
        .finalize()
        .into();
    let mac_key: [u8; 32] = Sha256::new()
        .chain_update(key)
        .chain_update(b"ssh-box-mac-v1")
        .finalize()
        .into();
    (enc_key, mac_key)
}

fn xor_keystream(enc_key: &[u8; 32], nonce: &[u8], data: &mut [u8]) {
    use sha2::{Digest, Sha256};
    for (block_idx, chunk) in data.chunks_mut(32).enumerate() {
        let block: [u8; 32] = Sha256::new()
            .chain_update(enc_key)
            .chain_update(nonce)
            .chain_update((block_idx as u64).to_le_bytes())
            .finalize()
            .into();
        for (b, k) in chunk.iter_mut().zip(block.iter()) {
            *b ^= *k;
        }
    }
}

pub fn encrypt_bundle_secrets(
    secrets: &MachinesBundleSecrets,
    passphrase: &str,
) -> Result<String, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha2::{Digest, Sha256};
    use ssh_key::rand_core::{OsRng, RngCore};

    let mut payload = serde_json::to_vec(secrets).map_err(|e| e.to_string())?;
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 16];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);

    let (enc_key, mac_key) = derive_backup_keys(passphrase, &salt);
    xor_keystream(&enc_key, &nonce, &mut payload);

    let tag: [u8; 32] = Sha256::new()
        .chain_update(mac_key)
        .chain_update(salt)
        .chain_update(nonce)
        .chain_update(&payload)
        .finalize()
        .into();

    let mut out = Vec::with_capacity(64 + payload.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&tag);
    out.extend_from_slice(&payload);
    Ok(format!("v1:{}", BASE64.encode(out)))
}

pub fn decrypt_bundle_secrets(
    encoded: &str,
    passphrase: &str,
) -> Result<MachinesBundleSecrets, String> {
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use sha2::{Digest, Sha256};

    let b64 = encoded
        .strip_prefix("v1:")
        .ok_or_else(|| "Unsupported encrypted backup format".to_string())?;
    let raw = BASE64
        .decode(b64.trim())
        .map_err(|_| "Corrupted encrypted backup payload".to_string())?;
    if raw.len() < 64 {
        return Err("Corrupted encrypted backup payload".to_string());
    }
    let salt = &raw[0..16];
    let nonce = &raw[16..32];
    let tag = &raw[32..64];
    let mut payload = raw[64..].to_vec();

    let (enc_key, mac_key) = derive_backup_keys(passphrase, salt);
    let expected_tag: [u8; 32] = Sha256::new()
        .chain_update(mac_key)
        .chain_update(salt)
        .chain_update(nonce)
        .chain_update(&payload)
        .finalize()
        .into();

    let diff = tag
        .iter()
        .zip(expected_tag.iter())
        .fold(0u8, |acc, (a, b)| acc | (a ^ b));
    if diff != 0 {
        return Err("Invalid backup passphrase".to_string());
    }

    xor_keystream(&enc_key, nonce, &mut payload);
    serde_json::from_slice(&payload).map_err(|e| format!("Failed to parse decrypted backup: {e}"))
}

pub fn export_machines_bundle_at(
    store_path: &Path,
    hosts: Vec<HostEntry>,
    include_identities: bool,
    include_ssh_keys: bool,
    passphrase: Option<String>,
) -> Result<MachinesExportBundle, String> {
    let store = load_resources_at(store_path);
    let mut exported_hosts = Vec::with_capacity(hosts.len());
    let mut identity_ids: Vec<String> = Vec::new();
    let mut ssh_key_ids: Vec<String> = Vec::new();
    let mut host_secrets = std::collections::BTreeMap::new();

    for mut h in hosts {
        h.source = crate::host_store::HostSource::Custom;
        let linked_identity = h
            .identity_id
            .as_ref()
            .and_then(|id| store.identities.iter().find(|i| &i.id == id));

        if include_identities {
            if let Some(ref id) = h.identity_id {
                if !identity_ids.contains(id) {
                    identity_ids.push(id.clone());
                }
            }
            if let Ok(Some(pw)) = vault::get_host_secret(&h.id) {
                if !pw.is_empty() {
                    host_secrets.insert(h.id.clone(), pw);
                }
            }
        } else {
            if h.auth_method == "identity" {
                h.auth_method = "passwordOrKey".to_string();
                if let Some(ident) = linked_identity {
                    if !ident.user_name.trim().is_empty() {
                        h.user = ident.user_name.clone();
                    }
                    if include_ssh_keys && h.ssh_key_id.is_none() {
                        h.ssh_key_id = ident.ssh_key_id.clone();
                    }
                }
            }
            h.identity_id = None;
        }

        if include_ssh_keys {
            if let Some(ref kid) = h.ssh_key_id {
                if !ssh_key_ids.contains(kid) {
                    ssh_key_ids.push(kid.clone());
                }
            }
        } else {
            h.ssh_key_id = None;
        }

        exported_hosts.push(h);
    }

    let mut identities = Vec::new();
    if include_identities {
        for id in &identity_ids {
            if let Some(mut ident) = store.identities.iter().find(|i| &i.id == id).cloned() {
                if include_ssh_keys {
                    if let Some(ref kid) = ident.ssh_key_id {
                        if !ssh_key_ids.contains(kid) {
                            ssh_key_ids.push(kid.clone());
                        }
                    }
                } else {
                    ident.ssh_key_id = None;
                }
                let password = vault::get_secret(&format!("identity:{}", ident.id))
                    .ok()
                    .flatten();
                identities.push(ExportedIdentityItem {
                    identity: ident,
                    password,
                });
            }
        }
    }

    let mut ssh_keys = Vec::new();
    if include_ssh_keys {
        for kid in &ssh_key_ids {
            if let Some(kitem) = store.ssh_keys.iter().find(|k| &k.id == kid).cloned() {
                let private_key = std::fs::read_to_string(&kitem.private_key_path)
                    .map_err(|e| format!("Failed to read SSH private key '{}': {e}", kitem.name))?;
                let key_pass = vault::get_secret(&format!("sshkey:{}", kitem.id))
                    .ok()
                    .flatten();
                ssh_keys.push(ExportedSshKeyItem {
                    key: kitem,
                    private_key,
                    passphrase: key_pass,
                });
            }
        }
    }

    let has_identities = !identities.is_empty() || !host_secrets.is_empty();
    let has_ssh_keys = !ssh_keys.is_empty();
    let secrets_obj = MachinesBundleSecrets {
        identities,
        ssh_keys,
        host_secrets,
    };

    let pass = passphrase.as_deref().unwrap_or("").trim();
    let (encrypted, secrets, encrypted_secrets) =
        if !pass.is_empty() && (has_identities || has_ssh_keys) {
            let enc = encrypt_bundle_secrets(&secrets_obj, pass)?;
            (true, None, Some(enc))
        } else if has_identities || has_ssh_keys {
            (false, Some(secrets_obj), None)
        } else {
            (false, None, None)
        };

    Ok(MachinesExportBundle {
        bundle_type: "ssh-box/machines".to_string(),
        items: exported_hosts,
        has_identities,
        has_ssh_keys,
        encrypted,
        secrets,
        encrypted_secrets,
    })
}

pub fn import_machines_bundle_at(
    ssh_cfg_path: &Path,
    custom_hosts_path: &Path,
    resources_path: &Path,
    keys_dir: &Path,
    bundle: MachinesExportBundle,
    selected_indices: Vec<usize>,
    include_identities: bool,
    include_ssh_keys: bool,
    passphrase: Option<String>,
) -> Result<ImportMachinesResult, String> {
    let want_identities = include_identities && bundle.has_identities;
    let want_ssh_keys = include_ssh_keys && bundle.has_ssh_keys;

    let secrets = if want_identities || want_ssh_keys {
        if bundle.encrypted {
            let pass = passphrase.as_deref().unwrap_or("").trim();
            if pass.is_empty() {
                return Err("Backup passphrase is required".to_string());
            }
            let enc = bundle
                .encrypted_secrets
                .as_deref()
                .ok_or_else(|| "Missing encrypted secrets payload".to_string())?;
            decrypt_bundle_secrets(enc, pass)?
        } else {
            bundle.secrets.unwrap_or_default()
        }
    } else {
        MachinesBundleSecrets::default()
    };

    let selected_hosts: Vec<HostEntry> = selected_indices
        .into_iter()
        .filter_map(|idx| bundle.items.get(idx).cloned())
        .collect();

    if want_ssh_keys {
        for exported_key in &secrets.ssh_keys {
            import_ssh_key_at(
                resources_path,
                keys_dir,
                Some(exported_key.key.id.clone()),
                exported_key.key.name.clone(),
                exported_key.key.remark.clone(),
                exported_key.private_key.clone(),
                Some(exported_key.key.public_key.clone()),
                exported_key.passphrase.clone(),
            )?;
        }
    }

    if want_identities {
        for exported_id in &secrets.identities {
            let mut ident = exported_id.identity.clone();
            if !want_ssh_keys {
                ident.ssh_key_id = None;
            }
            upsert_identity_at(resources_path, ident, exported_id.password.clone())?;
        }
    }

    let existing_hosts = crate::host_store::load_merged_hosts(ssh_cfg_path, custom_hosts_path);
    for mut host in selected_hosts {
        host.source = crate::host_store::HostSource::Custom;
        let existing = existing_hosts.iter().find(|h| h.id == host.id);

        if want_identities {
            if let Some(pw) = secrets.host_secrets.get(&host.id) {
                if !pw.trim().is_empty() {
                    let _ = vault::set_host_secret(&host.id, pw.trim());
                }
            }
        } else {
            if host.auth_method == "identity" {
                if let Some(ident_entry) = host
                    .identity_id
                    .as_ref()
                    .and_then(|id| secrets.identities.iter().find(|i| &i.identity.id == id))
                {
                    if !ident_entry.identity.user_name.trim().is_empty() {
                        host.user = ident_entry.identity.user_name.clone();
                    }
                    if want_ssh_keys && host.ssh_key_id.is_none() {
                        host.ssh_key_id = ident_entry.identity.ssh_key_id.clone();
                    }
                }
                host.auth_method = existing
                    .map(|ex| ex.auth_method.clone())
                    .unwrap_or_else(|| "passwordOrKey".to_string());
            }
            host.identity_id = existing.and_then(|ex| ex.identity_id.clone());
        }

        if !want_ssh_keys {
            host.ssh_key_id = existing.and_then(|ex| ex.ssh_key_id.clone());
        }

        crate::host_store::upsert_custom_host_at(custom_hosts_path, host)?;
    }

    Ok(ImportMachinesResult {
        hosts: crate::host_store::load_merged_hosts(ssh_cfg_path, custom_hosts_path),
        resources: load_resources_at(resources_path),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crud_resources_and_preserves_snippets_when_package_deleted() {
        let tmp_dir =
            std::env::temp_dir().join(format!("ssh-box-res-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&tmp_dir).unwrap();
        let store_path = tmp_dir.join("resources.json");
        let keys_dir = tmp_dir.join("keys");

        let s1 = upsert_group_at(
            &store_path,
            MachineGroup {
                id: "grp:prod".into(),
                name: "Production".into(),
                remark: "Core cluster".into(),
                created_date: 0,
            },
        )
        .unwrap();
        assert_eq!(s1.groups.len(), 1);

        let s2 = upsert_tag_at(
            &store_path,
            MachineTag {
                id: "tag:k8s".into(),
                name: "k8s".into(),
                color: "#38bdf8".into(),
                created_date: 0,
            },
        )
        .unwrap();
        assert_eq!(s2.tags.len(), 1);

        let s3 = upsert_snippet_package_at(
            &store_path,
            SnippetPackage {
                id: "pkg:ops".into(),
                name: "Ops".into(),
                remark: "Maintenance scripts".into(),
                created_date: 0,
            },
        )
        .unwrap();
        assert_eq!(s3.snippet_packages.len(), 1);

        let s4 = upsert_snippet_at(
            &store_path,
            SnippetItem {
                id: "snip:1".into(),
                name: "Check Disk".into(),
                remark: "df -h".into(),
                code: "df -h".into(),
                package_id: Some("pkg:ops".into()),
                created_date: 0,
            },
        )
        .unwrap();
        assert_eq!(s4.snippets.len(), 1);
        assert_eq!(s4.snippets[0].package_id.as_deref(), Some("pkg:ops"));

        // Deleting package keeps snippet with package_id = None (SwiftServer exact behavior)
        let s5 = delete_snippet_package_at(&store_path, "pkg:ops").unwrap();
        assert_eq!(s5.snippet_packages.len(), 0);
        assert_eq!(s5.snippets.len(), 1);
        assert_eq!(s5.snippets[0].package_id, None);

        // Generate real ed25519 SSH key via ssh-keygen
        let s6 = generate_ssh_key_at(
            &store_path,
            &keys_dir,
            "deploy-key".into(),
            "CI key".into(),
            "ed25519".into(),
            None,
        )
        .unwrap();
        assert_eq!(s6.ssh_keys.len(), 1);
        assert!(s6.ssh_keys[0].public_key.starts_with("ssh-ed25519 "));
        assert!(s6.ssh_keys[0].fingerprint.starts_with("SHA256:"));

        let _ = std::fs::remove_dir_all(&tmp_dir);
    }

    #[test]
    fn export_and_import_machines_with_separate_identity_and_ssh_key_options_and_passphrase() {
        let src_dir =
            std::env::temp_dir().join(format!("ssh-box-export-src-{}", uuid::Uuid::new_v4()));
        let dst_dir =
            std::env::temp_dir().join(format!("ssh-box-export-dst-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        crate::host_store::set_app_data_dir(src_dir.clone());

        let src_res_path = src_dir.join("resources.json");
        let src_keys_dir = src_dir.join("keys");

        let s_after_key = generate_ssh_key_at(
            &src_res_path,
            &src_keys_dir,
            "prod-ed25519".into(),
            "Primary key".into(),
            "ed25519".into(),
            Some("key-pass-123".into()),
        )
        .unwrap();
        let created_key = s_after_key.ssh_keys[0].clone();

        let ident_id = format!("id:{}", uuid::Uuid::new_v4());
        upsert_identity_at(
            &src_res_path,
            IdentityItem {
                id: ident_id.clone(),
                name: "DevOps Admin".into(),
                remark: "".into(),
                user_name: "devops".into(),
                ssh_key_id: Some(created_key.id.clone()),
                has_password: false,
                created_date: 0,
            },
            Some("ident-pw-456".into()),
        )
        .unwrap();

        let host = HostEntry {
            id: format!("custom:{}", uuid::Uuid::new_v4()),
            alias: "prod-node".into(),
            hostname: "10.20.30.40".into(),
            user: "root".into(),
            port: 22,
            auth_method: "identity".into(),
            identity_id: Some(ident_id.clone()),
            ssh_key_id: None,
            ..Default::default()
        };

        // 1. Export only SSH keys (without Identities): host should downgrade to passwordOrKey but keep SSH key from identity
        let keys_only_bundle = export_machines_bundle_at(
            &src_res_path,
            vec![host.clone()],
            false,
            true,
            None,
        )
        .unwrap();
        assert!(!keys_only_bundle.has_identities);
        assert!(keys_only_bundle.has_ssh_keys);
        assert_eq!(keys_only_bundle.items[0].auth_method, "passwordOrKey");
        assert_eq!(keys_only_bundle.items[0].user, "devops");
        assert_eq!(keys_only_bundle.items[0].identity_id, None);
        assert_eq!(
            keys_only_bundle.items[0].ssh_key_id.as_deref(),
            Some(created_key.id.as_str())
        );

        // 2. Export only Identities (without SSH keys): identity should strip ssh_key_id
        let id_only_bundle = export_machines_bundle_at(
            &src_res_path,
            vec![host.clone()],
            true,
            false,
            None,
        )
        .unwrap();
        assert!(id_only_bundle.has_identities);
        assert!(!id_only_bundle.has_ssh_keys);
        assert_eq!(
            id_only_bundle.secrets.as_ref().unwrap().identities[0]
                .identity
                .ssh_key_id,
            None
        );

        // 3. Export both with passphrase encryption and import into a separate destination directory (simulating Mac <-> Windows)
        let encrypted_bundle = export_machines_bundle_at(
            &src_res_path,
            vec![host.clone()],
            true,
            true,
            Some("backup-secret-999".into()),
        )
        .unwrap();
        assert!(encrypted_bundle.encrypted);
        assert!(encrypted_bundle.secrets.is_none());
        assert!(encrypted_bundle.encrypted_secrets.is_some());

        let dst_ssh_cfg = dst_dir.join("ssh_config");
        let dst_hosts_path = dst_dir.join("hosts.json");
        let dst_res_path = dst_dir.join("resources.json");
        let dst_keys_dir = dst_dir.join("win_or_mac_keys");

        // Wrong passphrase must fail before importing anything
        let bad_res = import_machines_bundle_at(
            &dst_ssh_cfg,
            &dst_hosts_path,
            &dst_res_path,
            &dst_keys_dir,
            encrypted_bundle.clone(),
            vec![0],
            true,
            true,
            Some("wrong-pass".into()),
        );
        assert!(bad_res.is_err());

        // Correct passphrase imports host, identity, and rewrites SSH key path to dst_keys_dir
        let imported = import_machines_bundle_at(
            &dst_ssh_cfg,
            &dst_hosts_path,
            &dst_res_path,
            &dst_keys_dir,
            encrypted_bundle,
            vec![0],
            true,
            true,
            Some("backup-secret-999".into()),
        )
        .unwrap();

        assert_eq!(imported.hosts.len(), 1);
        assert_eq!(imported.resources.identities.len(), 1);
        assert_eq!(imported.resources.ssh_keys.len(), 1);

        let imported_key = &imported.resources.ssh_keys[0];
        assert!(imported_key
            .private_key_path
            .starts_with(&dst_keys_dir.to_string_lossy().to_string()));
        assert!(PathBuf::from(&imported_key.private_key_path).exists());

        let resolved =
            resolve_host_credentials_from(&imported.resources, &imported.hosts[0], None);
        assert_eq!(resolved.user, "devops");
        assert_eq!(resolved.secret.as_deref(), Some("ident-pw-456"));
        assert_eq!(resolved.key_paths.len(), 1);
        assert_eq!(
            resolved.key_paths[0].0,
            PathBuf::from(&imported_key.private_key_path)
        );
        assert_eq!(resolved.key_paths[0].1.as_deref(), Some("key-pass-123"));

        let _ = std::fs::remove_dir_all(&src_dir);
        let _ = std::fs::remove_dir_all(&dst_dir);
    }
}
