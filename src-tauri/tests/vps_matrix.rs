//! Manual matrix runner (spec §6.3). It changes sshd, firewall and users as root on the target.
//! Point it ONLY at a disposable VM, never at a real server. It is #[ignore]d and needs MX_* env.
//!
//! VM (macOS, Lima; restarted by `limactl` in the reboot scenario):
//!   limactl start --name=u24 --vm-type=vz --network=vzNAT --plain template://ubuntu-24.04
//!   (also template://debian-13 and template://rocky-9; ufw / firewalld + semanage may need installing)
//! Run (MX_DISTRO = ubuntu | debian | rocky; MX_ONLY=S1,S5 runs a subset):
//!   MX_VM=u24 MX_IP=<lima0 address> MX_USER=$USER MX_KEY=$HOME/.lima/_config/user MX_DISTRO=ubuntu \
//!   env -u SSH_AUTH_SOCK cargo test --manifest-path src-tauri/Cargo.toml --test vps_matrix -- --ignored --nocapture

use ssh_box_lib::host_store::{upsert_custom_host, HostEntry};
use ssh_box_lib::ssh_pool::{AuthUsed, SshPool};
use ssh_box_lib::vps_config::*;
use std::time::Duration;

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| panic!("{k} not set"))
}

fn host(id: &str, user: &str, key: Option<String>, port: u16) -> HostEntry {
    HostEntry {
        id: id.to_string(),
        alias: id.to_string(),
        hostname: env("MX_IP"),
        user: user.to_string(),
        port,
        identity_file: key,
        auth_method: if id == "mx-pw" { "password".into() } else { "key".into() },
        ..Default::default()
    }
}

// Runs a root command on the VM through the app's own pooled session (Lima user has sudo -n).
async fn root(pool: &SshPool, h: &HostEntry, cmd: &str) -> String {
    pool.exec_on_host(h, &format!("sudo -n sh -c {} 2>&1 || true", shell_single_quote(cmd)))
        .await
        .unwrap_or_else(|e| format!("EXEC ERROR: {e}"))
}

async fn effective(pool: &SshPool, h: &HostEntry, key: &str) -> String {
    root(pool, h, &format!("sshd -T | grep -i '^{key} '")).await.trim().to_string()
}

fn ensure(cond: bool, msg: String) -> Result<(), String> {
    if cond { Ok(()) } else { Err(msg) }
}

async fn s1_port_change(pool: &SshPool, h: &mut HostEntry, distro: &str) -> Result<String, String> {
    let view = get_sshd_view(&h.id, pool, None).await?;
    let started = apply_sshd_settings(&h.id, pool, None, SshdSettings { port: 2299, ..view.settings.clone() }).await?;
    ensure(started.new_port == Some(2299), format!("new_port {:?}", started.new_port))?;
    let after = finish_sshd_apply(&h.id, pool, None, started.new_port).await?;
    ensure(after.settings.port == 2299, format!("view port {}", after.settings.port))?;
    let ports = effective(pool, h, "port").await;
    ensure(ports == "port 2299", format!("sshd -T: {ports}"))?;
    let listen = root(pool, h, "ss -tlnpH 'sport = :2299'").await;
    ensure(!listen.trim().is_empty(), "nothing listens on 2299".into())?;
    let leftovers = root(pool, h, "systemctl is-active sshbox-rollback.timer; ls /etc/ssh/sshbox-rollback.sh 2>&1").await;
    ensure(leftovers.contains("inactive") && leftovers.contains("No such file"), format!("rollback leftovers: {leftovers}"))?;
    let mut note = format!("{} listening;", listen.split_whitespace().nth(3).unwrap_or("?"));
    if distro == "rocky" {
        let se = root(pool, h, "semanage port -l | grep ssh_port_t").await;
        ensure(se.contains("2299"), format!("semanage: {se}"))?;
        note += " semanage ok;";
    }
    // What the UI does after a verified port change, then move back to 22 the same way.
    h.port = 2299;
    upsert_custom_host(h.clone())?;
    let back = apply_sshd_settings(&h.id, pool, None, SshdSettings { port: 22, ..after.settings }).await?;
    finish_sshd_apply(&h.id, pool, None, back.new_port).await?;
    h.port = 22;
    upsert_custom_host(h.clone())?;
    ensure(effective(pool, h, "port").await == "port 22", "did not move back to 22".into())?;
    Ok(note + " moved back to 22")
}

async fn s2_blocked_port(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    // Stand-in for a provider firewall the app cannot see or change.
    root(pool, h, "nft add table inet mxblock; nft add chain inet mxblock in '{ type filter hook input priority -10; }'; nft add rule inet mxblock in tcp dport 2300 drop").await;
    let view = get_sshd_view(&h.id, pool, None).await?;
    let started = apply_sshd_settings(&h.id, pool, None, SshdSettings { port: 2300, ..view.settings }).await;
    let result = match started {
        Err(e) => Err(format!("apply failed before verify: {e}")),
        Ok(s) => match finish_sshd_apply(&h.id, pool, None, s.new_port).await {
            Ok(_) => Err("verify passed through a blocked port".into()),
            Err(e) if e.starts_with("VERIFY_FAILED") => Ok(e),
            Err(e) => Err(format!("unexpected: {e}")),
        },
    };
    root(pool, h, "nft delete table inet mxblock").await;
    let err = result?;
    let port = effective(pool, h, "port").await;
    ensure(port == "port 22", format!("after rollback sshd -T: {port}"))?;
    pool.test_exec(h, "true").await.map_err(|e| format!("old port unreachable: {e}"))?;
    Ok(format!("rolled back ({}), port 22 reachable", err.chars().take(60).collect::<String>()))
}

async fn s3_password_blocked(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    root(pool, h, "id tester >/dev/null 2>&1 || useradd -m -s /bin/bash tester; echo 'tester:Mx-Test-123' | chpasswd; \
g=sudo; getent group sudo >/dev/null || g=wheel; usermod -aG $g tester; \
printf 'PasswordAuthentication yes\\n' > /etc/ssh/sshd_config.d/01-matrix.conf; systemctl reload ssh 2>/dev/null || systemctl reload sshd").await;
    let pw_host = host("mx-pw", "tester", None, h.port);
    upsert_custom_host(pw_host.clone())?;
    pool.get_or_connect(&pw_host, Some("Mx-Test-123".into())).await?;
    ensure(pool.session_auth(&pw_host.id).await == Some(AuthUsed::Password), "session not password".into())?;
    let sudo = Some("Mx-Test-123".to_string());
    let p = probe(&pw_host.id, pool, sudo.clone()).await?;
    ensure(p.privilege == PrivilegeLevel::SudoPassword, format!("privilege {:?}", p.privilege))?;
    let view = get_sshd_view(&pw_host.id, pool, sudo.clone()).await?;
    let res = apply_sshd_settings(&pw_host.id, pool, sudo, SshdSettings { password_authentication: false, ..view.settings }).await;
    root(pool, h, "rm -f /etc/ssh/sshd_config.d/01-matrix.conf; systemctl reload ssh 2>/dev/null || systemctl reload sshd").await;
    match res {
        Err(e) if e == "LOCKOUT_BLOCKED: password_in_use" => Ok("blocked: password_in_use".into()),
        other => Err(format!("expected LOCKOUT_BLOCKED, got {other:?}")),
    }
}

async fn s4_cloud_init_password(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    root(pool, h, "printf 'PasswordAuthentication yes\\n' > /etc/ssh/sshd_config.d/50-cloud-init.conf; systemctl reload ssh 2>/dev/null || systemctl reload sshd").await;
    let view = get_sshd_view(&h.id, pool, None).await?;
    ensure(view.settings.password_authentication, "app shows password off while 50-cloud-init.conf enables it".into())?;
    let s = apply_sshd_settings(&h.id, pool, None, SshdSettings { password_authentication: false, ..view.settings }).await?;
    finish_sshd_apply(&h.id, pool, None, s.new_port).await?;
    let pa = effective(pool, h, "passwordauthentication").await;
    ensure(pa == "passwordauthentication no", format!("sshd -T: {pa}"))?;
    let dropin = root(pool, h, "grep -v '^#' /etc/ssh/sshd_config.d/00-sshbox.conf").await;
    ensure(!dropin.contains("MaxAuthTries") && !dropin.contains("AllowUsers"), format!("drop-in wrote unchanged directives: {dropin}"))?;
    Ok(format!("drop-in: {}", dropin.split_whitespace().collect::<Vec<_>>().join(" ")))
}

async fn wait_ssh(h: &HostEntry) -> Result<(), String> {
    for _ in 0..90 {
        if std::net::TcpStream::connect_timeout(&format!("{}:{}", h.hostname, h.port).parse().unwrap(), Duration::from_secs(2)).is_ok() {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err("VM did not come back after reboot".into())
}

async fn s5_firewall_reboot(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let snap = set_firewall_state(&h.id, pool, None, true).await?;
    ensure(snap.enabled, format!("not enabled after enable: engine {}", snap.engine))?;
    // Restart from the host: a guest-initiated reboot can stop a vz VM instead of restarting it.
    let vm = env("MX_VM");
    let real_home = env("MX_REAL_HOME");
    let ok = tokio::task::spawn_blocking(move || {
        let lima = |args: &[&str]| {
            std::process::Command::new("limactl").args(args).env("HOME", &real_home).status().map(|s| s.success()).unwrap_or(false)
        };
        lima(&["stop", &vm]) && lima(&["start", "--tty=false", &vm])
    })
    .await
    .unwrap();
    ensure(ok, "limactl stop/start failed".into())?;
    wait_ssh(h).await?;
    // Lima regenerates SSH host keys on every start (test-env artifact): accept the new key.
    let kh = ssh_box_lib::ssh_pool::default_known_hosts_path();
    for e in ssh_box_lib::ssh_pool::list_known_hosts_at(&kh) {
        if e.host_pattern.contains(&h.hostname) {
            let _ = ssh_box_lib::ssh_pool::delete_known_host_at(&kh, &e.host_pattern, &e.key_type);
        }
    }
    tokio::time::sleep(Duration::from_secs(3)).await;
    let fresh = SshPool::default();
    fresh.get_or_connect(h, None).await.map_err(|e| format!("cannot reconnect after reboot: {e}"))?;
    let snap = get_firewall_snapshot(&h.id, &fresh, None).await?;
    ensure(snap.enabled, "firewall off after reboot".into())?;
    Ok(format!("{} on after reboot, {} rules", snap.engine, snap.rules.len()))
}

async fn s6_blocked_actions(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let ctx = probe(&h.id, pool, None).await?.context;
    let snap = get_firewall_snapshot(&h.id, pool, None).await?;
    let ssh_rule = snap
        .rules
        .iter()
        .find(|r| !r.v6 && ssh_box_lib::vps_guard::check_firewall_delete(&ctx, &snap.rules, r).is_err())
        .cloned()
        .ok_or("no single SSH-allow rule found")?;
    let del = delete_firewall_rule(&h.id, pool, None, ssh_rule.clone()).await;
    ensure(del == Err("LOCKOUT_BLOCKED: only_ssh_rule".into()), format!("delete {}: {del:?}", ssh_rule.raw_spec))?;
    let AuthUsed::Key { public_key_body } = ctx.auth.clone() else { return Err("main host not key auth".into()) };
    let rev = revoke_ssh_key(&h.id, pool, None, ctx.login_user.clone(), public_key_body).await;
    ensure(rev == Err("LOCKOUT_BLOCKED: key_in_use".into()), format!("revoke: {rev:?}"))?;
    let lock = set_user_lock(&h.id, pool, None, ctx.login_user.clone(), true).await;
    ensure(lock == Err("LOCKOUT_BLOCKED: login_user".into()), format!("lock self: {lock:?}"))?;
    Ok(format!("delete '{}', revoke, lock self: all blocked", ssh_rule.raw_spec))
}

async fn s7_users_keys(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    root(pool, h, "for u in mx1 mx2 mx3; do userdel -r $u 2>/dev/null; rm -f /etc/sudoers.d/90-sshbox-$u; done").await;
    create_linux_user(&h.id, pool, None, "mx1".into(), SudoMode::NoPassword, None).await?;
    let snap = create_linux_user(&h.id, pool, None, "mx2".into(), SudoMode::WithPassword, Some("Mx2-pass:word".into())).await?;
    let mx1 = snap.accounts.iter().find(|a| a.username == "mx1").ok_or("mx1 missing")?;
    ensure(mx1.nopasswd_sudo && !mx1.locked, format!("mx1 sudo/lock: {} {}", mx1.nopasswd_sudo, mx1.locked))?;
    let sudo_ok = root(pool, h, "runuser -u mx1 -- sudo -n true && echo MX1_SUDO_OK").await;
    ensure(sudo_ok.contains("MX1_SUDO_OK"), format!("mx1 sudo -n: {sudo_ok}"))?;
    let pw_ok = root(pool, h, "printf 'Mx2-pass:word\\n' | runuser -u mx2 -- sudo -S -p '' true && echo MX2_SUDO_OK").await;
    ensure(pw_ok.contains("MX2_SUDO_OK"), format!("mx2 sudo with password: {pw_ok}"))?;
    let line = "ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAIMxMatrixTestKeyOnlyForTheVmXXXXXXXXXXXXXXX mx@matrix";
    authorize_ssh_key(&h.id, pool, None, "mx1".into(), line.into()).await?;
    let ak = root(pool, h, "stat -c '%U %a' ~mx1/.ssh ~mx1/.ssh/authorized_keys; cat ~mx1/.ssh/authorized_keys").await;
    ensure(ak.contains("mx1 700") && ak.contains("mx1 600") && ak.contains("mx@matrix"), format!("authorized_keys: {ak}"))?;
    let body = line.split_whitespace().nth(1).unwrap().to_string();
    let snap = revoke_ssh_key(&h.id, pool, None, "mx1".into(), body).await?;
    ensure(snap.accounts.iter().any(|a| a.username == "mx1" && a.authorized_keys.is_empty()), "key not revoked".into())?;
    let snap = set_user_lock(&h.id, pool, None, "mx2".into(), true).await?;
    ensure(snap.accounts.iter().any(|a| a.username == "mx2" && a.locked), "mx2 not shown locked".into())?;
    let snap = set_user_lock(&h.id, pool, None, "mx2".into(), false).await?;
    ensure(snap.accounts.iter().any(|a| a.username == "mx2" && !a.locked), "mx2 still locked".into())?;
    // A plain `useradd` account ('!' / '!!' password) still logs in with keys: not "locked" (review I8).
    root(pool, h, "useradd -m -s /bin/bash mx3").await;
    let snap = get_access_keys_snapshot(&h.id, pool, None).await?;
    let shown = snap.accounts.iter().find(|a| a.username == "mx3").map(|a| a.locked);
    let shadow = root(pool, h, "getent shadow mx3 | cut -d: -f2").await;
    ensure(shown == Some(false), format!("useradd account shown locked: {shown:?} (shadow {})", shadow.trim()))?;
    root(pool, h, "for u in mx1 mx2 mx3; do userdel -r $u 2>/dev/null; rm -f /etc/sudoers.d/90-sshbox-$u; done").await;
    Ok(format!("nopasswd sudo, password sudo, key grant (700/600 as user), revoke, lock/unlock, useradd '{}' not locked", shadow.trim()))
}

#[test]
#[ignore]
fn matrix() {
    std::env::set_var("MX_REAL_HOME", std::env::var("HOME").unwrap());
    let home = std::env::temp_dir().join(format!("sshbox-matrix-home-{}", std::process::id()));
    std::fs::create_dir_all(&home).unwrap();
    std::env::set_var("HOME", &home);
    std::env::remove_var("SSH_AUTH_SOCK");
    let distro = env("MX_DISTRO");
    let only = std::env::var("MX_ONLY").unwrap_or_default();
    let mut h = host("mx-main", &env("MX_USER"), Some(env("MX_KEY")), 22);
    upsert_custom_host(h.clone()).unwrap();

    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let pool = SshPool::default();
        pool.get_or_connect(&h, None).await.expect("connect to VM");
        // Start every run from the distro's own sshd config (earlier runs leave 00-sshbox.conf behind).
        root(&pool, &h, "rm -f /etc/ssh/sshd_config.d/00-sshbox.conf* /etc/ssh/sshd_config.d/01-matrix.conf /etc/ssh/sshd_config.d/50-cloud-init.conf; \
systemctl reload ssh 2>/dev/null || systemctl reload sshd").await;
        let p = probe(&h.id, &pool, None).await.expect("probe");
        println!("\n== {distro}: {} {} | {} | socket={} selinux={} fw={:?} systemd={}",
            p.os_id, p.os_version, p.openssh_version, p.ssh_socket_active, p.selinux_enforcing, p.firewall_engine, p.has_systemd);
        let mut rows: Vec<(&str, Result<String, String>)> = Vec::new();
        macro_rules! run {
            ($name:expr, $f:expr) => {
                if only.is_empty() || only.split(',').any(|s| s == &$name[..2]) {
                    let r = $f.await;
                    println!("{} {}: {:?}", if r.is_ok() { "PASS" } else { "FAIL" }, $name, r);
                    rows.push(($name, r));
                }
            };
        }
        run!("S1 port change", s1_port_change(&pool, &mut h, &distro));
        run!("S2 blocked port -> rollback", s2_blocked_port(&pool, &h));
        run!("S3 password off while app uses password", s3_password_blocked(&pool, &h));
        run!("S4 cloud-init password on -> off", s4_cloud_init_password(&pool, &h));
        run!("S5 firewall on + reboot", s5_firewall_reboot(&pool, &h));
        let pool = SshPool::default();
        pool.get_or_connect(&h, None).await.expect("reconnect");
        run!("S6 blocked lockout actions", s6_blocked_actions(&pool, &h));
        run!("S7 users & keys smoke", s7_users_keys(&pool, &h));
        let failed = rows.iter().filter(|(_, r)| r.is_err()).count();
        println!("== {distro}: {} passed, {failed} failed", rows.len() - failed);
        assert_eq!(failed, 0);
    });
}
