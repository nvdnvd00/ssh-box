<div align="center">

<picture>
  <source media="(prefers-color-scheme: dark)" srcset="docs/brand/ssh-box-logo-dark.svg" />
  <source media="(prefers-color-scheme: light)" srcset="docs/brand/ssh-box-logo-light.svg" />
  <img alt="SSH Box" src="docs/brand/ssh-box-logo-dark.svg" width="360" />
</picture>

<br />

### Ứng dụng Quản lý SSH, Terminal 120fps & Máy chủ Hiện đại
*Nhanh, mượt mà và gọn nhẹ cho lập trình viên & quản trị viên trên macOS, Windows và Linux.*

[![Latest Release](https://img.shields.io/github/v/release/nvdnvd00/ssh-box?style=flat-square&logo=github&label=release&color=2563eb)](https://github.com/nvdnvd00/ssh-box/releases/latest)
[![Platform](https://img.shields.io/badge/platform-macOS%20%7C%20Windows%20%7C%20Linux-10b981?style=flat-square)](#download)
[![License](https://img.shields.io/badge/license-Apache--2.0-f59e0b?style=flat-square)](LICENSE)

<p align="center">
  <a href="README.md"><strong>🌐 English</strong></a>
  &nbsp;·&nbsp;
  <strong>🇻🇳 Tiếng Việt</strong>
</p>

<p align="center">
  <a href="#download"><strong>⬇️ Tải xuống</strong></a>
  &nbsp;·&nbsp;
  <a href="#features"><strong>✨ Tính năng</strong></a>
  &nbsp;·&nbsp;
  <a href="#shortcuts"><strong>⌨️ Phím tắt</strong></a>
  &nbsp;·&nbsp;
  <a href="#build"><strong>🛠️ Tự build</strong></a>
</p>

</div>

---

## ⚡ Vì sao chọn SSH Box?

**SSH Box** kết hợp trải nghiệm **Terminal 120fps siêu mượt** với **không gian quản trị máy chủ toàn diện**. Thay vì phải mở rời rạc nhiều phần mềm cho Terminal, truyền file SFTP, theo dõi tài nguyên và cấu hình bảo mật VPS, mọi công cụ bạn cần đều nằm gọn trong một ứng dụng duy nhất.

---

<a id="download"></a>

## ⬇️ Tải xuống

Tải bản cài đặt mới nhất tại [**GitHub Releases**](https://github.com/nvdnvd00/ssh-box/releases/latest):

| Hệ điều hành | Định dạng | Liên kết tải |
| :--- | :--- | :--- |
| **macOS** (`Apple Silicon` & `Intel`) | Universal `.dmg` | [**Tải `ssh-box.dmg`**](https://github.com/nvdnvd00/ssh-box/releases/latest/download/ssh-box.dmg) |
| **Windows** (`64-bit`) | Bộ cài `.exe` | [**Tải `ssh-box-setup.exe`**](https://github.com/nvdnvd00/ssh-box/releases/latest/download/ssh-box-setup.exe) |

> [!TIP]
> - **Mở lần đầu trên macOS:** Nếu macOS Gatekeeper chặn khi mở lần đầu, hãy chạy lệnh sau trong Terminal:
>   ```bash
>   xattr -cr "/Applications/SSH Box.app"
>   ```
> - **Tự động cập nhật:** Kiểm tra và nâng cấp phiên bản mới trực tiếp ngay trong **Cài đặt → Cập nhật**.

---

<a id="features"></a>

## ✨ Tính năng nổi bật

### 🖥️ Terminal 120fps & Chia đôi màn hình (Split Panes)
- **Hiển thị mượt mà:** Tăng tốc bằng GPU giúp terminal luôn giữ tốc độ phản hồi tức thì ngay cả khi xả log lớn hoặc chạy `btop`, `tmux`, `nvim`.
- **Chia khung linh hoạt:** Chia dọc hoặc ngang màn hình thành nhiều khung làm việc song song cho cả SSH và shell cục bộ.
- **Làm việc không gián đoạn:** Kết nối lại chỉ với 1 phím khi rớt mạng mà không mất log trên màn hình, hỗ trợ tự động gắn phiên `tmux` và **Bảng lệnh nhanh** (`⌘K` / `Ctrl+Shift+P`).

### 📊 Giám sát máy chủ thời gian thực & Quản lý Docker
- **Theo dõi sức khoẻ máy chủ:** Quan sát trực quan mức tải CPU từng nhân, RAM & Swap, dung lượng ổ đĩa, biểu đồ tốc độ mạng RX/TX, nhiệt độ CPU và các tiến trình tiêu thụ nhiều tài nguyên nhất.
- **Quản lý Docker trực quan:** Xem danh sách container và image, thực hiện bật, tắt, khởi động lại hoặc xoá chỉ với 1 cú nhấp chuột.

### 🛡️ Cấu hình bảo mật VPS & Cài đặt công cụ 1 chạm
- **Quản trị máy chủ không cần nhớ lệnh:** Cấu hình trực quan **SSH**, **Tường lửa (Firewall)** và **Người dùng & Khoá truy cập** trên cả máy chủ Linux, macOS và Windows.
- **Bảo vệ chống tự khoá máy (Anti-Lockout):** Xem trước mọi thay đổi (`cũ → mới`) trước khi áp dụng. SSH Box tự động kiểm tra kết nối mới và hoàn tác (rollback) nếu cấu hình mới làm gián đoạn truy cập của bạn.
- **Trình cài đặt công cụ (Tool Installer):** Kiểm tra những công cụ bảo mật và lập trình còn thiếu (`UFW`, `Fail2ban`, `Unattended Upgrades`, `Docker`, `Git`, `Node.js`, `Nginx`, `Tailscale`, `Claude Code`, `tmux`, `ripgrep`, `htop`, v.v.) và cài đặt hàng loạt chỉ với 1 chạm cho cả VPS lẫn máy cá nhân.

### 📂 Trình quản lý file SFTP 2 cột
- **Duyệt file song song Local ↔ Remote:** Kéo thả, tải lên và tải xuống file/thư mục nhanh chóng giữa máy tính của bạn và máy chủ từ xa.
- **Sửa file & Phân quyền trực tiếp:** Mở và chỉnh sửa nhanh file văn bản từ xa ngay trong ứng dụng, tạo/đổi tên/xoá thư mục và đổi quyền `chmod` trực quan.

### 🔐 Bảo mật Keychain & Bộ công cụ mạng chuyên sâu
- **Tự động đồng bộ `~/.ssh/config`:** Nhận diện ngay danh sách host sẵn có và sắp xếp khoa học theo nhóm hoặc thẻ màu.
- **An toàn tuyệt đối với Keychain hệ điều hành:** Mật khẩu và passphrase được bảo vệ trực tiếp trong **macOS Keychain**, **Windows Credential Manager**, hoặc **Linux Secret Service**.
- **Đầy đủ đồ nghề mạng:** Tích hợp **Tạo & Quản lý SSH Key**, **Định danh (Identities)** dùng chung, **Jump Host (Bastion)**, **Proxy SOCKS5 / HTTP**, **Port Forwarding** (`Local`, `Remote`, `Dynamic`), kho **Snippets** lệnh mẫu và **Sao lưu/Khôi phục dữ liệu mã hoá**.

### 🎨 Cá nhân hoá theo phong cách của bạn
- **Giao diện & Bảng màu:** **5 chủ đề ứng dụng** (Sáng & Tối), **6 màu nhấn** và **8 bảng màu Terminal** kinh điển (`Monokai`, `Tokyo Night`, `Catppuccin Mocha`, `Dracula`, `One Dark`, `Nord`, `Obsidian`, `Solarized Light`).
- **Phông chữ & Ngôn ngữ:** Hỗ trợ sẵn Nerd Font, tuỳ biến mọi phím tắt và chuyển đổi tức thì giữa **Tiếng Việt** và **English**.

---

<a id="shortcuts"></a>

## ⌨️ Phím tắt thông dụng

Bạn có thể tuỳ chỉnh lại toàn bộ phím tắt trong **Cài đặt → Phím tắt**.

| Thao tác | macOS | Windows / Linux |
| :--- | :--- | :--- |
| **Bảng lệnh nhanh (Command Palette)** | `⌘ K` | `Ctrl + Shift + P` |
| **Mở tab Local mới** | `⌘ T` | `Ctrl + Shift + T` |
| **Chia dọc / Chia ngang khung** | `⌘ D` / `⌘ ⇧ D` | `Alt + Shift + =` / `Alt + Shift + -` |
| **Đóng khung hiện tại** | `⌘ W` | `Ctrl + Shift + W` |
| **Chuyển khung kế / trước** | `⌘ ]` / `⌘ [` | `Alt + →` / `Alt + ←` |
| **Chuyển tab kế / trước** | `⌘ ⇧ ]` / `⌘ ⇧ [` | `Ctrl + Tab` / `Ctrl + Shift + Tab` |
| **Chuyển nhanh tới Tab 1 – 9** | `⌘ 1` … `⌘ 9` | `Ctrl + Alt + 1` … `9` |
| **Bật/tắt bảng giám sát máy chủ** | `⌘ I` | `Ctrl + Shift + I` |
| **Tìm kiếm trong Terminal** | `⌘ F` | `Ctrl + Shift + F` |
| **Mở Cài đặt** | `⌘ ,` | `Ctrl + ,` |

---

<a id="build"></a>

## 🛠️ Tự build từ mã nguồn

Yêu cầu cài sẵn **Node.js 20+** và **Rust (stable)**.

```bash
git clone https://github.com/nvdnvd00/ssh-box.git
cd ssh-box
npm install

# Chạy môi trường phát triển
npx tauri dev

# Đóng gói bộ cài đặt ứng dụng
npx tauri build
```

---

## 📄 Giấy phép (License)

Phát hành theo giấy phép [**Apache License 2.0**](LICENSE). Bản quyền © 2026 **Neos Nguyen** ([`@nvdnvd00`](https://github.com/nvdnvd00)).

---

<div align="center">
  <sub>Được phát triển với ❤️ bởi Neos Nguyen · <a href="https://github.com/nvdnvd00/ssh-box">github.com/nvdnvd00/ssh-box</a></sub>
</div>
