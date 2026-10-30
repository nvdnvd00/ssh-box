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
T=$(mktemp)
cat "$SSHBOX_SECRET_FILE" > "$T"
install -m 644 "$T" "$F"
rm -f "$T"
if ! sshd -t 2>&1; then sshbox_restore; echo SSHBOX_SSHD_INVALID; exit 1; fi
cat > "$R" << 'SSHBOX_ROLLBACK_EOF'
{rollback}SSHBOX_ROLLBACK_EOF
chmod 700 "$R"
cat > "$PL" << 'SSHBOX_PLIST_EOF'
{plist}SSHBOX_PLIST_EOF
chmod 644 "$PL"
trap 'rc=$?; [ "$rc" -eq 0 ] || /bin/sh "$R"; exit "$rc"' EXIT
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
