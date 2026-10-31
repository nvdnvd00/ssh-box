//! Manual matrix runner (spec §6.3). It changes sshd, firewall and users as root on the target.
//! Point it ONLY at a disposable VM, never at a real server. It is #[ignore]d and needs MX_* env.
//!
//! VM (macOS, Lima; restarted by `limactl` in the reboot scenario):
//!   limactl start --name=u24 --vm-type=vz --network=vzNAT --plain template://ubuntu-24.04
//!   (also template://debian-13 and template://rocky-9; ufw / firewalld + semanage may need installing)
//! MX_DISTRO=macos runs the macOS scenarios M1,M2,M4,M5,M6 (VM user needs passwordless sudo).
//! MX_DISTRO=windows runs W1-W12: MX_USER is an Administrator (key in administrators_authorized_keys),
//! `sshbox-std` a standard account accepting the same key; W8-W11 (sshd writes) run only when
//! WINDOWS_SSH_WRITE_VERIFIED is true; W7 reboots the VM (Restart-Computer) unless MX_NO_REBOOT is set.
//! W13 adds a temporary port-less block for MX_IP's /24 (raw admin PowerShell, not the app) while the firewall is off.
//! W14-W16 do the same with an ICMPv4 block, a .1-.10 block in that /24 and AllowInboundRules False on all three
//! profiles (restored to their original values); a SYSTEM task, registered before each change, undoes it after 120s in case
//! an enable that should be refused goes through.
//! Run (MX_DISTRO = ubuntu | debian | rocky; MX_ONLY=S1,S5 runs a subset):
//!   MX_VM=u24 MX_IP=<lima0 address> MX_USER=$USER MX_KEY=$HOME/.lima/_config/user MX_DISTRO=ubuntu \
//!   env -u SSH_AUTH_SOCK cargo test --manifest-path src-tauri/Cargo.toml --test vps_matrix -- --ignored --nocapture

use ssh_box_lib::host_store::{upsert_custom_host, HostEntry};
use ssh_box_lib::ssh_pool::{AuthUsed, SshPool};
use ssh_box_lib::vps_config::*;
use ssh_box_lib::vps_support::{OsFamily, SupportLevel, WINDOWS_SSH_WRITE_VERIFIED};
use ssh_box_lib::vps_windows::run_windows_script;
use std::time::{Duration, Instant};

fn env(k: &str) -> String {
    std::env::var(k).unwrap_or_else(|_| panic!("{k} not set"))
}

// The client's /24 as the VM sees it: MX_IP without its last octet (the host is .1 on the same bridge).
fn client_net() -> String {
    let ip = env("MX_IP");
    ip.rsplit_once('.').map(|(net, _)| net.to_string()).unwrap_or(ip)
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

// ---- macOS scenarios (MX_DISTRO=macos; sudo -n works for the VM admin user) ----

// Auth methods sshd offers to a client with no usable credentials (what a password login would be offered).
fn offered_methods() -> String {
    let kh = std::env::temp_dir().join(format!("mx-methods-kh-{}", std::process::id()));
    let out = std::process::Command::new("ssh")
        .args(["-o", &format!("UserKnownHostsFile={}", kh.display()), "-o", "StrictHostKeyChecking=accept-new", "-o", "BatchMode=yes"])
        .args(["-o", "PubkeyAuthentication=no", "-o", "PreferredAuthentications=keyboard-interactive,password", "-o", "ConnectTimeout=8"])
        .arg(format!("{}@{}", env("MX_USER"), env("MX_IP")))
        .arg("true")
        .output()
        .expect("run ssh");
    let text = String::from_utf8_lossy(&out.stderr).to_string();
    text.split("Permission denied (").nth(1).and_then(|r| r.split(')').next()).unwrap_or(text.trim()).to_string()
}

async fn mac_leftovers(pool: &SshPool, h: &HostEntry) -> String {
    root(pool, h, "launchctl print system/com.sshbox.rollback >/dev/null 2>&1 && echo JOB; \
ls /Library/LaunchDaemons/com.sshbox.rollback.plist /etc/ssh/sshbox-rollback.sh 2>/dev/null").await
}

async fn m1_probe(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let p = probe(&h.id, pool, None).await?;
    ensure(p.os_family == OsFamily::Macos, format!("family {:?}", p.os_family))?;
    ensure(p.os_id == "macOS" && p.os_version == "27.0.1", format!("os {} {}", p.os_id, p.os_version))?;
    ensure(p.privilege == PrivilegeLevel::SudoNoPassword, format!("privilege {:?}", p.privilege))?;
    ensure(p.support.ssh.level == SupportLevel::Full, format!("ssh {:?}", p.support.ssh))?;
    ensure(!p.support.ssh_rows.port && p.support.password_coupled, format!("rows {:?}", p.support))?;
    ensure(p.support.firewall.level == SupportLevel::Hidden, format!("firewall {:?}", p.support.firewall))?;
    ensure(p.support.access.level == SupportLevel::ReadOnly && p.support.access.reason.as_deref() == Some("read_only_os"), format!("access {:?}", p.support.access))?;
    Ok(format!("{} {} sudo-nopw, ssh Full, port row hidden, coupled, firewall Hidden, access ReadOnly", p.os_id, p.os_version))
}

async fn m2_password_off_on(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let view = get_sshd_view(&h.id, pool, None).await?;
    let off = SshdSettings { password_authentication: false, kbd_interactive_authentication: false, ..view.settings.clone() };
    let s = apply_sshd_settings(&h.id, pool, None, off).await?;
    finish_sshd_apply(&h.id, pool, None, s.new_port).await?;
    let t = root(pool, h, "sshd -T | grep -i '^passwordauthentication \\|^kbdinteractiveauthentication '").await;
    ensure(t.contains("passwordauthentication no") && t.contains("kbdinteractiveauthentication no"), format!("sshd -T: {t}"))?;
    let dropin = root(pool, h, "ls /etc/ssh/sshd_config.d/00-sshbox.conf").await;
    ensure(dropin.trim() == "/etc/ssh/sshd_config.d/00-sshbox.conf", format!("drop-in: {dropin}"))?;
    let left = mac_leftovers(pool, h).await;
    ensure(left.trim().is_empty(), format!("rollback leftovers: {left}"))?;
    let m_off = offered_methods();
    ensure(m_off == "publickey", format!("methods off: {m_off}"))?;
    let view = get_sshd_view(&h.id, pool, None).await?;
    let on = SshdSettings { password_authentication: true, kbd_interactive_authentication: true, ..view.settings };
    let s = apply_sshd_settings(&h.id, pool, None, on).await?;
    finish_sshd_apply(&h.id, pool, None, s.new_port).await?;
    let m_on = offered_methods();
    ensure(m_on.contains("password") && m_on.contains("keyboard-interactive"), format!("methods on: {m_on}"))?;
    let left = mac_leftovers(pool, h).await;
    ensure(left.trim().is_empty(), format!("rollback leftovers after on: {left}"))?;
    Ok(format!("off: sshd -T {} / methods '{m_off}'; on: methods '{m_on}'", t.split_whitespace().collect::<Vec<_>>().join(" ")))
}

async fn m4_apply_then_disconnect(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let view = get_sshd_view(&h.id, pool, None).await?;
    let off = SshdSettings { password_authentication: false, kbd_interactive_authentication: false, ..view.settings };
    apply_sshd_settings(&h.id, pool, None, off).await?;
    let m_mid = offered_methods();
    ensure(m_mid == "publickey", format!("methods right after apply: {m_mid}"))?;
    tokio::time::sleep(Duration::from_secs(75)).await;
    let fresh = SshPool::default();
    fresh.get_or_connect(h, None).await.map_err(|e| format!("key login after timeout: {e}"))?;
    let m_after = offered_methods();
    ensure(m_after.contains("password"), format!("methods after rollback: {m_after}"))?;
    let t = root(&fresh, h, "sshd -T | grep -i '^passwordauthentication \\|^kbdinteractiveauthentication '").await;
    ensure(t.contains("passwordauthentication yes") && t.contains("kbdinteractiveauthentication yes"), format!("sshd -T: {t}"))?;
    let dropin = root(&fresh, h, "cat /etc/ssh/sshd_config.d/00-sshbox.conf 2>&1 | grep -ic 'no$'").await;
    let left = mac_leftovers(&fresh, h).await;
    ensure(left.trim().is_empty(), format!("rollback leftovers: {left}"))?;
    Ok(format!("mid '{m_mid}' -> after 75s '{m_after}'; key login ok; no job/plist/script; dropin 'no' lines {}", dropin.trim()))
}

async fn m5_port_hidden(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let before = root(pool, h, "sshd -T | grep -i '^port '; ls /etc/ssh/sshd_config.d; shasum /etc/ssh/sshd_config.d/* 2>&1").await;
    let view = get_sshd_view(&h.id, pool, None).await?;
    let r = apply_sshd_settings(&h.id, pool, None, SshdSettings { port: 2299, ..view.settings }).await;
    match r {
        Err(e) if e.starts_with("UNSUPPORTED: hidden_setting") => {
            let after = root(pool, h, "sshd -T | grep -i '^port '; ls /etc/ssh/sshd_config.d; shasum /etc/ssh/sshd_config.d/* 2>&1").await;
            ensure(before == after, format!("VM changed:\n{before}\n---\n{after}"))?;
            Ok(format!("{e}; VM unchanged"))
        }
        other => Err(format!("expected UNSUPPORTED: hidden_setting, got {other:?}")),
    }
}

async fn m6_users_keys(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let snap = get_access_keys_snapshot(&h.id, pool, None).await?;
    let me = snap.accounts.iter().find(|a| a.username == h.user).ok_or("login user missing")?;
    ensure(me.uid == 501 && me.has_sudo, format!("{} uid {} sudo {}", me.username, me.uid, me.has_sudo))?;
    let pubkey = std::fs::read_to_string(format!("{}.pub", env("MX_KEY"))).map_err(|e| e.to_string())?;
    let body = pubkey.split_whitespace().nth(1).ok_or("bad pub key")?;
    ensure(me.authorized_keys.iter().any(|k| k.public_key_body == body), "vm key not in authorized_keys".into())?;
    let root_acc = snap.accounts.iter().find(|a| a.username == "root").ok_or("root missing")?;
    ensure(root_acc.home_dir == "/var/root", format!("root home {}", root_acc.home_dir))?;
    let w = create_linux_user(&h.id, pool, None, "mxnew".into(), SudoMode::NoPassword, None).await;
    ensure(matches!(&w, Err(e) if e.starts_with("UNSUPPORTED: read_only_os")), format!("create_linux_user: {w:?}"))?;
    let exists = root(pool, h, "id mxnew 2>&1").await;
    ensure(exists.contains("no such user"), format!("user created anyway: {exists}"))?;
    Ok(format!("{} uid 501 sudo, key present; root home /var/root; create -> {}", me.username, w.unwrap_err()))
}

fn run_macos(only: &str) {
    let h = host("mx-main", &env("MX_USER"), Some(env("MX_KEY")), 22);
    upsert_custom_host(h.clone()).unwrap();
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let pool = SshPool::default();
        pool.get_or_connect(&h, None).await.expect("connect to VM");
        let mut failed = 0;
        macro_rules! run {
            ($name:expr, $f:expr) => {
                if only.is_empty() || only.split(',').any(|s| s == &$name[..2]) {
                    let r = $f.await;
                    println!("{} {}: {:?}", if r.is_ok() { "PASS" } else { "FAIL" }, $name, r);
                    failed += r.is_err() as u32;
                }
            };
        }
        run!("M1 probe", m1_probe(&pool, &h));
        run!("M2 password off/on (coupled)", m2_password_off_on(&pool, &h));
        run!("M4 apply then disconnect -> rollback", m4_apply_then_disconnect(&pool, &h));
        run!("M5 port change blocked", m5_port_hidden(&pool, &h));
        run!("M6 users & keys read-only", m6_users_keys(&pool, &h));
        assert_eq!(failed, 0);
    });
}

// ---- Windows scenarios (MX_DISTRO=windows) ----

// Bounded like the app's helper: this sshd.exe can hang on -T.
const WIN_SSHD_T: &str = r#"$psi = New-Object Diagnostics.ProcessStartInfo "$env:WINDIR\System32\OpenSSH\sshd.exe", '-T'
$psi.UseShellExecute = $false; $psi.RedirectStandardOutput = $true
$p = [Diagnostics.Process]::Start($psi); $o = $p.StandardOutput.ReadToEndAsync()
if ($p.WaitForExit(15000)) { $o.Result } else { $p.Kill(); 'SSHD_T_TIMEOUT' }
"#;
const WIN_LEFTOVERS: &str = r#"if (Get-ScheduledTask -TaskName sshbox-rollback -ErrorAction SilentlyContinue) { 'TASK' }
Get-ChildItem C:\ProgramData\ssh -Filter 'sshbox-rollback.ps1' | ForEach-Object Name
Get-ChildItem C:\ProgramData\ssh -Filter 'sshd_config.sshbox*' | ForEach-Object Name
"#;
const WIN_INBOX_RULE: &str = "OpenSSH SSH Server (sshd)";

// PowerShell on the VM through the app's own runner and pooled session.
async fn ps(pool: &SshPool, h: &HostEntry, script: &str) -> String {
    run_windows_script(&h.id, pool, script, None).await.map(|o| o.trim().to_string()).unwrap_or_else(|e| format!("PS ERROR: {e}"))
}

// `sshd -T` lines for the given keys (`a|b`), joined with "; ".
async fn win_effective(pool: &SshPool, h: &HostEntry, keys: &str) -> String {
    let out = ps(pool, h, WIN_SSHD_T).await;
    let hits: Vec<&str> = out.lines().map(str::trim).filter(|l| keys.split('|').any(|k| l.starts_with(&format!("{k} ")))).collect();
    if hits.is_empty() { out } else { hits.join("; ") }
}

async fn win_leftovers(pool: &SshPool, h: &HostEntry) -> String {
    ps(pool, h, WIN_LEFTOVERS).await.split_whitespace().collect::<Vec<_>>().join(" ")
}

async fn win_config(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    use base64::Engine;
    let b64 = ps(pool, h, r"[Convert]::ToBase64String([IO.File]::ReadAllBytes('C:\ProgramData\ssh\sshd_config'))").await;
    let bytes = base64::engine::general_purpose::STANDARD.decode(b64.trim()).map_err(|e| format!("sshd_config read: {e}: {b64}"))?;
    String::from_utf8(bytes).map_err(|e| e.to_string())
}

async fn ssh_works(h: &HostEntry, after: &str) -> Result<(), String> {
    SshPool::default().test_exec(h, "exit 0").await.map(|_| ()).map_err(|e| format!("new SSH login after {after}: {e}"))
}

async fn wait_port(h: &HostEntry, up: bool, secs: u64) -> Result<(), String> {
    let addr = format!("{}:{}", h.hostname, h.port).parse().unwrap();
    let t = Instant::now();
    while t.elapsed().as_secs() < secs {
        if std::net::TcpStream::connect_timeout(&addr, Duration::from_secs(2)).is_ok() == up {
            return Ok(());
        }
        tokio::time::sleep(Duration::from_secs(2)).await;
    }
    Err(format!("port {} not {} after {secs}s", h.port, if up { "up" } else { "down" }))
}

// Right after boot sshd accepts logins before PowerShell runs: ready means a PowerShell command answered.
async fn win_ready_pool(h: &HostEntry) -> Result<SshPool, String> {
    let mut last = String::new();
    for _ in 0..18 {
        let pool = SshPool::default();
        match pool.get_or_connect(h, None).await {
            Ok(_) if ps(&pool, h, "'READY'").await == "READY" => return Ok(pool),
            Ok(_) => last = "PowerShell not answering yet".into(),
            Err(e) => last = e,
        }
        tokio::time::sleep(Duration::from_secs(10)).await;
    }
    Err(format!("VM not ready after reboot: {last}"))
}

fn vm_key_body() -> Result<String, String> {
    let pubkey = std::fs::read_to_string(format!("{}.pub", env("MX_KEY"))).map_err(|e| e.to_string())?;
    pubkey.split_whitespace().nth(1).map(str::to_string).ok_or("bad pub key".into())
}

async fn w1_probe(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let p = probe(&h.id, pool, None).await?;
    ensure(p.os_family == OsFamily::Windows && p.os_id == "Windows" && p.os_version.contains("11"), format!("os {:?} {} '{}'", p.os_family, p.os_id, p.os_version))?;
    ensure(p.privilege == PrivilegeLevel::Admin, format!("privilege {:?}", p.privilege))?;
    ensure(p.firewall_engine == FirewallEngine::Windows, format!("firewall engine {:?}", p.firewall_engine))?;
    let s = &p.support;
    let want = if WINDOWS_SSH_WRITE_VERIFIED { (SupportLevel::Full, None) } else { (SupportLevel::ReadOnly, Some("windows_ssh_read_only")) };
    ensure((s.ssh.level, s.ssh.reason.as_deref()) == want, format!("ssh {:?}", s.ssh))?;
    ensure(s.ssh_rows.port && !s.ssh_rows.permit_root_login && !s.ssh_rows.allow_deny_users && s.password_coupled, format!("rows {:?} coupled {}", s.ssh_rows, s.password_coupled))?;
    ensure(s.firewall.level == SupportLevel::Full, format!("firewall {:?}", s.firewall))?;
    ensure(s.access.level == SupportLevel::ReadOnly && s.access.reason.as_deref() == Some("read_only_os"), format!("access {:?}", s.access))?;
    Ok(format!("{} '{}' OpenSSH {} user '{}' Admin; ssh {:?}; rows root/users hidden, port shown; coupled; firewall Full; access ReadOnly read_only_os",
        p.os_id, p.os_version, p.openssh_version, p.context.login_user, s.ssh))
}

async fn w2_standard_account(pool: &SshPool, std_host: &HostEntry) -> Result<String, String> {
    pool.get_or_connect(std_host, None).await?;
    let p = probe(&std_host.id, pool, None).await?;
    ensure(p.privilege == PrivilegeLevel::None, format!("privilege {:?}", p.privilege))?;
    for (name, t) in [("ssh", &p.support.ssh), ("firewall", &p.support.firewall), ("access", &p.support.access)] {
        ensure(t.level == SupportLevel::ReadOnly && t.reason.as_deref() == Some("windows_not_admin"), format!("{name} {t:?}"))?;
    }
    let v = get_sshd_view(&std_host.id, pool, None).await;
    ensure(matches!(&v, Err(e) if e.starts_with("UNSUPPORTED: windows_not_admin")), format!("sshd view: {:?}", v.map(|v| v.settings.port)))?;
    ensure(p.os_version.contains("11 "), format!("os_version '{}'", p.os_version))?;
    Ok(format!("'{}' None; ssh/firewall/access ReadOnly windows_not_admin; sshd view refused; os '{}'", p.context.login_user, p.os_version))
}

async fn w3_sshd_view(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let v = get_sshd_view(&h.id, pool, None).await?;
    ensure(v.write_target == WriteTarget::MainFile && !v.rollback_pending && v.settings.port == 22, format!("target {:?} pending {} port {}", v.write_target, v.rollback_pending, v.settings.port))?;
    ensure(v.main_raw.contains("Match Group administrators") && v.main_raw.contains("Match User sshbox-std"), "main_raw lacks the Match blocks".into())?;
    let st = &v.settings;
    Ok(format!("MainFile, no pending rollback, port {} maxauthtries {} password {} kbd {} pubkey {}", st.port, st.max_auth_tries, st.password_authentication, st.kbd_interactive_authentication, st.pubkey_authentication))
}

async fn w4_users_keys(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let t = Instant::now();
    let snap = get_access_keys_snapshot(&h.id, pool, None).await?;
    let secs = t.elapsed().as_secs_f32();
    let find = |n: &str| snap.accounts.iter().find(|a| a.username == n).ok_or(format!("{n} missing"));
    let me = find(&h.user)?;
    ensure(me.has_sudo && me.keys_file == r"C:\ProgramData\ssh\administrators_authorized_keys", format!("{} sudo {} file {}", me.username, me.has_sudo, me.keys_file))?;
    let body = vm_key_body()?;
    ensure(me.authorized_keys.iter().any(|k| k.public_key_body == body), "vm key not listed".into())?;
    let std_acc = find("sshbox-std")?;
    ensure(!std_acc.has_sudo && !std_acc.locked, format!("sshbox-std sudo {} locked {}", std_acc.has_sudo, std_acc.locked))?;
    for b in ["Administrator", "Guest"] {
        ensure(find(b)?.locked, format!("{b} not shown disabled"))?;
    }
    ensure(secs < 20.0, format!("took {secs:.1}s"))?;
    Ok(format!("{} accounts in {secs:.1}s; '{}' admin, key from {}; sshbox-std standard ({}); Administrator/Guest disabled",
        snap.accounts.len(), me.username, me.keys_file, std_acc.keys_file))
}

async fn w5_firewall_read(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let t = Instant::now();
    let snap = get_firewall_snapshot(&h.id, pool, None).await?;
    let secs = t.elapsed().as_secs_f32();
    ensure(snap.engine == "windows" && snap.enabled, format!("engine {} enabled {}", snap.engine, snap.enabled))?;
    let inbox: Vec<_> = snap.rules.iter().filter(|r| r.raw_spec == WIN_INBOX_RULE).collect();
    let shown: Vec<_> = inbox.iter().map(|r| format!("{}/{} {} from {} group '{}' effective {}", r.to_port, r.protocol, r.action, r.from_source, r.comment, r.effective)).collect();
    ensure(!inbox.is_empty() && inbox.iter().all(|r| r.effective), format!("inbox rule: {shown:?}"))?;
    let s22: Vec<_> = snap.listening_sockets.iter().filter(|s| s.port == 22 && s.protocol == "tcp" && s.is_public).map(|s| format!("{} {} {}", s.bind_addr, s.process_name, s.exposure)).collect();
    ensure(!s22.is_empty() && s22.iter().all(|e| e.ends_with(" exposed")), format!("port 22: {s22:?}"))?;
    ensure(secs < 10.0, format!("took {secs:.1}s"))?;
    Ok(format!("{} rules, {} sockets in {secs:.1}s; {shown:?}; port 22 {s22:?}", snap.rules.len(), snap.listening_sockets.len()))
}

async fn w6_firewall_writes(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let snap = add_firewall_rule(&h.id, pool, None, "allow".into(), "10.0.0.0/8".into(), "8080".into(), "tcp".into()).await?;
    let mine: Vec<_> = snap.rules.iter().filter(|r| r.to_port == "8080" && r.from_source == "10.0.0.0/8").cloned().collect();
    ensure(mine.len() == 1 && mine[0].comment == "ssh-box" && mine[0].action == "ALLOW", format!("added: {mine:?}"))?;
    let snap = delete_firewall_rule(&h.id, pool, None, mine[0].clone()).await?;
    ensure(!snap.rules.iter().any(|r| r.raw_spec == mine[0].raw_spec), "rule still listed after delete".into())?;
    let inbox = snap.rules.iter().find(|r| r.raw_spec == WIN_INBOX_RULE).cloned().ok_or("inbox rule missing")?;
    let del = delete_firewall_rule(&h.id, pool, None, inbox).await;
    ensure(del == Err("UNSUPPORTED: windows_rule_not_managed".into()), format!("delete inbox: {del:?}"))?;
    let deny = add_firewall_rule(&h.id, pool, None, "deny".into(), "any".into(), "22".into(), "tcp".into()).await;
    let deny_e = deny.err().unwrap_or_else(|| "Ok(snapshot)".into());
    ensure(deny_e.starts_with("LOCKOUT_BLOCKED"), format!("deny 22 from anywhere: {deny_e}"))?;
    let noport = add_firewall_rule(&h.id, pool, None, "allow".into(), "any".into(), "".into(), "tcp".into()).await;
    ensure(noport == Err("UNSUPPORTED: windows_port_required".into()), format!("no port: {noport:?}"))?;
    let left = ps(pool, h, "Get-NetFirewallRule -Group 'ssh-box' -ErrorAction SilentlyContinue | ForEach-Object DisplayName").await;
    ensure(!left.contains("8080") && !left.contains("DENY"), format!("ssh-box rules left: {left}"))?;
    Ok(format!("added+deleted '{}'; inbox delete -> windows_rule_not_managed; deny 22 -> {deny_e}; no port -> windows_port_required", mine[0].raw_spec))
}

async fn w7_firewall_toggle_reboot(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let off = set_firewall_state(&h.id, pool, None, false).await?;
    ensure(!off.enabled, "still enabled after disable".into())?;
    ssh_works(h, "disable").await?;
    let on = set_firewall_state(&h.id, pool, None, true).await?;
    ensure(on.enabled, "not enabled after enable".into())?;
    ssh_works(h, "enable").await?;
    let auto: Vec<String> = on.rules.iter().filter(|r| r.comment == "ssh-box" && !r.v6).map(|r| r.raw_spec.clone()).collect();
    if std::env::var("MX_NO_REBOOT").is_ok() {
        return Ok(format!("off/on with SSH after each; enable auto-added {auto:?}; reboot skipped (MX_NO_REBOOT)"));
    }
    // A rule the app adds must survive the reboot as well.
    let snap = add_firewall_rule(&h.id, pool, None, "allow".into(), "10.0.0.0/8".into(), "8081".into(), "tcp".into()).await?;
    let managed = |s: &FirewallSnapshot| s.rules.iter().filter(|r| r.comment == "ssh-box").map(|r| r.raw_spec.clone()).collect::<std::collections::BTreeSet<_>>();
    let before = managed(&snap);
    let t = Instant::now();
    let _ = tokio::time::timeout(Duration::from_secs(60), ps(pool, h, "Restart-Computer -Force")).await;
    wait_port(h, false, 180).await?;
    wait_port(h, true, 300).await?;
    let fresh = win_ready_pool(h).await?;
    let up = t.elapsed().as_secs();
    let snap = get_firewall_snapshot(&h.id, &fresh, None).await?;
    ensure(snap.enabled, "firewall off after reboot".into())?;
    let after = managed(&snap);
    ensure(before == after, format!("ssh-box rules before {before:?} after {after:?}"))?;
    ensure(snap.rules.iter().any(|r| r.raw_spec == WIN_INBOX_RULE), "inbox rule gone after reboot".into())?;
    ssh_works(h, "reboot").await?;
    let probe_rule = snap.rules.iter().find(|r| r.to_port == "8081").cloned().ok_or("8081 rule missing")?;
    delete_firewall_rule(&h.id, &fresh, None, probe_rule).await?;
    Ok(format!("off/on with SSH after each; enable auto-added {auto:?}; PowerShell answering {up}s after Restart-Computer; on after reboot, ssh-box rules kept {after:?}"))
}

// C1: a saved block for every port from the client's subnet beats the auto-allow, so enabling must be refused.
// The firewall stays off while the block exists; a SYSTEM task removes it after 120s in case enabling wrongly succeeds.
async fn w13_enable_refused_by_port_less_block(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    const TMP: &str = "sshbox-c1-temp";
    let off = set_firewall_state(&h.id, pool, None, false).await?;
    ensure(!off.enabled, "still enabled after disable".into())?;
    let net = client_net();
    let added = ps(pool, h, &format!(
        "New-NetFirewallRule -DisplayName '{TMP}' -Group '{TMP}' -Direction Inbound -Action Block -RemoteAddress {net}.0/24 -Protocol Any | Out-Null
$a = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument \"-NoProfile -Command Remove-NetFirewallRule -Group '{TMP}'\"
Register-ScheduledTask -TaskName '{TMP}' -Action $a -Trigger (New-ScheduledTaskTrigger -Once -At (Get-Date).AddSeconds(120)) -User SYSTEM -RunLevel Highest -Force | Out-Null
'ADDED'")).await;
    let checked = async {
        ensure(added.ends_with("ADDED"), format!("temp rule: {added}"))?;
        let snap = get_firewall_snapshot(&h.id, pool, None).await?;
        ensure(!snap.rules.iter().any(|r| r.raw_spec == TMP), "temp block listed in the UI rules".into())?;
        let hidden: Vec<_> = snap.unlisted_blocks.iter().filter(|r| r.raw_spec == TMP).map(|r| format!("{} from {} effective {}", r.to_port, r.from_source, r.effective)).collect();
        ensure(!hidden.is_empty() && snap.unlisted_blocks.iter().any(|r| r.raw_spec == TMP && r.effective), format!("temp block not seen: {hidden:?}"))?;
        let on = set_firewall_state(&h.id, pool, None, true).await;
        ensure(on == Err("LOCKOUT_BLOCKED: windows_block_covers_me".into()), format!("enable with the block: {:?}", on.map(|s| s.enabled)))?;
        ssh_works(h, "refused enable").await?;
        ensure(!get_firewall_snapshot(&h.id, pool, None).await?.enabled, "firewall on after the refused enable".into())?;
        Ok::<_, String>(hidden)
    }
    .await;
    let cleanup = ps(pool, h, &format!("Remove-NetFirewallRule -Group '{TMP}' -ErrorAction SilentlyContinue; Unregister-ScheduledTask -TaskName '{TMP}' -Confirm:$false -ErrorAction SilentlyContinue; 'LEFT:' + @(Get-NetFirewallRule -Group '{TMP}' -ErrorAction SilentlyContinue).Count + ':' + @(Get-ScheduledTask -TaskName '{TMP}' -ErrorAction SilentlyContinue).Count")).await;
    let hidden = checked?;
    ensure(cleanup.ends_with("LEFT:0:0"), format!("cleanup: {cleanup}"))?;
    let on = set_firewall_state(&h.id, pool, None, true).await?;
    ensure(on.enabled, "not enabled after removing the block".into())?;
    ssh_works(h, "enable after cleanup").await?;
    let auto: Vec<String> = on.rules.iter().filter(|r| r.comment == "ssh-box").map(|r| r.raw_spec.clone()).collect();
    Ok(format!("block seen unlisted {hidden:?}; enable -> windows_block_covers_me, SSH kept, firewall stayed off; rule removed; enable ok, SSH ok, auto-added {auto:?}"))
}

// A temporary firewall change (raw admin PowerShell). The SYSTEM task that undoes it after 120s, should SSH be cut, is
// registered before the change, so even a half-applied change has an undo; a failed setup is undone at once.
async fn win_temp(pool: &SshPool, h: &HostEntry, tag: &str, setup: &str, undo: &str) -> Result<(), String> {
    let added = ps(pool, h, &format!("$a = New-ScheduledTaskAction -Execute 'powershell.exe' -Argument \"-NoProfile -Command {undo}\"
Register-ScheduledTask -TaskName '{tag}' -Action $a -Trigger (New-ScheduledTaskTrigger -Once -At (Get-Date).AddSeconds(120)) -User SYSTEM -RunLevel Highest -Force | Out-Null
{setup}
'ADDED'")).await;
    if added.ends_with("ADDED") {
        return Ok(());
    }
    let cleaned = win_temp_undo(pool, h, tag, undo).await;
    Err(format!("{tag} setup: {added}; undo: {cleaned:?}"))
}

async fn win_temp_undo(pool: &SshPool, h: &HostEntry, tag: &str, undo: &str) -> Result<(), String> {
    let out = ps(pool, h, &format!("{undo}; Unregister-ScheduledTask -TaskName '{tag}' -Confirm:$false -ErrorAction SilentlyContinue; 'LEFT:' + @(Get-ScheduledTask -TaskName '{tag}' -ErrorAction SilentlyContinue).Count")).await;
    ensure(out.ends_with("LEFT:0"), format!("{tag} cleanup: {out}"))
}

// R1: an ICMP block never touches SSH, so enabling with one saved must work and port 22 must still read exposed.
async fn w14_icmp_block_does_not_refuse(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    const TMP: &str = "sshbox-w14-temp";
    let undo = format!("Remove-NetFirewallRule -Group '{TMP}' -ErrorAction SilentlyContinue");
    ensure(!set_firewall_state(&h.id, pool, None, false).await?.enabled, "still enabled after disable".into())?;
    win_temp(pool, h, TMP, &format!("New-NetFirewallRule -DisplayName 'Block ICMPv4 from Any' -Group '{TMP}' -Direction Inbound -Action Block -Protocol ICMPv4 -RemoteAddress Any | Out-Null"), &undo).await?;
    let checked = async {
        let on = set_firewall_state(&h.id, pool, None, true).await?;
        ensure(on.enabled, "not enabled with the ICMP block".into())?;
        ssh_works(h, "enable with the ICMP block").await?;
        ensure(!on.unlisted_blocks.iter().any(|r| r.raw_spec == "Block ICMPv4 from Any"), "ICMP block read as a TCP block".into())?;
        let s22: Vec<_> = on.listening_sockets.iter().filter(|s| s.port == 22 && s.protocol == "tcp" && s.is_public).map(|s| format!("{} {}", s.bind_addr, s.exposure)).collect();
        ensure(!s22.is_empty() && s22.iter().all(|e| e.ends_with(" exposed")), format!("port 22: {s22:?}"))?;
        Ok::<_, String>(s22)
    }
    .await;
    let cleaned = win_temp_undo(pool, h, TMP, &undo).await;
    let s22 = checked?;
    cleaned?;
    Ok(format!("ICMP block saved; enable ok, SSH ok, ICMP rule not among blocks, port 22 {s22:?}; rule removed"))
}

// R2/R5: with the firewall off, a temporary change makes enabling cut SSH; enabling must be refused with `reason`,
// SSH must keep working and the firewall must stay off; after the undo enabling works again.
async fn win_enable_refused(pool: &SshPool, h: &HostEntry, tag: &str, setup: &str, undo: &str, reason: &str) -> Result<String, String> {
    ensure(!set_firewall_state(&h.id, pool, None, false).await?.enabled, "still enabled after disable".into())?;
    win_temp(pool, h, tag, setup, undo).await?;
    let checked = async {
        let on = set_firewall_state(&h.id, pool, None, true).await;
        ensure(on == Err(format!("LOCKOUT_BLOCKED: {reason}")), format!("enable: {:?}", on.map(|s| s.enabled)))?;
        ssh_works(h, "refused enable").await?;
        ensure(!get_firewall_snapshot(&h.id, pool, None).await?.enabled, "firewall on after the refused enable".into())
    }
    .await;
    let cleaned = win_temp_undo(pool, h, tag, undo).await;
    checked?;
    cleaned?;
    let on = set_firewall_state(&h.id, pool, None, true).await?;
    ensure(on.enabled, "not enabled after the undo".into())?;
    ssh_works(h, "enable after the undo").await?;
    Ok(format!("enable -> {reason}, SSH kept, firewall stayed off; undone; enable ok, SSH ok"))
}

async fn w15_range_block_refuses(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    const TMP: &str = "sshbox-w15-temp";
    let net = client_net();
    let setup = format!("New-NetFirewallRule -DisplayName '{TMP}' -Group '{TMP}' -Direction Inbound -Action Block -RemoteAddress {net}.1-{net}.10 -Protocol Any | Out-Null");
    win_enable_refused(pool, h, TMP, &setup, &format!("Remove-NetFirewallRule -Group '{TMP}' -ErrorAction SilentlyContinue"), "windows_block_covers_me").await
}

// The undo restores each profile's own AllowInboundRules (persistent store, where the setup writes), not a forced default.
async fn w16_inbound_allows_off_refuses(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    const TMP: &str = "sshbox-w16-temp";
    let before = ps(pool, h, "(Get-NetFirewallProfile -PolicyStore PersistentStore | ForEach-Object { [string]$_.Name + '=' + [string]$_.AllowInboundRules }) -join ';'").await;
    let mut undo = Vec::new();
    for pair in before.split(';') {
        match pair.split_once('=') {
            Some((name @ ("Domain" | "Private" | "Public"), value @ ("True" | "False" | "NotConfigured"))) => {
                undo.push(format!("Set-NetFirewallProfile -PolicyStore PersistentStore -Profile {name} -AllowInboundRules {value}"))
            }
            _ => return Err(format!("AllowInboundRules before: {before}")),
        }
    }
    ensure(undo.len() == 3, format!("AllowInboundRules before: {before}"))?;
    let setup = "Set-NetFirewallProfile -PolicyStore PersistentStore -Profile Domain,Private,Public -AllowInboundRules False";
    let done = win_enable_refused(pool, h, TMP, setup, &undo.join("; "), "windows_inbound_allows_off").await?;
    Ok(format!("AllowInboundRules before {before}, restored; {done}"))
}

async fn win_apply_finish(pool: &SshPool, h: &HostEntry, desired: SshdSettings) -> Result<SshdView, String> {
    let s = apply_sshd_settings(&h.id, pool, None, desired).await?;
    let armed = win_leftovers(pool, h).await;
    ensure(armed.contains("TASK"), format!("rollback not armed after apply: '{armed}'"))?;
    finish_sshd_apply(&h.id, pool, None, s.new_port).await
}

async fn w8_max_auth_tries(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let before = win_config(pool, h).await?;
    let view = get_sshd_view(&h.id, pool, None).await?;
    let orig = view.settings.max_auth_tries;
    let want = if orig > 3 { orig - 1 } else { orig + 1 };
    let kept = win_apply_finish(pool, h, SshdSettings { max_auth_tries: want, ..view.settings.clone() }).await?;
    ensure(kept.settings.max_auth_tries == want, format!("view maxauthtries {}", kept.settings.max_auth_tries))?;
    let t = win_effective(pool, h, "maxauthtries").await;
    ensure(t == format!("maxauthtries {want}"), format!("sshd -T: {t}"))?;
    let left = win_leftovers(pool, h).await;
    ensure(left.is_empty(), format!("rollback leftovers: {left}"))?;
    let after = win_config(pool, h).await?;
    ensure(!after.replace("\r\n", "").contains('\n'), "bare LF in sshd_config".into())?;
    let (b, a): (Vec<&str>, Vec<&str>) = (before.split("\r\n").collect(), after.split("\r\n").collect());
    let directive = format!("MaxAuthTries {want}");
    let at = a.iter().position(|l| *l == directive).ok_or("directive missing")?;
    let first_match = a.iter().position(|l| l.starts_with("Match ")).ok_or("no Match line")?;
    ensure(at < first_match, format!("directive at line {at}, first Match at {first_match}"))?;
    // An earlier ssh-box line is replaced in place; otherwise one contiguous block (directive, ssh-box header, blank
    // separators) goes before the first Match. Every other line, blank ones included, stays unchanged.
    let i = b.iter().zip(&a).take_while(|(x, y)| x == y).count();
    let added = a.len().saturating_sub(b.len());
    let block = &a[i..i + added];
    let added_blank = block.iter().filter(|l| l.is_empty()).count();
    if b.get(i) == Some(&format!("MaxAuthTries {orig}").as_str()) {
        ensure(added == 0 && a[i] == directive && a[i + 1..] == b[i + 1..], format!("in-place replace at {i} changed more: {:?}", &a[i..]))?;
    } else {
        ensure(a[i + added..] == b[i..], format!("lines changed outside the inserted block at {i}: {:?}", &a[i + added..]))?;
        ensure(block.iter().all(|l| l.is_empty() || *l == directive || *l == "# --- Managed by ssh-box ---") && added_blank <= 2 && i + added <= first_match, format!("inserted block {block:?}"))?;
    }
    win_apply_finish(pool, h, SshdSettings { max_auth_tries: orig, ..kept.settings }).await?;
    let back = win_effective(pool, h, "maxauthtries").await;
    ensure(back == format!("maxauthtries {orig}"), format!("set back: {back}"))?;
    Ok(format!("{orig}->{want} kept (sshd -T '{t}'), no task/script/markers, CRLF kept, +{added} lines ({added_blank} blank), directive line {at} < Match line {first_match}; set back to {orig}"))
}

async fn w9_password_off_on(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let view = get_sshd_view(&h.id, pool, None).await?;
    let off = SshdSettings { password_authentication: false, kbd_interactive_authentication: false, ..view.settings };
    let kept = win_apply_finish(pool, h, off).await?;
    let t = win_effective(pool, h, "passwordauthentication|kbdinteractiveauthentication").await;
    ensure(t.contains("passwordauthentication no") && t.contains("kbdinteractiveauthentication no"), format!("sshd -T: {t}"))?;
    let m_off = offered_methods();
    ensure(m_off == "publickey", format!("methods off: {m_off}"))?;
    let on = SshdSettings { password_authentication: true, kbd_interactive_authentication: true, ..kept.settings };
    win_apply_finish(pool, h, on).await?;
    let m_on = offered_methods();
    ensure(m_on.contains("password") && m_on.contains("keyboard-interactive"), format!("methods on: {m_on}"))?;
    let left = win_leftovers(pool, h).await;
    ensure(left.is_empty(), format!("rollback leftovers: {left}"))?;
    Ok(format!("off: '{t}', methods '{m_off}'; on: methods '{m_on}'"))
}

async fn w10_apply_then_disconnect(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    let view = get_sshd_view(&h.id, pool, None).await?;
    let off = SshdSettings { password_authentication: false, kbd_interactive_authentication: false, ..view.settings };
    apply_sshd_settings(&h.id, pool, None, off).await?;
    let armed = win_leftovers(pool, h).await;
    ensure(armed.contains("TASK"), format!("rollback not armed: '{armed}'"))?;
    let m_mid = offered_methods();
    ensure(m_mid == "publickey", format!("methods right after apply: {m_mid}"))?;
    tokio::time::sleep(Duration::from_secs(75)).await;
    let fresh = SshPool::default();
    fresh.get_or_connect(h, None).await.map_err(|e| format!("key login after timeout: {e}"))?;
    let m_after = offered_methods();
    ensure(m_after.contains("password") && m_after.contains("keyboard-interactive"), format!("methods after rollback: {m_after}"))?;
    let t = win_effective(&fresh, h, "passwordauthentication|kbdinteractiveauthentication").await;
    ensure(t.contains("passwordauthentication yes") && t.contains("kbdinteractiveauthentication yes"), format!("sshd -T: {t}"))?;
    let left = win_leftovers(&fresh, h).await;
    ensure(left.is_empty(), format!("rollback leftovers: {left}"))?;
    Ok(format!("armed ({armed}); mid '{m_mid}' -> after 75s '{m_after}'; key login ok; no task/script/markers"))
}

async fn w11_blocked_port(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    // Block beats the app's own allow for the new port, so only the new-connection check can catch it.
    ps(pool, h, "New-NetFirewallRule -DisplayName 'mx-block-2222' -Direction Inbound -Action Block -Protocol TCP -LocalPort 2222 | Out-Null").await;
    let view = get_sshd_view(&h.id, pool, None).await?;
    let result = match apply_sshd_settings(&h.id, pool, None, SshdSettings { port: 2222, ..view.settings }).await {
        Err(e) => Err(format!("apply failed before verify: {e}")),
        Ok(s) => match finish_sshd_apply(&h.id, pool, None, s.new_port).await {
            Ok(_) => Err("verify passed through a blocked port".into()),
            Err(e) if e.starts_with("VERIFY_FAILED") => Ok(e),
            Err(e) => Err(format!("unexpected: {e}")),
        },
    };
    ps(pool, h, "Get-NetFirewallRule -DisplayName 'mx-block-2222' | Remove-NetFirewallRule").await;
    let err = result?;
    let port = win_effective(pool, h, "port").await;
    ensure(port == "port 22", format!("after rollback sshd -T: {port}"))?;
    let rules = ps(pool, h, "Get-NetFirewallRule -Group 'ssh-box' -ErrorAction SilentlyContinue | ForEach-Object DisplayName").await;
    ensure(!rules.contains("2222"), format!("2222 rule left: {rules}"))?;
    let left = win_leftovers(pool, h).await;
    ensure(left.is_empty(), format!("rollback leftovers: {left}"))?;
    ssh_works(h, "rollback").await?;
    Ok(format!("rolled back ({}), port 22 reachable, no 2222 rule", err.chars().take(70).collect::<String>()))
}

async fn w12_powershell_default_shell(pool: &SshPool, h: &HostEntry) -> Result<String, String> {
    ps(pool, h, r"New-ItemProperty -Path 'HKLM:\SOFTWARE\OpenSSH' -Name DefaultShell -Value 'C:\Windows\System32\WindowsPowerShell\v1.0\powershell.exe' -PropertyType String -Force | Out-Null").await;
    let fresh = SshPool::default();
    let r = async {
        let shell = fresh.exec_on_host(h, "$PSVersionTable.PSVersion.Major").await?;
        ensure(shell.trim().parse::<u32>().is_ok(), format!("default shell is not PowerShell: '{}'", shell.trim()))?;
        Ok::<_, String>(format!("PS {} as DefaultShell | {} | {}", shell.trim(), w1_probe(&fresh, h).await?, w3_sshd_view(&fresh, h).await?))
    }
    .await;
    let unset = ps(pool, h, r"Remove-ItemProperty -Path 'HKLM:\SOFTWARE\OpenSSH' -Name DefaultShell; 'UNSET'").await;
    ensure(unset.ends_with("UNSET"), format!("DefaultShell not removed: {unset}"))?;
    r
}

fn run_windows(only: &str) {
    let h = host("mx-main", &env("MX_USER"), Some(env("MX_KEY")), 22);
    let std_host = host("mx-std", "sshbox-std", Some(env("MX_KEY")), 22);
    upsert_custom_host(h.clone()).unwrap();
    upsert_custom_host(std_host.clone()).unwrap();
    tokio::runtime::Runtime::new().unwrap().block_on(async {
        let pool = SshPool::default();
        pool.get_or_connect(&h, None).await.expect("connect to VM");
        let mut failed = 0;
        macro_rules! run {
            ($name:expr, $f:expr) => {
                let id = $name.split(' ').next().unwrap();
                if only.is_empty() || only.split(',').any(|s| s == id) {
                    let t = Instant::now();
                    let r = $f.await;
                    println!("{} {} ({}s): {:?}", if r.is_ok() { "PASS" } else { "FAIL" }, $name, t.elapsed().as_secs(), r);
                    failed += r.is_err() as u32;
                }
            };
        }
        run!("W1 probe as admin", w1_probe(&pool, &h));
        run!("W2 probe as standard account", w2_standard_account(&pool, &std_host));
        run!("W3 sshd view as admin", w3_sshd_view(&pool, &h));
        run!("W4 users & keys read-only", w4_users_keys(&pool, &h));
        run!("W5 firewall read", w5_firewall_read(&pool, &h));
        run!("W6 firewall writes", w6_firewall_writes(&pool, &h));
        run!("W7 firewall off/on + reboot", w7_firewall_toggle_reboot(&pool, &h));
        // W7 reboots the VM: start over with a fresh session.
        let pool = win_ready_pool(&h).await.expect("reconnect");
        run!("W13 enable refused by port-less block", w13_enable_refused_by_port_less_block(&pool, &h));
        run!("W14 ICMP block does not refuse enable", w14_icmp_block_does_not_refuse(&pool, &h));
        run!("W15 range block refuses enable", w15_range_block_refuses(&pool, &h));
        run!("W16 AllowInboundRules False refuses enable", w16_inbound_allows_off_refuses(&pool, &h));
        if WINDOWS_SSH_WRITE_VERIFIED {
            run!("W8 MaxAuthTries change kept", w8_max_auth_tries(&pool, &h));
            run!("W9 password off/on (coupled)", w9_password_off_on(&pool, &h));
            run!("W10 apply then disconnect -> rollback", w10_apply_then_disconnect(&pool, &h));
            run!("W11 blocked port -> rollback", w11_blocked_port(&pool, &h));
        } else {
            println!("SKIP W8-W11: WINDOWS_SSH_WRITE_VERIFIED is false");
        }
        run!("W12 PowerShell DefaultShell", w12_powershell_default_shell(&pool, &h));
        assert_eq!(failed, 0);
    });
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
    if distro == "macos" {
        return run_macos(&only);
    }
    if distro == "windows" {
        return run_windows(&only);
    }
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
