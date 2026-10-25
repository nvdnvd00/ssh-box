import React, { useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  ArrowDownRight,
  ArrowUpRight,
  Clock,
  Container,
  Cpu,
  Gauge,
  HardDrive,
  Layers,
  Maximize2,
  MemoryStick,
  Minimize2,
  Network,
  Search,
  Skull,
  Thermometer,
  X,
} from "lucide-react";
import { ServerMetrics } from "../types";
import { ACCENT_COLORS, AccentColorId } from "../themes";
import { SelectField } from "./SelectField";
import { withShortcutHint } from "../shortcuts";

interface TelemetryDrawerProps {
  isOpen: boolean;
  hostId: string;
  hostLabel: string;
  accentColor: AccentColorId;
  metrics: ServerMetrics | null;
  cpuHistory: number[];
  memHistory: number[];
  rxHistory: number[];
  txHistory: number[];
  onClose: () => void;
  onOpenDockerPanel?: () => void;
}

function formatUptime(seconds: number): string {
  if (!seconds || seconds <= 0) return "—";
  const d = Math.floor(seconds / 86400);
  const h = Math.floor((seconds % 86400) / 3600);
  const m = Math.floor((seconds % 3600) / 60);
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

function formatBytesTotal(bytes: number): string {
  if (!bytes || bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  let val = bytes;
  let idx = 0;
  while (val >= 1024 && idx < units.length - 1) {
    val /= 1024;
    idx++;
  }
  return `${val.toFixed(idx > 1 ? 2 : 0)} ${units[idx]}`;
}

function formatRateBytes(bps: number): string {
  if (!bps || bps <= 0) return "0 B/s";
  if (bps < 1024) return `${bps.toFixed(0)} B/s`;
  if (bps < 1024 * 1024) return `${(bps / 1024).toFixed(1)} KB/s`;
  return `${(bps / (1024 * 1024)).toFixed(2)} MB/s`;
}

const Sparkline: React.FC<{
  data: number[];
  maxVal?: number;
  stroke: string;
  fill: string;
}> = ({ data, maxVal, stroke, fill }) => {
  const width = 220;
  const height = 40;
  const padded = data.length >= 2 ? data : [0, ...(data.length ? data : [0])];
  const peak = Math.max(maxVal ?? 0, ...padded, 1);

  const points = padded.map((v, idx) => {
    const x = (idx / (padded.length - 1)) * width;
    const y = height - Math.min(1, Math.max(0, v / peak)) * (height - 6) - 3;
    return `${x.toFixed(1)},${y.toFixed(1)}`;
  });

  const linePath = `M ${points.join(" L ")}`;
  const areaPath = `${linePath} L ${width},${height} L 0,${height} Z`;

  return (
    <svg
      viewBox={`0 0 ${width} ${height}`}
      className="w-full h-10 overflow-visible"
      preserveAspectRatio="none"
    >
      <path d={areaPath} fill={fill} className="transition-all duration-300" />
      <path
        d={linePath}
        fill="none"
        stroke={stroke}
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
        className="transition-all duration-300"
      />
    </svg>
  );
};

export const TelemetryDrawer: React.FC<TelemetryDrawerProps> = ({
  isOpen,
  hostId,
  hostLabel,
  accentColor,
  metrics,
  cpuHistory,
  memHistory,
  rxHistory,
  txHistory,
  onClose,
  onOpenDockerPanel,
}) => {
  const [expandedWidth, setExpandedWidth] = useState(false);
  const [tempUnit, setTempUnit] = useState<"C" | "F">("C");
  const [showPerCore, setShowPerCore] = useState(false);
  const [selectedNic, setSelectedNic] = useState<string>("ALL");
  const [selectedVolume, setSelectedVolume] = useState<string>("");
  const [procSearch, setProcSearch] = useState("");
  const [procSort, setProcSort] = useState<"cpu" | "mem" | "pid">("cpu");
  const [killingPid, setKillingPid] = useState<number | null>(null);

  const activeAccent =
    ACCENT_COLORS.find((a) => a.id === accentColor) ?? ACCENT_COLORS[0];

  const memTotal = metrics?.memTotalMb ?? 0;
  const memUsed = metrics?.memUsedMb ?? 0;
  const memCached = metrics?.memCachedMb ?? 0;
  const memFree = metrics?.memFreeMb ?? 0;
  const memPct = memTotal > 0 ? Math.min(100, (memUsed / memTotal) * 100) : 0;
  const cachedPct =
    memTotal > 0 ? Math.min(100 - memPct, (memCached / memTotal) * 100) : 0;

  const swapTotal = metrics?.swapTotalMb ?? 0;
  const swapUsed = metrics?.swapUsedMb ?? 0;
  const swapPct =
    swapTotal > 0 ? Math.min(100, (swapUsed / swapTotal) * 100) : 0;

  const activeNicObj = useMemo(() => {
    if (!metrics || !metrics.interfaces?.length) return null;
    if (selectedNic === "ALL") return null;
    return (
      metrics.interfaces.find((n) => n.name === selectedNic) ??
      metrics.interfaces[0]
    );
  }, [metrics, selectedNic]);

  const activeVolumeObj = useMemo(() => {
    if (!metrics || !metrics.volumes?.length) return null;
    if (selectedVolume) {
      const found = metrics.volumes.find(
        (v) => v.mount === selectedVolume || v.device === selectedVolume,
      );
      if (found) return found;
    }
    return (
      metrics.volumes.find((v) => v.mount === "/") ?? metrics.volumes[0] ?? null
    );
  }, [metrics, selectedVolume]);

  const filteredProcesses = useMemo(() => {
    const list = [...(metrics?.topProcesses ?? [])];
    const q = procSearch.trim().toLowerCase();
    const filtered = q
      ? list.filter(
          (p) =>
            String(p.pid).includes(q) ||
            p.command.toLowerCase().includes(q) ||
            (p.user ?? "").toLowerCase().includes(q) ||
            (p.args ?? "").toLowerCase().includes(q),
        )
      : list;

    filtered.sort((a, b) => {
      if (procSort === "mem") return b.memPct - a.memPct;
      if (procSort === "pid") return a.pid - b.pid;
      return b.cpuPct - a.cpuPct;
    });
    return filtered.slice(0, 25);
  }, [metrics?.topProcesses, procSearch, procSort]);

  const handleKillProcess = async (pid: number) => {
    setKillingPid(pid);
    try {
      await invoke("kill_process", {
        hostId,
        pid,
        force: false,
      });
    } catch {
      // ignore error
    } finally {
      setKillingPid(null);
    }
  };

  const displayTemp = useMemo(() => {
    const c = metrics?.temperatureCelsius;
    if (c === undefined || c === null) return null;
    if (tempUnit === "F") {
      return `${((c * 9) / 5 + 32).toFixed(1)}°F`;
    }
    return `${c.toFixed(1)}°C`;
  }, [metrics?.temperatureCelsius, tempUnit]);

  const widthClass = expandedWidth ? "w-[410px]" : "w-80";

  return (
    <aside
      aria-hidden={!isOpen}
      className={`ui-drawer-shell shrink-0 h-full bg-card/95 border-border overflow-hidden select-none transition-all duration-200 ${
        isOpen
          ? `${widthClass} opacity-100 border-l`
          : "w-0 opacity-0 border-l-0 pointer-events-none"
      }`}
    >
      <div className={`${widthClass} h-full flex flex-col overflow-y-auto`}>
        {/* Header */}
        <div className="px-4 py-3 border-b border-border flex items-center justify-between shrink-0">
          <div className="flex items-center gap-2.5 min-w-0">
            <Activity className="w-4 h-4 text-primary shrink-0" />
            <div className="min-w-0">
              <div className="text-xs font-semibold text-foreground truncate">
                {metrics?.hostname || "System Monitor"}
              </div>
              <div className="text-[11px] text-muted-foreground truncate">
                {hostLabel}
              </div>
            </div>
          </div>

          <div className="flex items-center gap-1">
            {displayTemp && (
              <button
                type="button"
                onClick={() => setTempUnit((u) => (u === "C" ? "F" : "C"))}
                title="Toggle °C / °F"
                className="px-2 py-0.5 rounded-md bg-amber-500/15 text-amber-300 text-[11px] font-medium inline-flex items-center gap-1 cursor-pointer"
              >
                <Thermometer className="w-3 h-3" />
                {displayTemp}
              </button>
            )}
            <button
              type="button"
              onClick={() => setExpandedWidth((p) => !p)}
              title="Toggle Detailed Monitor Width"
              className="p-1.5 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
            >
              {expandedWidth ? (
                <Minimize2 className="w-3.5 h-3.5" />
              ) : (
                <Maximize2 className="w-3.5 h-3.5" />
              )}
            </button>
            <button
              type="button"
              onClick={onClose}
              title={withShortcutHint("Close Monitor", "toggleMonitor")}
              className="p-1.5 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        <div className="p-3.5 space-y-3.5 text-xs">
          {/* System Overview Pill Bar (OS, Uptime, CPU Model) */}
          {metrics && (
            <div className="p-3 rounded-2xl bg-secondary/45 border border-border/80 space-y-1">
              <div className="flex items-center justify-between text-xs">
                <span className="text-muted-foreground truncate max-w-[180px] font-medium">
                  {metrics.osName || "Linux / Unix"}
                </span>
                <span className="inline-flex items-center gap-1 text-[11px] font-medium text-emerald-400">
                  <Clock className="w-3 h-3" />
                  up {formatUptime(metrics.uptimeSeconds)}
                </span>
              </div>
              {metrics.cpuModel && (
                <div className="text-[11px] text-muted-foreground truncate">
                  {metrics.cpuModel} ({metrics.cpuCores} cores)
                </div>
              )}
            </div>
          )}

          {/* CPU Card + Load 1/5/15m + Per-Core Usage */}
          <div className="p-3.5 rounded-2xl bg-secondary/55 border border-border hover:border-muted-foreground/30 transition-colors space-y-2 shadow-2xs">
            <div className="flex items-center justify-between">
              <span className="flex items-center gap-1.5 text-card-foreground font-semibold">
                <Cpu className="w-3.5 h-3.5 text-primary" />
                CPU & Load
              </span>
              <div className="flex items-center gap-2">
                <span className="text-sm font-semibold text-primary">
                  {metrics ? `${metrics.cpuPercent.toFixed(1)}%` : "0.0%"}
                </span>
                {metrics && metrics.cores?.length > 0 && (
                  <button
                    type="button"
                    onClick={() => setShowPerCore((p) => !p)}
                    className="px-2 py-0.5 rounded-md bg-card border border-border text-[11px] font-medium text-muted-foreground hover:text-foreground cursor-pointer"
                  >
                    {showPerCore ? "Hide Cores" : `${metrics.cores.length}C`}
                  </button>
                )}
              </div>
            </div>

            <div className="flex items-center justify-between text-[11px] text-muted-foreground">
              <span>
                usr {metrics ? metrics.cpuUserPercent.toFixed(0) : 0}% • sys{" "}
                {metrics ? metrics.cpuSystemPercent.toFixed(0) : 0}%
              </span>
              <span>
                load {metrics ? metrics.loadOne.toFixed(2) : "0.00"} /{" "}
                {metrics ? metrics.loadFive.toFixed(2) : "0.00"} /{" "}
                {metrics ? metrics.loadFifteen.toFixed(2) : "0.00"}
              </span>
            </div>

            <Sparkline
              data={cpuHistory}
              maxVal={100}
              stroke={activeAccent.chartStroke}
              fill={activeAccent.chartFill}
            />

            {showPerCore && metrics && metrics.cores.length > 0 && (
              <div className="pt-2 border-t border-border/60 grid grid-cols-2 gap-1.5 max-h-36 overflow-y-auto">
                {metrics.cores.map((c) => (
                  <div
                    key={c.coreId}
                    className="p-2 rounded-lg bg-card/70 border border-border/60 space-y-1"
                  >
                    <div className="flex justify-between text-[11px]">
                      <span className="text-muted-foreground">{c.coreId}</span>
                      <span className="text-foreground font-medium">
                        {c.usagePct.toFixed(0)}%
                      </span>
                    </div>
                    <div className="w-full h-1.5 rounded-full bg-muted overflow-hidden">
                      <div
                        className="h-full bg-primary rounded-full"
                        style={{
                          width: `${Math.min(100, Math.max(0, c.usagePct))}%`,
                        }}
                      />
                    </div>
                  </div>
                ))}
              </div>
            )}
          </div>

          {/* Memory & Swap Breakdown Card */}
          <div className="p-3.5 rounded-2xl bg-secondary/55 border border-border hover:border-muted-foreground/30 transition-colors space-y-2 shadow-2xs">
            <div className="flex items-center justify-between">
              <span className="flex items-center gap-1.5 text-card-foreground font-semibold">
                <MemoryStick className="w-3.5 h-3.5 text-violet-400" />
                Memory & Swap
              </span>
              <span className="text-violet-400 font-semibold">
                {memPct.toFixed(1)}%
              </span>
            </div>

            <div className="w-full h-2 rounded-full bg-muted overflow-hidden flex">
              <div
                className="h-full bg-violet-400 transition-all duration-300"
                style={{ width: `${memPct}%` }}
                title={`Used: ${memUsed} MB`}
              />
              <div
                className="h-full bg-sky-400/75 transition-all duration-300"
                style={{ width: `${cachedPct}%` }}
                title={`Cached: ${memCached} MB`}
              />
            </div>

            <div className="grid grid-cols-3 gap-1 text-[11px] text-muted-foreground">
              <span>Used: {memUsed}M</span>
              <span>Cache: {memCached}M</span>
              <span className="text-right">Free: {memFree}M</span>
            </div>

            <Sparkline
              data={memHistory}
              maxVal={100}
              stroke="#a78bfa"
              fill="rgba(167, 139, 250, 0.14)"
            />

            {swapTotal > 0 && (
              <div className="pt-1.5 border-t border-border/60 space-y-1">
                <div className="flex justify-between text-[11px] text-muted-foreground">
                  <span>Swap</span>
                  <span>
                    {swapUsed} / {swapTotal} MB ({swapPct.toFixed(0)}%)
                  </span>
                </div>
                <div className="w-full h-1.5 rounded-full bg-muted overflow-hidden">
                  <div
                    className="h-full bg-amber-400 rounded-full transition-all"
                    style={{ width: `${swapPct}%` }}
                  />
                </div>
              </div>
            )}
          </div>

          {/* Multi-Interface Network Card */}
          <div className="p-3.5 rounded-2xl bg-secondary/55 border border-border hover:border-muted-foreground/30 transition-colors space-y-2 shadow-2xs">
            <div className="flex items-center justify-between gap-2">
              <span className="flex items-center gap-1.5 text-card-foreground font-semibold">
                <Network className="w-3.5 h-3.5 text-emerald-400" />
                Network
              </span>
              {metrics && metrics.interfaces?.length > 0 && (
                <div className="w-40">
                  <SelectField
                    size="sm"
                    value={selectedNic}
                    onChange={setSelectedNic}
                    options={[
                      { value: "ALL", label: "All Interfaces" },
                      ...metrics.interfaces.map((nic) => ({
                        value: nic.name,
                        label: `${nic.name}${nic.ip ? ` (${nic.ip})` : ""}`,
                      })),
                    ]}
                  />
                </div>
              )}
            </div>

            {activeNicObj && activeNicObj.ip && (
              <div className="text-[11px] text-sky-400">
                IP: {activeNicObj.ip}
              </div>
            )}

            <div className="flex items-center justify-between text-xs font-medium">
              <span className="inline-flex items-center text-emerald-400">
                <ArrowDownRight className="w-3.5 h-3.5 mr-0.5" />
                {(activeNicObj
                  ? activeNicObj.rxKbps
                  : (metrics?.netRxKbps ?? 0)
                ).toFixed(1)}{" "}
                KB/s
              </span>
              <span className="inline-flex items-center text-amber-400">
                <ArrowUpRight className="w-3.5 h-3.5 mr-0.5" />
                {(activeNicObj
                  ? activeNicObj.txKbps
                  : (metrics?.netTxKbps ?? 0)
                ).toFixed(1)}{" "}
                KB/s
              </span>
            </div>

            <div className="grid grid-cols-2 gap-2">
              <div>
                <div className="text-[11px] text-muted-foreground mb-0.5">
                  RX Stream
                </div>
                <Sparkline
                  data={rxHistory}
                  stroke="#34d399"
                  fill="rgba(52, 211, 153, 0.14)"
                />
              </div>
              <div>
                <div className="text-[11px] text-muted-foreground mb-0.5">
                  TX Stream
                </div>
                <Sparkline
                  data={txHistory}
                  stroke="#fbbf24"
                  fill="rgba(251, 191, 36, 0.14)"
                />
              </div>
            </div>

            {activeNicObj && (
              <div className="flex justify-between text-[11px] text-muted-foreground pt-1.5 border-t border-border/60">
                <span>
                  Total RX: {formatBytesTotal(activeNicObj.rxTotalBytes)}
                </span>
                <span>
                  Total TX: {formatBytesTotal(activeNicObj.txTotalBytes)}
                </span>
              </div>
            )}
          </div>

          {/* Multi-Volume Storage & Disk I/O Card */}
          <div className="p-3.5 rounded-2xl bg-secondary/55 border border-border hover:border-muted-foreground/30 transition-colors space-y-2 shadow-2xs">
            <div className="flex items-center justify-between gap-2">
              <span className="flex items-center gap-1.5 text-card-foreground font-semibold">
                <HardDrive className="w-3.5 h-3.5 text-sky-400" />
                Storage
              </span>
              {metrics && metrics.volumes?.length > 1 ? (
                <div className="w-40">
                  <SelectField
                    size="sm"
                    value={activeVolumeObj?.mount ?? "/"}
                    onChange={setSelectedVolume}
                    options={metrics.volumes.map((v) => ({
                      value: v.mount,
                      label: `${v.mount} (${v.device.split("/").pop()})`,
                    }))}
                  />
                </div>
              ) : (
                <span className="text-[11px] text-muted-foreground">
                  {activeVolumeObj?.mount ?? "/"}
                </span>
              )}
            </div>

            <div className="flex justify-between text-xs text-card-foreground">
              <span>
                {activeVolumeObj
                  ? `${activeVolumeObj.usedGb.toFixed(1)} / ${activeVolumeObj.totalGb.toFixed(1)} GB`
                  : `${(metrics?.diskUsedGb ?? 0).toFixed(1)} / ${(metrics?.diskTotalGb ?? 0).toFixed(1)} GB`}
              </span>
              <span className="text-sky-400 font-semibold">
                {(
                  activeVolumeObj?.usedPercent ??
                  metrics?.diskPercent ??
                  0
                ).toFixed(0)}
                %
              </span>
            </div>

            <div className="w-full h-2 rounded-full bg-muted overflow-hidden">
              <div
                className="h-full bg-sky-400 transition-all duration-500 ease-out rounded-full"
                style={{
                  width: `${Math.min(
                    100,
                    Math.max(
                      0,
                      activeVolumeObj?.usedPercent ?? metrics?.diskPercent ?? 0,
                    ),
                  )}%`,
                }}
              />
            </div>

            {activeVolumeObj && (
              <div className="flex justify-between text-[11px] text-muted-foreground pt-1.5 border-t border-border/60">
                <span>
                  R: {formatRateBytes(activeVolumeObj.readBytesPerSec)}
                </span>
                <span>
                  W: {formatRateBytes(activeVolumeObj.writeBytesPerSec)}
                </span>
              </div>
            )}
          </div>

          {/* NVIDIA GPU Card (if present) */}
          {metrics && metrics.gpus && metrics.gpus.length > 0 && (
            <div className="p-3.5 rounded-2xl bg-secondary/55 border border-border space-y-2 shadow-2xs">
              {metrics.gpus.map((gpu) => (
                <div key={gpu.index} className="space-y-1.5">
                  <div className="flex items-center justify-between">
                    <span className="flex items-center gap-1.5 font-semibold text-emerald-400 truncate">
                      <Gauge className="w-3.5 h-3.5 shrink-0" />
                      {gpu.name}
                    </span>
                    <span className="text-xs font-semibold text-emerald-400">
                      {gpu.utilizationGpu.toFixed(0)}%
                    </span>
                  </div>
                  <div className="flex justify-between text-[11px] text-muted-foreground">
                    <span>
                      VRAM: {gpu.memoryUsedMb.toFixed(0)} /{" "}
                      {gpu.memoryTotalMb.toFixed(0)} MB
                    </span>
                    <span>
                      {gpu.temperatureGpu.toFixed(0)}°C •{" "}
                      {gpu.powerDrawW.toFixed(0)}W
                    </span>
                  </div>
                </div>
              ))}
            </div>
          )}

          {/* Docker Quick Summary Card */}
          {metrics?.docker?.installed && (
            <div className="p-3.5 rounded-2xl bg-secondary/55 border border-border flex items-center justify-between shadow-2xs">
              <div className="space-y-0.5">
                <div className="flex items-center gap-1.5 font-semibold text-sky-400">
                  <Container className="w-3.5 h-3.5" />
                  Docker v{metrics.docker.version}
                </div>
                <div className="text-[11px] text-muted-foreground">
                  {metrics.docker.runningContainers} running •{" "}
                  {metrics.docker.stoppedContainers} stopped •{" "}
                  {metrics.docker.imagesCount} images
                </div>
              </div>
              {onOpenDockerPanel && (
                <button
                  type="button"
                  onClick={onOpenDockerPanel}
                  className="px-2.5 py-1 rounded-xl bg-sky-500/15 hover:bg-sky-500/25 text-sky-300 text-xs font-medium cursor-pointer"
                >
                  Manage
                </button>
              )}
            </div>
          )}

          {/* Searchable & Sortable Processes Table with Kill Action */}
          <div className="p-3.5 rounded-2xl bg-secondary/55 border border-border space-y-2.5 shadow-2xs">
            <div className="flex items-center justify-between gap-1">
              <span className="flex items-center gap-1.5 text-card-foreground font-semibold">
                <Layers className="w-3.5 h-3.5 text-amber-400" />
                Processes
              </span>
              <div className="flex items-center gap-1 text-[11px]">
                {(["cpu", "mem", "pid"] as const).map((s) => (
                  <button
                    key={s}
                    type="button"
                    onClick={() => setProcSort(s)}
                    className={`px-2 py-0.5 rounded-md uppercase cursor-pointer ${
                      procSort === s
                        ? "bg-primary/20 text-primary font-semibold"
                        : "text-muted-foreground hover:text-foreground"
                    }`}
                  >
                    {s}
                  </button>
                ))}
              </div>
            </div>

            <div className="relative">
              <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                value={procSearch}
                onChange={(e) => setProcSearch(e.target.value)}
                placeholder="Search PID, command, user..."
                className="w-full h-8 pl-8 pr-2.5 rounded-xl bg-input border border-border text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
              />
            </div>

            <div className="space-y-1 text-xs max-h-60 overflow-y-auto">
              <div className="grid grid-cols-12 text-[11px] font-medium text-muted-foreground pb-1 border-b border-border">
                <span className="col-span-2">PID</span>
                <span className="col-span-5">CMD</span>
                <span className="col-span-2 text-right">CPU</span>
                <span className="col-span-2 text-right">MEM</span>
                <span className="col-span-1 text-right" />
              </div>
              {filteredProcesses.length > 0 ? (
                filteredProcesses.map((proc) => (
                  <div
                    key={proc.pid}
                    className="group grid grid-cols-12 items-center py-1 px-1 -mx-1 rounded-lg hover:bg-accent/60 transition-colors text-card-foreground"
                  >
                    <span className="col-span-2 text-muted-foreground truncate">
                      {proc.pid}
                    </span>
                    <span
                      className="col-span-5 truncate font-medium"
                      title={`${proc.user ? `[${proc.user}] ` : ""}${proc.args || proc.command}`}
                    >
                      {proc.command}
                    </span>
                    <span className="col-span-2 text-right text-primary font-semibold">
                      {proc.cpuPct.toFixed(1)}%
                    </span>
                    <span className="col-span-2 text-right text-muted-foreground">
                      {proc.memPct.toFixed(1)}%
                    </span>
                    <div className="col-span-1 flex justify-end">
                      <button
                        type="button"
                        disabled={killingPid === proc.pid}
                        onClick={() => handleKillProcess(proc.pid)}
                        title={`Kill PID ${proc.pid}`}
                        className="opacity-0 group-hover:opacity-100 p-1 rounded-md hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                      >
                        <Skull className="w-3 h-3" />
                      </button>
                    </div>
                  </div>
                ))
              ) : (
                <div className="py-3 text-center text-muted-foreground text-xs">
                  {metrics
                    ? "No matching processes"
                    : "Waiting for telemetry snapshot..."}
                </div>
              )}
            </div>
          </div>
        </div>
      </div>
    </aside>
  );
};
