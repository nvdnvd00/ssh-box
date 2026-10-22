use crate::frame_batcher::{batch_stream_to_sink, ChannelReader};
use portable_pty::{native_pty_system, Child, CommandBuilder, MasterPty, PtySize};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc;

pub struct LocalPaneHandle {
    master: Box<dyn MasterPty + Send>,
    writer: Box<dyn Write + Send>,
    child: Box<dyn Child + Send + Sync>,
    shutdown_tx: mpsc::Sender<()>,
}

impl Drop for LocalPaneHandle {
    fn drop(&mut self) {
        let _ = self.shutdown_tx.try_send(());
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

#[derive(Default, Clone)]
pub struct LocalPtyManager {
    panes: Arc<Mutex<HashMap<String, LocalPaneHandle>>>,
}

impl LocalPtyManager {
    pub fn spawn_pane<F>(
        &self,
        pane_id: String,
        cols: u16,
        rows: u16,
        shell_override: Option<&str>,
        mut on_output: F,
    ) -> Result<(), String>
    where
        F: FnMut(Vec<u8>) -> bool + Send + 'static,
    {
        let pty_system = native_pty_system();
        let pair = pty_system
            .openpty(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())?;

        let shell = shell_override
            .map(|s| s.to_string())
            .or_else(|| std::env::var("SHELL").ok())
            .unwrap_or_else(|| "/bin/zsh".to_string());

        let mut cmd = CommandBuilder::new(&shell);
        cmd.env("TERM", "xterm-256color");
        cmd.env("COLORTERM", "truecolor");
        cmd.env("LANG", "en_US.UTF-8");
        if let Some(home) = dirs::home_dir() {
            cmd.cwd(home);
        }

        let child = pair.slave.spawn_command(cmd).map_err(|e| e.to_string())?;
        drop(pair.slave);

        let mut reader = pair.master.try_clone_reader().map_err(|e| e.to_string())?;
        let writer = pair.master.take_writer().map_err(|e| e.to_string())?;
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

        tokio::spawn(async move {
            let stream_reader = ChannelReader::new(rx_bytes);
            let _ = batch_stream_to_sink(stream_reader, move |chunk| on_output(chunk), shutdown_rx)
                .await;
        });

        self.panes.lock().unwrap().insert(
            pane_id,
            LocalPaneHandle {
                master: pair.master,
                writer,
                child,
                shutdown_tx,
            },
        );
        Ok(())
    }

    pub fn write_input(&self, pane_id: &str, data: &[u8]) -> Result<(), String> {
        let mut map = self.panes.lock().unwrap();
        let pane = map.get_mut(pane_id).ok_or("Pane not found")?;
        pane.writer.write_all(data).map_err(|e| e.to_string())?;
        pane.writer.flush().map_err(|e| e.to_string())
    }

    pub fn resize(&self, pane_id: &str, cols: u16, rows: u16) -> Result<(), String> {
        let map = self.panes.lock().unwrap();
        let pane = map.get(pane_id).ok_or("Pane not found")?;
        pane.master
            .resize(PtySize {
                rows,
                cols,
                pixel_width: 0,
                pixel_height: 0,
            })
            .map_err(|e| e.to_string())
    }

    pub fn close(&self, pane_id: &str) -> bool {
        self.panes.lock().unwrap().remove(pane_id).is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn spawns_local_shell_reads_batched_output_resizes_and_closes() {
        let mgr = LocalPtyManager::default();
        let pane_id = "test-local-1".to_string();
        let (tx, mut rx) = mpsc::unbounded_channel::<Vec<u8>>();

        mgr.spawn_pane(pane_id.clone(), 80, 24, Some("/bin/sh"), move |chunk| {
            tx.send(chunk).is_ok()
        })
        .expect("should spawn /bin/sh PTY");

        mgr.write_input(&pane_id, b"echo \"hello-ssh-box\"\n")
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
            mgr.write_input(&pane_id, b"pwd\n").is_err(),
            "closed pane should no longer accept input"
        );
    }
}
