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
}
