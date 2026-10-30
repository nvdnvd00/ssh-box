# Tool Installer — Design

Date: 2026-10-31 · Status: draft, awaiting user approval

## 1. Why

When the user opens a fresh machine (mostly Linux VPS) they reinstall the same tools every time — security/audit tools, node, docker, Claude Code, tailscale, utilities. It costs time and they forget things, **especially security**. The installer makes "what is missing on this machine" visible at a glance, puts security first, and installs/uninstalls with a click.

Success: on a new VPS the user opens one screen, sees security gaps first, ticks what they want, installs in one run, and nothing they normally use is forgotten.

## 2. Goals / non-goals / anti-goals

Goals
- Works on Linux, macOS, Windows hosts over SSH, and on the local machine ("Máy này").
- Per-tool status: not installed / installed (+ which package manager installed it) / installed but service not enabled or not running.
- Install, uninstall, enable service. Uninstall uses **the same manager that installed it** (brew-installed → `brew uninstall`, never npm).
- Curated catalog by category + search straight in the package managers' repos.
- Manage the package managers themselves (show present/missing, install missing ones).
- Multi-select and install in one run.

Non-goals
- No "saved tool set" / profiles (decided).
- No configuring tools beyond enabling/starting their service (firewall rules etc. stay in Server Config).
- No version pinning / upgrade management (possible later).

Anti-goals (failure even if it "works")
- Uninstalling with the wrong manager, or removing something the user didn't pick.
- Silent failures: a run that looks done while a step failed.
- Hanging forever on an interactive prompt (sudo password, apt dialog, `y/N`).

## 3. Where it lives

- SSH hosts: new tab "Cài công cụ" in `VpsConfigWorkspace.tsx`, reusing `src/components/vps/shared/` (`StatusBadge`, `ConfirmButton`, `Notice`, `useVpsResource`, `errors.ts`).
- Local machine: an entry "Máy này" that opens **only** the installer (no config tabs).
- Min width 360px; all strings via `getI18n` VI + EN.

## 4. Screen (layout B, chosen by user)

```
[ Tìm gói trong apt / snap / npm…                     ]
 (đang bỏ qua: brew — chưa cài)
[Chưa cài (7)] [Đã cài (5)] [Cần chú ý (1)]
 TRÌNH QUẢN LÝ GÓI  (always shown, in every tab)
 BẢO MẬT
 CHẠY ỨNG DỤNG · AI · MẠNG · TIỆN ÍCH
[ Cài 3 mục đã chọn ]   ← sticky action bar when ≥1 ticked
```

- Tabs: **Chưa cài**, **Đã cài**, **Cần chú ý** (installed service that is disabled/stopped — highlighted, with a **Bật** button). Counts in tab labels.
- Category order fixed: Package managers → Security → Run apps → AI → Network → Utilities.
- Row: name, short description, badge (`nên cài` for recommended), for installed rows the manager (`· apt`), actions.
- Rows in "Chưa cài" have a checkbox; the sticky bar installs all ticked.
- If the tool's only usable manager is missing: Install disabled, text "Cần cài brew trước" + button to install that manager.
- Uninstall goes through `ConfirmButton` and states the manager: "Gỡ docker bằng apt?".

## 5. Package managers

Per OS (my call, adjustable):

| OS | Managers | How a missing one is installed |
|---|---|---|
| Linux | apt / dnf (whichever the distro has; not installable), snap, npm | snap: via apt/dnf; npm: comes with node |
| macOS | brew, npm | brew: official install script (non-interactive) |
| Windows | winget, npm (scoop optional, later) | winget: if missing, show guidance to install App Installer (not automated) |

Detection: `command -v` (sh) / `Get-Command` (PowerShell).

## 6. Catalog

A static file in the repo (JSON/TS), per tool:
`id, name, category, recommended, description(vi,en), bin` (program name to look for, e.g. ripgrep → `rg`)`, methods[]` where each method = `{ os, manager, package, service? }`, ordered by preference. The first method whose manager is present is used. `service` (systemd unit / launchd / Windows service name) enables the "Cần chú ý" check and the Bật action.

Initial content (my call): Security — ufw, fail2ban, unattended-upgrades, lynis, rkhunter, (Windows: none built-in beyond Defender status → skip v1); Run apps — git, node, docker, python3, nginx; AI — Claude Code (npm `@anthropic-ai/claude-code`); Network — tailscale, curl, wget; Utilities — htop, tmux, jq, ripgrep. Exact list is the builder's call per OS; tools with no method for an OS are hidden on that OS.

## 7. Detecting "installed by which manager"

One read-only probe script per OS runs all checks in a single SSH call and returns JSON:
- Linux: `dpkg-query -W` / `rpm -q`, `snap list`, `npm ls -g --depth=0 --json`.
- macOS: `brew list --formula` + `--cask`, `npm ls -g`.
- Windows: `winget list` (parse by id), `npm ls -g`.

A tool is "installed via M" if its package appears in M's list. If the binary exists but no known manager owns it → status "Đã cài (ngoài trình quản lý gói)" and **no Uninstall button** (avoids wrong-manager removal). Tools installed by an official script (tailscale `curl|sh`, Claude Code native) fall here too — decided by user: shown as installed, no Uninstall. If several managers own it, list all and uninstall offers a choice.

Service state: `systemctl is-enabled/is-active`, `launchctl`/`brew services list`, `Get-Service`.

## 8. Running actions

- Linux/macOS: reuse `run_privileged_script` (sudo-aware) from `vps_config.rs`; brew and npm-global-under-user run **without** sudo.
- Windows: reuse `run_windows_script` (stdin-fed PowerShell). Usernames with spaces must be quoted (§8.1 of the multi-OS spec).
- Local: reuse `run_host_or_local_cmd` from `telemetry.rs`. Local sudo needs a password prompt → builder decides (simplest: ask for password once per run in a dialog, never stored).
- Local Windows (decided by user: in v1): run PowerShell locally without elevation; winget installers that need admin raise their own UAC prompt. Actions that need admin themselves (Start-Service/enable, uninstall of machine-wide packages) launch an elevated PowerShell (`Start-Process -Verb RunAs`, output via temp file) — one UAC prompt. Some installers ignore `--silent` and may show a window mid-batch; show a note. Test on an own disposable Windows VM.
- Non-interactive always: `DEBIAN_FRONTEND=noninteractive apt-get -y`, `brew` with `NONINTERACTIVE=1`, `winget --accept-source-agreements --accept-package-agreements --silent`.
- Every command gets a timeout (lesson from sshd -t hangs). Install timeout generous (e.g. 10 min), probe short.
- Package names come only from the catalog or search results and are shell-quoted (`ps_quote` on Windows); never interpolate raw user text.

Batch: ticked items run **one after another** (managers like apt hold a lock), a failure does not stop the rest. A progress panel shows each item: waiting / running / done / failed, with the command output tail expandable. After the run, re-probe and refresh tabs.

Ordering (my call): a batch always runs package-manager installs first (incl. node when npm is needed, snapd when snap is needed), then tools. Tools depending on a manager ticked in the same batch are allowed.

SSH sudo: `run_privileged_script` already takes an optional sudo password and the probe reports `nopasswd / password_ok / none`; reuse Server Config's existing sudo-password prompt — no new mechanism.

## 9. Search

Searches only managers present on the machine, in parallel per manager, results grouped by manager with Install buttons: `apt-cache search` / `dnf search`, `snap find`, `brew search`, `winget search`, `npm search`. Results can be ticked into the same batch as catalog items (my call). Debounced; capped at ~20 results per manager. A note lists managers skipped because missing.

## 10. Best-practice extras (proposed; drop any you don't want)

1. Security "nên cài" badges + count of missing recommended security tools on the tab.
2. Refresh package index once before a batch (`apt-get update`) instead of per item.
3. Show the exact command before running (reuses the preview idea of `ChangePreviewDialog`) — collapsed by default.
4. Keep the last run's log until the screen closes, so a failure can be read after the fact.

## 11. Testing

- Rust unit tests: probe-output parsers per manager, catalog method selection, command building/quoting (incl. names with spaces).
- Real runs only on **disposable VMs / temp environments**, never real servers, and **not** the UTM VMs used by the multi-OS session (macOS 192.168.64.5, Windows 192.168.65.2) — create own disposable ones.

## 12. Dependencies

Builds on the unmerged `feat/server-config-multi-os` work (OS detection, Windows runner, privileged runner). Implementation branches from it and uses its helpers as-is (my call); if review changes them, adapt then — no wrapper layer.

## 13. Left to the builder

Exact catalog entries per OS, timeout values, how local sudo password is asked, debounce/caps for search, icon choices.
