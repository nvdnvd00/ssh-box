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
  navVpsConfig: string;
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
  settingsClipboardTitle: string;
  settingsCopyOnSelectLabel: string;
  settingsCopyOnSelectDesc: string;
  settingsLocalShellLabel: string;
  settingsLocalShellDesc: string;
  settingsLocalShellAuto: string;
  settingsTmuxTitle: string;
  settingsTmuxSshLabel: string;
  settingsTmuxSshDesc: string;
  settingsTmuxLocalLabel: string;
  settingsTmuxLocalDesc: string;

  // App Update
  updateBtn: string;
  updateInstalling: string;
  updateConfirm: string;

  // Server Config (VPS) — shared
  vpsCommonApply: string;
  vpsCommonDiscard: string;
  vpsCommonPreviewApply: string;
  vpsCommonPreviewTitle: string;
  vpsCommonPendingChanges: string;
  vpsCommonRollbackNow: string;
  vpsCommonConfirm: string;
  vpsCommonCancel: string;
  vpsCommonReload: string;
  vpsCommonFix: string;
  vpsCommonPass: string;
  vpsCommonFail: string;
  vpsCommonLoading: string;
  vpsCommonReadOnly: string;
  vpsErrSudoRequired: string;
  vpsErrSudoDenied: string;
  vpsErrLockoutBlocked: string;
  vpsErrCommandFailed: string;
  vpsErrVerifyFailed: string;
  vpsErrUnsupported: string;
  vpsLockoutReasons: Record<string, string>;
  vpsUnsupportedReasons: Record<string, string>;
  vpsSshStatusTitle: string;
  vpsSshCheckPassword: string;
  vpsSshCheckRoot: string;
  vpsSshCheckMaxAuth: string;
  vpsSshGroupLogin: string;
  vpsSshGroupWho: string;
  vpsSshGroupLimits: string;
  vpsSshPort: string;
  vpsSshPortHint: string;
  vpsSshPassword: string;
  vpsSshPubkey: string;
  vpsSshRoot: string;
  vpsSshRootYes: string;
  vpsSshRootKeyOnly: string;
  vpsSshRootForced: string;
  vpsSshRootNo: string;
  vpsSshAllowUsers: string;
  vpsSshDenyUsers: string;
  vpsSshUsersHint: string;
  vpsSshMaxAuthTries: string;
  vpsSshMaxSessions: string;
  vpsSshLoginGrace: string;
  vpsSshAliveInterval: string;
  vpsSshAliveCount: string;
  vpsSshWriteDropIn: string;
  vpsSshWriteMain: string;
  vpsSshPending: string;
  vpsSshProgressTitle: string;
  vpsSshRollbackIn: string;
  vpsSshDone: string;
  vpsSshPortSaved: string;
  vpsSshRolledBack: string;
  vpsSshClose: string;
  vpsSshInvalid: string;
  vpsSshSteps: string[];
  vpsFwStatusTitle: string;
  vpsFwEnabled: string;
  vpsFwEngine: string;
  vpsFwIpv6Off: string;
  vpsFwNoEngine: string;
  vpsFwPortsTitle: string;
  vpsFwNoPorts: string;
  vpsFwRulesTitle: string;
  vpsFwNoRules: string;
  vpsFwFrom: string;
  vpsFwAnywhere: string;
  vpsFwAllPorts: string;
  vpsFwDelete: string;
  vpsFwDeleteConfirm: string;
  vpsFwAddTitle: string;
  vpsFwAction: string;
  vpsFwSource: string;
  vpsFwMyIp: string;
  vpsFwPort: string;
  vpsFwProto: string;
  vpsFwAny: string;
  vpsFwAdd: string;
  vpsFwOn: string;
  vpsFwOff: string;
  vpsFwExposure: Record<string, string>;
  vpsFwActions: Record<string, string>;
  vpsShellSelectHost: string;
  vpsShellNoHosts: string;
  vpsShellProbing: string;
  vpsTabSsh: string;
  vpsTabFirewall: string;
  vpsTabAccess: string;
  vpsPrivRoot: string;
  vpsPrivSudoNoPassword: string;
  vpsPrivSudoPassword: string;
  vpsPrivNone: string;
  vpsSudoTitle: string;
  vpsSudoHint: string;
  vpsSudoOpen: string;
  vpsSudoUnlock: string;
  vpsStudioPending: string;
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
  navVpsConfig: "Cấu hình VPS",
  navTerminal: "Terminal",
  navSettings: "Cài đặt",
  quickConnectBtn: "Tìm & Kết nối nhanh",
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
  settingsClipboardTitle: "Clipboard",
  settingsCopyOnSelectLabel: "Tự động copy khi bôi đen",
  settingsCopyOnSelectDesc: "Đoạn văn bản được bôi đen trong terminal sẽ tự động được copy vào clipboard",
  settingsLocalShellLabel: "Shell cho terminal local",
  settingsLocalShellDesc: "Áp dụng cho tab local mở sau khi đổi",
  settingsLocalShellAuto: "Mặc định hệ thống",
  settingsTmuxTitle: "Tmux",
  settingsTmuxSshLabel: "Tự động dùng tmux cho SSH",
  settingsTmuxSshDesc:
    "Tự gắn vào session 'ssh-box' trên máy chủ từ xa (bỏ qua nếu chưa cài tmux)",
  settingsTmuxLocalLabel: "Tự động dùng tmux cho Local Terminal",
  settingsTmuxLocalDesc:
    "Tự gắn vào session 'ssh-box' trên máy cục bộ (bỏ qua nếu chưa cài tmux)",

  updateBtn: "Cập nhật",
  updateInstalling: "Đang cập nhật...",
  updateConfirm:
    "Tải bản mới và khởi động lại app? Các phiên SSH đang mở sẽ bị ngắt.",
  vpsCommonApply: "Áp dụng",
  vpsCommonDiscard: "Bỏ",
  vpsCommonPreviewApply: "Xem trước & áp dụng",
  vpsCommonPreviewTitle: "Xem trước thay đổi",
  vpsCommonPendingChanges: "{n} thay đổi chưa áp dụng",
  vpsCommonRollbackNow: "Hoàn tác ngay",
  vpsCommonConfirm: "Xác nhận",
  vpsCommonCancel: "Huỷ",
  vpsCommonReload: "Tải lại",
  vpsCommonFix: "Sửa",
  vpsCommonPass: "Đạt",
  vpsCommonFail: "Chưa đạt",
  vpsCommonLoading: "Đang tải…",
  vpsCommonReadOnly: "Chỉ xem",
  vpsErrSudoRequired: "Cần mật khẩu sudo để làm việc này.",
  vpsErrSudoDenied: "Mật khẩu sudo không đúng.",
  vpsErrLockoutBlocked: "Thao tác bị chặn để bạn không tự khoá mình khỏi máy:",
  vpsErrCommandFailed: "Lệnh trên máy chủ bị lỗi:",
  vpsErrVerifyFailed: "Không mở được kết nối mới nên thay đổi đã được hoàn tác:",
  vpsErrUnsupported: "Máy chủ không hỗ trợ:",
  vpsLockoutReasons: {
    both_auth_off: "Không thể tắt cả đăng nhập bằng mật khẩu lẫn bằng SSH key.",
    password_in_use:
      "Ứng dụng đang đăng nhập bằng mật khẩu. Hãy chuyển máy này sang đăng nhập bằng SSH key trước khi tắt mật khẩu.",
    pubkey_in_use: "Ứng dụng đang đăng nhập bằng SSH key, nên không thể tắt đăng nhập bằng key.",
    root_in_use: "Ứng dụng đang đăng nhập bằng root. Hãy dùng một user khác có sudo trước khi chặn root.",
    root_password_in_use: "Ứng dụng đang đăng nhập root bằng mật khẩu. Hãy chuyển sang SSH key trước.",
    allow_users_excludes_me: "Danh sách AllowUsers mới không có user mà ứng dụng đang dùng.",
    deny_users_matches_me: "Danh sách DenyUsers mới chặn chính user mà ứng dụng đang dùng.",
    only_ssh_rule: "Đây là rule duy nhất cho phép ứng dụng vào cổng SSH.",
    closes_ssh_for_everyone: "Rule này đóng cổng SSH với mọi nơi, kể cả ứng dụng.",
    source_includes_me: "Nguồn của rule này có IP mà ứng dụng đang kết nối từ đó.",
    login_user: "Đây là user mà ứng dụng đang đăng nhập.",
    last_sudo_user: "Đây là user cuối cùng có quyền sudo.",
    key_in_use: "Đây là SSH key mà ứng dụng đang dùng để đăng nhập.",
  },
  vpsUnsupportedReasons: {
    linux_only: "Chỉ hỗ trợ máy chủ Linux",
    no_ssh_connection: "Không đọc được thông tin kết nối SSH ($SSH_CONNECTION) trên máy chủ.",
    no_systemd: "Máy chủ không dùng systemd nên chỉ xem được, không sửa được.",
    firewall_read_only: "Firewall này chỉ xem được. Cài ufw để sửa rule.",
  },
  vpsShellSelectHost: "Chọn máy",
  vpsShellNoHosts: "Chưa có máy chủ nào. Hãy thêm máy trong mục Máy chủ.",
  vpsShellProbing: "Đang kiểm tra máy chủ…",
  vpsTabSsh: "SSH",
  vpsTabFirewall: "Tường lửa",
  vpsTabAccess: "Người dùng & khoá SSH",
  vpsPrivRoot: "Root",
  vpsPrivSudoNoPassword: "Sudo không cần mật khẩu",
  vpsPrivSudoPassword: "Sudo đã mở khoá",
  vpsPrivNone: "Chưa có quyền sudo",
  vpsSudoTitle: "Mật khẩu sudo",
  vpsSudoHint: "Chỉ giữ trong bộ nhớ cho phiên làm việc này, không lưu lại.",
  vpsSudoOpen: "Nhập mật khẩu sudo",
  vpsSudoUnlock: "Mở khoá",
  vpsStudioPending: "Phần này đang được làm lại.",
  vpsSshStatusTitle: "Tình trạng",
  vpsSshCheckPassword: "Tắt đăng nhập bằng mật khẩu",
  vpsSshCheckRoot: "Không cho root đăng nhập bằng mật khẩu",
  vpsSshCheckMaxAuth: "Giới hạn số lần thử đăng nhập (tối đa 4)",
  vpsSshGroupLogin: "Đăng nhập",
  vpsSshGroupWho: "Ai được vào",
  vpsSshGroupLimits: "Giới hạn phiên",
  vpsSshPort: "Cổng SSH",
  vpsSshPortHint: "Cổng mới được thử bằng một kết nối mới trước khi lưu vào ứng dụng.",
  vpsSshPassword: "Đăng nhập bằng mật khẩu",
  vpsSshPubkey: "Đăng nhập bằng SSH key",
  vpsSshRoot: "Đăng nhập bằng root",
  vpsSshRootYes: "Cho phép",
  vpsSshRootKeyOnly: "Chỉ bằng SSH key",
  vpsSshRootForced: "Chỉ chạy lệnh cố định",
  vpsSshRootNo: "Không cho phép",
  vpsSshAllowUsers: "Chỉ cho các user này",
  vpsSshDenyUsers: "Chặn các user này",
  vpsSshUsersHint: "Cách nhau bằng dấu cách. Để trống là không giới hạn.",
  vpsSshMaxAuthTries: "Số lần thử đăng nhập tối đa",
  vpsSshMaxSessions: "Số phiên tối đa mỗi kết nối",
  vpsSshLoginGrace: "Thời gian chờ đăng nhập (giây)",
  vpsSshAliveInterval: "Kiểm tra kết nối mỗi (giây, 0 là tắt)",
  vpsSshAliveCount: "Số lần kiểm tra không phản hồi trước khi ngắt",
  vpsSshWriteDropIn: "Thay đổi được ghi vào /etc/ssh/sshd_config.d/00-sshbox.conf",
  vpsSshWriteMain: "Thay đổi được ghi vào /etc/ssh/sshd_config",
  vpsSshPending: "Một thay đổi trước đó đang chờ xác minh hoặc hoàn tác.",
  vpsSshProgressTitle: "Đang áp dụng thay đổi SSH",
  vpsSshRollbackIn: "Tự hoàn tác sau {s} giây nếu không xác minh được",
  vpsSshDone: "Đã áp dụng và xác minh bằng một kết nối mới.",
  vpsSshPortSaved: "Ứng dụng đã lưu cổng mới {p} cho máy này.",
  vpsSshRolledBack: "Đã hoàn tác về cấu hình trước.",
  vpsSshClose: "Đóng",
  vpsSshInvalid: "Có giá trị không hợp lệ.",
  vpsSshSteps: ["Kiểm tra bộ chặn tự khoá", "Sao lưu", "Ghi cấu hình", "Kiểm tra cú pháp (sshd -t)", "Hẹn giờ hoàn tác 60 giây", "Nạp lại SSH", "Kiểm tra cổng", "Kết nối thử từ ứng dụng", "Giữ thay đổi"],
  vpsFwStatusTitle: "Tình trạng",
  vpsFwEnabled: "Bật tường lửa",
  vpsFwEngine: "Công cụ tường lửa",
  vpsFwIpv6Off: "IPv6 không được lọc. Đặt IPV6=yes trong /etc/default/ufw để lọc cả IPv6.",
  vpsFwNoEngine: "Máy này chưa cài tường lửa. Cài ufw để dùng mục này.",
  vpsFwPortsTitle: "Cổng đang mở",
  vpsFwNoPorts: "Không có cổng nào đang lắng nghe.",
  vpsFwRulesTitle: "Quy tắc",
  vpsFwNoRules: "Chưa có quy tắc nào.",
  vpsFwFrom: "từ {s}",
  vpsFwAnywhere: "mọi nơi",
  vpsFwAllPorts: "mọi cổng",
  vpsFwDelete: "Xoá",
  vpsFwDeleteConfirm: "Xoá quy tắc",
  vpsFwAddTitle: "Thêm quy tắc",
  vpsFwAction: "Hành động",
  vpsFwSource: "Nguồn (IP hoặc dải IP, để trống là mọi nơi)",
  vpsFwMyIp: "IP của tôi",
  vpsFwPort: "Cổng (để trống là mọi cổng)",
  vpsFwProto: "Giao thức",
  vpsFwAny: "Tất cả",
  vpsFwAdd: "Thêm",
  vpsFwOn: "bật",
  vpsFwOff: "tắt",
  vpsFwExposure: { exposed: "Mở công khai", restricted: "Giới hạn IP", blocked: "Bị chặn", loopback: "Chỉ nội bộ" },
  vpsFwActions: { allow: "Cho phép", limit: "Cho phép, giới hạn tần suất", deny: "Chặn", reject: "Từ chối" },
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
  navVpsConfig: "Server Config",
  navTerminal: "Terminal",
  navSettings: "Settings",
  quickConnectBtn: "Quick Connect & Actions",
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
  settingsClipboardTitle: "Clipboard",
  settingsCopyOnSelectLabel: "Copy on Select",
  settingsCopyOnSelectDesc: "Automatically copy highlighted terminal text to the clipboard",
  settingsLocalShellLabel: "Local Terminal Shell",
  settingsLocalShellDesc: "Applies to local tabs opened after the change",
  settingsLocalShellAuto: "System default",
  settingsTmuxTitle: "Tmux",
  settingsTmuxSshLabel: "Auto-attach tmux on SSH",
  settingsTmuxSshDesc:
    "Attach or create 'ssh-box' session on remote hosts (falls back if tmux is missing)",
  settingsTmuxLocalLabel: "Auto-attach tmux on Local Terminal",
  settingsTmuxLocalDesc:
    "Attach or create 'ssh-box' session in local shells (falls back if tmux is missing)",

  updateBtn: "Update",
  updateInstalling: "Updating...",
  updateConfirm:
    "Download the new version and restart the app? Open SSH sessions will be closed.",
  vpsCommonApply: "Apply",
  vpsCommonDiscard: "Discard",
  vpsCommonPreviewApply: "Preview & apply",
  vpsCommonPreviewTitle: "Review changes",
  vpsCommonPendingChanges: "{n} unapplied changes",
  vpsCommonRollbackNow: "Roll back now",
  vpsCommonConfirm: "Confirm",
  vpsCommonCancel: "Cancel",
  vpsCommonReload: "Reload",
  vpsCommonFix: "Fix",
  vpsCommonPass: "OK",
  vpsCommonFail: "Needs attention",
  vpsCommonLoading: "Loading…",
  vpsCommonReadOnly: "Read only",
  vpsErrSudoRequired: "A sudo password is needed for this.",
  vpsErrSudoDenied: "The sudo password is incorrect.",
  vpsErrLockoutBlocked: "Blocked so you don't lock yourself out:",
  vpsErrCommandFailed: "The command failed on the server:",
  vpsErrVerifyFailed: "A new connection could not be opened, so the change was rolled back:",
  vpsErrUnsupported: "Not supported on this server:",
  vpsLockoutReasons: {
    both_auth_off: "Password and SSH key login cannot both be turned off.",
    password_in_use:
      "The app signs in with a password. Switch this host to SSH key login before turning passwords off.",
    pubkey_in_use: "The app signs in with an SSH key, so key login cannot be turned off.",
    root_in_use: "The app signs in as root. Use another user with sudo before blocking root.",
    root_password_in_use: "The app signs in as root with a password. Switch to an SSH key first.",
    allow_users_excludes_me: "The new AllowUsers list leaves out the user the app signs in as.",
    deny_users_matches_me: "The new DenyUsers list blocks the user the app signs in as.",
    only_ssh_rule: "This is the only rule that lets the app reach the SSH port.",
    closes_ssh_for_everyone: "This rule closes the SSH port for everyone, including the app.",
    source_includes_me: "This rule's source includes the IP the app connects from.",
    login_user: "This is the user the app signs in as.",
    last_sudo_user: "This is the last user with sudo.",
    key_in_use: "This is the SSH key the app signs in with.",
  },
  vpsUnsupportedReasons: {
    linux_only: "Linux servers only",
    no_ssh_connection: "Could not read the SSH connection details ($SSH_CONNECTION) on the server.",
    no_systemd: "This server does not use systemd, so settings are read only.",
    firewall_read_only: "This firewall is read only. Install ufw to edit rules.",
  },
  vpsShellSelectHost: "Select host",
  vpsShellNoHosts: "No hosts yet. Add one under Machines.",
  vpsShellProbing: "Checking the server…",
  vpsTabSsh: "SSH",
  vpsTabFirewall: "Firewall",
  vpsTabAccess: "Users & Keys",
  vpsPrivRoot: "Root",
  vpsPrivSudoNoPassword: "Sudo, no password",
  vpsPrivSudoPassword: "Sudo unlocked",
  vpsPrivNone: "No sudo yet",
  vpsSudoTitle: "Sudo password",
  vpsSudoHint: "Kept in memory for this session only, never saved.",
  vpsSudoOpen: "Enter sudo password",
  vpsSudoUnlock: "Unlock",
  vpsStudioPending: "This section is being rebuilt.",
  vpsSshStatusTitle: "Status",
  vpsSshCheckPassword: "Password login is off",
  vpsSshCheckRoot: "Root cannot sign in with a password",
  vpsSshCheckMaxAuth: "Login attempts are limited (4 or fewer)",
  vpsSshGroupLogin: "Login",
  vpsSshGroupWho: "Who can sign in",
  vpsSshGroupLimits: "Session limits",
  vpsSshPort: "SSH port",
  vpsSshPortHint: "The new port is tested with a new connection before the app saves it.",
  vpsSshPassword: "Password login",
  vpsSshPubkey: "SSH key login",
  vpsSshRoot: "Root login",
  vpsSshRootYes: "Allowed",
  vpsSshRootKeyOnly: "SSH key only",
  vpsSshRootForced: "Forced commands only",
  vpsSshRootNo: "Not allowed",
  vpsSshAllowUsers: "Only these users",
  vpsSshDenyUsers: "Block these users",
  vpsSshUsersHint: "Separate with spaces. Leave empty for no limit.",
  vpsSshMaxAuthTries: "Max login attempts",
  vpsSshMaxSessions: "Max sessions per connection",
  vpsSshLoginGrace: "Login grace time (seconds)",
  vpsSshAliveInterval: "Keep-alive every (seconds, 0 = off)",
  vpsSshAliveCount: "Missed keep-alives before disconnect",
  vpsSshWriteDropIn: "Changes are written to /etc/ssh/sshd_config.d/00-sshbox.conf",
  vpsSshWriteMain: "Changes are written to /etc/ssh/sshd_config",
  vpsSshPending: "A previous change is still waiting to be verified or rolled back.",
  vpsSshProgressTitle: "Applying SSH changes",
  vpsSshRollbackIn: "Rolls back in {s}s unless verified",
  vpsSshDone: "Applied and verified with a new connection.",
  vpsSshPortSaved: "The app saved the new port {p} for this host.",
  vpsSshRolledBack: "Rolled back to the previous configuration.",
  vpsSshClose: "Close",
  vpsSshInvalid: "Some values are invalid.",
  vpsSshSteps: ["Check lockout guard", "Back up", "Write config", "Check syntax (sshd -t)", "Arm 60-second rollback", "Reload SSH", "Check port", "Test a new connection", "Keep the change"],
  vpsFwStatusTitle: "Status",
  vpsFwEnabled: "Firewall on",
  vpsFwEngine: "Firewall tool",
  vpsFwIpv6Off: "IPv6 traffic is not filtered. Set IPV6=yes in /etc/default/ufw to filter it.",
  vpsFwNoEngine: "No firewall is installed on this server. Install ufw to use this section.",
  vpsFwPortsTitle: "Open ports",
  vpsFwNoPorts: "No ports are listening.",
  vpsFwRulesTitle: "Rules",
  vpsFwNoRules: "No rules yet.",
  vpsFwFrom: "from {s}",
  vpsFwAnywhere: "anywhere",
  vpsFwAllPorts: "all ports",
  vpsFwDelete: "Delete",
  vpsFwDeleteConfirm: "Delete rule",
  vpsFwAddTitle: "Add a rule",
  vpsFwAction: "Action",
  vpsFwSource: "Source (IP or range, empty = anywhere)",
  vpsFwMyIp: "My IP",
  vpsFwPort: "Port (empty = all ports)",
  vpsFwProto: "Protocol",
  vpsFwAny: "Any",
  vpsFwAdd: "Add",
  vpsFwOn: "on",
  vpsFwOff: "off",
  vpsFwExposure: { exposed: "Public", restricted: "Limited to some IPs", blocked: "Blocked", loopback: "Local only" },
  vpsFwActions: { allow: "Allow", limit: "Allow, rate-limited", deny: "Deny", reject: "Reject" },
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
