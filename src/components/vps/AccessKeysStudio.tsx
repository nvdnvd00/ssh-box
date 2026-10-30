import React, { useCallback, useEffect, useId, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CheckCircle2,
  Crown,
  FolderGit2,
  KeyRound,
  Link2,
  Lock,
  Plus,
  RefreshCw,
  Search,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Terminal,
  Trash2,
  Unlock,
  UserCheck,
  UserPlus,
  Users,
  X,
} from "lucide-react";
import {
  AccessKeysSnapshot,
  HostEntry,
  LinuxAccountItem,
  SshKeyItem,
  VpsPrivilegeInfo,
} from "../../types";
import { AppLanguage } from "../../themes";
import { SelectField } from "../SelectField";

interface AccessKeysStudioProps {
  host: HostEntry;
  sshKeys: SshKeyItem[];
  sudoPassword?: string;
  privilegeInfo: VpsPrivilegeInfo | null;
  language?: AppLanguage;
  onRequireSudoUnlock?: () => void;
}

function extractPublicKeyBody(rawPublicKey: string): string {
  const trimmed = rawPublicKey.trim();
  if (!trimmed) return "";
  const tokens = trimmed.split(/\s+/);
  const typeIdx = tokens.findIndex(
    (t) =>
      t.startsWith("ssh-") ||
      t.startsWith("ecdsa-sha2-") ||
      t.startsWith("sk-ssh-") ||
      t.startsWith("sk-ecdsa-"),
  );
  if (typeIdx >= 0 && tokens[typeIdx + 1]) {
    return tokens[typeIdx + 1].trim();
  }
  return tokens[1]?.trim() || tokens[0]?.trim() || "";
}

function formatKeyShortPreview(body: string): string {
  const clean = body.trim();
  if (clean.length > 18) {
    return `${clean.slice(0, 10)}...${clean.slice(-6)}`;
  }
  return clean;
}

function getKeyAlgorithmBadge(keyType: string): {
  label: string;
  className: string;
} {
  const lower = keyType.toLowerCase();
  if (lower.includes("ed25519")) {
    return {
      label: "ED25519",
      className:
        "bg-emerald-500/15 text-emerald-400 border-emerald-500/35",
    };
  }
  if (lower.includes("rsa")) {
    return {
      label: "RSA",
      className: "bg-amber-500/15 text-amber-400 border-amber-500/35",
    };
  }
  if (lower.includes("ecdsa")) {
    return {
      label: "ECDSA",
      className: "bg-cyan-500/15 text-cyan-400 border-cyan-500/35",
    };
  }
  return {
    label: keyType.toUpperCase(),
    className: "bg-violet-500/15 text-violet-300 border-violet-500/35",
  };
}

export const AccessKeysStudio: React.FC<AccessKeysStudioProps> = ({
  host,
  sshKeys,
  sudoPassword,
  language = "vi",
  onRequireSudoUnlock,
}) => {
  const isVi = language === "vi";
  const formUid = useId();

  const [snapshot, setSnapshot] = useState<AccessKeysSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  // Left Column: Account Deck search & selection
  const [accountSearch, setAccountSearch] = useState("");
  const [selectedUsername, setSelectedUsername] = useState<string>("");

  // Left Column: "+ Create Linux Account" interactive card/drawer
  const [createUserOpen, setCreateUserOpen] = useState(false);
  const [newUsername, setNewUsername] = useState("");
  const [newShell, setNewShell] = useState("/bin/bash");
  const [newSudoAccess, setNewSudoAccess] = useState(true);
  const [newNopasswdSudo, setNewNopasswdSudo] = useState(false);
  const [newDockerAccess, setNewDockerAccess] = useState(false);
  const [newInitialKeyId, setNewInitialKeyId] = useState<string>("");

  // Right Column Panel A: "+ Paste External Public Key" expandable bar
  const [pasteKeyOpen, setPasteKeyOpen] = useState(false);
  const [externalPublicKey, setExternalPublicKey] = useState("");

  const fetchSnapshot = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const res = await invoke<AccessKeysSnapshot>(
        "vps_get_access_keys_snapshot",
        {
          hostId: host.id,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(res);
      setSelectedUsername((prev) => {
        if (prev && res.accounts.some((a) => a.username === prev)) {
          return prev;
        }
        const hostUserMatch = res.accounts.find(
          (a) => a.username === host.user,
        );
        return hostUserMatch?.username ?? res.accounts[0]?.username ?? "";
      });
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setLoading(false);
    }
  }, [host.id, host.user, sudoPassword, onRequireSudoUnlock]);

  useEffect(() => {
    fetchSnapshot();
  }, [fetchSnapshot]);

  useEffect(() => {
    if (!toastMsg) return;
    const timer = setTimeout(() => setToastMsg(null), 4500);
    return () => clearTimeout(timer);
  }, [toastMsg]);

  const accounts = snapshot?.accounts ?? [];

  const filteredAccounts = useMemo(() => {
    const q = accountSearch.trim().toLowerCase();
    if (!q) return accounts;
    return accounts.filter(
      (acc) =>
        acc.username.toLowerCase().includes(q) ||
        acc.shell.toLowerCase().includes(q) ||
        acc.homeDir.toLowerCase().includes(q) ||
        acc.groups.some((g) => g.toLowerCase().includes(q)) ||
        String(acc.uid).includes(q),
    );
  }, [accounts, accountSearch]);

  const selectedAccount: LinuxAccountItem | null = useMemo(() => {
    if (accounts.length === 0) return null;
    return (
      accounts.find((a) => a.username === selectedUsername) ??
      accounts[0] ??
      null
    );
  }, [accounts, selectedUsername]);

  // Map local ssh-box keys by their extracted base64 body for O(1) cross-referencing
  const localKeyByBody = useMemo(() => {
    const map = new Map<string, SshKeyItem>();
    for (const k of sshKeys) {
      const body = extractPublicKeyBody(k.publicKey);
      if (body) {
        map.set(body, k);
      }
    }
    return map;
  }, [sshKeys]);

  // Set of base64 key bodies currently authorized on selectedAccount
  const selectedAccountKeyBodies = useMemo(() => {
    const set = new Set<string>();
    if (!selectedAccount) return set;
    for (const rk of selectedAccount.authorizedKeys) {
      if (rk.publicKeyBody) {
        set.add(rk.publicKeyBody.trim());
      }
    }
    return set;
  }, [selectedAccount]);

  const totalAuthorizedKeysCount = useMemo(
    () => accounts.reduce((sum, a) => sum + a.authorizedKeys.length, 0),
    [accounts],
  );

  const sudoersAccountsCount = useMemo(
    () => accounts.filter((a) => a.hasSudo || a.uid === 0).length,
    [accounts],
  );

  const handleAuthorizeKey = async (
    username: string,
    publicKey: string,
    labelForToast?: string,
  ) => {
    const cleanKey = publicKey.trim();
    if (!cleanKey) return;
    const body = extractPublicKeyBody(cleanKey);
    setBusyAction(`auth-${username}-${body.slice(-8)}`);
    setErrorMsg(null);
    try {
      const updated = await invoke<AccessKeysSnapshot>(
        "vps_authorize_ssh_key",
        {
          hostId: host.id,
          username,
          publicKey: cleanKey,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(updated);
      setToastMsg(
        isVi
          ? `Đã cấp quyền khóa SSH ${labelForToast ? `"${labelForToast}" ` : ""}cho tài khoản ${username} (chmod 700/600)!`
          : `Authorized SSH key ${labelForToast ? `"${labelForToast}" ` : ""}for ${username} (chmod 700/600)!`,
      );
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handlePasteExternalKeySubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedAccount) return;
    const clean = externalPublicKey.trim();
    if (!clean) return;
    await handleAuthorizeKey(selectedAccount.username, clean);
    setExternalPublicKey("");
    setPasteKeyOpen(false);
  };

  const handleRevokeKey = async (
    username: string,
    keyBody: string,
    comment: string,
  ) => {
    if (!keyBody) return;
    setBusyAction(`revoke-${username}-${keyBody.slice(-8)}`);
    setErrorMsg(null);
    try {
      const updated = await invoke<AccessKeysSnapshot>("vps_revoke_ssh_key", {
        hostId: host.id,
        username,
        keyBody,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(updated);
      setToastMsg(
        isVi
          ? `Đã thu hồi khóa SSH ${comment ? `(${comment}) ` : ""}khỏi ${username}/.ssh/authorized_keys.`
          : `Revoked SSH key ${comment ? `(${comment}) ` : ""}from ${username}/.ssh/authorized_keys.`,
      );
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handleCreateLinuxUser = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanUser = newUsername.trim();
    if (!cleanUser) return;

    const chosenLocalKey = sshKeys.find((k) => k.id === newInitialKeyId);
    const initialPublicKey = chosenLocalKey?.publicKey?.trim() || null;

    setBusyAction("create-user");
    setErrorMsg(null);
    try {
      const updated = await invoke<AccessKeysSnapshot>(
        "vps_create_linux_user",
        {
          hostId: host.id,
          username: cleanUser,
          shell: newShell,
          sudoAccess: newSudoAccess || newNopasswdSudo,
          nopasswdSudo: newNopasswdSudo,
          dockerAccess: newDockerAccess,
          initialPublicKey,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(updated);
      setSelectedUsername(cleanUser);
      setCreateUserOpen(false);
      setNewUsername("");
      setNewShell("/bin/bash");
      setNewSudoAccess(true);
      setNewNopasswdSudo(false);
      setNewDockerAccess(false);
      setNewInitialKeyId("");
      setToastMsg(
        isVi
          ? `Đã tạo tài khoản Linux "${cleanUser}"${chosenLocalKey ? ` và gắn khóa "${chosenLocalKey.name}"` : ""} thành công!`
          : `Created Linux account "${cleanUser}"${chosenLocalKey ? ` with key "${chosenLocalKey.name}"` : ""} successfully!`,
      );
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handleUpdatePrivilege = async (
    account: LinuxAccountItem,
    nextSudo: boolean,
    nextNopasswd: boolean,
    nextLocked: boolean,
  ) => {
    if (account.uid === 0) return;
    const effectiveSudo = nextNopasswd ? true : nextSudo;
    const effectiveNopasswd = effectiveSudo ? nextNopasswd : false;

    setBusyAction(`priv-${account.username}`);
    setErrorMsg(null);
    try {
      const updated = await invoke<AccessKeysSnapshot>(
        "vps_update_linux_user_privilege",
        {
          hostId: host.id,
          username: account.username,
          sudoAccess: effectiveSudo,
          nopasswdSudo: effectiveNopasswd,
          locked: nextLocked,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(updated);
      setToastMsg(
        isVi
          ? `Đã cập nhật đặc quyền Sudo & trạng thái khóa cho tài khoản "${account.username}".`
          : `Updated sudoers & lock posture for account "${account.username}".`,
      );
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const shellOptions = useMemo(
    () => [
      { value: "/bin/bash", label: "/bin/bash (Bash Login Shell)" },
      { value: "/bin/zsh", label: "/bin/zsh (Zsh Login Shell)" },
      { value: "/bin/sh", label: "/bin/sh (POSIX Shell)" },
      {
        value: "/usr/sbin/nologin",
        label: "/usr/sbin/nologin (Service / No Shell)",
      },
    ],
    [],
  );

  const initialKeyOptions = useMemo(
    () => [
      {
        value: "",
        label: isVi
          ? "— Không gắn sẵn khóa SSH khi tạo —"
          : "— Do not inject an SSH key yet —",
      },
      ...sshKeys.map((k) => ({
        value: k.id,
        label: `${k.name} (${k.keyType.toUpperCase()} · ${k.fingerprint || formatKeyShortPreview(extractPublicKeyBody(k.publicKey))})`,
      })),
    ],
    [sshKeys, isVi],
  );

  if (loading && !snapshot) {
    return (
      <div className="p-8 flex flex-col items-center justify-center gap-3 text-muted-foreground">
        <RefreshCw className="w-6 h-6 animate-spin text-primary" />
        <p className="text-xs font-medium">
          {isVi
            ? "Đang kiểm tra danh sách Linux Users, Sudoers và ~/.ssh/authorized_keys..."
            : "Inspecting Linux accounts, sudoers drop-ins, and ~/.ssh/authorized_keys..."}
        </p>
      </div>
    );
  }

  return (
    <div className="space-y-5">
      {/* Error / Toast Banners */}
      {errorMsg && (
        <div className="rounded-2xl border border-rose-500/40 bg-rose-500/10 px-4 py-3 flex items-start justify-between gap-3 text-xs text-rose-200 ui-banner-slide">
          <div className="flex items-start gap-2.5">
            <ShieldAlert className="w-4 h-4 text-rose-400 shrink-0 mt-0.5" />
            <div className="space-y-1">
              <div className="font-semibold text-rose-300">
                {isVi
                  ? "Không thể thực thi tác vụ Quản lý Tài khoản & Khóa SSH"
                  : "Linux Account & Keyring Operation Failed"}
              </div>
              <pre className="font-mono text-[11px] whitespace-pre-wrap break-all text-rose-200/90">
                {errorMsg}
              </pre>
            </div>
          </div>
          <div className="flex items-center gap-2 shrink-0">
            {onRequireSudoUnlock && (
              <button
                type="button"
                onClick={onRequireSudoUnlock}
                className="px-2.5 py-1 rounded-lg bg-amber-500/20 hover:bg-amber-500/30 border border-amber-500/40 text-amber-300 font-semibold cursor-pointer"
              >
                {isVi ? "Mở khóa Sudo" : "Unlock Sudo"}
              </button>
            )}
            <button
              type="button"
              onClick={() => setErrorMsg(null)}
              className="p-1 rounded-lg hover:bg-rose-500/20 text-rose-300 cursor-pointer"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>
      )}

      {toastMsg && (
        <div className="rounded-2xl border border-emerald-500/40 bg-emerald-500/10 px-4 py-3 flex items-center justify-between gap-3 text-xs text-emerald-200 ui-banner-slide">
          <div className="flex items-center gap-2.5">
            <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
            <span className="font-medium">{toastMsg}</span>
          </div>
          <button
            type="button"
            onClick={() => setToastMsg(null)}
            className="p-1 rounded-lg hover:bg-emerald-500/20 text-emerald-300 cursor-pointer"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}

      {/* Top Posture Summary Bar */}
      <div className="grid grid-cols-1 sm:grid-cols-3 gap-3.5">
        <div className="rounded-2xl border border-border/80 bg-card/75 px-4 py-3.5 flex items-center justify-between gap-3">
          <div>
            <div className="text-[11px] text-muted-foreground font-medium">
              {isVi ? "Tài khoản Linux (UID 0 & ≥1000)" : "Linux Accounts (UID 0 & ≥1000)"}
            </div>
            <div className="text-lg font-bold font-mono text-foreground mt-0.5">
              {accounts.length}
            </div>
          </div>
          <div className="w-9 h-9 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
            <Users className="w-4 h-4" />
          </div>
        </div>

        <div className="rounded-2xl border border-border/80 bg-card/75 px-4 py-3.5 flex items-center justify-between gap-3">
          <div>
            <div className="text-[11px] text-muted-foreground font-medium">
              {isVi ? "Tài khoản có quyền Sudo / Root" : "Privileged Sudo / Root Accounts"}
            </div>
            <div className="text-lg font-bold font-mono text-violet-300 mt-0.5">
              {sudoersAccountsCount}
            </div>
          </div>
          <div className="w-9 h-9 rounded-xl bg-violet-500/15 border border-violet-500/30 flex items-center justify-center text-violet-400">
            <ShieldCheck className="w-4 h-4" />
          </div>
        </div>

        <div className="rounded-2xl border border-border/80 bg-card/75 px-4 py-3.5 flex items-center justify-between gap-3">
          <div>
            <div className="text-[11px] text-muted-foreground font-medium">
              {isVi
                ? "Tổng Khóa SSH trên Server / Trong Keychain"
                : "Remote Authorized Keys / Local Keychain"}
            </div>
            <div className="text-lg font-bold font-mono text-emerald-400 mt-0.5">
              {totalAuthorizedKeysCount}{" "}
              <span className="text-xs font-normal text-muted-foreground">
                / {sshKeys.length} {isVi ? "khóa cục bộ" : "local keys"}
              </span>
            </div>
          </div>
          <div className="w-9 h-9 rounded-xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
            <KeyRound className="w-4 h-4" />
          </div>
        </div>
      </div>

      {/* TWO-COLUMN ACCOUNT PRIVILEGE DECK + VISUAL KEYCHAIN BRIDGE MATRIX */}
      <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-start">
        {/* LEFT COLUMN (5 cols on XL): Linux Accounts Deck & "+ Create Account" Interactive Card */}
        <div className="xl:col-span-5 space-y-4">
          <section className="rounded-2xl border border-border/80 bg-card/75 p-4 space-y-3.5 shadow-xs">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center text-cyan-400 shrink-0">
                  <Users className="w-4 h-4" />
                </div>
                <div>
                  <h2 className="text-xs font-bold uppercase tracking-wider text-foreground">
                    {isVi ? "Danh sách Tài khoản Linux" : "Linux Accounts Deck"}
                  </h2>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Chọn một tài khoản để quản lý quyền Sudoers & khóa SSH."
                      : "Select an account to inspect sudoers & authorized_keys."}
                  </p>
                </div>
              </div>

              <div className="flex items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => fetchSnapshot()}
                  disabled={loading}
                  title={isVi ? "Làm mới danh sách" : "Refresh accounts"}
                  className="p-2 rounded-xl bg-secondary/80 hover:bg-accent text-muted-foreground hover:text-foreground border border-border/70 transition-colors cursor-pointer"
                >
                  <RefreshCw
                    className={`w-3.5 h-3.5 ${loading ? "animate-spin text-primary" : ""}`}
                  />
                </button>
                <button
                  type="button"
                  onClick={() => setCreateUserOpen((v) => !v)}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-95 transition-opacity cursor-pointer"
                >
                  <UserPlus className="w-3.5 h-3.5" />
                  <span>
                    {isVi ? "+ Tạo Tài khoản" : "+ Create Account"}
                  </span>
                </button>
              </div>
            </div>

            {/* "+ Create Linux Account" Interactive Card / Drawer */}
            {createUserOpen && (
              <form
                onSubmit={handleCreateLinuxUser}
                className="rounded-xl border border-primary/40 bg-background/85 p-4 space-y-3.5 shadow-lg ui-fade-slide"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-bold text-foreground">
                    <UserPlus className="w-3.5 h-3.5 text-primary" />
                    <span>
                      {isVi
                        ? "Tạo Tài khoản Linux Mới (useradd)"
                        : "Create New Linux Account (useradd)"}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => setCreateUserOpen(false)}
                    className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent cursor-pointer"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>

                <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                  <div>
                    <label
                      htmlFor={`${formUid}-new-username`}
                      className="ui-label"
                    >
                      {isVi ? "Tên người dùng (Username)" : "Username"}
                    </label>
                    <input
                      id={`${formUid}-new-username`}
                      type="text"
                      required
                      pattern="^[a-z_][a-z0-9_-]{0,30}$"
                      value={newUsername}
                      onChange={(e) =>
                        setNewUsername(e.target.value.toLowerCase())
                      }
                      placeholder="deploy, devops, admin..."
                      className="ui-input font-mono text-xs"
                    />
                  </div>

                  <div>
                    <label className="ui-label">
                      {isVi ? "Login Shell" : "Login Shell"}
                    </label>
                    <SelectField
                      size="sm"
                      value={newShell}
                      options={shellOptions}
                      onChange={setNewShell}
                    />
                  </div>
                </div>

                {/* Privilege Switches */}
                <div className="divide-y divide-border/60 rounded-xl border border-border/75 bg-card/60 overflow-hidden">
                  <label className="flex items-center justify-between gap-3 px-3 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                    <div>
                      <div className="text-xs font-semibold text-foreground">
                        {isVi
                          ? "Cấp quyền nhóm Sudo (sudo / wheel)"
                          : "Grant Sudo Group (sudo / wheel)"}
                      </div>
                      <div className="text-[10px] text-muted-foreground">
                        {isVi
                          ? "Cho phép thực thi lệnh quản trị hệ thống qua sudo"
                          : "Add user to sudo / wheel administrative group"}
                      </div>
                    </div>
                    <input
                      type="checkbox"
                      {...({ switch: "" } as any)}
                      checked={newSudoAccess || newNopasswdSudo}
                      onChange={(e) => {
                        const checked = e.target.checked;
                        setNewSudoAccess(checked);
                        if (!checked) setNewNopasswdSudo(false);
                      }}
                    />
                  </label>

                  <label className="flex items-center justify-between gap-3 px-3 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                    <div>
                      <div className="text-xs font-semibold text-foreground">
                        {isVi
                          ? "Sudo không cần mật khẩu (NOPASSWD)"
                          : "Passwordless Sudo (NOPASSWD)"}
                      </div>
                      <div className="text-[10px] text-muted-foreground font-mono">
                        /etc/sudoers.d/90-sshbox-{newUsername.trim() || "user"}
                      </div>
                    </div>
                    <input
                      type="checkbox"
                      {...({ switch: "" } as any)}
                      checked={newNopasswdSudo}
                      onChange={(e) => {
                        const checked = e.target.checked;
                        setNewNopasswdSudo(checked);
                        if (checked) setNewSudoAccess(true);
                      }}
                    />
                  </label>

                  <label className="flex items-center justify-between gap-3 px-3 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                    <div>
                      <div className="text-xs font-semibold text-foreground">
                        {isVi
                          ? "Thêm vào nhóm Docker (docker)"
                          : "Add to Docker Group (docker)"}
                      </div>
                      <div className="text-[10px] text-muted-foreground">
                        {isVi
                          ? "Cho phép chạy docker CLI không cần sudo (nếu đã cài Docker)"
                          : "Run docker CLI without sudo (if docker group exists)"}
                      </div>
                    </div>
                    <input
                      type="checkbox"
                      {...({ switch: "" } as any)}
                      checked={newDockerAccess}
                      onChange={(e) => setNewDockerAccess(e.target.checked)}
                    />
                  </label>
                </div>

                {/* Optional Initial SSH Key from ssh-box Keychain */}
                <div>
                  <label className="ui-label">
                    {isVi
                      ? "Gắn ngay Khóa SSH từ Keychain (Tùy chọn)"
                      : "Inject SSH Key from ssh-box Keychain (Optional)"}
                  </label>
                  <SelectField
                    size="sm"
                    value={newInitialKeyId}
                    options={initialKeyOptions}
                    onChange={setNewInitialKeyId}
                  />
                </div>

                <div className="flex items-center justify-end gap-2 pt-1">
                  <button
                    type="button"
                    onClick={() => setCreateUserOpen(false)}
                    className="px-3 py-1.5 rounded-xl bg-secondary text-xs font-medium text-foreground hover:bg-accent cursor-pointer"
                  >
                    {isVi ? "Hủy" : "Cancel"}
                  </button>
                  <button
                    type="submit"
                    disabled={
                      busyAction === "create-user" || !newUsername.trim()
                    }
                    className="px-3.5 py-1.5 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-95 disabled:opacity-50 inline-flex items-center gap-1.5 cursor-pointer"
                  >
                    {busyAction === "create-user" ? (
                      <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                    ) : (
                      <UserPlus className="w-3.5 h-3.5" />
                    )}
                    <span>
                      {busyAction === "create-user"
                        ? isVi
                          ? "Đang khởi tạo..."
                          : "Creating..."
                        : isVi
                          ? "Tạo Tài khoản"
                          : "Create Linux Account"}
                    </span>
                  </button>
                </div>
              </form>
            )}

            {/* Search Filter Input */}
            <div className="relative flex items-center">
              <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-3 pointer-events-none" />
              <input
                type="text"
                value={accountSearch}
                onChange={(e) => setAccountSearch(e.target.value)}
                placeholder={
                  isVi
                    ? "Tìm theo tên user, UID, nhóm hoặc shell..."
                    : "Filter by username, UID, group, or shell..."
                }
                className="ui-input pl-8 text-xs"
              />
            </div>

            {/* Accounts Deck List */}
            <div className="space-y-2 max-h-[560px] overflow-y-auto pr-0.5">
              {filteredAccounts.map((acc) => {
                const isSelected = selectedAccount?.username === acc.username;
                const isRoot = acc.uid === 0;
                const hasDocker = acc.groups.includes("docker");
                const isNologin =
                  acc.shell.includes("nologin") || acc.shell.includes("false");
                const linkedCount = acc.authorizedKeys.filter((rk) =>
                  localKeyByBody.has(rk.publicKeyBody.trim()),
                ).length;

                return (
                  <button
                    key={acc.username}
                    type="button"
                    onClick={() => setSelectedUsername(acc.username)}
                    className={`w-full text-left p-3.5 rounded-xl border transition-all cursor-pointer space-y-2.5 ${
                      isSelected
                        ? "border-primary bg-primary/12 shadow-xs"
                        : "border-border/75 bg-background/50 hover:bg-accent/40 hover:border-border"
                    }`}
                  >
                    <div className="flex items-start justify-between gap-2">
                      <div className="flex items-center gap-2.5 min-w-0">
                        <div
                          className={`w-9 h-9 rounded-xl border flex items-center justify-center shrink-0 ${
                            isRoot
                              ? "bg-emerald-500/15 border-emerald-500/35 text-emerald-400"
                              : acc.locked
                                ? "bg-rose-500/15 border-rose-500/35 text-rose-400"
                                : "bg-cyan-500/15 border-cyan-500/30 text-cyan-400"
                          }`}
                        >
                          {isRoot ? (
                            <Crown className="w-4 h-4" />
                          ) : acc.locked ? (
                            <Lock className="w-4 h-4" />
                          ) : (
                            <UserCheck className="w-4 h-4" />
                          )}
                        </div>

                        <div className="min-w-0">
                          <div className="flex items-center gap-1.5 flex-wrap">
                            <span className="text-sm font-bold font-mono text-foreground truncate">
                              {acc.username}
                            </span>
                            {acc.username === host.user && (
                              <span className="px-1.5 py-0.5 rounded-md bg-primary/20 text-primary border border-primary/30 text-[10px] font-semibold">
                                {isVi ? "Phiên hiện tại" : "Current SSH"}
                              </span>
                            )}
                          </div>
                          <div className="flex items-center gap-1.5 mt-0.5 flex-wrap">
                            <span className="px-1.5 py-0.5 rounded-md bg-secondary text-[10px] font-mono text-muted-foreground border border-border/60">
                              UID {acc.uid}:{acc.gid}
                            </span>
                            <span
                              className={`px-1.5 py-0.5 rounded-md text-[10px] font-mono border ${
                                isNologin
                                  ? "bg-amber-500/10 text-amber-300 border-amber-500/25"
                                  : "bg-secondary/80 text-foreground border-border/60"
                              }`}
                            >
                              {acc.shell}
                            </span>
                          </div>
                        </div>
                      </div>

                      {/* Authorized Keys count badge */}
                      <div className="flex flex-col items-end gap-1 shrink-0">
                        <span
                          className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-semibold border ${
                            acc.authorizedKeys.length > 0
                              ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                              : "bg-secondary text-muted-foreground border-border"
                          }`}
                        >
                          <KeyRound className="w-3 h-3" />
                          <span>
                            {acc.authorizedKeys.length}{" "}
                            {acc.authorizedKeys.length === 1 ? "key" : "keys"}
                          </span>
                        </span>
                        {linkedCount > 0 && (
                          <span className="text-[10px] font-mono text-emerald-400">
                            {linkedCount} linked
                          </span>
                        )}
                      </div>
                    </div>

                    {/* Privilege Badges Row */}
                    <div className="flex flex-wrap items-center gap-1.5">
                      {isRoot && (
                        <span className="px-2 py-0.5 rounded-md bg-emerald-500/15 text-emerald-400 border border-emerald-500/35 text-[10px] font-mono font-bold">
                          ROOT
                        </span>
                      )}
                      {!isRoot && acc.nopasswdSudo && (
                        <span className="px-2 py-0.5 rounded-md bg-cyan-500/15 text-cyan-300 border border-cyan-500/35 text-[10px] font-mono font-bold">
                          SUDO + NOPASSWD
                        </span>
                      )}
                      {!isRoot && acc.hasSudo && !acc.nopasswdSudo && (
                        <span className="px-2 py-0.5 rounded-md bg-violet-500/15 text-violet-300 border border-violet-500/35 text-[10px] font-mono font-bold">
                          SUDO
                        </span>
                      )}
                      {hasDocker && (
                        <span className="px-2 py-0.5 rounded-md bg-sky-500/15 text-sky-300 border border-sky-500/35 text-[10px] font-mono font-bold">
                          DOCKER
                        </span>
                      )}
                      {acc.locked && (
                        <span className="px-2 py-0.5 rounded-md bg-rose-500/15 text-rose-400 border border-rose-500/35 text-[10px] font-mono font-bold">
                          LOCKED
                        </span>
                      )}
                      {!isRoot &&
                        !acc.hasSudo &&
                        !hasDocker &&
                        !acc.locked && (
                          <span className="px-2 py-0.5 rounded-md bg-secondary/70 text-muted-foreground border border-border/60 text-[10px] font-mono">
                            STANDARD USER
                          </span>
                        )}
                    </div>
                  </button>
                );
              })}

              {filteredAccounts.length === 0 && (
                <div className="p-6 text-center text-xs text-muted-foreground border border-dashed border-border rounded-xl">
                  {isVi
                    ? "Không tìm thấy tài khoản Linux nào khớp với bộ lọc."
                    : "No Linux accounts match your search filter."}
                </div>
              )}
            </div>
          </section>
        </div>

        {/* RIGHT COLUMN (7 cols on XL): Selected Account Privilege Controls + Authorized Keys Inspector + Local Keychain Bridge */}
        <div className="xl:col-span-7 space-y-4">
          {selectedAccount ? (
            <>
              {/* HEADER CARD: Account Privilege Controls */}
              <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
                <div className="flex flex-wrap items-start justify-between gap-3">
                  <div className="flex items-center gap-3 min-w-0">
                    <div
                      className={`w-10 h-10 rounded-xl border flex items-center justify-center shrink-0 ${
                        selectedAccount.uid === 0
                          ? "bg-emerald-500/15 border-emerald-500/35 text-emerald-400"
                          : selectedAccount.locked
                            ? "bg-rose-500/15 border-rose-500/35 text-rose-400"
                            : "bg-cyan-500/15 border-cyan-500/35 text-cyan-400"
                      }`}
                    >
                      {selectedAccount.uid === 0 ? (
                        <Crown className="w-5 h-5" />
                      ) : selectedAccount.locked ? (
                        <Lock className="w-5 h-5" />
                      ) : (
                        <Shield className="w-5 h-5" />
                      )}
                    </div>
                    <div className="min-w-0">
                      <div className="flex items-center gap-2 flex-wrap">
                        <h3 className="text-base font-bold font-mono text-foreground">
                          {selectedAccount.username}@{host.alias || host.hostname}
                        </h3>
                        <span className="px-2 py-0.5 rounded-md bg-secondary border border-border text-[11px] font-mono text-muted-foreground">
                          UID {selectedAccount.uid} · GID {selectedAccount.gid}
                        </span>
                      </div>
                      <div className="flex flex-wrap items-center gap-x-4 gap-y-1 text-xs text-muted-foreground mt-1 font-mono">
                        <span className="inline-flex items-center gap-1">
                          <FolderGit2 className="w-3.5 h-3.5 text-cyan-400" />
                          <span>Home: {selectedAccount.homeDir}</span>
                        </span>
                        <span className="inline-flex items-center gap-1">
                          <Terminal className="w-3.5 h-3.5 text-emerald-400" />
                          <span>Shell: {selectedAccount.shell}</span>
                        </span>
                      </div>
                    </div>
                  </div>

                  {busyAction === `priv-${selectedAccount.username}` && (
                    <span className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-primary/15 border border-primary/30 text-xs font-medium text-primary">
                      <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                      <span>
                        {isVi ? "Đang cập nhật..." : "Updating sudoers..."}
                      </span>
                    </span>
                  )}
                </div>

                {/* Supplementary Groups Pill Strip */}
                <div className="flex flex-wrap items-center gap-1.5 pt-1">
                  <span className="text-[11px] font-semibold text-muted-foreground mr-1">
                    {isVi ? "Nhóm (Groups):" : "Groups:"}
                  </span>
                  {selectedAccount.groups.map((grp) => (
                    <span
                      key={grp}
                      className={`px-2 py-0.5 rounded-lg text-[11px] font-mono border ${
                        grp === "sudo" || grp === "wheel" || grp === "root"
                          ? "bg-violet-500/15 text-violet-300 border-violet-500/35 font-semibold"
                          : grp === "docker"
                            ? "bg-sky-500/15 text-sky-300 border-sky-500/35 font-semibold"
                            : "bg-secondary/80 text-foreground border-border/70"
                      }`}
                    >
                      {grp}
                    </span>
                  ))}
                </div>

                {/* Non-root Interactive Privilege Switches */}
                {selectedAccount.uid === 0 ? (
                  <div className="rounded-xl border border-emerald-500/30 bg-emerald-500/10 p-3 flex items-center gap-2.5 text-xs text-emerald-200">
                    <ShieldCheck className="w-4 h-4 text-emerald-400 shrink-0" />
                    <span>
                      {isVi
                        ? "Tài khoản siêu quản trị root (UID 0) có toàn quyền hệ thống mặc định. Bạn có thể quản lý danh sách khóa SSH của root ngay bên dưới."
                        : "Root superuser (UID 0) holds full system privileges by default. Manage root's authorized_keys directly below."}
                    </span>
                  </div>
                ) : (
                  <div className="divide-y divide-border/60 rounded-xl border border-border/75 bg-background/45 overflow-hidden">
                    <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                      <div>
                        <div className="text-xs font-semibold text-foreground">
                          {isVi
                            ? "Quyền Quản trị Sudo (Sudo Group Access)"
                            : "Sudo Group Access (sudo / wheel)"}
                        </div>
                        <div className="text-[11px] text-muted-foreground">
                          {isVi
                            ? `Cho phép ${selectedAccount.username} nâng quyền quản trị bằng lệnh sudo`
                            : `Grant ${selectedAccount.username} membership in the system sudo/wheel group`}
                        </div>
                      </div>
                      <input
                        type="checkbox"
                        {...({ switch: "" } as any)}
                        disabled={
                          busyAction === `priv-${selectedAccount.username}`
                        }
                        checked={selectedAccount.hasSudo}
                        onChange={(e) =>
                          handleUpdatePrivilege(
                            selectedAccount,
                            e.target.checked,
                            e.target.checked
                              ? selectedAccount.nopasswdSudo
                              : false,
                            selectedAccount.locked,
                          )
                        }
                      />
                    </label>

                    <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                      <div>
                        <div className="text-xs font-semibold text-foreground">
                          {isVi
                            ? `Sudo không cần mật khẩu (/etc/sudoers.d/90-sshbox-${selectedAccount.username})`
                            : `Passwordless Sudo (/etc/sudoers.d/90-sshbox-${selectedAccount.username})`}
                        </div>
                        <div className="text-[11px] text-muted-foreground font-mono">
                          {selectedAccount.username} ALL=(ALL) NOPASSWD:ALL (visudo -cf)
                        </div>
                      </div>
                      <input
                        type="checkbox"
                        {...({ switch: "" } as any)}
                        disabled={
                          busyAction === `priv-${selectedAccount.username}`
                        }
                        checked={selectedAccount.nopasswdSudo}
                        onChange={(e) =>
                          handleUpdatePrivilege(
                            selectedAccount,
                            e.target.checked ? true : selectedAccount.hasSudo,
                            e.target.checked,
                            selectedAccount.locked,
                          )
                        }
                      />
                    </label>

                    <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                      <div>
                        <div className="text-xs font-semibold text-foreground flex items-center gap-1.5">
                          <span>
                            {isVi
                              ? "Khóa Tài khoản (Lock Account — usermod -L)"
                              : "Lock Account (Disable Password/SSH Login — usermod -L)"}
                          </span>
                          {selectedAccount.locked && (
                            <span className="px-1.5 py-0.5 rounded-md bg-rose-500/20 text-rose-300 text-[10px] font-mono">
                              LOCKED
                            </span>
                          )}
                        </div>
                        <div className="text-[11px] text-muted-foreground">
                          {isVi
                            ? "Đình chỉ tạm thời tài khoản mà không xóa thư mục home hay khóa SSH"
                            : "Suspend login access for this user without deleting their home directory or keys"}
                        </div>
                      </div>
                      <input
                        type="checkbox"
                        {...({ switch: "" } as any)}
                        disabled={
                          busyAction === `priv-${selectedAccount.username}`
                        }
                        checked={selectedAccount.locked}
                        onChange={(e) =>
                          handleUpdatePrivilege(
                            selectedAccount,
                            selectedAccount.hasSudo,
                            selectedAccount.nopasswdSudo,
                            e.target.checked,
                          )
                        }
                      />
                    </label>
                  </div>
                )}
              </section>

              {/* PANEL A: Keys Currently Authorized on <username> (~/.ssh/authorized_keys) */}
              <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-2.5">
                    <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shrink-0">
                      <KeyRound className="w-4 h-4" />
                    </div>
                    <div>
                      <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                        {isVi
                          ? `Khóa đang có quyền truy cập vào ${selectedAccount.username}`
                          : `Keys Currently Authorized on ${selectedAccount.username}`}
                      </h3>
                      <p className="text-[11px] text-muted-foreground font-mono">
                        {selectedAccount.homeDir}/.ssh/authorized_keys (
                        {selectedAccount.authorizedKeys.length})
                      </p>
                    </div>
                  </div>

                  <button
                    type="button"
                    onClick={() => setPasteKeyOpen((v) => !v)}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-secondary hover:bg-accent text-xs font-semibold text-foreground border border-border/80 transition-colors cursor-pointer"
                  >
                    <Plus className="w-3.5 h-3.5 text-emerald-400" />
                    <span>
                      {isVi
                        ? "+ Dán Khóa Công Khai Ngoài"
                        : "+ Paste External Public Key"}
                    </span>
                  </button>
                </div>

                {/* Expandable "+ Paste External Public Key" Bar */}
                {pasteKeyOpen && (
                  <form
                    onSubmit={handlePasteExternalKeySubmit}
                    className="rounded-xl border border-emerald-500/35 bg-background/80 p-3.5 space-y-2.5 ui-fade-slide"
                  >
                    <div className="flex items-center justify-between">
                      <label
                        htmlFor={`${formUid}-paste-pubkey`}
                        className="text-xs font-semibold text-foreground"
                      >
                        {isVi
                          ? `Thêm khóa công khai vào ${selectedAccount.username}/.ssh/authorized_keys`
                          : `Append Public Key to ${selectedAccount.username}/.ssh/authorized_keys`}
                      </label>
                      <button
                        type="button"
                        onClick={() => setPasteKeyOpen(false)}
                        className="p-1 rounded-lg text-muted-foreground hover:text-foreground"
                      >
                        <X className="w-3.5 h-3.5" />
                      </button>
                    </div>
                    <textarea
                      id={`${formUid}-paste-pubkey`}
                      rows={2}
                      required
                      value={externalPublicKey}
                      onChange={(e) => setExternalPublicKey(e.target.value)}
                      placeholder="ssh-ed25519 AAAAC3NzaC1lZDI1NTE5AAAAI... user@workstation"
                      className="ui-textarea font-mono text-xs"
                    />
                    <div className="flex items-center justify-end gap-2">
                      <button
                        type="button"
                        onClick={() => setPasteKeyOpen(false)}
                        className="px-3 py-1.5 rounded-xl bg-secondary text-xs font-medium text-foreground hover:bg-accent cursor-pointer"
                      >
                        {isVi ? "Đóng" : "Cancel"}
                      </button>
                      <button
                        type="submit"
                        disabled={!externalPublicKey.trim() || !!busyAction}
                        className="px-3.5 py-1.5 rounded-xl bg-emerald-500 text-slate-950 text-xs font-bold hover:opacity-95 disabled:opacity-50 cursor-pointer"
                      >
                        {isVi
                          ? `Cấp quyền cho ${selectedAccount.username}`
                          : `Authorize for ${selectedAccount.username}`}
                      </button>
                    </div>
                  </form>
                )}

                {/* Authorized Keys Visual Cards */}
                <div className="space-y-2.5">
                  {selectedAccount.authorizedKeys.map((rk) => {
                    const algo = getKeyAlgorithmBadge(rk.keyType);
                    const matchedLocalKey = localKeyByBody.get(
                      rk.publicKeyBody.trim(),
                    );
                    const revokeBusy =
                      busyAction ===
                      `revoke-${selectedAccount.username}-${rk.publicKeyBody.slice(-8)}`;

                    return (
                      <div
                        key={`${rk.index}-${rk.publicKeyBody.slice(-16)}`}
                        className={`p-3.5 rounded-xl border transition-all flex flex-col sm:flex-row sm:items-center justify-between gap-3 ${
                          matchedLocalKey
                            ? "border-emerald-500/40 bg-emerald-500/5"
                            : "border-border/75 bg-background/50"
                        }`}
                      >
                        <div className="min-w-0 space-y-1.5">
                          <div className="flex flex-wrap items-center gap-2">
                            <span
                              className={`px-2 py-0.5 rounded-md text-[10px] font-mono font-bold border ${algo.className}`}
                            >
                              {algo.label}
                            </span>

                            <span className="text-xs font-bold text-foreground truncate">
                              {rk.comment ||
                                (isVi
                                  ? `Khóa không nhãn #${rk.index}`
                                  : `Unlabeled Key #${rk.index}`)}
                            </span>

                            {matchedLocalKey && (
                              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full bg-emerald-500/20 text-emerald-300 border border-emerald-500/40 text-[10px] font-semibold shadow-2xs">
                                <Link2 className="w-3 h-3 text-emerald-400" />
                                <span>
                                  Linked in ssh-box: {matchedLocalKey.name}
                                </span>
                              </span>
                            )}
                          </div>

                          <div className="flex flex-wrap items-center gap-2 text-[11px] font-mono text-muted-foreground">
                            <span>{rk.keyType}</span>
                            <span>·</span>
                            <code className="px-1.5 py-0.5 rounded-md bg-secondary/90 text-foreground border border-border/60">
                              {rk.shortPreview}
                            </code>
                          </div>
                        </div>

                        <button
                          type="button"
                          disabled={revokeBusy}
                          onClick={() =>
                            handleRevokeKey(
                              selectedAccount.username,
                              rk.publicKeyBody,
                              rk.comment || matchedLocalKey?.name || "",
                            )
                          }
                          className="px-3 py-1.5 rounded-xl bg-rose-500/15 hover:bg-rose-500/25 text-rose-300 border border-rose-500/35 text-xs font-semibold inline-flex items-center gap-1.5 shrink-0 cursor-pointer transition-colors disabled:opacity-50"
                        >
                          {revokeBusy ? (
                            <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                          ) : (
                            <Trash2 className="w-3.5 h-3.5" />
                          )}
                          <span>{isVi ? "Thu hồi (Revoke)" : "Revoke Key"}</span>
                        </button>
                      </div>
                    );
                  })}

                  {selectedAccount.authorizedKeys.length === 0 && (
                    <div className="p-6 rounded-xl border border-dashed border-border text-center space-y-1.5">
                      <div className="text-xs font-semibold text-foreground">
                        {isVi
                          ? `Chưa có khóa SSH nào trong ${selectedAccount.homeDir}/.ssh/authorized_keys`
                          : `No SSH keys present in ${selectedAccount.homeDir}/.ssh/authorized_keys`}
                      </div>
                      <p className="text-[11px] text-muted-foreground">
                        {isVi
                          ? "Hãy bấm 'Authorize' ở bảng Cầu nối Keychain bên dưới để nạp khóa từ ssh-box chỉ với 1 cú nhấp."
                          : "Use the ssh-box Local Keychain Bridge below to authorize a key with 1 click."}
                      </p>
                    </div>
                  )}
                </div>
              </section>

              {/* PANEL B: ssh-box Local Keychain Bridge (1-Click Authorize) */}
              <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
                <div className="flex items-center justify-between gap-2">
                  <div className="flex items-center gap-2.5">
                    <div className="w-8 h-8 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center text-cyan-400 shrink-0">
                      <Link2 className="w-4 h-4" />
                    </div>
                    <div>
                      <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                        {isVi
                          ? `Cầu nối Kho Khóa Cục Bộ ssh-box → ${selectedAccount.username}`
                          : `ssh-box Local Keychain Bridge → ${selectedAccount.username}`}
                      </h3>
                      <p className="text-[11px] text-muted-foreground">
                        {isVi
                          ? "Cấp quyền 1-chạm cho các khóa lưu trong Keychain của ssh-box và tự động chuẩn hóa phân quyền 0700 / 0600."
                          : "1-click inject keys from your local ssh-box Keychain with automatic 0700 / 0600 permission hardening."}
                      </p>
                    </div>
                  </div>

                  <span className="px-2.5 py-1 rounded-full bg-secondary border border-border text-[11px] font-mono text-muted-foreground">
                    {sshKeys.length} {isVi ? "khóa cục bộ" : "local keys"}
                  </span>
                </div>

                {sshKeys.length === 0 ? (
                  <div className="p-6 rounded-xl border border-dashed border-border text-center space-y-1">
                    <div className="text-xs font-semibold text-foreground">
                      {isVi
                        ? "Kho khóa cục bộ (Keychain) của ssh-box đang trống"
                        : "No SSH Keys in Local ssh-box Keychain"}
                    </div>
                    <p className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Bạn có thể tạo hoặc nhập khóa Ed25519/RSA trong mục Keychain → Keys, hoặc dùng nút '+ Dán Khóa Công Khai Ngoài' ở trên."
                        : "Generate or import keys in Keychain → Keys, or use '+ Paste External Public Key' above."}
                    </p>
                  </div>
                ) : (
                  <div className="grid grid-cols-1 gap-2.5">
                    {sshKeys.map((localKey) => {
                      const body = extractPublicKeyBody(localKey.publicKey);
                      const isAlreadyAuthorized =
                        !!body && selectedAccountKeyBodies.has(body);
                      const algo = getKeyAlgorithmBadge(localKey.keyType);
                      const authBusy =
                        busyAction ===
                        `auth-${selectedAccount.username}-${body.slice(-8)}`;

                      return (
                        <div
                          key={localKey.id}
                          className={`p-3.5 rounded-xl border transition-all flex flex-col sm:flex-row sm:items-center justify-between gap-3 ${
                            isAlreadyAuthorized
                              ? "border-emerald-500/35 bg-emerald-500/8"
                              : "border-border/75 bg-background/50 hover:border-border"
                          }`}
                        >
                          <div className="min-w-0 space-y-1">
                            <div className="flex flex-wrap items-center gap-2">
                              <span
                                className={`px-2 py-0.5 rounded-md text-[10px] font-mono font-bold border ${algo.className}`}
                              >
                                {algo.label}
                              </span>
                              <span className="text-xs font-bold text-foreground truncate">
                                {localKey.name}
                              </span>
                              {localKey.remark && (
                                <span className="text-[11px] text-muted-foreground truncate">
                                  ({localKey.remark})
                                </span>
                              )}
                            </div>

                            <div className="flex flex-wrap items-center gap-2 text-[11px] font-mono text-muted-foreground">
                              {localKey.fingerprint && (
                                <span className="truncate">
                                  {localKey.fingerprint}
                                </span>
                              )}
                              {body && (
                                <code className="px-1.5 py-0.5 rounded-md bg-secondary/90 text-foreground border border-border/60">
                                  {formatKeyShortPreview(body)}
                                </code>
                              )}
                            </div>
                          </div>

                          <div className="shrink-0">
                            {isAlreadyAuthorized ? (
                              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-emerald-500/15 text-emerald-300 border border-emerald-500/35 text-xs font-semibold">
                                <CheckCircle2 className="w-3.5 h-3.5 text-emerald-400" />
                                <span>
                                  {isVi
                                    ? `Đã cấp quyền trên ${selectedAccount.username} ✓`
                                    : `Authorized on ${selectedAccount.username} ✓`}
                                </span>
                              </span>
                            ) : (
                              <button
                                type="button"
                                disabled={authBusy || !body}
                                onClick={() =>
                                  handleAuthorizeKey(
                                    selectedAccount.username,
                                    localKey.publicKey,
                                    localKey.name,
                                  )
                                }
                                className="px-3.5 py-1.5 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-95 disabled:opacity-50 inline-flex items-center gap-1.5 cursor-pointer transition-opacity"
                              >
                                {authBusy ? (
                                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                                ) : (
                                  <Unlock className="w-3.5 h-3.5" />
                                )}
                                <span>
                                  {isVi
                                    ? `Authorize cho ${selectedAccount.username}`
                                    : `Authorize for ${selectedAccount.username}`}
                                </span>
                              </button>
                            )}
                          </div>
                        </div>
                      );
                    })}
                  </div>
                )}
              </section>
            </>
          ) : (
            <div className="rounded-2xl border border-border/80 bg-card/75 p-8 text-center text-xs text-muted-foreground">
              {isVi
                ? "Chọn một tài khoản Linux ở cột bên trái để xem chi tiết."
                : "Select a Linux account on the left to inspect its privileges and authorized keys."}
            </div>
          )}
        </div>
      </div>
    </div>
  );
};
