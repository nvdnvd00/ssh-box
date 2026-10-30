//! Real-sshd tests for the VPS config flow. Each test starts /usr/sbin/sshd on 127.0.0.1 with a
//! throwaway HOME, so ~/.ssh and real servers are never touched. They are #[ignore]d because they
//! need OpenSSH installed. Run them with:
//!   env -u SSH_AUTH_SOCK cargo test --manifest-path src-tauri/Cargo.toml --test vps_sshd -- --ignored --test-threads=1

use ssh_box_lib::host_store::HostEntry;
use ssh_box_lib::ssh_pool::{AuthUsed, SshPool};
use ssh_box_lib::vps_config::verify_new_connection;
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
        let _ = Command::new("kill").arg(self.pid.to_string()).stderr(std::process::Stdio::null()).status();
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

// `client`: reuse another test sshd's client key so one HostEntry can log in to both.
fn start_sshd(name: &str, ports: &[u16], client: Option<&TestSshd>) -> TestSshd {
    start_sshd_with(name, ports, client, "")
}

fn start_sshd_with(name: &str, ports: &[u16], client: Option<&TestSshd>, extra: &str) -> TestSshd {
    let dir = std::env::temp_dir().join(format!("sshbox-it-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let host_key = dir.join("host_key");
    keygen(&host_key);
    let (client_key, client_pub_body) = match client {
        Some(c) => (c.client_key.clone(), c.client_pub_body.clone()),
        None => {
            let k = dir.join("client");
            let body = keygen(&k);
            (k, body)
        }
    };
    std::fs::copy(client_key.with_extension("pub"), dir.join("authorized_keys")).unwrap();

    let mut cfg: String = ports.iter().map(|p| format!("Port {p}\n")).collect();
    cfg.push_str(&format!(
        "ListenAddress 127.0.0.1\nHostKey {}\nAuthorizedKeysFile {}\nPidFile {}\nStrictModes no\nUsePAM no\n\
         PasswordAuthentication no\nKbdInteractiveAuthentication no\n{}",
        host_key.display(),
        dir.join("authorized_keys").display(),
        dir.join("sshd.pid").display(),
        extra,
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

// sshd itself decides precedence: Include before the main file's directives, first value wins.
#[test]
#[ignore]
fn dropin_wins_over_cloud_init_and_main_file() {
    use ssh_box_lib::vps_config::{changed_directives, parse_sshd_t, render_dropin, SshdSettings};
    let dir = std::env::temp_dir().join(format!("sshbox-it-{}-dropin", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let d = dir.join("sshd_config.d");
    std::fs::create_dir_all(&d).unwrap();
    let host_key = dir.join("host_key");
    keygen(&host_key);
    let main = dir.join("sshd_config");
    std::fs::write(
        &main,
        format!("Include {}/*.conf\nPasswordAuthentication no\nHostKey {}\n", d.display(), host_key.display()),
    )
    .unwrap();
    std::fs::write(d.join("50-cloud-init.conf"), "PasswordAuthentication yes\n").unwrap();
    let effective = || {
        let out = Command::new("/usr/sbin/sshd").arg("-T").arg("-f").arg(&main).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        parse_sshd_t(&String::from_utf8_lossy(&out.stdout))
    };
    assert!(effective().password_authentication, "cloud-init drop-in should beat the main file");

    let before = effective();
    let ours = SshdSettings { password_authentication: false, ..before.clone() };
    std::fs::write(d.join("00-sshbox.conf"), render_dropin("", &changed_directives(&before, &ours))).unwrap();
    assert!(!effective().password_authentication, "00-sshbox.conf should beat 50-cloud-init.conf");
    let _ = std::fs::remove_dir_all(&dir);
}

// The pool's sessions live on the runtime that opened them, so each test uses one block_on.
async fn fingerprint_of(pool: &SshPool, host: &HostEntry) -> String {
    pool.get_or_connect(host, None).await.unwrap();
    pool.session_fingerprint(&host.id).await.unwrap()
}

// Spec §4.2 step 8: a NEW connection on the new port, same host key, is the only proof.
#[test]
#[ignore]
fn verify_succeeds_on_new_port_with_same_host_key() {
    fake_home();
    let a = start_sshd("verify-ok", &[22901, 22902], None);
    let host = host_for(&a, 22901);
    let pool = SshPool::default();
    block_on(async {
        let fp = fingerprint_of(&pool, &host).await;
        verify_new_connection(&pool, &host, 22902, &fp).await.unwrap();
    });
}

#[test]
#[ignore]
fn verify_fails_on_closed_port() {
    fake_home();
    let a = start_sshd("verify-closed", &[22904], None);
    let host = host_for(&a, 22904);
    let pool = SshPool::default();
    block_on(async {
        let fp = fingerprint_of(&pool, &host).await;
        let err = verify_new_connection(&pool, &host, 22909, &fp).await.unwrap_err();
        assert!(err.starts_with("VERIFY_FAILED"), "{err}");
    });
}

// A different machine answering on the new port must fail, and its key must not stay trusted.
#[test]
#[ignore]
fn verify_fails_on_different_host_key_and_forgets_it() {
    let home = fake_home();
    let a = start_sshd("verify-a", &[22905], None);
    let b = start_sshd("verify-b", &[22903], Some(&a));
    let host = host_for(&a, 22905);
    let pool = SshPool::default();
    block_on(async {
        let fp = fingerprint_of(&pool, &host).await;
        let err = verify_new_connection(&pool, &host, 22903, &fp).await.unwrap_err();
        assert!(err.starts_with("VERIFY_FAILED") && err.contains("host key"), "{err}");
    });
    let known = std::fs::read_to_string(home.join(".ssh/known_hosts")).unwrap_or_default();
    assert!(!known.contains("[127.0.0.1]:22903"), "{known}");
    drop(b);
}

// After a reload the old session stays alive; verifying through it would always "succeed".
#[test]
#[ignore]
fn old_session_is_not_proof() {
    fake_home();
    let a = start_sshd("verify-old", &[22906], None);
    let host = host_for(&a, 22906);
    let pool = SshPool::default();
    block_on(async {
        let fp = fingerprint_of(&pool, &host).await;
        // Stop the listener; the per-connection child serving our session keeps running.
        assert!(Command::new("kill").arg(a.pid.to_string()).status().unwrap().success());
        tokio::time::sleep(Duration::from_millis(300)).await;
        assert_eq!(pool.exec_on_host(&host, "echo alive").await.unwrap().trim(), "alive");
        let err = verify_new_connection(&pool, &host, 22906, &fp).await.unwrap_err();
        assert!(err.starts_with("VERIFY_FAILED"), "{err}");
    });
}

// MaxSessions 0 (or ForceCommand) lets authentication succeed but refuses every shell/command.
#[test]
#[ignore]
fn verify_fails_when_sessions_are_refused() {
    fake_home();
    let a = start_sshd_with("verify-nosession", &[22907], None, "MaxSessions 0\n");
    let host = host_for(&a, 22907);
    let pool = SshPool::default();
    block_on(async {
        let fp = fingerprint_of(&pool, &host).await;
        let err = verify_new_connection(&pool, &host, 22907, &fp).await.unwrap_err();
        assert!(err.starts_with("VERIFY_FAILED"), "{err}");
    });
}
