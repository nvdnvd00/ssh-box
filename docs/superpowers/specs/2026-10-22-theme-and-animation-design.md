# SSH-Box Theme System (shadcn/ui Base + Classic Terminal Presets) & UI/UX Animations Design

## 1. Overview & Goals
Upgrade **SSH-Box** with a cohesive, customizable 2-layer theming architecture inspired by **shadcn/ui** for the application shell and classic terminal color schemes (featuring **Monokai Pro** as a primary highlight) for `xterm.js` panes, paired with smooth, hardware-accelerated UI/UX animations and micro-interactions.

## 2. Two-Layer Theme Architecture

### Layer 1: App UI Base Theme (shadcn/ui Semantic Tokens)
Defined via CSS variables in `src/index.css` and bridged into Tailwind CSS v4 via `@theme inline`:
- `--background`, `--foreground`
- `--card`, `--card-foreground`
- `--popover`, `--popover-foreground`
- `--secondary`, `--secondary-foreground`
- `--muted`, `--muted-foreground`
- `--accent`, `--accent-foreground`
- `--border`, `--input`, `--ring`
- `--primary`, `--primary-foreground`

**App Base Presets (`AppThemeId`):**
1. `zinc-dark` (Default shadcn Dark: `#09090b` background, `#121215` card/sidebar, `#27272a` hairline borders)
2. `obsidian-dark` (Cyber Slate Dark: `#0b0e14` background, `#0d1119` card/sidebar, `#1e293b` borders)
3. `stone-dark` (Warm Carbon Dark: `#0c0a09` background, `#141210` card/sidebar, `#292524` borders, ideal companion for Monokai)
4. `midnight-dark` (Deep Indigo/Violet Dark: `#090812` background, `#100e1d` card/sidebar, `#231f38` borders)
5. `zinc-light` (Clean Daylight shadcn Light: `#ffffff` background, `#f4f4f5` sidebar/muted, `#e4e4e7` borders, `#09090b` text)

**Accent Colors (`AccentColorId`):**
- `cyan` (`#06b6d4` / `#22d3ee`)
- `emerald` (`#10b981` / `#34d399`)
- `violet` (`#8b5cf6` / `#a78bfa`)
- `amber` (`#f59e0b` / `#fbbf24`)
- `rose` (`#f43f5e` / `#fb7185`)
- `monokai` (`#a9dc76` / `#ffd866` / `#ff6188` signature accent)

### Layer 2: Terminal Color Schemes (`TerminalPresetId`)
Full 16-color ANSI + background/foreground/cursor/selection palettes for `xterm.js`:
1. `monokai` (**Monokai Pro** — default terminal preset: `#19181a` bg, `#fcfcfa` fg, `#ffd866` cursor, classic `#ff6188` red/pink, `#a9dc76` green, `#ffd866` yellow, `#fc9867` orange/blue, `#ab9df2` purple, `#78dce8` cyan)
2. `tokyo-night` (**Tokyo Night** — `#1a1b26` bg, `#c0caf5` fg, `#7aa2f7` blue, `#bb9af7` purple, `#7dcfff` cyan)
3. `dracula` (**Dracula** — `#1e1f29` bg, `#f8f8f2` fg, `#ff79c6` pink, `#50fa7b` green, `#bd93f9` purple, `#8be9fd` cyan)
4. `catppuccin-mocha` (**Catppuccin Mocha** — `#1e1e2e` bg, `#cdd6f4` fg, `#cba6f7` mauve, `#a6e3a1` green, `#89b4fa` blue)
5. `one-dark` (**One Dark Pro** — `#1e222a` bg, `#abb2bf` fg, `#e06c75` red, `#98c379` green, `#61afef` blue, `#c678dd` purple)
6. `nord` (**Nord** — `#2e3440` bg, `#d8dee9` fg, `#88c0d0` frost cyan, `#81a1c1` blue, `#a3be8c` green)
7. `obsidian` (**Obsidian Cyber** — `#0b0e14` bg, `#e2e8f0` fg, `#38bdf8` cyan cursor)
8. `solarized-light` (**Solarized Light** — `#fdf6e3` bg, `#586e75` fg, `#268bd2` blue, `#859900` green)

**Terminal Customization Options:**
- `syncTermBgWithApp`: boolean (optional toggle to blend terminal background with the active shadcn surface or preserve the authentic terminal preset background)
- `fontSize`: number (`12` to `16`, default `13`)
- `cursorStyle`: `"block" | "bar" | "underline"` (default `"block"`)

## 3. UI/UX & Animation Architecture
1. **Collapsible Sidebars (`HostSidebar` & `TelemetryDrawer`):**
   - Smooth width + opacity + transform transitions (`cubic-bezier(0.16, 1, 0.3, 1)`, 240ms) instead of abrupt DOM unmounting, preserving inner width with overflow clipping so `ResizeObserver` smoothly fits `xterm.js`.
2. **Popover & Modal Choreography:**
   - Entrance keyframe animations (`scale(0.96) -> scale(1)`, `translateY(-4px) -> 0`, `opacity: 0 -> 1`) with subtle backdrop blur.
3. **Host Sidebar Enhancements:**
   - Collapsible host groups with animated chevron rotation.
   - Tactile card hover states (`translate-x-0.5`, subtle border glow, quick action reveal).
4. **Split Workspace Resizer (`SplitWorkspace`):**
   - shadcn-inspired `ResizableHandle` with visible grip pill on hover/drag and active accent highlight.
5. **Telemetry Micro-interactions (`TelemetryDrawer`):**
   - Theme-aware sparklines with smooth SVG transitions, animated progress bars, and hoverable process rows.
6. **Persistent State:**
   - Stored in `localStorage` under `ssh-box:theme-v1` and applied immediately on startup.
