# Dashboard Display Modes & Compact Action Buttons Design

**Date:** 2026-10-24
**Scope:** `src/components/DashboardWorkspace.tsx`, `src/components/ResourcesWorkspace.tsx`, `src/types.ts`

## 1. Problem Statement

1. **Dashboard Overview Display Modes (`DashboardWorkspace.tsx`):**
   - Currently supports only one layout: a tall 4-column telemetry card grid (`grid`). When managing many hosts, cards take too much vertical space and cannot be scanned quickly.
   - The top header renders every tag (`All`, `No Tags`, plus each tag in `resources.tags`) as individual horizontal pill buttons, which clutters the header bar. Additionally, tag filtering compares `selectedTagId` (`tag.id`) against `host.tags` (which stores `tag.name`), causing tag filtering on the Dashboard to fail.
2. **Sprawling Buttons in Machines View (`ResourcesWorkspace.tsx`):**
   - **Top Toolbar:** Renders 8 separate controls in a row (`Reload`, `Sort`, `Grid/List`, `Import`, `Export`, `Groups`, `Tags`, `+ New Machine`).
   - **Filter Bar:** Renders all groups (`All Groups`, each group) and all tags as sprawling multi-line pill buttons.
   - **Machine Card Actions & List View:** Each machine card footer has 5 buttons (`Terminal`, `Monitor`, `SFTP`, `Edit`, `Delete`), causing horizontal crowding. Switching `displayStyle` to `"list"` only changes the container from `grid` to `flex-col` without changing the tall vertical card into a compact horizontal row.

## 2. Architecture & Component Design

### 2.1. Dashboard Display Styles (`DashboardWorkspace.tsx` & `types.ts`)

- Define `export type DashboardDisplayStyle = "grid" | "compact" | "list";` in `src/types.ts`.
- Persist the selected mode in `localStorage` under key `"ssh-box:dashboard-display-style"` (defaulting to `"grid"`).
- **Dashboard Top Header:**
  - Replace the sprawling tag pill row with a compact `SelectField` (`size="sm"`) for filtering tags (`ALL` = All Machines, `NO_TAGS` = Untaggable / No Tags, or `tag.name` for each tag in `resources.tags`).
  - Fix tag filter matching so `filteredHosts` checks `(h.tags ?? []).includes(selectedTagFilter)` using the tag's `name` (with backward-compatible fallback to `tag.id`).
  - Add a 3-way segmented view switcher (`Grid` / `Compact` / `List`) using `LayoutGrid`, `Rows3` (or `Grip`), and `List` icons next to the search box and `+ Add Machine` button.
- **Three Display Modes:**
  1. **Detailed Grid (`grid`):**
     - Preserves the existing full telemetry cards (`grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4`) with Cores/RAM/Disk/Uptime metadata, dual `CircularRingGauge` (CPU & RAM), and Network/Disk I/O rates.
  2. **Compact Grid (`compact`):**
     - Dense card grid (`grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-3`).
     - Each card displays:
       - Top row: Machine icon, Name, subtitle (`osName` or `user@hostname:port`), and compact `Terminal` icon button (plus `SFTP` icon button for remote hosts).
       - Middle row: 3 inline mini progress bars with percentage labels for `CPU`, `RAM`, and `Disk`.
       - Bottom row: Single-line footer with `Uptime` on the left and `↑ TX / ↓ RX` network rates on the right.
  3. **List / Table View (`list`):**
     - Semantic responsive table inside a scrollable region (`role="region"`, `aria-label="Machines telemetry table"`, `tabIndex={0}`) with sticky `<thead>` headers.
     - Columns:
       - `Machine`: Icon + Name + `user@hostname:port`
       - `OS / Uptime`: OS badge + uptime string
       - `CPU`: Mini progress bar + percentage (`cpuPercent`)
       - `RAM`: Mini progress bar + percentage & total GB
       - `Disk`: Mini progress bar + percentage
       - `Network`: `↑ TX` and `↓ RX` rates
       - `Actions`: Compact icon buttons for `Terminal`, `SFTP` (remote), and `Docker` (`e.stopPropagation()` so row click still opens Detail Monitor).

### 2.2. Compact Buttons & List Layout in Machines View (`ResourcesWorkspace.tsx`)

1. **Top Toolbar Consolidation:**
   - Keep primary controls visible: `Search` input, `Reload` icon button, `Sort` toggle button, `Grid/List` segmented toggle, and primary `+ New Machine` CTA button.
   - Replace the 4 secondary buttons (`Import`, `Export`, `Groups`, `Tags`) with a single `Manage` dropdown popover button (`SlidersHorizontal` / `MoreHorizontal` icon) that opens a clean popover menu containing:
     - `Manage Groups` (opens Group modal)
     - `Manage Tags` (opens Tag modal)
     - `Import Machines (.json)`
     - `Export Machines (.json)`
   - Clicking outside or pressing `Escape` closes the `Manage` popover.
2. **Compact Group & Tag Filter Bar:**
   - Replace the sprawling pill rows in the `machines` tab with two compact `SelectField` dropdowns (`size="sm"`):
     - Group select: `All Groups (${hosts.length})` + each group in `allGroupNames`.
     - Tag select: `All Tags` + each tag in `resources.tags`.
     - When either filter is active (`!== "ALL"`), show a single compact `Clear` button to reset filters in one click.
3. **Compact Machine Card & True Horizontal List Row:**
   - **In Grid Mode (`displayStyle === "grid"`):**
     - Card action bar has: primary `Terminal` button (with icon + label), compact icon-only buttons for `Monitor` and `SFTP` (with `title` tooltips), and a compact `⋯` (`MoreHorizontal`) menu (or compact icon pair) for `Edit` and `Delete`.
   - **In List Mode (`displayStyle === "list"`):**
     - Render each machine as a single horizontal row (`flex items-center justify-between gap-4 px-4 py-2.5 rounded-xl`) showing:
       - Left: Icon + Alias + `user@hostname:port`
       - Middle: Group badge, auth/key/proxy badges, tag dots/badges, and truncated remark
       - Right: Compact action buttons (`Terminal` button + icon-only `Monitor`, `SFTP`, `Edit`, `Delete`).

## 3. Error Handling & Edge Cases

- **Invalid or missing `localStorage` value:** Fallback safely to `"grid"`.
- **Unmonitored remote hosts:** In `compact` and `list` modes on Dashboard, if `metricsMap[host.id]` is not yet populated, render `—` for percentages and `"Standby / Click to monitor"` for status without layout shift.
- **Outside click / Escape on popovers:** Close the toolbar `Manage` dropdown cleanly using `mousedown` and `keydown` (`Escape`) listeners.

## 4. Verification Plan

- Typecheck and build the frontend via `npm run build` (`tsc && vite build`).
- Verify Dashboard display mode switching (`grid`, `compact`, `list`) and persistence across reloads.
- Verify Tag filtering works accurately in both `DashboardWorkspace` and `ResourcesWorkspace`.
- Verify `Manage` menu actions (`Groups`, `Tags`, `Import`, `Export`) and compact machine actions (`Terminal`, `Monitor`, `SFTP`, `Edit`, `Delete`) in both `grid` and `list` views.
