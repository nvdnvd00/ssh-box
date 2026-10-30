# Thiết kế: Mở rộng Cấu hình VPS sang Windows và macOS

**Ngày:** 2026-10-30
**Dự án:** `ssh-box` (Tauri v2 + React 19 + TypeScript + Tailwind CSS v4 + Rust `russh`)
**Dựa trên:** `docs/superpowers/specs/2026-10-30-vps-server-center-design.md` (sau đây gọi là **spec gốc**). Mọi quy định của spec gốc vẫn áp dụng, trừ chỗ ghi khác ở đây.

---

## 0. Mục đích

Hiện nay Cấu hình VPS chỉ làm việc với máy chủ Linux. Gặp máy khác, app chỉ hiện "Chỉ hỗ trợ máy chủ Linux".

- **Vì sao:** để phục vụ những người dùng SSH Box có máy Windows hoặc macOS, không xuất phát từ một máy cụ thể của người đặt yêu cầu.
- **Thành công là:** người dùng chọn một máy Windows hoặc macOS trong Cấu hình VPS thì thấy đúng những gì làm được trên máy đó, và mọi thao tác hiện ra đều an toàn như trên Linux.
- **Vì sao làm trước tính năng cài tool:** phần nền móng ở đây (nhận ra máy bên kia chạy hệ gì, gửi lệnh đúng kiểu shell của hệ đó) chính là nền của dự án tiếp theo, tính năng cài tool/app trên máy. Thứ tự này do người dùng chọn. Tính năng cài tool có spec riêng, viết sau.

---

## 1. Các quyết định của người dùng

| Quyết định | Nội dung |
|---|---|
| Hệ máy | Thêm Windows và macOS, là các máy mà app SSH vào. Linux giữ nguyên. |
| Nguyên tắc | Tính năng nào phức tạp, khó làm thì **ẩn đi**, không bắt buộc hỗ trợ. |
| Máy local | Không có tab cấu hình nào. Dự án cài tool sau này mới thêm tab cài tool cho máy local. |
| Kiểm thử | Người dùng không có máy Windows hay Mac thứ hai. Dùng máy ảo trong **UTM** trên chiếc Mac này: Windows 11 bản ARM và macOS. Trước khi tải phải hỏi lại người dùng, kèm tên file, nguồn và dung lượng. |
| Phiên bản chưa thử | Cho sửa **theo khả năng của máy**, không giới hạn ở các bản đã thử (§3.1). |
| Cho dự án sau | Tính năng cài tool **không** làm "bộ tool đã lưu". |

---

## 2. Phạm vi

### 2.1. Trong phạm vi
1. **Nền móng:** nhận diện hệ máy, bộ chạy lệnh cho Windows, bảng mức hỗ trợ do backend tính.
2. **macOS:** tab SSH (trừ đổi port), tab Users & Keys ở mức chỉ xem.
3. **Windows:** tab SSH, Firewall, Users & Keys theo mức ở §4.

### 2.2. Ngoài phạm vi
- **Máy local.** Danh sách `hosts` đưa vào `VpsConfigWorkspace` đến từ `host_store`, vốn không có máy `local`. Vì vậy hiện tại máy local đã không xuất hiện trong ô chọn máy, không cần sửa gì.
- iPhone và Android làm máy đích. App chạy trên iPhone/Android vẫn phải cấu hình được máy Windows/macOS, theo đúng yêu cầu giao diện ở spec gốc §2.1.
- Máy Windows không bật OpenSSH Server.
- Kiểm thử trên Windows Server: không có máy ảo ARM để thử. Windows Server vẫn được hỗ trợ theo cùng quy tắc "theo khả năng của máy" (§3.1), vì OpenSSH trên đó là cùng một thành phần với Windows 10/11.
- Thao tác trên nhiều máy cùng lúc.

### 2.3. Những điều được coi là thất bại, dù code vẫn chạy
- Có một thao tác có thể khoá người dùng ra khỏi máy được mở ra trên một hệ mà cơ chế tự hoàn tác chưa được thử thật trên hệ đó.
- Có một nút hiện ra nhưng bấm vào thì lỗi, hoặc không có tác dụng gì, trên một hệ nào đó.
- Hành vi trên máy Linux thay đổi dù chỉ một chút: ma trận Linux trong spec gốc §6.3 phải qua lại y như cũ.
- Giao diện Windows/macOS lệch khỏi khuôn chung ở spec gốc §3.

---

## 3. Ba mức hỗ trợ

Mỗi tab, và khi cần thì từng dòng cài đặt, trên mỗi hệ máy có một trong ba mức:

| Mức | Hiển thị |
|---|---|
| **Đầy đủ** | Xem và sửa, như trên Linux |
| **Chỉ xem** | Hiện dữ liệu, mọi điều khiển bị khoá, có một `Notice` nói lý do (theo mẫu `no_systemd` / `firewall_read_only` sẵn có) |
| **Ẩn** | Tab bị bỏ khỏi thanh tab. Dòng cài đặt bị bỏ khỏi nhóm, không kèm ghi chú |

**Quy tắc cứng:**
- **Thay đổi SSH** (loại cần tự hoàn tác, theo spec gốc §4.2) chỉ được ở mức Đầy đủ trên một hệ khi cơ chế tự hoàn tác của hệ đó đã qua ma trận ở §7.3. Chưa qua thì tối đa là Chỉ xem.
- **Thay đổi firewall** được bảo vệ giống hệt Linux hiện nay: bằng bộ chặn tự khoá (spec gốc §4.3) cộng với việc tự thêm rule cho phép `server_port` khi bật firewall, không có hẹn giờ hoàn tác. Vì vậy Windows Firewall không phụ thuộc kết quả thử nghiệm §8.1, nhưng vẫn phải qua các dòng firewall trong ma trận §7.3. *(quyết định của Claude, theo đúng cách Linux đang làm)*

**Backend quyết định mức** *(quyết định của Claude)*. Probe trả kèm bảng mức hỗ trợ. Bảng này được tính bằng một hàm Rust thuần (có unit test) từ kết quả probe: hệ máy, quyền, có systemd không, engine firewall. Frontend chỉ hiển thị theo bảng, không tự suy luận theo hệ máy, giống cách bộ chặn ở spec gốc §4.3 quyết ở backend.

### 3.1. Phiên bản chưa thử: theo khả năng của máy (quyết định của người dùng)
Mức hỗ trợ **không** dựa trên danh sách các bản đã thử trên máy ảo. Probe kiểm tra những thứ mà từng luồng ghi cần. Thiếu thứ nào thì ô tương ứng về mức Chỉ xem, kèm lý do. Tối thiểu cần kiểm tra:
- **macOS, SSH ghi:** có dòng `Include /etc/ssh/sshd_config.d/*` đứng trước mọi directive (macOS 13 trở lên). Không có thì tab SSH ở mức Chỉ xem, không dùng nhánh sửa file chính trên macOS *(quyết định của Claude: bớt một luồng phải thử; macOS 12 trở xuống đã hết được Apple hỗ trợ)*. Cần có thêm các lệnh mà cơ chế hẹn giờ hoàn tác dùng tới.
- **Windows, SSH ghi:** có `sshd.exe` và dịch vụ `sshd` đang chạy; OpenSSH đủ mới để hiểu mọi directive mà luồng ghi dùng (người làm tự xác định phiên bản tối thiểu, dưới mức đó thì Chỉ xem); có module `ScheduledTasks`.
- **Windows, Firewall ghi:** có module `NetSecurity`.

Lý do chấp nhận được: mọi thay đổi SSH luôn qua bước kết nối thử mới, thất bại thì tự hoàn tác, nên một bản chưa thử có hành vi lạ vẫn quay về cấu hình cũ. Mô tả MR phải ghi trung thực những bản nào đã được thử thật.

### 3.2. Linux giữ nguyên
Với Linux, bảng phải cho ra **đúng hành vi hiện tại**:
- không có systemd thì tab SSH ở mức Chỉ xem;
- engine `iptables` thì tab Firewall ở mức Chỉ xem;
- quyền `none` vẫn hiện nút mở khoá sudo như hiện nay, không chuyển sang Chỉ xem.

---

## 4. Bảng hỗ trợ đích

| Tab / thao tác | Linux | macOS | Windows |
|---|---|---|---|
| SSH: xem | Đầy đủ (đã có) | Đầy đủ | Đầy đủ |
| SSH: cách đăng nhập (password, pubkey), giới hạn phiên, thời gian chờ | Đầy đủ | Đầy đủ | Đầy đủ nếu thử nghiệm §8 đạt, không thì Chỉ xem |
| SSH: `PermitRootLogin` | Đầy đủ | Đầy đủ | **Ẩn** (Windows không có root) |
| SSH: `AllowUsers` / `DenyUsers` | Đầy đủ | Đầy đủ | **Ẩn** (tên có domain, phân biệt hoa thường khác Linux) *(quyết định của Claude)* |
| SSH: đổi port | Đầy đủ | **Ẩn** (§6.1) | Cùng mức với dòng thứ hai |
| Firewall | Đầy đủ | **Ẩn** (tường lửa macOS chặn theo ứng dụng, không theo cổng) | Đầy đủ, với giới hạn ở §6.2 |
| Users & Keys | Đầy đủ | Chỉ xem | Chỉ xem |

Windows mà tài khoản đăng nhập không có quyền quản trị: mọi tab ở mức Chỉ xem, kèm lý do "Tài khoản này không có quyền quản trị".
- Tab chỉ hiện những dữ liệu đọc được mà không cần quyền quản trị. Phần cần quyền quản trị (ví dụ `sshd -T`, file key của admin) được thay bằng chính câu thông báo đó.
- Không tự đọc `sshd_config` để bù vào, vì spec gốc cấm tự parse file này.
- Cách hiển thị giống Linux khi quyền là `none` hiện nay, chỉ khác là không có nút mở khoá sudo. *(quyết định của Claude)*

---

## 5. Nền móng

### 5.1. Nhận diện hệ máy
- **Lệnh đầu tiên gửi sang máy phải vô hại** trên cả `sh`, `cmd.exe` và PowerShell. Lý do: SSH vào Windows thì shell mặc định là `cmd` hoặc PowerShell, nên `PROBE_SCRIPT` hiện tại (viết cho `sh`) không được phép chạy thẳng trên đó. Gợi ý: gửi riêng `uname -s` trước. Trên `sh` lệnh trả `Linux` / `Darwin`; trên `cmd` và PowerShell lệnh chỉ báo "không có lệnh này".
- `Linux` thì chạy probe Linux như cũ. `Darwin` thì chạy probe Unix có nhánh cho macOS: `OS_ID=macos`, phiên bản lấy từ `sw_vers -productVersion`. Không trả lời được thì chạy probe Windows qua bộ chạy lệnh §5.2. Kết quả khác (ví dụ `MINGW64_NT…`, `CYGWIN_NT…`) thì báo `UNSUPPORTED`.
- `VpsProbe` có thêm `os_family: linux | macos | windows` và bảng mức hỗ trợ (§3). `parse_probe_output` thôi từ chối mọi máy không phải Linux.
- Nhánh Linux giữ nguyên các trường `ConnectionContext`. macOS và Windows cũng phải điền đủ các trường này (xem §8 về `SSH_CONNECTION` trên Windows).

### 5.2. Bộ chạy lệnh cho Windows
- Mỗi lệnh gửi đi có dạng `powershell.exe -NoProfile -NonInteractive -EncodedCommand <base64 của script dạng UTF-16LE>`. Dạng này chạy được dù shell mặc định là `cmd` hay PowerShell, và không phải lo dấu nháy. PowerShell 5.1 có sẵn trên Windows 10 và 11.
- Bí mật (nội dung file, mật khẩu nếu sau này cần) đi qua stdin, đọc bằng `[Console]::In`, không bao giờ nằm trong dòng lệnh. Đây là quy tắc ở spec gốc §5.
- Script ghi bắt đầu bằng `$ErrorActionPreference = 'Stop'`, tương đương `set -eu`. Lỗi được trả về bằng cùng các dấu hiệu và mã lỗi như bên `sh` (`SSHBOX_EXIT_CODE`, `COMMAND_FAILED`…), để `classify_privileged_output` và phía frontend dùng chung.
- **Quyền:** Windows không có sudo. Probe báo `admin` nếu phiên SSH có quyền quản trị thật (token đã nâng quyền), báo `none` nếu không. Thêm giá trị `admin` vào `PrivilegeLevel`, nhãn VI "Quản trị viên", EN "Administrator". Trên Windows không hiện nút và hộp nhập mật khẩu sudo.

### 5.3. Giao diện
- `VpsConfigWorkspace` dựng thanh tab từ bảng mức hỗ trợ. Tab ẩn bị bỏ khỏi thanh. Nếu tab đang mở bị ẩn khi đổi máy thì chuyển sang tab đầu tiên còn hiện.
- Mỗi studio nhận mức của mình: ở mức Chỉ xem thì `fieldset disabled` cùng `Notice`, giống cách `SshHardeningStudio` đang làm với `no_systemd`.
- Thanh máy hiện tên hệ, ví dụ `macOS 15.1` hoặc `Windows 11 Pro 24H2`.
- Mọi chuỗi mới có đủ VI và EN trong `src/i18n.ts`. Không dùng `isVi ? … : …`.

---

## 6. Từng hệ máy

### 6.1. macOS
- **Chạy lệnh và quyền:** dùng nguyên `build_privileged_command` (script `sh` POSIX cùng `sudo`) và luồng mở khoá sudo như Linux. Phải kiểm tra trên máy ảo rằng mọi lệnh trong khung chạy (`mktemp`, `install`, `id`, `sudo -S`) đều chạy đúng với bản BSD của macOS.
- **SSH, xem:** `sshd -T -C …` như Linux.
- **SSH, ghi:** ghi file phụ `/etc/ssh/sshd_config.d/00-sshbox.conf` khi có dòng `Include` đứng đầu, không có thì sửa file chính (`sshd_write_target` sẵn có). Trên macOS, mỗi kết nối SSH được hệ thống (`launchd`) mở bằng một `sshd` mới, nên cấu hình mới có hiệu lực ngay từ kết nối kế tiếp mà **không cần nạp lại**. Bỏ bước `ss` (macOS không có lệnh này). Bằng chứng duy nhất vẫn là kết nối thử mới từ app (spec gốc §4.2 bước 8).
- **Tắt đăng nhập bằng mật khẩu:** trên macOS, `UsePAM yes` đang bật và `KbdInteractiveAuthentication` mặc định cũng bật. Nếu chỉ đặt `PasswordAuthentication no` thì vẫn đăng nhập bằng mật khẩu được qua cách "keyboard-interactive". *(các quyết định dưới đây là của Claude)*
  - Dòng "Đăng nhập bằng mật khẩu" trên macOS điều khiển **cả hai** directive.
  - Trạng thái hiện "đang bật" nếu còn một trong hai đang bật.
  - Bản xem trước liệt kê cả hai dòng.
  - Windows cũng làm như vậy nếu `sshd -T` ở đó báo `kbdinteractiveauthentication yes`.
  - Linux có thể có cùng lỗ hổng trên distro nào bật `KbdInteractiveAuthentication`. Ghi lại thành việc riêng, không sửa trong đợt này, để giữ nguyên hành vi Linux.
- **Tự hoàn tác:** thay `systemd-run` bằng một cơ chế hẹn giờ của macOS. Cơ chế này phải chạy được kể cả khi phiên SSH đã đóng, và huỷ được khi app xác nhận giữ thay đổi. Người làm tự chọn cách, ví dụ một LaunchDaemon tạm nạp bằng `launchctl bootstrap system`, xoá khi giữ thay đổi.
- **Ẩn đổi port:** trên macOS, cổng 22 do `launchd` giữ chứ không do `sshd_config`, nên sửa `Port` không có tác dụng. Đổi port thật phải sửa file cấu hình hệ thống, vốn được macOS bảo vệ khỏi bị sửa (SIP).
- **Users & Keys (chỉ xem):** user có UID ≥ 501 và `root`, có thuộc nhóm `admin` không, các key trong `~/.ssh/authorized_keys` của từng user.

### 6.2. Windows (OpenSSH Server có sẵn trong Windows 10 1809 trở lên và Windows 11)
**SSH**
- Xem bằng `sshd.exe -T` (`C:\Windows\System32\OpenSSH\`), file cấu hình `C:\ProgramData\ssh\sshd_config`.
- Ghi vào file chính, giữ nguyên comment, kiểu xuống dòng và mã hoá ký tự của file. File mặc định của Windows **kết thúc bằng khối `Match Group administrators`**, nên mọi directive phải được chèn **trước** dòng `Match` đầu tiên, nếu không chúng chỉ áp dụng cho nhóm admin.
- Kiểm tra cú pháp bằng `sshd.exe -t`, nạp lại bằng `Restart-Service sshd`.
- Tự hoàn tác bằng một Scheduled Task chạy dưới tài khoản SYSTEM, chạy một lần sau 60 giây: khôi phục bản sao lưu, khởi động lại `sshd`, đóng cổng mới trên firewall nếu có. Task này bị xoá khi app xác nhận giữ thay đổi.
- Đổi port: thêm rule cho phép port mới trong nhóm `ssh-box` của Windows Firewall trước khi khởi động lại. Kiểm tra cổng bằng `Get-NetTCPConnection -State Listen -LocalPort <port>`.
- Bộ chặn tự khoá: các dòng SSH ở spec gốc §4.3 vẫn áp dụng (`password_in_use`, `pubkey_in_use`, `both_auth_off`, `no_sessions`).

**Firewall (Windows Defender Firewall)** *(các giới hạn dưới đây là quyết định của Claude)*
- Thêm giá trị `windows` vào `FirewallEngine`.
- **Xem:** trạng thái bật/tắt của 3 profile (Domain, Private, Public) và profile đang dùng; các cổng đang lắng nghe kèm tiến trình và mức phơi ra ngoài; các rule inbound đang bật **có lọc theo cổng**. Không liệt kê hàng trăm rule mặc định theo chương trình của Windows.
- **Thêm rule:** allow hoặc block, theo cổng, giao thức, nguồn, vào nhóm rule `ssh-box`.
- **Xoá rule:** chỉ xoá được rule thuộc nhóm `ssh-box`. Các rule khác, kể cả rule `OpenSSH-Server-In-TCP` của Windows, chỉ được xem.
- **Bật/tắt:** áp dụng cho cả 3 profile cùng lúc. Khi bật, dòng "tự thêm rule cho phép `server_port`" ở spec gốc §4.3 vẫn áp dụng.
- **Mức phơi ra ngoài của cổng:** Windows hay cho phép kết nối theo chương trình hoặc dịch vụ, không ghi cổng. App không đối chiếu các rule loại này, vì việc đó chậm và phức tạp. Vì vậy, khi firewall đang bật mà không có rule theo cổng nào khớp, cổng hiện trạng thái **"Không xác định"** (màu `neutral`) chứ không hiện "bị chặn". Thêm giá trị này vào kiểu trạng thái và chuỗi i18n. Bộ chặn tự khoá không bị ảnh hưởng: bỏ qua các rule theo chương trình chỉ khiến nó thận trọng hơn. *(quyết định của Claude)*
- **Khác biệt quan trọng cho bộ chặn và cách tính mức phơi ra ngoài:** ở Windows Firewall, rule **block luôn thắng rule allow**, bất kể thứ tự. Không có quy tắc "rule khớp đầu tiên thắng" như `ufw`. `check_firewall_add`, `check_firewall_delete` và `compute_socket_exposure` phải dùng luật này khi engine là `windows`.

**Users & Keys (chỉ xem)**
- Liệt kê user cục bộ (`Get-LocalUser`), có thuộc nhóm Administrators không, và các key đang có hiệu lực. User thuộc nhóm admin dùng file `C:\ProgramData\ssh\administrators_authorized_keys` (khi khối `Match Group administrators` mặc định còn đó). User khác dùng `C:\Users\<user>\.ssh\authorized_keys`. Giao diện ghi rõ key đang được đọc từ file nào.

---

## 7. Kiểm thử

### 7.1. Unit test Rust (`cargo test`)
Mỗi test có một câu nói rõ nó bảo vệ điều gì, và phải được chạy một lần trên code chưa sửa để thấy nó bắt được lỗi (giống spec gốc §6.1).
- Đọc kết quả probe của macOS và Windows; `MINGW…` / `CYGWIN…` ra `UNSUPPORTED`.
- Bảng mức hỗ trợ: mỗi ô của bảng §4 có một test. Mỗi điều kiện ở §3.1 có một test: thiếu điều kiện đó thì ô tương ứng về Chỉ xem. Thêm một test chứng minh bảng của Linux cho ra đúng hành vi hiện tại (§3.2).
- Bộ chạy lệnh Windows: bí mật không bao giờ xuất hiện trong dòng lệnh; script có `'`, `"`, `$`, `` ` ``, ký tự Unicode tiếng Việt vẫn tới nguyên vẹn sau khi mã hoá.
- Chèn directive vào `sshd_config` của Windows: nằm trước `Match` đầu tiên, giữ CRLF nếu file dùng CRLF.
- Luật "block thắng" của Windows, ở đúng những chỗ Windows khác `ufw`:
  - (a) **Mức phơi ra ngoài:** một rule allow đứng trước và một rule block cùng cổng đứng sau. Với `ufw` cổng là "mở", với `windows` cổng là "bị chặn".
  - (b) **`check_firewall_delete` với engine `windows`:** một rule allow `server_port` cho IP người dùng mà đang bị một rule block khớp đè lên thì không được tính là "còn đường vào".
- Script hoàn tác của macOS và Windows được dựng đúng (đường dẫn, lệnh huỷ hẹn giờ).

### 7.2. Test tích hợp
Các test sshd tạm có sẵn (`src-tauri/tests/vps_sshd.rs`) vẫn phải qua.

### 7.3. Ma trận thủ công trên máy ảo
**Máy ảo:** UTM trên chiếc Mac này (chip Apple, 16 GB RAM, chạy mỗi lần một máy ảo): Windows 11 ARM có bật OpenSSH Server, và bản macOS mới nhất mà UTM cài được. Linux chạy lại trên các máy ảo Lima có sẵn (`u24`, `d13`, `r9`, bằng `src-tauri/tests/vps_matrix.rs`) để chứng minh không có gì thay đổi. Không bao giờ thử trên máy thật.

| Kịch bản | macOS | Windows | Kết quả mong đợi |
|---|---|---|---|
| Probe | ✓ | ✓ | Đúng hệ, đúng phiên bản, đúng quyền; tab và dòng hiện đúng theo §4 |
| Tắt password khi app đăng nhập bằng key | ✓ | ✓ | Kết nối mới thành công, thay đổi được giữ |
| Tắt password khi app đăng nhập bằng password | ✓ | ✓ | Bị chặn, có hướng dẫn |
| Sau khi tắt password, thử `ssh -o PreferredAuthentications=keyboard-interactive,password` | ✓ | ✓ | Đăng nhập thất bại |
| Áp dụng rồi ngắt app trước khi xác minh | ✓ | ✓ | Sau 60 giây cấu hình cũ trở lại, SSH vào được |
| Đổi port sang một port bị chặn | — | ✓ (nếu Đầy đủ) | Kết nối thử thất bại, tự hoàn tác, port cũ vẫn vào được |
| Thêm rule block `server_port` từ mọi nơi | — | ✓ | Bị chặn |
| Bật firewall rồi khởi động lại máy | — | ✓ | Rule còn, SSH vẫn vào được |
| Tài khoản không phải admin | — | ✓ | Mọi tab Chỉ xem, có lý do |

**Phía app:** macOS app, hai theme sáng và tối, tiếng Việt và tiếng Anh, rộng 360px, như spec gốc §6.3. Kiểm tra app trên Windows, iOS và Android vẫn còn mở như ở MR !50, vì máy này không có Xcode và Android SDK.

### 7.4. Tiêu chí hoàn thành
- `cargo test` và `npm run build` đều qua.
- Ma trận §7.3 đã đi hết, kết quả đạt/không đạt ghi trong mô tả MR.
- Kết quả thử nghiệm §8 và mức cuối cùng của từng ô trong bảng §4 được ghi lại trong MR.

---

## 8. Những điều phải thử thật trước khi viết phần ghi cho Windows/macOS

Chưa chắc chắn, phải thử trên máy ảo ngay sau khi dựng xong. Kết quả quyết định mức của Windows SSH:

1. **Windows:** `Restart-Service sshd` có làm rớt phiên SSH đang chạy lệnh không? Nếu có, luồng áp dụng phải đổi: script tự chạy nền (Scheduled Task), app chờ rồi mở kết nối mới để xác minh. Không làm được an toàn thì Windows SSH ở mức **Chỉ xem**.
2. **Windows:** SSH vào bằng tài khoản admin có được token quản trị thật không (để biết nhận diện `admin` / `none` ở §5.2 có đúng không)?
3. **Windows:** phiên SSH có biến `SSH_CONNECTION` không? Nếu không, phải tìm cách khác để lấy `client_ip` và `server_port`. Không lấy được `client_ip` thì Firewall chỉ ở mức Chỉ xem, vì bộ chặn cần thông tin này.
4. **Windows:** gửi stdin qua `powershell -EncodedCommand` trong kênh exec của `russh` có tới đủ và đúng không?
5. **macOS:** cơ chế hẹn giờ hoàn tác còn chạy sau khi phiên SSH đóng, và huỷ được.
6. **macOS:** bản macOS trên máy ảo phải đi đúng nhánh ghi file phụ. Trường hợp không có dòng `Include` thì được kiểm bằng unit test với kết quả probe giả, phải ra mức Chỉ xem.

---

## 9. Thứ tự thực hiện

Làm tuần tự trong một luồng, trên nhánh `feat/server-config-multi-os` tách từ `develop`. Mỗi bước tối đa khoảng 400 dòng code; bước nào lớn hơn thì tách tiếp.

1. **Nền móng backend:** nhận diện hệ máy, bộ chạy lệnh Windows, bảng mức hỗ trợ (ban đầu mọi ô của macOS/Windows là Ẩn), test §7.1 tương ứng.
2. **Nền móng giao diện:** thanh tab theo bảng mức, tên hệ trên thanh máy, quyền `admin`, chuỗi i18n. Sau bước này, máy Windows/macOS hiện đúng tên hệ thay vì báo lỗi.
3. **Dựng máy ảo và thử nghiệm §8.** Hỏi người dùng trước khi tải. Ghi kết quả vào spec này.
4. **macOS:** SSH xem và ghi (trừ port) kèm tự hoàn tác; Users & Keys chỉ xem.
5. **Windows, phần xem:** SSH, Firewall, Users & Keys.
6. **Windows, phần ghi:** Firewall; SSH chỉ khi thử nghiệm §8 cho phép.
7. **Ma trận §7.3,** chạy lại ma trận Linux, ghi kết quả vào MR.

Mỗi bước mở thêm ô nào thì sửa bảng mức hỗ trợ và test của ô đó trong cùng bước.

---

## 10. Để người làm tự quyết
- Lệnh và cmdlet cụ thể, miễn là đáp ứng đúng yêu cầu ở §5 và §6.
- Cách nhận diện hệ máy ở §5.1, miễn là lệnh đầu tiên vô hại trên cả ba loại shell.
- Cơ chế hẹn giờ hoàn tác trên macOS.
- Hình dạng dữ liệu của bảng mức hỗ trợ, tên hàm, tên biến.

## 11. Các quyết định do Claude đưa ra (người dùng chưa duyệt từng điểm)
1. Backend tính bảng mức hỗ trợ, frontend chỉ hiển thị (§3).
2. Dòng cài đặt bị ẩn thì ẩn hẳn, không kèm ghi chú (§3).
3. Windows: ẩn `PermitRootLogin`, `AllowUsers`, `DenyUsers` (§4).
4. Windows Firewall: chỉ hiện rule có lọc theo cổng; chỉ xoá được rule trong nhóm `ssh-box`; bật/tắt cả 3 profile cùng lúc (§6.2).
5. Users & Keys ở mức Chỉ xem trên cả macOS lẫn Windows trong đợt này (§4).
6. Thêm quyền `admin` cho Windows, không hiện ô mật khẩu sudo (§5.2).
7. Windows Server theo cùng quy tắc "theo khả năng của máy" như Windows 10/11, dù không thử được trên máy ảo (§2.2). Đây là hệ quả của câu trả lời "theo khả năng máy" của người dùng.
8. Firewall được bảo vệ bằng bộ chặn như Linux, không cần hẹn giờ hoàn tác (§3).
9. Trên macOS (và Windows nếu cần), dòng "Đăng nhập bằng mật khẩu" điều khiển cả `KbdInteractiveAuthentication` (§6.1).
10. Tài khoản Windows không phải admin: hiện thông báo kèm những gì đọc được, không tự parse `sshd_config` (§4).
11. Cổng trên Windows không khớp rule theo cổng nào thì hiện "Không xác định" (§6.2).
12. macOS không có dòng `Include` thì tab SSH ở mức Chỉ xem (§3.1).
