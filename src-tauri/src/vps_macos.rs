// sshd editing on macOS (spec §6.1): same flow as Linux, but the 60s rollback timer is a launchd job.
use crate::vps_config::{rollback_pending_cmd, shell_single_quote, sshd_restore_fn};
use crate::vps_support::OsFamily;

pub const MACOS_ROLLBACK_LABEL: &str = "com.sshbox.rollback";
pub const MACOS_ROLLBACK_PLIST: &str = "/Library/LaunchDaemons/com.sshbox.rollback.plist";

// bootout kills the process running this script, so it must stay the last line.
pub fn build_sshd_rollback_script_macos(file: &str, plist_path: &str) -> String {
    format!(
        "#!/bin/sh\n{restore}sshbox_restore\nrm -f {plist}\nrm -f \"$0\"\nlaunchctl bootout system/{MACOS_ROLLBACK_LABEL}\n",
        restore = sshd_restore_fn(file),
        plist = shell_single_quote(plist_path),
    )
}

// The new file content comes from $SSHBOX_SECRET_FILE. launchd (not the SSH session) owns the timer.
pub fn build_sshd_apply_script_macos(file: &str, rollback_path: &str, plist_path: &str) -> String {
    let plist = format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<plist version=\"1.0\"><dict>\n\
<key>Label</key><string>{MACOS_ROLLBACK_LABEL}</string>\n\
<key>ProgramArguments</key><array><string>/bin/sh</string><string>-c</string>\
<string>sleep 60; exec /bin/sh {rollback_path}</string></array>\n\
<key>RunAtLoad</key><true/>\n</dict></plist>\n"
    );
    format!(
        r#"set -eu
{restore}R={r}
PL={pl}
if [ "$({pending})" = active ]; then echo SSHBOX_ROLLBACK_PENDING; exit 3; fi
if [ -e "$F" ]; then cp -p "$F" "$F.sshbox.bak"; rm -f "$F.sshbox.absent"; else : > "$F.sshbox.absent"; rm -f "$F.sshbox.bak"; fi
# Until the rollback is armed, any failure puts the old file back.
trap 'rc=$?; [ "$rc" -eq 0 ] || sshbox_restore; exit "$rc"' EXIT
T=$(mktemp)
cat "$SSHBOX_SECRET_FILE" > "$T"
install -m 644 "$T" "$F"
rm -f "$T"
if ! sshd -t 2>&1; then sshbox_restore; echo SSHBOX_SSHD_INVALID; exit 1; fi
cat > "$R" << 'SSHBOX_ROLLBACK_EOF'
{rollback}SSHBOX_ROLLBACK_EOF
chmod 700 "$R"
trap 'rc=$?; [ "$rc" -eq 0 ] || /bin/sh "$R"; exit "$rc"' EXIT
cat > "$PL" << 'SSHBOX_PLIST_EOF'
{plist}SSHBOX_PLIST_EOF
chmod 644 "$PL"
launchctl bootstrap system "$PL"
echo "NOW:$(date +%s)"
echo "ROLLBACK_AT:$(( $(date +%s) + 60 ))"
"#,
        restore = sshd_restore_fn(file),
        r = shell_single_quote(rollback_path),
        pl = shell_single_quote(plist_path),
        pending = rollback_pending_cmd(OsFamily::Macos),
        rollback = build_sshd_rollback_script_macos(file, plist_path),
    )
}

// Same sections as the Linux ACCOUNTS_READ_SCRIPT so parse_access_keys_snapshot is reused; macOS has no shadow file.
pub const MACOS_ACCOUNTS_READ_SCRIPT: &str = r#"echo "TODAY:$(( $(date +%s) / 86400 ))"
echo "MIN_UID:501"
echo "---SUDOERS---"
grep -vhE '^[[:space:]]*(#|$)' /etc/sudoers /etc/sudoers.d/* 2>/dev/null || true
dscl . -list /Users UniqueID 2>/dev/null | while read -r usr uid; do
  case "$usr" in _*) continue ;; esac
  [ "$usr" = root ] || [ "$uid" -ge 501 ] 2>/dev/null || continue
  dsget() { dscl . -read "/Users/$usr" "$1" 2>/dev/null | sed -n "s/^$1: //p"; }
  # dscl lists several homes for root; dscacheutil gives the single effective one (spaces stay intact).
  home=$(dscacheutil -q user -a name "$usr" 2>/dev/null | sed -n 's/^dir: //p')
  echo "---ACCOUNT---"
  echo "PASSWD:$usr:*:$uid:$(dsget PrimaryGroupID)::$home:$(dsget UserShell)"
  echo "GROUPS:$(id -Gn "$usr" 2>/dev/null | tr '\n' ' ')"
  echo "SHADOW:"
  echo "---AUTHORIZED_KEYS---"
  if [ -n "$home" ] && [ -f "$home/.ssh/authorized_keys" ]; then cat "$home/.ssh/authorized_keys" 2>/dev/null || true; fi
done
"#;
