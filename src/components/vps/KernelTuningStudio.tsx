import React, { useCallback, useEffect, useId, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  AlertTriangle,
  Check,
  CheckCircle2,
  Cpu,
  Database,
  EyeOff,
  FileDiff,
  Gauge,
  HardDrive,
  Network,
  RefreshCw,
  RotateCcw,
  Shield,
  ShieldCheck,
  Sparkles,
  Zap,
} from "lucide-react";
import {
  HostEntry,
  KernelSysctlSnapshot,
  KernelSysctlUpdate,
  VpsPrivilegeInfo,
} from "../../types";
import { AppLanguage } from "../../themes";

interface KernelTuningStudioProps {
  host: HostEntry;
  sudoPassword?: string;
  privilegeInfo: VpsPrivilegeInfo | null;
  language?: AppLanguage;
  onRequireSudoUnlock?: () => void;
}

function snapshotToUpdate(snap: KernelSysctlSnapshot): KernelSysctlUpdate {
  return {
    congestionControl: snap.congestionControl,
    defaultQdisc: snap.defaultQdisc,
    swappiness: snap.swappiness,
    overcommitMemory: snap.overcommitMemory,
    vfsCachePressure: snap.vfsCachePressure,
    fileMax: snap.fileMax,
    somaxconn: snap.somaxconn,
    tcpMaxSynBacklog: snap.tcpMaxSynBacklog,
    tcpTwReuse: snap.tcpTwReuse,
    tcpFinTimeout: snap.tcpFinTimeout,
    tcpSyncookies: snap.tcpSyncookies,
    icmpEchoIgnoreAll: snap.icmpEchoIgnoreAll,
    rpFilter: snap.rpFilter,
    acceptRedirects: snap.acceptRedirects,
    acceptSourceRoute: snap.acceptSourceRoute,
    ipForward: snap.ipForward,
  };
}

interface SysctlDiffItem {
  key: string;
  before: string;
  after: string;
  changed: boolean;
}

function buildSysctlDiff(
  base: KernelSysctlSnapshot,
  draft: KernelSysctlUpdate,
): SysctlDiffItem[] {
  const b01 = (v: boolean) => (v ? "1" : "0");
  return [
    {
      key: "net.core.default_qdisc",
      before: base.defaultQdisc,
      after: draft.defaultQdisc,
      changed: base.defaultQdisc !== draft.defaultQdisc,
    },
    {
      key: "net.ipv4.tcp_congestion_control",
      before: base.congestionControl,
      after: draft.congestionControl,
      changed: base.congestionControl !== draft.congestionControl,
    },
    {
      key: "vm.swappiness",
      before: String(base.swappiness),
      after: String(draft.swappiness),
      changed: base.swappiness !== draft.swappiness,
    },
    {
      key: "vm.overcommit_memory",
      before: String(base.overcommitMemory),
      after: String(draft.overcommitMemory),
      changed: base.overcommitMemory !== draft.overcommitMemory,
    },
    {
      key: "vm.vfs_cache_pressure",
      before: String(base.vfsCachePressure),
      after: String(draft.vfsCachePressure),
      changed: base.vfsCachePressure !== draft.vfsCachePressure,
    },
    {
      key: "fs.file-max",
      before: String(base.fileMax),
      after: String(draft.fileMax),
      changed: base.fileMax !== draft.fileMax,
    },
    {
      key: "net.core.somaxconn",
      before: String(base.somaxconn),
      after: String(draft.somaxconn),
      changed: base.somaxconn !== draft.somaxconn,
    },
    {
      key: "net.ipv4.tcp_max_syn_backlog",
      before: String(base.tcpMaxSynBacklog),
      after: String(draft.tcpMaxSynBacklog),
      changed: base.tcpMaxSynBacklog !== draft.tcpMaxSynBacklog,
    },
    {
      key: "net.ipv4.tcp_tw_reuse",
      before: b01(base.tcpTwReuse),
      after: b01(draft.tcpTwReuse),
      changed: base.tcpTwReuse !== draft.tcpTwReuse,
    },
    {
      key: "net.ipv4.tcp_fin_timeout",
      before: String(base.tcpFinTimeout),
      after: String(draft.tcpFinTimeout),
      changed: base.tcpFinTimeout !== draft.tcpFinTimeout,
    },
    {
      key: "net.ipv4.tcp_syncookies",
      before: b01(base.tcpSyncookies),
      after: b01(draft.tcpSyncookies),
      changed: base.tcpSyncookies !== draft.tcpSyncookies,
    },
    {
      key: "net.ipv4.icmp_echo_ignore_all",
      before: b01(base.icmpEchoIgnoreAll),
      after: b01(draft.icmpEchoIgnoreAll),
      changed: base.icmpEchoIgnoreAll !== draft.icmpEchoIgnoreAll,
    },
    {
      key: "net.ipv4.conf.all.rp_filter",
      before: b01(base.rpFilter),
      after: b01(draft.rpFilter),
      changed: base.rpFilter !== draft.rpFilter,
    },
    {
      key: "net.ipv4.conf.all.accept_redirects",
      before: b01(base.acceptRedirects),
      after: b01(draft.acceptRedirects),
      changed: base.acceptRedirects !== draft.acceptRedirects,
    },
    {
      key: "net.ipv4.conf.all.accept_source_route",
      before: b01(base.acceptSourceRoute),
      after: b01(draft.acceptSourceRoute),
      changed: base.acceptSourceRoute !== draft.acceptSourceRoute,
    },
    {
      key: "net.ipv4.ip_forward",
      before: b01(base.ipForward),
      after: b01(draft.ipForward),
      changed: base.ipForward !== draft.ipForward,
    },
  ];
}

function renderSysctlConfPreview(draft: KernelSysctlUpdate): string {
  const b01 = (v: boolean) => (v ? 1 : 0);
  return `# /etc/sysctl.d/99-sshbox.conf — Managed by SSH-Box Kernel & Network Tuning Studio
# Congestion Control & Packet Scheduler
net.core.default_qdisc = ${draft.defaultQdisc}
net.ipv4.tcp_congestion_control = ${draft.congestionControl}

# Virtual Memory & Swap Behavior
vm.swappiness = ${draft.swappiness}
vm.overcommit_memory = ${draft.overcommitMemory}
vm.vfs_cache_pressure = ${draft.vfsCachePressure}

# Filesystem & High-Concurrency Socket Backlogs
fs.file-max = ${draft.fileMax}
net.core.somaxconn = ${draft.somaxconn}
net.ipv4.tcp_max_syn_backlog = ${draft.tcpMaxSynBacklog}
net.ipv4.tcp_tw_reuse = ${b01(draft.tcpTwReuse)}
net.ipv4.tcp_fin_timeout = ${draft.tcpFinTimeout}

# Network Fortress, Anti-Spoofing & Routing
net.ipv4.tcp_syncookies = ${b01(draft.tcpSyncookies)}
net.ipv4.icmp_echo_ignore_all = ${b01(draft.icmpEchoIgnoreAll)}
net.ipv4.conf.all.rp_filter = ${b01(draft.rpFilter)}
net.ipv4.conf.default.rp_filter = ${b01(draft.rpFilter)}
net.ipv4.conf.all.accept_redirects = ${b01(draft.acceptRedirects)}
net.ipv4.conf.default.accept_redirects = ${b01(draft.acceptRedirects)}
net.ipv4.conf.all.accept_source_route = ${b01(draft.acceptSourceRoute)}
net.ipv4.conf.default.accept_source_route = ${b01(draft.acceptSourceRoute)}
net.ipv4.ip_forward = ${b01(draft.ipForward)}`;
}

export const KernelTuningStudio: React.FC<KernelTuningStudioProps> = ({
  host,
  sudoPassword,
  privilegeInfo,
  language = "vi",
  onRequireSudoUnlock,
}) => {
  const isVi = language === "vi";
  const swappinessRangeId = useId();
  const vfsRangeId = useId();

  const [snapshot, setSnapshot] = useState<KernelSysctlSnapshot | null>(null);
  const [draft, setDraft] = useState<KernelSysctlUpdate | null>(null);
  const [loading, setLoading] = useState(false);
  const [applying, setApplying] = useState(false);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  const fetchSnapshot = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const res = await invoke<KernelSysctlSnapshot>("vps_get_kernel_snapshot", {
        hostId: host.id,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(res);
      setDraft(snapshotToUpdate(res));
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
    fetchSnapshot();
  }, [fetchSnapshot]);

  useEffect(() => {
    if (!toastMsg) return;
    const timer = setTimeout(() => setToastMsg(null), 5000);
    return () => clearTimeout(timer);
  }, [toastMsg]);

  const diffItems = useMemo(() => {
    if (!snapshot || !draft) return [];
    return buildSysctlDiff(snapshot, draft);
  }, [snapshot, draft]);

  const changedItems = useMemo(
    () => diffItems.filter((d) => d.changed),
    [diffItems],
  );

  const sysctlConfPreview = useMemo(() => {
    if (!draft) return "";
    return renderSysctlConfPreview(draft);
  }, [draft]);

  const handleApplyKernelTuning = async () => {
    if (!draft) return;
    setApplying(true);
    setErrorMsg(null);
    try {
      const updated = await invoke<KernelSysctlSnapshot>(
        "vps_apply_kernel_tuning",
        {
          hostId: host.id,
          update: draft,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(updated);
      setDraft(snapshotToUpdate(updated));
      setToastMsg(
        isVi
          ? "Đã ghi /etc/sysctl.d/99-sshbox.conf và kích hoạt tham số Kernel & TCP BBR trực tiếp (sysctl -p)!"
          : "Persisted /etc/sysctl.d/99-sshbox.conf and applied live kernel & TCP BBR sysctl directives!",
      );
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setApplying(false);
    }
  };

  const handleResetChanges = () => {
    if (!snapshot) return;
    setDraft(snapshotToUpdate(snapshot));
    setToastMsg(
      isVi
        ? "Đã hoàn tác các thay đổi tạm về cấu hình Kernel đang chạy."
        : "Reverted staged sysctl changes back to live kernel values.",
    );
  };

  if (loading && (!snapshot || !draft)) {
    return (
      <div className="rounded-2xl border border-border/80 bg-card/70 p-10 flex flex-col items-center justify-center gap-3 text-center">
        <RefreshCw className="w-6 h-6 text-violet-400 animate-spin" />
        <div className="text-sm font-bold text-foreground">
          {isVi
            ? "Đang thăm dò Kernel Sysctl, thuật toán TCP BBR và hàng đợi mạng..."
            : "Probing Linux Kernel Release, TCP Congestion Control & Sysctl Parameters..."}
        </div>
        <p className="text-xs text-muted-foreground max-w-md font-mono">
          sysctl net.core.default_qdisc net.ipv4.tcp_congestion_control vm.* fs.*
        </p>
      </div>
    );
  }

  if (!snapshot || !draft) {
    return (
      <div className="rounded-2xl border border-rose-500/35 bg-rose-500/10 p-6 space-y-3">
        <div className="flex items-center gap-2 text-rose-300 font-bold text-sm">
          <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0" />
          <span>
            {isVi
              ? "Không thể đọc thông số Kernel Sysctl từ máy chủ"
              : "Failed to Probe Remote Kernel Sysctl Snapshot"}
          </span>
        </div>
        {errorMsg && (
          <pre className="text-xs font-mono text-rose-200/90 bg-background/60 p-3 rounded-xl border border-rose-500/25 whitespace-pre-wrap">
            {errorMsg}
          </pre>
        )}
        <div className="flex items-center gap-2">
          <button
            type="button"
            onClick={fetchSnapshot}
            className="px-3.5 py-1.5 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-95 cursor-pointer"
          >
            {isVi ? "Thử lại" : "Retry Probe"}
          </button>
          {privilegeInfo && !privilegeInfo.sudoUnlocked && (
            <button
              type="button"
              onClick={() => onRequireSudoUnlock?.()}
              className="px-3.5 py-1.5 rounded-xl bg-amber-500/20 border border-amber-500/40 text-amber-300 text-xs font-semibold hover:bg-amber-500/30 cursor-pointer"
            >
              {isVi ? "Mở khóa Sudo" : "Unlock Sudo"}
            </button>
          )}
        </div>
      </div>
    );
  }

  const isBbrStaged =
    draft.congestionControl.toLowerCase() === "bbr" &&
    draft.defaultQdisc.toLowerCase() === "fq";

  const isHighConcurrencyOptimized =
    draft.somaxconn >= 16384 &&
    draft.fileMax >= 1000000 &&
    draft.tcpMaxSynBacklog >= 8192 &&
    draft.tcpTwReuse === true;

  const isFortressShieldActive =
    draft.tcpSyncookies &&
    draft.rpFilter &&
    !draft.acceptRedirects &&
    !draft.acceptSourceRoute;

  const ramRetentionPct = Math.max(0, Math.min(100, 100 - draft.swappiness));
  const swapAggressivenessPct = Math.max(0, Math.min(100, draft.swappiness));

  return (
    <div className="space-y-5 pb-8">
      {/* Status Toast & Error Banner */}
      {toastMsg && (
        <div className="ui-banner-slide rounded-xl border border-emerald-500/40 bg-emerald-500/15 px-4 py-2.5 flex items-center justify-between gap-3 text-xs text-emerald-200 shadow-sm">
          <div className="flex items-center gap-2">
            <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
            <span className="font-medium">{toastMsg}</span>
          </div>
          <button
            type="button"
            onClick={() => setToastMsg(null)}
            className="text-[11px] text-emerald-300 hover:underline cursor-pointer"
          >
            {isVi ? "Đóng" : "Dismiss"}
          </button>
        </div>
      )}

      {errorMsg && (
        <div className="rounded-xl border border-rose-500/40 bg-rose-500/15 px-4 py-3 flex items-start gap-2.5 text-xs text-rose-200">
          <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0 mt-0.5" />
          <div className="flex-1 min-w-0">
            <div className="font-bold text-rose-300">
              {isVi
                ? "Lỗi khi áp dụng cấu hình Kernel Sysctl"
                : "Kernel Sysctl Operation Error"}
            </div>
            <div className="font-mono text-[11px] mt-0.5 break-words">
              {errorMsg}
            </div>
          </div>
        </div>
      )}

      {/* ====================================================================
          ZONE 1: TOP DECK — 3 SIGNATURE INTERACTIVE PROFILE CARDS
          ==================================================================== */}
      <div className="grid grid-cols-1 lg:grid-cols-3 gap-4">
        {/* Card 1: Google TCP BBR Congestion Accelerator Card */}
        <section
          className={`rounded-2xl border p-4 flex flex-col justify-between gap-3.5 transition-all shadow-xs ${
            isBbrStaged
              ? "border-emerald-500/45 bg-emerald-500/8"
              : "border-border/80 bg-card/75"
          }`}
        >
          <div className="space-y-2.5">
            <div className="flex items-start justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <div
                  className={`w-9 h-9 rounded-xl border flex items-center justify-center shrink-0 ${
                    isBbrStaged
                      ? "bg-emerald-500/20 border-emerald-500/40 text-emerald-300"
                      : "bg-violet-500/15 border-violet-500/35 text-violet-300"
                  }`}
                >
                  <Zap className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-xs font-bold text-foreground">
                    {isVi
                      ? "Tăng tốc Google TCP BBR"
                      : "Google TCP BBR Congestion Accelerator"}
                  </h3>
                  <div className="flex items-center gap-1.5 mt-0.5 flex-wrap">
                    <span className="px-1.5 py-0.5 rounded-md bg-secondary text-[10px] font-mono text-foreground border border-border/70">
                      Linux {snapshot.kernelRelease}
                    </span>
                    <span
                      className={`px-1.5 py-0.5 rounded-md text-[10px] font-mono font-bold border ${
                        isBbrStaged
                          ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/35"
                          : "bg-amber-500/15 text-amber-300 border-amber-500/30"
                      }`}
                    >
                      {draft.defaultQdisc} → {draft.congestionControl}
                    </span>
                  </div>
                </div>
              </div>

              <label className="inline-flex items-center gap-1.5 text-[11px] font-mono font-semibold text-foreground cursor-pointer shrink-0">
                <span>BBR</span>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={isBbrStaged}
                  onChange={(e) => {
                    const nextBbr = e.target.checked;
                    setDraft((prev) =>
                      prev
                        ? {
                            ...prev,
                            defaultQdisc: nextBbr ? "fq" : "fq_codel",
                            congestionControl: nextBbr ? "bbr" : "cubic",
                          }
                        : prev,
                    );
                  }}
                />
              </label>
            </div>

            <p className="text-[11px] text-muted-foreground leading-relaxed">
              {isVi
                ? "Tối ưu băng thông đường truyền quốc tế & giảm độ trễ gói tin TCP bằng bộ lập lịch Fair Queue (fq) + thuật toán Google BBR."
                : "Maximizes WAN throughput and minimizes bufferbloat latency over lossy links using Fair Queueing (fq) + Google TCP BBR."}
            </p>

            <div className="flex flex-wrap items-center gap-1.5 pt-0.5">
              <span className="text-[10px] text-muted-foreground font-mono">
                {isVi ? "Thuật toán hỗ trợ:" : "Available:"}
              </span>
              {snapshot.availableCongestionControls.map((algo) => (
                <span
                  key={algo}
                  className={`px-1.5 py-0.5 rounded text-[10px] font-mono border ${
                    algo === draft.congestionControl
                      ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/40 font-bold"
                      : "bg-secondary/70 text-muted-foreground border-border/60"
                  }`}
                >
                  {algo}
                </span>
              ))}
            </div>
          </div>

          <div className="flex items-center gap-2 pt-1">
            <button
              type="button"
              onClick={() =>
                setDraft((prev) =>
                  prev
                    ? {
                        ...prev,
                        defaultQdisc: isBbrStaged ? "fq_codel" : "fq",
                        congestionControl: isBbrStaged ? "cubic" : "bbr",
                      }
                    : prev,
                )
              }
              className={`w-full py-2 px-3 rounded-xl text-xs font-semibold border transition-all cursor-pointer flex items-center justify-center gap-1.5 ${
                isBbrStaged
                  ? "bg-emerald-500/15 border-emerald-500/35 text-emerald-300 hover:bg-emerald-500/25"
                  : "bg-primary text-primary-foreground border-primary hover:opacity-95"
              }`}
            >
              <Zap className="w-3.5 h-3.5" />
              <span>
                {isBbrStaged
                  ? isVi
                    ? "Đang chọn TCP BBR (fq + bbr) — Nhấn để về CUBIC"
                    : "TCP BBR Staged (fq + bbr) — Switch to Standard CUBIC"
                  : isVi
                    ? "Bật Tăng tốc TCP BBR (fq + bbr)"
                    : "Enable TCP BBR (fq + bbr)"}
              </span>
            </button>
          </div>
        </section>

        {/* Card 2: High-Concurrency & Docker Server Profile Card */}
        <section
          className={`rounded-2xl border p-4 flex flex-col justify-between gap-3.5 transition-all shadow-xs ${
            isHighConcurrencyOptimized
              ? "border-emerald-500/45 bg-emerald-500/8"
              : "border-border/80 bg-card/75"
          }`}
        >
          <div className="space-y-2.5">
            <div className="flex items-start justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <div
                  className={`w-9 h-9 rounded-xl border flex items-center justify-center shrink-0 ${
                    isHighConcurrencyOptimized
                      ? "bg-emerald-500/20 border-emerald-500/40 text-emerald-300"
                      : "bg-cyan-500/15 border-cyan-500/35 text-cyan-300"
                  }`}
                >
                  <Gauge className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-xs font-bold text-foreground">
                    {isVi
                      ? "Cấu hình Chịu tải Cao & Docker"
                      : "High-Concurrency & Docker Profile"}
                  </h3>
                  <div className="flex items-center gap-1.5 mt-0.5">
                    <span
                      className={`px-2 py-0.5 rounded-md text-[10px] font-mono font-bold border ${
                        isHighConcurrencyOptimized
                          ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/35"
                          : "bg-secondary text-muted-foreground border-border/70"
                      }`}
                    >
                      {isHighConcurrencyOptimized ? "OPTIMIZED" : "DEFAULT"}
                    </span>
                    <span className="text-[10px] font-mono text-muted-foreground">
                      somaxconn {draft.somaxconn.toLocaleString()}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            <p className="text-[11px] text-muted-foreground leading-relaxed">
              {isVi
                ? "Mở rộng giới hạn File Descriptors (2M), hàng đợi kết nối đồng thời (somaxconn 65535), tái sử dụng TIME_WAIT và giảm FIN timeout cho Nginx/Docker."
                : "Expands open file descriptors (2M), listen queue (somaxconn 65535), SYN backlog (16384), TIME_WAIT socket reuse, and 15s FIN timeout."}
            </p>

            <div className="grid grid-cols-2 gap-1.5 text-[10px] font-mono">
              <div className="px-2 py-1 rounded-lg bg-background/60 border border-border/60 flex items-center justify-between">
                <span className="text-muted-foreground">file-max:</span>
                <span className="text-foreground font-semibold">
                  {draft.fileMax.toLocaleString()}
                </span>
              </div>
              <div className="px-2 py-1 rounded-lg bg-background/60 border border-border/60 flex items-center justify-between">
                <span className="text-muted-foreground">syn_backlog:</span>
                <span className="text-foreground font-semibold">
                  {draft.tcpMaxSynBacklog.toLocaleString()}
                </span>
              </div>
            </div>
          </div>

          <button
            type="button"
            onClick={() =>
              setDraft((prev) =>
                prev
                  ? {
                      ...prev,
                      somaxconn: 65535,
                      fileMax: 2097152,
                      tcpMaxSynBacklog: 16384,
                      tcpTwReuse: true,
                      tcpFinTimeout: 15,
                      swappiness: 10,
                    }
                  : prev,
              )
            }
            className="w-full py-2 px-3 rounded-xl bg-cyan-500/15 hover:bg-cyan-500/25 border border-cyan-500/35 text-cyan-300 text-xs font-semibold transition-all cursor-pointer flex items-center justify-center gap-1.5"
          >
            <Sparkles className="w-3.5 h-3.5" />
            <span>
              {isVi
                ? "Áp mẫu Chịu tải Cao (Stage High-Throughput)"
                : "Stage High-Throughput Profile"}
            </span>
          </button>
        </section>

        {/* Card 3: Kernel Network Fortress & Ping Stealth Card */}
        <section
          className={`rounded-2xl border p-4 flex flex-col justify-between gap-3.5 transition-all shadow-xs ${
            isFortressShieldActive
              ? "border-emerald-500/45 bg-emerald-500/8"
              : "border-border/80 bg-card/75"
          }`}
        >
          <div className="space-y-2.5">
            <div className="flex items-start justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <div
                  className={`w-9 h-9 rounded-xl border flex items-center justify-center shrink-0 ${
                    isFortressShieldActive
                      ? "bg-emerald-500/20 border-emerald-500/40 text-emerald-300"
                      : "bg-amber-500/15 border-amber-500/35 text-amber-300"
                  }`}
                >
                  <ShieldCheck className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-xs font-bold text-foreground">
                    {isVi
                      ? "Pháo đài Mạng & Ẩn Ping (Stealth)"
                      : "Kernel Network Fortress & Ping Stealth"}
                  </h3>
                  <div className="flex items-center gap-1.5 mt-0.5">
                    <span
                      className={`px-2 py-0.5 rounded-md text-[10px] font-mono font-bold border ${
                        isFortressShieldActive
                          ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/35"
                          : "bg-amber-500/15 text-amber-300 border-amber-500/30"
                      }`}
                    >
                      {isFortressShieldActive
                        ? "SHIELDS ACTIVE"
                        : "PARTIAL SHIELDS"}
                    </span>
                  </div>
                </div>
              </div>
            </div>

            {/* Dedicated Ping Stealth Mode Switch */}
            <label className="flex items-center justify-between gap-2 px-3 py-2 rounded-xl bg-background/65 border border-border/75 cursor-pointer hover:bg-accent/30 transition-colors">
              <div className="flex items-center gap-2 min-w-0">
                <EyeOff className="w-3.5 h-3.5 text-violet-400 shrink-0" />
                <div className="min-w-0">
                  <div className="text-[11px] font-semibold text-foreground truncate">
                    {isVi
                      ? "Chế độ Ẩn Ping (Ping Stealth Mode)"
                      : "Ping Stealth Mode (icmp_echo_ignore_all)"}
                  </div>
                  <div className="text-[10px] text-muted-foreground font-mono">
                    {draft.icmpEchoIgnoreAll
                      ? isVi
                        ? "Đang chặn toàn bộ ICMP Ping từ bên ngoài"
                        : "Dropping all external ICMP ping probes"
                      : isVi
                        ? "Cho phép phản hồi gói tin ICMP Echo"
                        : "Responding normally to ICMP ping requests"}
                  </div>
                </div>
              </div>
              <input
                type="checkbox"
                {...({ switch: "" } as any)}
                checked={draft.icmpEchoIgnoreAll}
                onChange={(e) =>
                  setDraft((prev) =>
                    prev
                      ? { ...prev, icmpEchoIgnoreAll: e.target.checked }
                      : prev,
                  )
                }
              />
            </label>

            <div className="flex flex-wrap items-center gap-1.5 text-[10px] font-mono">
              <span
                className={`px-1.5 py-0.5 rounded border ${
                  draft.tcpSyncookies
                    ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                    : "bg-rose-500/15 text-rose-300 border-rose-500/30"
                }`}
              >
                syncookies={draft.tcpSyncookies ? 1 : 0}
              </span>
              <span
                className={`px-1.5 py-0.5 rounded border ${
                  draft.rpFilter
                    ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                    : "bg-rose-500/15 text-rose-300 border-rose-500/30"
                }`}
              >
                rp_filter={draft.rpFilter ? 1 : 0}
              </span>
              <span
                className={`px-1.5 py-0.5 rounded border ${
                  !draft.acceptRedirects
                    ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                    : "bg-rose-500/15 text-rose-300 border-rose-500/30"
                }`}
              >
                redirects={draft.acceptRedirects ? 1 : 0}
              </span>
              <span
                className={`px-1.5 py-0.5 rounded border ${
                  !draft.acceptSourceRoute
                    ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/30"
                    : "bg-rose-500/15 text-rose-300 border-rose-500/30"
                }`}
              >
                src_route={draft.acceptSourceRoute ? 1 : 0}
              </span>
            </div>
          </div>

          <button
            type="button"
            onClick={() =>
              setDraft((prev) =>
                prev
                  ? {
                      ...prev,
                      tcpSyncookies: true,
                      rpFilter: true,
                      acceptRedirects: false,
                      acceptSourceRoute: false,
                    }
                  : prev,
              )
            }
            className="w-full py-2 px-3 rounded-xl bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/35 text-emerald-300 text-xs font-semibold transition-all cursor-pointer flex items-center justify-center gap-1.5"
          >
            <Shield className="w-3.5 h-3.5" />
            <span>
              {isVi
                ? "Kích hoạt Khiên Chống Giả mạo Mạng (Stage Network Fortress)"
                : "Stage Network Fortress"}
            </span>
          </button>
        </section>
      </div>

      {/* ====================================================================
          ZONE 2 (LEFT/CENTER) & ZONE 3 (RIGHT): SUBSYSTEM GAUGES + LIVE INSPECTOR
          ==================================================================== */}
      <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-start">
        {/* ZONE 2: Left/Center 7 Cols — Interactive Sysctl Gauges & Range Sliders */}
        <div className="xl:col-span-7 space-y-5">
          {/* CARD A: Memory & Swap Behavior (vm.*) */}
          <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-5 shadow-xs">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-xl bg-violet-500/15 border border-violet-500/35 flex items-center justify-center text-violet-300">
                  <Database className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-sm font-bold text-foreground">
                    {isVi
                      ? "Bộ nhớ Ảo & Hành vi Swap (vm.*)"
                      : "Memory & Swap Behavior (vm.*)"}
                  </h3>
                  <p className="text-[11px] text-muted-foreground font-mono">
                    vm.swappiness · vm.overcommit_memory · vm.vfs_cache_pressure
                  </p>
                </div>
              </div>
            </div>

            {/* 1. Visual Swappiness Range Slider (0-100) */}
            <div className="rounded-xl border border-border/75 bg-background/50 p-4 space-y-3">
              <div className="flex items-center justify-between gap-2">
                <label
                  htmlFor={swappinessRangeId}
                  className="text-xs font-bold text-foreground font-mono"
                >
                  vm.swappiness ={" "}
                  <span className="text-primary">{draft.swappiness}</span>
                </label>
                <span className="text-[11px] text-muted-foreground font-mono">
                  {isVi ? "Đang chạy:" : "Live:"} {snapshot.swappiness}
                </span>
              </div>

              {/* Dual-Color Bar: RAM Retention vs Swap Aggressiveness */}
              <div className="space-y-1.5">
                <div className="flex items-center justify-between text-[10px] font-mono">
                  <span className="text-emerald-400 font-semibold">
                    {isVi
                      ? `Ưu tiên Giữ RAM (${ramRetentionPct}%)`
                      : `RAM Retention Preference (${ramRetentionPct}%)`}
                  </span>
                  <span className="text-amber-400 font-semibold">
                    {isVi
                      ? `Độ tích cực Swap (${swapAggressivenessPct}%)`
                      : `Swap Aggressiveness (${swapAggressivenessPct}%)`}
                  </span>
                </div>
                <div className="h-2.5 w-full rounded-full overflow-hidden bg-secondary flex border border-border/60">
                  <div
                    className="h-full bg-emerald-500/80 transition-all duration-150"
                    style={{ width: `${ramRetentionPct}%` }}
                  />
                  <div
                    className="h-full bg-amber-500/80 transition-all duration-150"
                    style={{ width: `${swapAggressivenessPct}%` }}
                  />
                </div>
              </div>

              <input
                id={swappinessRangeId}
                type="range"
                min={0}
                max={100}
                step={1}
                value={draft.swappiness}
                onChange={(e) =>
                  setDraft((prev) =>
                    prev
                      ? { ...prev, swappiness: Number(e.target.value) }
                      : prev,
                  )
                }
                className="w-full cursor-pointer"
              />

              {/* Quick Preset Chips */}
              <div className="flex flex-wrap items-center gap-2 pt-1">
                {[
                  {
                    val: 10,
                    label: isVi
                      ? "10 — Khuyên dùng cho Server"
                      : "10 — Server Recommended",
                  },
                  {
                    val: 1,
                    label: isVi
                      ? "1 — Database / Tối thiểu Swap"
                      : "1 — Database / Minimal Swap",
                  },
                  {
                    val: 60,
                    label: isVi ? "60 — Mặc định Linux" : "60 — Linux Default",
                  },
                ].map((preset) => (
                  <button
                    key={preset.val}
                    type="button"
                    onClick={() =>
                      setDraft((prev) =>
                        prev ? { ...prev, swappiness: preset.val } : prev,
                      )
                    }
                    className={`px-2.5 py-1 rounded-lg text-[11px] font-mono border transition-colors cursor-pointer ${
                      draft.swappiness === preset.val
                        ? "bg-primary/20 text-primary border-primary/45 font-bold"
                        : "bg-secondary/80 text-muted-foreground border-border/70 hover:text-foreground"
                    }`}
                  >
                    {preset.label}
                  </button>
                ))}
              </div>
            </div>

            {/* 2. Overcommit Memory Mode Selector (vm.overcommit_memory) */}
            <div className="space-y-2">
              <div className="flex items-center justify-between">
                <span className="text-xs font-bold font-mono text-foreground">
                  vm.overcommit_memory
                </span>
                <span className="text-[11px] font-mono text-muted-foreground">
                  {isVi ? "Đang chạy:" : "Live:"} {snapshot.overcommitMemory}
                </span>
              </div>
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-2.5">
                {[
                  {
                    mode: 0,
                    title: "0 — Heuristic Default",
                    desc: isVi
                      ? "Nhân Linux tự ước lượng RAM trống hợp lý"
                      : "Kernel heuristic overcommit (standard)",
                  },
                  {
                    mode: 1,
                    title: "1 — Always Overcommit (Redis / AI)",
                    desc: isVi
                      ? "Cần thiết cho Redis background save & ML"
                      : "Prevents fork() OOM for Redis BGSAVE & AI",
                  },
                  {
                    mode: 2,
                    title: "2 — Strict No-Overcommit",
                    desc: isVi
                      ? "Không cấp phát vượt quá giới hạn Swap + RAM"
                      : "Strict commit limit (Swap + RAM ratio)",
                  },
                ].map((item) => {
                  const active = draft.overcommitMemory === item.mode;
                  return (
                    <button
                      key={item.mode}
                      type="button"
                      onClick={() =>
                        setDraft((prev) =>
                          prev
                            ? { ...prev, overcommitMemory: item.mode }
                            : prev,
                        )
                      }
                      className={`p-3 rounded-xl border text-left transition-all cursor-pointer space-y-1 ${
                        active
                          ? "border-primary bg-primary/14 shadow-2xs"
                          : "border-border/75 bg-background/45 hover:bg-accent/35"
                      }`}
                    >
                      <div className="text-xs font-bold font-mono text-foreground">
                        {item.title}
                      </div>
                      <div className="text-[10px] text-muted-foreground leading-snug">
                        {item.desc}
                      </div>
                    </button>
                  );
                })}
              </div>
            </div>

            {/* 3. Inode/Dentry Cache Pressure (vm.vfs_cache_pressure) */}
            <div className="rounded-xl border border-border/75 bg-background/50 p-4 space-y-3">
              <div className="flex items-center justify-between gap-2">
                <label
                  htmlFor={vfsRangeId}
                  className="text-xs font-bold font-mono text-foreground"
                >
                  vm.vfs_cache_pressure ={" "}
                  <span className="text-primary">{draft.vfsCachePressure}</span>
                </label>
                <div className="flex items-center gap-1.5">
                  {[
                    {
                      val: 50,
                      label: "50 — Keep Cache Longer",
                    },
                    {
                      val: 100,
                      label: "100 — Default",
                    },
                  ].map((preset) => (
                    <button
                      key={preset.val}
                      type="button"
                      onClick={() =>
                        setDraft((prev) =>
                          prev
                            ? { ...prev, vfsCachePressure: preset.val }
                            : prev,
                        )
                      }
                      className={`px-2 py-0.5 rounded-lg text-[10px] font-mono border cursor-pointer ${
                        draft.vfsCachePressure === preset.val
                          ? "bg-primary/20 text-primary border-primary/45 font-bold"
                          : "bg-secondary text-muted-foreground border-border/70 hover:text-foreground"
                      }`}
                    >
                      {preset.label}
                    </button>
                  ))}
                </div>
              </div>
              <input
                id={vfsRangeId}
                type="range"
                min={10}
                max={200}
                step={5}
                value={draft.vfsCachePressure}
                onChange={(e) =>
                  setDraft((prev) =>
                    prev
                      ? { ...prev, vfsCachePressure: Number(e.target.value) }
                      : prev,
                  )
                }
                className="w-full cursor-pointer"
              />
            </div>
          </section>

          {/* CARD B: Network Backlog & Routing (net.*) */}
          <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-xl bg-cyan-500/15 border border-cyan-500/35 flex items-center justify-center text-cyan-300">
                <Network className="w-4 h-4" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-foreground">
                  {isVi
                    ? "Hàng đợi Kết nối Mạng & Định tuyến (net.*)"
                    : "Network Backlog & Routing (net.*)"}
                </h3>
                <p className="text-[11px] text-muted-foreground font-mono">
                  somaxconn · tcp_max_syn_backlog · tcp_fin_timeout · ip_forward
                </p>
              </div>
            </div>

            <div className="grid grid-cols-1 md:grid-cols-3 gap-3">
              {/* somaxconn */}
              <div className="rounded-xl border border-border/75 bg-background/50 p-3.5 space-y-2.5">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-mono font-bold text-foreground">
                    net.core.somaxconn
                  </span>
                </div>
                <div className="text-base font-extrabold font-mono text-primary">
                  {draft.somaxconn.toLocaleString()}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {[4096, 16384, 65535].map((val) => (
                    <button
                      key={val}
                      type="button"
                      onClick={() =>
                        setDraft((prev) =>
                          prev ? { ...prev, somaxconn: val } : prev,
                        )
                      }
                      className={`px-2 py-0.5 rounded-md text-[10px] font-mono border cursor-pointer ${
                        draft.somaxconn === val
                          ? "bg-primary/20 text-primary border-primary/45 font-bold"
                          : "bg-secondary text-muted-foreground border-border/60 hover:text-foreground"
                      }`}
                    >
                      {val}
                    </button>
                  ))}
                </div>
              </div>

              {/* tcp_max_syn_backlog */}
              <div className="rounded-xl border border-border/75 bg-background/50 p-3.5 space-y-2.5">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-mono font-bold text-foreground">
                    tcp_max_syn_backlog
                  </span>
                </div>
                <div className="text-base font-extrabold font-mono text-primary">
                  {draft.tcpMaxSynBacklog.toLocaleString()}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {[4096, 8192, 16384].map((val) => (
                    <button
                      key={val}
                      type="button"
                      onClick={() =>
                        setDraft((prev) =>
                          prev ? { ...prev, tcpMaxSynBacklog: val } : prev,
                        )
                      }
                      className={`px-2 py-0.5 rounded-md text-[10px] font-mono border cursor-pointer ${
                        draft.tcpMaxSynBacklog === val
                          ? "bg-primary/20 text-primary border-primary/45 font-bold"
                          : "bg-secondary text-muted-foreground border-border/60 hover:text-foreground"
                      }`}
                    >
                      {val}
                    </button>
                  ))}
                </div>
              </div>

              {/* tcp_fin_timeout */}
              <div className="rounded-xl border border-border/75 bg-background/50 p-3.5 space-y-2.5">
                <div className="flex items-center justify-between">
                  <span className="text-[11px] font-mono font-bold text-foreground">
                    tcp_fin_timeout
                  </span>
                </div>
                <div className="text-base font-extrabold font-mono text-primary">
                  {draft.tcpFinTimeout}s
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {[15, 30, 60].map((val) => (
                    <button
                      key={val}
                      type="button"
                      onClick={() =>
                        setDraft((prev) =>
                          prev ? { ...prev, tcpFinTimeout: val } : prev,
                        )
                      }
                      className={`px-2 py-0.5 rounded-md text-[10px] font-mono border cursor-pointer ${
                        draft.tcpFinTimeout === val
                          ? "bg-primary/20 text-primary border-primary/45 font-bold"
                          : "bg-secondary text-muted-foreground border-border/60 hover:text-foreground"
                      }`}
                    >
                      {val}s
                    </button>
                  ))}
                </div>
              </div>
            </div>

            <div className="divide-y divide-border/60 rounded-xl border border-border/75 bg-background/45 overflow-hidden">
              <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                <div>
                  <div className="text-xs font-semibold font-mono text-foreground">
                    net.ipv4.tcp_tw_reuse
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Cho phép tái sử dụng socket ở trạng thái TIME_WAIT cho kết nối mới (Reverse Proxy / Load Balancer)"
                      : "Reuse sockets in TIME_WAIT state for outgoing connections when safe from protocol viewpoint"}
                  </div>
                </div>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={draft.tcpTwReuse}
                  onChange={(e) =>
                    setDraft((prev) =>
                      prev ? { ...prev, tcpTwReuse: e.target.checked } : prev,
                    )
                  }
                />
              </label>

              <label className="flex items-center justify-between gap-3 px-3.5 py-3 cursor-pointer hover:bg-accent/30 transition-colors">
                <div>
                  <div className="text-xs font-semibold font-mono text-foreground">
                    net.ipv4.ip_forward (Docker / WireGuard / VPN Routing)
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Bắt buộc bật cho Docker bridge networks, Kubernetes, WireGuard và Tailscale subnet router"
                      : "Required for Docker container bridge NAT, Kubernetes pods, and WireGuard/OpenVPN gateways"}
                  </div>
                </div>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={draft.ipForward}
                  onChange={(e) =>
                    setDraft((prev) =>
                      prev ? { ...prev, ipForward: e.target.checked } : prev,
                    )
                  }
                />
              </label>
            </div>
          </section>

          {/* CARD C: Filesystem & Kernel Security (fs.* & net.ipv4.conf.*) */}
          <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
            <div className="flex items-center gap-2.5">
              <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/35 flex items-center justify-center text-emerald-300">
                <HardDrive className="w-4 h-4" />
              </div>
              <div>
                <h3 className="text-sm font-bold text-foreground">
                  {isVi
                    ? "Giới hạn Filesystem & Bảo mật Nhân (fs.* & net.ipv4.conf.*)"
                    : "Filesystem & Kernel Security (fs.* & net.ipv4.conf.*)"}
                </h3>
                <p className="text-[11px] text-muted-foreground font-mono">
                  fs.file-max · tcp_syncookies · rp_filter · accept_redirects
                </p>
              </div>
            </div>

            {/* fs.file-max preset selector */}
            <div className="rounded-xl border border-border/75 bg-background/50 p-3.5 flex flex-wrap items-center justify-between gap-3">
              <div>
                <div className="text-xs font-bold font-mono text-foreground">
                  fs.file-max ={" "}
                  <span className="text-primary">
                    {draft.fileMax.toLocaleString()}
                  </span>
                </div>
                <div className="text-[11px] text-muted-foreground">
                  {isVi
                    ? "Số lượng File Descriptors tối đa toàn hệ thống"
                    : "Maximum system-wide open file descriptors"}
                </div>
              </div>
              <div className="flex flex-wrap items-center gap-1.5">
                {[524288, 1048576, 2097152].map((val) => (
                  <button
                    key={val}
                    type="button"
                    onClick={() =>
                      setDraft((prev) =>
                        prev ? { ...prev, fileMax: val } : prev,
                      )
                    }
                    className={`px-2.5 py-1 rounded-lg text-[11px] font-mono border cursor-pointer ${
                      draft.fileMax === val
                        ? "bg-primary/20 text-primary border-primary/45 font-bold"
                        : "bg-secondary text-muted-foreground border-border/70 hover:text-foreground"
                    }`}
                  >
                    {val}
                  </button>
                ))}
              </div>
            </div>

            {/* Security Switches */}
            <div className="divide-y divide-border/60 rounded-xl border border-border/75 bg-background/45 overflow-hidden">
              <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                <div>
                  <div className="text-xs font-semibold font-mono text-foreground">
                    net.ipv4.tcp_syncookies
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Chống tấn công làm tràn hàng đợi TCP SYN Flood DDoS"
                      : "Mitigate TCP SYN flood attacks when the SYN backlog queue fills up"}
                  </div>
                </div>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={draft.tcpSyncookies}
                  onChange={(e) =>
                    setDraft((prev) =>
                      prev
                        ? { ...prev, tcpSyncookies: e.target.checked }
                        : prev,
                    )
                  }
                />
              </label>

              <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                <div>
                  <div className="text-xs font-semibold font-mono text-foreground">
                    net.ipv4.conf.all.rp_filter
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Kiểm tra đường dẫn ngược (Reverse Path Filtering) chống giả mạo IP nguồn"
                      : "Strict source validation by reversed path (RFC 3704 anti-IP-spoofing)"}
                  </div>
                </div>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={draft.rpFilter}
                  onChange={(e) =>
                    setDraft((prev) =>
                      prev ? { ...prev, rpFilter: e.target.checked } : prev,
                    )
                  }
                />
              </label>

              <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                <div>
                  <div className="text-xs font-semibold font-mono text-foreground">
                    net.ipv4.conf.all.accept_redirects
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Tắt (OFF) để ngăn chặn tấn công Man-in-the-Middle bằng ICMP Redirect"
                      : "Keep OFF on servers to prevent malicious ICMP routing table redirects"}
                  </div>
                </div>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={draft.acceptRedirects}
                  onChange={(e) =>
                    setDraft((prev) =>
                      prev
                        ? { ...prev, acceptRedirects: e.target.checked }
                        : prev,
                    )
                  }
                />
              </label>

              <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                <div>
                  <div className="text-xs font-semibold font-mono text-foreground">
                    net.ipv4.conf.all.accept_source_route
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Tắt (OFF) để từ chối gói tin tự chỉ định lộ trình định tuyến"
                      : "Keep OFF to reject source-routed packets that bypass routing policies"}
                  </div>
                </div>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={draft.acceptSourceRoute}
                  onChange={(e) =>
                    setDraft((prev) =>
                      prev
                        ? { ...prev, acceptSourceRoute: e.target.checked }
                        : prev,
                    )
                  }
                />
              </label>

              <label className="flex items-center justify-between gap-3 px-3.5 py-2.5 cursor-pointer hover:bg-accent/30 transition-colors">
                <div>
                  <div className="text-xs font-semibold font-mono text-foreground">
                    net.ipv4.icmp_echo_ignore_all
                  </div>
                  <div className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Bật (ON) để ẩn máy chủ khỏi các công cụ quét mạng ICMP Ping"
                      : "Enable to drop all incoming ICMP echo requests (Ping Stealth Mode)"}
                  </div>
                </div>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  checked={draft.icmpEchoIgnoreAll}
                  onChange={(e) =>
                    setDraft((prev) =>
                      prev
                        ? { ...prev, icmpEchoIgnoreAll: e.target.checked }
                        : prev,
                    )
                  }
                />
              </label>
            </div>
          </section>
        </div>

        {/* ZONE 3: Right 5 Cols — Live /etc/sysctl.d/99-sshbox.conf Inspector & Apply Bar */}
        <div className="xl:col-span-5 space-y-4 xl:sticky xl:top-2">
          <section className="rounded-2xl border border-border/80 bg-card/80 p-5 space-y-4 shadow-xs">
            <div className="flex items-center justify-between gap-2">
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/35 flex items-center justify-center text-emerald-300">
                  <FileDiff className="w-4 h-4" />
                </div>
                <div>
                  <h3 className="text-sm font-bold font-mono text-foreground">
                    /etc/sysctl.d/99-sshbox.conf
                  </h3>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Xem trước thay đổi & cấu hình bền vững qua khởi động lại"
                      : "Live sysctl diff & persistent configuration preview"}
                  </p>
                </div>
              </div>

              <span
                className={`px-2.5 py-1 rounded-full text-[11px] font-mono font-bold border ${
                  changedItems.length > 0
                    ? "bg-amber-500/20 text-amber-300 border-amber-500/40"
                    : "bg-emerald-500/15 text-emerald-300 border-emerald-500/35"
                }`}
              >
                {changedItems.length > 0
                  ? `${changedItems.length} staged`
                  : isVi
                    ? "Đồng bộ"
                    : "In Sync"}
              </span>
            </div>

            {/* Staged Changes Pills List */}
            {changedItems.length > 0 ? (
              <div className="rounded-xl border border-amber-500/35 bg-amber-500/8 p-3 space-y-2">
                <div className="text-[11px] font-bold text-amber-300 flex items-center gap-1.5">
                  <Activity className="w-3.5 h-3.5" />
                  <span>
                    {isVi
                      ? "Các tham số chuẩn bị áp dụng (live → staged):"
                      : "Staged sysctl modifications (live → staged):"}
                  </span>
                </div>
                <div className="space-y-1.5 max-h-48 overflow-y-auto pr-1">
                  {changedItems.map((item) => (
                    <div
                      key={item.key}
                      className="flex items-center justify-between gap-2 px-2.5 py-1.5 rounded-lg bg-background/70 border border-border/70 text-[11px] font-mono"
                    >
                      <span className="text-foreground truncate">
                        {item.key}
                      </span>
                      <div className="flex items-center gap-1.5 shrink-0">
                        <span className="px-1.5 py-0.5 rounded bg-amber-500/15 text-amber-300 border border-amber-500/30">
                          {item.before}
                        </span>
                        <span className="text-muted-foreground">→</span>
                        <span className="px-1.5 py-0.5 rounded bg-emerald-500/20 text-emerald-300 border border-emerald-500/35 font-bold">
                          {item.after}
                        </span>
                      </div>
                    </div>
                  ))}
                </div>
              </div>
            ) : (
              <div className="rounded-xl border border-border/70 bg-background/45 p-3 text-xs text-muted-foreground flex items-center gap-2">
                <Check className="w-4 h-4 text-emerald-400 shrink-0" />
                <span>
                  {isVi
                    ? "Mọi tham số trên giao diện đang khớp với nhân Linux thực tế."
                    : "All staged controls match the running Linux kernel values."}
                </span>
              </div>
            )}

            {/* Exact /etc/sysctl.d/99-sshbox.conf Preview */}
            <div className="space-y-1.5">
              <div className="flex items-center justify-between text-[11px] font-mono text-muted-foreground">
                <span>/etc/sysctl.d/99-sshbox.conf</span>
                <span>chmod 0644 · sysctl -p</span>
              </div>
              <pre className="p-3.5 rounded-xl bg-background/85 border border-border/80 text-[11px] font-mono text-foreground overflow-x-auto leading-relaxed max-h-80">
                {sysctlConfPreview}
              </pre>
            </div>

            {/* Action Buttons: Reset Changes & Apply & Persist (sysctl -p) */}
            <div className="flex items-center gap-2.5 pt-1">
              <button
                type="button"
                disabled={applying || changedItems.length === 0}
                onClick={handleResetChanges}
                className="px-3.5 py-2.5 rounded-xl bg-secondary hover:bg-accent text-xs font-semibold text-foreground border border-border/75 disabled:opacity-45 cursor-pointer inline-flex items-center gap-1.5"
              >
                <RotateCcw className="w-3.5 h-3.5" />
                <span>{isVi ? "Reset Changes" : "Reset Changes"}</span>
              </button>

              <button
                type="button"
                disabled={applying}
                onClick={handleApplyKernelTuning}
                className="flex-1 py-2.5 px-4 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-95 disabled:opacity-50 cursor-pointer inline-flex items-center justify-center gap-2 shadow-sm"
              >
                {applying ? (
                  <>
                    <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                    <span>
                      {isVi
                        ? "Đang ghi & chạy sysctl -p..."
                        : "Applying sysctl -p..."}
                    </span>
                  </>
                ) : (
                  <>
                    <Cpu className="w-3.5 h-3.5" />
                    <span>Apply &amp; Persist (sysctl -p)</span>
                  </>
                )}
              </button>
            </div>
          </section>
        </div>
      </div>
    </div>
  );
};
