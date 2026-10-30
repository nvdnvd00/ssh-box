//! Lockout guard (spec §4.3): pure checks run in the backend before every write.
//! A blocked change returns `LOCKOUT_BLOCKED: <reason_id>`; the UI translates the id (vpsLockoutReasons).

use crate::ssh_pool::AuthUsed;
use crate::vps_config::{
    is_anywhere_source, rule_port_matches, vps_err, ConnectionContext, FirewallRuleItem,
    LinuxAccountItem, SshdSettings, ERR_LOCKOUT_BLOCKED,
};
use std::net::IpAddr;

fn block(reason: &str) -> Result<(), String> {
    Err(vps_err(ERR_LOCKOUT_BLOCKED, reason))
}

// sshd patterns support `*` and `?`.
fn glob(pattern: &str, text: &str) -> bool {
    let (p, t): (Vec<char>, Vec<char>) = (pattern.chars().collect(), text.chars().collect());
    fn go(p: &[char], t: &[char]) -> bool {
        match p.first() {
            None => t.is_empty(),
            Some('*') => go(&p[1..], t) || (!t.is_empty() && go(p, &t[1..])),
            Some('?') => !t.is_empty() && go(&p[1..], &t[1..]),
            Some(c) => t.first() == Some(c) && go(&p[1..], &t[1..]),
        }
    }
    go(&p, &t)
}

// `user[@host]`; a host part we cannot match against the client IP counts as "not me".
fn user_pattern_matches(pattern: &str, ctx: &ConnectionContext) -> bool {
    let (user, host) = match pattern.split_once('@') {
        Some((u, h)) => (u, Some(h)),
        None => (pattern, None),
    };
    glob(user, &ctx.login_user)
        && host.map_or(true, |h| {
            if h.contains('/') {
                source_covers(h, &ctx.client_ip)
            } else {
                glob(h, &ctx.client_ip)
            }
        })
}

pub fn check_sshd(ctx: &ConnectionContext, next: &SshdSettings) -> Result<(), String> {
    if !next.password_authentication && !next.pubkey_authentication {
        return block("both_auth_off");
    }
    if next.max_sessions == 0 {
        return block("no_sessions");
    }
    match ctx.auth {
        AuthUsed::Password if !next.password_authentication => {
            return block("password_in_use")
        }
        AuthUsed::Key { .. } if !next.pubkey_authentication => {
            return block("pubkey_in_use")
        }
        _ => {}
    }
    if ctx.login_user == "root" {
        match next.permit_root_login.as_str() {
            "no" | "forced-commands-only" => return block("root_in_use"),
            "prohibit-password" | "without-password" if ctx.auth == AuthUsed::Password => {
                return block("root_password_in_use")
            }
            _ => {}
        }
    }
    if !next.allow_users.is_empty() && !next.allow_users.iter().any(|p| user_pattern_matches(p, ctx)) {
        return block("allow_users_excludes_me");
    }
    if next.deny_users.iter().any(|p| user_pattern_matches(p, ctx)) {
        return block("deny_users_matches_me");
    }
    Ok(())
}

fn is_allow(action: &str) -> bool {
    matches!(action.to_ascii_uppercase().as_str(), "ALLOW" | "LIMIT" | "ACCEPT")
}

fn allows_my_ssh(ctx: &ConnectionContext, r: &FirewallRuleItem) -> bool {
    is_allow(&r.action)
        && r.v6 == ctx.client_ip.contains(':')
        && rule_port_matches(&r.to_port, &r.protocol, ctx.server_port, "tcp")
        && source_covers(&r.from_source, &ctx.client_ip)
}

pub fn ssh_allow_needed(ctx: &ConnectionContext, rules: &[FirewallRuleItem]) -> bool {
    !rules.iter().any(|r| allows_my_ssh(ctx, r))
}

pub fn check_firewall_delete(
    ctx: &ConnectionContext,
    rules: &[FirewallRuleItem],
    target: &FirewallRuleItem,
) -> Result<(), String> {
    // Deletion is by content, so every rule with the same raw_spec goes (firewalld family twins,
    // and the current copy of a rule whose index went stale).
    let removed = |r: &FirewallRuleItem| r.raw_spec == target.raw_spec;
    let removes_mine = std::iter::once(target)
        .chain(rules.iter().filter(|r| removed(r)))
        .any(|r| allows_my_ssh(ctx, r));
    if removes_mine && !rules.iter().any(|r| !removed(r) && allows_my_ssh(ctx, r)) {
        return block("only_ssh_rule");
    }
    Ok(())
}

// Windows Firewall: a block rule wins over any allow, so an allow only counts while no block covers the same ssh path.
pub fn check_firewall_delete_windows(
    ctx: &ConnectionContext,
    rules: &[FirewallRuleItem],
    target: &FirewallRuleItem,
) -> Result<(), String> {
    let removed = |r: &FirewallRuleItem| r.raw_spec == target.raw_spec;
    let blocks_me = |r: &FirewallRuleItem| {
        !is_allow(&r.action)
            && r.v6 == ctx.client_ip.contains(':')
            && rule_port_matches(&r.to_port, &r.protocol, ctx.server_port, "tcp")
            && source_covers(&r.from_source, &ctx.client_ip)
    };
    let removes_mine = std::iter::once(target).chain(rules.iter().filter(|r| removed(r))).any(|r| allows_my_ssh(ctx, r));
    let still_open = rules.iter().any(|r| !removed(r) && allows_my_ssh(ctx, r))
        && !rules.iter().any(|r| !removed(r) && blocks_me(r));
    if removes_mine && !still_open {
        return block("only_ssh_rule");
    }
    Ok(())
}

pub fn check_firewall_add(
    ctx: &ConnectionContext,
    action: &str,
    source: &str,
    port: &str,
) -> Result<(), String> {
    if is_allow(action)
        || !source_covers(source, &ctx.client_ip)
        || !rule_port_matches(port, "any", ctx.server_port, "tcp")
    {
        return Ok(());
    }
    if is_anywhere_source(source) {
        return block("closes_ssh_for_everyone");
    }
    block("source_includes_me")
}

pub enum UserChange {
    Lock,
    RemoveSudo,
}

pub fn check_user_change(
    ctx: &ConnectionContext,
    accounts: &[LinuxAccountItem],
    user: &str,
    change: UserChange,
) -> Result<(), String> {
    if user == ctx.login_user {
        return block("login_user");
    }
    if let UserChange::RemoveSudo = change {
        let others = accounts
            .iter()
            .filter(|a| a.has_sudo && a.username != "root" && a.username != user)
            .count();
        if others == 0 {
            return block("last_sudo_user");
        }
    }
    Ok(())
}

pub fn check_key_revoke(ctx: &ConnectionContext, user: &str, key_body: &str) -> Result<(), String> {
    if let AuthUsed::Key { public_key_body } = &ctx.auth {
        if user == ctx.login_user && public_key_body == key_body {
            return block("key_in_use");
        }
    }
    Ok(())
}

// Does a firewall source ("Anywhere", CIDR, single IP) include `ip`?
pub fn source_covers(source: &str, ip: &str) -> bool {
    if is_anywhere_source(source) {
        return true;
    }
    let Ok(ip) = ip.trim().parse::<IpAddr>() else {
        return false;
    };
    let (net, prefix) = match source.trim().split_once('/') {
        Some((n, p)) => (n, p.parse::<u32>().ok()),
        None => (source.trim(), None),
    };
    let Ok(net) = net.parse::<IpAddr>() else {
        return false;
    };
    match (net, ip) {
        (IpAddr::V4(n), IpAddr::V4(i)) => prefix_eq(u32::from(n).into(), u32::from(i).into(), 32, prefix),
        (IpAddr::V6(n), IpAddr::V6(i)) => prefix_eq(u128::from(n), u128::from(i), 128, prefix),
        _ => false,
    }
}

fn prefix_eq(a: u128, b: u128, bits: u32, prefix: Option<u32>) -> bool {
    match prefix.unwrap_or(bits) {
        0 => true,
        p if p > bits => false,
        p => (a >> (bits - p)) == (b >> (bits - p)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ssh_pool::AuthUsed;
    use crate::vps_config::{ConnectionContext, FirewallRuleItem, LinuxAccountItem, SshdSettings};

    fn ctx(user: &str, auth: AuthUsed) -> ConnectionContext {
        ConnectionContext {
            login_user: user.to_string(),
            auth,
            client_ip: "203.0.113.7".to_string(),
            server_ip: "198.51.100.2".to_string(),
            server_port: 2222,
        }
    }
    fn key() -> AuthUsed {
        AuthUsed::Key { public_key_body: "AAAAMYKEY".to_string() }
    }
    fn deploy_key() -> ConnectionContext {
        ctx("deploy", key())
    }
    fn deploy_pw() -> ConnectionContext {
        ctx("deploy", AuthUsed::Password)
    }
    fn sshd() -> SshdSettings {
        SshdSettings {
            port: 2222,
            permit_root_login: "prohibit-password".to_string(),
            password_authentication: true,
            pubkey_authentication: true,
            allow_users: vec![],
            deny_users: vec![],
            max_auth_tries: 6,
            max_sessions: 10,
            login_grace_time: 120,
            client_alive_interval: 0,
            client_alive_count_max: 3,
            kbd_interactive_authentication: true,
        }
    }
    fn rule(index: u32, action: &str, to: &str, from: &str) -> FirewallRuleItem {
        FirewallRuleItem {
            index,
            to_port: to.to_string(),
            protocol: if to.ends_with("/tcp") { "tcp" } else { "any" }.to_string(),
            action: action.to_string(),
            from_source: from.to_string(),
            v6: false,
            comment: String::new(),
            raw_spec: format!("{to} {action} IN {from}"),
        }
    }
    fn account(name: &str, sudo: bool) -> LinuxAccountItem {
        LinuxAccountItem {
            username: name.to_string(),
            uid: if name == "root" { 0 } else { 1000 },
            gid: 1000,
            home_dir: format!("/home/{name}"),
            shell: "/bin/bash".to_string(),
            locked: false,
            groups: vec![],
            has_sudo: sudo,
            nopasswd_sudo: false,
            authorized_keys: vec![],
            keys_file: String::new(),
        }
    }
    fn blocked(r: Result<(), String>) -> bool {
        matches!(r, Err(ref e) if e.starts_with("LOCKOUT_BLOCKED"))
    }

    // Each row of spec §4.3: one case that must be blocked, one nearby case that must pass.

    #[test]
    fn password_off_blocked_when_app_uses_password() {
        let next = SshdSettings { password_authentication: false, ..sshd() };
        assert!(blocked(check_sshd(&deploy_pw(), &next)));
        assert_eq!(check_sshd(&deploy_key(), &next), Ok(()));
    }

    #[test]
    fn pubkey_off_blocked_when_app_uses_key() {
        let next = SshdSettings { pubkey_authentication: false, ..sshd() };
        assert!(blocked(check_sshd(&deploy_key(), &next)));
        assert_eq!(check_sshd(&deploy_pw(), &next), Ok(()));
    }

    #[test]
    fn both_auth_off_always_blocked() {
        let next = SshdSettings { password_authentication: false, pubkey_authentication: false, ..sshd() };
        assert!(blocked(check_sshd(&deploy_key(), &next)));
        assert!(blocked(check_sshd(&deploy_pw(), &next)));
    }

    // forced-commands-only still authenticates, but the app can no longer run its commands.
    #[test]
    fn root_login_off_blocked_for_root() {
        for value in ["no", "forced-commands-only"] {
            let next = SshdSettings { permit_root_login: value.to_string(), ..sshd() };
            assert!(blocked(check_sshd(&ctx("root", key()), &next)), "{value}");
            assert_eq!(check_sshd(&deploy_key(), &next), Ok(()), "{value}");
        }
    }

    #[test]
    fn root_prohibit_password_blocked_for_root_with_password() {
        for value in ["prohibit-password", "without-password"] {
            let next = SshdSettings { permit_root_login: value.to_string(), ..sshd() };
            assert!(blocked(check_sshd(&ctx("root", AuthUsed::Password), &next)), "{value}");
            assert_eq!(check_sshd(&ctx("root", key()), &next), Ok(()), "{value}");
        }
    }

    // MaxSessions 0 still lets logins through, so only the guard can stop it.
    #[test]
    fn zero_sessions_always_blocked() {
        assert!(blocked(check_sshd(&deploy_key(), &SshdSettings { max_sessions: 0, ..sshd() })));
        assert_eq!(check_sshd(&deploy_key(), &SshdSettings { max_sessions: 1, ..sshd() }), Ok(()));
    }

    #[test]
    fn allow_users_excluding_me_blocked() {
        let next = SshdSettings { allow_users: vec!["alice".to_string(), "bob".to_string()], ..sshd() };
        assert!(blocked(check_sshd(&deploy_key(), &next)));
        let next = SshdSettings { allow_users: vec!["alice".to_string(), "dep*".to_string()], ..sshd() };
        assert_eq!(check_sshd(&deploy_key(), &next), Ok(()));
    }

    #[test]
    fn deny_users_matching_me_blocked() {
        let next = SshdSettings { deny_users: vec!["d?ploy".to_string()], ..sshd() };
        assert!(blocked(check_sshd(&deploy_key(), &next)));
        let next = SshdSettings { deny_users: vec!["alice".to_string()], ..sshd() };
        assert_eq!(check_sshd(&deploy_key(), &next), Ok(()));
    }

    #[test]
    fn allow_users_host_part_excluding_my_ip_blocked() {
        let next = SshdSettings { allow_users: vec!["deploy@10.0.0.0/8".to_string()], ..sshd() };
        assert!(blocked(check_sshd(&deploy_key(), &next)));
        let next = SshdSettings { allow_users: vec!["deploy@203.0.113.0/24".to_string()], ..sshd() };
        assert_eq!(check_sshd(&deploy_key(), &next), Ok(()));
        let next = SshdSettings { allow_users: vec!["deploy@203.0.113.*".to_string()], ..sshd() };
        assert_eq!(check_sshd(&deploy_key(), &next), Ok(()));
    }

    #[test]
    fn ssh_allow_needed_when_no_rule_allows_server_port() {
        let rules = vec![rule(1, "ALLOW", "80/tcp", "Anywhere"), rule(2, "ALLOW", "22/tcp", "Anywhere")];
        assert!(ssh_allow_needed(&deploy_key(), &rules));
        let rules = vec![rule(1, "ALLOW", "2222/tcp", "Anywhere")];
        assert!(!ssh_allow_needed(&deploy_key(), &rules));
        let rules = vec![rule(1, "LIMIT", "2222/tcp", "203.0.113.0/24")];
        assert!(!ssh_allow_needed(&deploy_key(), &rules));
    }

    #[test]
    fn deleting_only_rule_for_my_ip_and_port_blocked() {
        let only = rule(1, "ALLOW", "2222/tcp", "203.0.113.7");
        let rules = vec![only.clone(), rule(2, "ALLOW", "80/tcp", "Anywhere")];
        assert!(blocked(check_firewall_delete(&deploy_key(), &rules, &only)));
        let rules = vec![only.clone(), rule(2, "ALLOW", "2222/tcp", "Anywhere")];
        assert_eq!(check_firewall_delete(&deploy_key(), &rules, &only), Ok(()));
        let web = rule(2, "ALLOW", "80/tcp", "Anywhere");
        assert_eq!(check_firewall_delete(&deploy_key(), &[only, web.clone()], &web), Ok(()));
    }

    // firewalld lists service:ssh once per family under one raw_spec; deleting it removes both copies.
    #[test]
    fn deleting_rule_twins_from_ipv6_client_blocked() {
        let v6 = ConnectionContext { client_ip: "2001:db8::7".to_string(), ..deploy_key() };
        let mut v4_copy = rule(3, "ALLOW", "2222/tcp", "Anywhere");
        v4_copy.raw_spec = "service:ssh".to_string();
        let v6_copy = FirewallRuleItem { v6: true, index: 4, ..v4_copy.clone() };
        let rules = vec![v4_copy.clone(), v6_copy];
        assert!(blocked(check_firewall_delete(&v6, &rules, &v4_copy)));
    }

    // Numbers shift outside the app (fail2ban inserts at 1); a stale index is still the same rule.
    #[test]
    fn deleting_with_stale_index_still_blocked() {
        let current = rule(2, "ALLOW", "2222/tcp", "203.0.113.7");
        let stale = FirewallRuleItem { index: 1, ..current.clone() };
        let rules = vec![rule(1, "DENY", "Anywhere", "198.51.100.9"), current];
        assert!(blocked(check_firewall_delete(&deploy_key(), &rules, &stale)));
    }

    #[test]
    fn deny_covering_client_ip_blocked() {
        assert!(blocked(check_firewall_add(&deploy_key(), "deny", "203.0.113.0/24", "2222")));
        assert!(blocked(check_firewall_add(&deploy_key(), "reject", "203.0.113.7", "any")));
        assert_eq!(check_firewall_add(&deploy_key(), "deny", "198.51.100.0/24", "2222"), Ok(()));
        // Denying my own IP on a non-SSH port cannot cut the app off.
        assert_eq!(check_firewall_add(&deploy_key(), "deny", "203.0.113.0/24", "80"), Ok(()));
        assert_eq!(check_firewall_add(&deploy_key(), "allow", "203.0.113.7", "2222"), Ok(()));
    }

    #[test]
    fn deny_server_port_from_anywhere_blocked() {
        assert!(blocked(check_firewall_add(&deploy_key(), "deny", "any", "2222")));
        assert!(blocked(check_firewall_add(&deploy_key(), "deny", "Anywhere", "2222/tcp")));
        assert_eq!(check_firewall_add(&deploy_key(), "deny", "any", "3306"), Ok(()));
    }

    #[test]
    fn firewall_deny_covering_client_ipv6_is_blocked() {
        let v6 = ConnectionContext { client_ip: "2001:db8::7".to_string(), ..deploy_key() };
        assert!(blocked(check_firewall_add(&v6, "deny", "2001:db8::/32", "2222")));
        assert_eq!(check_firewall_add(&v6, "deny", "2001:db9::/32", "2222"), Ok(()));
        assert_eq!(check_firewall_add(&v6, "deny", "203.0.113.0/24", "2222"), Ok(()));
    }

    #[test]
    fn lock_or_unsudo_login_user_blocked() {
        let accounts = vec![account("deploy", true), account("alice", true)];
        assert!(blocked(check_user_change(&deploy_key(), &accounts, "deploy", UserChange::Lock)));
        assert!(blocked(check_user_change(&deploy_key(), &accounts, "deploy", UserChange::RemoveSudo)));
        assert_eq!(check_user_change(&deploy_key(), &accounts, "alice", UserChange::Lock), Ok(()));
    }

    #[test]
    fn removing_last_sudo_user_blocked() {
        let root_ctx = ctx("root", key());
        let accounts = vec![account("root", true), account("alice", true), account("bob", false)];
        assert!(blocked(check_user_change(&root_ctx, &accounts, "alice", UserChange::RemoveSudo)));
        let accounts = vec![account("root", true), account("alice", true), account("bob", true)];
        assert_eq!(check_user_change(&root_ctx, &accounts, "alice", UserChange::RemoveSudo), Ok(()));
    }

    #[test]
    fn revoking_key_app_uses_blocked() {
        assert!(blocked(check_key_revoke(&deploy_key(), "deploy", "AAAAMYKEY")));
        assert_eq!(check_key_revoke(&deploy_key(), "deploy", "AAAAOTHER"), Ok(()));
        assert_eq!(check_key_revoke(&deploy_key(), "alice", "AAAAMYKEY"), Ok(()));
        assert_eq!(check_key_revoke(&deploy_pw(), "deploy", "AAAAMYKEY"), Ok(()));
    }

    // The UI looks these ids up in vpsLockoutReasons; an English sentence here shows untranslated.
    #[test]
    fn lockout_reasons_are_ids_the_ui_translates() {
        let reason = |r: Result<(), String>| r.unwrap_err().trim_start_matches("LOCKOUT_BLOCKED: ").to_string();
        let off = SshdSettings { password_authentication: false, ..sshd() };
        assert_eq!(reason(check_sshd(&deploy_pw(), &off)), "password_in_use");
        let root_no = SshdSettings { permit_root_login: "no".to_string(), ..sshd() };
        assert_eq!(reason(check_sshd(&ctx("root", key()), &root_no)), "root_in_use");
        assert_eq!(reason(check_firewall_add(&deploy_key(), "deny", "any", "2222")), "closes_ssh_for_everyone");
        assert_eq!(reason(check_key_revoke(&deploy_key(), "deploy", "AAAAMYKEY")), "key_in_use");
    }

    #[test]
    fn source_covers_cidr_and_single_addresses() {
        assert!(source_covers("Anywhere", "203.0.113.7"));
        assert!(source_covers("203.0.113.0/24", "203.0.113.7"));
        assert!(!source_covers("203.0.112.0/24", "203.0.113.7"));
        assert!(source_covers("203.0.113.7", "203.0.113.7"));
        assert!(!source_covers("2001:db8::/32", "203.0.113.7"));
        assert!(source_covers("::/0", "2001:db8::7"));
    }

    // Windows: block thắng allow, nên allow bị block che không còn là đường vào; Linux thì vẫn còn.
    #[test]
    fn windows_delete_ignores_allow_shadowed_by_block() {
        let c = deploy_key();
        let rules = vec![
            rule(1, "ALLOW", "2222/tcp", "Anywhere"),
            { let mut r = rule(2, "ALLOW", "2222/tcp", "203.0.113.0/24"); r.raw_spec = "allowB".into(); r },
            { let mut r = rule(3, "DENY", "2222/tcp", "Anywhere"); r.raw_spec = "blockC".into(); r },
        ];
        assert!(blocked(check_firewall_delete_windows(&c, &rules, &rules[0])));
        assert!(check_firewall_delete(&c, &rules, &rules[0]).is_ok());
        // Không có block thì allow còn lại vẫn là đường vào.
        assert!(check_firewall_delete_windows(&c, &rules[..2], &rules[0]).is_ok());
    }

    // Máy khách IPv6 chỉ có một allow cho /64 của nó: xoá luật đó là tự khoá, nên phải bị chặn.
    #[test]
    fn windows_delete_blocks_only_ipv6_allow() {
        let mut c = deploy_key();
        c.client_ip = "2001:db8::7".to_string();
        let mut r = rule(1, "ALLOW", "2222/tcp", "2001:db8::/64");
        r.v6 = true;
        assert!(blocked(check_firewall_delete_windows(&c, std::slice::from_ref(&r), &r)));
    }
}
