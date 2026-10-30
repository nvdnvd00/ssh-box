import React, { useCallback, useEffect, useId, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CheckCircle2,
  Flame,
  Globe,
  Lock,
  Network,
  Plus,
  Radar,
  RefreshCw,
  Search,
  Shield,
  ShieldAlert,
  ShieldCheck,
  ShieldOff,
  Trash2,
  X,
  Zap,
} from "lucide-react";
import {
  FirewallSnapshot,
  HostEntry,
  ListeningSocketItem,
  VpsPrivilegeInfo,
} from "../../types";
import { AppLanguage } from "../../themes";

interface FirewallStudioProps {
  host: HostEntry;
  sudoPassword?: string;
  privilegeInfo: VpsPrivilegeInfo | null;
  language?: AppLanguage;
  onRequireSudoUnlock?: () => void;
}

type SocketFilter = "all" | "exposed" | "restricted" | "blocked" | "loopback";
type RuleAction = "ALLOW" | "LIMIT" | "DENY";
type RuleProto = "tcp" | "udp" | "any";
type SourceScope = "anywhere" | "my_ip" | "custom";

export const FirewallStudio: React.FC<FirewallStudioProps> = ({
  host,
  sudoPassword,
  privilegeInfo,
  language = "vi",
  onRequireSudoUnlock,
}) => {
  const isVi = language === "vi";
  const socketsCaptionId = useId();
  const rulesCaptionId = useId();

  const [snapshot, setSnapshot] = useState<FirewallSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  // Client IP override when detectedClientIp is empty or private/LAN
  const [manualClientIp, setManualClientIp] = useState("");

  // Zone 2: Socket Radar filters
  const [socketFilter, setSocketFilter] = useState<SocketFilter>("all");
  const [socketSearch, setSocketSearch] = useState("");

  // Zone 3: Visual Rule Composer state
  const [composerAction, setComposerAction] = useState<RuleAction>("ALLOW");
  const [composerPort, setComposerPort] = useState("443");
  const [composerProto, setComposerProto] = useState<RuleProto>("tcp");
  const [composerSourceScope, setComposerSourceScope] =
    useState<SourceScope>("anywhere");
  const [composerCustomSource, setComposerCustomSource] =
    useState("203.0.113.0/24");
  const [composerComment, setComposerComment] = useState("");

  const fetchSnapshot = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const res = await invoke<FirewallSnapshot>("vps_get_firewall_snapshot", {
        hostId: host.id,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(res);
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
    const timer = setTimeout(() => setToastMsg(null), 4500);
    return () => clearTimeout(timer);
  }, [toastMsg]);

  const effectiveClientIp = useMemo(() => {
    const manual = manualClientIp.trim();
    if (manual) return manual;
    if (snapshot?.detectedClientIp?.trim()) {
      return snapshot.detectedClientIp.trim();
    }
    if (privilegeInfo?.detectedClientIp?.trim()) {
      return privilegeInfo.detectedClientIp.trim();
    }
    return "";
  }, [manualClientIp, snapshot?.detectedClientIp, privilegeInfo?.detectedClientIp]);

  const effectiveSshPort = useMemo(
    () => snapshot?.sshPort || host.port || 22,
    [snapshot?.sshPort, host.port],
  );

  const handleSetFirewallState = async (
    nextEnabled: boolean,
    nextIncoming: string,
  ) => {
    setBusyAction("state");
    setErrorMsg(null);
    try {
      const updated = await invoke<FirewallSnapshot>("vps_set_firewall_state", {
        hostId: host.id,
        enabled: nextEnabled,
        defaultIncoming: nextIncoming,
        sshPort: effectiveSshPort,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(updated);
      setToastMsg(
        isVi
          ? `Đã cập nhật trạng thái tường lửa (${updated.enabled ? "BẬT" : "TẮT"} • Incoming: ${updated.defaultIncoming.toUpperCase()}) với bảo vệ chống khóa cổng SSH :${effectiveSshPort}!`
          : `Updated firewall state (${updated.enabled ? "ACTIVE" : "INACTIVE"} • Incoming: ${updated.defaultIncoming.toUpperCase()}) with SSH :${effectiveSshPort} anti-lockout protection!`,
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setBusyAction(null);
    }
  };

  const handleQuickAddRule = async (
    action: string,
    portSpec: string,
    protocol: string,
    sourceIp: string,
    comment: string,
    busyKey: string,
  ) => {
    setBusyAction(busyKey);
    setErrorMsg(null);
    try {
      const updated = await invoke<FirewallSnapshot>("vps_add_firewall_rule", {
        hostId: host.id,
        action,
        portSpec,
        protocol,
        sourceIp,
        comment,
        sudoPassword: sudoPassword || null,
      });
      setSnapshot(updated);
      setToastMsg(
        isVi
          ? `Đã áp dụng quy tắc tường lửa: ${action} ${portSpec}/${protocol} (${sourceIp || "Anywhere"})`
          : `Added firewall rule: ${action} ${portSpec}/${protocol} from ${sourceIp || "Anywhere"}`,
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setBusyAction(null);
    }
  };

  const handleDeleteRule = async (ruleIndex: number, rawSpec: string) => {
    setBusyAction(`del-${ruleIndex}`);
    setErrorMsg(null);
    try {
      const updated = await invoke<FirewallSnapshot>(
        "vps_delete_firewall_rule",
        {
          hostId: host.id,
          ruleIndex,
          rawSpec,
          sudoPassword: sudoPassword || null,
        },
      );
      setSnapshot(updated);
      setToastMsg(
        isVi
          ? `Đã xóa quy tắc tường lửa #${ruleIndex}.`
          : `Deleted firewall rule #${ruleIndex}.`,
      );
    } catch (err) {
      setErrorMsg(String(err));
    } finally {
      setBusyAction(null);
    }
  };

  const handleSubmitComposer = async (e: React.FormEvent) => {
    e.preventDefault();
    const resolvedSource =
      composerSourceScope === "anywhere"
        ? "Anywhere"
        : composerSourceScope === "my_ip"
          ? effectiveClientIp
          : composerCustomSource.trim();

    if (composerSourceScope !== "anywhere" && !resolvedSource) {
      setErrorMsg(
        isVi
          ? "Vui lòng nhập địa chỉ IP nguồn (My IP hoặc CIDR tùy chỉnh) trước khi thêm quy tắc."
          : "Please enter a source IP or CIDR before adding an IP-restricted rule.",
      );
      return;
    }

    await handleQuickAddRule(
      composerAction,
      composerPort.trim() || "any",
      composerProto,
      resolvedSource,
      composerComment.trim(),
      "composer-submit",
    );
    setComposerComment("");
  };

  const socketCounts = useMemo(() => {
    const list = snapshot?.listeningSockets ?? [];
    return {
      all: list.length,
      exposed: list.filter((s) => s.exposure === "exposed").length,
      restricted: list.filter((s) => s.exposure === "restricted").length,
      blocked: list.filter((s) => s.exposure === "blocked").length,
      loopback: list.filter((s) => s.exposure === "loopback").length,
    };
  }, [snapshot?.listeningSockets]);

  const filteredSockets = useMemo(() => {
    const list = snapshot?.listeningSockets ?? [];
    const q = socketSearch.trim().toLowerCase();
    return list.filter((sock) => {
      if (socketFilter !== "all" && sock.exposure !== socketFilter) {
        return false;
      }
      if (!q) return true;
      return (
        String(sock.port).includes(q) ||
        sock.processName.toLowerCase().includes(q) ||
        sock.bindAddr.toLowerCase().includes(q) ||
        sock.protocol.toLowerCase().includes(q)
      );
    });
  }, [snapshot?.listeningSockets, socketFilter, socketSearch]);

  const renderExposureBadge = (sock: ListeningSocketItem) => {
    switch (sock.exposure) {
      case "exposed":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-amber-500/15 text-amber-300 border border-amber-500/35">
            <Globe className="w-2.5 h-2.5 text-amber-400" />
            <span>EXPOSED PUBLIC</span>
          </span>
        );
      case "restricted":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-cyan-500/15 text-cyan-300 border border-cyan-500/35">
            <ShieldCheck className="w-2.5 h-2.5 text-cyan-400" />
            <span>IP RESTRICTED</span>
          </span>
        );
      case "blocked":
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-emerald-500/15 text-emerald-300 border border-emerald-500/35">
            <Lock className="w-2.5 h-2.5 text-emerald-400" />
            <span>SHIELDED / BLOCKED</span>
          </span>
        );
      default:
        return (
          <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-full text-[10px] font-mono font-bold bg-secondary text-muted-foreground border border-border">
            <Shield className="w-2.5 h-2.5 text-emerald-400" />
            <span>LOOPBACK ONLY</span>
          </span>
        );
    }
  };

  if (loading && !snapshot) {
    return (
      <div className="p-8 flex flex-col items-center justify-center gap-3 text-muted-foreground">
        <RefreshCw className="w-6 h-6 animate-spin text-primary" />
        <p className="text-xs font-medium">
          {isVi
            ? "Đang quét tường lửa (UFW / iptables) và radar cổng lắng nghe (ss -tulpn)..."
            : "Scanning firewall engine (UFW / iptables) and live listening sockets (ss -tulpn)..."}
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
                  ? "Không thể thực thi tác vụ Tường lửa"
                  : "Firewall Operation Failed"}
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

      {/* ZONE 1: Top Command Bar & Client IP Shield Banner */}
      {snapshot && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-4 items-stretch">
          {/* Firewall Engine Master Control Card */}
          <div className="xl:col-span-7 rounded-2xl border border-border/80 bg-card/75 p-4 flex flex-wrap items-center justify-between gap-4 shadow-xs">
            <div className="flex items-center gap-3">
              <div
                className={`w-10 h-10 rounded-2xl border flex items-center justify-center shrink-0 ${
                  snapshot.enabled
                    ? "bg-emerald-500/15 border-emerald-500/35 text-emerald-400"
                    : "bg-amber-500/15 border-amber-500/35 text-amber-400"
                }`}
              >
                {snapshot.enabled ? (
                  <Flame className="w-5 h-5" />
                ) : (
                  <ShieldOff className="w-5 h-5" />
                )}
              </div>

              <div>
                <div className="flex flex-wrap items-center gap-2">
                  <span className="px-2 py-0.5 rounded-md bg-secondary border border-border font-mono text-[11px] font-bold uppercase text-foreground">
                    {snapshot.engine.toUpperCase()}
                  </span>
                  <span
                    className={`px-2 py-0.5 rounded-full text-[10px] font-mono font-bold uppercase border ${
                      snapshot.enabled
                        ? "bg-emerald-500/15 text-emerald-400 border-emerald-500/30"
                        : "bg-rose-500/15 text-rose-400 border-rose-500/30"
                    }`}
                  >
                    {snapshot.enabled
                      ? isVi
                        ? "ĐANG BẬT (ACTIVE)"
                        : "ACTIVE"
                      : isVi
                        ? "ĐANG TẮT (INACTIVE)"
                        : "INACTIVE"}
                  </span>
                  <span className="inline-flex items-center gap-1 px-2.5 py-0.5 rounded-full bg-emerald-500/10 border border-emerald-500/30 text-[10px] font-mono font-semibold text-emerald-300">
                    <ShieldCheck className="w-3 h-3 text-emerald-400" />
                    <span>SSH Port :{effectiveSshPort} Protected</span>
                  </span>
                </div>
                <p className="text-[11px] text-muted-foreground mt-1">
                  {isVi
                    ? `Tự động mở cổng SSH :${effectiveSshPort}/tcp trước khi bật UFW hoặc chặn kết nối đến.`
                    : `Anti-lockout guard always allows SSH :${effectiveSshPort}/tcp before enabling UFW or denying incoming traffic.`}
                </p>
              </div>
            </div>

            <div className="flex flex-wrap items-center gap-3">
              {/* Default Incoming Policy Segmented Toggle */}
              <div className="flex items-center p-1 rounded-xl bg-secondary/80 border border-border/80">
                <button
                  type="button"
                  disabled={busyAction !== null}
                  onClick={() =>
                    handleSetFirewallState(snapshot.enabled, "deny")
                  }
                  className={`px-2.5 py-1.5 rounded-lg text-[11px] font-bold transition-all cursor-pointer ${
                    snapshot.defaultIncoming !== "allow"
                      ? "bg-emerald-500 text-slate-950 shadow-2xs"
                      : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  {isVi ? "DENY INCOMING (Khuyên dùng)" : "DENY INCOMING (Rec.)"}
                </button>
                <button
                  type="button"
                  disabled={busyAction !== null}
                  onClick={() =>
                    handleSetFirewallState(snapshot.enabled, "allow")
                  }
                  className={`px-2.5 py-1.5 rounded-lg text-[11px] font-bold transition-all cursor-pointer ${
                    snapshot.defaultIncoming === "allow"
                      ? "bg-amber-500 text-slate-950 shadow-2xs"
                      : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  ALLOW INCOMING
                </button>
              </div>

              {/* Master Firewall Switch */}
              <label className="flex items-center gap-2.5 px-3 py-2 rounded-xl bg-secondary/70 border border-border cursor-pointer hover:bg-accent/50 transition-colors">
                <span className="text-xs font-bold text-foreground">
                  {isVi ? "Tường lửa" : "Firewall"}
                </span>
                <input
                  type="checkbox"
                  {...({ switch: "" } as any)}
                  disabled={busyAction !== null}
                  checked={snapshot.enabled}
                  onChange={(e) =>
                    handleSetFirewallState(
                      e.target.checked,
                      snapshot.defaultIncoming || "deny",
                    )
                  }
                />
              </label>

              <button
                type="button"
                onClick={fetchSnapshot}
                disabled={loading || busyAction !== null}
                title="Refresh sockets & rules"
                className="p-2 rounded-xl bg-secondary hover:bg-accent border border-border text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
              >
                <RefreshCw
                  className={`w-3.5 h-3.5 ${loading ? "animate-spin text-primary" : ""}`}
                />
              </button>
            </div>
          </div>

          {/* Client IP Shield Banner */}
          <div className="xl:col-span-5 rounded-2xl border border-cyan-500/35 bg-cyan-500/8 p-4 flex flex-col justify-between gap-3 shadow-xs">
            <div className="flex items-start justify-between gap-2">
              <div className="flex items-center gap-2">
                <div className="w-8 h-8 rounded-xl bg-cyan-500/15 border border-cyan-500/30 flex items-center justify-center text-cyan-400 shrink-0">
                  <Shield className="w-4 h-4" />
                </div>
                <div>
                  <div className="text-xs font-bold text-foreground flex items-center gap-2">
                    <span>
                      {isVi
                        ? "Khiên Bảo Vệ IP Quản Trị (My IP Shield)"
                        : "Operator Client IP Shield"}
                    </span>
                  </div>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Tự động phát hiện từ phiên SSH hoặc nhập IP tĩnh/VPN của bạn:"
                      : "Detected from active SSH session or specify your admin IP/CIDR:"}
                  </p>
                </div>
              </div>

              <input
                type="text"
                value={manualClientIp || effectiveClientIp}
                onChange={(e) => setManualClientIp(e.target.value)}
                placeholder="e.g. 203.0.113.42"
                aria-label="Operator Client IP"
                className="ui-input !w-36 !h-8 font-mono text-xs font-bold text-cyan-300"
              />
            </div>

            <div className="flex flex-wrap items-center gap-2">
              <button
                type="button"
                disabled={!effectiveClientIp || busyAction !== null}
                onClick={() =>
                  handleQuickAddRule(
                    "ALLOW",
                    "any",
                    "any",
                    effectiveClientIp,
                    "Operator IP Full Whitelist",
                    "whitelist-all",
                  )
                }
                className="flex-1 min-w-[165px] py-1.5 px-3 rounded-xl bg-cyan-500/20 hover:bg-cyan-500/30 border border-cyan-500/40 text-cyan-200 text-xs font-semibold inline-flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-45 transition-colors"
              >
                <ShieldCheck className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                <span>
                  {isVi
                    ? "Whitelist My IP (Mọi cổng)"
                    : "Whitelist My IP (All Ports)"}
                </span>
              </button>

              <button
                type="button"
                disabled={!effectiveClientIp || busyAction !== null}
                onClick={() =>
                  handleQuickAddRule(
                    "ALLOW",
                    String(effectiveSshPort),
                    "tcp",
                    effectiveClientIp,
                    `Admin SSH :${effectiveSshPort}`,
                    "whitelist-ssh",
                  )
                }
                className="flex-1 min-w-[165px] py-1.5 px-3 rounded-xl bg-emerald-500/20 hover:bg-emerald-500/30 border border-emerald-500/40 text-emerald-200 text-xs font-semibold inline-flex items-center justify-center gap-1.5 cursor-pointer disabled:opacity-45 transition-colors"
              >
                <Lock className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                <span>
                  {isVi
                    ? `Whitelist My IP trên SSH (:${effectiveSshPort})`
                    : `Whitelist My IP on SSH (:${effectiveSshPort})`}
                </span>
              </button>
            </div>
          </div>
        </div>
      )}

      {/* SPLIT SCREEN: ZONE 2 (Left Live Socket Radar) & ZONE 3 (Right Presets, Rule Composer & Matrix) */}
      {snapshot && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-start">
          {/* ZONE 2: Left Split Pane — Live Listening Socket Radar (7 cols on XL) */}
          <section className="xl:col-span-7 rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
            <div className="flex flex-wrap items-center justify-between gap-3">
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-xl bg-orange-500/15 border border-orange-500/30 flex items-center justify-center text-orange-400">
                  <Radar className="w-4 h-4" />
                </div>
                <div>
                  <h2
                    id={socketsCaptionId}
                    className="text-xs font-bold uppercase tracking-wider text-foreground"
                  >
                    {isVi
                      ? "Radar Cổng Đang Lắng Nghe Trực Tiếp (ss -tulpn)"
                      : "Live Listening Socket Radar (ss -tulpn)"}
                  </h2>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Đối chiếu tức thời mọi tiến trình mở cổng mạng với trạng thái phơi nhiễm trên tường lửa."
                      : "Cross-references every listening daemon socket against active firewall rules in real time."}
                  </p>
                </div>
              </div>

              {/* Search by port or process */}
              <div className="relative w-full sm:w-52">
                <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
                <input
                  type="text"
                  value={socketSearch}
                  onChange={(e) => setSocketSearch(e.target.value)}
                  placeholder={
                    isVi ? "Tìm cổng, tiến trình..." : "Filter port, process..."
                  }
                  aria-label="Filter listening sockets"
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
                    label: isVi ? "Tất cả cổng" : "All Sockets",
                    count: socketCounts.all,
                  },
                  {
                    id: "exposed",
                    label: isVi ? "Phơi nhiễm Public" : "Public Exposed",
                    count: socketCounts.exposed,
                  },
                  {
                    id: "restricted",
                    label: isVi ? "Giới hạn IP" : "Restricted",
                    count: socketCounts.restricted,
                  },
                  {
                    id: "blocked",
                    label: isVi ? "Đã chặn" : "Blocked",
                    count: socketCounts.blocked,
                  },
                  {
                    id: "loopback",
                    label: "Loopback (127.0.0.1)",
                    count: socketCounts.loopback,
                  },
                ] as const
              ).map((pill) => {
                const active = socketFilter === pill.id;
                return (
                  <button
                    key={pill.id}
                    type="button"
                    onClick={() => setSocketFilter(pill.id)}
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

            {/* Sticky Responsive Table */}
            <div
              role="region"
              tabIndex={0}
              aria-labelledby={socketsCaptionId}
              className="vps-table-wrapper rounded-xl border border-border/80 bg-background/50 max-h-[540px]"
            >
              <table className="w-full text-left text-xs">
                <caption className="sr-only">
                  Live listening network sockets and firewall exposure status
                </caption>
                <thead className="bg-secondary/95 text-[11px] uppercase tracking-wider text-muted-foreground">
                  <tr>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "Cổng / Giao thức" : "Port / Proto"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "Tiến trình (PID)" : "Process / PID"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      Bind Address
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border">
                      {isVi ? "Trạng thái Phơi nhiễm" : "Live Exposure"}
                    </th>
                    <th className="py-2.5 px-3 font-semibold border-b border-border text-right">
                      {isVi ? "Thao tác 1-Click" : "1-Click Shield Actions"}
                    </th>
                  </tr>
                </thead>
                <tbody className="divide-y divide-border/60">
                  {filteredSockets.length === 0 ? (
                    <tr>
                      <td
                        colSpan={5}
                        className="py-8 px-4 text-center text-xs text-muted-foreground"
                      >
                        {isVi
                          ? "Không có cổng lắng nghe nào khớp với bộ lọc hiện tại."
                          : "No listening sockets match the current filter."}
                      </td>
                    </tr>
                  ) : (
                    filteredSockets.map((sock) => {
                      const rowKey = `${sock.protocol}-${sock.bindAddr}-${sock.port}`;
                      return (
                        <tr
                          key={rowKey}
                          className="hover:bg-accent/30 transition-colors"
                        >
                          <th
                            scope="row"
                            className="py-2.5 px-3 font-mono font-bold text-foreground whitespace-nowrap"
                          >
                            <div className="inline-flex items-center gap-1.5">
                              <span className="px-2 py-0.5 rounded-lg bg-secondary border border-border text-xs text-foreground">
                                :{sock.port}
                              </span>
                              <span className="px-1.5 py-0.5 rounded-md bg-primary/15 text-primary text-[10px] uppercase font-bold">
                                {sock.protocol}
                              </span>
                            </div>
                          </th>

                          <td className="py-2.5 px-3 whitespace-nowrap">
                            <div className="inline-flex items-center gap-1.5 font-mono text-xs text-foreground">
                              <span className="font-semibold">
                                {sock.processName}
                              </span>
                              {sock.pid && (
                                <span className="text-[10px] text-muted-foreground bg-secondary px-1.5 py-0.5 rounded">
                                  PID {sock.pid}
                                </span>
                              )}
                            </div>
                          </td>

                          <td className="py-2.5 px-3 font-mono text-[11px] whitespace-nowrap">
                            <span
                              className={
                                sock.isPublic
                                  ? "text-amber-300 font-semibold"
                                  : "text-muted-foreground"
                              }
                            >
                              {sock.bindAddr}
                            </span>
                          </td>

                          <td className="py-2.5 px-3 whitespace-nowrap">
                            {renderExposureBadge(sock)}
                          </td>

                          <td className="py-2.5 px-3 text-right whitespace-nowrap">
                            {sock.isPublic ? (
                              <div className="inline-flex items-center justify-end gap-1">
                                <button
                                  type="button"
                                  disabled={busyAction !== null}
                                  onClick={() =>
                                    handleQuickAddRule(
                                      "ALLOW",
                                      String(sock.port),
                                      sock.protocol,
                                      "Anywhere",
                                      `${sock.processName} public`,
                                      `sock-open-${rowKey}`,
                                    )
                                  }
                                  title={`ALLOW ${sock.port}/${sock.protocol} from Anywhere`}
                                  className="px-2 py-1 rounded-lg bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/35 text-[10px] font-mono font-bold text-emerald-300 cursor-pointer disabled:opacity-45"
                                >
                                  Open Public
                                </button>

                                <button
                                  type="button"
                                  disabled={
                                    busyAction !== null || !effectiveClientIp
                                  }
                                  onClick={() =>
                                    handleQuickAddRule(
                                      "ALLOW",
                                      String(sock.port),
                                      sock.protocol,
                                      effectiveClientIp,
                                      `${sock.processName} My IP`,
                                      `sock-myip-${rowKey}`,
                                    )
                                  }
                                  title={
                                    effectiveClientIp
                                      ? `ALLOW ${sock.port}/${sock.protocol} from ${effectiveClientIp}`
                                      : "Enter your Client IP in the top banner first"
                                  }
                                  className="px-2 py-1 rounded-lg bg-cyan-500/15 hover:bg-cyan-500/25 border border-cyan-500/35 text-[10px] font-mono font-bold text-cyan-300 cursor-pointer disabled:opacity-45"
                                >
                                  My IP Only
                                </button>

                                <button
                                  type="button"
                                  disabled={busyAction !== null}
                                  onClick={() =>
                                    handleQuickAddRule(
                                      "LIMIT",
                                      String(sock.port),
                                      sock.protocol,
                                      "Anywhere",
                                      `${sock.processName} rate-limit`,
                                      `sock-limit-${rowKey}`,
                                    )
                                  }
                                  title={`LIMIT ${sock.port}/${sock.protocol} (anti-brute-force)`}
                                  className="px-2 py-1 rounded-lg bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/35 text-[10px] font-mono font-bold text-amber-300 cursor-pointer disabled:opacity-45"
                                >
                                  Rate Limit
                                </button>

                                <button
                                  type="button"
                                  disabled={busyAction !== null}
                                  onClick={() =>
                                    handleQuickAddRule(
                                      "DENY",
                                      String(sock.port),
                                      sock.protocol,
                                      "Anywhere",
                                      `Block ${sock.processName}`,
                                      `sock-block-${rowKey}`,
                                    )
                                  }
                                  title={`DENY ${sock.port}/${sock.protocol} from Anywhere`}
                                  className="px-2 py-1 rounded-lg bg-rose-500/15 hover:bg-rose-500/25 border border-rose-500/35 text-[10px] font-mono font-bold text-rose-300 cursor-pointer disabled:opacity-45"
                                >
                                  Block
                                </button>
                              </div>
                            ) : (
                              <span className="text-[11px] font-mono text-muted-foreground">
                                {isVi
                                  ? "Nội bộ (127.0.0.1)"
                                  : "Isolated on loopback"}
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

          {/* ZONE 3: Right Split Pane — 1-Click Service Presets, Rule Composer & Active Rules Matrix (5 cols on XL) */}
          <div className="xl:col-span-5 space-y-4">
            {/* 1-Click Service Presets Strip */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-4 space-y-3 shadow-xs">
              <div className="flex items-center gap-2">
                <Zap className="w-4 h-4 text-amber-400" />
                <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                  {isVi
                    ? "Mẫu Dịch Vụ 1-Click (Quick Service Presets)"
                    : "1-Click Service Firewall Presets"}
                </h3>
              </div>

              <div className="flex flex-wrap gap-2">
                <button
                  type="button"
                  disabled={busyAction !== null}
                  onClick={() =>
                    handleQuickAddRule(
                      "ALLOW",
                      "80,443",
                      "tcp",
                      "Anywhere",
                      "Web HTTP/HTTPS",
                      "preset-web",
                    )
                  }
                  className="px-2.5 py-1.5 rounded-xl bg-emerald-500/12 hover:bg-emerald-500/25 border border-emerald-500/35 text-emerald-300 text-xs font-semibold cursor-pointer transition-colors disabled:opacity-45"
                >
                  + Web HTTP/HTTPS (80, 443)
                </button>

                <button
                  type="button"
                  disabled={busyAction !== null}
                  onClick={() =>
                    handleQuickAddRule(
                      "LIMIT",
                      String(effectiveSshPort),
                      "tcp",
                      "Anywhere",
                      "SSH Brute-Force Limit",
                      "preset-ssh-limit",
                    )
                  }
                  className="px-2.5 py-1.5 rounded-xl bg-amber-500/12 hover:bg-amber-500/25 border border-amber-500/35 text-amber-300 text-xs font-semibold cursor-pointer transition-colors disabled:opacity-45"
                >
                  + SSH Brute-Force Limit (:{effectiveSshPort})
                </button>

                <button
                  type="button"
                  disabled={busyAction !== null || !effectiveClientIp}
                  onClick={() =>
                    handleQuickAddRule(
                      "ALLOW",
                      "5432",
                      "tcp",
                      effectiveClientIp,
                      "Postgres from My IP",
                      "preset-pg",
                    )
                  }
                  className="px-2.5 py-1.5 rounded-xl bg-cyan-500/12 hover:bg-cyan-500/25 border border-cyan-500/35 text-cyan-300 text-xs font-semibold cursor-pointer transition-colors disabled:opacity-45"
                >
                  + Postgres (5432 from My IP)
                </button>

                <button
                  type="button"
                  disabled={busyAction !== null || !effectiveClientIp}
                  onClick={() =>
                    handleQuickAddRule(
                      "ALLOW",
                      "3306",
                      "tcp",
                      effectiveClientIp,
                      "MySQL from My IP",
                      "preset-mysql",
                    )
                  }
                  className="px-2.5 py-1.5 rounded-xl bg-cyan-500/12 hover:bg-cyan-500/25 border border-cyan-500/35 text-cyan-300 text-xs font-semibold cursor-pointer transition-colors disabled:opacity-45"
                >
                  + MySQL (3306 from My IP)
                </button>

                <button
                  type="button"
                  disabled={busyAction !== null}
                  onClick={() =>
                    handleQuickAddRule(
                      "DENY",
                      "6379",
                      "tcp",
                      "Anywhere",
                      "Block Public Redis",
                      "preset-redis",
                    )
                  }
                  className="px-2.5 py-1.5 rounded-xl bg-rose-500/12 hover:bg-rose-500/25 border border-rose-500/35 text-rose-300 text-xs font-semibold cursor-pointer transition-colors disabled:opacity-45"
                >
                  + Redis Block Public (6379)
                </button>
              </div>
            </section>

            {/* Interactive Visual Rule Composer Card */}
            <form
              onSubmit={handleSubmitComposer}
              className="rounded-2xl border border-border/80 bg-card/75 p-4 space-y-3.5 shadow-xs"
            >
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Network className="w-4 h-4 text-primary" />
                  <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                    {isVi
                      ? "Trình Soạn Quy Tắc Tường Lửa (Visual Rule Composer)"
                      : "Visual Firewall Rule Composer"}
                  </h3>
                </div>
              </div>

              {/* Action + Protocol Row */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div>
                  <label className="ui-label">
                    {isVi ? "Hành động (Action)" : "Action"}
                  </label>
                  <div className="grid grid-cols-3 gap-1 p-1 rounded-xl bg-secondary/80 border border-border">
                    {(
                      [
                        {
                          id: "ALLOW",
                          activeCls: "bg-emerald-500 text-slate-950",
                        },
                        {
                          id: "LIMIT",
                          activeCls: "bg-amber-500 text-slate-950",
                        },
                        {
                          id: "DENY",
                          activeCls: "bg-rose-500 text-white",
                        },
                      ] as const
                    ).map((act) => (
                      <button
                        key={act.id}
                        type="button"
                        onClick={() => setComposerAction(act.id)}
                        className={`py-1.5 px-2 rounded-lg text-xs font-mono font-bold transition-all cursor-pointer ${
                          composerAction === act.id
                            ? act.activeCls
                            : "text-muted-foreground hover:text-foreground"
                        }`}
                      >
                        {act.id}
                      </button>
                    ))}
                  </div>
                </div>

                <div>
                  <label className="ui-label">
                    {isVi ? "Giao thức (Protocol)" : "Protocol"}
                  </label>
                  <div className="grid grid-cols-3 gap-1 p-1 rounded-xl bg-secondary/80 border border-border">
                    {(["tcp", "udp", "any"] as const).map((pr) => (
                      <button
                        key={pr}
                        type="button"
                        onClick={() => setComposerProto(pr)}
                        className={`py-1.5 px-2 rounded-lg text-xs font-mono font-bold uppercase transition-all cursor-pointer ${
                          composerProto === pr
                            ? "bg-primary text-primary-foreground"
                            : "text-muted-foreground hover:text-foreground"
                        }`}
                      >
                        {pr}
                      </button>
                    ))}
                  </div>
                </div>
              </div>

              {/* Port / Range + Comment Row */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div>
                  <label className="ui-label">
                    {isVi
                      ? "Cổng / Dải cổng (443, 8000:8010, any)"
                      : "Port / Range (443, 8000:8010, any)"}
                  </label>
                  <input
                    type="text"
                    value={composerPort}
                    onChange={(e) => setComposerPort(e.target.value)}
                    placeholder="443, 80,443, 8000:8010, or any"
                    className="ui-input font-mono text-xs"
                  />
                </div>

                <div>
                  <label className="ui-label">
                    {isVi ? "Ghi chú (Comment)" : "Rule Comment / Note"}
                  </label>
                  <input
                    type="text"
                    value={composerComment}
                    onChange={(e) => setComposerComment(e.target.value)}
                    placeholder={
                      isVi ? "VD: API Gateway..." : "e.g. Internal API..."
                    }
                    className="ui-input text-xs"
                  />
                </div>
              </div>

              {/* Source Scope Selector */}
              <div className="space-y-2">
                <label className="ui-label">
                  {isVi ? "Phạm vi IP nguồn (Source Scope)" : "Source IP Scope"}
                </label>
                <div className="grid grid-cols-3 gap-1.5 p-1 rounded-xl bg-secondary/80 border border-border">
                  {(
                    [
                      {
                        id: "anywhere",
                        label: "Anywhere (0.0.0.0/0)",
                      },
                      {
                        id: "my_ip",
                        label: effectiveClientIp
                          ? `My IP (${effectiveClientIp})`
                          : "My Current IP",
                      },
                      {
                        id: "custom",
                        label: "Custom IP / CIDR",
                      },
                    ] as const
                  ).map((sc) => (
                    <button
                      key={sc.id}
                      type="button"
                      onClick={() => setComposerSourceScope(sc.id)}
                      className={`py-1.5 px-2 rounded-lg text-[11px] font-semibold truncate transition-all cursor-pointer ${
                        composerSourceScope === sc.id
                          ? "bg-primary text-primary-foreground"
                          : "text-muted-foreground hover:text-foreground"
                      }`}
                    >
                      {sc.label}
                    </button>
                  ))}
                </div>

                {composerSourceScope === "custom" && (
                  <input
                    type="text"
                    value={composerCustomSource}
                    onChange={(e) => setComposerCustomSource(e.target.value)}
                    placeholder="e.g. 203.0.113.5 or 10.0.0.0/8"
                    className="ui-input font-mono text-xs"
                  />
                )}
              </div>

              <button
                type="submit"
                disabled={busyAction !== null}
                className="w-full py-2.5 px-4 rounded-xl bg-primary text-primary-foreground text-xs font-bold hover:opacity-95 disabled:opacity-50 inline-flex items-center justify-center gap-1.5 cursor-pointer transition-opacity"
              >
                <Plus className="w-4 h-4" />
                <span>
                  {isVi ? "Thêm Quy Tắc Tường Lửa" : "Add Firewall Rule"}
                </span>
              </button>
            </form>

            {/* Active Firewall Rules Matrix */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-4 space-y-3 shadow-xs">
              <div className="flex items-center justify-between">
                <h3
                  id={rulesCaptionId}
                  className="text-xs font-bold uppercase tracking-wider text-foreground"
                >
                  {isVi
                    ? `Danh Sách Quy Tắc Đang Áp Dụng (${snapshot.rules.length})`
                    : `Active Firewall Rules (${snapshot.rules.length})`}
                </h3>
                <span className="text-[11px] font-mono text-muted-foreground">
                  {snapshot.engine.toUpperCase()}
                </span>
              </div>

              <div
                role="region"
                tabIndex={0}
                aria-labelledby={rulesCaptionId}
                className="vps-table-wrapper rounded-xl border border-border/80 bg-background/50 max-h-[340px]"
              >
                <table className="w-full text-left text-xs">
                  <caption className="sr-only">
                    Active firewall policy rules
                  </caption>
                  <thead className="bg-secondary/95 text-[11px] uppercase tracking-wider text-muted-foreground">
                    <tr>
                      <th className="py-2 px-2.5 font-semibold border-b border-border">
                        #
                      </th>
                      <th className="py-2 px-2.5 font-semibold border-b border-border">
                        To Port
                      </th>
                      <th className="py-2 px-2.5 font-semibold border-b border-border">
                        Action
                      </th>
                      <th className="py-2 px-2.5 font-semibold border-b border-border">
                        From Source
                      </th>
                      <th className="py-2 px-2.5 font-semibold border-b border-border text-right">
                        Del
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border/60">
                    {snapshot.rules.length === 0 ? (
                      <tr>
                        <td
                          colSpan={5}
                          className="py-6 px-3 text-center text-xs text-muted-foreground"
                        >
                          {isVi
                            ? "Chưa có quy tắc tường lửa nào được cấu hình."
                            : "No firewall rules configured yet."}
                        </td>
                      </tr>
                    ) : (
                      snapshot.rules.map((rule) => {
                        const actBadge =
                          rule.action === "ALLOW"
                            ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/35"
                            : rule.action === "LIMIT"
                              ? "bg-amber-500/15 text-amber-300 border-amber-500/35"
                              : "bg-rose-500/15 text-rose-300 border-rose-500/35";
                        return (
                          <tr
                            key={`${rule.index}-${rule.rawSpec}`}
                            className="hover:bg-accent/30 transition-colors"
                          >
                            <th
                              scope="row"
                              className="py-2 px-2.5 font-mono text-[11px] text-muted-foreground"
                            >
                              [{rule.index}]
                            </th>
                            <td className="py-2 px-2.5 font-mono font-semibold text-foreground whitespace-nowrap">
                              <div className="inline-flex items-center gap-1.5">
                                <span>{rule.toPort}</span>
                                <span
                                  className={`px-1.5 py-0.2 rounded text-[9px] font-bold uppercase ${
                                    rule.v6
                                      ? "bg-violet-500/15 text-violet-300"
                                      : "bg-secondary text-muted-foreground"
                                  }`}
                                >
                                  {rule.v6 ? "IPv6" : "IPv4"}
                                </span>
                              </div>
                              {rule.comment && (
                                <div className="text-[10px] font-sans font-normal text-muted-foreground truncate max-w-[160px]">
                                  # {rule.comment}
                                </div>
                              )}
                            </td>
                            <td className="py-2 px-2.5 whitespace-nowrap">
                              <span
                                className={`px-2 py-0.5 rounded-md border font-mono text-[10px] font-bold ${actBadge}`}
                              >
                                {rule.action}
                              </span>
                            </td>
                            <td className="py-2 px-2.5 font-mono text-[11px] whitespace-nowrap">
                              <span
                                className={
                                  rule.fromSource.toLowerCase() === "anywhere"
                                    ? "text-muted-foreground"
                                    : "text-cyan-300 font-semibold"
                                }
                              >
                                {rule.fromSource}
                              </span>
                            </td>
                            <td className="py-2 px-2.5 text-right">
                              <button
                                type="button"
                                disabled={busyAction !== null}
                                onClick={() =>
                                  handleDeleteRule(rule.index, rule.rawSpec)
                                }
                                title={`Delete rule #${rule.index}`}
                                className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-300 cursor-pointer disabled:opacity-45 transition-colors"
                              >
                                <Trash2 className="w-3.5 h-3.5" />
                              </button>
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
        </div>
      )}
    </div>
  );
};
