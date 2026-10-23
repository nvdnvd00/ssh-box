# Thiết kế Kỹ thuật: Trình Quản lý File SFTP 2 Cột (Dual-Pane Local ↔ Remote)

**Ngày:** 2026-10-23  
**Dự án:** `ssh-box` (`~/ssh-box`)  
**Mục tiêu:** Bổ sung tính năng quản lý file SFTP 2 cột (Local ↔ Remote) vào Toolbox, cho phép duyệt thư mục, truyền file/thư mục 2 chiều, tạo mới, đổi tên, xóa, phân quyền (`chmod`), và chỉnh sửa nhanh file văn bản trên cả máy cục bộ lẫn máy chủ SSH từ xa.

---

## 1. Kiến trúc & Module

### 1.1. Backend Rust (`src-tauri/src/sftp.rs` & `src-tauri/src/ssh_pool.rs`)
- Tận dụng kết nối SSH Multiplexed sẵn có trong `SshPool` (`russh`) để thực thi thao tác file từ xa và truyền luồng dữ liệu nhị phân mà không cần tạo kết nối TCP mới hay thêm crate ngoài.
- Cấu trúc dữ liệu `SftpEntry`:
  - `name: String`
  - `path: String`
  - `kind: String` (`"dir"` | `"file"` | `"symlink"`)
  - `size: u64`
  - `modified: u64` (Unix timestamp)
  - `permissions: String` (ví dụ `"drwxr-xr-x"`)
  - `mode_octal: String` (ví dụ `"0755"`)
- Cấu trúc phản hồi `SftpListResponse`:
  - `cwd: String` (đường dẫn tuyệt đối đã chuẩn hóa)
  - `entries: Vec<SftpEntry>`
- Các lệnh Tauri IPC trong `src-tauri/src/lib.rs`:
  - `sftp_list_dir(state, host_id, path)`
  - `sftp_mkdir(state, host_id, path)`
  - `sftp_rename(state, host_id, old_path, new_path)`
  - `sftp_delete(state, host_id, path)`
  - `sftp_chmod(state, host_id, path, mode)`
  - `sftp_read_text(state, host_id, path)`
  - `sftp_write_text(state, host_id, path, content)`
  - `sftp_transfer(state, source_host_id, source_path, target_host_id, target_dir)`

### 1.2. Frontend React (`src/components/SftpWorkspace.tsx` & Tích hợp)
- Thêm `"sftp"` vào union `TabResources` trong `src/types.ts`.
- Thêm mục **SFTP** vào nhóm **Toolbox** ở thanh bên trái (`src/App.tsx`).
- Thêm nút mở nhanh **SFTP** từ:
  - Thẻ Machine trong `src/components/ResourcesWorkspace.tsx` (kèm trường cấu hình `Default SFTP Path` trong modal tạo/sửa Machine).
  - Thanh tiêu đề chi tiết máy chủ trong `src/components/DashboardWorkspace.tsx`.
  - Thanh công cụ phía trên của `Terminal` trong `src/App.tsx`.
- Giao diện `SftpWorkspace`:
  - **Bố cục 2 cột song song:** Cột trái (`Local Machine`) và Cột phải (`Remote Machine` với dropdown chọn máy chủ SSH).
  - **Thanh điều hướng mỗi cột:** Nút Lên (`..`), Home (`~`), ô nhập đường dẫn trực tiếp, Tìm kiếm lọc nhanh, Bật/tắt file ẩn (`.`), Làm mới.
  - **Thanh thao tác mỗi cột:** Upload → / ← Download, New Folder, New File, Edit Text, Rename, Chmod, Delete.
  - **Modal tích hợp:** Xem/Sửa văn bản nhanh, Đổi quyền (`chmod` octal + checkboxes), Tạo thư mục/file, Đổi tên.

---

## 2. Xử lý Lỗi & Kiểm thử
- Mọi đường dẫn truyền qua shell từ xa được bọc nháy đơn chuẩn POSIX (`'...'` với `'\\''`) để chống lỗi ký tự đặc biệt và khoảng trắng.
- Giới hạn kích thước đọc văn bản trực tiếp ở mức `2 MB` để bảo vệ bộ nhớ giao diện.
- Kiểm thử tự động với `cargo test` cho parser danh sách file từ xa, thao tác file cục bộ, và kiểm tra biên dịch toàn bộ dự án với `npm run build`.
