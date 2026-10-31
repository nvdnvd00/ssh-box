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

// The rules that hold on every OS; Windows hides the user and root rows, so it stops here.
pub fn check_sshd_auth(ctx: &ConnectionContext, next: &SshdSettings) -> Result<(), String> {
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
    Ok(())
}

pub fn check_sshd(ctx: &ConnectionContext, next: &SshdSettings) -> Result<(), String> {
    check_sshd_auth(ctx, next)?;
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

// Windows only. The source decides the family (source_covers and the range check compare families), so a keyword item,
// tagged v4 by the parser, still counts for an IPv6 client.
fn blocks_my_ssh(ctx: &ConnectionContext, r: &FirewallRuleItem) -> bool {
    !is_allow(&r.action)
        && rule_port_matches(&r.to_port, &r.protocol, ctx.server_port, "tcp")
        && block_source_covers(&r.from_source, &ctx.client_ip)
}

// Blocks only (never allows): an "a-b" range is matched within its family; a keyword (LocalSubnet, Internet, DNS…)
// or any source we cannot parse may hold the client, so it covers (conservative).
fn block_source_covers(source: &str, ip: &str) -> bool {
    let s = source.trim();
    if let Some((a, b)) = s.split_once('-') {
        return match (a.trim().parse::<IpAddr>(), b.trim().parse::<IpAddr>(), ip.trim().parse::<IpAddr>()) {
            (Ok(a), Ok(b), Ok(i)) if a.is_ipv4() == i.is_ipv4() && b.is_ipv4() == i.is_ipv4() => a <= i && i <= b,
            (Ok(a), Ok(b), Ok(_)) if a.is_ipv4() == b.is_ipv4() => false,
            _ => true,
        };
    }
    let parsed = match s.split_once('/') {
        Some((n, p)) => n.trim().parse::<IpAddr>().is_ok() && p.trim().parse::<u32>().is_ok(),
        None => s.parse::<IpAddr>().is_ok(),
    };
    !parsed || source_covers(s, ip)
}

// Windows Firewall: a block rule wins over any allow, so an allow only counts while no block covers the same ssh path.
pub fn check_firewall_delete_windows(
    ctx: &ConnectionContext,
    rules: &[FirewallRuleItem],
    target: &FirewallRuleItem,
) -> Result<(), String> {
    let removed = |r: &FirewallRuleItem| r.raw_spec == target.raw_spec;
    // Only effective allows carry the client in; blocks count either way (conservative).
    let carries = |r: &FirewallRuleItem| r.effective && allows_my_ssh(ctx, r);
    let removes_mine = std::iter::once(target).chain(rules.iter().filter(|r| removed(r))).any(carries);
    let still_open = rules.iter().any(|r| !removed(r) && carries(r)) && !rules.iter().any(|r| !removed(r) && blocks_my_ssh(ctx, r));
    if removes_mine && !still_open {
        return block("only_ssh_rule");
    }
    Ok(())
}

// Windows: enabling turns every saved block on, and the auto-allow cannot beat a block that covers this session,
// nor a profile that ignores every allow (AllowInboundRules False). The auto-allow is a local rule, so when Group Policy
// ignores local rules only a policy allow can carry this session.
pub fn check_firewall_enable_windows(
    ctx: &ConnectionContext,
    rules: &[FirewallRuleItem],
    inbound_allows_off: bool,
    local_rules_off: bool,
) -> Result<(), String> {
    if inbound_allows_off {
        return block("windows_inbound_allows_off");
    }
    if rules.iter().any(|r| r.effective && blocks_my_ssh(ctx, r)) {
        return block("windows_block_covers_me");
    }
    let effective: Vec<FirewallRuleItem> = rules.iter().filter(|r| r.effective).cloned().collect();
    if local_rules_off && ssh_allow_needed(ctx, &effective) {
        return block("windows_local_rules_off");
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
            effective: true,
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

    // Windows: block beats allow, so an allow shadowed by a block is no way in; on Linux it still is.
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
        // Without the block, the remaining allow is still a way in.
        assert!(check_firewall_delete_windows(&c, &rules[..2], &rules[0]).is_ok());
    }

    // An IPv6 client has a single allow for its /64: deleting it locks the app out, so it must be blocked.
    #[test]
    fn windows_delete_blocks_only_ipv6_allow() {
        let mut c = deploy_key();
        c.client_ip = "2001:db8::7".to_string();
        let mut r = rule(1, "ALLOW", "2222/tcp", "2001:db8::/64");
        r.v6 = true;
        assert!(blocked(check_firewall_delete_windows(&c, std::slice::from_ref(&r), &r)));
    }

    // An allow of an unused profile or tied to a program does not let the client in; it is no fallback path.
    #[test]
    fn windows_delete_ignores_ineffective_backup_allows() {
        let c = deploy_key();
        let real = rule(1, "ALLOW", "2222/tcp", "Anywhere");
        for name in ["inactive-profile", "program-scoped"] {
            let mut backup = rule(2, "ALLOW", "2222/tcp", "Anywhere");
            backup.raw_spec = name.into();
            backup.effective = false;
            assert!(blocked(check_firewall_delete_windows(&c, &[real.clone(), backup.clone()], &real)), "{name}");
            // Deleting the ineffective allow itself removes no way in.
            assert!(check_firewall_delete_windows(&c, &[real.clone(), backup.clone()], &backup).is_ok(), "{name}");
        }
    }

    // From real JSON: the only IPv6 allow for the client's /64; deleting it must be blocked (parser and guard together).
    #[test]
    fn windows_delete_from_parsed_json_blocks_only_ipv6_allow() {
        let j = r#"{"profiles":[{"name":"Private","enabled":true,"defaultInbound":"Block"}],"active":["Private"],"rules":[{"displayName":"ssh6","service":"Any","group":"","program":"Any","profile":"Any","scoped":false,"action":"Allow","localPort":"2222","protocol":"TCP","remoteAddress":"2001:db8::/64"}],"sockets":[]}"#;
        let rules = crate::vps_windows::parse_windows_firewall(j, 22).unwrap().rules;
        let mut c = deploy_key();
        c.client_ip = "2001:db8::7".to_string();
        assert!(blocked(check_firewall_delete_windows(&c, &rules, &rules[0])));
    }

    // A Windows block on a range covering the client, or on a keyword/odd source we cannot resolve (LocalSubnet…), beats the
    // auto-allow: enabling must be refused. A range that misses the client, or of the other family, must not refuse it.
    #[test]
    fn windows_enable_counts_range_and_keyword_blocks() {
        let deny = |from: &str| FirewallRuleItem { v6: from.contains(':'), ..rule(1, "DENY", "2222/tcp", from) };
        let c = deploy_key();
        for from in ["203.0.113.1-203.0.113.50", "LocalSubnet", "Internet", "DefaultGateway", "10.0.0.0/255.0.255.0"] {
            assert!(blocked(check_firewall_enable_windows(&c, &[deny(from)], false, false)), "{from}");
        }
        for from in ["203.0.113.8-203.0.113.50", "2001:db8::1-2001:db8::ff"] {
            assert!(check_firewall_enable_windows(&c, &[deny(from)], false, false).is_ok(), "{from}");
        }
        let c6 = ConnectionContext { client_ip: "2001:db8::7".to_string(), ..deploy_key() };
        assert!(blocked(check_firewall_enable_windows(&c6, &[deny("2001:db8::1-2001:db8::ff")], false, false)));
        assert!(blocked(check_firewall_enable_windows(&c6, &[deny("LocalSubnet")], false, false)), "a keyword item is v4-tagged yet may hold a v6 client");
        assert!(check_firewall_enable_windows(&c6, &[deny("203.0.113.1-203.0.113.50")], false, false).is_ok());
    }

    // Allows are never widened: an allow on LocalSubnet or a range is no known way in, so deleting the real allow stays refused.
    #[test]
    fn windows_keyword_or_range_allow_is_not_my_way_in() {
        let c = deploy_key();
        let real = rule(1, "ALLOW", "2222/tcp", "203.0.113.7");
        for from in ["LocalSubnet", "203.0.113.1-203.0.113.50"] {
            let other = FirewallRuleItem { raw_spec: from.into(), ..rule(2, "ALLOW", "2222/tcp", from) };
            assert!(blocked(check_firewall_delete_windows(&c, &[real.clone(), other], &real)), "{from}");
        }
    }

    // Windows hides AllowUsers; a leftover DOMAIN\\bob entry must not block an unrelated change, while the auth rules still do.
    #[test]
    fn windows_guard_ignores_hidden_allow_users() {
        let next = SshdSettings { allow_users: vec!["DOMAIN\\bob".into()], max_auth_tries: 3, ..sshd() };
        assert_eq!(check_sshd_auth(&deploy_key(), &next), Ok(()));
        assert!(blocked(check_sshd(&deploy_key(), &next)));
        assert!(blocked(check_sshd_auth(&deploy_key(), &SshdSettings { pubkey_authentication: false, ..sshd() })));
    }
}
