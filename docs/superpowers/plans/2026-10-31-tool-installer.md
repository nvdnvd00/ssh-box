# Tool Installer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A "Cài công cụ" screen that shows per-machine tool status (by category, security first), installs/uninstalls with the manager that owns each tool, enables stopped services, searches package repos, and batch-installs — on Linux/macOS/Windows over SSH and on the local machine.

**Architecture:** One Rust module `installer.rs` holds the catalog (embedded JSON), pure script builders and output parsers (unit-tested), and thin runners that reuse existing remote runners plus a new local runner with timeouts. Three Tauri commands (`installer_scan`, `installer_search`, `installer_run`). The frontend `InstallerStudio.tsx` drives batches item-by-item via `installer_run`, so progress is just React state.

**Tech Stack:** Rust (tokio, serde, serde_json — already deps), Tauri v2 commands, React 19 + TS, existing `src/components/vps/shared/*`.

**Spec:** `docs/superpowers/specs/2026-10-31-tool-installer-design.md`

## Global Constraints

- All UI strings via `getI18n`, VI + EN, in `src/i18n.ts`.
- Min width 360px; reuse `StatusBadge`, `ConfirmButton`, `Notice`, `useVpsResource`, `vpsErrorText`.
- Uninstall only with a manager that lists the package; tools not owned by any manager (incl. official-script installs) show "installed", no Uninstall.
- Non-interactive always: `DEBIAN_FRONTEND=noninteractive apt-get -y`, `dnf -y`, `NONINTERACTIVE=1` for brew, `winget --silent --accept-source-agreements --accept-package-agreements --disable-interactivity`.
- Every command bounded: scan 60s, search 30s, action 600s.
- Package/service names only from the catalog or search results, validated against `^[A-Za-z0-9@/._+:-]{1,128}$` before entering any script; Windows strings go through `ps_quote`.
- No new dependencies. No saved tool sets.
- Testing only on disposable machines. The UTM Windows VM (192.168.65.2) may be used **only after** the `feat/server-config-multi-os` session has finished — check first. Never edit that branch/worktree.
- Branch from `feat/server-config-multi-os` tip (this worktree already contains its commits; rebase onto its latest tip before Task 4 if it moved).

## Review Focus

1. Package installed by two managers (e.g. node via apt and npm-less brew) → both shown, Uninstall asks which; never removes both.
2. Sudo password needed mid-batch on SSH host → `SUDO_REQUIRED` opens the existing sudo prompt and the batch stops cleanly (remaining items stay "waiting"), not marked failed.
3. Manager missing for a ticked tool whose manager is ticked in the same batch → manager runs first, tool then succeeds.
4. Windows username with spaces / package ids with dots (`Tailscale.Tailscale`) → quoted correctly, scan still parses.
5. Search query with shell metacharacters (`; rm -rf`) → rejected by validation, no command sent.

---

### Task 1: Catalog and core types

**Files:**
- Create: `src-tauri/src/installer.rs`, `src-tauri/src/installer_catalog.json`
- Modify: `src-tauri/src/lib.rs` (add `pub mod installer;`)

**Interfaces:**
- Produces (all `#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]`, `#[serde(rename_all = "camelCase")]`):
  - `enum HostOs { Linux, Macos, Windows }` (serde lowercase)
  - `struct Method { os: HostOs, manager: String, package: String, service: Option<String> }`
  - `struct CatalogTool { id, name, category: String, recommended: bool, description_vi, description_en: String, bin: Option<String>, methods: Vec<Method> }`
  - `fn catalog() -> &'static [CatalogTool]` (parse `include_str!` once via `OnceLock`)
  - `fn managers_for(os: HostOs) -> &'static [&'static str]` — Linux `["apt","dnf","snap","npm"]`, Macos `["brew","npm"]`, Windows `["winget","npm"]`
  - `fn pick_method<'a>(tool: &'a CatalogTool, os: HostOs, present: &[String]) -> Option<&'a Method>` — first method for `os` whose manager is in `present`
  - `fn valid_name(s: &str) -> bool` (regex from Global Constraints, hand-written char check)
- Categories (fixed order): `managers`, `security`, `apps`, `ai`, `network`, `utils`.

- [ ] **Step 1: Write failing tests** in `installer.rs` `#[cfg(test)] mod tests`:
  - `catalog_parses_and_ids_unique` — `catalog()` non-empty, ids unique, every method's manager ∈ `managers_for(method.os)`, every package `valid_name`.
  - `pick_method_prefers_first_present` — for a tool with Linux methods `[apt, snap]`: present `["snap"]` → snap; present `["apt","snap"]` → apt; present `[]` → None.
  - `valid_name_rejects_shell` — accepts `@anthropic-ai/claude-code`, `Tailscale.Tailscale`, `python3`; rejects `a;b`, `a b`, `$(x)`, `""`, 129 chars.
- [ ] **Step 2:** `cd src-tauri && cargo test installer::` → FAIL (not defined).
- [ ] **Step 3: Implement** types + functions; fill catalog per spec §6: security ufw, fail2ban (service), unattended-upgrades (service `unattended-upgrades`), lynis, rkhunter; apps git, node (apt `nodejs`, brew `node`, winget `OpenJS.NodeJS.LTS`), docker (service `docker`), python3, nginx (service); ai claude-code (npm `@anthropic-ai/claude-code`); network tailscale (apt/brew/winget, service `tailscaled`/`Tailscale`), curl, wget; utils htop, tmux, jq, ripgrep (`bin: "rg"`). Managers category entries: snapd (apt), node-for-npm is the `node` tool; brew/winget handled in Task 3 not catalog.
- [ ] **Step 4:** `cargo test installer::` → PASS.
- [ ] **Step 5: Commit** `feat(installer): tool catalog and core types`.

### Task 2: Scan script and parser

**Files:** Modify `src-tauri/src/installer.rs`

**Interfaces:**
- Produces:
  - `fn scan_script(os: HostOs) -> String` — prints lines only of these forms:
    `MGR:<mgr>:<0|1>`, `PKG:<mgr>:<package>`, `BIN:<bin>:<0|1>`, `SVC:<service>:<enabled 0|1>:<active 0|1>`.
    sh (Linux/macOS): `command -v`, `dpkg-query -W -f='${Package}\n'`/`rpm -qa --qf '%{NAME}\n'`, `snap list`, `npm ls -g --depth=0 --parseable` (basename after `node_modules/`), `brew list --formula -1` + `--cask -1`, `systemctl is-enabled/is-active` (Linux), `brew services list` (macOS). PowerShell: `Get-Command`, `winget list --source winget` (ids from catalog matched with `-match [regex]::Escape(id)`), `Get-Service`. Bins/services come from the catalog, quoted.
  - `enum ToolStatus { NotInstalled, Installed { via: Vec<String> }, External }` (serde tag `kind`)
  - `struct ServiceState { enabled: bool, active: bool }`
  - `struct ManagerState { id: String, present: bool }`
  - `struct ToolView { id, name, category, recommended, description_vi, description_en, status: ToolStatus, service: Option<String>, service_state: Option<ServiceState>, install: Option<Method>, blocked_by: Option<String> }` — `install` = `pick_method` among present managers; `blocked_by` = manager of the first method for this OS when `install` is None.
  - `struct InstallerScan { os: HostOs, managers: Vec<ManagerState>, tools: Vec<ToolView> }` (tools with no method for `os` omitted)
  - `fn parse_scan(os: HostOs, raw: &str) -> InstallerScan` — unknown lines ignored.
- Rule: `Installed{via}` = managers whose `PKG` list contains the method's package for that manager; else `External` if `BIN:<bin or package>:1`; else `NotInstalled`.

- [ ] **Step 1: Failing tests:**
  - `parse_scan_linux_owner_detection` — raw with `MGR:apt:1`, `PKG:apt:docker.io`(catalog package), `SVC:docker:1:0` → docker `Installed{via:["apt"]}`, `service_state {enabled:true, active:false}`.
  - `parse_scan_two_owners` — node in `PKG:apt:nodejs` and `PKG:snap:node` → `via == ["apt","snap"]`.
  - `parse_scan_external_has_no_owner` — `BIN:tailscale:1`, no PKG → `External`.
  - `parse_scan_blocked_by_missing_manager` — macOS, `MGR:brew:0` → a brew-only tool has `install: None, blocked_by: Some("brew")`.
  - `scan_script_quotes_catalog_names` — Windows script contains `'Tailscale.Tailscale'` single-quoted, sh script contains no unquoted `$(`.
- [ ] **Step 2:** run → FAIL.
- [ ] **Step 3: Implement.**
- [ ] **Step 4:** `cargo test installer::` → PASS.
- [ ] **Step 5: Commit** `feat(installer): scan script and parser`.

### Task 3: Action and search scripts

**Files:** Modify `src-tauri/src/installer.rs`

**Interfaces:**
- Produces:
  - `enum InstallAction { Install{manager,package}, Uninstall{manager,package}, EnableService{service}, InstallManager{manager}, Refresh }` (serde tag `kind`, camelCase)
  - `fn action_script(os: HostOs, a: &InstallAction) -> Result<String, String>` — `Err` if any name fails `valid_name` or manager not in `managers_for(os)`. Use: apt `apt-get install -y PKG` / `apt-get remove -y PKG`; dnf `dnf install -y` / `dnf remove -y`; snap `snap install` / `snap remove`; npm `npm install -g` / `npm uninstall -g`; brew `brew install` / `brew uninstall` (cask auto-detected by brew); winget `winget install --id PKG -e …flags` / `winget uninstall --id PKG -e --silent`. EnableService: `systemctl enable --now`, `brew services start`, `Set-Service -StartupType Automatic; Start-Service`. InstallManager: brew → official script with `NONINTERACTIVE=1`; snap → `apt-get install -y snapd`; npm → install `node` via first present system manager; winget → `Err("WINGET_MANUAL")`.
  - `fn needs_root(os: HostOs, a: &InstallAction) -> bool` — true for apt/dnf/snap/systemctl on Linux; false for brew, npm, all macOS brew services, Windows.
  - `Refresh`: Linux apt → `apt-get update -qq`, dnf → `dnf makecache -q`; other OS → `Err`. `needs_root` true on Linux.
  - `struct SearchHit { manager: String, package: String, description: String }`
  - `fn search_script(os: HostOs, present: &[String], query: &str) -> Result<String,String>` — validate query; per manager prefix lines `HIT:<mgr>:<pkg>:<desc>`; ≤20 per manager.
  - `fn parse_search(raw: &str) -> Vec<SearchHit>`
- [ ] **Step 1: Failing tests:** `action_apt_install_noninteractive` (contains `DEBIAN_FRONTEND=noninteractive` and `apt-get install -y 'fail2ban'`), `action_rejects_bad_name` (`"a;b"` → Err), `uninstall_uses_given_manager` (brew uninstall never contains `npm`), `winget_flags_present`, `needs_root_matrix` (apt true, brew false, npm on macOS false), `search_rejects_metachars`, `parse_search_splits_desc_with_colons` (`HIT:apt:jq:JSON: processor` → desc `JSON: processor`).
- [ ] **Step 2:** FAIL. **Step 3:** implement. **Step 4:** PASS.
- [ ] **Step 5: Commit** `feat(installer): action and search scripts`.

### Task 4: Runners and Tauri commands

**Files:**
- Modify: `src-tauri/src/installer.rs`, `src-tauri/src/lib.rs` (commands + `invoke_handler`)

**Interfaces:**
- Consumes: `vps_config::run_privileged_script(host_id, pool, sudo_password, script, None)`, `vps_windows::run_windows_script(host_id, pool, script, None)`, `vps_config::probe` result's `os_family` for remote OS.
- Produces:
  - `async fn run_local(os: HostOs, script: &str, sudo_password: Option<&str>, root: bool, elevated: bool) -> Result<String,String>` — Unix: `sh -s` with stdin = script (when `root`, script is wrapped `sudo -S -p '' sh -c …` with password as first stdin line; missing password → `Err` coded `SUDO_REQUIRED` like `vps_config`). Windows: `powershell.exe -NoProfile -NonInteractive -Command -` via stdin, `CREATE_NO_WINDOW`; when `elevated`, outer `Start-Process powershell -Verb RunAs -Wait` writing output to a temp file then read back.
  - `async fn run(host_id, pool, os, script, sudo_password, root, elevated, secs: u64)` — routes `host_id == "local"` → `run_local`, Windows → `run_windows_script`, else root → `run_privileged_script` / non-root → `pool.exec` plain (`run_host_or_local_cmd`); wrapped in `tokio::time::timeout`, timeout → `Err` coded `TIMEOUT`.
  - Tauri commands (camelCase args from TS):
    - `installer_scan(host_id: String, os: HostOs, sudo_password: Option<String>) -> Result<InstallerScan,String>` (60s)
    - `installer_search(host_id, os, present: Vec<String>, query: String) -> Result<Vec<SearchHit>,String>` (30s)
    - `installer_run(host_id, os, action: InstallAction, sudo_password: Option<String>) -> Result<String,String>` (600s; elevated = local Windows && action is Uninstall/EnableService)
  - Local OS: `HostOs` from `cfg!(target_os)`, exposed as `installer_local_os() -> HostOs`.
- [ ] **Step 1: Failing test** `run_local_times_out` (`#[tokio::test]`, Unix only): `run` with script `sleep 5`, secs 1, host "local" → Err contains `TIMEOUT`. And `run_local_echo` → output `hi`.
- [ ] **Step 2:** FAIL. **Step 3:** implement + register commands. **Step 4:** `cargo test installer:: && cargo clippy` → PASS, no new warnings.
- [ ] **Step 5: Commit** `feat(installer): runners and tauri commands`.

### Task 5: Frontend types and i18n

**Files:** Modify `src/types.ts`, `src/i18n.ts`, `src/components/vps/shared/errors.ts` (map `TIMEOUT`, `WINGET_MANUAL`)

**Interfaces:**
- Produces TS mirrors of `HostOs`, `InstallerScan`, `ToolView`, `ToolStatus`, `InstallAction`, `SearchHit`, `ManagerState`; extend `VpsStudioTab` with `"installer"`.
- i18n keys (VI/EN both): `vpsTabInstaller` ("Cài công cụ"/"Tools"), `instTabMissing`, `instTabInstalled`, `instTabAttention`, `instCat.{managers,security,apps,ai,network,utils}` ("Trình quản lý gói","Bảo mật","Chạy ứng dụng","AI","Mạng","Tiện ích"), `instRecommended` ("nên cài"), `instInstall` ("Cài"), `instUninstall` ("Gỡ"), `instEnable` ("Bật"), `instInstallSelected(n)` ("Cài {n} mục đã chọn"), `instNeedsManager(m)` ("Cần cài {m} trước"), `instUninstallConfirm(name,m)` ("Gỡ {name} bằng {m}?"), `instExternal` ("Đã cài (ngoài trình quản lý gói)"), `instNotRunning` ("Đã cài · chưa bật"), `instSearchPlaceholder` ("Tìm gói trong …"), `instSearchSkipped(list)`, `instStatus.{waiting,running,done,failed}`, `instWindowsPopupNote`, `instWingetManual`, `vpsErrTimeout`.
- [ ] **Step 1:** add; **Step 2:** `npx tsc --noEmit` → PASS. **Step 3: Commit** `feat(installer): frontend types and strings`.

### Task 6: InstallerStudio UI

**Files:** Create `src/components/vps/InstallerStudio.tsx`

**Interfaces:**
- Consumes: `VpsCtx` from `useVpsResource.ts`; Task 4 commands; Task 5 types/strings.
- Produces: `InstallerStudio: React.FC<{ vps: VpsCtx; os: HostOs }>`.
- Behaviour (spec §4, §8, §9):
  - `useVpsResource(vps, (pw) => invoke("installer_scan", { hostId, os, sudoPassword: pw }))`.
  - Tabs Chưa cài / Đã cài / Cần chú ý with counts; Cần chú ý = installed with `serviceState` and `!(enabled && active)`, rows highlighted (warn tone) with Bật.
  - Managers group first in every tab; then categories in fixed order.
  - Missing rows: checkbox (disabled when `blockedBy` and that manager not ticked), "nên cài" `StatusBadge`, `instNeedsManager` + button to tick the manager.
  - Installed rows: `· via` list; Uninstall via `ConfirmButton` with `instUninstallConfirm`; if `via.length > 1` a small select chooses the manager first; `External` → no Uninstall.
  - Sticky bar `instInstallSelected(n)`. Batch: build actions, order = `Refresh` (Linux only) → `InstallManager`s → tool installs; run sequentially with `installer_run`; per-item status + expandable output tail (last 40 lines); failure continues; `SUDO_REQUIRED` → `vps.requestSudo()` and stop, rest stay waiting. After batch → `refresh()`.
  - Search: debounced 400ms, ≥2 chars, results grouped by manager with checkbox joining the same batch; skipped managers note.
  - Local Windows: show `instWindowsPopupNote`.
  - Layout ok at 360px (rows wrap; actions below name on narrow).
- [ ] **Step 1:** implement. **Step 2:** `npx tsc --noEmit && npm run build` → PASS. **Step 3: Commit** `feat(installer): installer studio UI`.

### Task 7: Wire into the workspace

**Files:** Modify `src/components/VpsConfigWorkspace.tsx`

- Remote: add tab `{ id: "installer", label: t.vpsTabInstaller }` always shown when `probe` exists (not gated by `probe.support`); render `<InstallerStudio vps={vps} os={probe.osFamily} />`.
- Local (`isLocal`): replace the `linux_only` notice with `<InstallerStudio vps={vps} os={localOs} />` where `localOs` comes from `invoke("installer_local_os")` once; no tab bar, no probe; `vps.hostId = "local"`; sudo prompt reused (it already lives in this component) — `requestSudo` opens it and `sudoPasswords["local"]` stores the result without calling `vps_probe` for local (validate by retrying the action).
- [ ] **Step 1:** implement. **Step 2:** `npx tsc --noEmit && npm run build` PASS. **Step 3:** browser preview check per memory "Browser preview testing" at 360px and desktop: tabs, counts, highlight, sticky bar render with a stubbed scan. **Step 4: Commit** `feat(installer): installer tab and local machine view`.

### Task 8: Verify on disposable machines

- [ ] Linux: disposable container/VM with sshd (never a real server). Scan, batch install fail2ban+jq+node, Bật fail2ban, uninstall jq via apt, search `ripgrep`. Expected: statuses flip after each, apt lock never collides.
- [ ] macOS local: isolated app run (memory "Isolated desktop run"). Scan; install/uninstall `jq` via brew; confirm npm never used for a brew package.
- [ ] Windows: **only after confirming** the multi-OS session is done, use UTM Windows VM: scan, install `jq` via winget, uninstall, Bật a stopped service. Username-with-spaces account if available.
- [ ] Record results (pass/fail per row) in the spec's appendix; commit `docs(installer): VM verification results`.
