use crate::frame_batcher::{batch_stream_to_sink, ChannelReader};
use serde::Serialize;
use std::collections::{BTreeMap, HashMap};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::net::{lookup_host, TcpStream};
use tokio::sync::mpsc;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
#[cfg(not(any(target_os = "ios", target_os = "android")))]
use std::io::Read;

#[cfg(not(any(target_os = "ios", target_os = "android")))]
pub struct NativePaneHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    shutdown_tx: mpsc::Sender<()>,
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
impl Drop for NativePaneHandle {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.try_send(());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

pub struct EmbeddedPaneHandle {
    input_tx: mpsc::UnboundedSender<Vec<u8>>,
    resize_tx: mpsc::UnboundedSender<(u16, u16)>,
    shutdown_tx: mpsc::Sender<()>,
}

impl Drop for EmbeddedPaneHandle {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.try_send(());
    }
}

pub enum LocalPaneBackend {
    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    Native(NativePaneHandle),
    Embedded(EmbeddedPaneHandle),
}

#[derive(Default, Clone)]
pub struct LocalPtyManager {
    panes: Arc<Mutex<HashMap<String, LocalPaneBackend>>>,
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn resolve_platform_shell(shell_override: Option<&str>) -> (String, Vec<String>) {
    if let Some(s) = shell_override {
        if let Some(distro) = s.strip_prefix("wsl:") {
            // --cd ~ opens in the Linux home instead of /mnt/c/Users/<user>
            return (
                "wsl.exe".to_string(),
                vec!["-d".into(), distro.into(), "--cd".into(), "~".into()],
            );
        }
        let args = if !cfg!(target_os = "windows") {
            vec!["-l".to_string()]
        } else if matches!(
            Path::new(s).file_stem().and_then(|f| f.to_str()),
            Some("powershell" | "pwsh")
        ) {
            vec!["-NoLogo".to_string()]
        } else {
            Vec::new()
        };
        return (s.to_string(), args);
    }
    if cfg!(target_os = "windows") {
        if let Ok(comspec) = std::env::var("COMSPEC") {
            if Path::new("C:\\Windows\\System32\\WindowsPowerShell\\v1.0\\powershell.exe").exists()
            {
                return (
                    "powershell.exe".to_string(),
                    vec!["-NoLogo".to_string()],
                );
            }
            return (comspec, Vec::new());
        }
        return ("powershell.exe".to_string(), vec!["-NoLogo".to_string()]);
    }
    if cfg!(target_os = "android") {
        if Path::new("/system/bin/sh").exists() {
            return ("/system/bin/sh".to_string(), Vec::new());
        }
        return ("/bin/sh".to_string(), Vec::new());
    }
    if let Ok(sh) = std::env::var("SHELL") {
        let trimmed = sh.trim();
        if !trimmed.is_empty() && Path::new(trimmed).exists() {
            return (trimmed.to_string(), vec!["-l".to_string()]);
        }
    }
    for candidate in ["/bin/zsh", "/bin/bash", "/bin/sh"] {
        if Path::new(candidate).exists() {
            return (candidate.to_string(), vec!["-l".to_string()]);
        }
    }
    ("/bin/sh".to_string(), vec!["-l".to_string()])
}

#[derive(Serialize)]
pub struct LocalShellOption {
    pub id: String,
    pub label: String,
}

/// Shells offered in Settings; `id` is passed back to spawn_pane as shell_override.
#[allow(unused_mut)]
pub async fn list_local_shells() -> Vec<LocalShellOption> {
    let mut out = Vec::new();
    #[cfg(target_os = "windows")]
    {
        let opt = |id: String, label: String| LocalShellOption { id, label };
        out.push(opt("powershell.exe".into(), "Windows PowerShell".into()));
        if let Some(pwsh) = std::env::var_os("PATH").and_then(|p| {
            std::env::split_paths(&p)
                .map(|d| d.join("pwsh.exe"))
                .find(|f| f.exists())
        }) {
            out.push(opt(pwsh.to_string_lossy().into(), "PowerShell 7".into()));
        }
        out.push(opt("cmd.exe".into(), "Command Prompt".into()));
        if let Ok(o) = tokio::process::Command::new("wsl.exe")
            .args(["-l", "-q"])
            // CREATE_NO_WINDOW: GUI-subsystem app otherwise pops a console per spawn
            .creation_flags(0x0800_0000)
            .output()
            .await
        {
            if o.status.success() {
                for d in parse_wsl_list(&o.stdout) {
                    out.push(opt(format!("wsl:{d}"), format!("WSL: {d}")));
                }
            }
        }
    }
    #[cfg(not(any(target_os = "windows", target_os = "ios", target_os = "android")))]
    {
        for line in fs::read_to_string("/etc/shells").unwrap_or_default().lines() {
            let p = line.trim();
            if p.starts_with('/') && Path::new(p).exists() {
                out.push(LocalShellOption { id: p.into(), label: p.into() });
            }
        }
    }
    out
}

/// `wsl.exe -l -q` prints UTF-16LE (UTF-8 only when WSL_UTF8=1 is set).
/// docker-desktop distros are Docker's internal VMs without a usable shell.
#[cfg(any(target_os = "windows", test))]
fn parse_wsl_list(raw: &[u8]) -> Vec<String> {
    let text = if raw.contains(&0) {
        let units: Vec<u16> = raw
            .chunks_exact(2)
            .map(|c| u16::from_le_bytes([c[0], c[1]]))
            .collect();
        String::from_utf16_lossy(&units)
    } else {
        String::from_utf8_lossy(raw).into_owned()
    };
    text.lines()
        .map(|l| l.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}'))
        .filter(|l| !l.is_empty() && !l.starts_with("docker-desktop"))
        .map(String::from)
        .collect()
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn build_native_shell_path(inherited_path: Option<&str>, home: &Path) -> String {
    let sep = if cfg!(target_os = "windows") { ";" } else { ":" };
    let mut entries: Vec<String> = Vec::new();
    let mut push_unique = |dir: String| {
        if !dir.is_empty() && !entries.iter().any(|e| e == &dir) {
            entries.push(dir);
        }
    };

    if !cfg!(target_os = "windows") {
        for user_rel in [".local/bin", ".orbstack/bin", ".bun/bin", ".cargo/bin"] {
            let p = home.join(user_rel);
            if p.exists() {
                push_unique(p.to_string_lossy().to_string());
            }
        }
        for sys_dir in [
            "/opt/homebrew/bin",
            "/opt/homebrew/sbin",
            "/usr/local/bin",
            "/usr/local/sbin",
        ] {
            if Path::new(sys_dir).exists() {
                push_unique(sys_dir.to_string());
            }
        }
    }

    if let Some(raw) = inherited_path {
        for part in raw.split(sep) {
            push_unique(part.trim().to_string());
        }
    }

    if !cfg!(target_os = "windows") {
        for fallback in ["/usr/bin", "/bin", "/usr/sbin", "/sbin"] {
            push_unique(fallback.to_string());
        }
    }

    entries.join(sep)
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn find_executable_in_path(bin: &str, path_env: &str) -> Option<PathBuf> {
    std::env::split_paths(path_env)
        .map(|dir| dir.join(bin))
        .find(|p| p.is_file())
}

#[cfg(not(any(target_os = "ios", target_os = "android")))]
fn resolve_local_command(
    shell_override: Option<&str>,
    use_tmux: bool,
    path_env: Option<&str>,
) -> (String, Vec<String>, String) {
    let (shell, mut args) = resolve_platform_shell(shell_override);
    if !use_tmux {
        return (shell.clone(), args, shell);
    }
    if shell == "wsl.exe" {
        args.extend([
            "--".to_string(),
            "sh".to_string(),
            "-lc".to_string(),
            "[ -z \"$TMUX\" ] && command -v tmux >/dev/null 2>&1 && exec tmux new-session -A -s ssh-box; exec \"${SHELL:-/bin/bash}\" -l".to_string(),
        ]);
        return (shell.clone(), args, shell);
    }
    if !cfg!(target_os = "windows") && std::env::var_os("TMUX").is_none() {
        if let Some(tmux_bin) = path_env.and_then(|p| find_executable_in_path("tmux", p)) {
            return (
                tmux_bin.to_string_lossy().to_string(),
                vec![
                    "new-session".to_string(),
                    "-A".to_string(),
                    "-s".to_string(),
                    "ssh-box".to_string(),
                ],
                shell,
            );
        }
    }
    (shell.clone(), args, shell)
}

impl LocalPtyManager {
    pub fn spawn_pane<F>(
        &self,
        pane_id: String,
        cols: u16,
        rows: u16,
        shell_override: Option<&str>,
        use_tmux: bool,
        on_output: F,
    ) -> Result<(), String>
    where
        F: FnMut(Vec<u8>) -> bool + Send + 'static,
    {
        if shell_override == Some("embedded") {
            return self.spawn_embedded_pane(pane_id, cols, rows, on_output);
        }

        #[cfg(not(any(target_os = "ios", target_os = "android")))]
        {
            // A stale setting (uninstalled pwsh, deleted shell) falls back to the auto shell first
            match Self::try_spawn_native(cols, rows, shell_override, use_tmux)
                .or_else(|_| Self::try_spawn_native(cols, rows, None, false))
            {
                Ok((master, writer, child, mut reader)) => {
                    let (tx_bytes, rx_bytes) = mpsc::channel::<Vec<u8>>(64);
                    let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);

                    std::thread::spawn(move || {
                        let mut buf = [0u8; 16 * 1024];
                        while let Ok(n) = reader.read(&mut buf) {
                            if n == 0 {
                                break;
                            }
                            if tx_bytes.blocking_send(buf[..n].to_vec()).is_err() {
                                break;
                            }
                        }
                    });

                    let mut cb = on_output;
                    tokio::spawn(async move {
                        let stream_reader = ChannelReader::new(rx_bytes);
                        let _ = batch_stream_to_sink(
                            stream_reader,
                            move |chunk| cb(chunk),
                            shutdown_rx,
                        )
                        .await;
                    });

                    self.panes.lock().unwrap().insert(
                        pane_id,
                        LocalPaneBackend::Native(NativePaneHandle {
                            master,
                            writer,
                            child,
                            shutdown_tx,
                        }),
                    );
                    return Ok(());
                }
                Err(_) => {
                    return self.spawn_embedded_pane(pane_id, cols, rows, on_output);
                }
            }
        }

        #[cfg(any(target_os = "ios", target_os = "android"))]
        {
            self.spawn_embedded_pane(pane_id, cols, rows, on_output)
        }
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    fn try_spawn_native(
        cols: u16,
        rows: u16,
        shell_override: Option<&str>,
        use_tmux: bool,
    ) -> Result<
        (
            Box<dyn MasterPty + Send>,
            Box<dyn Write + Send>,
            Box<dyn Child + Send + Sync>,
            Box<dyn Read + Send>,
        ),
        String,
    > {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let home = crate::host_store::app_home_dir();
        let inherited_path = std::env::var("PATH").ok();
        let full_path = if !cfg!(target_os = "windows") && !cfg!(target_os = "android") {
            Some(build_native_shell_path(inherited_path.as_deref(), &home))
        } else {
            inherited_path
        };

        let (program, args, shell) =
            resolve_local_command(shell_override, use_tmux, full_path.as_deref());
        let mut cmd = CommandBuilder::new(&program);
        for arg in args {
            cmd.arg(arg);
        }
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("TERM_PROGRAM", "SSH-Box");
        cmd.env("CLICOLOR", "1");
        cmd.env("CLICOLOR_FORCE", "1");
        cmd.env("LANG", "en_US.UTF-8");
        cmd.env("SHELL", &shell);
        if cfg!(target_os = "android") {
            cmd.env(
                "PATH",
                "/system/bin:/system/xbin:/sbin:/vendor/bin:/data/local/tmp",
            );
        } else if !cfg!(target_os = "windows") {
            if let Some(ref p) = full_path {
                cmd.env("PATH", p);
            }
        }
        if home.exists() {
            cmd.env("HOME", home.to_string_lossy().to_string());
            cmd.cwd(home);
        }

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);

        let reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
        Ok((pair.master, writer, child, reader))
    }

    fn spawn_embedded_pane<F>(
        &self,
        pane_id: String,
        cols: u16,
        rows: u16,
        mut on_output: F,
    ) -> Result<(), String>
    where
        F: FnMut(Vec<u8>) -> bool + Send + 'static,
    {
        let (tx_bytes, rx_bytes) = mpsc::channel::<Vec<u8>>(64);
        let (shutdown_tx, shutdown_rx) = mpsc::channel::<()>(1);
        let (input_tx, mut input_rx) = mpsc::unbounded_channel::<Vec<u8>>();
        let (resize_tx, mut resize_rx) = mpsc::unbounded_channel::<(u16, u16)>();

        tokio::spawn(async move {
            let stream_reader = ChannelReader::new(rx_bytes);
            let _ =
                batch_stream_to_sink(stream_reader, move |chunk| on_output(chunk), shutdown_rx)
                    .await;
        });

        tokio::spawn(async move {
            let mut session = EmbeddedShellSession::new(cols, rows);
            let banner = session.welcome_banner();
            if tx_bytes.send(banner.into_bytes()).await.is_err() {
                return;
            }

            loop {
                tokio::select! {
                    maybe_in = input_rx.recv() => {
                        let Some(bytes) = maybe_in else { break };
                        let out = session.feed_bytes(&bytes).await;
                        if !out.is_empty() && tx_bytes.send(out).await.is_err() {
                            break;
                        }
                        if session.should_exit {
                            break;
                        }
                    }
                    maybe_sz = resize_rx.recv() => {
                        if let Some((c, r)) = maybe_sz {
                            session.cols = c;
                            session.rows = r;
                        }
                    }
                }
            }
        });

        self.panes.lock().unwrap().insert(
            pane_id,
            LocalPaneBackend::Embedded(EmbeddedPaneHandle {
                input_tx,
                resize_tx,
                shutdown_tx,
            }),
        );
        Ok(())
    }

    pub fn write_input(&self, pane_id: &str, data: &[u8]) -> Result<(), String> {
        let mut map = self.panes.lock().unwrap();
        let pane = map.get_mut(pane_id).ok_or("Pane not found")?;
        match pane {
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            LocalPaneBackend::Native(native) => {
                native.writer.write_all(data).map_err(|e| e.to_string())?;
                native.writer.flush().map_err(|e| e.to_string())
            }
            LocalPaneBackend::Embedded(embedded) => embedded
                .input_tx
                .send(data.to_vec())
                .map_err(|e| e.to_string()),
        }
    }

    pub fn resize(&self, pane_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let map = self.panes.lock().unwrap();
        let pane = map.get(pane_id).ok_or("Pane not found")?;
        match pane {
            #[cfg(not(any(target_os = "ios", target_os = "android")))]
            LocalPaneBackend::Native(native) => native
                .master
                .resize(PtySize {
                    rows,
                    cols,
                    pixel_width: 0,
                    pixel_height: 0,
                })
                .map_err(|e| e.to_string()),
            LocalPaneBackend::Embedded(embedded) => embedded
                .resize_tx
                .send((cols, rows))
                .map_err(|e| e.to_string()),
        }
    }

    pub fn close(&self, pane_id: &str) -> bool {
        self.panes.lock().unwrap().remove(pane_id).is_some()
    }
}

pub async fn eval_embedded_command(cmd_line: &str) -> String {
    let mut session = EmbeddedShellSession::new(80, 24);
    session.execute_line(cmd_line).await
}

struct EmbeddedShellSession {
    cwd: PathBuf,
    env: BTreeMap<String, String>,
    history: Vec<String>,
    history_index: Option<usize>,
    line_buf: String,
    esc_state: u8,
    cols: u16,
    rows: u16,
    started_at: Instant,
    should_exit: bool,
}

impl EmbeddedShellSession {
    fn new(cols: u16, rows: u16) -> Self {
        let cwd = crate::host_store::app_home_dir();
        let _ = fs::create_dir_all(&cwd);
        let mut env = BTreeMap::new();
        env.insert("HOME".into(), cwd.to_string_lossy().to_string());
        env.insert(
            "USER".into(),
            std::env::var("USER")
                .or_else(|_| std::env::var("USERNAME"))
                .unwrap_or_else(|_| "mobile".into()),
        );
        env.insert("TERM".into(), "xterm-256color".into());
        env.insert("SHELL".into(), "sshbox-sh".into());
        Self {
            cwd,
            env,
            history: Vec::new(),
            history_index: None,
            line_buf: String::new(),
            esc_state: 0,
            cols,
            rows,
            started_at: Instant::now(),
            should_exit: false,
        }
    }

    fn welcome_banner(&self) -> String {
        let os = std::env::consts::OS;
        let arch = std::env::consts::ARCH;
        format!(
            "\x1b[1;36mSSH Box Embedded Shell\x1b[0m ({os}/{arch}) — type \x1b[33mhelp\x1b[0m for commands\r\n{}",
            self.prompt()
        )
    }

    fn prompt(&self) -> String {
        let home = self.env.get("HOME").cloned().unwrap_or_default();
        let cwd_str = self.cwd.to_string_lossy().to_string();
        let display_cwd = if !home.is_empty() && cwd_str.starts_with(&home) {
            let rest = &cwd_str[home.len()..];
            if rest.is_empty() {
                "~".to_string()
            } else {
                format!("~{rest}")
            }
        } else {
            cwd_str
        };
        let user = self.env.get("USER").map(|s| s.as_str()).unwrap_or("user");
        format!("\x1b[1;32m{user}@ssh-box\x1b[0m:\x1b[1;34m{display_cwd}\x1b[0m$ ")
    }

    async fn feed_bytes(&mut self, bytes: &[u8]) -> Vec<u8> {
        let mut out = String::new();
        let text = String::from_utf8_lossy(bytes);
        for ch in text.chars() {
            if self.esc_state == 1 {
                if ch == '[' {
                    self.esc_state = 2;
                } else {
                    self.esc_state = 0;
                }
                continue;
            } else if self.esc_state == 2 {
                self.esc_state = 0;
                match ch {
                    'A' => {
                        // Up arrow: previous history
                        if !self.history.is_empty() {
                            let idx = match self.history_index {
                                Some(i) => i.saturating_sub(1),
                                None => self.history.len() - 1,
                            };
                            self.history_index = Some(idx);
                            self.replace_current_line(&mut out, &self.history[idx].clone());
                        }
                    }
                    'B' => {
                        // Down arrow: next history
                        if let Some(i) = self.history_index {
                            if i + 1 < self.history.len() {
                                self.history_index = Some(i + 1);
                                self.replace_current_line(&mut out, &self.history[i + 1].clone());
                            } else {
                                self.history_index = None;
                                self.replace_current_line(&mut out, "");
                            }
                        }
                    }
                    _ => {}
                }
                continue;
            }

            match ch {
                '\x1b' => {
                    self.esc_state = 1;
                }
                '\r' | '\n' => {
                    out.push_str("\r\n");
                    let cmd_line = self.line_buf.trim().to_string();
                    self.line_buf.clear();
                    self.history_index = None;
                    if !cmd_line.is_empty() {
                        if self.history.last() != Some(&cmd_line) {
                            self.history.push(cmd_line.clone());
                        }
                        let result = self.execute_line(&cmd_line).await;
                        if !result.is_empty() {
                            let normalized = result.replace("\r\n", "\n").replace('\n', "\r\n");
                            out.push_str(&normalized);
                            if !out.ends_with("\r\n") {
                                out.push_str("\r\n");
                            }
                        }
                    }
                    if !self.should_exit {
                        out.push_str(&self.prompt());
                    }
                }
                '\x7f' | '\x08' => {
                    if self.line_buf.pop().is_some() {
                        out.push_str("\x08 \x08");
                    }
                }
                '\x03' => {
                    // Ctrl+C
                    self.line_buf.clear();
                    self.history_index = None;
                    out.push_str("^C\r\n");
                    out.push_str(&self.prompt());
                }
                '\x0c' => {
                    // Ctrl+L
                    out.push_str("\x1b[2J\x1b[H");
                    out.push_str(&self.prompt());
                    out.push_str(&self.line_buf);
                }
                '\x15' => {
                    // Ctrl+U
                    self.replace_current_line(&mut out, "");
                }
                '\t' => {
                    self.handle_tab_completion(&mut out);
                }
                c if !c.is_control() => {
                    self.line_buf.push(c);
                    out.push(c);
                }
                _ => {}
            }
        }
        out.into_bytes()
    }

    fn replace_current_line(&mut self, out: &mut String, new_text: &str) {
        let prev_len = self.line_buf.chars().count();
        for _ in 0..prev_len {
            out.push_str("\x08 \x08");
        }
        self.line_buf = new_text.to_string();
        out.push_str(new_text);
    }

    fn handle_tab_completion(&mut self, out: &mut String) {
        let prefix = self
            .line_buf
            .rsplit(|c: char| c.is_whitespace())
            .next()
            .unwrap_or("")
            .to_string();
        if prefix.is_empty() {
            return;
        }
        if let Ok(entries) = fs::read_dir(&self.cwd) {
            let mut matches: Vec<String> = entries
                .filter_map(|e| e.ok())
                .map(|e| {
                    let mut name = e.file_name().to_string_lossy().to_string();
                    if e.file_type().map(|t| t.is_dir()).unwrap_or(false) {
                        name.push('/');
                    }
                    name
                })
                .filter(|name| name.starts_with(&prefix))
                .collect();
            matches.sort();
            if matches.len() == 1 {
                let rest = &matches[0][prefix.len()..];
                self.line_buf.push_str(rest);
                out.push_str(rest);
            }
        }
    }

    fn resolve_path(&self, raw: &str) -> PathBuf {
        let trimmed = raw.trim();
        let home = self
            .env
            .get("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(crate::host_store::app_home_dir);
        let candidate = if trimmed.is_empty() || trimmed == "~" {
            home
        } else if let Some(stripped) = trimmed.strip_prefix("~/") {
            home.join(stripped)
        } else {
            let p = PathBuf::from(trimmed);
            if p.is_absolute() {
                p
            } else {
                self.cwd.join(p)
            }
        };
        let mut normalized = PathBuf::new();
        for comp in candidate.components() {
            match comp {
                std::path::Component::ParentDir => {
                    normalized.pop();
                }
                std::path::Component::CurDir => {}
                other => normalized.push(other.as_os_str()),
            }
        }
        if normalized.as_os_str().is_empty() {
            PathBuf::from("/")
        } else {
            normalized
        }
    }

    fn expand_vars(&self, input: &str) -> String {
        let mut result = input.to_string();
        for (k, v) in &self.env {
            result = result.replace(&format!("${{{k}}}"), v);
            result = result.replace(&format!("${k}"), v);
        }
        result
    }

    async fn execute_line(&mut self, line: &str) -> String {
        let mut combined_out = Vec::new();
        for statement in line.split(';') {
            let stmt = statement.trim();
            if stmt.is_empty() {
                continue;
            }
            let out = self.execute_pipeline(stmt).await;
            if !out.is_empty() {
                combined_out.push(out);
            }
            if self.should_exit {
                break;
            }
        }
        combined_out.join("\n")
    }

    async fn execute_pipeline(&mut self, stmt: &str) -> String {
        // Check output redirection (> or >>)
        let (pipeline_part, redirect): (&str, Option<(bool, &str)>) =
            if let Some((left, right)) = stmt.split_once(">>") {
                (left.trim(), Some((true, right.trim())))
            } else if let Some((left, right)) = stmt.split_once('>') {
                (left.trim(), Some((false, right.trim())))
            } else {
                (stmt, None)
            };

        let mut current_input: Option<String> = None;
        for stage in pipeline_part.split('|') {
            let stage_trimmed = self.expand_vars(stage.trim());
            let tokens = split_shell_words(&stage_trimmed);
            if tokens.is_empty() {
                continue;
            }
            let cmd = tokens[0].as_str();
            let args = &tokens[1..];
            current_input = Some(
                self.run_single_command(cmd, args, current_input.as_deref())
                    .await,
            );
        }

        let final_out = current_input.unwrap_or_default();
        if let Some((append, target_raw)) = redirect {
            let target = self.resolve_path(target_raw);
            if let Some(parent) = target.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let mut content = final_out;
            if !content.ends_with('\n') {
                content.push('\n');
            }
            let write_res = if append {
                fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&target)
                    .and_then(|mut f| f.write_all(content.as_bytes()))
            } else {
                fs::write(&target, content.as_bytes())
            };
            if let Err(e) = write_res {
                return format!("redirect error: {e}");
            }
            return String::new();
        }
        final_out
    }

    async fn run_single_command(
        &mut self,
        cmd: &str,
        args: &[String],
        stdin: Option<&str>,
    ) -> String {
        match cmd {
            "help" => [
                "SSH Box Built-in POSIX & Network Commands:",
                "  Filesystem : pwd, cd, ls [-la], cat, head, tail, mkdir, touch, cp, mv, rm, chmod, stat, find, grep, wc, df",
                "  Environment: echo, env, export KEY=VAL, unset KEY, clear, history, exit",
                "  System     : uname [-a], hostname, whoami, id, date, uptime, ps",
                "  Network    : ping <host[:port]>, nc <host> <port>, nslookup <domain>",
                "  SSH Box    : hosts (list saved SSH servers), keys (list saved SSH keys)",
            ]
            .join("\n"),
            "pwd" => self.cwd.to_string_lossy().to_string(),
            "cd" => {
                let target = args.first().map(|s| s.as_str()).unwrap_or("~");
                let resolved = self.resolve_path(target);
                if resolved.is_dir() {
                    self.cwd = resolved;
                    String::new()
                } else {
                    format!("cd: no such directory: {target}")
                }
            }
            "ls" => {
                let mut long_fmt = false;
                let mut show_all = false;
                let mut target_arg: Option<&str> = None;
                for a in args {
                    if a.starts_with('-') {
                        if a.contains('l') {
                            long_fmt = true;
                        }
                        if a.contains('a') {
                            show_all = true;
                        }
                    } else {
                        target_arg = Some(a.as_str());
                    }
                }
                let dir_path = target_arg
                    .map(|p| self.resolve_path(p))
                    .unwrap_or_else(|| self.cwd.clone());
                match crate::sftp::list_local_dir(&dir_path.to_string_lossy()) {
                    Ok(resp) => {
                        let mut lines = Vec::new();
                        for entry in resp.entries {
                            if !show_all && entry.name.starts_with('.') {
                                continue;
                            }
                            let colored_name = if entry.kind == "dir" {
                                format!("\x1b[1;34m{}/\x1b[0m", entry.name)
                            } else {
                                entry.name.clone()
                            };
                            if long_fmt {
                                lines.push(format!(
                                    "{} {:>4} {:>8}  {}",
                                    entry.permissions, entry.mode_octal, entry.size, colored_name
                                ));
                            } else {
                                lines.push(colored_name);
                            }
                        }
                        if long_fmt {
                            lines.join("\n")
                        } else {
                            lines.join("  ")
                        }
                    }
                    Err(e) => format!("ls: {e}"),
                }
            }
            "cat" => {
                if args.is_empty() {
                    return stdin.unwrap_or_default().to_string();
                }
                let mut out = Vec::new();
                for a in args {
                    let p = self.resolve_path(a);
                    match fs::read_to_string(&p) {
                        Ok(s) => out.push(s.trim_end_matches('\n').to_string()),
                        Err(e) => out.push(format!("cat: {a}: {e}")),
                    }
                }
                out.join("\n")
            }
            "head" | "tail" => {
                let mut count: usize = 10;
                let mut file_arg: Option<&str> = None;
                let mut i = 0;
                while i < args.len() {
                    if args[i] == "-n" && i + 1 < args.len() {
                        count = args[i + 1].parse().unwrap_or(10);
                        i += 2;
                    } else if let Some(stripped) = args[i].strip_prefix("-n") {
                        count = stripped.parse().unwrap_or(10);
                        i += 1;
                    } else {
                        file_arg = Some(args[i].as_str());
                        i += 1;
                    }
                }
                let content = if let Some(f) = file_arg {
                    match fs::read_to_string(self.resolve_path(f)) {
                        Ok(s) => s,
                        Err(e) => return format!("{cmd}: {f}: {e}"),
                    }
                } else {
                    stdin.unwrap_or_default().to_string()
                };
                let all_lines: Vec<&str> = content.lines().collect();
                let slice: Vec<&str> = if cmd == "head" {
                    all_lines.into_iter().take(count).collect()
                } else {
                    let start = all_lines.len().saturating_sub(count);
                    all_lines.into_iter().skip(start).collect()
                };
                slice.join("\n")
            }
            "grep" => {
                let mut ignore_case = false;
                let mut positional = Vec::new();
                for a in args {
                    if a == "-i" {
                        ignore_case = true;
                    } else if !a.starts_with('-') {
                        positional.push(a.as_str());
                    }
                }
                let Some(&pattern) = positional.first() else {
                    return "usage: grep [-i] <pattern> [file]".to_string();
                };
                let content = if let Some(&f) = positional.get(1) {
                    match fs::read_to_string(self.resolve_path(f)) {
                        Ok(s) => s,
                        Err(e) => return format!("grep: {f}: {e}"),
                    }
                } else {
                    stdin.unwrap_or_default().to_string()
                };
                let pat_cmp = if ignore_case {
                    pattern.to_lowercase()
                } else {
                    pattern.to_string()
                };
                content
                    .lines()
                    .filter(|line| {
                        if ignore_case {
                            line.to_lowercase().contains(&pat_cmp)
                        } else {
                            line.contains(&pat_cmp)
                        }
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            }
            "wc" => {
                let content = if let Some(f) = args.iter().find(|a| !a.starts_with('-')) {
                    match fs::read_to_string(self.resolve_path(f)) {
                        Ok(s) => s,
                        Err(e) => return format!("wc: {f}: {e}"),
                    }
                } else {
                    stdin.unwrap_or_default().to_string()
                };
                let lines = content.lines().count();
                let words = content.split_whitespace().count();
                let bytes = content.len();
                format!("{lines:>6} {words:>6} {bytes:>6}")
            }
            "mkdir" => {
                let mut errs = Vec::new();
                for a in args.iter().filter(|a| !a.starts_with('-')) {
                    let p = self.resolve_path(a);
                    if let Err(e) = fs::create_dir_all(&p) {
                        errs.push(format!("mkdir: {a}: {e}"));
                    }
                }
                errs.join("\n")
            }
            "touch" => {
                let mut errs = Vec::new();
                for a in args {
                    let p = self.resolve_path(a);
                    if let Err(e) = fs::OpenOptions::new().create(true).append(true).open(&p) {
                        errs.push(format!("touch: {a}: {e}"));
                    }
                }
                errs.join("\n")
            }
            "rm" => {
                let mut errs = Vec::new();
                for a in args.iter().filter(|a| !a.starts_with('-')) {
                    let p = self.resolve_path(a);
                    let res = if p.is_dir() {
                        fs::remove_dir_all(&p)
                    } else {
                        fs::remove_file(&p)
                    };
                    if let Err(e) = res {
                        errs.push(format!("rm: {a}: {e}"));
                    }
                }
                errs.join("\n")
            }
            "cp" => {
                let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
                if pos.len() < 2 {
                    return "usage: cp [-r] <src> <dst>".to_string();
                }
                let src = self.resolve_path(pos[0]);
                let mut dst = self.resolve_path(pos[1]);
                if dst.is_dir() {
                    if let Some(name) = src.file_name() {
                        dst = dst.join(name);
                    }
                }
                if src.is_dir() {
                    match crate::sftp::copy_dir_recursive(&src, &dst) {
                        Ok(()) => String::new(),
                        Err(e) => format!("cp: {e}"),
                    }
                } else {
                    match fs::copy(&src, &dst) {
                        Ok(_) => String::new(),
                        Err(e) => format!("cp: {e}"),
                    }
                }
            }
            "mv" => {
                if args.len() < 2 {
                    return "usage: mv <src> <dst>".to_string();
                }
                let src = self.resolve_path(&args[0]);
                let mut dst = self.resolve_path(&args[1]);
                if dst.is_dir() {
                    if let Some(name) = src.file_name() {
                        dst = dst.join(name);
                    }
                }
                match fs::rename(&src, &dst) {
                    Ok(()) => String::new(),
                    Err(e) => format!("mv: {e}"),
                }
            }
            "find" => {
                let root = args
                    .first()
                    .filter(|a| !a.starts_with('-'))
                    .map(|a| self.resolve_path(a))
                    .unwrap_or_else(|| self.cwd.clone());
                let name_filter = args
                    .windows(2)
                    .find(|w| w[0] == "-name")
                    .map(|w| w[1].replace('*', ""));
                let mut out = Vec::new();
                collect_find_entries(&root, name_filter.as_deref(), &mut out, 0);
                out.join("\n")
            }
            "stat" => {
                let Some(target) = args.first() else {
                    return "usage: stat <path>".to_string();
                };
                let p = self.resolve_path(target);
                match fs::metadata(&p) {
                    Ok(m) => format!(
                        "  File: {}\n  Size: {} bytes\n  Type: {}",
                        p.display(),
                        m.len(),
                        if m.is_dir() { "directory" } else { "regular file" }
                    ),
                    Err(e) => format!("stat: {target}: {e}"),
                }
            }
            "echo" => args.join(" "),
            "env" => self
                .env
                .iter()
                .map(|(k, v)| format!("{k}={v}"))
                .collect::<Vec<_>>()
                .join("\n"),
            "export" => {
                for a in args {
                    if let Some((k, v)) = a.split_once('=') {
                        self.env
                            .insert(k.trim().to_string(), v.trim_matches('"').to_string());
                    }
                }
                String::new()
            }
            "unset" => {
                for a in args {
                    self.env.remove(a);
                }
                String::new()
            }
            "uname" => {
                if args.iter().any(|a| a.contains('a')) {
                    format!(
                        "{} ssh-box 2.0.0 {} {}",
                        std::env::consts::OS,
                        std::env::consts::ARCH,
                        std::env::consts::FAMILY
                    )
                } else {
                    std::env::consts::OS.to_string()
                }
            }
            "whoami" => self
                .env
                .get("USER")
                .cloned()
                .unwrap_or_else(|| "mobile".into()),
            "hostname" => std::env::var("HOSTNAME")
                .or_else(|_| std::env::var("COMPUTERNAME"))
                .unwrap_or_else(|_| "ssh-box-device".into()),
            "id" => {
                let u = self
                    .env
                    .get("USER")
                    .cloned()
                    .unwrap_or_else(|| "mobile".into());
                format!("uid=1000({u}) gid=1000({u})")
            }
            "date" => {
                let secs = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .map(|d| d.as_secs())
                    .unwrap_or(0);
                format!("epoch: {secs}s")
            }
            "uptime" => {
                let up = self.started_at.elapsed().as_secs();
                let cpus = std::thread::available_parallelism()
                    .map(|n| n.get())
                    .unwrap_or(1);
                format!("up {up}s, {cpus} logical CPU cores available")
            }
            "df" => {
                format!(
                    "Filesystem      Mounted on\nsshbox-sandbox  {}",
                    self.cwd.display()
                )
            }
            "ps" => [
                "  PID COMMAND",
                "    1 ssh-box-runtime",
                "    2 sshbox-embedded-sh",
            ]
            .join("\n"),
            "hosts" => {
                let hosts = crate::host_store::load_all_hosts();
                if hosts.is_empty() {
                    "No SSH hosts configured yet.".to_string()
                } else {
                    let mut lines = vec![format!(
                        "{:<18} {:<26} {:<6} {}",
                        "ALIAS", "USER@HOST", "PORT", "GROUP"
                    )];
                    for h in hosts {
                        lines.push(format!(
                            "{:<18} {}@{:<20} {:<6} {}",
                            h.alias, h.user, h.hostname, h.port, h.group
                        ));
                    }
                    lines.join("\n")
                }
            }
            "keys" => {
                let store = crate::resources::load_resources();
                if store.ssh_keys.is_empty() {
                    "No SSH keys stored in SSH Box.".to_string()
                } else {
                    let mut lines =
                        vec![format!("{:<18} {:<10} {}", "NAME", "TYPE", "FINGERPRINT")];
                    for k in store.ssh_keys {
                        lines.push(format!("{:<18} {:<10} {}", k.name, k.key_type, k.fingerprint));
                    }
                    lines.join("\n")
                }
            }
            "nslookup" | "dig" => {
                let Some(host) = args.first() else {
                    return format!("usage: {cmd} <hostname>");
                };
                let target = if host.contains(':') {
                    host.clone()
                } else {
                    format!("{host}:22")
                };
                let res = match lookup_host(target.as_str()).await {
                    Ok(addrs) => {
                        let ips: Vec<String> = addrs.map(|a| a.ip().to_string()).collect();
                        if ips.is_empty() {
                            format!("{cmd}: no addresses found for {host}")
                        } else {
                            format!("Name:   {host}\nAddress: {}", ips.join(", "))
                        }
                    }
                    Err(e) => format!("{cmd}: lookup failed for {host}: {e}"),
                };
                res
            }
            "ping" | "nc" => {
                let pos: Vec<&String> = args.iter().filter(|a| !a.starts_with('-')).collect();
                let Some(first) = pos.first() else {
                    return "usage: ping <host[:port]> | nc <host> <port>".to_string();
                };
                let (host, port) = if let Some(second) = pos.get(1) {
                    (first.as_str(), second.parse::<u16>().unwrap_or(22))
                } else if let Some((h, p)) = first.split_once(':') {
                    (h, p.parse::<u16>().unwrap_or(22))
                } else {
                    (first.as_str(), 22)
                };
                let addr = format!("{host}:{port}");
                let start = Instant::now();
                match tokio::time::timeout(Duration::from_secs(3), TcpStream::connect(&addr)).await
                {
                    Ok(Ok(_)) => {
                        let ms = start.elapsed().as_secs_f64() * 1000.0;
                        format!("TCP connected to {addr} — time={ms:.1} ms")
                    }
                    Ok(Err(e)) => format!("Connect to {addr} failed: {e}"),
                    Err(_) => format!("Connect to {addr} timed out (3000 ms)"),
                }
            }
            "history" => self
                .history
                .iter()
                .enumerate()
                .map(|(i, h)| format!("{:>4}  {h}", i + 1))
                .collect::<Vec<_>>()
                .join("\n"),
            "clear" => "\x1b[2J\x1b[H".to_string(),
            "exit" | "logout" => {
                self.should_exit = true;
                "logout".to_string()
            }
            other => {
                #[cfg(target_os = "android")]
                {
                    if Path::new("/system/bin/sh").exists() {
                        let full = if args.is_empty() {
                            other.to_string()
                        } else {
                            format!("{other} {}", args.join(" "))
                        };
                        if let Ok(out) = tokio::process::Command::new("/system/bin/sh")
                            .arg("-c")
                            .arg(&full)
                            .current_dir(&self.cwd)
                            .env("PATH", "/system/bin:/system/xbin:/sbin:/vendor/bin")
                            .output()
                            .await
                        {
                            let stdout = String::from_utf8_lossy(&out.stdout).trim_end().to_string();
                            let stderr = String::from_utf8_lossy(&out.stderr).trim_end().to_string();
                            if !stdout.is_empty() || !stderr.is_empty() {
                                return if stderr.is_empty() {
                                    stdout
                                } else if stdout.is_empty() {
                                    stderr
                                } else {
                                    format!("{stdout}\n{stderr}")
                                };
                            }
                        }
                    }
                }
                format!(
                    "sshbox-sh: command not found: {other} (type 'help' for built-in commands)"
                )
            }
        }
    }
}

fn collect_find_entries(
    dir: &Path,
    name_sub: Option<&str>,
    out: &mut Vec<String>,
    depth: usize,
) {
    if depth > 6 || out.len() >= 200 {
        return;
    }
    let Ok(rd) = fs::read_dir(dir) else { return };
    for entry in rd.flatten() {
        let p = entry.path();
        let file_name = entry.file_name().to_string_lossy().to_string();
        let matched = name_sub
            .map(|sub| file_name.contains(sub))
            .unwrap_or(true);
        if matched {
            out.push(p.to_string_lossy().to_string());
        }
        if entry.file_type().map(|t| t.is_dir()).unwrap_or(false) {
            collect_find_entries(&p, name_sub, out, depth + 1);
        }
    }
}

fn split_shell_words(input: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut in_single = false;
    let mut in_double = false;
    for ch in input.chars() {
        match ch {
            '\'' if !in_double => in_single = !in_single,
            '"' if !in_single => in_double = !in_double,
            c if c.is_whitespace() && !in_single && !in_double => {
                if !current.is_empty() {
                    words.push(std::mem::take(&mut current));
                }
            }
            c => current.push(c),
        }
    }
    if !current.is_empty() {
        words.push(current);
    }
    words
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;

    #[test]
    fn wsl_distro_list_decodes_utf16_and_hides_docker_vms() {
        // Default wsl.exe output is UTF-16LE; read as UTF-8 it becomes "U\0b\0u\0..." garbage.
        let utf16: Vec<u8> = "Ubuntu-22.04\r\ndocker-desktop\r\ndocker-desktop-data\r\n"
            .encode_utf16()
            .flat_map(|u| u.to_le_bytes())
            .collect();
        assert_eq!(parse_wsl_list(&utf16), vec!["Ubuntu-22.04"]);
        assert_eq!(parse_wsl_list(b"Debian\n"), vec!["Debian"]);
    }

    #[cfg(not(any(target_os = "ios", target_os = "android")))]
    #[test]
    fn wsl_setting_opens_chosen_distro_in_linux_home() {
        let (exe, args) = resolve_platform_shell(Some("wsl:Ubuntu-22.04"));
        assert_eq!(exe, "wsl.exe");
        assert_eq!(args, vec!["-d", "Ubuntu-22.04", "--cd", "~"]);

        let (tmux_exe, tmux_args, _) =
            resolve_local_command(Some("wsl:Ubuntu-22.04"), true, None);
        assert_eq!(tmux_exe, "wsl.exe");
        assert!(tmux_args.iter().any(|a| a.contains("tmux new-session -A -s ssh-box")));
    }

    #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "windows")))]
    #[test]
    fn local_command_uses_tmux_when_found_in_path_and_falls_back_when_missing() {
        let tmp_dir =
            std::env::temp_dir().join(format!("ssh-box-tmux-test-{}", uuid::Uuid::new_v4()));
        fs::create_dir_all(&tmp_dir).unwrap();
        let fake_tmux = tmp_dir.join("tmux");
        fs::write(&fake_tmux, b"#!/bin/sh\n").unwrap();

        let (prog_missing, args_missing, shell_missing) =
            resolve_local_command(Some("/bin/zsh"), true, Some("/nonexistent-ssh-box-path"));
        assert_eq!(prog_missing, "/bin/zsh");
        assert_eq!(args_missing, vec!["-l"]);
        assert_eq!(shell_missing, "/bin/zsh");

        if std::env::var_os("TMUX").is_none() {
            let path_str = tmp_dir.to_string_lossy().to_string();
            let (prog_found, args_found, shell_found) =
                resolve_local_command(Some("/bin/zsh"), true, Some(&path_str));
            assert_eq!(prog_found, fake_tmux.to_string_lossy().to_string());
            assert_eq!(args_found, vec!["new-session", "-A", "-s", "ssh-box"]);
            assert_eq!(shell_found, "/bin/zsh");
        }

        let _ = fs::remove_dir_all(&tmp_dir);
    }

    #[tokio::test]
    async fn spawns_local_shell_reads_batched_output_resizes_and_closes() {
        let mgr = LocalPtyManager::default();
        let pane_id = "test-local-1".to_string();
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        mgr.spawn_pane(pane_id.clone(), 80, 24, None, false, move |chunk| {
            tx.send(chunk).is_ok()
        })
        .expect("should spawn local PTY or embedded shell");

        mgr.write_input(&pane_id, b"echo \"hello-ssh-box\"\r")
            .expect("should write command to PTY");

        let mut collected = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            if let Ok(Some(chunk)) =
                tokio::time::timeout(Duration::from_millis(200), rx.recv()).await
            {
                collected.extend_from_slice(&chunk);
                let text = String::from_utf8_lossy(&collected);
                if text.contains("hello-ssh-box") {
                    break;
                }
            }
        }

        let output_str = String::from_utf8_lossy(&collected);
        assert!(
            output_str.contains("hello-ssh-box"),
            "expected PTY output to contain hello-ssh-box, got: {:?}",
            output_str
        );

        mgr.resize(&pane_id, 120, 40)
            .expect("should resize PTY from 80x24 to 120x40");

        assert!(mgr.close(&pane_id), "should close active local pane");
        assert!(
            mgr.write_input(&pane_id, b"pwd\r").is_err(),
            "closed pane should no longer accept input"
        );
    }

    #[tokio::test]
    async fn embedded_mobile_shell_executes_filesystem_and_pipeline_commands() {
        let mgr = LocalPtyManager::default();
        let pane_id = "test-embedded-ios-1".to_string();
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        mgr.spawn_pane(
            pane_id.clone(),
            80,
            24,
            Some("embedded"),
            false,
            move |chunk| tx.send(chunk).is_ok(),
        )
        .expect("should spawn embedded mobile shell");

        mgr.write_input(
            &pane_id,
            b"echo \"alpha\" | grep alpha ; uname -a\r",
        )
        .expect("should send pipeline to embedded shell");

        let mut collected = Vec::new();
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while tokio::time::Instant::now() < deadline {
            if let Ok(Some(chunk)) =
                tokio::time::timeout(Duration::from_millis(200), rx.recv()).await
            {
                collected.extend_from_slice(&chunk);
                let text = String::from_utf8_lossy(&collected);
                if text.contains("ssh-box 2.0.0") {
                    break;
                }
            }
        }

        let output_str = String::from_utf8_lossy(&collected);
        assert!(
            output_str.contains("SSH Box Embedded Shell"),
            "should print welcome banner"
        );
        assert!(
            output_str.contains("ssh-box 2.0.0"),
            "should execute pipeline and uname -a, got: {:?}",
            output_str
        );
        assert!(mgr.close(&pane_id));
    }

    #[cfg(not(any(target_os = "ios", target_os = "android", target_os = "windows")))]
    #[test]
    fn unix_shell_spawns_as_login_shell_with_homebrew_path_fallbacks() {
        let (shell, args) = resolve_platform_shell(None);
        assert!(!shell.is_empty());
        assert_eq!(
            args,
            vec!["-l".to_string()],
            "Unix local shell must be spawned as a login shell (-l) so ~/.zprofile is sourced"
        );

        let home = crate::host_store::app_home_dir();
        let minimal_gui_path = "/usr/bin:/bin:/usr/sbin:/sbin";
        let resolved_path = build_native_shell_path(Some(minimal_gui_path), &home);
        if Path::new("/opt/homebrew/bin").exists() {
            assert!(
                resolved_path.split(':').any(|p| p == "/opt/homebrew/bin"),
                "PATH should include /opt/homebrew/bin when present on macOS, got: {resolved_path}"
            );
        }
        assert!(
            resolved_path.split(':').any(|p| p == "/usr/bin"),
            "PATH should preserve standard system bin directories"
        );
    }
}
