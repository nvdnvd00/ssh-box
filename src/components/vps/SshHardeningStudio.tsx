import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  AlertTriangle,
  Check,
  CheckCircle2,
  Clock,
  Dices,
  FileDiff,
  KeyRound,
  Lock,
  Network,
  Plus,
  RefreshCw,
  RotateCcw,
  Shield,
  ShieldAlert,
  ShieldCheck,
  Sparkles,
  Undo2,
  Users,
  X,
  Zap,
} from "lucide-react";
import {
  HostEntry,
  SshdConfigSnapshot,
  SshdConfigUpdate,
  VpsPrivilegeInfo,
} from "../../types";
import { AppLanguage } from "../../themes";

interface SshHardeningStudioProps {
  host: HostEntry;
  sudoPassword?: string;
  privilegeInfo: VpsPrivilegeInfo | null;
  language?: AppLanguage;
  onSaveHost: (host: HostEntry, secret?: string) => Promise<void>;
  onRequireSudoUnlock?: () => void;
}

function snapshotToUpdate(snap: SshdConfigSnapshot): SshdConfigUpdate {
  return {
    port: snap.port,
    permitRootLogin: snap.permitRootLogin,
    passwordAuthentication: snap.passwordAuthentication,
    pubkeyAuthentication: snap.pubkeyAuthentication,
    permitEmptyPasswords: snap.permitEmptyPasswords,
    maxAuthTries: snap.maxAuthTries,
    loginGraceTime: snap.loginGraceTime,
    clientAliveInterval: snap.clientAliveInterval,
    clientAliveCountMax: snap.clientAliveCountMax,
    allowUsers: [...snap.allowUsers],
    x11Forwarding: snap.x11Forwarding,
    allowTcpForwarding: snap.allowTcpForwarding,
    allowAgentForwarding: snap.allowAgentForwarding,
    addressFamily: snap.addressFamily,
  };
}

function computeDraftSecurityScore(cfg: SshdConfigUpdate): {
  score: number;
  checks: { id: string; label: string; points: number; passed: boolean }[];
} {
  const checks = [
    {
      id: "empty-pw",
      label: "PermitEmptyPasswords no",
      points: 15,
      passed: !cfg.permitEmptyPasswords,
    },
    {
      id: "pubkey",
      label: "PubkeyAuthentication yes",
      points: 15,
      passed: cfg.pubkeyAuthentication,
    },
    {
      id: "no-pw",
      label: "PasswordAuthentication no",
      points: 25,
      passed: !cfg.passwordAuthentication,
    },
    {
      id: "root-login",
      label:
        cfg.permitRootLogin === "no"
          ? "PermitRootLogin no (+20)"
          : cfg.permitRootLogin === "prohibit-password"
            ? "PermitRootLogin key-only (+12)"
            : "PermitRootLogin no",
      points:
        cfg.permitRootLogin === "no"
          ? 20
          : cfg.permitRootLogin === "prohibit-password"
            ? 12
            : 0,
      passed: cfg.permitRootLogin !== "yes",
    },
    {
      id: "custom-port",
      label: `Custom SSH Port (${cfg.port})`,
      points: 10,
      passed: cfg.port !== 22,
    },
    {
      id: "max-tries",
      label: `MaxAuthTries ≤ 4 (${cfg.maxAuthTries})`,
      points: 5,
      passed: cfg.maxAuthTries <= 4,
    },
    {
      id: "x11-off",
      label: "X11Forwarding no",
      points: 5,
      passed: !cfg.x11Forwarding,
    },
    {
      id: "session-hardening",
      label: "Idle Keepalive / Grace ≤ 60s / AllowUsers",
      points: 5,
      passed:
        (cfg.clientAliveInterval > 0 && cfg.clientAliveInterval <= 600) ||
        cfg.allowUsers.length > 0 ||
        cfg.loginGraceTime <= 60,
    },
  ];

  const score = Math.min(
    100,
    checks.reduce((acc, item) => acc + (item.passed ? item.points : 0), 0),
  );
  return { score, checks };
}

interface DiffEntry {
  directive: string;
  before: string | null;
  after: string | null;
  changed: boolean;
}

function buildSshdDirectiveDiff(
  base: SshdConfigSnapshot,
  draft: SshdConfigUpdate,
): DiffEntry[] {
  const yn = (b: boolean) => (b ? "yes" : "no");
  const rows: DiffEntry[] = [
    {
      directive: "Port",
      before: String(base.port),
      after: String(draft.port),
      changed: base.port !== draft.port,
    },
    {
      directive: "AddressFamily",
      before: base.addressFamily,
      after: draft.addressFamily,
      changed: base.addressFamily !== draft.addressFamily,
    },
    {
      directive: "PermitRootLogin",
      before: base.permitRootLogin,
      after: draft.permitRootLogin,
      changed: base.permitRootLogin !== draft.permitRootLogin,
    },
    {
      directive: "PubkeyAuthentication",
      before: yn(base.pubkeyAuthentication),
      after: yn(draft.pubkeyAuthentication),
      changed: base.pubkeyAuthentication !== draft.pubkeyAuthentication,
    },
    {
      directive: "PasswordAuthentication",
      before: yn(base.passwordAuthentication),
      after: yn(draft.passwordAuthentication),
      changed: base.passwordAuthentication !== draft.passwordAuthentication,
    },
    {
      directive: "PermitEmptyPasswords",
      before: yn(base.permitEmptyPasswords),
      after: yn(draft.permitEmptyPasswords),
      changed: base.permitEmptyPasswords !== draft.permitEmptyPasswords,
    },
    {
      directive: "MaxAuthTries",
      before: String(base.maxAuthTries),
      after: String(draft.maxAuthTries),
      changed: base.maxAuthTries !== draft.maxAuthTries,
    },
    {
      directive: "LoginGraceTime",
      before: String(base.loginGraceTime),
      after: String(draft.loginGraceTime),
      changed: base.loginGraceTime !== draft.loginGraceTime,
    },
    {
      directive: "ClientAliveInterval",
      before: String(base.clientAliveInterval),
      after: String(draft.clientAliveInterval),
      changed: base.clientAliveInterval !== draft.clientAliveInterval,
    },
    {
      directive: "ClientAliveCountMax",
      before: String(base.clientAliveCountMax),
      after: String(draft.clientAliveCountMax),
      changed: base.clientAliveCountMax !== draft.clientAliveCountMax,
    },
    {
      directive: "AllowUsers",
      before: base.allowUsers.length > 0 ? base.allowUsers.join(" ") : null,
      after: draft.allowUsers.length > 0 ? draft.allowUsers.join(" ") : null,
      changed: base.allowUsers.join(" ") !== draft.allowUsers.join(" "),
    },
    {
      directive: "X11Forwarding",
      before: yn(base.x11Forwarding),
      after: yn(draft.x11Forwarding),
      changed: base.x11Forwarding !== draft.x11Forwarding,
    },
    {
      directive: "AllowTcpForwarding",
      before: yn(base.allowTcpForwarding),
      after: yn(draft.allowTcpForwarding),
      changed: base.allowTcpForwarding !== draft.allowTcpForwarding,
    },
    {
      directive: "AllowAgentForwarding",
      before: yn(base.allowAgentForwarding),
      after: yn(draft.allowAgentForwarding),
      changed: base.allowAgentForwarding !== draft.allowAgentForwarding,
    },
  ];
  return rows;
}

export const SshHardeningStudio: React.FC<SshHardeningStudioProps> = ({
  host,
  sudoPassword,
  privilegeInfo,
  language = "vi",
  onSaveHost,
  onRequireSudoUnlock,
}) => {
  const isVi = language === "vi";
  const [snapshot, setSnapshot] = useState<SshdConfigSnapshot | null>(null);
  const [draft, setDraft] = useState<SshdConfigUpdate | null>(null);
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);
  const [allowUserInput, setAllowUserInput] = useState("");
  const [showRawConfig, setShowRawConfig] = useState(false);

  // 30-second anti-lockout rollback state
  const [rollbackModalOpen, setRollbackModalOpen] = useState(false);
  const [rollbackSecondsLeft, setRollbackSecondsLeft] = useState(30);
  const [confirmingRollback, setConfirmingRollback] = useState(false);

  const fetchConfig = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const res = await invoke<SshdConfigSnapshot>("vps_get_sshd_config", {
        hostId: host.id,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(res);
      setDraft(snapshotToUpdate(res));
      if (res.rollbackPending) {
        setRollbackSecondsLeft(25);
        setRollbackModalOpen(true);
      }
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setLoading(false);
    }
  }, [host.id, sudoPassword, onRequireSudoUnlock]);

  useEffect(() => {
    fetchConfig();
  }, [fetchConfig]);

  // Live 30s countdown timer when rollbackModalOpen is true
  useEffect(() => {
    if (!rollbackModalOpen) return;
    if (rollbackSecondsLeft <= 0) {
      setRollbackModalOpen(false);
      setToastMsg(
        isVi
          ? "Đã hết 30 giây: Máy chủ đã tự động khôi phục cấu hình sshd_config an toàn trước đó."
          : "30s elapsed: Server automatically rolled back to previous safe sshd_config.",
      );
      fetchConfig();
      return;
    }
    const timer = setTimeout(() => {
      setRollbackSecondsLeft((prev) => Math.max(0, prev - 1));
    }, 1000);
    return () => clearTimeout(timer);
  }, [rollbackModalOpen, rollbackSecondsLeft, fetchConfig, isVi]);

  useEffect(() => {
    if (!toastMsg) return;
    const t = setTimeout(() => setToastMsg(null), 5000);
    return () => clearTimeout(t);
  }, [toastMsg]);

  const posture = useMemo(
    () => (draft ? computeDraftSecurityScore(draft) : null),
    [draft],
  );

  const diffEntries = useMemo(
    () => (snapshot && draft ? buildSshdDirectiveDiff(snapshot, draft) : []),
    [snapshot, draft],
  );

  const changedEntries = useMemo(
    () => diffEntries.filter((d) => d.changed),
    [diffEntries],
  );

  const isLockoutRisk = useMemo(() => {
    if (!draft || !snapshot) return false;
    if (!draft.passwordAuthentication && !draft.pubkeyAuthentication) {
      return true;
    }
    if (!draft.passwordAuthentication && snapshot.authorizedKeysCount === 0) {
      return true;
    }
    return false;
  }, [draft, snapshot]);

  const handleApplyPreset = (
    preset: "fortress" | "balanced" | "reset",
  ) => {
    if (!snapshot || !draft) return;
    if (preset === "reset") {
      setDraft(snapshotToUpdate(snapshot));
      return;
    }
    if (preset === "fortress") {
      setDraft({
        ...draft,
        permitRootLogin: "no",
        passwordAuthentication: false,
        pubkeyAuthentication: true,
        permitEmptyPasswords: false,
        maxAuthTries: 3,
        loginGraceTime: 45,
        clientAliveInterval: 300,
        clientAliveCountMax: 2,
        x11Forwarding: false,
        allowTcpForwarding: true,
        allowAgentForwarding: false,
      });
    } else if (preset === "balanced") {
      setDraft({
        ...draft,
        permitRootLogin: "prohibit-password",
        passwordAuthentication: true,
        pubkeyAuthentication: true,
        permitEmptyPasswords: false,
        maxAuthTries: 3,
        loginGraceTime: 60,
        clientAliveInterval: 300,
        clientAliveCountMax: 3,
        x11Forwarding: false,
        allowTcpForwarding: true,
        allowAgentForwarding: true,
      });
    }
  };

  const handleGenerateRandomPort = () => {
    if (!draft) return;
    const randomPort = Math.floor(20000 + Math.random() * 40000);
    setDraft({ ...draft, port: randomPort });
  };

  const handleAddAllowUser = (rawUser?: string) => {
    if (!draft) return;
    const candidate = (rawUser ?? allowUserInput).trim();
    if (!candidate) return;
    const tokens = candidate
      .split(/[\s,]+/)
      .map((s) => s.trim())
      .filter(Boolean);
    const nextUsers = [...draft.allowUsers];
    for (const tok of tokens) {
      if (!nextUsers.includes(tok)) {
        nextUsers.push(tok);
      }
    }
    setDraft({ ...draft, allowUsers: nextUsers });
    if (!rawUser) {
      setAllowUserInput("");
    }
  };

  const handleRemoveAllowUser = (target: string) => {
    if (!draft) return;
    setDraft({
      ...draft,
      allowUsers: draft.allowUsers.filter((u) => u !== target),
    });
  };

  const handleApplyConfig = async (withAutoRollback: boolean) => {
    if (!draft) return;
    if (!draft.passwordAuthentication && !draft.pubkeyAuthentication) {
      setErrorMsg(
        isVi
          ? "Không thể tắt đồng thời cả PasswordAuthentication và PubkeyAuthentication (bạn sẽ bị khóa hoàn toàn khỏi SSH)."
          : "Cannot disable both PasswordAuthentication and PubkeyAuthentication simultaneously.",
      );
      return;
    }

    setApplying(true);
    setErrorMsg(null);
    try {
      const updated = await invoke<SshdConfigSnapshot>("vps_apply_sshd_config", {
        hostId: host.id,
        update: draft,
        sudoPassword: sudoPassword || null,
        withAutoRollback,
      });
      setSnapshot(updated);
      setDraft(snapshotToUpdate(updated));

      if (withAutoRollback) {
        setRollbackSecondsLeft(30);
        setRollbackModalOpen(true);
      } else {
        if (updated.port !== host.port) {
          await onSaveHost({ ...host, port: updated.port });
        }
        setToastMsg(
          isVi
            ? "Đã áp dụng cấu hình sshd_config và đồng bộ cổng máy chủ thành công!"
            : "Applied sshd_config and synchronized host port!",
        );
      }
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setApplying(false);
    }
  };

  const handleConfirmChanges = async () => {
    setConfirmingRollback(true);
    setErrorMsg(null);
    try {
      const confirmed = await invoke<SshdConfigSnapshot>(
        "vps_confirm_sshd_config",
        {
          hostId: host.id,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(confirmed);
      setDraft(snapshotToUpdate(confirmed));
      setRollbackModalOpen(false);
      if (confirmed.port !== host.port) {
        await onSaveHost({ ...host, port: confirmed.port });
      }
      setToastMsg(
        isVi
          ? "Đã xác nhận cấu hình SSH mới và hủy hẹn giờ hoàn tác!"
          : "Confirmed new SSH configuration and cancelled auto-rollback timer!",
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setConfirmingRollback(false);
    }
  };

  const handleRevertChanges = async () => {
    setConfirmingRollback(true);
    setErrorMsg(null);
    try {
      const reverted = await invoke<SshdConfigSnapshot>(
        "vps_revert_sshd_config",
        {
          hostId: host.id,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(reverted);
      setDraft(snapshotToUpdate(reverted));
      setRollbackModalOpen(false);
      setToastMsg(
        isVi
          ? "Đã hoàn tác về bản sao lưu /etc/ssh/sshd_config.sshbox.bak!"
          : "Reverted to backup /etc/ssh/sshd_config.sshbox.bak!",
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setConfirmingRollback(false);
    }
  };

  if (loading && !draft) {
    return (
      <div className="p-8 flex flex-col items-center justify-center gap-3 text-muted-foreground">
        <RefreshCw className="w-6 h-6 animate-spin text-primary" />
        <p className="text-xs font-medium">
          {isVi
            ? "Đang kiểm tra /etc/ssh/sshd_config và khóa authorized_keys..."
            : "Inspecting /etc/ssh/sshd_config and authorized_keys posture..."}
        </p>
      </div>
    );
  }

  const scoreVal = posture?.score ?? 0;
  const gaugeColor =
    scoreVal >= 80 ? "#10b981" : scoreVal >= 55 ? "#f59e0b" : "#f43f5e";
  const ringSize = 116;
  const ringStroke = 9;
  const ringRadius = (ringSize - ringStroke) / 2;
  const ringCircumference = 2 * Math.PI * ringRadius;
  const ringOffset =
    ringCircumference - (scoreVal / 100) * ringCircumference;

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
                  ? "Không thể thực thi tác vụ cấu hình SSH"
                  : "SSH Daemon Configuration Error"}
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

      {/* ZONE A: Top Security Posture Gauge + 3 One-Click Preset Cards */}
      {draft && snapshot && posture && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-4 items-stretch">
          {/* Posture Gauge & Rule Breakdown */}
          <div className="xl:col-span-7 rounded-2xl border border-border/80 bg-card/75 p-5 flex flex-col sm:flex-row items-center gap-5 shadow-xs">
            <div
              className="relative flex items-center justify-center shrink-0"
              style={{ width: ringSize, height: ringSize }}
            >
              <svg width={ringSize} height={ringSize} className="-rotate-90">
                <circle
                  cx={ringSize / 2}
                  cy={ringSize / 2}
                  r={ringRadius}
                  fill="transparent"
                  stroke="currentColor"
                  strokeWidth={ringStroke}
                  className="text-secondary"
                />
                <circle
                  cx={ringSize / 2}
                  cy={ringSize / 2}
                  r={ringRadius}
                  fill="transparent"
                  stroke={gaugeColor}
                  strokeWidth={ringStroke}
                  strokeDasharray={ringCircumference}
                  strokeDashoffset={ringOffset}
                  strokeLinecap="round"
                  className="transition-all duration-500 ease-out"
                />
              </svg>
              <div className="absolute flex flex-col items-center justify-center text-center">
                <span className="text-2xl font-bold font-mono text-foreground leading-none">
                  {scoreVal}
                </span>
                <span className="text-[10px] font-semibold uppercase tracking-wider text-muted-foreground mt-1">
                  / 100
                </span>
              </div>
            </div>

            <div className="flex-1 min-w-0 space-y-3">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div>
                  <div className="flex items-center gap-2">
                    <h2 className="text-sm font-bold text-foreground">
                      {isVi
                        ? "Chỉ số An toàn SSH Daemon (Posture Score)"
                        : "SSH Daemon Security Posture"}
                    </h2>
                    <span
                      className={`px-2 py-0.5 rounded-full text-[10px] font-bold uppercase tracking-wider border ${
                        scoreVal >= 80
                          ? "bg-emerald-500/15 text-emerald-400 border-emerald-500/30"
                          : scoreVal >= 55
                            ? "bg-amber-500/15 text-amber-400 border-amber-500/30"
                            : "bg-rose-500/15 text-rose-400 border-rose-500/30"
                      }`}
                    >
                      {scoreVal >= 80
                        ? isVi
                          ? "Cấp Pháo Đài"
                          : "Fortress"
                        : scoreVal >= 55
                          ? isVi
                            ? "Cân Bằng"
                            : "Balanced"
                          : isVi
                            ? "Cần Gia Cố"
                            : "Vulnerable"}
                    </span>
                  </div>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {isVi
                      ? `Điểm hiện tại trên máy chủ: ${snapshot.securityScore}/100 • Khóa SSH đã cài: ${snapshot.authorizedKeysCount}`
                      : `Server baseline: ${snapshot.securityScore}/100 • Installed authorized_keys: ${snapshot.authorizedKeysCount}`}
                  </p>
                </div>
              </div>

              <div className="flex flex-wrap gap-1.5">
                {posture.checks.map((chk) => (
                  <span
                    key={chk.id}
                    className={`inline-flex items-center gap-1 px-2 py-0.5 rounded-lg text-[11px] font-mono border ${
                      chk.passed
                        ? "bg-emerald-500/10 text-emerald-300 border-emerald-500/25"
                        : "bg-secondary/70 text-muted-foreground border-border/60"
                    }`}
                  >
                    {chk.passed ? (
                      <Check className="w-3 h-3 text-emerald-400 shrink-0" />
                    ) : (
                      <X className="w-3 h-3 text-muted-foreground/60 shrink-0" />
                    )}
                    <span>{chk.label}</span>
                  </span>
                ))}
              </div>
            </div>
          </div>

          {/* 3 One-Click Preset Cards */}
          <div className="xl:col-span-5 grid grid-cols-1 sm:grid-cols-3 gap-3">
            <button
              type="button"
              onClick={() => handleApplyPreset("fortress")}
              className="group rounded-2xl border border-emerald-500/30 bg-emerald-500/5 hover:bg-emerald-500/15 p-4 text-left flex flex-col justify-between gap-2 transition-all cursor-pointer"
            >
              <div className="flex items-center justify-between">
                <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
                  <ShieldCheck className="w-4 h-4" />
                </div>
                <span className="text-[10px] font-mono font-bold text-emerald-400 bg-emerald-500/15 px-2 py-0.5 rounded-full">
                  90–100 pts
                </span>
              </div>
              <div>
                <div className="text-xs font-bold text-foreground">
                  {isVi ? "Key-Only Fortress" : "Key-Only Fortress"}
                </div>
                <p className="text-[11px] text-muted-foreground mt-0.5 leading-snug">
                  {isVi
                    ? "Tắt mật khẩu & Root, chỉ nhận SSH Key, MaxAuthTries=3."
                    : "Disable passwords & root login, key-only auth, MaxTries=3."}
                </p>
              </div>
            </button>

            <button
              type="button"
              onClick={() => handleApplyPreset("balanced")}
              className="group rounded-2xl border border-sky-500/30 bg-sky-500/5 hover:bg-sky-500/15 p-4 text-left flex flex-col justify-between gap-2 transition-all cursor-pointer"
            >
              <div className="flex items-center justify-between">
                <div className="w-8 h-8 rounded-xl bg-sky-500/15 border border-sky-500/30 flex items-center justify-center text-sky-400">
                  <Sparkles className="w-4 h-4" />
                </div>
                <span className="text-[10px] font-mono font-bold text-sky-400 bg-sky-500/15 px-2 py-0.5 rounded-full">
                  Prohibit-PW
                </span>
              </div>
              <div>
                <div className="text-xs font-bold text-foreground">
                  {isVi ? "Balanced Production" : "Balanced Production"}
                </div>
                <p className="text-[11px] text-muted-foreground mt-0.5 leading-snug">
                  {isVi
                    ? "Root chỉ dùng Key, tắt X11 & mật khẩu rỗng, giữ Keepalive 300s."
                    : "Root key-only, X11 off, MaxTries=3, 300s keepalive."}
                </p>
              </div>
            </button>

            <button
              type="button"
              onClick={() => handleApplyPreset("reset")}
              className="group rounded-2xl border border-border/80 bg-card/75 hover:bg-accent/60 p-4 text-left flex flex-col justify-between gap-2 transition-all cursor-pointer"
            >
              <div className="flex items-center justify-between">
                <div className="w-8 h-8 rounded-xl bg-secondary border border-border flex items-center justify-center text-muted-foreground group-hover:text-foreground">
                  <RotateCcw className="w-4 h-4" />
                </div>
                <span className="text-[10px] font-mono text-muted-foreground">
                  Server
                </span>
              </div>
              <div>
                <div className="text-xs font-bold text-foreground">
                  {isVi ? "Reset to Server" : "Reset to Server"}
                </div>
                <p className="text-[11px] text-muted-foreground mt-0.5 leading-snug">
                  {isVi
                    ? "Hoàn lại mọi thay đổi nháp về cấu hình đang chạy trên VPS."
                    : "Discard unsaved edits and match live server config."}
                </p>
              </div>
            </button>
          </div>
        </div>
      )}

      {/* MAIN WORKSPACE: ZONE B (Center 3 Domain Policy Cards) + ZONE C (Right Live Diff & Safe Apply) */}
      {draft && snapshot && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-start">
          {/* ZONE B: 3 Visual Domain Policy Cards (7 cols on XL) */}
          <div className="xl:col-span-7 space-y-4">
            {/* Card 1: Network & Port Gateway */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
                    <Network className="w-4 h-4" />
                  </div>
                  <div>
                    <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                      {isVi
                        ? "1. Cổng Mạng & Giao Thức Lắng Nghe"
                        : "1. Network & Port Gateway"}
                    </h3>
                    <p className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Đổi cổng mặc định 22 giúp giảm hơn 95% lượng bot quét mật khẩu tự động."
                        : "Moving SSH off port 22 eliminates over 95% of automated scanner noise."}
                    </p>
                  </div>
                </div>
              </div>

              <div className="grid grid-cols-1 sm:grid-cols-2 gap-4 pt-1">
                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-muted-foreground block">
                    SSH Listen Port (`Port`)
                  </label>
                  <div className="flex items-center gap-2">
                    <input
                      type="number"
                      min={1}
                      max={65535}
                      value={draft.port}
                      onChange={(e) => {
                        const val = parseInt(e.target.value, 10);
                        setDraft({
                          ...draft,
                          port: Number.isFinite(val)
                            ? Math.min(65535, Math.max(1, val))
                            : 22,
                        });
                      }}
                      className="ui-input font-mono font-bold text-sm w-28"
                    />
                    <button
                      type="button"
                      onClick={handleGenerateRandomPort}
                      title="Generate random high port between 20000 and 60000"
                      className="h-[38px] px-3 rounded-xl bg-secondary/90 hover:bg-accent text-xs font-medium text-foreground border border-border inline-flex items-center gap-1.5 cursor-pointer transition-colors shrink-0"
                    >
                      <Dices className="w-3.5 h-3.5 text-cyan-400" />
                      <span>
                        {isVi ? "Cổng cao ngẫu nhiên" : "Random (20k–60k)"}
                      </span>
                    </button>
                  </div>
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs font-medium text-muted-foreground block">
                    IP Stack (`AddressFamily`)
                  </label>
                  <div className="grid grid-cols-3 gap-1.5 p-1 rounded-xl bg-secondary/70 border border-border/80">
                    {(
                      [
                        { id: "any", label: "Dual (any)" },
                        { id: "inet", label: "IPv4 Only" },
                        { id: "inet6", label: "IPv6 Only" },
                      ] as const
                    ).map((opt) => {
                      const active = draft.addressFamily === opt.id;
                      return (
                        <button
                          key={opt.id}
                          type="button"
                          onClick={() =>
                            setDraft({ ...draft, addressFamily: opt.id })
                          }
                          className={`py-1.5 px-2 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                            active
                              ? "bg-primary text-primary-foreground shadow-2xs"
                              : "text-muted-foreground hover:text-foreground"
                          }`}
                        >
                          {opt.label}
                        </button>
                      );
                    })}
                  </div>
                </div>
              </div>

              {draft.port !== snapshot.port && (
                <div className="rounded-xl border border-cyan-500/35 bg-cyan-500/10 p-3 flex items-start gap-2.5 text-xs text-cyan-200">
                  <Zap className="w-4 h-4 text-cyan-400 shrink-0 mt-0.5" />
                  <div>
                    <span className="font-semibold text-cyan-300">
                      {isVi
                        ? `Đồng bộ cổng tự động (${snapshot.port} → ${draft.port}): `
                        : `Automatic Port Sync (${snapshot.port} → ${draft.port}): `}
                    </span>
                    {isVi
                      ? `Khi áp dụng, ssh-box sẽ tự động mở cổng ${draft.port}/tcp trên UFW (nếu UFW đang bật) và cập nhật lại cổng kết nối của máy chủ "${host.alias || host.hostname}" trong danh bạ.`
                      : `When applied, ssh-box automatically runs 'ufw allow ${draft.port}/tcp' (if UFW is active) and updates "${host.alias || host.hostname}" port in your host list upon confirmation.`}
                  </div>
                </div>
              )}
            </section>

            {/* Card 2: Authentication Gate */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
                    <KeyRound className="w-4 h-4" />
                  </div>
                  <div>
                    <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                      {isVi
                        ? "2. Cổng Xác Thực & Quyền Root"
                        : "2. Authentication Gate & Root Policy"}
                    </h3>
                    <p className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Kiểm soát phương thức đăng nhập SSH và quyền truy cập trực tiếp vào tài khoản root."
                        : "Control SSH login mechanisms and direct root account access."}
                    </p>
                  </div>
                </div>
                <span className="px-2.5 py-1 rounded-full bg-secondary border border-border text-[11px] font-mono text-foreground">
                  authorized_keys: <strong>{snapshot.authorizedKeysCount}</strong>
                </span>
              </div>

              {/* 3-card selector for PermitRootLogin */}
              <div className="space-y-1.5">
                <label className="text-xs font-medium text-muted-foreground block">
                  Root Login Policy (`PermitRootLogin`)
                </label>
                <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                  {[
                    {
                      value: "no",
                      title: isVi ? "Chặn Root (no)" : "Disabled (no)",
                      desc: isVi
                        ? "Tắt hoàn toàn đăng nhập SSH trực tiếp bằng root"
                        : "Block all direct root SSH logins",
                      badge: "+20 pts",
                    },
                    {
                      value: "prohibit-password",
                      title: isVi
                        ? "Chỉ SSH Key"
                        : "Key-Only (prohibit-password)",
                      desc: isVi
                        ? "Cho phép root đăng nhập bằng khóa công khai SSH"
                        : "Allow root only via public key auth",
                      badge: "+12 pts",
                    },
                    {
                      value: "yes",
                      title: isVi ? "Cho phép (yes)" : "Full Root (yes)",
                      desc: isVi
                        ? "Cho phép root đăng nhập bằng cả mật khẩu (rủi ro cao)"
                        : "Allow password & key login for root",
                      badge: "Risk",
                    },
                  ].map((item) => {
                    const selected = draft.permitRootLogin === item.value;
                    return (
                      <button
                        key={item.value}
                        type="button"
                        onClick={() =>
                          setDraft({ ...draft, permitRootLogin: item.value })
                        }
                        className={`p-3 rounded-xl border text-left transition-all cursor-pointer flex flex-col justify-between gap-2 ${
                          selected
                            ? "border-primary bg-primary/12 shadow-xs"
                            : "border-border/80 bg-background/50 hover:bg-accent/50"
                        }`}
                      >
                        <div className="flex items-center justify-between gap-1">
                          <span className="text-xs font-bold text-foreground">
                            {item.title}
                          </span>
                          <span
                            className={`text-[10px] font-mono px-1.5 py-0.5 rounded-md ${
                              item.value === "yes"
                                ? "bg-rose-500/15 text-rose-400"
                                : "bg-emerald-500/15 text-emerald-400"
                            }`}
                          >
                            {item.badge}
                          </span>
                        </div>
                        <span className="text-[11px] text-muted-foreground leading-snug">
                          {item.desc}
                        </span>
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* Switch toggles */}
              <div className="divide-y divide-border/60 rounded-xl border border-border/75 bg-background/40 overflow-hidden">
                <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                  <div>
                    <div className="text-xs font-semibold text-foreground">
                      PubkeyAuthentication (`yes` / `no`)
                    </div>
                    <div className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Xác thực bằng cặp khóa bất đối xứng Ed25519 / RSA trong ~/.ssh/authorized_keys"
                        : "Authenticate using Ed25519 / RSA public keys in ~/.ssh/authorized_keys"}
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    {...({ switch: "" } as any)}
                    checked={draft.pubkeyAuthentication}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        pubkeyAuthentication: e.target.checked,
                      })
                    }
                  />
                </label>

                <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                  <div>
                    <div className="text-xs font-semibold text-foreground">
                      PasswordAuthentication (`yes` / `no`)
                    </div>
                    <div className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Cho phép đăng nhập bằng mật khẩu văn bản thuần (Nên tắt sau khi đã thêm SSH Key)"
                        : "Allow interactive cleartext password authentication (Disable after installing SSH keys)"}
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    {...({ switch: "" } as any)}
                    checked={draft.passwordAuthentication}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        passwordAuthentication: e.target.checked,
                      })
                    }
                  />
                </label>

                <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                  <div>
                    <div className="text-xs font-semibold text-foreground">
                      PermitEmptyPasswords (`yes` / `no`)
                    </div>
                    <div className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Cho phép tài khoản không có mật khẩu đăng nhập qua SSH (Luôn phải tắt)"
                        : "Allow accounts with empty password strings to log in (Must remain off)"}
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    {...({ switch: "" } as any)}
                    checked={draft.permitEmptyPasswords}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        permitEmptyPasswords: e.target.checked,
                      })
                    }
                  />
                </label>
              </div>

              {/* Smart Lockout Guard Warning */}
              {isLockoutRisk && (
                <div className="rounded-xl border border-amber-500/45 bg-amber-500/12 p-3.5 flex items-start gap-3 text-xs text-amber-200">
                  <AlertTriangle className="w-5 h-5 text-amber-400 shrink-0 mt-0.5" />
                  <div className="space-y-1">
                    <div className="font-bold text-amber-300">
                      {isVi
                        ? "Cảnh báo Chống Khóa Máy (Smart Lockout Guard)"
                        : "Smart Lockout Guard Warning"}
                    </div>
                    <p className="text-[11px] text-amber-100/90 leading-relaxed">
                      {!draft.passwordAuthentication &&
                      !draft.pubkeyAuthentication
                        ? isVi
                          ? "Bạn đang tắt cả PasswordAuthentication lẫn PubkeyAuthentication. Máy chủ sẽ từ chối mọi kết nối SSH mới!"
                          : "Both PasswordAuthentication and PubkeyAuthentication are disabled. All new SSH connections will be rejected!"
                        : isVi
                          ? "Bạn đang tắt PasswordAuthentication nhưng máy chủ hiện có 0 khóa trong ~/.ssh/authorized_keys. Hãy đẩy SSH Key lên máy chủ trước hoặc sử dụng chế độ 'Safe Apply (30s Auto-Rollback)'."
                          : "You disabled PasswordAuthentication while authorized_keys count is 0. Export an SSH key first or use 'Safe Apply (30s Auto-Rollback)'."}
                    </p>
                  </div>
                </div>
              )}
            </section>

            {/* Card 3: Session & Access Limits */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-xl bg-violet-500/15 border border-violet-500/30 flex items-center justify-center text-violet-400">
                  <Users className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                    {isVi
                      ? "3. Giới Hạn Phiên, AllowUsers & Chuyển Tiếp"
                      : "3. Session Limits, AllowUsers & Forwarding"}
                  </h3>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Giới hạn danh sách user được phép SSH, số lần thử mật khẩu và các kênh chuyển tiếp."
                      : "Restrict allowed SSH users, brute-force retry thresholds, and forwarding channels."}
                  </p>
                </div>
              </div>

              {/* AllowUsers chip-tag builder */}
              <div className="space-y-2">
                <div className="flex items-center justify-between gap-2">
                  <label className="text-xs font-medium text-muted-foreground">
                    Allowed SSH Users (`AllowUsers` —{" "}
                    {isVi
                      ? "Để trống = cho phép mọi user hợp lệ"
                      : "Empty = all valid users"}
                    )
                  </label>
                  <div className="flex items-center gap-1.5">
                    {host.user && !draft.allowUsers.includes(host.user) && (
                      <button
                        type="button"
                        onClick={() => handleAddAllowUser(host.user)}
                        className="px-2 py-0.5 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono text-primary border border-border cursor-pointer"
                      >
                        + {host.user}
                      </button>
                    )}
                    {privilegeInfo?.detectedClientIp &&
                      host.user &&
                      !draft.allowUsers.includes(
                        `${host.user}@${privilegeInfo.detectedClientIp}`,
                      ) && (
                        <button
                          type="button"
                          onClick={() =>
                            handleAddAllowUser(
                              `${host.user}@${privilegeInfo.detectedClientIp}`,
                            )
                          }
                          className="px-2 py-0.5 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono text-cyan-400 border border-border cursor-pointer"
                        >
                          + {host.user}@{privilegeInfo.detectedClientIp}
                        </button>
                      )}
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-1.5 min-h-[36px] p-2 rounded-xl bg-background/60 border border-border/80">
                  {draft.allowUsers.length === 0 && (
                    <span className="text-[11px] text-muted-foreground px-1">
                      {isVi
                        ? "Chưa giới hạn AllowUsers (mọi user hệ thống đều có thể thử đăng nhập)"
                        : "No AllowUsers restriction configured"}
                    </span>
                  )}
                  {draft.allowUsers.map((u) => (
                    <span
                      key={u}
                      className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-primary/15 border border-primary/35 text-xs font-mono text-foreground"
                    >
                      <span>{u}</span>
                      <button
                        type="button"
                        onClick={() => handleRemoveAllowUser(u)}
                        className="text-muted-foreground hover:text-rose-400 cursor-pointer"
                      >
                        <X className="w-3 h-3" />
                      </button>
                    </span>
                  ))}
                </div>

                <div className="flex items-center gap-2">
                  <input
                    type="text"
                    value={allowUserInput}
                    onChange={(e) => setAllowUserInput(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        handleAddAllowUser();
                      }
                    }}
                    placeholder={
                      isVi
                        ? "Nhập tên user hoặc user@203.0.113.5 rồi nhấn Enter..."
                        : "Enter username or user@203.0.113.5 and press Enter..."
                    }
                    className="ui-input font-mono text-xs flex-1"
                  />
                  <button
                    type="button"
                    onClick={() => handleAddAllowUser()}
                    className="h-[38px] px-3.5 rounded-xl bg-secondary hover:bg-accent text-xs font-semibold text-foreground border border-border inline-flex items-center gap-1.5 cursor-pointer shrink-0"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    <span>{isVi ? "Thêm" : "Add"}</span>
                  </button>
                </div>
              </div>

              {/* Numeric session limits */}
              <div className="grid grid-cols-2 sm:grid-cols-4 gap-3 pt-1">
                <div className="space-y-1">
                  <label className="text-[11px] font-medium text-muted-foreground block">
                    MaxAuthTries
                  </label>
                  <input
                    type="number"
                    min={1}
                    max={20}
                    value={draft.maxAuthTries}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        maxAuthTries: Math.max(
                          1,
                          Math.min(20, parseInt(e.target.value, 10) || 1),
                        ),
                      })
                    }
                    className="ui-input font-mono text-xs"
                  />
                </div>

                <div className="space-y-1">
                  <label className="text-[11px] font-medium text-muted-foreground block">
                    LoginGraceTime (s)
                  </label>
                  <input
                    type="number"
                    min={10}
                    max={600}
                    value={draft.loginGraceTime}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        loginGraceTime: Math.max(
                          10,
                          Math.min(600, parseInt(e.target.value, 10) || 60),
                        ),
                      })
                    }
                    className="ui-input font-mono text-xs"
                  />
                </div>

                <div className="space-y-1">
                  <label className="text-[11px] font-medium text-muted-foreground block">
                    AliveInterval (s)
                  </label>
                  <input
                    type="number"
                    min={0}
                    max={3600}
                    value={draft.clientAliveInterval}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        clientAliveInterval: Math.max(
                          0,
                          Math.min(3600, parseInt(e.target.value, 10) || 0),
                        ),
                      })
                    }
                    className="ui-input font-mono text-xs"
                  />
                </div>

                <div className="space-y-1">
                  <label className="text-[11px] font-medium text-muted-foreground block">
                    AliveCountMax
                  </label>
                  <input
                    type="number"
                    min={0}
                    max={20}
                    value={draft.clientAliveCountMax}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        clientAliveCountMax: Math.max(
                          0,
                          Math.min(20, parseInt(e.target.value, 10) || 0),
                        ),
                      })
                    }
                    className="ui-input font-mono text-xs"
                  />
                </div>
              </div>

              {/* Forwarding switches */}
              <div className="divide-y divide-border/60 rounded-xl border border-border/75 bg-background/40 overflow-hidden">
                <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                  <div>
                    <div className="text-xs font-semibold text-foreground">
                      X11Forwarding
                    </div>
                    <div className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Chuyển tiếp giao diện đồ họa X11 (Khuyến nghị tắt trên máy chủ VPS)"
                        : "Forward X11 graphical windows (Recommended off on headless servers)"}
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    {...({ switch: "" } as any)}
                    checked={draft.x11Forwarding}
                    onChange={(e) =>
                      setDraft({ ...draft, x11Forwarding: e.target.checked })
                    }
                  />
                </label>

                <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                  <div>
                    <div className="text-xs font-semibold text-foreground">
                      AllowTcpForwarding
                    </div>
                    <div className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Cho phép tạo SSH Tunnel / Port Forwarding qua máy chủ này"
                        : "Allow local/remote SSH port forwarding tunnels"}
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    {...({ switch: "" } as any)}
                    checked={draft.allowTcpForwarding}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        allowTcpForwarding: e.target.checked,
                      })
                    }
                  />
                </label>

                <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                  <div>
                    <div className="text-xs font-semibold text-foreground">
                      AllowAgentForwarding
                    </div>
                    <div className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Cho phép chuyển tiếp khóa SSH Agent từ máy trạm vào phiên SSH"
                        : "Allow forwarding local ssh-agent socket into remote sessions"}
                    </div>
                  </div>
                  <input
                    type="checkbox"
                    {...({ switch: "" } as any)}
                    checked={draft.allowAgentForwarding}
                    onChange={(e) =>
                      setDraft({
                        ...draft,
                        allowAgentForwarding: e.target.checked,
                      })
                    }
                  />
                </label>
              </div>
            </section>
          </div>

          {/* ZONE C: Live sshd_config Diff Inspector & Safe Apply Engine (5 cols on XL) */}
          <div className="xl:col-span-5 space-y-4 sticky top-4">
            <section className="rounded-2xl border border-border/80 bg-card/85 p-5 space-y-4 shadow-xs">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <FileDiff className="w-4 h-4 text-primary" />
                  <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                    {isVi
                      ? "Kiểm tra Diff /etc/ssh/sshd_config"
                      : "Live /etc/ssh/sshd_config Diff"}
                  </h3>
                </div>
                <div className="flex items-center gap-2">
                  <span
                    className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-semibold ${
                      changedEntries.length > 0
                        ? "bg-amber-500/15 text-amber-300 border border-amber-500/30"
                        : "bg-secondary text-muted-foreground"
                    }`}
                  >
                    {changedEntries.length}{" "}
                    {isVi ? "thay đổi" : "modified"}
                  </span>
                  <button
                    type="button"
                    onClick={() => setShowRawConfig((p) => !p)}
                    className="px-2 py-0.5 rounded-lg bg-secondary hover:bg-accent text-[11px] font-mono text-muted-foreground hover:text-foreground border border-border cursor-pointer"
                  >
                    {showRawConfig ? "Diff View" : "Raw File"}
                  </button>
                </div>
              </div>

              {showRawConfig ? (
                <pre className="p-3 rounded-xl bg-background/90 border border-border/80 font-mono text-[11px] text-muted-foreground max-h-[380px] overflow-auto whitespace-pre">
                  {snapshot.rawConfig || "# Empty /etc/ssh/sshd_config"}
                </pre>
              ) : (
                <div className="rounded-xl bg-background/90 border border-border/80 p-3 font-mono text-xs space-y-1.5 max-h-[380px] overflow-y-auto">
                  {changedEntries.length === 0 ? (
                    <div className="py-6 text-center text-xs text-muted-foreground font-sans space-y-1">
                      <div>
                        {isVi
                          ? "Không có chỉ thị nào thay đổi so với /etc/ssh/sshd_config hiện tại."
                          : "No directive changes compared to live /etc/ssh/sshd_config."}
                      </div>
                      <div className="text-[11px] text-muted-foreground/70">
                        {isVi
                          ? "Điều chỉnh các chính sách bên trái hoặc chọn Preset để xem trước bản vá."
                          : "Modify policies on the left or pick a preset to preview diff."}
                      </div>
                    </div>
                  ) : (
                    changedEntries.map((d) => (
                      <div
                        key={d.directive}
                        className="rounded-lg overflow-hidden border border-border/50"
                      >
                        {d.before !== null && (
                          <div className="px-2.5 py-1 bg-rose-500/15 text-rose-300 flex items-center gap-2">
                            <span className="text-rose-400 font-bold select-none">
                              -
                            </span>
                            <span>
                              {d.directive} {d.before}
                            </span>
                          </div>
                        )}
                        {d.after !== null && (
                          <div className="px-2.5 py-1 bg-emerald-500/15 text-emerald-300 flex items-center gap-2">
                            <span className="text-emerald-400 font-bold select-none">
                              +
                            </span>
                            <span>
                              {d.directive} {d.after}
                            </span>
                          </div>
                        )}
                      </div>
                    ))
                  )}

                  {/* Unchanged directives summary */}
                  <div className="pt-2 border-t border-border/50 space-y-1">
                    <div className="text-[10px] uppercase tracking-wider text-muted-foreground/70 font-sans">
                      {isVi
                        ? "Chỉ thị đang hiệu lực (Không đổi)"
                        : "Active Directives (Unchanged)"}
                    </div>
                    {diffEntries
                      .filter((d) => !d.changed && d.after !== null)
                      .map((d) => (
                        <div
                          key={d.directive}
                          className="px-2 py-0.5 text-[11px] text-muted-foreground flex items-center justify-between"
                        >
                          <span>{d.directive}</span>
                          <span className="text-foreground/80">{d.after}</span>
                        </div>
                      ))}
                  </div>
                </div>
              )}

              {/* Validation & Rollback Assurance Box */}
              <div className="rounded-xl bg-secondary/50 border border-border/70 p-3 space-y-1.5 text-[11px] text-muted-foreground">
                <div className="flex items-center gap-1.5 font-semibold text-foreground">
                  <Shield className="w-3.5 h-3.5 text-emerald-400" />
                  <span>
                    {isVi
                      ? "Quy trình Bảo vệ 3 Lớp (Zero-Lockout Pipeline)"
                      : "3-Stage Zero-Lockout Verification Pipeline"}
                  </span>
                </div>
                <ul className="list-disc list-inside space-y-0.5 text-[11px]">
                  <li>
                    {isVi
                      ? "Sao lưu tự động ra /etc/ssh/sshd_config.sshbox.bak"
                      : "Backs up current config to /etc/ssh/sshd_config.sshbox.bak"}
                  </li>
                  <li>
                    {isVi
                      ? "Kiểm tra cú pháp bằng `sshd -t -f` trước khi ghi đè"
                      : "Validates syntax via `sshd -t -f` before replacing file"}
                  </li>
                  <li>
                    {isVi
                      ? "Hẹn giờ tự động hoàn tác sau 30 giây nếu bạn mất kết nối"
                      : "Spawns a 30s background auto-rollback timer until confirmed"}
                  </li>
                </ul>
              </div>

              {/* Action Buttons */}
              <div className="space-y-2 pt-1">
                <button
                  type="button"
                  disabled={applying || changedEntries.length === 0}
                  onClick={() => handleApplyConfig(true)}
                  className="w-full py-2.5 px-4 rounded-xl bg-primary text-primary-foreground text-xs font-bold shadow-sm hover:opacity-95 disabled:opacity-50 transition-all cursor-pointer flex items-center justify-center gap-2"
                >
                  {applying ? (
                    <RefreshCw className="w-4 h-4 animate-spin" />
                  ) : (
                    <ShieldCheck className="w-4 h-4" />
                  )}
                  <span>
                    {isVi
                      ? "Safe Apply (Tự động hoàn tác sau 30s)"
                      : "Safe Apply (30s Auto-Rollback)"}
                  </span>
                </button>

                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    disabled={applying || changedEntries.length === 0}
                    onClick={() => handleApplyConfig(false)}
                    className="flex-1 py-2 px-3 rounded-xl bg-secondary hover:bg-accent text-foreground text-xs font-semibold border border-border disabled:opacity-50 transition-colors cursor-pointer"
                  >
                    {isVi ? "Áp dụng ngay" : "Apply Immediately"}
                  </button>

                  <button
                    type="button"
                    disabled={applying || confirmingRollback}
                    onClick={handleRevertChanges}
                    title="Restore /etc/ssh/sshd_config.sshbox.bak"
                    className="py-2 px-3 rounded-xl bg-secondary/70 hover:bg-rose-500/15 hover:text-rose-300 hover:border-rose-500/35 text-muted-foreground text-xs font-medium border border-border transition-colors cursor-pointer inline-flex items-center gap-1.5"
                  >
                    <Undo2 className="w-3.5 h-3.5" />
                    <span>{isVi ? "Khôi phục .bak" : "Restore .bak"}</span>
                  </button>
                </div>
              </div>
            </section>
          </div>
        </div>
      )}

      {/* 30-SECOND ANTI-LOCKOUT CONFIRMATION MODAL */}
      {rollbackModalOpen && (
        <div className="fixed inset-0 z-50 bg-black/70 backdrop-blur-xs flex items-center justify-center p-4 ui-modal-backdrop">
          <div className="w-full max-w-md rounded-2xl bg-card border border-amber-500/40 p-6 shadow-2xl space-y-5 ui-modal-content">
            <div className="flex items-center gap-4">
              <div className="relative w-16 h-16 flex items-center justify-center shrink-0">
                <svg width={64} height={64} className="-rotate-90">
                  <circle
                    cx={32}
                    cy={32}
                    r={27}
                    fill="transparent"
                    stroke="currentColor"
                    strokeWidth={5}
                    className="text-secondary"
                  />
                  <circle
                    cx={32}
                    cy={32}
                    r={27}
                    fill="transparent"
                    stroke="#f59e0b"
                    strokeWidth={5}
                    strokeDasharray={2 * Math.PI * 27}
                    strokeDashoffset={
                      2 * Math.PI * 27 * (1 - rollbackSecondsLeft / 30)
                    }
                    strokeLinecap="round"
                    className="transition-all duration-700"
                  />
                </svg>
                <span className="absolute font-mono text-sm font-bold text-amber-400">
                  {rollbackSecondsLeft}s
                </span>
              </div>

              <div>
                <div className="inline-flex items-center gap-1.5 px-2 py-0.5 rounded-full bg-amber-500/15 text-amber-400 text-[10px] font-bold uppercase tracking-wider mb-1">
                  <Clock className="w-3 h-3" />
                  <span>Anti-Lockout Guard Active</span>
                </div>
                <h3 className="text-sm font-bold text-foreground">
                  {isVi
                    ? "Xác nhận kết nối SSH vẫn hoạt động"
                    : "Confirm SSH Connectivity"}
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  {isVi
                    ? `Nếu bạn không xác nhận trong ${rollbackSecondsLeft} giây, máy chủ sẽ tự động khôi phục lại sshd_config cũ.`
                    : `If not confirmed within ${rollbackSecondsLeft}s, the server will automatically revert to the backup sshd_config.`}
                </p>
              </div>
            </div>

            {snapshot && snapshot.port !== host.port && (
              <div className="p-3 rounded-xl bg-cyan-500/10 border border-cyan-500/30 text-xs text-cyan-200">
                {isVi
                  ? `Khi nhấn Xác nhận, cổng máy chủ trong ssh-box cũng sẽ được cập nhật từ ${host.port} sang ${snapshot.port}.`
                  : `Confirming will also update this host's saved SSH port from ${host.port} to ${snapshot.port}.`}
              </div>
            )}

            <div className="flex items-center justify-end gap-2.5 pt-2">
              <button
                type="button"
                disabled={confirmingRollback}
                onClick={handleRevertChanges}
                className="px-4 py-2.5 rounded-xl bg-rose-500/15 hover:bg-rose-500/25 text-rose-300 border border-rose-500/35 text-xs font-semibold cursor-pointer transition-colors inline-flex items-center gap-1.5"
              >
                <Undo2 className="w-3.5 h-3.5" />
                <span>{isVi ? "Hoàn tác ngay" : "Revert Now"}</span>
              </button>

              <button
                type="button"
                disabled={confirmingRollback}
                onClick={handleConfirmChanges}
                className="px-5 py-2.5 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 text-xs font-bold shadow-md cursor-pointer transition-all inline-flex items-center gap-1.5"
              >
                {confirmingRollback ? (
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Lock className="w-3.5 h-3.5" />
                )}
                <span>
                  {isVi ? "Xác nhận & Giữ cấu hình" : "Confirm & Keep Changes"}
                </span>
              </button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
};
