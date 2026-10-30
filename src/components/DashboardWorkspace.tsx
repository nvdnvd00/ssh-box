import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  ArrowDown,
  ArrowUp,
  CheckCircle2,
  ChevronLeft,
  Container,
  Cpu,
  Grip,
  HardDrive,
  Laptop,
  LayoutGrid,
  List,
  MemoryStick,
  Network,
  Plus,
  Search,
  Server,
  Shield,
  Skull,
  Terminal as TerminalIcon,
  X,
  XCircle,
} from "lucide-react";
import {
  DashboardDisplayStyle,
  DockerFullSnapshot,
  HostEntry,
  ResourcesStoreData,
  ServerMetrics,
} from "../types";
import { AppSettingsConfig } from "../themes";
import { SelectField } from "./SelectField";

const DASHBOARD_DISPLAY_STYLE_KEY = "ssh-box:dashboard-display-style";

interface DashboardWorkspaceProps {
  hosts: HostEntry[];
  resources: ResourcesStoreData;
  metricsMap: Record<string, ServerMetrics>;
  selectedHostId: string | null;
  appSettings: AppSettingsConfig;
  localMonitorEnabled?: boolean;
  onToggleHostMonitor?: (hostId: string, enabled: boolean) => void;
  onSelectHost: (hostId: string | null) => void;
  onConnectHostTerminal: (host: HostEntry) => void;
  /** Omitted on mobile, where local terminals are disabled */
  onOpenLocalTerminal?: () => void;
  onOpenAddMachine: () => void;
  onOpenDockerManager: (hostId: string) => void;
  onOpenSftpManager?: (hostId: string) => void;
  onOpenVpsConfig?: (hostId: string) => void;
}

interface LoadHistoryPoint {
  one: number;
  five: number;
  fifteen: number;
}

function formatBytesPerSec(kbps: number): string {
  const bytes = Math.max(0, kbps * 1024);
  if (bytes < 1024) return `${Math.round(bytes)} B/s`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB/s`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} MB/s`;
}

function formatRawBytesPerSec(bytes: number): string {
  const b = Math.max(0, bytes);
  if (b < 1024) return `${Math.round(b)} B/s`;
  if (b < 1024 * 1024) return `${(b / 1024).toFixed(1)} KB/s`;
  return `${(b / (1024 * 1024)).toFixed(2)} MB/s`;
}

function formatTotalBytes(bytes: number): string {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "K", "M", "G", "T"];
  let val = bytes;
  let idx = 0;
  while (val >= 1024 && idx < units.length - 1) {
    val /= 1024;
    idx++;
  }
  return `${val.toFixed(idx >= 2 ? 2 : 0)} ${units[idx]}`;
}

function formatMbToGb(mb: number): string {
  if (!mb || mb <= 0) return "0 B";
  if (mb < 1024) return `${Math.round(mb)} M`;
  return `${(mb / 1024).toFixed(2)} G`;
}

function formatRssKb(kb: number): string {
  if (!kb || kb <= 0) return "0 K";
  if (kb < 1024) return `${kb} K`;
  if (kb < 1024 * 1024) return `${(kb / 1024).toFixed(1)} M`;
  return `${(kb / (1024 * 1024)).toFixed(2)} G`;
}

function formatUptime(seconds?: number): string {
  if (!seconds || seconds <= 0) return "—";
  const days = Math.floor(seconds / 86400);
  const hours = Math.floor((seconds % 86400) / 3600);
  const mins = Math.floor((seconds % 3600) / 60);
  if (days > 0) return `${days}d ${hours}h ${mins}m`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

function parsePercentString(raw: string): number {
  const cleaned = raw.replace(/[^0-9.]/g, "");
  const n = parseFloat(cleaned);
  return Number.isFinite(n) ? Math.min(100, Math.max(0, n)) : 0;
}

const CircularRingGauge: React.FC<{
  percent: number;
  label: string;
  size?: number;
  strokeWidth?: number;
  color?: string;
}> = ({ percent, label, size = 62, strokeWidth = 5.5, color }) => {
  const clamped = Math.min(100, Math.max(0, Number.isFinite(percent) ? percent : 0));
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;
  const offset = circumference - (clamped / 100) * circumference;

  const strokeColor =
    color ??
    (clamped > 85 ? "#f43f5e" : clamped > 65 ? "#f59e0b" : "#22c55e");

  return (
    <div className="flex flex-col items-center gap-1">
      <div
        className="relative flex items-center justify-center"
        style={{ width: size, height: size }}
      >
        <svg width={size} height={size} className="-rotate-90">
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="transparent"
            stroke="currentColor"
            strokeWidth={strokeWidth}
            className="text-secondary/90"
          />
          <circle
            cx={size / 2}
            cy={size / 2}
            r={radius}
            fill="transparent"
            stroke={strokeColor}
            strokeWidth={strokeWidth}
            strokeDasharray={circumference}
            strokeDashoffset={offset}
            strokeLinecap="round"
            className="transition-all duration-500 ease-out"
          />
        </svg>
        <span className="absolute text-[11px] font-bold font-mono text-foreground">
          {Math.round(clamped)}%
        </span>
      </div>
      <span className="text-[11px] font-medium text-muted-foreground">
        {label}
      </span>
    </div>
  );
};

const SegmentedCoreBar: React.FC<{ percent: number; segments?: number }> = ({
  percent,
  segments = 28,
}) => {
  const clamped = Math.min(100, Math.max(0, percent));
  const activeCount = Math.round((clamped / 100) * segments);
  return (
    <div className="flex items-center gap-[3px] w-full h-3.5">
      {Array.from({ length: segments }).map((_, idx) => {
        const isLit = idx < activeCount;
        const barColor = !isLit
          ? "bg-secondary/80"
          : idx / segments > 0.85
            ? "bg-rose-500"
            : idx / segments > 0.65
              ? "bg-amber-400"
              : "bg-emerald-500";
        return (
          <span
            key={idx}
            className={`flex-1 h-full rounded-[2px] transition-colors duration-300 ${barColor}`}
          />
        );
      })}
    </div>
  );
};

const MemoryDonutChart: React.FC<{
  usedMb: number;
  cachedMb: number;
  freeMb: number;
  totalMb: number;
}> = ({ usedMb, cachedMb, freeMb, totalMb }) => {
  const size = 132;
  const strokeWidth = 15;
  const radius = (size - strokeWidth) / 2;
  const circumference = 2 * Math.PI * radius;

  const safeTotal = Math.max(1, totalMb || usedMb + cachedMb + freeMb);
  const usedRatio = Math.min(1, Math.max(0, usedMb / safeTotal));
  const cachedRatio = Math.min(
    1 - usedRatio,
    Math.max(0, cachedMb / safeTotal),
  );

  const usedLen = usedRatio * circumference;
  const cachedLen = cachedRatio * circumference;

  return (
    <div
      className="relative flex items-center justify-center shrink-0"
      style={{ width: size, height: size }}
    >
      <svg width={size} height={size} className="-rotate-90">
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="transparent"
          stroke="currentColor"
          strokeWidth={strokeWidth}
          className="text-secondary"
        />
        {/* Cached arc */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="transparent"
          stroke="#f59e0b"
          strokeWidth={strokeWidth}
          strokeDasharray={`${cachedLen} ${circumference}`}
          strokeDashoffset={-usedLen}
          className="transition-all duration-500"
        />
        {/* Used arc */}
        <circle
          cx={size / 2}
          cy={size / 2}
          r={radius}
          fill="transparent"
          stroke="#22c55e"
          strokeWidth={strokeWidth}
          strokeDasharray={`${usedLen} ${circumference}`}
          strokeDashoffset={0}
          strokeLinecap="round"
          className="transition-all duration-500"
        />
      </svg>
      <div className="absolute flex flex-col items-center justify-center text-center">
        <span className="text-[11px] text-muted-foreground">Total</span>
        <span className="text-sm font-bold font-mono text-foreground">
          {formatMbToGb(totalMb)}
        </span>
      </div>
    </div>
  );
};

const MultiLineLoadChart: React.FC<{ points: LoadHistoryPoint[] }> = ({
  points,
}) => {
  const width = 460;
  const height = 115;
  const padX = 28;
  const padY = 14;

  const safePoints =
    points.length >= 2
      ? points
      : [
          { one: 0.1, five: 0.15, fifteen: 0.2 },
          { one: 0.1, five: 0.15, fifteen: 0.2 },
        ];

  const maxVal = Math.max(
    1.0,
    ...safePoints.flatMap((p) => [p.one, p.five, p.fifteen]),
  );

  const buildPolyline = (getter: (p: LoadHistoryPoint) => number) => {
    return safePoints
      .map((pt, i) => {
        const x =
          padX +
          (i / Math.max(1, safePoints.length - 1)) * (width - padX * 1.5);
        const y =
          height -
          padY -
          (Math.min(maxVal, Math.max(0, getter(pt))) / maxVal) *
            (height - padY * 2);
        return `${x.toFixed(1)},${y.toFixed(1)}`;
      })
      .join(" ");
  };

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className="w-full h-28 overflow-visible"
    >
      {[0, 0.5, 1].map((ratio, idx) => {
        const y = height - padY - ratio * (height - padY * 2);
        const label = (ratio * maxVal).toFixed(1);
        return (
          <g key={idx}>
            <line
              x1={padX}
              y1={y}
              x2={width - 8}
              y2={y}
              stroke="currentColor"
              strokeDasharray="3 3"
              className="text-border/70"
            />
            <text
              x={0}
              y={y + 3}
              className="fill-muted-foreground text-[9px] font-mono"
            >
              {label}
            </text>
          </g>
        );
      })}
      <polyline
        fill="none"
        stroke="#f59e0b"
        strokeWidth="1.8"
        points={buildPolyline((p) => p.fifteen)}
      />
      <polyline
        fill="none"
        stroke="#38bdf8"
        strokeWidth="1.8"
        points={buildPolyline((p) => p.five)}
      />
      <polyline
        fill="none"
        stroke="#22c55e"
        strokeWidth="2.2"
        points={buildPolyline((p) => p.one)}
      />
    </svg>
  );
};

export const DashboardWorkspace: React.FC<DashboardWorkspaceProps> = ({
  hosts,
  resources,
  metricsMap,
  selectedHostId,
  localMonitorEnabled = true,
  onToggleHostMonitor,
  onSelectHost,
  onConnectHostTerminal,
  onOpenLocalTerminal,
  onOpenAddMachine,
  onOpenDockerManager,
  onOpenSftpManager,
  onOpenVpsConfig,
}) => {
  // Icon toggle sized like the neighbouring action buttons; `small` matches the compact/list rows.
  const renderMonitorSwitch = (hostId: string, enabled: boolean, small = false) => (
    <button
      type="button"
      role="switch"
      aria-checked={enabled}
      aria-label={enabled ? "Disable auto-update monitor" : "Enable auto-update monitor"}
      title={
        enabled
          ? "Auto-update monitor: On (Click to turn off)"
          : "Auto-update monitor: Off (Click to turn on)"
      }
      onClick={(e) => {
        e.stopPropagation();
        onToggleHostMonitor?.(hostId, !enabled);
      }}
      className={`shrink-0 transition-colors cursor-pointer ${
        small ? "p-1.5 rounded-lg" : "p-2 rounded-xl"
      } ${
        enabled
          ? "bg-primary/15 text-primary ring-1 ring-inset ring-primary/40 hover:bg-primary/25"
          : "bg-secondary/90 text-muted-foreground/60 hover:bg-accent hover:text-foreground"
      }`}
    >
      <Activity className={small ? "w-3.5 h-3.5" : "w-4 h-4"} />
    </button>
  );
  const [selectedTagId, setSelectedTagId] = useState<string>("ALL");
  const [displayStyle, setDisplayStyle] = useState<DashboardDisplayStyle>(() => {
    try {
      const saved = localStorage.getItem(DASHBOARD_DISPLAY_STYLE_KEY);
      if (saved === "grid" || saved === "compact" || saved === "list") {
        return saved;
      }
    } catch {
      // ignore
    }
    return "grid";
  });
  const [searchQuery, setSearchQuery] = useState("");

  const handleChangeDisplayStyle = useCallback((next: DashboardDisplayStyle) => {
    setDisplayStyle(next);
    try {
      localStorage.setItem(DASHBOARD_DISPLAY_STYLE_KEY, next);
    } catch {
      // ignore
    }
  }, []);
  const [procSearch, setProcSearch] = useState("");
  const [loadHistoryMap, setLoadHistoryMap] = useState<
    Record<string, LoadHistoryPoint[]>
  >({});
  const [detailDockerSnap, setDetailDockerSnap] =
    useState<DockerFullSnapshot | null>(null);
  const [killingPid, setKillingPid] = useState<number | null>(null);

  const selectedHostObj =
    !selectedHostId || selectedHostId === "local"
      ? null
      : hosts.find((h) => h.id === selectedHostId) ?? null;
  const isSelectedAutoUpdate = !selectedHostId
    ? false
    : selectedHostId === "local"
      ? localMonitorEnabled
      : selectedHostObj?.showStatus !== false;
  const selectedMetrics =
    selectedHostId && isSelectedAutoUpdate ? metricsMap[selectedHostId] : null;

  // Record load history when metrics update
  useEffect(() => {
    if (!selectedHostId || !selectedMetrics) return;
    setLoadHistoryMap((prev) => {
      const existing = prev[selectedHostId] ?? [];
      const nextPoint: LoadHistoryPoint = {
        one: selectedMetrics.loadOne,
        five: selectedMetrics.loadFive,
        fifteen: selectedMetrics.loadFifteen,
      };
      return {
        ...prev,
        [selectedHostId]: [...existing.slice(-29), nextPoint],
      };
    });
  }, [
    selectedHostId,
    selectedMetrics?.loadOne,
    selectedMetrics?.loadFive,
    selectedMetrics?.loadFifteen,
  ]);

  // Fetch Docker containers for detail view
  const refreshDetailDocker = useCallback((hostId: string) => {
    invoke<DockerFullSnapshot>("fetch_docker_snapshot", { hostId })
      .then((snap) => setDetailDockerSnap(snap))
      .catch(() => setDetailDockerSnap(null));
  }, []);

  useEffect(() => {
    if (!selectedHostId || !isSelectedAutoUpdate) {
      setDetailDockerSnap(null);
      return;
    }
    refreshDetailDocker(selectedHostId);
    const timer = setInterval(() => {
      refreshDetailDocker(selectedHostId);
    }, 6000);
    return () => clearInterval(timer);
  }, [selectedHostId, isSelectedAutoUpdate, refreshDetailDocker]);

  const handleKillProcess = async (pid: number) => {
    if (!selectedHostId) return;
    setKillingPid(pid);
    try {
      await invoke("kill_process", {
        hostId: selectedHostId,
        pid,
        force: false,
      });
    } catch {
      // ignore
    } finally {
      setKillingPid(null);
    }
  };

  const filteredHosts = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const matchedTag = resources.tags.find(
      (t) => t.name === selectedTagId || t.id === selectedTagId,
    );
    return hosts.filter((h) => {
      if (selectedTagId === "NO_TAGS" && (h.tags?.length ?? 0) > 0) {
        return false;
      }
      if (selectedTagId !== "ALL" && selectedTagId !== "NO_TAGS") {
        const hostTags = h.tags ?? [];
        const hasTag =
          hostTags.includes(selectedTagId) ||
          (matchedTag
            ? hostTags.includes(matchedTag.name) ||
              hostTags.includes(matchedTag.id)
            : false);
        if (!hasTag) return false;
      }
      if (!q) return true;
      return (
        h.alias.toLowerCase().includes(q) ||
        h.hostname.toLowerCase().includes(q) ||
        h.user.toLowerCase().includes(q)
      );
    });
  }, [hosts, resources.tags, selectedTagId, searchQuery]);

  // Build unified machine cards list (Localhost + configured SSH Hosts)
  const totalMachinesCount = filteredHosts.length + 1; // +1 for Localhost
  const onlineMachinesCount =
    (localMonitorEnabled && Boolean(metricsMap["local"]) ? 1 : 0) +
    filteredHosts.filter(
      (h) => h.showStatus !== false && Boolean(metricsMap[h.id]),
    ).length;
  const offlineMachinesCount = Math.max(
    0,
    totalMachinesCount - onlineMachinesCount,
  );

  // ============================================================================
  // DETAIL MONITOR VIEW (Matches Reference Screenshots 1 & 2: HRM-PROD-ELS)
  // ============================================================================
  if (selectedHostId) {
    const hostObj = selectedHostObj;
    const displayName =
      selectedHostId === "local"
        ? "Localhost (macOS)"
        : hostObj?.alias || hostObj?.hostname || selectedHostId;

    const m = selectedMetrics;
    const memPercent =
      m && m.memTotalMb > 0 ? (m.memUsedMb / m.memTotalMb) * 100 : 0;
    const primaryNic = m?.interfaces?.[0] ?? null;
    const primaryVol = m?.volumes?.[0] ?? null;
    const runningContainers =
      detailDockerSnap?.containers.filter((c) => c.state === "running") ?? [];

    const filteredProcs = (m?.topProcesses ?? []).filter((p) => {
      const q = procSearch.trim().toLowerCase();
      if (!q) return true;
      return (
        p.command.toLowerCase().includes(q) ||
        p.args.toLowerCase().includes(q) ||
        p.user.toLowerCase().includes(q) ||
        String(p.pid).includes(q)
      );
    });

    return (
      <div className="flex flex-col h-full w-full bg-background text-foreground overflow-hidden select-none ui-fade-slide">
        {/* Detail Monitor Top Header Bar */}
        <div className="h-13 shrink-0 px-6 border-b border-border/80 bg-header/60 backdrop-blur-md flex items-center justify-between">
          <button
            type="button"
            onClick={() => onSelectHost(null)}
            className="inline-flex items-center gap-2 px-3 py-1.5 rounded-xl bg-secondary/80 hover:bg-accent text-foreground text-[13px] font-semibold border border-border/70 transition-all cursor-pointer active:scale-97"
          >
            <ChevronLeft className="w-4 h-4" />
            <span>{displayName}</span>
          </button>

          <div className="flex items-center gap-2">
            {renderMonitorSwitch(selectedHostId, isSelectedAutoUpdate)}

            {(selectedHostId !== "local" || onOpenLocalTerminal) && (
              <button
                type="button"
                onClick={() => {
                  if (selectedHostId === "local") {
                    onOpenLocalTerminal?.();
                  } else if (hostObj) {
                    onConnectHostTerminal(hostObj);
                  }
                }}
                className="inline-flex items-center gap-2 px-3.5 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-semibold shadow-xs hover:opacity-95 transition-all cursor-pointer active:scale-97"
              >
                <TerminalIcon className="w-3.5 h-3.5" />
                <span>Open SSH Terminal</span>
              </button>
            )}

            {onOpenSftpManager && (
              <button
                type="button"
                onClick={() => onOpenSftpManager(selectedHostId)}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-xl bg-secondary/80 hover:bg-accent text-foreground text-xs font-medium border border-border/70 transition-all cursor-pointer"
              >
                <HardDrive className="w-3.5 h-3.5 text-amber-400" />
                <span>SFTP</span>
              </button>
            )}

            <button
              type="button"
              onClick={() => onOpenDockerManager(selectedHostId)}
              className="inline-flex items-center gap-1.5 px-3 py-2 rounded-xl bg-secondary/80 hover:bg-accent text-foreground text-xs font-medium border border-border/70 transition-all cursor-pointer"
            >
              <Container className="w-3.5 h-3.5 text-sky-400" />
              <span>Docker</span>
            </button>

            {selectedHostId !== "local" && onOpenVpsConfig && (
              <button
                type="button"
                onClick={() => onOpenVpsConfig(selectedHostId)}
                className="inline-flex items-center gap-1.5 px-3 py-2 rounded-xl bg-secondary/80 hover:bg-accent text-foreground text-xs font-medium border border-border/70 transition-all cursor-pointer"
              >
                <Shield className="w-3.5 h-3.5 text-emerald-400" />
                <span>VPS Hardening</span>
              </button>
            )}
          </div>
        </div>

        {/* Scrollable Telemetry Canvas */}
        <div className="flex-1 overflow-y-auto p-6 space-y-5">
          {/* Top Machine Banner with 3 Circular Ring Gauges */}
          <div className="rounded-2xl border border-border/80 bg-card/75 p-5 flex flex-wrap items-center justify-between gap-4 shadow-xs">
            <div className="flex items-center gap-4">
              <div className="w-12 h-12 rounded-2xl bg-orange-500/15 border border-orange-500/30 flex items-center justify-center text-orange-400 shadow-inner">
                {selectedHostId === "local" ? (
                  <Laptop className="w-6 h-6" />
                ) : (
                  <Server className="w-6 h-6" />
                )}
              </div>
              <div>
                <div className="flex items-center gap-2.5">
                  <h1 className="text-lg font-bold tracking-tight text-foreground">
                    {displayName}
                  </h1>
                  <span
                    className={`inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full text-[11px] font-medium border ${
                      !isSelectedAutoUpdate
                        ? "bg-secondary/80 text-muted-foreground border-border"
                        : "bg-emerald-500/15 text-emerald-400 border-emerald-500/30"
                    }`}
                  >
                    <span
                      className={`w-1.5 h-1.5 rounded-full ${
                        !isSelectedAutoUpdate
                          ? "bg-muted-foreground/50"
                          : "bg-emerald-400 animate-pulse"
                      }`}
                    />
                    {!isSelectedAutoUpdate
                      ? "Auto-update Off"
                      : m
                        ? "Live Telemetry"
                        : "Connecting Probe..."}
                  </span>
                </div>
                <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                  <span className="px-2 py-0.5 rounded-md bg-secondary/90 border border-border/60 font-mono text-[11px] text-foreground">
                    {m?.osName ||
                      (hostObj
                        ? `${hostObj.user}@${hostObj.hostname}:${hostObj.port}`
                        : "Local System")}
                  </span>
                  {m && (
                    <span>
                      Uptime:{" "}
                      <strong className="text-foreground font-mono">
                        {formatUptime(m.uptimeSeconds)}
                      </strong>
                    </span>
                  )}
                </div>
              </div>
            </div>

            <div className="flex items-center gap-5">
              <CircularRingGauge
                percent={m?.cpuPercent ?? 0}
                label="CPU"
                size={60}
              />
              <CircularRingGauge
                percent={memPercent}
                label="RAM"
                size={60}
              />
              <CircularRingGauge
                percent={m?.diskPercent ?? 0}
                label="Disk"
                size={60}
              />
            </div>
          </div>

          {/* 2-Column Main Layout: Left = CPU/Load/Memory/Network/Storage, Right = Processes */}
          <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-start">
            {/* LEFT COLUMN (7 cols on XL) */}
            <div className="xl:col-span-6 space-y-5">
              {/* 1. CPU Usage Card */}
              <div className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                    <Cpu className="w-4 h-4 text-emerald-400" />
                    <span>CPU Usage</span>
                  </div>
                  <span className="text-xs font-mono text-muted-foreground truncate max-w-[260px]">
                    {m?.cpuModel || "Processor"}
                  </span>
                </div>

                <div className="text-3xl font-bold font-mono text-foreground">
                  {m ? `${Math.round(m.cpuPercent)}%` : "—"}
                </div>

                {/* Per-Core Segmented Tick Bars */}
                <div className="space-y-2">
                  {(m?.cores && m.cores.length > 0
                    ? m.cores.slice(0, 8)
                    : [
                        {
                          coreId: "cpu0",
                          usagePct: m?.cpuPercent ?? 0,
                          userPct: 0,
                          systemPct: 0,
                          iowaitPct: 0,
                        },
                      ]
                  ).map((core) => (
                    <div key={core.coreId} className="space-y-1">
                      <SegmentedCoreBar percent={core.usagePct} />
                    </div>
                  ))}
                </div>

                <div className="grid grid-cols-4 gap-2 pt-3 border-t border-border/60 text-center">
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      Cores
                    </div>
                    <div className="text-sm font-bold font-mono text-foreground mt-0.5">
                      {m?.cpuCores ?? 1}
                    </div>
                  </div>
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      User
                    </div>
                    <div className="text-sm font-bold font-mono text-foreground mt-0.5">
                      {m ? `${m.cpuUserPercent.toFixed(1)}%` : "0%"}
                    </div>
                  </div>
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      System
                    </div>
                    <div className="text-sm font-bold font-mono text-foreground mt-0.5">
                      {m ? `${m.cpuSystemPercent.toFixed(1)}%` : "0%"}
                    </div>
                  </div>
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      IOWait
                    </div>
                    <div className="text-sm font-bold font-mono text-foreground mt-0.5">
                      {m ? `${m.cpuIowaitPercent.toFixed(1)}%` : "0%"}
                    </div>
                  </div>
                </div>
              </div>

              {/* 2. CPU Load Card */}
              <div className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-3 shadow-xs">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                    <Activity className="w-4 h-4 text-sky-400" />
                    <span>CPU Load</span>
                  </div>
                  <div className="flex items-center gap-3 text-[11px] font-mono">
                    <span className="inline-flex items-center gap-1 text-emerald-400">
                      <span className="w-2 h-2 rounded-full bg-emerald-400" />
                      1m
                    </span>
                    <span className="inline-flex items-center gap-1 text-sky-400">
                      <span className="w-2 h-2 rounded-full bg-sky-400" />
                      5m
                    </span>
                    <span className="inline-flex items-center gap-1 text-amber-400">
                      <span className="w-2 h-2 rounded-full bg-amber-400" />
                      15m
                    </span>
                  </div>
                </div>

                <MultiLineLoadChart
                  points={loadHistoryMap[selectedHostId] ?? []}
                />

                <div className="flex items-center justify-between pt-2 border-t border-border/60 text-xs">
                  <span className="text-muted-foreground">Current Load</span>
                  <span className="font-mono font-semibold text-foreground">
                    {m
                      ? `${m.loadOne.toFixed(2)} / ${m.loadFive.toFixed(2)} / ${m.loadFifteen.toFixed(2)}`
                      : "0.00 / 0.00 / 0.00"}
                  </span>
                </div>
              </div>

              {/* 3. Memory Usage Card */}
              <div className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                    <MemoryStick className="w-4 h-4 text-violet-400" />
                    <span>Memory Usage</span>
                  </div>
                </div>

                <div className="flex items-center justify-around gap-6 py-1">
                  <MemoryDonutChart
                    usedMb={m?.memUsedMb ?? 0}
                    cachedMb={m?.memCachedMb ?? 0}
                    freeMb={m?.memFreeMb ?? 0}
                    totalMb={m?.memTotalMb ?? 0}
                  />

                  <div className="flex-1 max-w-[210px] space-y-2.5 text-xs">
                    <div className="flex items-center justify-between">
                      <span className="inline-flex items-center gap-2 text-muted-foreground">
                        <span className="w-2.5 h-2.5 rounded-full bg-emerald-500" />
                        Used
                      </span>
                      <span className="font-mono font-semibold text-foreground">
                        {formatMbToGb(m?.memUsedMb ?? 0)}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="inline-flex items-center gap-2 text-muted-foreground">
                        <span className="w-2.5 h-2.5 rounded-full bg-amber-400" />
                        Cached
                      </span>
                      <span className="font-mono font-semibold text-foreground">
                        {formatMbToGb(m?.memCachedMb ?? 0)}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="inline-flex items-center gap-2 text-muted-foreground">
                        <span className="w-2.5 h-2.5 rounded-full bg-secondary border border-border" />
                        Free
                      </span>
                      <span className="font-mono font-semibold text-foreground">
                        {formatMbToGb(m?.memAvailableMb ?? m?.memFreeMb ?? 0)}
                      </span>
                    </div>
                  </div>
                </div>

                {/* Swap Bar */}
                <div className="pt-3 border-t border-border/60 space-y-1.5">
                  <div className="flex items-center justify-between text-xs">
                    <span className="text-muted-foreground">Swap</span>
                    <span className="font-mono text-muted-foreground">
                      {formatMbToGb(m?.swapUsedMb ?? 0)} /{" "}
                      {formatMbToGb(m?.swapTotalMb ?? 0)}
                    </span>
                  </div>
                  <div className="w-full h-2 rounded-full bg-secondary overflow-hidden">
                    <div
                      className="h-full bg-violet-500 rounded-full transition-all duration-300"
                      style={{
                        width: `${
                          m && m.swapTotalMb > 0
                            ? Math.min(100, (m.swapUsedMb / m.swapTotalMb) * 100)
                            : 0
                        }%`,
                      }}
                    />
                  </div>
                </div>
              </div>

              {/* 4. Network Usage Card */}
              <div className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                    <Network className="w-4 h-4 text-cyan-400" />
                    <span>Network Usage</span>
                    {primaryNic && (
                      <span className="px-2 py-0.5 rounded-md bg-secondary text-foreground font-mono text-[11px]">
                        {primaryNic.name}
                      </span>
                    )}
                  </div>
                  <span className="text-xs font-mono text-muted-foreground">
                    {primaryNic?.ip || hostObj?.hostname || "127.0.0.1"}
                  </span>
                </div>

                <div className="grid grid-cols-2 gap-4 pt-1">
                  <div className="space-y-3 border-r border-border/60 pr-4">
                    <div className="flex items-center gap-2.5">
                      <div className="p-2 rounded-xl bg-emerald-500/15 text-emerald-400">
                        <ArrowUp className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="text-[11px] text-muted-foreground">
                          Upload
                        </div>
                        <div className="text-sm font-bold font-mono text-foreground">
                          {formatBytesPerSec(m?.netTxKbps ?? 0)}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center gap-2.5">
                      <div className="p-2 rounded-xl bg-sky-500/15 text-sky-400">
                        <ArrowDown className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="text-[11px] text-muted-foreground">
                          Download
                        </div>
                        <div className="text-sm font-bold font-mono text-foreground">
                          {formatBytesPerSec(m?.netRxKbps ?? 0)}
                        </div>
                      </div>
                    </div>
                  </div>

                  <div className="flex flex-col justify-center space-y-2.5 pl-2">
                    <div>
                      <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                        <span className="w-2 h-2 rounded-full bg-emerald-400" />
                        Total Upload
                      </div>
                      <div className="text-sm font-bold font-mono text-foreground pl-3.5">
                        {formatTotalBytes(primaryNic?.txTotalBytes ?? 0)}
                      </div>
                    </div>
                    <div>
                      <div className="flex items-center gap-1.5 text-[11px] text-muted-foreground">
                        <span className="w-2 h-2 rounded-full bg-sky-400" />
                        Total Download
                      </div>
                      <div className="text-sm font-bold font-mono text-foreground pl-3.5">
                        {formatTotalBytes(primaryNic?.rxTotalBytes ?? 0)}
                      </div>
                    </div>
                  </div>
                </div>
              </div>

              {/* 5. Storage Card */}
              <div className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                    <HardDrive className="w-4 h-4 text-emerald-400" />
                    <span>Storage</span>
                    <span className="px-2 py-0.5 rounded-md bg-secondary text-foreground font-mono text-[11px]">
                      {primaryVol?.device || "/dev/root"}
                    </span>
                  </div>
                  <span className="text-xs font-mono font-semibold text-foreground">
                    {m
                      ? `${m.diskUsedGb.toFixed(1)}G / ${m.diskTotalGb.toFixed(1)}G`
                      : "—"}
                  </span>
                </div>

                <div className="space-y-1.5">
                  <div className="w-full h-3 rounded-lg bg-secondary overflow-hidden">
                    <div
                      className="h-full bg-emerald-500 rounded-lg transition-all duration-500"
                      style={{
                        width: `${Math.min(100, Math.max(0, m?.diskPercent ?? 0))}%`,
                      }}
                    />
                  </div>
                  <div className="flex items-center justify-between text-[11px] text-muted-foreground font-mono">
                    <span>Mount: {primaryVol?.mount || "/"}</span>
                    <span>{m ? `${m.diskPercent.toFixed(1)}% used` : "0%"}</span>
                  </div>
                </div>

                <div className="grid grid-cols-4 gap-2 pt-3 border-t border-border/60 text-center">
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      Read
                    </div>
                    <div className="text-xs font-bold font-mono text-foreground mt-0.5">
                      {formatRawBytesPerSec(primaryVol?.readBytesPerSec ?? 0)}
                    </div>
                  </div>
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      Write
                    </div>
                    <div className="text-xs font-bold font-mono text-foreground mt-0.5">
                      {formatRawBytesPerSec(primaryVol?.writeBytesPerSec ?? 0)}
                    </div>
                  </div>
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      IOPS
                    </div>
                    <div className="text-xs font-bold font-mono text-foreground mt-0.5">
                      {Math.round(
                        (primaryVol?.iopsRead ?? 0) +
                          (primaryVol?.iopsWrite ?? 0),
                      )}
                    </div>
                  </div>
                  <div>
                    <div className="text-[11px] text-muted-foreground">
                      Available
                    </div>
                    <div className="text-xs font-bold font-mono text-foreground mt-0.5">
                      {primaryVol
                        ? `${primaryVol.availableGb.toFixed(1)}G`
                        : m
                          ? `${Math.max(0, m.diskTotalGb - m.diskUsedGb).toFixed(1)}G`
                          : "—"}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* RIGHT COLUMN: Processes Table Card (Matches Reference Screenshot 1) */}
            <div className="xl:col-span-6 rounded-2xl border border-border/80 bg-card/75 p-5 flex flex-col h-[1060px] shadow-xs">
              <div className="flex items-center justify-between gap-3 pb-3 border-b border-border/60">
                <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                  <Activity className="w-4 h-4 text-primary" />
                  <span>Processes</span>
                  <span className="px-2 py-0.5 rounded-full bg-secondary text-foreground font-mono text-[11px]">
                    {filteredProcs.length}
                  </span>
                </div>
                <div className="relative w-56">
                  <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
                  <input
                    type="text"
                    value={procSearch}
                    onChange={(e) => setProcSearch(e.target.value)}
                    placeholder="Filter processes..."
                    className="w-full pl-8 pr-3 py-1.5 rounded-xl bg-secondary/80 border border-border text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
                  />
                </div>
              </div>

              <div className="flex-1 overflow-auto mt-2">
                <table className="w-full text-left border-collapse text-xs">
                  <thead className="sticky top-0 bg-card/95 backdrop-blur-xs text-[11px] text-muted-foreground border-b border-border/60">
                    <tr>
                      <th className="py-2 px-2 font-medium">Pid</th>
                      <th className="py-2 px-2 font-medium">Process</th>
                      <th className="py-2 px-2 font-medium">Args</th>
                      <th className="py-2 px-2 font-medium text-right">
                        Threads
                      </th>
                      <th className="py-2 px-2 font-medium">User</th>
                      <th className="py-2 px-2 font-medium text-right">
                        CPU%
                      </th>
                      <th className="py-2 px-2 font-medium text-right">Mem</th>
                      <th className="py-2 px-1 font-medium text-right" />
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border/40 font-mono text-[11.5px]">
                    {filteredProcs.map((proc) => (
                      <tr
                        key={proc.pid}
                        className="group hover:bg-accent/50 transition-colors"
                      >
                        <td className="py-2 px-2 text-muted-foreground">
                          {proc.pid}
                        </td>
                        <td className="py-2 px-2 font-semibold text-foreground truncate max-w-[120px]">
                          {proc.command}
                        </td>
                        <td
                          title={proc.args}
                          className="py-2 px-2 text-muted-foreground truncate max-w-[150px]"
                        >
                          {proc.args}
                        </td>
                        <td className="py-2 px-2 text-right text-muted-foreground">
                          {proc.threads}
                        </td>
                        <td className="py-2 px-2 text-muted-foreground truncate max-w-[70px]">
                          {proc.user}
                        </td>
                        <td className="py-2 px-2 text-right font-semibold text-emerald-400">
                          {proc.cpuPct.toFixed(1)}%
                        </td>
                        <td className="py-2 px-2 text-right text-foreground">
                          {formatRssKb(proc.rssKb)}
                        </td>
                        <td className="py-2 px-1 text-right">
                          <button
                            type="button"
                            disabled={killingPid === proc.pid}
                            onClick={() => handleKillProcess(proc.pid)}
                            title={`Terminate PID ${proc.pid}`}
                            className="opacity-0 group-hover:opacity-100 p-1 rounded-md text-muted-foreground hover:text-rose-400 hover:bg-rose-500/15 transition-all cursor-pointer"
                          >
                            <Skull className="w-3.5 h-3.5" />
                          </button>
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </div>
          </div>

          {/* BOTTOM FULL-WIDTH SECTION: Running Docker Containers (Matches Screenshot 2) */}
          {detailDockerSnap?.installed && (
            <div className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2 text-xs font-semibold text-muted-foreground">
                  <Container className="w-4 h-4 text-sky-400" />
                  <span>Docker</span>
                </div>
                <span className="text-xs font-mono text-muted-foreground">
                  Running:{" "}
                  <strong className="text-foreground">
                    {runningContainers.length}
                  </strong>
                </span>
              </div>

              {runningContainers.length === 0 ? (
                <div className="py-6 text-center text-xs text-muted-foreground">
                  No running containers on this machine.
                </div>
              ) : (
                <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-5 gap-3.5">
                  {runningContainers.map((c) => {
                    const cpuPct = parsePercentString(c.cpuPerc);
                    const memPct = parsePercentString(c.memPerc);
                    const netParts = (c.netIo || "0B / 0B").split("/");
                    const blockParts = (c.blockIo || "0B / 0B").split("/");

                    return (
                      <div
                        key={c.id}
                        className="rounded-2xl border border-border/75 bg-background/60 p-4 flex flex-col justify-between gap-3.5 hover:border-primary/40 transition-all"
                      >
                        <div className="flex items-center justify-between gap-2">
                          <span className="text-[13px] font-bold text-foreground truncate">
                            {c.name}
                          </span>
                          <span className="inline-flex items-center gap-1 text-[10px] text-emerald-400 shrink-0">
                            <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
                            {c.status.replace(/^Up\s+/i, "")}
                          </span>
                        </div>

                        <div className="flex items-center justify-around py-1">
                          <CircularRingGauge
                            percent={cpuPct}
                            label="CPU"
                            size={56}
                          />
                          <CircularRingGauge
                            percent={memPct}
                            label="RAM"
                            size={56}
                          />
                        </div>

                        <div className="pt-2.5 border-t border-border/50 space-y-1.5 text-[11px]">
                          <div className="flex items-center justify-between">
                            <span className="text-muted-foreground">
                              Network
                            </span>
                            <div className="text-right font-mono text-[10.5px]">
                              <div className="text-emerald-400">
                                ↑ {(netParts[0] || "0 B").trim()}
                              </div>
                              <div className="text-sky-400">
                                ↓ {(netParts[1] || "0 B").trim()}
                              </div>
                            </div>
                          </div>
                          <div className="flex items-center justify-between">
                            <span className="text-muted-foreground">
                              Block IO
                            </span>
                            <div className="text-right font-mono text-[10.5px] text-foreground">
                              <div>R {(blockParts[0] || "0 B").trim()}</div>
                              <div>W {(blockParts[1] || "0 B").trim()}</div>
                            </div>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}
        </div>
      </div>
    );
  }

  // ============================================================================
  // DASHBOARD OVERVIEW MODE (Matches Reference Screenshot 7)
  // ============================================================================
  const localMetrics = localMonitorEnabled ? metricsMap["local"] : undefined;
  const localMemPct =
    localMetrics && localMetrics.memTotalMb > 0
      ? (localMetrics.memUsedMb / localMetrics.memTotalMb) * 100
      : 0;

  const metricBarColor = (pct: number) =>
    pct > 85 ? "bg-rose-500" : pct > 65 ? "bg-amber-400" : "bg-emerald-500";

  const tagFilterOptions = [
    { value: "ALL", label: `All Tags (${hosts.length + 1})` },
    { value: "NO_TAGS", label: "No Tags" },
    ...resources.tags.map((tag) => ({
      value: tag.name,
      label: tag.name,
    })),
  ];

  const unifiedRows = [
    {
      id: "local",
      isLocal: true,
      name: "Localhost",
      subtitle: localMetrics?.osName || "macOS",
      hostObj: null as HostEntry | null,
      metrics: localMetrics,
      memPct: localMemPct,
      autoUpdate: localMonitorEnabled,
    },
    ...filteredHosts.map((host) => {
      const autoUpdate = host.showStatus !== false;
      const m = autoUpdate ? metricsMap[host.id] : undefined;
      const memPct =
        m && m.memTotalMb > 0 ? (m.memUsedMb / m.memTotalMb) * 100 : 0;
      return {
        id: host.id,
        isLocal: false,
        name: host.alias || host.hostname,
        subtitle: `${host.user}@${host.hostname}:${host.port}`,
        hostObj: host,
        metrics: m,
        memPct,
        autoUpdate,
      };
    }),
  ];

  return (
    <div className="flex flex-col h-full w-full bg-background text-foreground overflow-hidden select-none ui-fade-slide">
      {/* Top Header */}
      <div className="h-14 shrink-0 px-6 border-b border-border/80 bg-header/50 backdrop-blur-md flex items-center justify-between gap-3">
        <div className="flex items-center gap-2 min-w-0">
          <div className="w-44">
            <SelectField
              size="sm"
              value={selectedTagId}
              onChange={setSelectedTagId}
              options={tagFilterOptions}
            />
          </div>
          {selectedTagId !== "ALL" && (
            <button
              type="button"
              onClick={() => setSelectedTagId("ALL")}
              title="Clear Tag Filter"
              className="h-7 px-2 rounded-lg bg-secondary/80 hover:bg-accent text-muted-foreground hover:text-foreground border border-border text-xs inline-flex items-center gap-1 cursor-pointer transition-colors"
            >
              <X className="w-3 h-3" />
              <span>Reset</span>
            </button>
          )}
        </div>

        <div className="flex items-center gap-2 shrink-0">
          <div className="relative w-52">
            <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
            <input
              type="text"
              value={searchQuery}
              onChange={(e) => setSearchQuery(e.target.value)}
              placeholder="Search machines..."
              className="w-full h-8 pl-8 pr-3 rounded-xl bg-secondary/80 border border-border text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
            />
          </div>

          <div
            role="group"
            aria-label="Dashboard display mode"
            className="h-8 inline-flex items-center rounded-xl bg-secondary/80 p-0.5 border border-border"
          >
            <button
              type="button"
              onClick={() => handleChangeDisplayStyle("grid")}
              title="Detailed Cards View"
              className={`p-1.5 rounded-lg cursor-pointer transition-colors ${
                displayStyle === "grid"
                  ? "bg-card text-primary shadow-2xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <LayoutGrid className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => handleChangeDisplayStyle("compact")}
              title="Compact Cards View"
              className={`p-1.5 rounded-lg cursor-pointer transition-colors ${
                displayStyle === "compact"
                  ? "bg-card text-primary shadow-2xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <Grip className="w-3.5 h-3.5" />
            </button>
            <button
              type="button"
              onClick={() => handleChangeDisplayStyle("list")}
              title="Table List View"
              className={`p-1.5 rounded-lg cursor-pointer transition-colors ${
                displayStyle === "list"
                  ? "bg-card text-primary shadow-2xs"
                  : "text-muted-foreground hover:text-foreground"
              }`}
            >
              <List className="w-3.5 h-3.5" />
            </button>
          </div>

          <button
            type="button"
            onClick={onOpenAddMachine}
            className="h-8 inline-flex items-center gap-1.5 px-3 rounded-xl bg-primary text-primary-foreground text-xs font-semibold shadow-xs hover:opacity-95 transition-all cursor-pointer active:scale-97"
          >
            <Plus className="w-3.5 h-3.5" />
            <span>Add Machine</span>
          </button>
        </div>
      </div>

      {/* Main Dashboard Content */}
      <div className="flex-1 overflow-y-auto p-6 space-y-6">
        {/* 3 Summary Counter Cards: Total / Online / Offline */}
        <div className="grid grid-cols-1 sm:grid-cols-3 gap-4">
          <div className="rounded-2xl border border-border/80 bg-card/75 p-5 flex items-center justify-between shadow-xs">
            <div>
              <div className="text-3xl font-bold font-mono text-foreground">
                {totalMachinesCount}
              </div>
              <div className="text-xs font-medium text-muted-foreground mt-1">
                Total
              </div>
            </div>
            <div className="w-11 h-11 rounded-2xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
              <Server className="w-5 h-5" />
            </div>
          </div>

          <div className="rounded-2xl border border-border/80 bg-card/75 p-5 flex items-center justify-between shadow-xs">
            <div>
              <div className="text-3xl font-bold font-mono text-foreground">
                {onlineMachinesCount}
              </div>
              <div className="text-xs font-medium text-muted-foreground mt-1">
                Online
              </div>
            </div>
            <div className="w-11 h-11 rounded-2xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
              <CheckCircle2 className="w-5 h-5" />
            </div>
          </div>

          <div className="rounded-2xl border border-border/80 bg-card/75 p-5 flex items-center justify-between shadow-xs">
            <div>
              <div className="text-3xl font-bold font-mono text-foreground">
                {offlineMachinesCount}
              </div>
              <div className="text-xs font-medium text-muted-foreground mt-1">
                Standby
              </div>
            </div>
            <div className="w-11 h-11 rounded-2xl bg-rose-500/15 border border-rose-500/30 flex items-center justify-center text-rose-400">
              <XCircle className="w-5 h-5" />
            </div>
          </div>
        </div>

        {/* MODE 1: DETAILED GRID */}
        {displayStyle === "grid" && (
          <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-4">
            {/* 1. Localhost Machine Card */}
            <div
              onClick={() => onSelectHost("local")}
              className="group rounded-2xl border border-border/80 bg-card/75 hover:bg-card hover:border-primary/50 p-5 flex flex-col justify-between gap-4 shadow-xs transition-all duration-200 cursor-pointer hover:-translate-y-0.5"
            >
              <div className="flex items-center justify-between gap-2">
                <div className="flex items-center gap-2.5 min-w-0">
                  <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shrink-0">
                    <Laptop className="w-4 h-4" />
                  </div>
                  <div className="min-w-0">
                    <div className="text-sm font-bold text-foreground truncate">
                      Localhost
                    </div>
                    <div className="text-[11px] text-muted-foreground truncate">
                      {localMetrics?.osName || "macOS"}
                    </div>
                  </div>
                </div>

                <div className="flex items-center gap-2 shrink-0">
                  {renderMonitorSwitch("local", localMonitorEnabled)}
                  {onOpenLocalTerminal && (
                    <button
                      type="button"
                      onClick={(e) => {
                        e.stopPropagation();
                        onOpenLocalTerminal();
                      }}
                      title="Open Local Terminal"
                      className="p-2 rounded-xl bg-secondary/90 hover:bg-primary hover:text-primary-foreground text-muted-foreground transition-colors cursor-pointer"
                    >
                      <TerminalIcon className="w-4 h-4" />
                    </button>
                  )}
                </div>
              </div>

              <div className="space-y-1.5 text-xs border-t border-border/60 pt-3">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Cores</span>
                  <span className="font-mono font-semibold text-foreground">
                    {localMetrics?.cpuCores ?? "—"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">RAM</span>
                  <span className="font-mono font-semibold text-foreground">
                    {localMetrics
                      ? `${(localMetrics.memTotalMb / 1024).toFixed(2)} GB`
                      : "—"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Disk</span>
                  <span className="font-mono font-semibold text-foreground">
                    {localMetrics
                      ? `${localMetrics.diskTotalGb.toFixed(2)} GB`
                      : "—"}
                  </span>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Uptime</span>
                  <span className="font-mono font-semibold text-foreground">
                    {localMetrics
                      ? formatUptime(localMetrics.uptimeSeconds)
                      : localMonitorEnabled
                        ? "Connecting..."
                        : "Auto-update off"}
                  </span>
                </div>
              </div>

              <div className="flex items-center justify-around py-2 border-t border-border/60">
                <CircularRingGauge
                  percent={localMetrics?.cpuPercent ?? 0}
                  label="CPU"
                  size={64}
                />
                <CircularRingGauge
                  percent={localMemPct}
                  label="RAM"
                  size={64}
                />
              </div>

              <div className="space-y-1.5 text-xs border-t border-border/60 pt-3">
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Network</span>
                  <div className="text-right font-mono text-[11px]">
                    <div className="text-emerald-400">
                      ↑ {formatBytesPerSec(localMetrics?.netTxKbps ?? 0)}
                    </div>
                    <div className="text-sky-400">
                      ↓ {formatBytesPerSec(localMetrics?.netRxKbps ?? 0)}
                    </div>
                  </div>
                </div>
                <div className="flex items-center justify-between">
                  <span className="text-muted-foreground">Disk</span>
                  <div className="text-right font-mono text-[11px] text-foreground">
                    <div>
                      R{" "}
                      {formatRawBytesPerSec(
                        localMetrics?.volumes?.[0]?.readBytesPerSec ?? 0,
                      )}
                    </div>
                    <div>
                      W{" "}
                      {formatRawBytesPerSec(
                        localMetrics?.volumes?.[0]?.writeBytesPerSec ?? 0,
                      )}
                    </div>
                  </div>
                </div>
              </div>
            </div>

            {/* 2. Configured Remote SSH Machine Cards */}
            {filteredHosts.map((host) => {
              const autoUpdate = host.showStatus !== false;
              const m = autoUpdate ? metricsMap[host.id] : undefined;
              const memPct =
                m && m.memTotalMb > 0 ? (m.memUsedMb / m.memTotalMb) * 100 : 0;

              return (
                <div
                  key={host.id}
                  onClick={() => onSelectHost(host.id)}
                  className="group rounded-2xl border border-border/80 bg-card/75 hover:bg-card hover:border-primary/50 p-5 flex flex-col justify-between gap-4 shadow-xs transition-all duration-200 cursor-pointer hover:-translate-y-0.5"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2.5 min-w-0">
                      <div className="w-8 h-8 rounded-xl bg-orange-500/15 border border-orange-500/30 flex items-center justify-center text-orange-400 shrink-0">
                        <Server className="w-4 h-4" />
                      </div>
                      <div className="min-w-0">
                        <div className="text-sm font-bold text-foreground truncate">
                          {host.alias || host.hostname}
                        </div>
                        <div className="text-[11px] text-muted-foreground font-mono truncate">
                          {host.user}@{host.hostname}:{host.port}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-2 shrink-0">
                      {renderMonitorSwitch(host.id, autoUpdate)}
                      {onOpenVpsConfig && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onOpenVpsConfig(host.id);
                          }}
                          title="VPS Hardening & Config"
                          className="p-2 rounded-xl bg-secondary/90 hover:bg-accent text-muted-foreground hover:text-emerald-400 transition-colors cursor-pointer"
                        >
                          <Shield className="w-4 h-4" />
                        </button>
                      )}
                      <button
                        type="button"
                        onClick={(e) => {
                          e.stopPropagation();
                          onConnectHostTerminal(host);
                        }}
                        title="Connect SSH Terminal"
                        className="p-2 rounded-xl bg-secondary/90 hover:bg-primary hover:text-primary-foreground text-muted-foreground transition-colors cursor-pointer"
                      >
                        <TerminalIcon className="w-4 h-4" />
                      </button>
                    </div>
                  </div>

                  <div className="space-y-1.5 text-xs border-t border-border/60 pt-3">
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Cores</span>
                      <span className="font-mono font-semibold text-foreground">
                        {m?.cpuCores ?? "—"}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">RAM</span>
                      <span className="font-mono font-semibold text-foreground">
                        {m ? `${(m.memTotalMb / 1024).toFixed(2)} GB` : "—"}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Disk</span>
                      <span className="font-mono font-semibold text-foreground">
                        {m ? `${m.diskTotalGb.toFixed(2)} GB` : "—"}
                      </span>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Uptime</span>
                      <span className="font-mono font-semibold text-foreground">
                        {m
                          ? formatUptime(m.uptimeSeconds)
                          : autoUpdate
                            ? "Connecting..."
                            : "Auto-update off"}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center justify-around py-2 border-t border-border/60">
                    <CircularRingGauge
                      percent={m?.cpuPercent ?? 0}
                      label="CPU"
                      size={64}
                    />
                    <CircularRingGauge
                      percent={memPct}
                      label="RAM"
                      size={64}
                    />
                  </div>

                  <div className="space-y-1.5 text-xs border-t border-border/60 pt-3">
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Network</span>
                      <div className="text-right font-mono text-[11px]">
                        <div className="text-emerald-400">
                          ↑ {formatBytesPerSec(m?.netTxKbps ?? 0)}
                        </div>
                        <div className="text-sky-400">
                          ↓ {formatBytesPerSec(m?.netRxKbps ?? 0)}
                        </div>
                      </div>
                    </div>
                    <div className="flex items-center justify-between">
                      <span className="text-muted-foreground">Disk</span>
                      <div className="text-right font-mono text-[11px] text-foreground">
                        <div>
                          R{" "}
                          {formatRawBytesPerSec(
                            m?.volumes?.[0]?.readBytesPerSec ?? 0,
                          )}
                        </div>
                        <div>
                          W{" "}
                          {formatRawBytesPerSec(
                            m?.volumes?.[0]?.writeBytesPerSec ?? 0,
                          )}
                        </div>
                      </div>
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* MODE 2: COMPACT GRID */}
        {displayStyle === "compact" && (
          <div className="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 2xl:grid-cols-5 gap-3">
            {unifiedRows.map((row) => {
              const m = row.metrics;
              const cpuPct = Math.min(100, Math.max(0, m?.cpuPercent ?? 0));
              const ramPct = Math.min(100, Math.max(0, row.memPct));
              const diskPct = Math.min(100, Math.max(0, m?.diskPercent ?? 0));

              return (
                <div
                  key={row.id}
                  onClick={() => onSelectHost(row.id)}
                  className="group rounded-2xl border border-border/80 bg-card/75 hover:bg-card hover:border-primary/50 p-3.5 flex flex-col justify-between gap-3 shadow-xs transition-all duration-150 cursor-pointer"
                >
                  <div className="flex items-center justify-between gap-2">
                    <div className="flex items-center gap-2 min-w-0">
                      <div
                        className={`w-7 h-7 rounded-xl flex items-center justify-center shrink-0 border ${
                          row.isLocal
                            ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-400"
                            : "bg-orange-500/15 border-orange-500/30 text-orange-400"
                        }`}
                      >
                        {row.isLocal ? (
                          <Laptop className="w-3.5 h-3.5" />
                        ) : (
                          <Server className="w-3.5 h-3.5" />
                        )}
                      </div>
                      <div className="min-w-0">
                        <div className="text-xs font-bold text-foreground truncate flex items-center gap-1.5">
                          <span className="truncate">{row.name}</span>
                          <span
                            className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                              m ? "bg-emerald-400" : "bg-muted-foreground/40"
                            }`}
                          />
                        </div>
                        <div className="text-[10.5px] text-muted-foreground font-mono truncate">
                          {row.subtitle}
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center gap-1.5 shrink-0">
                      {renderMonitorSwitch(row.id, row.autoUpdate, true)}
                      {(!row.isLocal || onOpenLocalTerminal) && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            if (row.isLocal) {
                              onOpenLocalTerminal?.();
                            } else if (row.hostObj) {
                              onConnectHostTerminal(row.hostObj);
                            }
                          }}
                          title="Open Terminal"
                          className="p-1.5 rounded-lg bg-secondary/90 hover:bg-primary hover:text-primary-foreground text-muted-foreground transition-colors cursor-pointer"
                        >
                          <TerminalIcon className="w-3.5 h-3.5" />
                        </button>
                      )}
                      {!row.isLocal && onOpenSftpManager && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onOpenSftpManager(row.id);
                          }}
                          title="Open SFTP"
                          className="p-1.5 rounded-lg bg-secondary/90 hover:bg-accent text-muted-foreground hover:text-amber-400 transition-colors cursor-pointer"
                        >
                          <HardDrive className="w-3.5 h-3.5" />
                        </button>
                      )}
                      {!row.isLocal && onOpenVpsConfig && (
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onOpenVpsConfig(row.id);
                          }}
                          title="VPS Hardening & Config"
                          className="p-1.5 rounded-lg bg-secondary/90 hover:bg-accent text-muted-foreground hover:text-emerald-400 transition-colors cursor-pointer"
                        >
                          <Shield className="w-3.5 h-3.5" />
                        </button>
                      )}
                    </div>
                  </div>

                  <div className="space-y-1.5 pt-2 border-t border-border/50 text-[11px]">
                    <div className="flex items-center gap-2">
                      <span className="w-8 text-muted-foreground">CPU</span>
                      <div className="flex-1 h-1.5 rounded-full bg-secondary overflow-hidden">
                        <div
                          className={`h-full rounded-full transition-all duration-300 ${metricBarColor(
                            cpuPct,
                          )}`}
                          style={{ width: `${cpuPct}%` }}
                        />
                      </div>
                      <span className="w-9 text-right font-mono font-semibold text-foreground">
                        {m ? `${Math.round(cpuPct)}%` : "—"}
                      </span>
                    </div>

                    <div className="flex items-center gap-2">
                      <span className="w-8 text-muted-foreground">RAM</span>
                      <div className="flex-1 h-1.5 rounded-full bg-secondary overflow-hidden">
                        <div
                          className={`h-full rounded-full transition-all duration-300 ${metricBarColor(
                            ramPct,
                          )}`}
                          style={{ width: `${ramPct}%` }}
                        />
                      </div>
                      <span className="w-9 text-right font-mono font-semibold text-foreground">
                        {m ? `${Math.round(ramPct)}%` : "—"}
                      </span>
                    </div>

                    <div className="flex items-center gap-2">
                      <span className="w-8 text-muted-foreground">Disk</span>
                      <div className="flex-1 h-1.5 rounded-full bg-secondary overflow-hidden">
                        <div
                          className={`h-full rounded-full transition-all duration-300 ${metricBarColor(
                            diskPct,
                          )}`}
                          style={{ width: `${diskPct}%` }}
                        />
                      </div>
                      <span className="w-9 text-right font-mono font-semibold text-foreground">
                        {m ? `${Math.round(diskPct)}%` : "—"}
                      </span>
                    </div>
                  </div>

                  <div className="flex items-center justify-between pt-2 border-t border-border/50 text-[10.5px] font-mono text-muted-foreground">
                    <span>
                      {m
                        ? formatUptime(m.uptimeSeconds)
                        : row.autoUpdate
                          ? "Connecting..."
                          : "Auto-update off"}
                    </span>
                    <span className="inline-flex items-center gap-2">
                      <span className="text-emerald-400">
                        ↑ {formatBytesPerSec(m?.netTxKbps ?? 0)}
                      </span>
                      <span className="text-sky-400">
                        ↓ {formatBytesPerSec(m?.netRxKbps ?? 0)}
                      </span>
                    </span>
                  </div>
                </div>
              );
            })}
          </div>
        )}

        {/* MODE 3: TABLE LIST VIEW */}
        {displayStyle === "list" && (
          <div
            tabIndex={0}
            role="region"
            aria-label="Machines telemetry table"
            className="rounded-2xl border border-border/80 bg-card/75 overflow-x-auto shadow-xs"
          >
            <table className="w-full text-left border-collapse text-xs">
              <thead className="sticky top-0 z-10 bg-card/95 backdrop-blur-xs text-[11px] text-muted-foreground border-b border-border/70">
                <tr>
                  <th className="py-3 px-4 font-semibold">Machine</th>
                  <th className="py-3 px-3 font-semibold">System / Uptime</th>
                  <th className="py-3 px-3 font-semibold">CPU</th>
                  <th className="py-3 px-3 font-semibold">RAM</th>
                  <th className="py-3 px-3 font-semibold">Disk</th>
                  <th className="py-3 px-3 font-semibold">Network</th>
                  <th className="py-3 px-4 font-semibold text-right">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border/50">
                {unifiedRows.map((row) => {
                  const m = row.metrics;
                  const cpuPct = Math.min(100, Math.max(0, m?.cpuPercent ?? 0));
                  const ramPct = Math.min(100, Math.max(0, row.memPct));
                  const diskPct = Math.min(
                    100,
                    Math.max(0, m?.diskPercent ?? 0),
                  );

                  return (
                    <tr
                      key={row.id}
                      onClick={() => onSelectHost(row.id)}
                      className="group hover:bg-accent/50 transition-colors cursor-pointer"
                    >
                      <th
                        scope="row"
                        className="py-2.5 px-4 font-normal text-left"
                      >
                        <div className="flex items-center gap-2.5 min-w-0">
                          <div
                            className={`w-7 h-7 rounded-xl flex items-center justify-center shrink-0 border ${
                              row.isLocal
                                ? "bg-emerald-500/15 border-emerald-500/30 text-emerald-400"
                                : "bg-orange-500/15 border-orange-500/30 text-orange-400"
                            }`}
                          >
                            {row.isLocal ? (
                              <Laptop className="w-3.5 h-3.5" />
                            ) : (
                              <Server className="w-3.5 h-3.5" />
                            )}
                          </div>
                          <div className="min-w-0">
                            <div className="text-xs font-bold text-foreground truncate flex items-center gap-1.5">
                              <span>{row.name}</span>
                              <span
                                className={`w-1.5 h-1.5 rounded-full shrink-0 ${
                                  m
                                    ? "bg-emerald-400"
                                    : "bg-muted-foreground/40"
                                }`}
                              />
                            </div>
                            <div className="text-[11px] text-muted-foreground font-mono truncate">
                              {row.subtitle}
                            </div>
                          </div>
                        </div>
                      </th>

                      <td className="py-2.5 px-3 font-mono text-[11px] text-muted-foreground">
                        <div className="text-foreground">
                          {m?.osName || (row.isLocal ? "macOS" : "Remote SSH")}
                        </div>
                        <div>
                          {m
                            ? `${m.cpuCores}c • ${formatUptime(m.uptimeSeconds)}`
                            : row.autoUpdate
                              ? "Connecting..."
                              : "Auto-update off"}
                        </div>
                      </td>

                      <td className="py-2.5 px-3 min-w-[120px]">
                        <div className="flex items-center gap-2">
                          <div className="w-16 h-1.5 rounded-full bg-secondary overflow-hidden">
                            <div
                              className={`h-full rounded-full ${metricBarColor(
                                cpuPct,
                              )}`}
                              style={{ width: `${cpuPct}%` }}
                            />
                          </div>
                          <span className="font-mono font-semibold text-foreground text-[11px]">
                            {m ? `${Math.round(cpuPct)}%` : "—"}
                          </span>
                        </div>
                      </td>

                      <td className="py-2.5 px-3 min-w-[140px]">
                        <div className="flex items-center gap-2">
                          <div className="w-16 h-1.5 rounded-full bg-secondary overflow-hidden">
                            <div
                              className={`h-full rounded-full ${metricBarColor(
                                ramPct,
                              )}`}
                              style={{ width: `${ramPct}%` }}
                            />
                          </div>
                          <span className="font-mono text-[11px] text-foreground">
                            {m
                              ? `${Math.round(ramPct)}% (${(
                                  m.memTotalMb / 1024
                                ).toFixed(1)}G)`
                              : "—"}
                          </span>
                        </div>
                      </td>

                      <td className="py-2.5 px-3 min-w-[120px]">
                        <div className="flex items-center gap-2">
                          <div className="w-16 h-1.5 rounded-full bg-secondary overflow-hidden">
                            <div
                              className={`h-full rounded-full ${metricBarColor(
                                diskPct,
                              )}`}
                              style={{ width: `${diskPct}%` }}
                            />
                          </div>
                          <span className="font-mono text-[11px] text-foreground">
                            {m ? `${Math.round(diskPct)}%` : "—"}
                          </span>
                        </div>
                      </td>

                      <td className="py-2.5 px-3 font-mono text-[11px]">
                        <div className="text-emerald-400">
                          ↑ {formatBytesPerSec(m?.netTxKbps ?? 0)}
                        </div>
                        <div className="text-sky-400">
                          ↓ {formatBytesPerSec(m?.netRxKbps ?? 0)}
                        </div>
                      </td>

                      <td
                        className="py-2.5 px-4 text-right"
                        onClick={(e) => e.stopPropagation()}
                      >
                        <div className="inline-flex items-center justify-end gap-1.5">
                          {renderMonitorSwitch(row.id, row.autoUpdate, true)}
                          {(!row.isLocal || onOpenLocalTerminal) && (
                            <button
                              type="button"
                              onClick={() => {
                                if (row.isLocal) {
                                  onOpenLocalTerminal?.();
                                } else if (row.hostObj) {
                                  onConnectHostTerminal(row.hostObj);
                                }
                              }}
                              title="Open Terminal"
                              className="p-1.5 rounded-lg bg-secondary/90 hover:bg-primary hover:text-primary-foreground text-muted-foreground transition-colors cursor-pointer"
                            >
                              <TerminalIcon className="w-3.5 h-3.5" />
                            </button>
                          )}
                          {!row.isLocal && onOpenSftpManager && (
                            <button
                              type="button"
                              onClick={() => onOpenSftpManager(row.id)}
                              title="Open SFTP"
                              className="p-1.5 rounded-lg bg-secondary/90 hover:bg-accent text-muted-foreground hover:text-amber-400 transition-colors cursor-pointer"
                            >
                              <HardDrive className="w-3.5 h-3.5" />
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() => onOpenDockerManager(row.id)}
                            title="Open Docker Manager"
                            className="p-1.5 rounded-lg bg-secondary/90 hover:bg-accent text-muted-foreground hover:text-sky-400 transition-colors cursor-pointer"
                          >
                            <Container className="w-3.5 h-3.5" />
                          </button>
                          {!row.isLocal && onOpenVpsConfig && (
                            <button
                              type="button"
                              onClick={() => onOpenVpsConfig(row.id)}
                              title="VPS Hardening & Config"
                              className="p-1.5 rounded-lg bg-secondary/90 hover:bg-accent text-muted-foreground hover:text-emerald-400 transition-colors cursor-pointer"
                            >
                              <Shield className="w-3.5 h-3.5" />
                            </button>
                          )}
                        </div>
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          </div>
        )}
      </div>
    </div>
  );
};
