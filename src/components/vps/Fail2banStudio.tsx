import React, { useCallback, useEffect, useId, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  Ban,
  CheckCircle2,
  Clock,
  Download,
  KeyRound,
  Lock,
  Radar,
  RefreshCw,
  Search,
  Shield,
  ShieldAlert,
  ShieldCheck,
  ShieldOff,
  Sliders,
  Unlock,
  UserX,
  X,
  Zap,
} from "lucide-react";
import {
  Fail2banSnapshot,
  HostEntry,
  SshAuthAuditEvent,
  VpsPrivilegeInfo,
} from "../../types";
import { AppLanguage } from "../../themes";

interface Fail2banStudioProps {
  host: HostEntry;
  sudoPassword?: string;
  privilegeInfo: VpsPrivilegeInfo | null;
  language?: AppLanguage;
  onRequireSudoUnlock?: () => void;
}

type AuthEventFilter = "all" | "accepted" | "failed" | "invalid_user";

export const Fail2banStudio: React.FC<Fail2banStudioProps> = ({
  host,
  sudoPassword,
  privilegeInfo,
  language = "vi",
  onRequireSudoUnlock,
}) => {
  const isVi = language === "vi";
  const timelineCaptionId = useId();

  const [snapshot, setSnapshot] = useState<Fail2banSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  // Interactive Jail Policy Tuner state
  const [maxretry, setMaxretry] = useState<number>(5);
  const [findtimeSecs, setFindtimeSecs] = useState<number>(600);
  const [bantimeSecs, setBantimeSecs] = useState<number>(3600);
  const [ignoreIps, setIgnoreIps] = useState<string[]>([]);
  const [newIgnoreIp, setNewIgnoreIp] = useState<string>("");

  // Zone 2: Quarantine Vault search & manual ban input
  const [quarantineSearch, setQuarantineSearch] = useState<string>("");
  const [manualBanIp, setManualBanIp] = useState<string>("");
  const [selectedJail, setSelectedJail] = useState<string>("sshd");

  // Zone 3: Auth Forensics Timeline filter & search
  const [eventFilter, setEventFilter] = useState<AuthEventFilter>("all");
  const [eventSearch, setEventSearch] = useState<string>("");

  const syncPolicyFromSnapshot = useCallback((snap: Fail2banSnapshot) => {
    const sshdJail =
      snap.jails.find((j) => j.name === "sshd") ?? snap.jails[0] ?? null;
    if (sshdJail) {
      setMaxretry(sshdJail.maxretry || 5);
      setFindtimeSecs(sshdJail.findtimeSecs || 600);
      setBantimeSecs(sshdJail.bantimeSecs ?? 3600);
      setSelectedJail(sshdJail.name);
    }
    setIgnoreIps(snap.ignoreIps ?? []);
  }, []);

  const fetchSnapshot = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const res = await invoke<Fail2banSnapshot>("vps_get_fail2ban_snapshot", {
        hostId: host.id,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(res);
      syncPolicyFromSnapshot(res);
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setLoading(false);
    }
  }, [host.id, sudoPassword, onRequireSudoUnlock, syncPolicyFromSnapshot]);

  useEffect(() => {
    fetchSnapshot();
  }, [fetchSnapshot]);

  useEffect(() => {
    if (!toastMsg) return;
    const timer = setTimeout(() => setToastMsg(null), 4500);
    return () => clearTimeout(timer);
  }, [toastMsg]);

  const effectiveClientIp = useMemo(() => {
    if (snapshot?.detectedClientIp?.trim()) {
      return snapshot.detectedClientIp.trim();
    }
    if (privilegeInfo?.detectedClientIp?.trim()) {
      return privilegeInfo.detectedClientIp.trim();
    }
    return "";
  }, [snapshot?.detectedClientIp, privilegeInfo?.detectedClientIp]);

  const handleInstallFail2ban = async () => {
    setBusyAction("install-f2b");
    setErrorMsg(null);
    try {
      const updated = await invoke<Fail2banSnapshot>("vps_install_fail2ban", {
        hostId: host.id,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(updated);
      syncPolicyFromSnapshot(updated);
      setToastMsg(
        isVi
          ? "Đã cài đặt và kích hoạt Fail2ban (jail sshd) thành công!"
          : "Installed and enabled Fail2ban (sshd jail) successfully!",
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setBusyAction(null);
    }
  };

  const handleBanOrUnbanIp = async (
    jail: string,
    ip: string,
    action: "ban" | "unban",
  ) => {
    const cleanIp = ip.trim();
    if (!cleanIp) return;
    const targetJail = jail.trim() || "sshd";
    setBusyAction(`${action}-${cleanIp}`);
    setErrorMsg(null);
    try {
      const updated = await invoke<Fail2banSnapshot>(
        "vps_fail2ban_ban_action",
        {
          hostId: host.id,
          jail: targetJail,
          ip: cleanIp,
          action,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(updated);
      setToastMsg(
        action === "ban"
          ? isVi
            ? `Đã cách ly (Ban) địa chỉ IP ${cleanIp} trên jail [${targetJail}].`
            : `Quarantined (Banned) IP ${cleanIp} in jail [${targetJail}].`
          : isVi
            ? `Đã gỡ lệnh cấm (Unban) cho địa chỉ IP ${cleanIp} khỏi jail [${targetJail}].`
            : `Unbanned IP ${cleanIp} from jail [${targetJail}].`,
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setBusyAction(null);
    }
  };

  const handleManualBanSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    const clean = manualBanIp.trim();
    if (!clean) return;
    await handleBanOrUnbanIp(selectedJail || "sshd", clean, "ban");
    setManualBanIp("");
  };

  const handleApplyPolicy = async (e: React.FormEvent) => {
    e.preventDefault();
    setBusyAction("apply-policy");
    setErrorMsg(null);
    try {
      const updated = await invoke<Fail2banSnapshot>(
        "vps_update_fail2ban_jail",
        {
          hostId: host.id,
          maxretry,
          findtimeSecs,
          bantimeSecs,
          ignoreIps,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(updated);
      syncPolicyFromSnapshot(updated);
      setToastMsg(
        isVi
          ? `Đã lưu cấu hình /etc/fail2ban/jail.d/99-sshbox.local và nạp lại Fail2ban!`
          : `Saved /etc/fail2ban/jail.d/99-sshbox.local and reloaded Fail2ban!`,
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setBusyAction(null);
    }
  };

  const handleAddIgnoreIp = (ipToAdd: string) => {
    const clean = ipToAdd.trim();
    if (!clean) return;
    if (!ignoreIps.includes(clean)) {
      setIgnoreIps((prev) => [...prev, clean]);
    }
    setNewIgnoreIp("");
  };

  const handleRemoveIgnoreIp = (ipToRemove: string) => {
    setIgnoreIps((prev) => prev.filter((ip) => ip !== ipToRemove));
  };

  const applyPolicyPreset = (preset: "strict" | "balanced" | "permanent") => {
    if (preset === "strict") {
      setMaxretry(3);
      setFindtimeSecs(900); // 15m
      setBantimeSecs(86400); // 24h
    } else if (preset === "balanced") {
      setMaxretry(5);
      setFindtimeSecs(600); // 10m
      setBantimeSecs(3600); // 1h
    } else {
      setMaxretry(3);
      setFindtimeSecs(604800); // 7d window
      setBantimeSecs(-1); // Permanent
    }
  };

  // Flatten all banned IPs across active jails
  const quarantinedEntries = useMemo(() => {
    const entries: { ip: string; jail: string }[] = [];
    for (const j of snapshot?.jails ?? []) {
      for (const ip of j.bannedIps) {
        entries.push({ ip, jail: j.name });
      }
    }
    return entries;
  }, [snapshot?.jails]);

  const bannedIpSet = useMemo(
    () => new Set(quarantinedEntries.map((e) => e.ip)),
    [quarantinedEntries],
  );

  const filteredQuarantined = useMemo(() => {
    const q = quarantineSearch.trim().toLowerCase();
    if (!q) return quarantinedEntries;
    return quarantinedEntries.filter(
      (e) =>
        e.ip.toLowerCase().includes(q) || e.jail.toLowerCase().includes(q),
    );
  }, [quarantinedEntries, quarantineSearch]);

  const totalActiveBans = useMemo(
    () =>
      (snapshot?.jails ?? []).reduce(
        (acc, j) => acc + Math.max(j.currentlyBanned, j.bannedIps.length),
        0,
      ),
    [snapshot?.jails],
  );

  const totalFailedCount = useMemo(() => {
    const fromJails = (snapshot?.jails ?? []).reduce(
      (acc, j) => acc + j.totalFailed,
      0,
    );
    const fromLogs = (snapshot?.authEvents ?? []).filter(
      (ev) => ev.eventType !== "accepted",
    ).length;
    return Math.max(fromJails, fromLogs);
  }, [snapshot?.jails, snapshot?.authEvents]);

  // Display auth events in reverse chronological order (newest at top)
  const reversedAuthEvents = useMemo(
    () => [...(snapshot?.authEvents ?? [])].reverse(),
    [snapshot?.authEvents],
  );

  const eventCounts = useMemo(() => {
    const list = snapshot?.authEvents ?? [];
    return {
      all: list.length,
      accepted: list.filter((e) => e.eventType === "accepted").length,
      failed: list.filter((e) => e.eventType === "failed").length,
      invalid_user: list.filter((e) => e.eventType === "invalid_user").length,
    };
  }, [snapshot?.authEvents]);

  const filteredAuthEvents = useMemo(() => {
    const q = eventSearch.trim().toLowerCase();
    return reversedAuthEvents.filter((ev) => {
      if (eventFilter !== "all" && ev.eventType !== eventFilter) {
        return false;
      }
      if (!q) return true;
      return (
        ev.ip.toLowerCase().includes(q) ||
        ev.username.toLowerCase().includes(q) ||
        ev.method.toLowerCase().includes(q) ||
        ev.timestamp.toLowerCase().includes(q)
      );
    });
  }, [reversedAuthEvents, eventFilter, eventSearch]);

  const maxAttackerCount = useMemo(() => {
    const top = snapshot?.topAttackerIps?.[0]?.failedCount ?? 1;
    return Math.max(top, 1);
  }, [snapshot?.topAttackerIps]);

  const formatBantimeLabel = (secs: number) => {
    if (secs < 0) return isVi ? "Vĩnh viễn (-1)" : "Permanent (-1)";
    if (secs >= 86400 && secs % 86400 === 0) return `${secs / 86400}d`;
    if (secs >= 3600 && secs % 3600 === 0) return `${secs / 3600}h`;
    if (secs >= 60) return `${Math.round(secs / 60)}m`;
    return `${secs}s`;
  };

  const renderEventBadge = (ev: SshAuthAuditEvent) => {
    if (ev.eventType === "accepted") {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/15 text-emerald-300 border border-emerald-500/35">
          <CheckCircle2 className="w-2.5 h-2.5 text-emerald-400" />
          <span>ACCEPTED</span>
        </span>
      );
    }
    if (ev.eventType === "invalid_user") {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/15 text-amber-300 border border-amber-500/35">
          <UserX className="w-2.5 h-2.5 text-amber-400" />
          <span>INVALID USER</span>
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-rose-500/15 text-rose-300 border border-rose-500/35">
        <ShieldAlert className="w-2.5 h-2.5 text-rose-400" />
        <span>FAILED PASSWORD</span>
      </span>
    );
  };

  const renderMethodPill = (method: string) => {
    if (method === "publickey") {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-cyan-500/15 border border-cyan-500/30 text-[10px] font-mono font-bold text-cyan-300">
          <KeyRound className="w-2.5 h-2.5 text-cyan-400" />
          <span>publickey</span>
        </span>
      );
    }
    if (method === "password") {
      return (
        <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/30 text-[10px] font-mono font-bold text-amber-300">
          <Lock className="w-2.5 h-2.5 text-amber-400" />
          <span>password</span>
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-secondary border border-border text-[10px] font-mono text-muted-foreground">
        <span>{method}</span>
      </span>
    );
  };

  if (loading && !snapshot) {
    return (
      <div className="p-8 flex flex-col items-center justify-center gap-3 text-muted-foreground">
        <RefreshCw className="w-6 h-6 animate-spin text-primary" />
        <p className="text-xs font-medium">
          {isVi
            ? "Đang phân tích Fail2ban Jails và nhật ký xác thực SSH (/var/log/auth.log)..."
            : "Scanning Fail2ban jails and live SSH authentication forensic logs (/var/log/auth.log)..."}
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
                  ? "Không thể thực thi tác vụ Fail2ban"
                  : "Fail2ban Operation Failed"}
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

      {/* ZERO-STATE ONBOARDING HERO CARD (when !snapshot.installed) */}
      {snapshot && !snapshot.installed && (
        <section className="rounded-2xl border border-rose-500/40 bg-rose-500/8 p-6 flex flex-col lg:flex-row items-start lg:items-center justify-between gap-5 shadow-xs">
          <div className="flex items-start gap-4">
            <div className="w-12 h-12 rounded-2xl bg-rose-500/15 border border-rose-500/35 flex items-center justify-center text-rose-400 shrink-0">
              <ShieldOff className="w-6 h-6" />
            </div>
            <div className="space-y-2 max-w-2xl">
              <div className="flex flex-wrap items-center gap-2">
                <span className="px-2.5 py-0.5 rounded-full bg-rose-500/20 border border-rose-500/40 text-[10px] font-mono font-bold uppercase text-rose-300">
                  {isVi ? "CHƯA CÀI ĐẶT FAIL2BAN" : "FAIL2BAN NOT INSTALLED"}
                </span>
                <span className="px-2.5 py-0.5 rounded-full bg-amber-500/15 border border-amber-500/35 text-[10px] font-mono font-bold text-amber-300">
                  {isVi
                    ? `Phát hiện ${eventCounts.failed + eventCounts.invalid_user} lần đăng nhập SSH thất bại trực tiếp`
                    : `${eventCounts.failed + eventCounts.invalid_user} live failed SSH login attempts detected in auth logs`}
                </span>
              </div>
              <h2 className="text-sm font-bold text-foreground">
                {isVi
                  ? "Bảo vệ máy chủ khỏi tấn công dò mật khẩu SSH (Brute-Force Intrusion Prevention)"
                  : "Protect SSH Against Automated Brute-Force Dictionary Attacks"}
              </h2>
              <p className="text-xs text-muted-foreground leading-relaxed">
                {isVi
                  ? "Botnet trên Internet liên tục quét cổng SSH để thử hàng nghìn mật khẩu mỗi phút. Cài đặt Fail2ban chỉ với 1-Click để tự động phát hiện và khóa IP kẻ tấn công vào tường lửa ngay khi vượt ngưỡng thử sai."
                  : "Internet scanners continuously probe SSH ports with credential stuffing. Install and activate Fail2ban with 1 click to automatically quarantine offending IPs at the firewall level."}
              </p>
            </div>
          </div>

          <div className="flex items-center gap-2.5 shrink-0 w-full lg:w-auto">
            <button
              type="button"
              disabled={busyAction !== null}
              onClick={handleInstallFail2ban}
              className="flex-1 lg:flex-initial py-2.5 px-4 rounded-xl bg-emerald-500 hover:bg-emerald-400 text-slate-950 text-xs font-bold inline-flex items-center justify-center gap-2 cursor-pointer disabled:opacity-50 transition-colors shadow-sm"
            >
              {busyAction === "install-f2b" ? (
                <RefreshCw className="w-4 h-4 animate-spin" />
              ) : (
                <Download className="w-4 h-4" />
              )}
              <span>
                {busyAction === "install-f2b"
                  ? isVi
                    ? "Đang cài đặt & kích hoạt Fail2ban..."
                    : "Installing & Enabling Fail2ban..."
                  : isVi
                    ? "Cài đặt & Bật Fail2ban (sshd Jail)"
                    : "Install & Enable Fail2ban (sshd Jail)"}
              </span>
            </button>

            <button
              type="button"
              onClick={fetchSnapshot}
              disabled={loading || busyAction !== null}
              title="Refresh logs"
              className="p-2.5 rounded-xl bg-secondary hover:bg-accent border border-border text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
            >
              <RefreshCw
                className={`w-4 h-4 ${loading ? "animate-spin text-primary" : ""}`}
              />
            </button>
          </div>
        </section>
      )}

      {/* ZONE 1: Top Deck — Jail Health Cards & Interactive Policy Tuner */}
      {snapshot && snapshot.installed && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-4 items-stretch">
          {/* Left: Service Status & Active Jails Telemetry Deck (5 cols) */}
          <div className="xl:col-span-5 rounded-2xl border border-border/80 bg-card/75 p-4 flex flex-col justify-between gap-4 shadow-xs">
            <div className="flex items-start justify-between gap-3">
              <div className="flex items-center gap-3">
                <div
                  className={`w-10 h-10 rounded-2xl border flex items-center justify-center shrink-0 ${
                    snapshot.running
                      ? "bg-emerald-500/15 border-emerald-500/35 text-emerald-400"
                      : "bg-rose-500/15 border-rose-500/35 text-rose-400"
                  }`}
                >
                  {snapshot.running ? (
                    <ShieldCheck className="w-5 h-5" />
                  ) : (
                    <ShieldAlert className="w-5 h-5" />
                  )}
                </div>
                <div>
                  <div className="flex flex-wrap items-center gap-2">
                    <span className="text-xs font-bold uppercase tracking-wider text-foreground">
                      Fail2ban IPS
                    </span>
                    <span
                      className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-bold uppercase border ${
                        snapshot.running
                          ? "bg-emerald-500/15 text-emerald-400 border-emerald-500/35"
                          : "bg-rose-500/15 text-rose-400 border-rose-500/35"
                      }`}
                    >
                      {snapshot.running ? "RUNNING" : "STOPPED"}
                    </span>
                  </div>
                  <p className="text-[11px] text-muted-foreground mt-0.5">
                    {isVi
                      ? "Giám sát nhật ký xác thực và tự động cách ly IP tấn công."
                      : "Real-time intrusion prevention and automated IP quarantine."}
                  </p>
                </div>
              </div>

              <button
                type="button"
                onClick={fetchSnapshot}
                disabled={loading || busyAction !== null}
                title="Refresh Fail2ban & Auth Logs"
                className="p-2 rounded-xl bg-secondary hover:bg-accent border border-border text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
              >
                <RefreshCw
                  className={`w-3.5 h-3.5 ${loading ? "animate-spin text-primary" : ""}`}
                />
              </button>
            </div>

            {/* KPI Counters */}
            <div className="grid grid-cols-3 gap-2.5">
              <div className="rounded-xl bg-background/60 border border-border/70 p-3">
                <div className="text-[10px] font-mono uppercase text-muted-foreground">
                  {isVi ? "IP Đang Bị Khóa" : "Active Bans"}
                </div>
                <div className="text-lg font-mono font-bold text-rose-400 mt-0.5">
                  {totalActiveBans}
                </div>
              </div>

              <div className="rounded-xl bg-background/60 border border-border/70 p-3">
                <div className="text-[10px] font-mono uppercase text-muted-foreground">
                  {isVi ? "Lần Thử Thất Bại" : "Failed Attempts"}
                </div>
                <div className="text-lg font-mono font-bold text-amber-400 mt-0.5">
                  {totalFailedCount}
                </div>
              </div>

              <div className="rounded-xl bg-background/60 border border-border/70 p-3">
                <div className="text-[10px] font-mono uppercase text-muted-foreground">
                  {isVi ? "Jails Hoạt Động" : "Active Jails"}
                </div>
                <div className="text-lg font-mono font-bold text-emerald-400 mt-0.5">
                  {snapshot.jails.length}
                </div>
              </div>
            </div>

            {/* Active Jails Deck */}
            <div className="space-y-1.5">
              <div className="text-[10px] font-mono uppercase tracking-wider text-muted-foreground">
                {isVi ? "Danh sách Jails đang bật" : "Active Jails Deck"}
              </div>
              <div className="flex flex-wrap gap-2">
                {snapshot.jails.length === 0 ? (
                  <span className="text-xs text-muted-foreground">
                    {isVi
                      ? "Chưa có jail nào kích hoạt. Lưu cấu hình bên phải để bật [sshd]."
                      : "No active jails reported. Apply policy on the right to enable [sshd]."}
                  </span>
                ) : (
                  snapshot.jails.map((j) => (
                    <div
                      key={j.name}
                      className="px-3 py-1.5 rounded-xl bg-secondary/80 border border-border flex items-center gap-2.5 text-xs font-mono"
                    >
                      <span className="font-bold text-foreground">
                        [{j.name}]
                      </span>
                      <span className="text-[11px] text-rose-300">
                        {j.currentlyBanned} banned
                      </span>
                      <span className="text-[10px] text-muted-foreground">
                        ({j.maxretry} tries / {formatBantimeLabel(j.bantimeSecs)})
                      </span>
                    </div>
                  ))
                )}
              </div>
            </div>
          </div>

          {/* Right: Interactive Jail Policy Tuner Card (7 cols) */}
          <form
            onSubmit={handleApplyPolicy}
            className="xl:col-span-7 rounded-2xl border border-border/80 bg-card/75 p-4 flex flex-col justify-between gap-3.5 shadow-xs"
          >
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2">
                <Sliders className="w-4 h-4 text-primary" />
                <h2 className="text-xs font-bold uppercase tracking-wider text-foreground">
                  {isVi
                    ? "Tinh Chỉnh Chính Sách Jail (99-sshbox.local)"
                    : "Interactive Jail Policy Tuner (99-sshbox.local)"}
                </h2>
              </div>

              {/* Presets Strip */}
              <div className="flex flex-wrap items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => applyPolicyPreset("strict")}
                  className="px-2.5 py-1 rounded-lg bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/35 text-[11px] font-semibold text-amber-300 cursor-pointer transition-colors"
                >
                  {isVi
                    ? "Nghiêm ngặt (3 lần / 24h)"
                    : "Strict (3 retries / 24h ban)"}
                </button>
                <button
                  type="button"
                  onClick={() => applyPolicyPreset("balanced")}
                  className="px-2.5 py-1 rounded-lg bg-cyan-500/15 hover:bg-cyan-500/25 border border-cyan-500/35 text-[11px] font-semibold text-cyan-300 cursor-pointer transition-colors"
                >
                  {isVi
                    ? "Cân bằng (5 lần / 1h)"
                    : "Balanced (5 retries / 1h ban)"}
                </button>
                <button
                  type="button"
                  onClick={() => applyPolicyPreset("permanent")}
                  className="px-2.5 py-1 rounded-lg bg-rose-500/15 hover:bg-rose-500/25 border border-rose-500/35 text-[11px] font-semibold text-rose-300 cursor-pointer transition-colors"
                >
                  {isVi
                    ? "Cấm vĩnh viễn (-1 / 7d)"
                    : "Permanent Ban (-1 / 7d)"}
                </button>
              </div>
            </div>

            {/* Policy Numeric Controls */}
            <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
              <div>
                <label className="ui-label flex items-center justify-between">
                  <span>{isVi ? "Số lần thử tối đa (maxretry)" : "Max Retries (maxretry)"}</span>
                  <span className="font-mono font-bold text-primary">
                    {maxretry}
                  </span>
                </label>
                <input
                  type="number"
                  min={1}
                  max={20}
                  value={maxretry}
                  onChange={(e) =>
                    setMaxretry(
                      Math.max(1, Math.min(20, Number(e.target.value) || 1)),
                    )
                  }
                  className="ui-input font-mono text-xs"
                />
              </div>

              <div>
                <label className="ui-label flex items-center justify-between">
                  <span>{isVi ? "Khung thời gian (findtime)" : "Find Window (findtime)"}</span>
                  <span className="font-mono text-muted-foreground">
                    {Math.round(findtimeSecs / 60)}m ({findtimeSecs}s)
                  </span>
                </label>
                <select
                  value={findtimeSecs}
                  onChange={(e) => setFindtimeSecs(Number(e.target.value))}
                  className="ui-input font-mono text-xs"
                >
                  <option value={300}>5 minutes (300s)</option>
                  <option value={600}>10 minutes (600s)</option>
                  <option value={900}>15 minutes (900s)</option>
                  <option value={1800}>30 minutes (1800s)</option>
                  <option value={3600}>1 hour (3600s)</option>
                  <option value={86400}>24 hours (86400s)</option>
                  <option value={604800}>7 days (604800s)</option>
                </select>
              </div>

              <div>
                <label className="ui-label flex items-center justify-between">
                  <span>{isVi ? "Thời gian khóa (bantime)" : "Ban Duration (bantime)"}</span>
                  <span className="font-mono font-bold text-rose-300">
                    {formatBantimeLabel(bantimeSecs)}
                  </span>
                </label>
                <select
                  value={bantimeSecs}
                  onChange={(e) => setBantimeSecs(Number(e.target.value))}
                  className="ui-input font-mono text-xs"
                >
                  <option value={1800}>30 minutes (1800s)</option>
                  <option value={3600}>1 hour (3600s)</option>
                  <option value={21600}>6 hours (21600s)</option>
                  <option value={43200}>12 hours (43200s)</option>
                  <option value={86400}>24 hours (86400s)</option>
                  <option value={604800}>7 days (604800s)</option>
                  <option value={2592000}>30 days (2592000s)</option>
                  <option value={-1}>Permanent Ban (-1)</option>
                </select>
              </div>
            </div>

            {/* IgnoreIP Whitelist Chip Editor + 1-Click Trust My IP */}
            <div className="space-y-2">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <label className="ui-label !mb-0">
                  {isVi
                    ? "Danh sách IP miễn trừ (ignoreip — Luôn bao gồm 127.0.0.1/8 ::1)"
                    : "Trusted Whitelist IPs (ignoreip — Always includes 127.0.0.1/8 ::1)"}
                </label>
                {effectiveClientIp && (
                  <button
                    type="button"
                    onClick={() => handleAddIgnoreIp(effectiveClientIp)}
                    disabled={ignoreIps.includes(effectiveClientIp)}
                    className="px-2.5 py-1 rounded-lg bg-cyan-500/15 hover:bg-cyan-500/25 border border-cyan-500/35 text-[11px] font-mono font-bold text-cyan-300 inline-flex items-center gap-1 cursor-pointer disabled:opacity-45 transition-colors"
                  >
                    <Shield className="w-3 h-3 text-cyan-400" />
                    <span>+ Trust My IP ({effectiveClientIp})</span>
                  </button>
                )}
              </div>

              <div className="flex flex-wrap items-center gap-2">
                <span className="px-2.5 py-1 rounded-lg bg-secondary/70 border border-border text-[11px] font-mono text-muted-foreground">
                  127.0.0.1/8 ::1
                </span>
                {ignoreIps.map((ip) => (
                  <span
                    key={ip}
                    className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-cyan-500/15 border border-cyan-500/35 text-[11px] font-mono font-bold text-cyan-200"
                  >
                    <span>{ip}</span>
                    <button
                      type="button"
                      onClick={() => handleRemoveIgnoreIp(ip)}
                      className="hover:text-rose-300 cursor-pointer"
                      aria-label={`Remove ${ip}`}
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </span>
                ))}

                <div className="flex items-center gap-1.5 flex-1 min-w-[200px]">
                  <input
                    type="text"
                    value={newIgnoreIp}
                    onChange={(e) => setNewIgnoreIp(e.target.value)}
                    onKeyDown={(e) => {
                      if (e.key === "Enter") {
                        e.preventDefault();
                        handleAddIgnoreIp(newIgnoreIp);
                      }
                    }}
                    placeholder={
                      isVi
                        ? "Thêm IP/CIDR tin cậy (VD: 203.0.113.5)..."
                        : "Add trusted IP/CIDR (e.g. 203.0.113.5)..."
                    }
                    className="ui-input !h-8 font-mono text-xs"
                  />
                  <button
                    type="button"
                    onClick={() => handleAddIgnoreIp(newIgnoreIp)}
                    disabled={!newIgnoreIp.trim()}
                    className="px-2.5 h-8 rounded-lg bg-secondary hover:bg-accent border border-border text-xs font-semibold text-foreground cursor-pointer disabled:opacity-45 shrink-0"
                  >
                    + Add
                  </button>
                </div>
              </div>
            </div>

            <div className="flex items-center justify-end pt-1">
              <button
                type="submit"
                disabled={busyAction !== null}
                className="py-2 px-4 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-95 disabled:opacity-50 inline-flex items-center gap-1.5 cursor-pointer transition-opacity"
              >
                <Zap className="w-3.5 h-3.5" />
                <span>
                  {busyAction === "apply-policy"
                    ? isVi
                      ? "Đang áp dụng & Reload Fail2ban..."
                      : "Applying & Reloading Fail2ban..."
                    : isVi
                      ? "Lưu Chính Sách & Reload Fail2ban"
                      : "Save Policy & Reload Fail2ban"}
                </span>
              </button>
            </div>
          </form>
        </div>
      )}

      {/* SPLIT SCREEN: ZONE 2 (Left Quarantine Vault & Offenders Radar) & ZONE 3 (Right Live SSH Auth Forensics Timeline) */}
      {snapshot && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-start">
          {/* ZONE 2: Left Split Pane (5 cols on XL) */}
          <div className="xl:col-span-5 space-y-4">
            {/* Active Quarantine Vault Card */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-4 space-y-3.5 shadow-xs">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <div className="w-8 h-8 rounded-xl bg-rose-500/15 border border-rose-500/35 flex items-center justify-center text-rose-400">
                    <Ban className="w-4 h-4" />
                  </div>
                  <div>
                    <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                      {isVi
                        ? `Kho Cách Ly IP Đang Bị Cấm (${quarantinedEntries.length})`
                        : `Active Quarantine Vault (${quarantinedEntries.length})`}
                    </h3>
                    <p className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Danh sách IP đang bị chặn bởi Fail2ban hoặc thêm thủ công."
                        : "Currently banned IPs across active Fail2ban jails."}
                    </p>
                  </div>
                </div>

                <div className="relative w-40">
                  <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
                  <input
                    type="text"
                    value={quarantineSearch}
                    onChange={(e) => setQuarantineSearch(e.target.value)}
                    placeholder={isVi ? "Tìm IP..." : "Search IP..."}
                    aria-label="Search quarantined IPs"
                    className="ui-input !h-8 !pl-8 text-xs font-mono"
                  />
                </div>
              </div>

              {/* Manual Ban IP Immediately Bar */}
              <form
                onSubmit={handleManualBanSubmit}
                className="flex items-center gap-2"
              >
                <input
                  type="text"
                  value={manualBanIp}
                  onChange={(e) => setManualBanIp(e.target.value)}
                  placeholder={
                    isVi
                      ? "Nhập IP cần khóa khẩn cấp (VD: 185.220.101.9)..."
                      : "Ban IP immediately (e.g. 185.220.101.9)..."
                  }
                  disabled={!snapshot.installed || busyAction !== null}
                  className="ui-input !h-8 font-mono text-xs flex-1"
                />
                <button
                  type="submit"
                  disabled={
                    !snapshot.installed ||
                    !manualBanIp.trim() ||
                    busyAction !== null
                  }
                  className="px-3 h-8 rounded-xl bg-rose-500/20 hover:bg-rose-500/30 border border-rose-500/40 text-rose-200 text-xs font-bold inline-flex items-center gap-1.5 cursor-pointer disabled:opacity-45 shrink-0 transition-colors"
                >
                  <Ban className="w-3.5 h-3.5 text-rose-400" />
                  <span>{isVi ? "Khóa IP Ngay" : "Ban IP Immediately"}</span>
                </button>
              </form>

              {/* Quarantined IP List */}
              <div className="max-h-56 overflow-y-auto space-y-1.5 pr-1">
                {filteredQuarantined.length === 0 ? (
                  <div className="rounded-xl border border-border/60 bg-background/40 py-6 px-4 text-center text-xs text-muted-foreground">
                    {isVi
                      ? "Hiện không có địa chỉ IP nào đang nằm trong danh sách cách ly."
                      : "No IPs are currently quarantined across active jails."}
                  </div>
                ) : (
                  filteredQuarantined.map((item) => (
                    <div
                      key={`${item.jail}-${item.ip}`}
                      className="px-3 py-2 rounded-xl bg-background/60 border border-border/70 flex items-center justify-between gap-2"
                    >
                      <div className="flex items-center gap-2 min-w-0">
                        <span className="px-2 py-0.5 rounded-md bg-rose-500/15 border border-rose-500/35 text-[10px] font-mono font-bold text-rose-300">
                          {item.jail}
                        </span>
                        <span className="font-mono text-xs font-bold text-foreground truncate">
                          {item.ip}
                        </span>
                      </div>

                      <button
                        type="button"
                        disabled={busyAction !== null}
                        onClick={() =>
                          handleBanOrUnbanIp(item.jail, item.ip, "unban")
                        }
                        className="px-2.5 py-1 rounded-lg bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/35 text-[11px] font-mono font-bold text-emerald-300 inline-flex items-center gap-1 cursor-pointer disabled:opacity-45 transition-colors shrink-0"
                      >
                        <Unlock className="w-3 h-3 text-emerald-400" />
                        <span>{isVi ? "Gỡ cấm (Unban)" : "Unban IP"}</span>
                      </button>
                    </div>
                  ))
                )}
              </div>
            </section>

            {/* Top Brute-Force Offenders Frequency Radar */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-4 space-y-3.5 shadow-xs">
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2">
                  <div className="w-8 h-8 rounded-xl bg-amber-500/15 border border-amber-500/35 flex items-center justify-center text-amber-400">
                    <Radar className="w-4 h-4" />
                  </div>
                  <div>
                    <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                      {isVi
                        ? "Radar Tần Suất Kẻ Tấn Công Hàng Đầu"
                        : "Top Attacking IPs Frequency Radar"}
                    </h3>
                    <p className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Tổng hợp từ các phiên thử mật khẩu sai trong nhật ký SSH."
                        : "Aggregated brute-force offenders from recent SSH auth logs."}
                    </p>
                  </div>
                </div>
              </div>

              <div className="space-y-2.5 max-h-80 overflow-y-auto pr-1">
                {snapshot.topAttackerIps.length === 0 ? (
                  <div className="rounded-xl border border-border/60 bg-background/40 py-6 px-4 text-center text-xs text-muted-foreground">
                    {isVi
                      ? "Không phát hiện IP tấn công brute-force nào trong nhật ký gần đây."
                      : "No brute-force attacker IPs detected in recent SSH logs."}
                  </div>
                ) : (
                  snapshot.topAttackerIps.slice(0, 12).map((attacker) => {
                    const pct = Math.max(
                      8,
                      Math.round(
                        (attacker.failedCount / maxAttackerCount) * 100,
                      ),
                    );
                    const isBanned =
                      attacker.isCurrentlyBanned ||
                      bannedIpSet.has(attacker.ip);

                    return (
                      <div
                        key={attacker.ip}
                        className="p-2.5 rounded-xl bg-background/60 border border-border/70 space-y-1.5"
                      >
                        <div className="flex items-center justify-between gap-2 text-xs">
                          <div className="flex items-center gap-2 min-w-0">
                            <span className="font-mono font-bold text-foreground truncate">
                              {attacker.ip}
                            </span>
                            <span className="px-1.5 py-0.5 rounded bg-secondary text-[10px] font-mono text-muted-foreground">
                              target: {attacker.lastTargetUser}
                            </span>
                          </div>

                          <div className="flex items-center gap-2 shrink-0">
                            <span className="font-mono text-[11px] font-bold text-amber-300">
                              {attacker.failedCount}{" "}
                              {isVi ? "lần sai" : "fails"}
                            </span>

                            {isBanned ? (
                              <span className="px-2 py-0.5 rounded-full bg-rose-500/20 border border-rose-500/40 text-[10px] font-mono font-bold text-rose-300">
                                QUARANTINED
                              </span>
                            ) : (
                              <button
                                type="button"
                                disabled={
                                  !snapshot.installed || busyAction !== null
                                }
                                onClick={() =>
                                  handleBanOrUnbanIp(
                                    selectedJail || "sshd",
                                    attacker.ip,
                                    "ban",
                                  )
                                }
                                title={
                                  snapshot.installed
                                    ? `fail2ban-client set ${selectedJail || "sshd"} banip ${attacker.ip}`
                                    : "Install Fail2ban first to ban IPs"
                                }
                                className="px-2 py-0.5 rounded-lg bg-rose-500/15 hover:bg-rose-500/25 border border-rose-500/35 text-[10px] font-mono font-bold text-rose-300 cursor-pointer disabled:opacity-45"
                              >
                                Ban IP
                              </button>
                            )}
                          </div>
                        </div>

                        {/* Horizontal Frequency Bar */}
                        <div className="w-full h-1.5 rounded-full bg-secondary overflow-hidden">
                          <div
                            className={`h-full rounded-full transition-all ${
                              isBanned ? "bg-rose-500" : "bg-amber-400"
                            }`}
                            style={{ width: `${pct}%` }}
                          />
                        </div>
                      </div>
                    );
                  })
                )}
              </div>
            </section>
          </div>

          {/* ZONE 3: Right Split Pane — Live SSH Authentication Forensics Timeline (7 cols on XL) */}
          <section className="xl:col-span-7 rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center text-cyan-400">
                  <Activity className="w-4 h-4" />
                </div>
                <div>
                  <h2
                    id={timelineCaptionId}
                    className="text-xs font-bold uppercase tracking-wider text-foreground"
                  >
                    {isVi
                      ? "Dòng Thời Gian Pháp Y Xác Thực SSH Trực Tiếp (Live Auth Forensics)"
                      : "Live SSH Authentication Forensics Timeline"}
                  </h2>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Phân tích trực tiếp các sự kiện Accepted, Failed Password và Invalid User từ sshd."
                      : "Structured forensic stream of Accepted logins, Failed passwords, and Invalid user probes."}
                  </p>
                </div>
              </div>

              {/* Search by IP or username */}
              <div className="relative w-full sm:w-56">
                <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
                <input
                  type="text"
                  value={eventSearch}
                  onChange={(e) => setEventSearch(e.target.value)}
                  placeholder={
                    isVi ? "Lọc theo IP, username..." : "Filter IP, username..."
                  }
                  aria-label="Filter SSH authentication events"
                  className="ui-input !h-8 !pl-8 text-xs font-mono"
                />
              </div>
            </div>

            {/* Filter Pills */}
            <div className="flex flex-wrap items-center gap-1.5">
              {(
                [
                  {
                    id: "all",
                    label: isVi ? "Tất cả sự kiện" : "All Auth Events",
                    count: eventCounts.all,
                  },
                  {
                    id: "accepted",
                    label: isVi ? "Đăng nhập thành công" : "Accepted Logins",
                    count: eventCounts.accepted,
                  },
                  {
                    id: "failed",
                    label: isVi ? "Sai mật khẩu (Brute-Force)" : "Brute-Force / Failed",
                    count: eventCounts.failed,
                  },
                  {
                    id: "invalid_user",
                    label: isVi ? "User không tồn tại" : "Invalid Users",
                    count: eventCounts.invalid_user,
                  },
                ] as const
              ).map((pill) => {
                const active = eventFilter === pill.id;
                return (
                  <button
                    key={pill.id}
                    type="button"
                    onClick={() => setEventFilter(pill.id)}
                    className={`px-2.5 py-1 rounded-xl text-xs font-semibold border inline-flex items-center gap-1.5 transition-all cursor-pointer ${
                      active
                        ? "bg-primary text-primary-foreground border-primary shadow-2xs"
                        : "bg-secondary/70 text-muted-foreground border-border/70 hover:text-foreground"
                    }`}
                  >
                    <span>{pill.label}</span>
                    <span
                      className={`px-1.5 py-0.2 rounded-full text-[10px] font-mono ${
                        active
                          ? "bg-black/20 text-primary-foreground"
                          : "bg-background/70 text-foreground"
                      }`}
                    >
                      {pill.count}
                    </span>
                  </button>
                );
              })}
            </div>

            {/* Sticky Responsive Forensic Timeline Table */}
            <div
              role="region"
              tabIndex={0}
              aria-labelledby={timelineCaptionId}
              className="vps-table-wrapper rounded-xl border border-border/80 bg-background/50 max-h-[540px]"
            >
              <table className="w-full text-left text-xs">
                <caption className="sr-only">
                  Live SSH authentication forensic audit events
                </caption>
                <thead className="bg-secondary/95 text-[11px] uppercase tracking-wider text-muted-foreground">
                  <tr>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "Sự kiện" : "Event"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "Phương thức" : "Method"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "Tài khoản đích" : "Target User"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "IP Nguồn : Cổng" : "Source IP : Port"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "Thời gian" : "Timestamp"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border text-right">
                      {isVi ? "Phản ứng 1-Click" : "Action"}
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border/60">
                  {filteredAuthEvents.length === 0 ? (
                    <tr>
                      <td
                        colSpan={6}
                        className="py-8 px-4 text-center text-xs text-muted-foreground"
                      >
                        {isVi
                          ? "Không có sự kiện xác thực SSH nào khớp với bộ lọc hiện tại."
                          : "No SSH authentication events match the current filter."}
                      </td>
                    </tr>
                  ) : (
                    filteredAuthEvents.map((ev, idx) => {
                      const isBanned = bannedIpSet.has(ev.ip);
                      const isSuspicious = ev.eventType !== "accepted";
                      return (
                        <tr
                          key={`${ev.timestamp}-${ev.ip}-${ev.port}-${idx}`}
                          className="hover:bg-accent/30 transition-colors"
                        >
                          <th
                            scope="row"
                            className="py-2.5 px-3 whitespace-nowrap font-normal"
                          >
                            {renderEventBadge(ev)}
                          </th>

                          <td className="py-2.5 px-3 whitespace-nowrap">
                            {renderMethodPill(ev.method)}
                          </td>

                          <td className="py-2.5 px-3 font-mono font-bold text-foreground whitespace-nowrap">
                            {ev.username}
                          </td>

                          <td className="py-2.5 px-3 font-mono whitespace-nowrap">
                            <span
                              className={
                                isSuspicious
                                  ? "text-rose-300 font-bold"
                                  : "text-emerald-300 font-semibold"
                              }
                            >
                              {ev.ip}
                            </span>
                            <span className="text-[10px] text-muted-foreground ml-1">
                              :{ev.port}
                            </span>
                          </td>

                          <td className="py-2.5 px-3 font-mono text-[11px] text-muted-foreground whitespace-nowrap">
                            <span className="inline-flex items-center gap-1">
                              <Clock className="w-3 h-3 opacity-65" />
                              <span>{ev.timestamp || "recent"}</span>
                            </span>
                          </td>

                          <td className="py-2.5 px-3 text-right whitespace-nowrap">
                            {isSuspicious ? (
                              isBanned ? (
                                <span className="px-2 py-0.5 rounded-full bg-rose-500/20 border border-rose-500/40 text-[10px] font-mono font-bold text-rose-300">
                                  QUARANTINED
                                </span>
                              ) : (
                                <button
                                  type="button"
                                  disabled={
                                    !snapshot.installed || busyAction !== null
                                  }
                                  onClick={() =>
                                    handleBanOrUnbanIp(
                                      selectedJail || "sshd",
                                      ev.ip,
                                      "ban",
                                    )
                                  }
                                  title={
                                    snapshot.installed
                                      ? `fail2ban-client set ${selectedJail || "sshd"} banip ${ev.ip}`
                                      : "Install Fail2ban first to ban offending IPs"
                                  }
                                  className="px-2.5 py-1 rounded-lg bg-rose-500/15 hover:bg-rose-500/25 border border-rose-500/35 text-[10px] font-mono font-bold text-rose-300 inline-flex items-center gap-1 cursor-pointer disabled:opacity-45"
                                >
                                  <Ban className="w-2.5 h-2.5" />
                                  <span>Ban IP</span>
                                </button>
                              )
                            ) : (
                              <span className="text-[10px] font-mono text-emerald-400/80">
                                Authorized
                              </span>
                            )}
                          </td>
                        </tr>
                      );
                    })
                  )}
                </tbody>
              </table>
            </div>
          </section>
        </div>
      )}
    </div>
  );
};
