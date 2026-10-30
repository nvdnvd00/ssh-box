# Thiết kế Kỹ thuật: Cấu hình VPS (VPS Server Center) — Bản sửa đổi 2

**Ngày:** 2026-10-30 (sửa đổi sau review MR !50)
**Dự án:** `ssh-box` (Tauri v2 + React 19 + TypeScript + Tailwind CSS v4 + Rust `russh`)
**Thay thế:** bản "VPS Server Center & 6 Advanced Configuration Studios" cùng ngày.

---

## 0. Vì sao viết lại

Review MR !50 cho thấy phần lớn lỗi bắt nguồn từ chính bản thiết kế cũ, không phải do code làm sai:

| Bản cũ yêu cầu | Hệ quả | Bản này đổi thành |
|---|---|---|
| Mỗi studio một bố cục riêng ("bespoke"), 6 subagent độc lập | 6 phong cách khác nhau, boilerplate lặp 6 lần | Một khuôn giao diện chung (§3), làm phần dùng chung trước (§7) |
| `<input type="checkbox" switch>` | Chỉ WebKit vẽ thành công tắc; Windows/Android thấy checkbox | Dùng toggle `role="switch"` sẵn có của app (§3.3) |
| Nút "Confirm & Keep" huỷ hẹn giờ rollback | Phiên SSH cũ vẫn sống sau reload, nên xác nhận luôn "thành công" | App tự mở **kết nối mới** để xác minh (§4.2) |
| Cảnh báo lockout chỉ khi tắt password và không có key | Bỏ sót root, `AllowUsers`, cách app đang đăng nhập | Bộ chặn tự khoá ở backend theo bảng (§4.3) |
| Đọc/ghi mỗi `/etc/ssh/sshd_config` | Bỏ qua `sshd_config.d/`, điểm bảo mật sai | Đọc bằng `sshd -T`, ghi một file drop-in (§4.2) |
| 6 studio trong một đợt | 15.8k dòng, nhiều thao tác dễ làm hỏng máy | Đợt 1 chỉ 3 studio (§1) |
| Test: parser + `npm run build` | Lỗi quoting mật khẩu sudo, luồng đổi port không được kiểm | Test bộ chạy lệnh, bộ chặn, luồng kết nối mới, ma trận nền tảng (§6) |

---

## 1. Phạm vi

### 1.1. Đợt 1 (MR này)
1. **Khung chung:** chọn máy, nhận diện quyền, bộ chạy lệnh đặc quyền, bộ chặn tự khoá, thành phần UI dùng chung.
2. **SSH:** xem và đổi cấu hình `sshd` (port, cách đăng nhập, root login, `AllowUsers`, giới hạn phiên).
3. **Firewall:** xem cổng đang mở và rule; bật/tắt; thêm/xoá rule; cho phép IP của tôi.
4. **Users & Keys:** xem user (`root` và `UID >= 1000`), tạo user, sudo, khoá/mở khoá, cấp/thu hồi SSH key từ kho key của `ssh-box`.

### 1.2. Hoãn sang đợt 2 (không nằm trong MR này)
Fail2ban, Kernel/sysctl, Swap, Systemd, Cron, Hostname/Timezone/DNS. Code hiện có của các studio này được chuyển sang nhánh riêng `feat/vps-server-center-phase2`, không xoá.

Điều kiện mở lại đợt 2: đợt 1 qua hết ma trận kiểm thử §6.3, và mỗi thao tác của đợt 2 có một dòng trong bảng chặn §4.3.

### 1.3. Ngoài phạm vi
- Host `local` và máy không phải Linux: probe phát hiện `uname -s` khác `Linux` thì hiện thông báo "Chỉ hỗ trợ máy chủ Linux", không chạy lệnh nào khác.
- Thao tác hàng loạt trên nhiều máy.

---

## 2. Nền tảng hỗ trợ

### 2.1. Phía app (client)
macOS (WKWebView), iOS (WKWebView), Windows (WebView2), Linux (WebKitGTK), Android (Android WebView).

- Giao diện phải giống nhau trên cả 5 nền tảng: chỉ dùng HTML/CSS mà cả WebKit lẫn Chromium đều hỗ trợ mặc định.
- Rộng tối thiểu 360px (`minWidth` trong `src-tauri/tauri.conf.json`): không có phần tử rộng cố định lớn hơn màn hình, không có popover bị cắt mép.
- Thao tác cảm ứng: không có thông tin chỉ hiện khi rê chuột (`title`, `group-hover`).

### 2.2. Phía server
| Distro | Điểm đặc biệt phải xử lý |
|---|---|
| Ubuntu 22.04 / 24.04 | 24.04 dùng `ssh.socket` (socket activation); cloud image có `sshd_config.d/50-cloud-init.conf` |
| Debian 12 / 13 | 13 dùng OpenSSH 10 (log ghi `sshd-session[pid]`); bản tối giản không có `/var/log/auth.log` |
| Rocky / Alma 9 | `firewalld`, SELinux `Enforcing` (đổi port phải `semanage`) |

Distro khác: được phép dùng nếu probe nhận ra `systemctl` và một engine firewall có hỗ trợ; nếu không thì studio tương ứng chỉ hiển thị, không cho sửa.

---

## 3. Khuôn giao diện chung

### 3.1. Nguyên tắc
1. **Một khuôn cho mọi studio**, lấy theo trang Cài đặt (`SettingsWorkspace.tsx`): tiêu đề nhóm `h2` + thẻ `rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60`, mỗi dòng là một cài đặt.
2. **Từ ngữ thường, không ẩn dụ:** không dùng "Fortress", "Radar", "Vault", "SOC", "SHIELDS ACTIVE". Tên directive (`PasswordAuthentication`…) hiện ở dòng phụ bằng font mono để người rành kỹ thuật đối chiếu.
3. **Không đồng hồ điểm, không preset "1 click".** Trạng thái bảo mật là danh sách kiểm tra (đạt / chưa đạt + lý do), mỗi mục có nút "Sửa" đưa tới đúng dòng cài đặt.
4. **Mọi thay đổi đi qua cùng một luồng:** chỉnh → thanh "N thay đổi" → "Xem trước" (cũ → mới) → "Áp dụng".
5. **Cho phép bôi đen và copy** nội dung (log, IP, cấu hình): không đặt `select-none` lên vùng nội dung.

### 3.2. Bố cục

```
┌ Thanh máy ───────────────────────────────────────────────┐
│ [Chọn máy ▾]  user@host:port · Ubuntu 24.04 · [Quyền: …] ⟳│
├ Tab ─────────────────────────────────────────────────────┤
│  SSH   Firewall   Users & Keys                           │
├──────────────────────────────────────────────────────────┤
│ [Banner: lỗi / đang xác minh / chỉ xem]                   │
│ Tình trạng            ← danh sách kiểm tra đạt/chưa đạt   │
│ Nhóm cài đặt 1        ← các dòng: nhãn · mô tả · điều khiển│
│ Nhóm cài đặt 2                                            │
├──────────────────────────────────────────────────────────┤
│ 3 thay đổi chưa áp dụng      [Bỏ]  [Xem trước & áp dụng]  │ ← dính đáy
└──────────────────────────────────────────────────────────┘
```

- Từ 640px trở xuống: dòng cài đặt xếp dọc (nhãn trên, điều khiển dưới); tab cuộn ngang; thanh hành động rộng hết màn hình; bảng chuyển thành danh sách thẻ.
- Popover mật khẩu sudo dùng modal (`ui-modal-backdrop` + `ui-modal-content`), không dùng popover neo theo nút, để không bị cắt mép trên điện thoại.

### 3.3. Thành phần dùng chung
Tạo trong `src/components/vps/shared/`, studio nào cũng chỉ ghép từ các thành phần này cùng `SelectField` và các lớp `.ui-input`, `.ui-label` có sẵn:

| Thành phần | Nguồn / mô tả |
|---|---|
| `Switch` | Trích từ toggle `role="switch"` ở `SettingsWorkspace.tsx` (cùng kích thước, màu `bg-primary` / `bg-secondary`) |
| `SettingRow` | Nhãn, mô tả, tên directive (mono), giá trị đang hiệu lực, điều khiển; tự xếp dọc dưới 640px |
| `StatusBadge` | 4 mức `ok / warn / danger / neutral`; là nơi **duy nhất** định nghĩa màu trạng thái |
| `ConfirmButton` | Xác nhận 2 bước tại chỗ, theo mẫu `confirmReset` ở `SettingsWorkspace.tsx` |
| `ChangePreviewDialog` | Modal liệt kê từng thay đổi "cũ → mới" và cảnh báo của bộ chặn |
| `useVpsResource` | Hook: tải dữ liệu, `loading`, lỗi, `refresh`, `mutate`; xử lý mã lỗi `SUDO_REQUIRED` một chỗ |

Màu trạng thái: thêm 3 token `--success`, `--warning`, `--danger` vào từng khối `:root[data-app-theme=…]` trong `src/index.css`, và khai báo `--color-success`… như các token sẵn có, để theme sáng và tối đều đọc được. Không dùng lớp màu nhạt kiểu `text-cyan-200`, `text-emerald-300`.

### 3.4. Chữ và tên gọi
- Mọi chuỗi lấy từ `getI18n(lang)` (`src/i18n.ts`); không viết `isVi ? … : …` trong JSX.
- Một tên duy nhất `t.navVpsConfig`: VI **"Cấu hình VPS"**, EN **"Server Config"**. Dùng cho sidebar, nút trên Dashboard, nút trên thanh terminal và tooltip.
- Backend trả lỗi dạng `MÃ: chi tiết` (vẫn `Result<T, String>` như các lệnh khác trong `lib.rs`); frontend dịch `MÃ` thành câu theo ngôn ngữ. Các mã: `SUDO_REQUIRED`, `SUDO_DENIED`, `LOCKOUT_BLOCKED`, `COMMAND_FAILED`, `VERIFY_FAILED`, `UNSUPPORTED`.

---

## 4. Chống khoá

### 4.1. Định nghĩa
**Ngữ cảnh kết nối** (`ConnectionContext`) là bộ thông tin app đang dùng để vào máy:
- `login_user`: user đăng nhập (`host.user` hoặc user của identity).
- `auth`: `Password` hoặc `Key { public_key_body }` — key app thực sự dùng cho host này.
- `client_ip`, `server_port`: lấy từ trường 1 và trường 4 của `$SSH_CONNECTION` trong probe **không qua sudo** (vì sudo xoá biến này).

Một thay đổi chỉ được coi là **an toàn** khi, sau khi áp dụng, app mở được một **kết nối SSH mới** bằng đúng ngữ cảnh này (hoặc ngữ cảnh mới đã khai báo, ví dụ port mới). Giữ được phiên đang mở không chứng minh được gì.

### 4.2. Luồng áp dụng thay đổi SSH

```
Kiểm tra bộ chặn → Sao lưu → Ghi drop-in → sshd -t → Hẹn giờ hoàn tác
→ Nạp lại đúng cách → Kiểm tra cổng → Kết nối thử từ app → Huỷ hẹn giờ → Lưu port
            (bất kỳ bước nào lỗi → hoàn tác ngay, không lưu port)
```

1. **Đọc cấu hình đang hiệu lực** bằng `sshd -T -C user=<login_user>,addr=<client_ip>,lport=<server_port>`, đã gồm `sshd_config.d/` và `Match`. Không tự parse file.
2. **Nơi ghi:** nếu dòng `Include /etc/ssh/sshd_config.d/*.conf` đứng trước mọi directive trong file chính, ghi toàn bộ thay đổi vào `/etc/ssh/sshd_config.d/00-sshbox.conf` (tên sắp xếp đứng đầu, nên thắng theo quy tắc giá trị đầu tiên). Nếu không có `Include`, sửa file chính và giữ nguyên comment.
3. **Sao lưu** file sẽ ghi vào `<file>.sshbox.bak`, chỉ khi không có hoàn tác nào đang chờ. Đang có thay đổi chờ xác minh thì từ chối áp dụng tiếp.
4. **Kiểm tra cú pháp** `sshd -t` trên cấu hình đầy đủ; lỗi thì khôi phục bản sao lưu và dừng. sshd đang chạy chỉ đọc lại file khi reload, nên bước ghi không ảnh hưởng kết nối mới.
5. **Hẹn giờ hoàn tác trên server, trước khi nạp lại:** `systemd-run --on-active=60 --unit=sshbox-rollback …` (không bị logind huỷ khi phiên SSH đóng). Script hoàn tác khôi phục bản sao lưu rồi chạy lệnh nạp lại ở bước 6, dùng `restart` (không chỉ `reload`) để cứu cả trường hợp sshd đã dừng. Server trả về thời điểm hết hạn; UI đếm ngược theo đó, không tự đặt 30 giây.
6. **Nạp lại đúng cách:**
   - `ssh.socket` đang active: `systemctl daemon-reload && systemctl restart ssh.socket`.
   - Ngược lại: `systemctl reload-or-restart ssh` (hoặc `sshd`).
   - SELinux `Enforcing` và port khác 22: `semanage port -a -t ssh_port_t -p tcp <port>` **trước** khi nạp lại; lỗi thì dừng ở bước này.
   - Firewall đang bật và port đổi: thêm rule cho phép port mới qua engine đang dùng (§4.4).
7. **Kiểm tra cổng trên server:** `ss -tlnH "sport = :<port>"` phải có tiến trình `sshd`.
8. **Kết nối thử từ app:** gọi `SshPool::test_connection` (`src-tauri/src/ssh_pool.rs`) với bản sao `HostEntry` mang port mới. Kết nối này là kết nối mới hoàn toàn. Fingerprint host key trả về phải trùng fingerprint đã biết của port cũ; khác thì coi là thất bại.
9. **Thành công:** `systemctl stop sshbox-rollback.timer`, rồi mới lưu `HostEntry.port`. **Thất bại hoặc hết giờ:** hoàn tác ngay, không đụng `HostEntry`, hiện bước thất bại và lý do.

UI: modal tiến trình hiện các bước trên với trạng thái từng bước, cùng nút "Hoàn tác ngay". Không có nút "Xác nhận" thủ công, không có nút "Áp dụng không hẹn giờ hoàn tác".

### 4.3. Bộ chặn tự khoá
Chạy ở backend (Rust, hàm thuần, có unit test) trước mọi thao tác ghi. Frontend hiển thị kết quả trong `ChangePreviewDialog`, không tự quyết.

| Thao tác | Điều kiện | Kết quả |
|---|---|---|
| SSH: `PasswordAuthentication no` | `auth = Password` | Chặn |
| SSH: `PubkeyAuthentication no` | `auth = Key` | Chặn |
| SSH: tắt cả password lẫn pubkey | luôn | Chặn |
| SSH: `PermitRootLogin no` | `login_user = root` | Chặn |
| SSH: `PermitRootLogin prohibit-password` | `login_user = root` và `auth = Password` | Chặn |
| SSH: `AllowUsers` / `DenyUsers` | danh sách mới loại trừ `login_user` | Chặn |
| Firewall: bật với incoming `deny` | chưa có rule cho phép `server_port` | Tự thêm rule cho phép `server_port` trước khi bật |
| Firewall: xoá rule | rule này là rule duy nhất cho phép `server_port` từ `client_ip` | Chặn |
| Firewall: thêm `deny` / `reject` | nguồn bao gồm `client_ip`, hoặc cổng là `server_port` từ mọi nơi | Chặn |
| Users: khoá, bỏ sudo | user là `login_user` | Chặn |
| Users: bỏ sudo | user là người cuối cùng có sudo (trừ root) | Chặn |
| Keys: thu hồi | key là key app đang dùng cho `login_user` | Chặn |
| Mọi thao tác xoá/khoá/thu hồi khác | — | Hỏi lại bằng `ConfirmButton` |

Bị chặn thì trả lỗi `LOCKOUT_BLOCKED: <lý do>`, UI giải thích bằng một câu, ví dụ: "Ứng dụng đang đăng nhập bằng mật khẩu. Hãy chuyển máy này sang đăng nhập bằng SSH key trước khi tắt mật khẩu."

### 4.4. Firewall
- **Engine được phép ghi:** `ufw` và `firewalld`. iptables thuần chỉ được xem, kèm hướng dẫn cài `ufw`, vì không có cách lưu bền qua reboot thống nhất giữa các distro.
- **firewalld:** mọi thay đổi dùng `--permanent` rồi `--reload`.
- **Thứ tự rule:** `deny` / `reject` chèn lên đầu (`ufw insert 1`; firewalld rich rule có `priority` âm). `allow` nối cuối.
- **Xoá theo nội dung, không theo số thứ tự:** script đọc lại `ufw status numbered`, tìm dòng có nội dung trùng rule cần xoá, lấy số hiện tại rồi xoá trong cùng script. Không tìm thấy thì báo lỗi, không xoá gì.
- **IPv6:** ufw kiểm tra `IPV6=yes` trong `/etc/default/ufw`; tắt thì hiện cảnh báo "IPv6 không được lọc".
- **Trạng thái cổng** (đang mở công khai / giới hạn IP / bị chặn / chỉ nội bộ) tính theo thứ tự rule, rule khớp đầu tiên thắng; tách IPv4 và IPv6.

### 4.5. Users & Keys
- **Tạo user, quyền sudo** có 3 lựa chọn: không sudo / sudo không cần mật khẩu (`/etc/sudoers.d/90-sshbox-<user>`, kiểm bằng `visudo -cf`) / sudo có mật khẩu. Lựa chọn thứ ba bắt buộc nhập mật khẩu; mật khẩu truyền vào `chpasswd` qua stdin.
- **Khoá user:** `usermod -L` kèm `usermod -e 1`, để chặn cả đăng nhập bằng key (với `UsePAM yes`, chỉ `-L` không chặn key). Mở khoá: `usermod -U -e ''`.
- **Thao tác trong thư mục của user** (`~/.ssh`, `authorized_keys`) chạy dưới quyền chính user đó (`runuser -u <user> -- sh -c …`, `umask 077`), không ghi bằng root, để tránh bị lừa bằng symlink.
- **Thu hồi key:** so khớp chính xác trường key (trường thứ 2 của dòng), không tìm chuỗi con.

---

## 5. Lớp thực thi đặc quyền (backend)

- Giữ cách chạy script qua heredoc `sh -s` của `build_privileged_command` (`src-tauri/src/vps_config.rs`). Bỏ dòng `:` cuối; mọi script ghi bắt đầu bằng `set -eu`, để lệnh nào lỗi thì trả `COMMAND_FAILED`.
- **Mật khẩu sudo** chỉ đi qua stdin của `sudo -S`, không bao giờ nằm trong chuỗi `sh -c` hay trong dòng lệnh. Sửa luôn `probe_privilege` theo cách này.
- **Không tự lấy mật khẩu đăng nhập trong vault để thử sudo** (bỏ nhánh này trong `resolve_sudo_password`): mỗi lần đoán sai bị sudo tính là 3 lần thử sai, có thể khoá tài khoản trên máy dùng `pam_faillock`. Người dùng nhập mật khẩu sudo một lần mỗi phiên làm việc, chỉ giữ trong bộ nhớ.
- **Probe** chạy không qua sudo và trả về:
  - quyền: `root | sudoNoPassword | sudoPassword | none`;
  - `ConnectionContext` (§4.1);
  - `os_id`, `os_version`, `openssh_version`;
  - `ssh_socket_active`, `selinux_enforcing`, `firewall_engine`.
- **File tạm** dùng `mktemp`, không dùng đường dẫn đoán trước được trong `/tmp`.
- Mỗi thao tác ghi trả về snapshot mới trong cùng một lần chạy script, không gọi thêm lệnh đọc riêng.

---

## 6. Kiểm thử

### 6.1. Unit test Rust (`cargo test`)
- Giữ các test parser đã có (ufw, `ss`, `authorized_keys`, `passwd`/sudoers).
- **Bộ chạy lệnh:** dựng lệnh bằng `build_privileged_command` và probe rồi chạy thật bằng `sh` cục bộ, với `sudo` được thay bằng một script giả (đặt trước trong `PATH`) chỉ ghi lại stdin. Mật khẩu thử: dấu cách, `'`, `"`, `$HOME`, `;`, `` ` ``, `\`. Phải chứng minh mật khẩu tới nguyên vẹn và không có lệnh nào khác bị chạy.
- **Lỗi giữa script:** một lệnh lỗi ở giữa thì `run_privileged_script` phải trả `Err(COMMAND_FAILED…)`.
- **Bộ chặn:** mỗi dòng của bảng §4.3 có ít nhất một test chặn và một test cho phép.
- **Parser `sshd -T`:** gồm trường hợp drop-in đặt `PasswordAuthentication yes` thắng file chính.
- **Xoá rule ufw theo nội dung:** khi số thứ tự đã bị lệch thì phải xoá đúng rule.
- Mỗi test có một câu nói rõ nó bảo vệ điều gì. Mỗi test phải được chạy một lần trên code chưa sửa để thấy nó bắt được lỗi.

### 6.2. Test tích hợp với sshd tạm
Dùng cách dự án đã làm: sshd tạm trên cổng cao, chạy bằng user hiện tại, `HOME` giả, không đụng `~/.ssh` thật và không dùng server thật.
- Luồng xác minh §4.2 bước 8: đổi port của sshd tạm → `test_connection` trên port mới thành công; port sai thất bại; host key khác thì thất bại.
- Kết nối cũ vẫn sống sau reload nhưng **không** được dùng làm bằng chứng: test phải thất bại nếu code xác minh bằng phiên cũ.

### 6.3. Ma trận kiểm thử thủ công
**Server**, trên máy ảo dùng một lần (không dùng server thật): Ubuntu 24.04, Debian 13, Rocky 9.

| Kịch bản | Kết quả mong đợi |
|---|---|
| Đổi port SSH | Hoạt động trên cả 3 distro (`ssh.socket`, SELinux); `HostEntry.port` chỉ đổi khi kết nối thử thành công |
| Đổi port bằng một port chưa mở trên firewall của nhà cung cấp | Kết nối thử thất bại → tự hoàn tác, port cũ vẫn vào được |
| Tắt password khi app đăng nhập bằng password | Bị chặn, có hướng dẫn |
| Cloud image có `50-cloud-init.conf` bật password | Tình trạng hiển thị đúng "đang bật" và tắt được thật |
| Bật firewall rồi reboot | Rule còn, SSH vẫn vào được |
| Xoá rule SSH duy nhất / thu hồi key đang dùng / khoá chính mình | Bị chặn |

**App:** macOS, Windows, iOS hoặc Android (thiết bị thật hoặc trình giả lập) × theme tối / sáng × tiếng Việt / tiếng Anh.
- Công tắc trông giống nhau trên mọi nền tảng.
- Ở 360px không có thanh cuộn ngang toàn trang và không có phần tử bị cắt.
- Theme sáng đọc được mọi trạng thái.
- Chế độ tiếng Việt không còn chữ tiếng Anh, trừ tên directive và lệnh.

### 6.4. Tiêu chí hoàn thành
- `cargo test` và `npm run build` đều qua; các test ở 6.1 và 6.2 đã được chứng minh là bắt được lỗi cũ.
- Đã đi hết ma trận 6.3 và ghi kết quả (đạt/không đạt) vào mô tả MR.

---

## 7. Thứ tự thực hiện

Làm tuần tự trong một luồng, không chia cho nhiều agent độc lập. Mỗi bước tối đa khoảng 400 dòng code; bước nào lớn hơn thì tách nhỏ tiếp.

1. **Backend chung:** sửa bộ chạy lệnh và probe, thêm `ConnectionContext`, mã lỗi, bộ chặn, kèm test §6.1.
2. **UI chung:** `src/components/vps/shared/`, 3 token màu, chuỗi i18n, khung `VpsConfigWorkspace` theo §3.
3. **SSH:** luồng §4.2, test tích hợp §6.2.
4. **Firewall:** §4.4.
5. **Users & Keys:** §4.5.
6. **Kiểm thử thủ công:** ma trận §6.3.

Sau khi spec này được duyệt, viết lại `docs/superpowers/plans/2026-10-30-vps-server-center-plan.md` theo đúng các bước trên.

### Xử lý code đang có trong MR !50
- **Giữ:** các parser đã có test, `build_privileged_command` (sau khi sửa theo §5), các điểm vào từ sidebar, Dashboard và thanh terminal (đổi nhãn sang `t.navVpsConfig`).
- **Viết lại theo khuôn §3:** `VpsConfigWorkspace.tsx`, `SshHardeningStudio.tsx`, `FirewallStudio.tsx`, `AccessKeysStudio.tsx`.
- **Chuyển sang nhánh `feat/vps-server-center-phase2`:** `Fail2banStudio.tsx`, `KernelTuningStudio.tsx`, `SystemOpsStudio.tsx` cùng các lệnh backend và kiểu dữ liệu tương ứng.
