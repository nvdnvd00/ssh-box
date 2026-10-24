use crate::host_store;
use crate::ssh_pool::SshPool;
use russh::ChannelMsg;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::{Instant, UNIX_EPOCH};

fn local_command(program: &str) -> Command {
    #[cfg_attr(not(windows), allow(unused_mut))]
    let mut cmd = Command::new(program);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        // CREATE_NO_WINDOW: GUI-subsystem app otherwise pops a console per spawn
        cmd.creation_flags(0x0800_0000);
    }
    cmd
}

fn local_permissions_mode(meta: &fs::Metadata, is_dir: bool) -> u32 {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = is_dir;
        meta.permissions().mode() & 0o777
    }
    #[cfg(not(unix))]
    {
        if meta.permissions().readonly() {
            if is_dir { 0o555 } else { 0o444 }
        } else if is_dir {
            0o755
        } else {
            0o644
        }
    }
}

fn set_local_permissions_mode(path: &Path, mode: u32) -> Result<(), String> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = fs::Permissions::from_mode(mode);
        fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to chmod '{}': {e}", path.display()))
    }
    #[cfg(not(unix))]
    {
        let meta = fs::metadata(path)
            .map_err(|e| format!("Failed to inspect '{}': {e}", path.display()))?;
        let mut perms = meta.permissions();
        perms.set_readonly((mode & 0o200) == 0);
        fs::set_permissions(path, perms)
            .map_err(|e| format!("Failed to update permissions on '{}': {e}", path.display()))
    }
}

pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<(), String> {
    fs::create_dir_all(dst)
        .map_err(|e| format!("Failed to create directory '{}': {e}", dst.display()))?;
    let entries = fs::read_dir(src)
        .map_err(|e| format!("Failed to read directory '{}': {e}", src.display()))?;
    for entry in entries {
        let entry = entry.map_err(|e| e.to_string())?;
        let file_type = entry.file_type().map_err(|e| e.to_string())?;
        let dest_path = dst.join(entry.file_name());
        if file_type.is_dir() {
            copy_dir_recursive(&entry.path(), &dest_path)?;
        } else {
            fs::copy(entry.path(), &dest_path)
                .map_err(|e| format!("Failed to copy file to '{}': {e}", dest_path.display()))?;
        }
    }
    Ok(())
}

const MAX_TEXT_FILE_BYTES: u64 = 2 * 1024 * 1024; // 2 MB safety limit for quick text editor
pub const SFTP_CHUNK_SIZE: usize = 65536; // 64 KB streaming chunk

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferProgress {
    pub transfer_id: String,
    pub file_name: String,
    pub direction: String,
    pub mode: String, // "fast_sync" | "chunk_stream" | "local_copy"
    pub mode_detail: String,
    pub bytes_transferred: u64,
    pub total_bytes: u64,
    pub speed_bytes_per_sec: u64,
    pub percent: u8,
    pub status: String, // "running" | "completed" | "failed"
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SftpTransferResult {
    pub transfer_id: String,
    pub file_name: String,
    pub mode: String,
    pub mode_detail: String,
    pub bytes_transferred: u64,
    pub duration_ms: u64,
    pub speed_bytes_per_sec: u64,
}

pub fn is_already_compressed(file_name: &str) -> bool {
    let lower = file_name.to_ascii_lowercase();
    [
        ".zip", ".gz", ".tgz", ".bz2", ".xz", ".zst", ".7z", ".rar", ".mp4", ".mkv", ".mov",
        ".avi", ".webm", ".jpg", ".jpeg", ".png", ".webp", ".gif", ".mp3", ".aac", ".flac",
        ".dmg", ".iso", ".deb", ".rpm", ".apk", ".ipa",
    ]
    .iter()
    .any(|ext| lower.ends_with(ext))
}

pub fn compute_local_dir_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_file() {
        return meta.len();
    }
    if !meta.is_dir() {
        return 0;
    }
    let mut sum = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            sum = sum.saturating_add(compute_local_dir_size(&entry.path()));
        }
    }
    sum
}

pub fn compute_local_file_sha256(path: &Path, max_bytes: u64) -> Option<String> {
    let meta = fs::metadata(path).ok()?;
    if meta.len() > max_bytes {
        return None;
    }
    let mut file = fs::File::open(path).ok()?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; SFTP_CHUNK_SIZE];
    loop {
        let n = file.read(&mut buf).ok()?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Some(format!("{:x}", hasher.finalize()))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SftpEntry {
    pub name: String,
    pub path: String,
    pub kind: String, // "dir" | "file" | "symlink"
    pub size: u64,
    pub modified: u64,
    pub permissions: String,
    pub mode_octal: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SftpListResponse {
    pub cwd: String,
    pub entries: Vec<SftpEntry>,
}

pub fn shell_quote(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

pub fn join_posix_path(base: &str, name: &str) -> String {
    if base.is_empty() || base == "/" {
        format!("/{}", name.trim_start_matches('/'))
    } else {
        format!(
            "{}/{}",
            base.trim_end_matches('/'),
            name.trim_start_matches('/')
        )
    }
}

pub fn format_unix_permissions(mode: u32, is_dir: bool, is_symlink: bool) -> String {
    let file_type = if is_symlink {
        'l'
    } else if is_dir {
        'd'
    } else {
        '-'
    };
    let flags = [
        (0o400, 'r'),
        (0o200, 'w'),
        (0o100, 'x'),
        (0o040, 'r'),
        (0o020, 'w'),
        (0o010, 'x'),
        (0o004, 'r'),
        (0o002, 'w'),
        (0o001, 'x'),
    ];
    let mut out = String::with_capacity(10);
    out.push(file_type);
    for (bit, ch) in flags {
        out.push(if (mode & bit) != 0 { ch } else { '-' });
    }
    out
}

pub fn sort_entries(entries: &mut [SftpEntry]) {
    entries.sort_by(|a, b| {
        let a_is_dir = a.kind == "dir";
        let b_is_dir = b.kind == "dir";
        match b_is_dir.cmp(&a_is_dir) {
            std::cmp::Ordering::Equal => a.name.to_lowercase().cmp(&b.name.to_lowercase()),
            other => other,
        }
    });
}

pub fn parse_remote_stat_listing(raw: &str, fallback_path: &str) -> SftpListResponse {
    let mut parts = raw.splitn(2, "---SSHBOX_ENTRIES---\n");
    let header = parts.next().unwrap_or("").trim();
    let body = parts.next().unwrap_or("");

    let cwd = header
        .lines()
        .last()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(if fallback_path.is_empty() {
            "/"
        } else {
            fallback_path
        })
        .to_string();

    let mut entries = Vec::new();
    for line in body.lines() {
        let trimmed = line.trim_end_matches('\r');
        if trimmed.is_empty() {
            continue;
        }
        let cols: Vec<&str> = trimmed.splitn(5, '\t').collect();
        if cols.len() < 5 {
            continue;
        }
        let permissions = cols[0].trim().to_string();
        let raw_octal = cols[1].trim().trim_start_matches('0');
        let mode_octal = if raw_octal.is_empty() {
            "0".to_string()
        } else {
            raw_octal.to_string()
        };
        let size = cols[2].trim().parse::<u64>().unwrap_or(0);
        let modified = cols[3].trim().parse::<u64>().unwrap_or(0);
        let raw_name = cols[4].trim_start_matches("./");
        if raw_name.is_empty() || raw_name == "." || raw_name == ".." {
            continue;
        }
        let kind = if permissions.starts_with('d') {
            "dir"
        } else if permissions.starts_with('l') {
            "symlink"
        } else {
            "file"
        };

        entries.push(SftpEntry {
            name: raw_name.to_string(),
            path: join_posix_path(&cwd, raw_name),
            kind: kind.to_string(),
            size,
            modified,
            permissions,
            mode_octal,
        });
    }

    sort_entries(&mut entries);
    SftpListResponse { cwd, entries }
}

fn resolve_local_path(raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed == "~" {
        return crate::host_store::app_home_dir();
    }
    if let Some(stripped) = trimmed.strip_prefix("~/") {
        return crate::host_store::app_home_dir().join(stripped);
    }
    PathBuf::from(trimmed)
}

fn find_host_by_id(host_id: &str) -> Result<host_store::HostEntry, String> {
    let hosts = host_store::load_all_hosts();
    hosts
        .into_iter()
        .find(|h| h.id == host_id)
        .ok_or_else(|| format!("Host '{host_id}' not found"))
}

pub fn list_local_dir(path: &str) -> Result<SftpListResponse, String> {
    let target = resolve_local_path(path);
    let canonical = fs::canonicalize(&target).unwrap_or(target);
    let cwd = canonical.to_string_lossy().to_string();

    let read_dir =
        fs::read_dir(&canonical).map_err(|e| format!("Failed to read '{}': {e}", cwd))?;

    let mut entries = Vec::new();
    for item in read_dir {
        let Ok(entry) = item else { continue };
        let name = entry.file_name().to_string_lossy().to_string();
        if name == "." || name == ".." {
            continue;
        }
        let full_path = entry.path();
        let sym_meta = fs::symlink_metadata(&full_path);
        let Ok(meta) = sym_meta else { continue };

        let file_type = meta.file_type();
        let is_symlink = file_type.is_symlink();
        let is_dir = if is_symlink {
            fs::metadata(&full_path).map(|m| m.is_dir()).unwrap_or(false)
        } else {
            file_type.is_dir()
        };

        let kind = if is_dir {
            "dir"
        } else if is_symlink {
            "symlink"
        } else {
            "file"
        };

        let mode = local_permissions_mode(&meta, is_dir);
        let permissions = format_unix_permissions(mode, is_dir, is_symlink);
        let mode_octal = format!("{:o}", mode & 0o777);
        let modified = meta
            .modified()
            .ok()
            .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
            .map(|d| d.as_secs())
            .unwrap_or(0);

        entries.push(SftpEntry {
            name,
            path: full_path.to_string_lossy().to_string(),
            kind: kind.to_string(),
            size: meta.len(),
            modified,
            permissions,
            mode_octal,
        });
    }

    sort_entries(&mut entries);
    Ok(SftpListResponse { cwd, entries })
}

pub async fn list_dir(
    pool: &SshPool,
    host_id: &str,
    path: &str,
) -> Result<SftpListResponse, String> {
    if host_id == "local" {
        return list_local_dir(path);
    }

    let host = find_host_by_id(host_id)?;
    let requested = path.trim();
    let effective_path = if requested.is_empty() {
        if !host.default_sftp_path.trim().is_empty() {
            host.default_sftp_path.trim()
        } else {
            "~"
        }
    } else {
        requested
    };

    let cd_expr = if effective_path == "~" {
        "cd ~".to_string()
    } else if let Some(rest) = effective_path.strip_prefix("~/") {
        format!("cd ~/{}", shell_quote(rest))
    } else {
        format!("cd -- {}", shell_quote(effective_path))
    };

    let script = format!(
        "LC_ALL=C sh -c '\
        {cd_expr} || exit 1; \
        pwd; \
        printf \"---SSHBOX_ENTRIES---\\n\"; \
        set --; \
        for f in * .*; do \
          [ \"$f\" = \".\" ] || [ \"$f\" = \"..\" ] && continue; \
          [ -e \"$f\" ] || [ -L \"$f\" ] || continue; \
          set -- \"$@\" \"$f\"; \
        done; \
        if [ \"$#\" -gt 0 ]; then \
          if stat -c \"%A\" . >/dev/null 2>&1; then \
            stat -c \"%A\t%a\t%s\t%Y\t%n\" -- \"$@\"; \
          else \
            stat -f \"%Sp\t%Lp\t%z\t%m\t%N\" -- \"$@\"; \
          fi; \
        fi'"
    );

    let out_bytes = pool.exec_bytes_on_host(&host, &script, None).await?;
    let raw = String::from_utf8_lossy(&out_bytes);
    Ok(parse_remote_stat_listing(&raw, effective_path))
}

pub async fn mkdir(pool: &SshPool, host_id: &str, path: &str) -> Result<(), String> {
    if host_id == "local" {
        let target = resolve_local_path(path);
        return fs::create_dir_all(&target)
            .map_err(|e| format!("Failed to create directory '{}': {e}", target.display()));
    }
    let host = find_host_by_id(host_id)?;
    let cmd = format!("mkdir -p -- {}", shell_quote(path));
    pool.exec_bytes_on_host(&host, &cmd, None).await.map(|_| ())
}

pub async fn rename_entry(
    pool: &SshPool,
    host_id: &str,
    old_path: &str,
    new_path: &str,
) -> Result<(), String> {
    if host_id == "local" {
        let src = resolve_local_path(old_path);
        let dst = resolve_local_path(new_path);
        return fs::rename(&src, &dst).map_err(|e| {
            format!(
                "Failed to rename '{}' to '{}': {e}",
                src.display(),
                dst.display()
            )
        });
    }
    let host = find_host_by_id(host_id)?;
    let cmd = format!("mv -- {} {}", shell_quote(old_path), shell_quote(new_path));
    pool.exec_bytes_on_host(&host, &cmd, None).await.map(|_| ())
}

pub async fn delete_entry(pool: &SshPool, host_id: &str, path: &str) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() || trimmed == "/" {
        return Err("Refusing to delete root directory '/'".to_string());
    }
    if host_id == "local" {
        let target = resolve_local_path(trimmed);
        let meta = fs::symlink_metadata(&target)
            .map_err(|e| format!("Failed to inspect '{}': {e}", target.display()))?;
        if meta.file_type().is_dir() {
            return fs::remove_dir_all(&target)
                .map_err(|e| format!("Failed to remove directory '{}': {e}", target.display()));
        }
        return fs::remove_file(&target)
            .map_err(|e| format!("Failed to remove file '{}': {e}", target.display()));
    }
    let host = find_host_by_id(host_id)?;
    let cmd = format!("rm -rf -- {}", shell_quote(trimmed));
    pool.exec_bytes_on_host(&host, &cmd, None).await.map(|_| ())
}

pub async fn chmod_entry(
    pool: &SshPool,
    host_id: &str,
    path: &str,
    mode: &str,
) -> Result<(), String> {
    let clean_mode = mode.trim();
    if clean_mode.is_empty() || !clean_mode.chars().all(|c| ('0'..='7').contains(&c)) {
        return Err(format!(
            "Invalid octal permission mode '{clean_mode}' (expected digits 0-7, e.g. 755 or 0644)"
        ));
    }

    if host_id == "local" {
        let target = resolve_local_path(path);
        let parsed = u32::from_str_radix(clean_mode, 8)
            .map_err(|e| format!("Invalid octal mode '{clean_mode}': {e}"))?;
        return set_local_permissions_mode(&target, parsed);
    }

    let host = find_host_by_id(host_id)?;
    let cmd = format!("chmod {} -- {}", clean_mode, shell_quote(path));
    pool.exec_bytes_on_host(&host, &cmd, None).await.map(|_| ())
}

pub async fn read_text_file(pool: &SshPool, host_id: &str, path: &str) -> Result<String, String> {
    if host_id == "local" {
        let target = resolve_local_path(path);
        let meta = fs::metadata(&target)
            .map_err(|e| format!("Failed to inspect '{}': {e}", target.display()))?;
        if meta.len() > MAX_TEXT_FILE_BYTES {
            return Err(format!(
                "File is too large for quick text editor ({} bytes, max 2 MB)",
                meta.len()
            ));
        }
        let bytes = fs::read(&target)
            .map_err(|e| format!("Failed to read '{}': {e}", target.display()))?;
        return String::from_utf8(bytes)
            .map_err(|_| "File appears to be binary (invalid UTF-8)".to_string());
    }

    let host = find_host_by_id(host_id)?;
    let cmd = format!("head -c {} -- {}", MAX_TEXT_FILE_BYTES + 1, shell_quote(path));
    let bytes = pool.exec_bytes_on_host(&host, &cmd, None).await?;
    if bytes.len() as u64 > MAX_TEXT_FILE_BYTES {
        return Err("File is too large for quick text editor (max 2 MB)".to_string());
    }
    String::from_utf8(bytes).map_err(|_| "File appears to be binary (invalid UTF-8)".to_string())
}

pub async fn write_text_file(
    pool: &SshPool,
    host_id: &str,
    path: &str,
    content: &str,
) -> Result<(), String> {
    if host_id == "local" {
        let target = resolve_local_path(path);
        return fs::write(&target, content.as_bytes())
            .map_err(|e| format!("Failed to write '{}': {e}", target.display()));
    }

    let host = find_host_by_id(host_id)?;
    let cmd = format!("cat > {}", shell_quote(path));
    pool.exec_bytes_on_host(&host, &cmd, Some(content.as_bytes()))
        .await
        .map(|_| ())
}

async fn finish_exec_channel(
    channel: &mut russh::Channel<russh::client::Msg>,
) -> Result<(), String> {
    let _ = channel.eof().await;
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
            let err_text = String::from_utf8_lossy(&stderr_buf).trim().to_string();
            return Err(if err_text.is_empty() {
                format!("Remote transfer command exited with status {code}")
            } else {
                err_text
            });
        }
    }
    Ok(())
}

pub async fn transfer_entry(
    pool: &SshPool,
    source_host_id: &str,
    source_path: &str,
    target_host_id: &str,
    target_dir: &str,
) -> Result<SftpTransferResult, String> {
    transfer_entry_with_progress(
        pool,
        source_host_id,
        source_path,
        target_host_id,
        target_dir,
        true,
        |_| {},
    )
    .await
}

pub async fn transfer_entry_with_progress<F>(
    pool: &SshPool,
    source_host_id: &str,
    source_path: &str,
    target_host_id: &str,
    target_dir: &str,
    prefer_fast_sync: bool,
    mut on_progress: F,
) -> Result<SftpTransferResult, String>
where
    F: FnMut(SftpTransferProgress),
{
    let transfer_id = format!("tx:{}", uuid::Uuid::new_v4());
    let started_at = Instant::now();

    if source_host_id == "local" && target_host_id == "local" {
        let src = resolve_local_path(source_path);
        let dst_dir = resolve_local_path(target_dir);
        let file_name = src
            .file_name()
            .ok_or_else(|| "Invalid source path".to_string())?
            .to_string_lossy()
            .to_string();
        let total_bytes = compute_local_dir_size(&src);
        let dst = dst_dir.join(&file_name);
        if src != dst {
            if src.is_dir() {
                copy_dir_recursive(&src, &dst)?;
            } else {
                fs::copy(&src, &dst).map_err(|e| format!("Failed to copy local file: {e}"))?;
            }
        }
        let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
        let speed = (total_bytes.saturating_mul(1000)) / duration_ms;
        on_progress(SftpTransferProgress {
            transfer_id: transfer_id.clone(),
            file_name: file_name.clone(),
            direction: "local".to_string(),
            mode: "local_copy".to_string(),
            mode_detail: "Local filesystem copy".to_string(),
            bytes_transferred: total_bytes,
            total_bytes,
            speed_bytes_per_sec: speed,
            percent: 100,
            status: "completed".to_string(),
            error_message: None,
        });
        return Ok(SftpTransferResult {
            transfer_id,
            file_name,
            mode: "local_copy".to_string(),
            mode_detail: "Local filesystem copy".to_string(),
            bytes_transferred: total_bytes,
            duration_ms,
            speed_bytes_per_sec: speed,
        });
    }

    // Upload: Local -> Remote (Streaming 64KB chunks + Auto Fast Sync)
    if source_host_id == "local" && target_host_id != "local" {
        let src = resolve_local_path(source_path);
        let host = find_host_by_id(target_host_id)?;
        let file_name = src
            .file_name()
            .ok_or_else(|| "Invalid local source file name".to_string())?
            .to_string_lossy()
            .to_string();
        let total_bytes = compute_local_dir_size(&src).max(1);

        if src.is_dir() {
            let parent = src.parent().unwrap_or_else(|| Path::new("/"));
            let has_remote_rsync = if prefer_fast_sync {
                pool.exec_on_host(
                    &host,
                    "command -v rsync >/dev/null 2>&1 && printf 'YES' || printf 'NO'",
                )
                .await
                .map(|s| s.trim() == "YES")
                .unwrap_or(false)
            } else {
                false
            };

            let (mode, mode_detail, tar_args, remote_cmd) = if prefer_fast_sync {
                let detail = if has_remote_rsync {
                    "Fast Sync (rsync + tar-gz 64KB stream)"
                } else {
                    "Fast Sync (tar-gz 64KB stream)"
                };
                let rcmd = if has_remote_rsync {
                    let stage_dir = format!("/tmp/.sshbox_sync_{}", uuid::Uuid::new_v4());
                    format!(
                        "mkdir -p -- {stage} {dst} && tar -xzf - -C {stage} && rsync -a --inplace {stage}/{fname}/ {dst}/{fname}/ && rm -rf -- {stage}",
                        stage = shell_quote(&stage_dir),
                        dst = shell_quote(target_dir),
                        fname = shell_quote(&file_name)
                    )
                } else {
                    format!(
                        "mkdir -p -- {} && tar -xzf - -C {}",
                        shell_quote(target_dir),
                        shell_quote(target_dir)
                    )
                };
                (
                    "fast_sync".to_string(),
                    detail.to_string(),
                    vec!["-czf", "-", "-C"],
                    rcmd,
                )
            } else {
                (
                    "chunk_stream".to_string(),
                    "Chunk Stream (64KB tar)".to_string(),
                    vec!["-cf", "-", "-C"],
                    format!(
                        "mkdir -p -- {} && tar -xf - -C {}",
                        shell_quote(target_dir),
                        shell_quote(target_dir)
                    ),
                )
            };

            let mut child = local_command("tar")
                .args(&tar_args)
                .arg(&parent.to_string_lossy().to_string())
                .arg(&file_name)
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn local tar stream: {e}"))?;

            let mut stdout = child
                .stdout
                .take()
                .ok_or_else(|| "Failed to open local tar stdout".to_string())?;
            let mut channel = pool.open_exec_channel(&host, &remote_cmd).await?;

            let mut buf = vec![0u8; SFTP_CHUNK_SIZE];
            let mut sent_bytes = 0u64;
            loop {
                let n = stdout
                    .read(&mut buf)
                    .map_err(|e| format!("Failed reading local tar stream: {e}"))?;
                if n == 0 {
                    break;
                }
                channel
                    .data(&buf[..n])
                    .await
                    .map_err(|e| format!("Failed streaming chunk to remote host: {e}"))?;
                sent_bytes = sent_bytes.saturating_add(n as u64);
                let elapsed_ms = started_at.elapsed().as_millis().max(1) as u64;
                let speed = (sent_bytes.saturating_mul(1000)) / elapsed_ms;
                let pct = ((sent_bytes.saturating_mul(100)) / total_bytes).min(99) as u8;
                on_progress(SftpTransferProgress {
                    transfer_id: transfer_id.clone(),
                    file_name: file_name.clone(),
                    direction: "upload".to_string(),
                    mode: mode.clone(),
                    mode_detail: mode_detail.clone(),
                    bytes_transferred: sent_bytes,
                    total_bytes,
                    speed_bytes_per_sec: speed,
                    percent: pct,
                    status: "running".to_string(),
                    error_message: None,
                });
            }

            let _ = child.wait();
            finish_exec_channel(&mut channel).await?;

            let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
            let speed = (sent_bytes.saturating_mul(1000)) / duration_ms;
            on_progress(SftpTransferProgress {
                transfer_id: transfer_id.clone(),
                file_name: file_name.clone(),
                direction: "upload".to_string(),
                mode: mode.clone(),
                mode_detail: mode_detail.clone(),
                bytes_transferred: sent_bytes,
                total_bytes: sent_bytes.max(1),
                speed_bytes_per_sec: speed,
                percent: 100,
                status: "completed".to_string(),
                error_message: None,
            });
            return Ok(SftpTransferResult {
                transfer_id,
                file_name,
                mode,
                mode_detail,
                bytes_transferred: sent_bytes,
                duration_ms,
                speed_bytes_per_sec: speed,
            });
        } else {
            let remote_dest = join_posix_path(target_dir, &file_name);
            let mode_bits = fs::metadata(&src)
                .map(|m| local_permissions_mode(&m, false))
                .unwrap_or(0o644);

            // Fast Sync Step 1: Delta / SHA-256 unchanged check
            if prefer_fast_sync {
                if let Some(local_sha) = compute_local_file_sha256(&src, 64 * 1024 * 1024) {
                    let probe_cmd = format!(
                        "if [ -f {p} ]; then (sha256sum {p} 2>/dev/null || shasum -a 256 {p} 2>/dev/null) | awk '{{print $1}}'; fi",
                        p = shell_quote(&remote_dest)
                    );
                    if let Ok(remote_sha) = pool.exec_on_host(&host, &probe_cmd).await {
                        if remote_sha.trim().eq_ignore_ascii_case(&local_sha) {
                            let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
                            let detail = "Fast Sync (rsync delta: unchanged - skipped)".to_string();
                            on_progress(SftpTransferProgress {
                                transfer_id: transfer_id.clone(),
                                file_name: file_name.clone(),
                                direction: "upload".to_string(),
                                mode: "fast_sync".to_string(),
                                mode_detail: detail.clone(),
                                bytes_transferred: total_bytes,
                                total_bytes,
                                speed_bytes_per_sec: (total_bytes.saturating_mul(1000))
                                    / duration_ms,
                                percent: 100,
                                status: "completed".to_string(),
                                error_message: None,
                            });
                            return Ok(SftpTransferResult {
                                transfer_id,
                                file_name,
                                mode: "fast_sync".to_string(),
                                mode_detail: detail,
                                bytes_transferred: total_bytes,
                                duration_ms,
                                speed_bytes_per_sec: (total_bytes.saturating_mul(1000))
                                    / duration_ms,
                            });
                        }
                    }
                }
            }

            let use_gzip_stream = prefer_fast_sync && !is_already_compressed(&file_name);
            let (mode, mode_detail, remote_cmd) = if use_gzip_stream {
                (
                    "fast_sync".to_string(),
                    "Fast Sync (zstream 64KB chunks)".to_string(),
                    format!(
                        "gzip -d > {dst} && chmod {:o} -- {dst}",
                        mode_bits,
                        dst = shell_quote(&remote_dest)
                    ),
                )
            } else if prefer_fast_sync {
                (
                    "fast_sync".to_string(),
                    "Fast Sync (direct 64KB stream)".to_string(),
                    format!(
                        "cat > {dst} && chmod {:o} -- {dst}",
                        mode_bits,
                        dst = shell_quote(&remote_dest)
                    ),
                )
            } else {
                (
                    "chunk_stream".to_string(),
                    "Chunk Stream (64KB)".to_string(),
                    format!(
                        "cat > {dst} && chmod {:o} -- {dst}",
                        mode_bits,
                        dst = shell_quote(&remote_dest)
                    ),
                )
            };

            let mut channel = pool.open_exec_channel(&host, &remote_cmd).await?;
            let mut sent_bytes = 0u64;
            let mut buf = vec![0u8; SFTP_CHUNK_SIZE];

            if use_gzip_stream {
                let mut child = local_command("gzip")
                    .args(["-1", "-c", &src.to_string_lossy()])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::piped())
                    .spawn()
                    .map_err(|e| format!("Failed to spawn gzip stream: {e}"))?;
                let mut stdout = child
                    .stdout
                    .take()
                    .ok_or_else(|| "Failed to read gzip stdout".to_string())?;
                loop {
                    let n = stdout
                        .read(&mut buf)
                        .map_err(|e| format!("Failed reading compressed stream: {e}"))?;
                    if n == 0 {
                        break;
                    }
                    channel
                        .data(&buf[..n])
                        .await
                        .map_err(|e| format!("Failed streaming chunk to remote host: {e}"))?;
                    sent_bytes = sent_bytes.saturating_add(n as u64);
                    let elapsed_ms = started_at.elapsed().as_millis().max(1) as u64;
                    let speed = (sent_bytes.saturating_mul(1000)) / elapsed_ms;
                    let pct = ((sent_bytes.saturating_mul(100)) / total_bytes).min(99) as u8;
                    on_progress(SftpTransferProgress {
                        transfer_id: transfer_id.clone(),
                        file_name: file_name.clone(),
                        direction: "upload".to_string(),
                        mode: mode.clone(),
                        mode_detail: mode_detail.clone(),
                        bytes_transferred: sent_bytes,
                        total_bytes,
                        speed_bytes_per_sec: speed,
                        percent: pct,
                        status: "running".to_string(),
                        error_message: None,
                    });
                }
                let _ = child.wait();
            } else {
                let mut file = fs::File::open(&src)
                    .map_err(|e| format!("Failed to open local file '{}': {e}", src.display()))?;
                loop {
                    let n = file
                        .read(&mut buf)
                        .map_err(|e| format!("Failed reading local file chunk: {e}"))?;
                    if n == 0 {
                        break;
                    }
                    channel
                        .data(&buf[..n])
                        .await
                        .map_err(|e| format!("Failed streaming chunk to remote host: {e}"))?;
                    sent_bytes = sent_bytes.saturating_add(n as u64);
                    let elapsed_ms = started_at.elapsed().as_millis().max(1) as u64;
                    let speed = (sent_bytes.saturating_mul(1000)) / elapsed_ms;
                    let pct = ((sent_bytes.saturating_mul(100)) / total_bytes).min(99) as u8;
                    on_progress(SftpTransferProgress {
                        transfer_id: transfer_id.clone(),
                        file_name: file_name.clone(),
                        direction: "upload".to_string(),
                        mode: mode.clone(),
                        mode_detail: mode_detail.clone(),
                        bytes_transferred: sent_bytes,
                        total_bytes,
                        speed_bytes_per_sec: speed,
                        percent: pct,
                        status: "running".to_string(),
                        error_message: None,
                    });
                }
            }

            finish_exec_channel(&mut channel).await?;

            let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
            let speed = (sent_bytes.saturating_mul(1000)) / duration_ms;
            on_progress(SftpTransferProgress {
                transfer_id: transfer_id.clone(),
                file_name: file_name.clone(),
                direction: "upload".to_string(),
                mode: mode.clone(),
                mode_detail: mode_detail.clone(),
                bytes_transferred: total_bytes,
                total_bytes,
                speed_bytes_per_sec: speed,
                percent: 100,
                status: "completed".to_string(),
                error_message: None,
            });
            return Ok(SftpTransferResult {
                transfer_id,
                file_name,
                mode,
                mode_detail,
                bytes_transferred: total_bytes,
                duration_ms,
                speed_bytes_per_sec: speed,
            });
        }
    }

    // Download: Remote -> Local (Streaming 64KB chunks + Auto Fast Sync)
    if source_host_id != "local" && target_host_id == "local" {
        let host = find_host_by_id(source_host_id)?;
        let dst_dir = resolve_local_path(target_dir);
        fs::create_dir_all(&dst_dir)
            .map_err(|e| format!("Failed to create local target directory: {e}"))?;

        let trimmed_src = source_path.trim_end_matches('/');
        let file_name = trimmed_src
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Invalid remote source path".to_string())?
            .to_string();
        let parent_dir = match trimmed_src.rfind('/') {
            Some(0) | None => "/",
            Some(idx) => &trimmed_src[..idx],
        };

        let probe_cmd = format!(
            "if [ -d {p} ]; then printf 'DIR\\t'; (du -sb {p} 2>/dev/null | awk '{{print $1}}') || printf '0'; else printf 'FILE\\t'; (stat -c '%s' {p} 2>/dev/null || stat -f '%z' {p} 2>/dev/null || printf '0'); fi",
            p = shell_quote(trimmed_src)
        );
        let probe_out = pool.exec_on_host(&host, &probe_cmd).await?;
        let mut parts = probe_out.trim().splitn(2, '\t');
        let is_dir = parts.next().unwrap_or("FILE").trim() == "DIR";
        let total_bytes = parts
            .next()
            .and_then(|s| s.lines().next())
            .and_then(|s| s.trim().parse::<u64>().ok())
            .unwrap_or(0)
            .max(1);

        if is_dir {
            let (mode, mode_detail, remote_tar_cmd, local_tar_flags) = if prefer_fast_sync {
                (
                    "fast_sync".to_string(),
                    "Fast Sync (tar-gz 64KB stream)".to_string(),
                    format!(
                        "tar -czf - -C {} {}",
                        shell_quote(parent_dir),
                        shell_quote(&file_name)
                    ),
                    vec!["-xzf", "-", "-C"],
                )
            } else {
                (
                    "chunk_stream".to_string(),
                    "Chunk Stream (64KB tar)".to_string(),
                    format!(
                        "tar -cf - -C {} {}",
                        shell_quote(parent_dir),
                        shell_quote(&file_name)
                    ),
                    vec!["-xf", "-", "-C"],
                )
            };

            let mut channel = pool.open_exec_channel(&host, &remote_tar_cmd).await?;
            let mut child = local_command("tar")
                .args(&local_tar_flags)
                .arg(&dst_dir.to_string_lossy().to_string())
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn local tar extractor: {e}"))?;

            let mut stdin = child
                .stdin
                .take()
                .ok_or_else(|| "Failed to open local tar stdin".to_string())?;
            let mut received_bytes = 0u64;
            let mut stderr_buf = Vec::new();
            let mut exit_code: Option<u32> = None;

            while let Some(msg) = channel.wait().await {
                match msg {
                    ChannelMsg::Data { data } => {
                        stdin
                            .write_all(&data)
                            .map_err(|e| format!("Failed writing to local tar stream: {e}"))?;
                        received_bytes = received_bytes.saturating_add(data.len() as u64);
                        let elapsed_ms = started_at.elapsed().as_millis().max(1) as u64;
                        let speed = (received_bytes.saturating_mul(1000)) / elapsed_ms;
                        let pct = ((received_bytes.saturating_mul(100)) / total_bytes).min(99) as u8;
                        on_progress(SftpTransferProgress {
                            transfer_id: transfer_id.clone(),
                            file_name: file_name.clone(),
                            direction: "download".to_string(),
                            mode: mode.clone(),
                            mode_detail: mode_detail.clone(),
                            bytes_transferred: received_bytes,
                            total_bytes,
                            speed_bytes_per_sec: speed,
                            percent: pct,
                            status: "running".to_string(),
                            error_message: None,
                        });
                    }
                    ChannelMsg::ExtendedData { data, .. } => stderr_buf.extend_from_slice(&data),
                    ChannelMsg::ExitStatus { exit_status } => exit_code = Some(exit_status),
                    ChannelMsg::Close => break,
                    _ => {}
                }
            }
            drop(stdin);
            let out = child
                .wait_with_output()
                .map_err(|e| format!("Failed waiting for local tar: {e}"))?;
            if let Some(code) = exit_code {
                if code != 0 {
                    return Err(format!(
                        "Remote archive failed: {}",
                        String::from_utf8_lossy(&stderr_buf)
                    ));
                }
            }
            if !out.status.success() {
                return Err(format!(
                    "Failed extracting downloaded folder: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }

            let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
            let speed = (received_bytes.saturating_mul(1000)) / duration_ms;
            on_progress(SftpTransferProgress {
                transfer_id: transfer_id.clone(),
                file_name: file_name.clone(),
                direction: "download".to_string(),
                mode: mode.clone(),
                mode_detail: mode_detail.clone(),
                bytes_transferred: received_bytes,
                total_bytes: received_bytes.max(1),
                speed_bytes_per_sec: speed,
                percent: 100,
                status: "completed".to_string(),
                error_message: None,
            });
            return Ok(SftpTransferResult {
                transfer_id,
                file_name,
                mode,
                mode_detail,
                bytes_transferred: received_bytes,
                duration_ms,
                speed_bytes_per_sec: speed,
            });
        } else {
            let local_dest = dst_dir.join(&file_name);

            // Fast Sync delta check on download
            if prefer_fast_sync && local_dest.exists() {
                if let Some(local_sha) = compute_local_file_sha256(&local_dest, 64 * 1024 * 1024) {
                    let sha_cmd = format!(
                        "(sha256sum {p} 2>/dev/null || shasum -a 256 {p} 2>/dev/null) | awk '{{print $1}}'",
                        p = shell_quote(trimmed_src)
                    );
                    if let Ok(remote_sha) = pool.exec_on_host(&host, &sha_cmd).await {
                        if remote_sha.trim().eq_ignore_ascii_case(&local_sha) {
                            let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
                            let detail = "Fast Sync (rsync delta: unchanged - skipped)".to_string();
                            on_progress(SftpTransferProgress {
                                transfer_id: transfer_id.clone(),
                                file_name: file_name.clone(),
                                direction: "download".to_string(),
                                mode: "fast_sync".to_string(),
                                mode_detail: detail.clone(),
                                bytes_transferred: total_bytes,
                                total_bytes,
                                speed_bytes_per_sec: (total_bytes.saturating_mul(1000))
                                    / duration_ms,
                                percent: 100,
                                status: "completed".to_string(),
                                error_message: None,
                            });
                            return Ok(SftpTransferResult {
                                transfer_id,
                                file_name,
                                mode: "fast_sync".to_string(),
                                mode_detail: detail,
                                bytes_transferred: total_bytes,
                                duration_ms,
                                speed_bytes_per_sec: (total_bytes.saturating_mul(1000))
                                    / duration_ms,
                            });
                        }
                    }
                }
            }

            let (mode, mode_detail) = if prefer_fast_sync {
                (
                    "fast_sync".to_string(),
                    "Fast Sync (direct 64KB stream)".to_string(),
                )
            } else {
                (
                    "chunk_stream".to_string(),
                    "Chunk Stream (64KB)".to_string(),
                )
            };

            let cat_cmd = format!("cat -- {}", shell_quote(trimmed_src));
            let mut channel = pool.open_exec_channel(&host, &cat_cmd).await?;
            let mut out_file = fs::File::create(&local_dest).map_err(|e| {
                format!(
                    "Failed to create downloaded file '{}': {e}",
                    local_dest.display()
                )
            })?;

            let mut received_bytes = 0u64;
            let mut stderr_buf = Vec::new();
            let mut exit_code: Option<u32> = None;

            while let Some(msg) = channel.wait().await {
                match msg {
                    ChannelMsg::Data { data } => {
                        out_file
                            .write_all(&data)
                            .map_err(|e| format!("Failed writing chunk to local file: {e}"))?;
                        received_bytes = received_bytes.saturating_add(data.len() as u64);
                        let elapsed_ms = started_at.elapsed().as_millis().max(1) as u64;
                        let speed = (received_bytes.saturating_mul(1000)) / elapsed_ms;
                        let pct = ((received_bytes.saturating_mul(100)) / total_bytes).min(99) as u8;
                        on_progress(SftpTransferProgress {
                            transfer_id: transfer_id.clone(),
                            file_name: file_name.clone(),
                            direction: "download".to_string(),
                            mode: mode.clone(),
                            mode_detail: mode_detail.clone(),
                            bytes_transferred: received_bytes,
                            total_bytes,
                            speed_bytes_per_sec: speed,
                            percent: pct,
                            status: "running".to_string(),
                            error_message: None,
                        });
                    }
                    ChannelMsg::ExtendedData { data, .. } => stderr_buf.extend_from_slice(&data),
                    ChannelMsg::ExitStatus { exit_status } => exit_code = Some(exit_status),
                    ChannelMsg::Close => break,
                    _ => {}
                }
            }
            out_file
                .flush()
                .map_err(|e| format!("Failed flushing downloaded file: {e}"))?;
            if let Some(code) = exit_code {
                if code != 0 {
                    return Err(format!(
                        "Remote read failed: {}",
                        String::from_utf8_lossy(&stderr_buf)
                    ));
                }
            }

            let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
            let speed = (received_bytes.saturating_mul(1000)) / duration_ms;
            on_progress(SftpTransferProgress {
                transfer_id: transfer_id.clone(),
                file_name: file_name.clone(),
                direction: "download".to_string(),
                mode: mode.clone(),
                mode_detail: mode_detail.clone(),
                bytes_transferred: received_bytes,
                total_bytes: received_bytes.max(1),
                speed_bytes_per_sec: speed,
                percent: 100,
                status: "completed".to_string(),
                error_message: None,
            });
            return Ok(SftpTransferResult {
                transfer_id,
                file_name,
                mode,
                mode_detail,
                bytes_transferred: received_bytes,
                duration_ms,
                speed_bytes_per_sec: speed,
            });
        }
    }

    // Remote A -> Remote B direct chunk streaming bridge
    if source_host_id != "local" && target_host_id != "local" {
        let src_host = find_host_by_id(source_host_id)?;
        let dst_host = find_host_by_id(target_host_id)?;
        let trimmed_src = source_path.trim_end_matches('/');
        let file_name = trimmed_src
            .rsplit('/')
            .next()
            .filter(|s| !s.is_empty())
            .ok_or_else(|| "Invalid remote source path".to_string())?
            .to_string();
        let parent_dir = match trimmed_src.rfind('/') {
            Some(0) | None => "/",
            Some(idx) => &trimmed_src[..idx],
        };

        let (mode, mode_detail, src_cmd, dst_cmd) = if prefer_fast_sync {
            (
                "fast_sync".to_string(),
                "Fast Sync (Remote-to-Remote tar-gz bridge)".to_string(),
                format!(
                    "tar -czf - -C {} {}",
                    shell_quote(parent_dir),
                    shell_quote(&file_name)
                ),
                format!(
                    "mkdir -p -- {} && tar -xzf - -C {}",
                    shell_quote(target_dir),
                    shell_quote(target_dir)
                ),
            )
        } else {
            (
                "chunk_stream".to_string(),
                "Chunk Stream (Remote-to-Remote 64KB)".to_string(),
                format!(
                    "tar -cf - -C {} {}",
                    shell_quote(parent_dir),
                    shell_quote(&file_name)
                ),
                format!(
                    "mkdir -p -- {} && tar -xf - -C {}",
                    shell_quote(target_dir),
                    shell_quote(target_dir)
                ),
            )
        };

        let mut src_channel = pool.open_exec_channel(&src_host, &src_cmd).await?;
        let mut dst_channel = pool.open_exec_channel(&dst_host, &dst_cmd).await?;
        let mut bridged_bytes = 0u64;

        while let Some(msg) = src_channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => {
                    for chunk in data.chunks(SFTP_CHUNK_SIZE) {
                        dst_channel
                            .data(chunk)
                            .await
                            .map_err(|e| format!("Failed writing chunk to target host: {e}"))?;
                    }
                    bridged_bytes = bridged_bytes.saturating_add(data.len() as u64);
                    let elapsed_ms = started_at.elapsed().as_millis().max(1) as u64;
                    let speed = (bridged_bytes.saturating_mul(1000)) / elapsed_ms;
                    on_progress(SftpTransferProgress {
                        transfer_id: transfer_id.clone(),
                        file_name: file_name.clone(),
                        direction: "remote_to_remote".to_string(),
                        mode: mode.clone(),
                        mode_detail: mode_detail.clone(),
                        bytes_transferred: bridged_bytes,
                        total_bytes: bridged_bytes,
                        speed_bytes_per_sec: speed,
                        percent: 50,
                        status: "running".to_string(),
                        error_message: None,
                    });
                }
                ChannelMsg::Close | ChannelMsg::Eof => break,
                _ => {}
            }
        }

        finish_exec_channel(&mut dst_channel).await?;
        let duration_ms = started_at.elapsed().as_millis().max(1) as u64;
        let speed = (bridged_bytes.saturating_mul(1000)) / duration_ms;
        return Ok(SftpTransferResult {
            transfer_id,
            file_name,
            mode,
            mode_detail,
            bytes_transferred: bridged_bytes,
            duration_ms,
            speed_bytes_per_sec: speed,
        });
    }

    Err("Unsupported transfer direction".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_remote_stat_output_and_sorts_dirs_first() {
        let sample = "/var/www/app\n---SSHBOX_ENTRIES---\n\
-rw-r--r--\t644\t1420\t1700000100\tindex.ts\n\
drwxr-xr-x\t755\t4096\t1700000200\tsrc\n\
lrwxr-xr-x\t777\t12\t1700000300\tcurrent\n\
-rw-------\t600\t310\t1700000400\t.env.production\n";

        let res = parse_remote_stat_listing(sample, "/var/www");
        assert_eq!(res.cwd, "/var/www/app");
        assert_eq!(res.entries.len(), 4);

        // Directories must be sorted before files/symlinks
        assert_eq!(res.entries[0].name, "src");
        assert_eq!(res.entries[0].kind, "dir");
        assert_eq!(res.entries[0].path, "/var/www/app/src");
        assert_eq!(res.entries[0].mode_octal, "755");

        // Remaining entries sorted alphabetically case-insensitively
        assert_eq!(res.entries[1].name, ".env.production");
        assert_eq!(res.entries[1].kind, "file");
        assert_eq!(res.entries[1].mode_octal, "600");

        assert_eq!(res.entries[2].name, "current");
        assert_eq!(res.entries[2].kind, "symlink");

        assert_eq!(res.entries[3].name, "index.ts");
        assert_eq!(res.entries[3].size, 1420);
    }

    #[test]
    fn local_sftp_operations_roundtrip() {
        let pool = SshPool::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = std::env::temp_dir().join(format!("ssh-box-sftp-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp).unwrap();
        let tmp_str = tmp.to_string_lossy().to_string();

        rt.block_on(async {
            let sub = tmp.join("configs");
            mkdir(&pool, "local", &sub.to_string_lossy()).await.unwrap();

            let file_path = tmp.join("hello.txt");
            write_text_file(
                &pool,
                "local",
                &file_path.to_string_lossy(),
                "hello sftp world\n",
            )
            .await
            .unwrap();

            let read_back = read_text_file(&pool, "local", &file_path.to_string_lossy())
                .await
                .unwrap();
            assert_eq!(read_back, "hello sftp world\n");

            chmod_entry(&pool, "local", &file_path.to_string_lossy(), "750")
                .await
                .unwrap();

            let listing = list_dir(&pool, "local", &tmp_str).await.unwrap();
            assert_eq!(listing.entries.len(), 2);
            assert_eq!(listing.entries[0].name, "configs");
            assert_eq!(listing.entries[0].kind, "dir");
            assert_eq!(listing.entries[1].name, "hello.txt");
            #[cfg(unix)]
            assert_eq!(listing.entries[1].mode_octal, "750");
            #[cfg(not(unix))]
            assert_eq!(listing.entries[1].mode_octal, "644");

            let renamed_path = tmp.join("renamed.txt");
            rename_entry(
                &pool,
                "local",
                &file_path.to_string_lossy(),
                &renamed_path.to_string_lossy(),
            )
            .await
            .unwrap();

            delete_entry(&pool, "local", &renamed_path.to_string_lossy())
                .await
                .unwrap();
            let listing_after = list_dir(&pool, "local", &tmp_str).await.unwrap();
            assert_eq!(listing_after.entries.len(), 1);
        });

        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn detects_compressed_files_and_computes_sha256_and_progress() {
        assert!(is_already_compressed("archive.tar.gz"));
        assert!(is_already_compressed("video.MP4"));
        assert!(!is_already_compressed("app.tsx"));
        assert!(!is_already_compressed("server.log"));

        let pool = SshPool::default();
        let rt = tokio::runtime::Runtime::new().unwrap();
        let tmp = std::env::temp_dir().join(format!("ssh-box-fastsync-test-{}", uuid::Uuid::new_v4()));
        let src_dir = tmp.join("src");
        let dst_dir = tmp.join("dst");
        fs::create_dir_all(&src_dir).unwrap();
        fs::create_dir_all(&dst_dir).unwrap();

        let sample_file = src_dir.join("data.txt");
        fs::write(&sample_file, b"fast-sync-chunk-payload").unwrap();

        let sha = compute_local_file_sha256(&sample_file, 1024 * 1024).unwrap();
        assert_eq!(sha.len(), 64);
        assert_eq!(compute_local_dir_size(&src_dir), 23);

        rt.block_on(async {
            let mut events = Vec::new();
            let res = transfer_entry_with_progress(
                &pool,
                "local",
                &sample_file.to_string_lossy(),
                "local",
                &dst_dir.to_string_lossy(),
                true,
                |ev| events.push(ev),
            )
            .await
            .unwrap();
            assert_eq!(res.file_name, "data.txt");
            assert_eq!(res.bytes_transferred, 23);
            assert!(!events.is_empty());
            assert_eq!(events.last().unwrap().percent, 100);
        });

        let _ = fs::remove_dir_all(&tmp);
    }
}
