use crate::host_store;
use crate::ssh_pool::SshPool;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::UNIX_EPOCH;

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

pub async fn transfer_entry(
    pool: &SshPool,
    source_host_id: &str,
    source_path: &str,
    target_host_id: &str,
    target_dir: &str,
) -> Result<(), String> {
    if source_host_id == "local" && target_host_id == "local" {
        let src = resolve_local_path(source_path);
        let dst_dir = resolve_local_path(target_dir);
        let file_name = src
            .file_name()
            .ok_or_else(|| "Invalid source path".to_string())?;
        let dst = dst_dir.join(file_name);
        if src == dst {
            return Ok(());
        }
        if src.is_dir() {
            return copy_dir_recursive(&src, &dst);
        } else {
            fs::copy(&src, &dst)
                .map_err(|e| format!("Failed to copy local file: {e}"))?;
            return Ok(());
        }
    }

    // Upload: Local -> Remote
    if source_host_id == "local" && target_host_id != "local" {
        let src = resolve_local_path(source_path);
        let host = find_host_by_id(target_host_id)?;
        let file_name = src
            .file_name()
            .ok_or_else(|| "Invalid local source file name".to_string())?
            .to_string_lossy()
            .to_string();

        if src.is_dir() {
            let parent = src.parent().unwrap_or_else(|| Path::new("/"));
            let output = Command::new("tar")
                .args([
                    "-cf",
                    "-",
                    "-C",
                    &parent.to_string_lossy(),
                    &file_name,
                ])
                .output()
                .map_err(|e| format!("Failed to archive local directory '{}': {e}", src.display()))?;
            if !output.status.success() {
                return Err(format!(
                    "Local tar failed: {}",
                    String::from_utf8_lossy(&output.stderr)
                ));
            }
            let cmd = format!(
                "mkdir -p -- {} && tar -xf - -C {}",
                shell_quote(target_dir),
                shell_quote(target_dir)
            );
            pool.exec_bytes_on_host(&host, &cmd, Some(&output.stdout))
                .await?;
            return Ok(());
        } else {
            let bytes = fs::read(&src)
                .map_err(|e| format!("Failed to read local file '{}': {e}", src.display()))?;
            let remote_dest = join_posix_path(target_dir, &file_name);
            let mode = fs::metadata(&src)
                .map(|m| local_permissions_mode(&m, false))
                .unwrap_or(0o644);
            let cmd = format!(
                "cat > {} && chmod {:o} -- {}",
                shell_quote(&remote_dest),
                mode,
                shell_quote(&remote_dest)
            );
            pool.exec_bytes_on_host(&host, &cmd, Some(&bytes)).await?;
            return Ok(());
        }
    }

    // Download: Remote -> Local
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
            .ok_or_else(|| "Invalid remote source path".to_string())?;
        let parent_dir = match trimmed_src.rfind('/') {
            Some(0) | None => "/",
            Some(idx) => &trimmed_src[..idx],
        };

        let check_dir_cmd = format!(
            "if [ -d {} ]; then printf 'DIR'; else printf 'FILE'; fi",
            shell_quote(trimmed_src)
        );
        let kind_bytes = pool.exec_bytes_on_host(&host, &check_dir_cmd, None).await?;
        let is_dir = String::from_utf8_lossy(&kind_bytes).trim() == "DIR";

        if is_dir {
            let tar_cmd = format!(
                "tar -cf - -C {} {}",
                shell_quote(parent_dir),
                shell_quote(file_name)
            );
            let tar_bytes = pool.exec_bytes_on_host(&host, &tar_cmd, None).await?;
            let mut child = Command::new("tar")
                .args(["-xf", "-", "-C", &dst_dir.to_string_lossy()])
                .stdin(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to spawn local tar: {e}"))?;
            if let Some(mut stdin) = child.stdin.take() {
                stdin
                    .write_all(&tar_bytes)
                    .map_err(|e| format!("Failed to write tar stream locally: {e}"))?;
            }
            let out = child
                .wait_with_output()
                .map_err(|e| format!("Failed waiting for local tar: {e}"))?;
            if !out.status.success() {
                return Err(format!(
                    "Failed extracting downloaded folder: {}",
                    String::from_utf8_lossy(&out.stderr)
                ));
            }
            return Ok(());
        } else {
            let cat_cmd = format!("cat -- {}", shell_quote(trimmed_src));
            let file_bytes = pool.exec_bytes_on_host(&host, &cat_cmd, None).await?;
            let local_dest = dst_dir.join(file_name);
            fs::write(&local_dest, &file_bytes).map_err(|e| {
                format!(
                    "Failed to save downloaded file '{}': {e}",
                    local_dest.display()
                )
            })?;
            return Ok(());
        }
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
}
