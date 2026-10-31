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
  vpsCommandReasons: Record<string, string>;
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
  vpsSshWriteWindows: string;
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
  vpsFwPortOnly: string;
  vpsFwProto: string;
  vpsFwAny: string;
  vpsFwAdd: string;
  vpsFwOn: string;
  vpsFwOff: string;
  vpsFwExposure: Record<string, string>;
  vpsFwActions: Record<string, string>;
  vpsUsersCreateTitle: string;
  vpsUsersName: string;
  vpsUsersInvalidName: string;
  vpsUsersSudo: string;
  vpsUsersPassword: string;
  vpsUsersCreate: string;
  vpsUsersLocked: string;
  vpsUsersLock: string;
  vpsUsersLockHint: string;
  vpsUsersLockConfirm: string;
  vpsUsersUnlock: string;
  vpsUsersKeysTitle: string;
  vpsUsersNoKeys: string;
  vpsUsersRevoke: string;
  vpsUsersRevokeConfirm: string;
  vpsUsersKeyInUse: string;
  vpsUsersGrant: string;
  vpsUsersGrantBtn: string;
  vpsUsersNoLocalKeys: string;
  vpsUsersRootReadOnly: string;
  vpsUsersSudoModes: Record<string, string>;
  vpsShellSelectHost: string;
  vpsShellNoHosts: string;
  vpsShellProbing: string;
  vpsTabSsh: string;
  vpsTabFirewall: string;
  vpsTabAccess: string;
  vpsTabInstaller: string;
  vpsErrTimeout: string;
  instTabMissing: string;
  instTabInstalled: string;
  instTabAttention: string;
  instCat: Record<string, string>;
  instMgrDesc: Record<string, string>;
  instRecommended: string;
  instInstall: string;
  instBrewAdminNote: string;
  instUninstall: string;
  instEnable: string;
  instInstallSelected: (n: number) => string;
  instNeedsManager: (m: string) => string;
  instUninstallConfirm: (name: string, m: string) => string;
  instExternal: string;
  instNotRunning: string;
  instSearchPlaceholder: string;
  instSearchSkipped: (list: string) => string;
  instStatus: Record<"waiting" | "running" | "done" | "failed", string>;
  instWindowsPopupNote: string;
  instWingetManual: string;
  instRefresh: string;
  vpsPrivRoot: string;
  vpsPrivSudoNoPassword: string;
  vpsPrivSudoPassword: string;
  vpsPrivAdmin: string;
  vpsPrivNone: string;
  vpsPrivStandard: string;
  vpsSudoTitle: string;
  vpsSudoHint: string;
  vpsSudoLocalLabel: string;
  vpsSudoOpen: string;
  vpsSudoUnlock: string;
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
  vpsCommandReasons: {
    no_answer: "Máy chủ không trả lời trong 120 giây. Nếu lúc đó đang áp dụng thay đổi cài đặt SSH, thay đổi ấy sẽ tự hoàn tác trong vòng 60 giây.",
  },
  vpsLockoutReasons: {
    both_auth_off: "Không thể tắt cả đăng nhập bằng mật khẩu lẫn bằng SSH key.",
    no_sessions: "MaxSessions 0 từ chối mọi phiên làm việc, kể cả của ứng dụng.",
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
    windows_block_covers_me:
      "Đang có rule chặn chính kết nối SSH của bạn. Bật firewall lúc này sẽ cắt kết nối, nên ứng dụng không bật.",
    windows_inbound_allows_off:
      "Firewall đang đặt chế độ chặn mọi kết nối vào, kể cả ứng dụng được phép. Bật firewall lúc này sẽ cắt SSH, nên ứng dụng không bật.",
  },
  vpsUnsupportedReasons: {
    linux_only: "Cấu hình VPS không dùng cho máy local",
    unsupported_os: "Hệ điều hành của máy này chưa được hỗ trợ",
    windows_not_admin: "Tài khoản này không có quyền quản trị nên chỉ xem được.",
    windows_no_sshd: "Dịch vụ OpenSSH Server (sshd) không chạy nên chỉ xem được.",
    windows_ssh_read_only: "Trên Windows, cấu hình SSH hiện chỉ xem được.",
    windows_no_scheduled_tasks: "Máy này thiếu module ScheduledTasks nên cấu hình SSH chỉ xem được.",
    windows_openssh_old: "Bản OpenSSH trên máy này cũ hơn 8.7 nên cấu hình SSH chỉ xem được.",
    macos_no_dropin: "Bản macOS này không có thư mục sshd_config.d nên chỉ xem được.",
    read_only_os: "Trên hệ điều hành này, mục này chỉ xem được.",
    hidden_setting: "Cài đặt này không sửa được trên hệ điều hành này.",
    windows_rule_not_managed: "Chỉ xoá được rule do SSH Box tạo.",
    no_ssh_connection: "Không đọc được thông tin kết nối SSH ($SSH_CONNECTION) trên máy chủ.",
    no_systemd: "Máy chủ không dùng systemd nên chỉ xem được, không sửa được.",
    firewall_read_only: "Firewall này chỉ xem được. Cài ufw để sửa rule.",
    windows_port_required: "Trên Windows cần nhập cổng cụ thể cho rule.",
    windows_action_unsupported: "Windows Firewall không hỗ trợ hành động này.",
    set_in_main_file:
      "Port, AllowUsers hoặc DenyUsers đang được đặt trong /etc/ssh/sshd_config. Hãy xoá dòng đó trong file chính rồi thử lại, nếu không giá trị cũ vẫn còn hiệu lực.",
    multiple_ports: "SSH đang nghe trên nhiều cổng. Hãy để lại một cổng trước khi đổi cổng từ đây.",
  },
  vpsShellSelectHost: "Chọn máy",
  vpsShellNoHosts: "Chưa có máy chủ nào. Hãy thêm máy trong mục Máy chủ.",
  vpsShellProbing: "Đang kiểm tra máy chủ…",
  vpsTabSsh: "SSH",
  vpsTabFirewall: "Tường lửa",
  vpsTabAccess: "Người dùng & khoá SSH",
  vpsTabInstaller: "Cài công cụ",
  vpsErrTimeout: "Máy chủ phản hồi quá lâu nên đã dừng:",
  instTabMissing: "Chưa cài",
  instTabInstalled: "Đã cài",
  instTabAttention: "Cần chú ý",
  instCat: { managers: "Trình quản lý gói", security: "Bảo mật", apps: "Chạy ứng dụng", ai: "AI", network: "Mạng", utils: "Tiện ích" },
  instMgrDesc: {
    apt: "Trình cài phần mềm có sẵn trên Ubuntu và Debian.",
    dnf: "Trình cài phần mềm có sẵn trên Fedora, RHEL, Rocky.",
    snap: "Kho ứng dụng của Canonical, một gói chạy được trên nhiều bản Linux.",
    npm: "Cài thư viện và công cụ JavaScript; đi kèm Node.js.",
    brew: "Homebrew: trình cài công cụ dòng lệnh phổ biến nhất trên macOS.",
    winget: "Trình cài ứng dụng chính thức của Windows.",
  },
  instRecommended: "nên cài",
  instInstall: "Cài",
  instBrewAdminNote: "Cài Homebrew cần quyền quản trị; nếu lỗi, hãy chạy lệnh cài chính thức trong Terminal",
  instUninstall: "Gỡ",
  instEnable: "Bật",
  instInstallSelected: (n) => `Cài ${n} mục đã chọn`,
  instNeedsManager: (m) => `Cần cài ${m} trước`,
  instUninstallConfirm: (name, m) => `Gỡ ${name} bằng ${m}?`,
  instExternal: "Đã cài (ngoài trình quản lý gói)",
  instNotRunning: "Đã cài · chưa bật",
  instSearchPlaceholder: "Tìm gói trong …",
  instSearchSkipped: (list) => `Bỏ qua (chưa có trình quản lý): ${list}`,
  instStatus: { waiting: "Đang chờ", running: "Đang chạy", done: "Xong", failed: "Lỗi" },
  instWindowsPopupNote: "Windows có thể hiện hộp thoại xin quyền quản trị; hãy bấm đồng ý trên máy chủ.",
  instRefresh: "Cập nhật danh sách gói",
  instWingetManual: "winget không cài được từ xa. Hãy cài \"App Installer\" từ Microsoft Store trên máy chủ rồi quét lại.",
  vpsPrivRoot: "Root",
  vpsPrivSudoNoPassword: "Sudo không cần mật khẩu",
  vpsPrivSudoPassword: "Sudo đã mở khoá",
  vpsPrivAdmin: "Quản trị viên",
  vpsPrivNone: "Chưa có quyền sudo",
  vpsPrivStandard: "Tài khoản thường",
  vpsSudoTitle: "Mật khẩu sudo",
  vpsSudoLocalLabel: "Máy này",
  vpsSudoHint: "Chỉ giữ trong bộ nhớ cho phiên làm việc này, không lưu lại.",
  vpsSudoOpen: "Nhập mật khẩu sudo",
  vpsSudoUnlock: "Mở khoá",
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
  vpsSshWriteWindows: "Thay đổi được ghi vào C:\\ProgramData\\ssh\\sshd_config",
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
  vpsFwPortOnly: "Cổng",
  vpsFwProto: "Giao thức",
  vpsFwAny: "Tất cả",
  vpsFwAdd: "Thêm",
  vpsFwOn: "bật",
  vpsFwOff: "tắt",
  vpsFwExposure: { exposed: "Mở công khai", restricted: "Giới hạn IP", blocked: "Bị chặn", loopback: "Chỉ nội bộ", unknown: "Không xác định" },
  vpsFwActions: { allow: "Cho phép", limit: "Cho phép, giới hạn tần suất", deny: "Chặn", reject: "Từ chối" },
  vpsUsersCreateTitle: "Tạo user mới",
  vpsUsersName: "Tên user",
  vpsUsersInvalidName: "Tên user chỉ gồm chữ thường, số, _ và -, bắt đầu bằng chữ.",
  vpsUsersSudo: "Quyền sudo",
  vpsUsersPassword: "Mật khẩu của user (dùng khi gõ sudo)",
  vpsUsersCreate: "Tạo",
  vpsUsersLocked: "Đã khoá",
  vpsUsersLock: "Khoá đăng nhập",
  vpsUsersLockHint: "Khoá cả đăng nhập bằng mật khẩu lẫn bằng SSH key.",
  vpsUsersLockConfirm: "Khoá user này",
  vpsUsersUnlock: "Mở khoá",
  vpsUsersKeysTitle: "SSH key được phép",
  vpsUsersNoKeys: "Chưa có key nào.",
  vpsUsersRevoke: "Thu hồi",
  vpsUsersRevokeConfirm: "Thu hồi key",
  vpsUsersKeyInUse: "Ứng dụng đang dùng",
  vpsUsersGrant: "Cấp key từ kho của ssh-box",
  vpsUsersGrantBtn: "Cấp",
  vpsUsersNoLocalKeys: "Kho key của ssh-box đang trống.",
  vpsUsersRootReadOnly: "Tài khoản root chỉ xem được ở đây.",
  vpsUsersSudoModes: { none: "Không có sudo", noPassword: "Sudo không cần mật khẩu", withPassword: "Sudo có mật khẩu" },
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
  vpsCommandReasons: {
    no_answer: "The server did not answer within 120 seconds. If an SSH settings change was being applied, it rolls back by itself within 60 seconds.",
  },
  vpsLockoutReasons: {
    both_auth_off: "Password and SSH key login cannot both be turned off.",
    no_sessions: "MaxSessions 0 refuses every session, including the app's.",
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
    windows_block_covers_me:
      "A rule blocks your own SSH connection. Turning the firewall on now would cut you off, so the app will not do it.",
    windows_inbound_allows_off:
      "The firewall is set to block all incoming connections, even allowed apps. Turning it on now would cut SSH, so the app will not do it.",
  },
  vpsUnsupportedReasons: {
    linux_only: "Server Config is not available for the local machine",
    unsupported_os: "This machine's operating system is not supported",
    windows_not_admin: "This account is not an administrator, so settings are read only.",
    windows_no_sshd: "The OpenSSH Server service (sshd) is not running, so settings are read only.",
    windows_ssh_read_only: "On Windows, SSH settings are read only for now.",
    windows_no_scheduled_tasks: "This machine lacks the ScheduledTasks module, so SSH settings are read only.",
    windows_openssh_old: "OpenSSH on this machine is older than 8.7, so SSH settings are read only.",
    macos_no_dropin: "This macOS version has no sshd_config.d folder, so settings are read only.",
    read_only_os: "On this operating system, this section is read only.",
    hidden_setting: "This setting cannot be changed on this operating system.",
    windows_rule_not_managed: "Only rules created by SSH Box can be deleted.",
    no_ssh_connection: "Could not read the SSH connection details ($SSH_CONNECTION) on the server.",
    no_systemd: "This server does not use systemd, so settings are read only.",
    firewall_read_only: "This firewall is read only. Install ufw to edit rules.",
    windows_port_required: "On Windows, a rule needs a specific port.",
    windows_action_unsupported: "Windows Firewall does not support this action.",
    set_in_main_file:
      "Port, AllowUsers or DenyUsers is set in /etc/ssh/sshd_config. Remove that line from the main file and try again, or the old value stays in effect.",
    multiple_ports: "SSH listens on more than one port. Keep a single port before changing it here.",
  },
  vpsShellSelectHost: "Select host",
  vpsShellNoHosts: "No hosts yet. Add one under Machines.",
  vpsShellProbing: "Checking the server…",
  vpsTabSsh: "SSH",
  vpsTabFirewall: "Firewall",
  vpsTabAccess: "Users & Keys",
  vpsTabInstaller: "Tools",
  vpsErrTimeout: "The server took too long and the task was stopped:",
  instTabMissing: "Not installed",
  instTabInstalled: "Installed",
  instTabAttention: "Needs attention",
  instCat: { managers: "Package managers", security: "Security", apps: "Run apps", ai: "AI", network: "Network", utils: "Utilities" },
  instMgrDesc: {
    apt: "Built-in package installer on Ubuntu and Debian.",
    dnf: "Built-in package installer on Fedora, RHEL and Rocky.",
    snap: "Canonical's app store; one package runs on many Linux distros.",
    npm: "Installs JavaScript libraries and tools; ships with Node.js.",
    brew: "Homebrew: the most popular command-line package installer on macOS.",
    winget: "The official Windows app installer.",
  },
  instRecommended: "recommended",
  instInstall: "Install",
  instBrewAdminNote: "Installing Homebrew needs admin rights; if it fails, run the official install command in Terminal",
  instUninstall: "Remove",
  instEnable: "Enable",
  instInstallSelected: (n) => `Install ${n} selected`,
  instNeedsManager: (m) => `Install ${m} first`,
  instUninstallConfirm: (name, m) => `Remove ${name} using ${m}?`,
  instExternal: "Installed (outside a package manager)",
  instNotRunning: "Installed · not enabled",
  instSearchPlaceholder: "Search packages in …",
  instSearchSkipped: (list) => `Skipped (no package manager): ${list}`,
  instStatus: { waiting: "Waiting", running: "Running", done: "Done", failed: "Failed" },
  instWindowsPopupNote: "Windows may show an administrator prompt; approve it on the server.",
  instRefresh: "Refresh package lists",
  instWingetManual: "winget cannot be installed remotely. Install \"App Installer\" from the Microsoft Store on the server, then rescan.",
  vpsPrivRoot: "Root",
  vpsPrivSudoNoPassword: "Sudo, no password",
  vpsPrivSudoPassword: "Sudo unlocked",
  vpsPrivAdmin: "Administrator",
  vpsPrivNone: "No sudo yet",
  vpsPrivStandard: "Standard account",
  vpsSudoTitle: "Sudo password",
  vpsSudoLocalLabel: "This machine",
  vpsSudoHint: "Kept in memory for this session only, never saved.",
  vpsSudoOpen: "Enter sudo password",
  vpsSudoUnlock: "Unlock",
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
  vpsSshWriteWindows: "Changes are written to C:\\ProgramData\\ssh\\sshd_config",
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
  vpsFwPortOnly: "Port",
  vpsFwProto: "Protocol",
  vpsFwAny: "Any",
  vpsFwAdd: "Add",
  vpsFwOn: "on",
  vpsFwOff: "off",
  vpsFwExposure: { exposed: "Public", restricted: "Limited to some IPs", blocked: "Blocked", loopback: "Local only", unknown: "Unknown" },
  vpsFwActions: { allow: "Allow", limit: "Allow, rate-limited", deny: "Deny", reject: "Reject" },
  vpsUsersCreateTitle: "Create a user",
  vpsUsersName: "Username",
  vpsUsersInvalidName: "Use lowercase letters, digits, _ and -, starting with a letter.",
  vpsUsersSudo: "Sudo access",
  vpsUsersPassword: "The user's password (asked by sudo)",
  vpsUsersCreate: "Create",
  vpsUsersLocked: "Locked",
  vpsUsersLock: "Lock login",
  vpsUsersLockHint: "Blocks both password and SSH key login.",
  vpsUsersLockConfirm: "Lock this user",
  vpsUsersUnlock: "Unlock",
  vpsUsersKeysTitle: "Authorized SSH keys",
  vpsUsersNoKeys: "No keys yet.",
  vpsUsersRevoke: "Revoke",
  vpsUsersRevokeConfirm: "Revoke key",
  vpsUsersKeyInUse: "Used by this app",
  vpsUsersGrant: "Grant a key from ssh-box",
  vpsUsersGrantBtn: "Grant",
  vpsUsersNoLocalKeys: "ssh-box has no saved keys.",
  vpsUsersRootReadOnly: "The root account is read only here.",
  vpsUsersSudoModes: { none: "No sudo", noPassword: "Sudo without password", withPassword: "Sudo with password" },
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
