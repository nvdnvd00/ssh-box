//! Real-sshd tests for the VPS config flow. Each test starts /usr/sbin/sshd on 127.0.0.1 with a
//! throwaway HOME, so ~/.ssh and real servers are never touched. They are #[ignore]d because they
//! need OpenSSH installed. Run them with:
//!   env -u SSH_AUTH_SOCK cargo test --manifest-path src-tauri/Cargo.toml --test vps_sshd -- --ignored --test-threads=1

use ssh_box_lib::host_store::HostEntry;
use ssh_box_lib::ssh_pool::{AuthUsed, SshPool};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

struct TestSshd {
    dir: PathBuf,
    pid: u32,
    client_key: PathBuf,
    client_pub_body: String,
}

impl Drop for TestSshd {
    fn drop(&mut self) {
        let _ = Command::new("kill").arg(self.pid.to_string()).status();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

// known_hosts and resources resolve under HOME, so point it at a temp dir before any pool use.
fn fake_home() -> PathBuf {
    let home = std::env::temp_dir().join(format!("sshbox-it-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    std::env::remove_var("SSH_AUTH_SOCK");
    home
}

fn keygen(path: &Path) -> String {
    let ok = Command::new("ssh-keygen")
        .args(["-q", "-t", "ed25519", "-N", "", "-f"])
        .arg(path)
        .status()
        .unwrap()
        .success();
    assert!(ok, "ssh-keygen failed");
    let public = std::fs::read_to_string(path.with_extension("pub")).unwrap();
    public.split_whitespace().nth(1).unwrap().to_string()
}

fn start_sshd(name: &str, ports: &[u16], host_key: Option<&Path>) -> TestSshd {
    let dir = std::env::temp_dir().join(format!("sshbox-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let host_key = match host_key {
        Some(p) => p.to_path_buf(),
        None => {
            let p = dir.join("host_key");
            keygen(&p);
            p
        }
    };
    let client_key = dir.join("client");
    let client_pub_body = keygen(&client_key);
    std::fs::copy(client_key.with_extension("pub"), dir.join("authorized_keys")).unwrap();

    let mut cfg: String = ports.iter().map(|p| format!("Port {p}\n")).collect();
    cfg.push_str(&format!(
        "ListenAddress 127.0.0.1\nHostKey {}\nAuthorizedKeysFile {}\nPidFile {}\nStrictModes no\nUsePAM no\n\
         PasswordAuthentication no\nKbdInteractiveAuthentication no\n",
        host_key.display(),
        dir.join("authorized_keys").display(),
        dir.join("sshd.pid").display(),
    ));
    std::fs::write(dir.join("sshd_config"), cfg).unwrap();
    let ok = Command::new("/usr/sbin/sshd")
        .arg("-f")
        .arg(dir.join("sshd_config"))
        .arg("-E")
        .arg(dir.join("sshd.log"))
        .status()
        .unwrap()
        .success();
    assert!(ok, "sshd failed to start");

    let deadline = Instant::now() + Duration::from_secs(5);
    let pid = loop {
        if let Ok(raw) = std::fs::read_to_string(dir.join("sshd.pid")) {
            if let Ok(pid) = raw.trim().parse::<u32>() {
                break pid;
            }
        }
        assert!(Instant::now() < deadline, "sshd did not write its pid file");
        std::thread::sleep(Duration::from_millis(50));
    };
    for p in ports {
        while std::net::TcpStream::connect(("127.0.0.1", *p)).is_err() {
            assert!(Instant::now() < deadline, "sshd is not listening on {p}");
            std::thread::sleep(Duration::from_millis(50));
        }
    }
    TestSshd { dir, pid, client_key, client_pub_body }
}

fn host_for(s: &TestSshd, port: u16) -> HostEntry {
    HostEntry {
        id: format!("it-{port}"),
        alias: format!("it-{port}"),
        hostname: "127.0.0.1".to_string(),
        user: std::env::var("USER").unwrap(),
        port,
        identity_file: Some(s.client_key.display().to_string()),
        ..Default::default()
    }
}

fn block_on<F: std::future::Future>(f: F) -> F::Output {
    tokio::runtime::Runtime::new().unwrap().block_on(f)
}

// The lockout guard needs the key that actually logged in, not the keys the host could use.
#[test]
#[ignore]
fn session_auth_reports_the_key_that_logged_in() {
    fake_home();
    let sshd = start_sshd("auth", &[22911], None);
    let host = host_for(&sshd, 22911);
    block_on(async {
        let pool = SshPool::default();
        pool.get_or_connect(&host, None).await.unwrap();
        assert_eq!(
            pool.session_auth(&host.id).await,
            Some(AuthUsed::Key { public_key_body: sshd.client_pub_body.clone() })
        );
    });
}
