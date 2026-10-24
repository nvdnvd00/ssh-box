import { AppLanguage } from "./themes";

export interface AppI18nStrings {
  // Navigation & Sidebar
  navDashboard: string;
  navHome: string;
  navBack: string;
  navKeychain: string;
  navMachines: string;
  navIdentities: string;
  navSshKeys: string;
  navKnownHosts: string;
  navProxyProfiles: string;
  navPortForwarding: string;
  navSnippets: string;
  navDocker: string;
  navSftp: string;
  navTerminal: string;
  navSettings: string;
  quickConnectBtn: string;
  newLocalShell: string;

  // Command Palette (Cmd+K)
  cmdPalettePlaceholder: string;
  cmdPaletteNoResults: string;
  cmdPaletteSectionHosts: string;
  cmdPaletteSectionSnippets: string;
  cmdPaletteSectionPortForward: string;
  cmdPaletteSectionQuickActions: string;
  cmdActionOpenSsh: string;
  cmdActionOpenSftp: string;
  cmdActionRunSnippet: string;
  cmdActionToggleTunnel: string;

  // Terminal & Productivity
  termSearchPlaceholder: string;
  termBroadcastLabel: string;
  termBroadcastActive: string;
  termSnippetsDrawer: string;
  termRunSnippetHere: string;
  termFillVariablesTitle: string;
  termRunBtn: string;
  termCancelBtn: string;
  termSessionLogStart: string;
  termSessionLogStop: string;
  termSessionLogExport: string;
  termSessionLogSaved: string;
  termSplitVertical: string;
  termSplitHorizontal: string;
  termReconnect: string;
  termClosePane: string;

  // SSH, Keychain & Security
  exportKeyToHostBtn: string;
  exportKeyModalTitle: string;
  exportKeyModalSubtitle: string;
  exportKeySuccess: string;
  knownHostsTitle: string;
  knownHostsSubtitle: string;
  knownHostsEmpty: string;
  knownHostsDelete: string;
  knownHostsClearAll: string;
  hostAgentForwarding: string;
  hostAgentForwardingDesc: string;
  hostStartupCommand: string;
  hostStartupCommandPlaceholder: string;
  hostEnvVars: string;
  hostEnvVarsPlaceholder: string;

  // SFTP & Fast Sync / Chunk Stream
  sftpLocalSystem: string;
  sftpSelectRemoteHost: string;
  sftpUpload: string;
  sftpDownload: string;
  sftpFastSyncTag: string;
  sftpChunkStreamTag: string;
  sftpFastSyncActiveDesc: string;
  sftpChunkStreamActiveDesc: string;
  sftpTransferProgress: string;
  sftpTransferCompleted: string;
  sftpNewFolder: string;
  sftpNewFile: string;
  sftpRefresh: string;
  sftpToggleHidden: string;

  // Settings & Language
  settingsLanguageLabel: string;
  settingsLanguageDesc: string;
  settingsFastSyncLabel: string;
  settingsFastSyncDesc: string;
  settingsBackupTitle: string;
  settingsBackupExport: string;
  settingsBackupImport: string;
  settingsClipboardTitle: string;
  settingsCopyOnSelectLabel: string;
  settingsCopyOnSelectDesc: string;
  settingsLocalShellLabel: string;
  settingsLocalShellDesc: string;
  settingsLocalShellAuto: string;

  // App Update
  updateBtn: string;
  updateInstalling: string;
  updateConfirm: string;
}

const VI_STRINGS: AppI18nStrings = {
  navDashboard: "Bảng điều khiển",
  navHome: "Trang chủ",
  navBack: "Quay lại",
  navKeychain: "Kho khóa (Keychain)",
  navMachines: "Máy chủ (Hosts)",
  navIdentities: "Định danh (Identities)",
  navSshKeys: "Khóa SSH (Keys)",
  navKnownHosts: "Known Hosts",
  navProxyProfiles: "Hồ sơ Proxy",
  navPortForwarding: "Chuyển tiếp cổng",
  navSnippets: "Đoạn lệnh (Snippets)",
  navDocker: "Quản lý Docker",
  navSftp: "Truyền tệp SFTP",
  navTerminal: "Terminal",
  navSettings: "Cài đặt",
  quickConnectBtn: "Tìm & Kết nối nhanh (⌘K)",
  newLocalShell: "Mở Terminal nội bộ",

  cmdPalettePlaceholder:
    "Tìm máy chủ SSH, mở SFTP, chạy Snippet hoặc bật Port Forwarding...",
  cmdPaletteNoResults: "Không tìm thấy kết quả phù hợp.",
  cmdPaletteSectionHosts: "Máy chủ SSH & SFTP",
  cmdPaletteSectionSnippets: "Đoạn lệnh nhanh (Snippets)",
  cmdPaletteSectionPortForward: "Chuyển tiếp cổng (Port Forwarding)",
  cmdPaletteSectionQuickActions: "Thao tác hệ thống",
  cmdActionOpenSsh: "Kết nối SSH",
  cmdActionOpenSftp: "Mở SFTP",
  cmdActionRunSnippet: "Chèn vào Terminal",
  cmdActionToggleTunnel: "Bật/Tắt Tunnel",

  termSearchPlaceholder: "Tìm trong Terminal (Enter / Shift+Enter)...",
  termBroadcastLabel: "Multi-Exec",
  termBroadcastActive: "Đang gõ đồng thời tất cả Pane",
  termSnippetsDrawer: "Snippets nhanh",
  termRunSnippetHere: "Chạy trong Pane này",
  termFillVariablesTitle: "Điền tham số cho Snippet",
  termRunBtn: "Thực thi",
  termCancelBtn: "Hủy",
  termSessionLogStart: "Ghi Log phiên",
  termSessionLogStop: "Đang ghi Log (REC)",
  termSessionLogExport: "Xuất file .log",
  termSessionLogSaved: "Đã lưu file log phiên làm việc",
  termSplitVertical: "Chia dọc",
  termSplitHorizontal: "Chia ngang",
  termReconnect: "Kết nối lại",
  termClosePane: "Đóng khung",

  exportKeyToHostBtn: "Đẩy Key lên Host (1-Click)",
  exportKeyModalTitle: "Đẩy khóa công khai lên máy chủ (ssh-copy-id)",
  exportKeyModalSubtitle:
    "Tự động thêm Public Key vào ~/.ssh/authorized_keys, phân quyền chuẩn 600 và gắn khóa vào cấu hình máy chủ.",
  exportKeySuccess: "Đã đẩy khóa SSH lên máy chủ và liên kết thành công!",
  knownHostsTitle: "Quản lý Known Hosts (Host Key Fingerprints)",
  knownHostsSubtitle:
    "Kiểm soát dấu vân tay bảo mật SHA-256 của các máy chủ đã kết nối để chống tấn công giả mạo (MITM).",
  knownHostsEmpty: "Chưa có khóa máy chủ nào được lưu trong ~/.ssh/known_hosts.",
  knownHostsDelete: "Xóa khóa",
  knownHostsClearAll: "Xóa toàn bộ",
  hostAgentForwarding: "Chuyển tiếp SSH Agent (-A)",
  hostAgentForwardingDesc:
    "Cho phép sử dụng khóa SSH ở máy local khi đứng bên trong máy chủ từ xa (ví dụ git pull/push).",
  hostStartupCommand: "Lệnh khởi chạy tự động (Startup Snippet)",
  hostStartupCommandPlaceholder: "Ví dụ: tmux attach -t main || tmux new -s main",
  hostEnvVars: "Biến môi trường tùy chỉnh (ENV)",
  hostEnvVarsPlaceholder: "Ví dụ: APP_ENV=production\nTZ=Asia/Ho_Chi_Minh",

  sftpLocalSystem: "Máy cục bộ (Local)",
  sftpSelectRemoteHost: "Chọn máy chủ từ xa...",
  sftpUpload: "Tải lên",
  sftpDownload: "Tải xuống",
  sftpFastSyncTag: "FAST SYNC",
  sftpChunkStreamTag: "CHUNK STREAM",
  sftpFastSyncActiveDesc: "Tự động kích hoạt đồng bộ nén/delta tốc độ cao",
  sftpChunkStreamActiveDesc: "Truyền theo luồng 64KB không ngốn RAM",
  sftpTransferProgress: "Đang truyền dữ liệu...",
  sftpTransferCompleted: "Hoàn tất truyền tệp",
  sftpNewFolder: "Tạo thư mục",
  sftpNewFile: "Tạo tệp mới",
  sftpRefresh: "Làm mới",
  sftpToggleHidden: "Ẩn/Hiện tệp ẩn",

  settingsLanguageLabel: "Ngôn ngữ giao diện (Language)",
  settingsLanguageDesc: "Chuyển đổi ngôn ngữ hiển thị giữa Tiếng Việt và Tiếng Anh",
  settingsFastSyncLabel: "Tự động bật Fast Sync khi truyền tệp",
  settingsFastSyncDesc:
    "Sử dụng rsync delta hoặc luồng nén tốc độ cao kèm gắn nhãn FAST SYNC trên SFTP",
  settingsBackupTitle: "Sao lưu & Khôi phục dữ liệu (Vault Backup)",
  settingsBackupExport: "Xuất bản sao lưu (.json)",
  settingsBackupImport: "Khôi phục từ bản sao lưu",
  settingsClipboardTitle: "Clipboard",
  settingsCopyOnSelectLabel: "Tự động copy khi bôi đen",
  settingsCopyOnSelectDesc: "Đoạn văn bản được bôi đen trong terminal sẽ tự động được copy vào clipboard",
  settingsLocalShellLabel: "Shell cho terminal local",
  settingsLocalShellDesc: "Áp dụng cho tab local mở sau khi đổi",
  settingsLocalShellAuto: "Mặc định hệ thống",

  updateBtn: "Cập nhật",
  updateInstalling: "Đang cập nhật...",
  updateConfirm:
    "Tải bản mới và khởi động lại app? Các phiên SSH đang mở sẽ bị ngắt.",
};

const EN_STRINGS: AppI18nStrings = {
  navDashboard: "Dashboard",
  navHome: "Home",
  navBack: "Back",
  navKeychain: "Keychain",
  navMachines: "Machines",
  navIdentities: "Identities",
  navSshKeys: "SSH Keys",
  navKnownHosts: "Known Hosts",
  navProxyProfiles: "Proxy Profiles",
  navPortForwarding: "Port Forwarding",
  navSnippets: "Snippets",
  navDocker: "Docker",
  navSftp: "SFTP Browser",
  navTerminal: "Terminal",
  navSettings: "Settings",
  quickConnectBtn: "Quick Connect & Actions (⌘K)",
  newLocalShell: "New Local Shell",

  cmdPalettePlaceholder:
    "Search SSH hosts, open SFTP, run snippets, or toggle port forwarding...",
  cmdPaletteNoResults: "No matching results found.",
  cmdPaletteSectionHosts: "SSH Machines & SFTP",
  cmdPaletteSectionSnippets: "Quick Snippets",
  cmdPaletteSectionPortForward: "Port Forwarding Rules",
  cmdPaletteSectionQuickActions: "System Actions",
  cmdActionOpenSsh: "Connect SSH",
  cmdActionOpenSftp: "Open SFTP",
  cmdActionRunSnippet: "Insert into Terminal",
  cmdActionToggleTunnel: "Toggle Tunnel",

  termSearchPlaceholder: "Find in terminal (Enter / Shift+Enter)...",
  termBroadcastLabel: "Multi-Exec",
  termBroadcastActive: "Broadcasting input to all panes",
  termSnippetsDrawer: "Quick Snippets",
  termRunSnippetHere: "Run in Active Pane",
  termFillVariablesTitle: "Fill Snippet Variables",
  termRunBtn: "Execute",
  termCancelBtn: "Cancel",
  termSessionLogStart: "Record Session",
  termSessionLogStop: "Recording (REC)",
  termSessionLogExport: "Export .log",
  termSessionLogSaved: "Session log saved to disk",
  termSplitVertical: "Split Vertical",
  termSplitHorizontal: "Split Horizontal",
  termReconnect: "Reconnect",
  termClosePane: "Close Pane",

  exportKeyToHostBtn: "Export to Host (1-Click)",
  exportKeyModalTitle: "Export Public Key to Host (ssh-copy-id)",
  exportKeyModalSubtitle:
    "Automatically append public key to ~/.ssh/authorized_keys with 600 permissions and link this key to the host.",
  exportKeySuccess: "Public key exported and linked to host!",
  knownHostsTitle: "Known Hosts Manager (Host Key Fingerprints)",
  knownHostsSubtitle:
    "Inspect and manage trusted SHA-256 host key fingerprints to prevent MITM attacks.",
  knownHostsEmpty: "No trusted host keys recorded in ~/.ssh/known_hosts yet.",
  knownHostsDelete: "Remove Key",
  knownHostsClearAll: "Clear All",
  hostAgentForwarding: "SSH Agent Forwarding (-A)",
  hostAgentForwardingDesc:
    "Allow remote server commands to use your local SSH keys (e.g. git pull/push).",
  hostStartupCommand: "Startup Command / Snippet",
  hostStartupCommandPlaceholder: "e.g. tmux attach -t main || tmux new -s main",
  hostEnvVars: "Custom Environment Variables (ENV)",
  hostEnvVarsPlaceholder: "e.g. APP_ENV=production\nTZ=Asia/Ho_Chi_Minh",

  sftpLocalSystem: "Local System",
  sftpSelectRemoteHost: "Select SSH Machine...",
  sftpUpload: "Upload",
  sftpDownload: "Download",
  sftpFastSyncTag: "FAST SYNC",
  sftpChunkStreamTag: "CHUNK STREAM",
  sftpFastSyncActiveDesc: "High-speed delta / compressed stream enabled",
  sftpChunkStreamActiveDesc: "Zero-RAM-bloat 64KB chunked streaming",
  sftpTransferProgress: "Transferring data...",
  sftpTransferCompleted: "Transfer complete",
  sftpNewFolder: "New Folder",
  sftpNewFile: "New File",
  sftpRefresh: "Refresh",
  sftpToggleHidden: "Toggle Hidden Files",

  settingsLanguageLabel: "Interface Language (Ngôn ngữ)",
  settingsLanguageDesc: "Switch UI display language between Tiếng Việt and English",
  settingsFastSyncLabel: "Auto-enable Fast Sync on File Transfers",
  settingsFastSyncDesc:
    "Automatically use rsync delta or compressed streaming with FAST SYNC badge",
  settingsBackupTitle: "Vault Configuration Backup & Restore",
  settingsBackupExport: "Export Vault Backup (.json)",
  settingsBackupImport: "Import Vault Backup",
  settingsClipboardTitle: "Clipboard",
  settingsCopyOnSelectLabel: "Copy on Select",
  settingsCopyOnSelectDesc: "Automatically copy highlighted terminal text to the clipboard",
  settingsLocalShellLabel: "Local Terminal Shell",
  settingsLocalShellDesc: "Applies to local tabs opened after the change",
  settingsLocalShellAuto: "System default",

  updateBtn: "Update",
  updateInstalling: "Updating...",
  updateConfirm:
    "Download the new version and restart the app? Open SSH sessions will be closed.",
};

export function getI18n(lang?: AppLanguage): AppI18nStrings {
  return lang === "en" ? EN_STRINGS : VI_STRINGS;
}

export function extractSnippetVariables(code: string): string[] {
  const regex = /\{\{\s*([a-zA-Z0-9_.-]+)\s*\}\}/g;
  const vars: string[] = [];
  let match: RegExpExecArray | null;
  while ((match = regex.exec(code)) !== null) {
    const name = match[1].trim();
    if (name && !vars.includes(name)) {
      vars.push(name);
    }
  }
  return vars;
}

export function interpolateSnippetVariables(
  code: string,
  values: Record<string, string>,
): string {
  return code.replace(/\{\{\s*([a-zA-Z0-9_.-]+)\s*\}\}/g, (_, key: string) => {
    const trimmed = key.trim();
    return values[trimmed] ?? "";
  });
}
