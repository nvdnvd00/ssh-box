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
            vault::set_secret(&format!("identity:{}", identity.id), pw.trim())?;
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

    // Vault first: if it fails, no key file is left behind without a store entry.
    let has_passphrase = !pass_str.is_empty();
    if has_passphrase {
        vault::set_secret(&format!("sshkey:{id}"), pass_str)?;
    }

    std::fs::write(&priv_path, priv_openssh.as_str()).map_err(|e| e.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = std::fs::set_permissions(&priv_path, std::fs::Permissions::from_mode(0o600));
    }
    std::fs::write(&pub_path, format!("{public_key}\n")).map_err(|e| e.to_string())?;

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
    // The id becomes a file name under keys_dir and can come from an imported file.
    if !id
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, ':' | '-' | '_'))
    {
        return Err(format!("Invalid SSH key id '{id}'"));
    }
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
            vault::set_secret(&format!("sshkey:{id}"), pp.trim())?;
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
            vault::set_secret(&format!("proxy:{}", profile.id), pw.trim())?;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub encrypted_secrets: Option<String>,
}

// v1 = PBKDF2-HMAC-SHA256 (OWASP 2023 iteration count) + AES-256-GCM: salt(16) | nonce(12) | ciphertext+tag.
// Unit tests only need a round trip and their bundles never leave the test, so they skip the cost.
const BACKUP_KDF_ROUNDS: u32 = if cfg!(test) { 1_000 } else { 600_000 };

fn backup_cipher(passphrase: &str, salt: &[u8]) -> aes_gcm::Aes256Gcm {
    use aes_gcm::KeyInit;
    let mut key = [0u8; 32];
    pbkdf2::pbkdf2_hmac::<sha2::Sha256>(passphrase.as_bytes(), salt, BACKUP_KDF_ROUNDS, &mut key);
    aes_gcm::Aes256Gcm::new(aes_gcm::Key::<aes_gcm::Aes256Gcm>::from_slice(&key))
}

pub fn encrypt_bundle_secrets(
    secrets: &MachinesBundleSecrets,
    passphrase: &str,
) -> Result<String, String> {
    use aes_gcm::aead::Aead;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    use ssh_key::rand_core::{OsRng, RngCore};

    let payload = serde_json::to_vec(secrets).map_err(|e| e.to_string())?;
    let mut salt = [0u8; 16];
    let mut nonce = [0u8; 12];
    OsRng.fill_bytes(&mut salt);
    OsRng.fill_bytes(&mut nonce);
    let ciphertext = backup_cipher(passphrase, &salt)
        .encrypt(aes_gcm::Nonce::from_slice(&nonce), payload.as_slice())
        .map_err(|_| "Failed to encrypt backup".to_string())?;

    let mut out = Vec::with_capacity(28 + ciphertext.len());
    out.extend_from_slice(&salt);
    out.extend_from_slice(&nonce);
    out.extend_from_slice(&ciphertext);
    Ok(format!("v1:{}", BASE64.encode(out)))
}

pub fn decrypt_bundle_secrets(
    encoded: &str,
    passphrase: &str,
) -> Result<MachinesBundleSecrets, String> {
    use aes_gcm::aead::Aead;
    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};

    let b64 = encoded
        .strip_prefix("v1:")
        .ok_or_else(|| "Unsupported encrypted backup format".to_string())?;
    let raw = BASE64
        .decode(b64.trim())
        .map_err(|_| "Corrupted encrypted backup payload".to_string())?;
    // salt + nonce + 16-byte GCM tag
    if raw.len() < 44 {
        return Err("Corrupted encrypted backup payload".to_string());
    }
    let payload = backup_cipher(passphrase, &raw[0..16])
        .decrypt(aes_gcm::Nonce::from_slice(&raw[16..28]), &raw[28..])
        .map_err(|_| "Invalid backup passphrase".to_string())?;
    serde_json::from_slice(&payload).map_err(|e| format!("Failed to parse decrypted backup: {e}"))
}

// Copies an identity's login user (and, if asked, its key link) onto a host that will
// not resolve the identity itself.
fn flatten_identity_into_host(host: &mut HostEntry, ident: &IdentityItem, with_key: bool) {
    if !ident.user_name.trim().is_empty() {
        host.user = ident.user_name.clone();
    }
    if with_key && host.ssh_key_id.is_none() {
        host.ssh_key_id = ident.ssh_key_id.clone();
    }
}

pub fn export_machines_bundle_at(
    store_path: &Path,
    hosts: Vec<HostEntry>,
    include_identities: bool,
    include_ssh_keys: bool,
    passphrase: Option<String>,
) -> Result<MachinesExportBundle, String> {
    let store = load_resources_at(store_path);
    let key_exists = |kid: &String| store.ssh_keys.iter().any(|k| &k.id == kid);
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

        if h.auth_method == "identity" {
            if let Some(ident) = linked_identity {
                // The login user is not a secret: carry it so the host keeps working
                // when identities are left out on either side.
                flatten_identity_into_host(&mut h, ident, include_ssh_keys && !include_identities);
            }
        }
        match linked_identity {
            Some(ident) if include_identities => {
                if !identity_ids.contains(&ident.id) {
                    identity_ids.push(ident.id.clone());
                }
            }
            _ => {
                if h.auth_method == "identity" {
                    h.auth_method = "passwordOrKey".to_string();
                }
                h.identity_id = None;
            }
        }

        if include_identities {
            if let Some(pw) = vault::get_host_secret(&h.id)? {
                if !pw.is_empty() {
                    host_secrets.insert(h.id.clone(), pw);
                }
            }
        }

        if !include_ssh_keys || !h.ssh_key_id.as_ref().is_some_and(key_exists) {
            h.ssh_key_id = None;
        }
        if let Some(ref kid) = h.ssh_key_id {
            if !ssh_key_ids.contains(kid) {
                ssh_key_ids.push(kid.clone());
            }
        }

        exported_hosts.push(h);
    }

    let mut identities = Vec::new();
    for ident in store
        .identities
        .iter()
        .filter(|i| identity_ids.contains(&i.id))
    {
        let mut ident = ident.clone();
        match ident.ssh_key_id {
            Some(ref kid) if include_ssh_keys && key_exists(kid) => {
                if !ssh_key_ids.contains(kid) {
                    ssh_key_ids.push(kid.clone());
                }
            }
            _ => ident.ssh_key_id = None,
        }
        let password = vault::get_secret(&format!("identity:{}", ident.id))?;
        if ident.has_password && password.is_none() {
            return Err(format!(
                "Password for identity '{}' could not be read from the vault",
                ident.name
            ));
        }
        identities.push(ExportedIdentityItem {
            identity: ident,
            password,
        });
    }

    let mut ssh_keys = Vec::new();
    for kitem in store
        .ssh_keys
        .iter()
        .filter(|k| ssh_key_ids.contains(&k.id))
    {
        let private_key = std::fs::read_to_string(&kitem.private_key_path)
            .map_err(|e| format!("Failed to read SSH private key '{}': {e}", kitem.name))?;
        let key_pass = vault::get_secret(&format!("sshkey:{}", kitem.id))?;
        if kitem.has_passphrase && key_pass.is_none() {
            return Err(format!(
                "Passphrase for SSH key '{}' could not be read from the vault",
                kitem.name
            ));
        }
        ssh_keys.push(ExportedSshKeyItem {
            key: kitem.clone(),
            private_key,
            passphrase: key_pass,
        });
    }

    let has_identities = !identities.is_empty() || !host_secrets.is_empty();
    let has_ssh_keys = !ssh_keys.is_empty();
    let encrypted_secrets = if has_identities || has_ssh_keys {
        let pass = passphrase.as_deref().unwrap_or("").trim();
        if pass.is_empty() {
            return Err("A passphrase is required to export credentials".to_string());
        }
        let secrets = MachinesBundleSecrets {
            identities,
            ssh_keys,
            host_secrets,
        };
        Some(encrypt_bundle_secrets(&secrets, pass)?)
    } else {
        None
    };

    Ok(MachinesExportBundle {
        bundle_type: "ssh-box/machines".to_string(),
        items: exported_hosts,
        has_identities,
        has_ssh_keys,
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
) -> Result<(), String> {
    let want_identities = include_identities && bundle.has_identities;
    let want_ssh_keys = include_ssh_keys && bundle.has_ssh_keys;

    let secrets = if want_identities || want_ssh_keys {
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
        MachinesBundleSecrets::default()
    };

    let selected_hosts: Vec<HostEntry> = selected_indices
        .into_iter()
        .filter_map(|idx| bundle.items.get(idx).cloned())
        .collect();

    // Only bring in credentials that the selected machines use.
    let used_identities: Vec<&ExportedIdentityItem> = secrets
        .identities
        .iter()
        .filter(|i| {
            selected_hosts
                .iter()
                .any(|h| h.identity_id.as_ref() == Some(&i.identity.id))
        })
        .collect();
    let key_used = |kid: &String| {
        selected_hosts
            .iter()
            .any(|h| h.ssh_key_id.as_ref() == Some(kid))
            || used_identities
                .iter()
                .any(|i| i.identity.ssh_key_id.as_ref() == Some(kid))
    };

    if want_ssh_keys {
        for exported_key in secrets.ssh_keys.iter().filter(|k| key_used(&k.key.id)) {
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
        let local = load_resources_at(resources_path);
        for exported_id in &used_identities {
            let mut ident = exported_id.identity.clone();
            if !want_ssh_keys {
                // Keys stay behind: keep the key this identity already uses here.
                ident.ssh_key_id = local
                    .identities
                    .iter()
                    .find(|i| i.id == ident.id)
                    .and_then(|i| i.ssh_key_id.clone());
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
                    vault::set_host_secret(&host.id, pw.trim())?;
                }
            }
        } else {
            if want_ssh_keys && host.auth_method == "identity" {
                if let Some(i) = used_identities
                    .iter()
                    .find(|i| host.identity_id.as_ref() == Some(&i.identity.id))
                {
                    flatten_identity_into_host(&mut host, &i.identity, true);
                }
            }
            // Same id already here: keep its local auth and identity link.
            match existing {
                Some(ex) => {
                    host.auth_method = ex.auth_method.clone();
                    host.identity_id = ex.identity_id.clone();
                }
                None => {
                    if host.auth_method == "identity" {
                        host.auth_method = "passwordOrKey".to_string();
                    }
                    host.identity_id = None;
                }
            }
        }

        if !want_ssh_keys {
            host.ssh_key_id = existing.and_then(|ex| ex.ssh_key_id.clone());
        }

        crate::host_store::upsert_custom_host_at(custom_hosts_path, host)?;
    }

    Ok(())
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
        // Mock keyring: secrets land in a vault file under the temp app dir, never the real Keychain.
        let _vault = crate::vault::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        let src_dir =
            std::env::temp_dir().join(format!("ssh-box-export-src-{}", uuid::Uuid::new_v4()));
        let dst_dir =
            std::env::temp_dir().join(format!("ssh-box-export-dst-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&src_dir).unwrap();
        std::fs::create_dir_all(&dst_dir).unwrap();
        crate::host_store::set_app_data_dir(src_dir.clone());

        let src_res_path = src_dir.join("resources.json");
        let src_keys_dir = src_dir.join("keys");
        let pass = Some("backup-secret-999".to_string());

        // Key ids become file names: an id from an imported file must not escape keys_dir.
        for bad_id in ["../../evil", "/tmp/evil"] {
            let res = import_ssh_key_at(
                &src_res_path,
                &src_keys_dir,
                Some(bad_id.into()),
                "evil".into(),
                "".into(),
                "not a key".into(),
                None,
                None,
            );
            assert!(res.is_err(), "{bad_id} must be rejected");
        }

        let created_key = generate_ssh_key_at(
            &src_res_path,
            &src_keys_dir,
            "prod-ed25519".into(),
            "Primary key".into(),
            "ed25519".into(),
            Some("key-pass-123".into()),
        )
        .unwrap()
        .ssh_keys[0]
            .clone();
        let other_key = generate_ssh_key_at(
            &src_res_path,
            &src_keys_dir,
            "other".into(),
            "".into(),
            "ed25519".into(),
            None,
        )
        .unwrap()
        .ssh_keys[1]
            .clone();

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
        let other_host = HostEntry {
            id: format!("custom:{}", uuid::Uuid::new_v4()),
            alias: "other-node".into(),
            auth_method: "passwordOrKey".into(),
            identity_id: None,
            ssh_key_id: Some(other_key.id.clone()),
            ..host.clone()
        };
        vault::set_host_secret(&host.id, "host-pw-789").unwrap();

        // Credentials never leave the machine unencrypted.
        assert!(
            export_machines_bundle_at(&src_res_path, vec![host.clone()], false, true, None)
                .is_err()
        );

        // 1. Keys only: host downgrades to passwordOrKey but keeps the identity's user and key.
        let keys_only_bundle =
            export_machines_bundle_at(&src_res_path, vec![host.clone()], false, true, pass.clone())
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

        // 2. Identities only: the identity loses its key link, and the login user still travels
        // in the clear so an import without identities logs in as the right user.
        let id_only_bundle =
            export_machines_bundle_at(&src_res_path, vec![host.clone()], true, false, pass.clone())
                .unwrap();
        assert!(id_only_bundle.has_identities);
        assert!(!id_only_bundle.has_ssh_keys);
        assert_eq!(id_only_bundle.items[0].user, "devops");
        let id_only_secrets = decrypt_bundle_secrets(
            id_only_bundle.encrypted_secrets.as_deref().unwrap(),
            "backup-secret-999",
        )
        .unwrap();
        assert_eq!(id_only_secrets.identities[0].identity.ssh_key_id, None);

        // 3. Both, two hosts, then import only the first into another "device".
        let full_bundle = export_machines_bundle_at(
            &src_res_path,
            vec![host.clone(), other_host.clone()],
            true,
            true,
            pass.clone(),
        )
        .unwrap();
        assert!(full_bundle.encrypted_secrets.is_some());

        // The vault is shared within this process: wipe the source secrets so the
        // assertions below can only pass if the import restored them.
        vault::delete_secret(&format!("identity:{ident_id}")).unwrap();
        vault::delete_secret(&format!("sshkey:{}", created_key.id)).unwrap();
        vault::delete_host_secret(&host.id).unwrap();

        let dst_ssh_cfg = dst_dir.join("ssh_config");
        let dst_hosts_path = dst_dir.join("hosts.json");
        let dst_res_path = dst_dir.join("resources.json");
        let dst_keys_dir = dst_dir.join("win_or_mac_keys");
        let import = |bundle: &MachinesExportBundle, ids: bool, keys: bool, pw: &str| {
            import_machines_bundle_at(
                &dst_ssh_cfg,
                &dst_hosts_path,
                &dst_res_path,
                &dst_keys_dir,
                bundle.clone(),
                vec![0],
                ids,
                keys,
                Some(pw.into()),
            )
        };

        assert!(import(&full_bundle, true, true, "wrong-pass").is_err());
        assert!(load_resources_at(&dst_res_path).ssh_keys.is_empty());

        import(&full_bundle, true, true, "backup-secret-999").unwrap();
        let hosts = crate::host_store::load_merged_hosts(&dst_ssh_cfg, &dst_hosts_path);
        let res = load_resources_at(&dst_res_path);
        assert_eq!(hosts.len(), 1);
        assert_eq!(res.identities.len(), 1);
        // other_host was not selected, so its key must stay behind.
        assert_eq!(res.ssh_keys.len(), 1);
        assert_eq!(res.ssh_keys[0].id, created_key.id);

        let imported_key = &res.ssh_keys[0];
        assert!(imported_key
            .private_key_path
            .starts_with(&dst_keys_dir.to_string_lossy().to_string()));
        assert!(PathBuf::from(&imported_key.private_key_path).exists());
        assert_eq!(
            vault::get_secret(&format!("identity:{ident_id}"))
                .unwrap()
                .as_deref(),
            Some("ident-pw-456")
        );

        let resolved = resolve_host_credentials_from(&res, &hosts[0], None);
        assert_eq!(resolved.user, "devops");
        assert_eq!(resolved.secret.as_deref(), Some("host-pw-789"));
        assert_eq!(resolved.key_paths.len(), 1);
        assert_eq!(
            resolved.key_paths[0].0,
            PathBuf::from(&imported_key.private_key_path)
        );
        assert_eq!(resolved.key_paths[0].1.as_deref(), Some("key-pass-123"));

        // 4. A keys-only re-import over that host keeps its local identity auth.
        import(&keys_only_bundle, false, true, "backup-secret-999").unwrap();
        let h = &crate::host_store::load_merged_hosts(&dst_ssh_cfg, &dst_hosts_path)[0];
        assert_eq!(h.auth_method, "identity");
        assert_eq!(h.identity_id.as_deref(), Some(ident_id.as_str()));

        // 5. An identities-only re-import keeps the key the identity already uses here.
        import(&id_only_bundle, true, false, "backup-secret-999").unwrap();
        assert_eq!(
            load_resources_at(&dst_res_path).identities[0]
                .ssh_key_id
                .as_deref(),
            Some(created_key.id.as_str())
        );

        // 6. Links to an identity or key that no longer exists are dropped, not exported dangling.
        let dangling = HostEntry {
            id: format!("custom:{}", uuid::Uuid::new_v4()),
            identity_id: Some("id:gone".into()),
            ssh_key_id: Some("key:gone".into()),
            ..host.clone()
        };
        let b = export_machines_bundle_at(&src_res_path, vec![dangling], true, true, pass).unwrap();
        assert_eq!(b.items[0].auth_method, "passwordOrKey");
        assert_eq!(b.items[0].identity_id, None);
        assert_eq!(b.items[0].ssh_key_id, None);

        let _ = std::fs::remove_dir_all(&src_dir);
        let _ = std::fs::remove_dir_all(&dst_dir);
    }

    #[test]
    fn failed_vault_write_is_reported_and_saves_nothing() {
        let _vault = crate::vault::TEST_LOCK
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        keyring::set_default_credential_builder(keyring::mock::default_credential_builder());
        let tmp_dir =
            std::env::temp_dir().join(format!("ssh-box-vault-fail-{}", uuid::Uuid::new_v4()));
        crate::host_store::set_app_data_dir(tmp_dir.clone());
        let store_path = tmp_dir.join("resources.json");
        let keys_dir = tmp_dir.join("keys");

        // A directory where the vault file belongs makes every vault write fail.
        let vault_path = crate::host_store::app_config_dir().join("vault.json");
        assert!(
            vault_path.starts_with(std::env::temp_dir()),
            "never touch a real vault"
        );
        let _ = std::fs::remove_file(&vault_path);
        std::fs::create_dir_all(&vault_path).unwrap();

        let key = generate_ssh_key_at(
            &store_path,
            &keys_dir,
            "k".into(),
            "".into(),
            "ed25519".into(),
            Some("key-pass".into()),
        );
        let key_files = std::fs::read_dir(&keys_dir).map(|d| d.count()).unwrap_or(0);
        let proxy = upsert_proxy_profile_at(
            &store_path,
            ConnectionProxyProfile {
                id: "proxy:p".into(),
                name: "p".into(),
                remark: "".into(),
                proxy_type: "socks5".into(),
                host: "127.0.0.1".into(),
                port: 1080,
                authentication_method: "usernamePassword".into(),
                username: "u".into(),
                has_password: false,
                created_date: 0,
            },
            Some("proxy-pw".into()),
        );
        std::fs::remove_dir(&vault_path).unwrap();

        // Saving a key or proxy whose secret was lost would show "has password" and then fail
        // to connect, so the save must fail; a key file without its store entry would be orphaned.
        assert!(key.is_err());
        assert_eq!(key_files, 0);
        assert!(proxy.is_err());
        let store = load_resources_at(&store_path);
        assert!(store.ssh_keys.is_empty());
        assert!(store.proxy_profiles.is_empty());
        let _ = std::fs::remove_dir_all(&tmp_dir);
    }
}
