<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/brand/ssh-box-logo-dark.svg" />
  <source media="(prefers-color-scheme: light)" srcset="docs/brand/ssh-box-logo-light.svg" />
  <img alt="SSH Box" src="docs/brand/ssh-box-logo-dark.svg" width="360" />
</picture>

<br />

### The Modern SSH Client, 120fps Terminal & Server Manager
*Fast, native, and crafted for developers and sysadmins on macOS, Windows, and Linux.*

[![Latest Release](https://img.shields.io/github/v/release/nvdnvd00/ssh-box?style=flat-square&logo=github&label=release&color=2563eb)](https://github.com/nvdnvd00/ssh-box/releases/latest)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-10b981?style=flat-square)](#download)

<p align="center">
  <strong>🌐 English</strong>
  &nbsp;·&nbsp;
  <a href="README.vi.md"><strong>🇻🇳 Tiếng Việt</strong></a>
</p>

<p align="center">
  <a href="#download"><strong>⬇️ Download</strong></a>
  &nbsp;·&nbsp;
  <a href="#features"><strong>✨ Features</strong></a>
  &nbsp;·&nbsp;
  <a href="#shortcuts"><strong>⌨️ Shortcuts</strong></a>
  &nbsp;·&nbsp;
  <a href="#build"><strong>🛠️ Build</strong></a>
</p>

</div>

---

## ⚡ Why SSH Box?

**SSH Box** brings together the speed of a native **120fps GPU-accelerated terminal** and an **all-in-one server workspace**. Instead of switching between a terminal emulator, an SFTP client, a server monitor, and manual config files, everything you need to manage your servers lives in one lightweight desktop app.

---

<a id="download"></a>

## ⬇️ Download

Get the latest version from [**GitHub Releases**](https://github.com/nvdnvd00/ssh-box/releases/latest):

| Platform | Package | Download |
| :--- | :--- | :--- |
| **macOS** (`Apple Silicon` & `Intel`) | Universal `.dmg` | [**Download `ssh-box.dmg`**](https://github.com/nvdnvd00/ssh-box/releases/latest/download/ssh-box.dmg) |
| **Windows** (`64-bit`) | Installer `.exe` | [**Download `ssh-box-setup.exe`**](https://github.com/nvdnvd00/ssh-box/releases/latest/download/ssh-box-setup.exe) |

> [!TIP]
> - **macOS First Launch:** If Gatekeeper blocks the app on first open, run:
>   ```bash
>   xattr -cr "/Applications/SSH Box.app"
>   ```
> - **Auto-Updates:** SSH Box checks for new releases and updates directly from **Settings → Updates**.

---

<a id="features"></a>

## ✨ Features

### 🖥️ 120fps Terminal & Split Panes
- **Silky-Smooth Rendering:** GPU-powered terminal that stays fast and responsive even under heavy logs, `btop`, `tmux`, or `nvim`.
- **Flexible Split Workspace:** Split any tab vertically or horizontally into multiple SSH and local shell panes.
- **Always Ready:** Instant one-key reconnect when a network drops, optional automatic `tmux` session persistence, and a global **Command Palette** (`⌘K` / `Ctrl+Shift+P`).

### 📊 Live Server Dashboard & Docker Control
- **Real-Time Health Monitoring:** Track CPU load per core, RAM & Swap, Disk usage, live Network RX/TX speeds, CPU temperature, and top processes at a glance.
- **Docker Manager:** Inspect running and stopped containers or images, and start, stop, restart, or remove them with a click.

### 🛡️ Visual Server Config & One-Click Tool Installer
- **Safe Server Hardening:** Visually configure **SSH settings**, **Firewall rules**, and **Users & SSH Access Keys** across Linux, macOS, and Windows servers.
- **Anti-Lockout Protection:** Preview every change (`old → new`) before applying. SSH Box verifies new settings before committing and automatically rolls back if a change would disconnect you.
- **One-Click Tool Installer:** Audit and install essential security and developer tools (`UFW`, `Fail2ban`, `Unattended Upgrades`, `Docker`, `Git`, `Node.js`, `Nginx`, `Tailscale`, `Claude Code`, `tmux`, `ripgrep`, `htop`, and more) on remote servers or your local machine.

### 📂 Dual-Pane SFTP File Manager
- **Side-by-Side Local ↔ Remote Browser:** Transfer files and folders effortlessly between your computer and server.
- **Built-in Editing & Permissions:** Edit remote text files directly inside the app, manage folders, and update file permissions (`chmod`) visually.

### 🔐 Keychain Security & Network Toolbox
- **Instant `~/.ssh/config` Sync:** Automatically imports your existing SSH config and lets you organize servers with custom groups and color tags.
- **Native System Keychain:** Passwords and SSH key passphrases are protected by **macOS Keychain**, **Windows Credential Manager**, or **Linux Secret Service**.
- **Complete SSH Toolkit:** Built-in **SSH Key Generator**, reusable **Identities**, **Jump Hosts (Bastion)**, **SOCKS5 / HTTP Proxies**, visual **Port Forwarding** (`Local`, `Remote`, `Dynamic`), **Command Snippets**, and **Encrypted Backup Export/Import**.

### 🎨 Designed to Fit Your Workflow
- **Themes & Colors:** Choose from **5 App Themes** (Dark & Light), **6 Accent Colors**, and **8 Terminal Color Schemes** (`Monokai`, `Tokyo Night`, `Catppuccin Mocha`, `Dracula`, `One Dark`, `Nord`, `Obsidian`, `Solarized Light`).
- **Typography & Localization:** Built-in support for Nerd Fonts, customizable keyboard shortcuts, and instant switching between **English** and **Tiếng Việt**.

---

<a id="shortcuts"></a>

## ⌨️ Keyboard Shortcuts

All shortcuts can be customized in **Settings → Shortcuts**.

| Action | macOS | Windows / Linux |
| :--- | :--- | :--- |
| **Command Palette** | `⌘ K` | `Ctrl + Shift + P` |
| **New Local Tab** | `⌘ T` | `Ctrl + Shift + T` |
| **Split Vertically / Horizontally** | `⌘ D` / `⌘ ⇧ D` | `Alt + Shift + =` / `Alt + Shift + -` |
| **Close Active Pane** | `⌘ W` | `Ctrl + Shift + W` |
| **Next / Previous Pane** | `⌘ ]` / `⌘ [` | `Alt + →` / `Alt + ←` |
| **Next / Previous Tab** | `⌘ ⇧ ]` / `⌘ ⇧ [` | `Ctrl + Tab` / `Ctrl + Shift + Tab` |
| **Switch to Tab 1 – 9** | `⌘ 1` … `⌘ 9` | `Ctrl + Alt + 1` … `9` |
| **Toggle Server Monitor** | `⌘ I` | `Ctrl + Shift + I` |
| **Find in Terminal** | `⌘ F` | `Ctrl + Shift + F` |
| **Open Settings** | `⌘ ,` | `Ctrl + ,` |

---

<a id="build"></a>

## 🛠️ Build from Source

Requires **Node.js 20+** and **Rust (stable)**.

```bash
git clone https://github.com/nvdnvd00/ssh-box.git
cd ssh-box
npm install

# Run in development mode
npx tauri dev

# Build desktop installer
npx tauri build
```

---

<div align="center">
  <sub>Crafted with ❤️ · <a href="https://github.com/nvdnvd00/ssh-box">github.com/nvdnvd00/ssh-box</a></sub>
</div>
