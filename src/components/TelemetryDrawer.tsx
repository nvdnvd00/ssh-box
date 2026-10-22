import React from "react";
import {
  Activity,
  ArrowDownRight,
  ArrowUpRight,
  Cpu,
  HardDrive,
  Layers,
  MemoryStick,
  X,
} from "lucide-react";
import { ServerMetrics } from "../types";

interface TelemetryDrawerProps {
  isOpen: boolean;
  hostLabel: string;
  metrics: ServerMetrics | null;
  cpuHistory: number[];
  memHistory: number[];
  rxHistory: number[];
  txHistory: number[];
  onClose: () => void;
}

const Sparkline: React.FC<{
  data: number[];
  maxVal?: number;
  stroke: string;
  fill: string;
}> = ({ data, maxVal, stroke, fill }) => {
  const width = 220;
  const height = 42;
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
      className="w-full h-11 overflow-visible"
      preserveAspectRatio="none"
    >
      <path d={areaPath} fill={fill} />
      <path
        d={linePath}
        fill="none"
        stroke={stroke}
        strokeWidth="1.75"
        strokeLinecap="round"
        strokeLinejoin="round"
      />
    </svg>
  );
};

export const TelemetryDrawer: React.FC<TelemetryDrawerProps> = ({
  isOpen,
  hostLabel,
  metrics,
  cpuHistory,
  memHistory,
  rxHistory,
  txHistory,
  onClose,
}) => {
  if (!isOpen) return null;

  const memPct =
    metrics && metrics.memTotalMb > 0
      ? Math.min(100, (metrics.memUsedMb / metrics.memTotalMb) * 100)
      : 0;

  return (
    <aside className="w-72 shrink-0 h-full bg-[#0d1119]/95 border-l border-slate-800/80 flex flex-col select-none overflow-y-auto">
      {/* Header */}
      <div className="px-3.5 py-2.5 border-b border-slate-800/80 flex items-center justify-between">
        <div className="flex items-center gap-2 min-w-0">
          <Activity className="w-4 h-4 text-cyan-400 shrink-0" />
          <div className="min-w-0">
            <div className="text-xs font-semibold text-slate-100 truncate">
              System Monitor
            </div>
            <div className="text-[10px] text-slate-400 font-mono truncate">
              {hostLabel}
            </div>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          title="Close Monitor (Cmd+I)"
          className="p-1 rounded hover:bg-slate-800 text-slate-400 hover:text-slate-200 cursor-pointer"
        >
          <X className="w-3.5 h-3.5" />
        </button>
      </div>

      <div className="p-3 space-y-3 text-xs">
        {/* CPU Card */}
        <div className="p-3 rounded-lg bg-[#111723] border border-slate-800/90 space-y-2">
          <div className="flex items-center justify-between">
            <span className="flex items-center gap-1.5 text-slate-300 font-medium">
              <Cpu className="w-3.5 h-3.5 text-cyan-400" />
              CPU Load
            </span>
            <div className="text-right font-mono">
              <span className="text-sm font-semibold text-cyan-300">
                {metrics ? `${metrics.cpuPercent.toFixed(1)}%` : "0.0%"}
              </span>
              <span className="ml-2 text-[10px] text-slate-400">
                load {metrics ? metrics.loadOne.toFixed(2) : "0.00"}
              </span>
            </div>
          </div>
          <Sparkline
            data={cpuHistory}
            maxVal={100}
            stroke="#22d3ee"
            fill="rgba(34, 211, 238, 0.14)"
          />
        </div>

        {/* Memory & Swap Card */}
        <div className="p-3 rounded-lg bg-[#111723] border border-slate-800/90 space-y-2">
          <div className="flex items-center justify-between">
            <span className="flex items-center gap-1.5 text-slate-300 font-medium">
              <MemoryStick className="w-3.5 h-3.5 text-violet-400" />
              Memory & Swap
            </span>
            <span className="font-mono text-violet-300 font-semibold">
              {memPct.toFixed(0)}%
            </span>
          </div>
          <div className="flex justify-between text-[11px] text-slate-400 font-mono">
            <span>
              RAM: {metrics ? metrics.memUsedMb : 0} /{" "}
              {metrics ? metrics.memTotalMb : 0} MB
            </span>
            <span>
              Swap: {metrics ? metrics.swapUsedMb : 0}/
              {metrics ? metrics.swapTotalMb : 0}M
            </span>
          </div>
          <Sparkline
            data={memHistory}
            maxVal={100}
            stroke="#a78bfa"
            fill="rgba(167, 139, 250, 0.14)"
          />
        </div>

        {/* Network RX / TX Card */}
        <div className="p-3 rounded-lg bg-[#111723] border border-slate-800/90 space-y-2">
          <div className="flex items-center justify-between">
            <span className="text-slate-300 font-medium">Net Throughput</span>
            <div className="flex items-center gap-2 font-mono text-[11px]">
              <span className="inline-flex items-center text-emerald-400">
                <ArrowDownRight className="w-3 h-3 mr-0.5" />
                {metrics ? metrics.netRxKbps.toFixed(1) : "0.0"} KB/s
              </span>
              <span className="inline-flex items-center text-amber-400">
                <ArrowUpRight className="w-3 h-3 mr-0.5" />
                {metrics ? metrics.netTxKbps.toFixed(1) : "0.0"} KB/s
              </span>
            </div>
          </div>
          <div className="grid grid-cols-2 gap-2">
            <div>
              <div className="text-[10px] text-slate-500 mb-0.5 font-mono">
                RX Stream
              </div>
              <Sparkline
                data={rxHistory}
                stroke="#34d399"
                fill="rgba(52, 211, 153, 0.14)"
              />
            </div>
            <div>
              <div className="text-[10px] text-slate-500 mb-0.5 font-mono">
                TX Stream
              </div>
              <Sparkline
                data={txHistory}
                stroke="#fbbf24"
                fill="rgba(251, 191, 36, 0.14)"
              />
            </div>
          </div>
        </div>

        {/* Disk Usage Card */}
        <div className="p-3 rounded-lg bg-[#111723] border border-slate-800/90 space-y-2">
          <div className="flex items-center justify-between">
            <span className="flex items-center gap-1.5 text-slate-300 font-medium">
              <HardDrive className="w-3.5 h-3.5 text-sky-400" />
              Root Disk (/)
            </span>
            <span className="font-mono text-[11px] text-slate-300">
              {metrics ? metrics.diskUsedGb.toFixed(1) : "0.0"} /{" "}
              {metrics ? metrics.diskTotalGb.toFixed(1) : "0.0"} GB (
              {metrics ? metrics.diskPercent.toFixed(0) : "0"}%)
            </span>
          </div>
          <div className="w-full h-2 rounded-full bg-slate-800 overflow-hidden">
            <div
              className="h-full bg-gradient-to-r from-cyan-500 to-sky-400 transition-all duration-300"
              style={{
                width: `${Math.min(100, Math.max(0, metrics?.diskPercent ?? 0))}%`,
              }}
            />
          </div>
        </div>

        {/* Top Processes Table */}
        <div className="p-3 rounded-lg bg-[#111723] border border-slate-800/90 space-y-2">
          <div className="flex items-center justify-between">
            <span className="flex items-center gap-1.5 text-slate-300 font-medium">
              <Layers className="w-3.5 h-3.5 text-amber-400" />
              Top Processes
            </span>
            <span className="text-[10px] text-slate-500 font-mono">by %CPU</span>
          </div>

          <div className="space-y-1 font-mono text-[11px]">
            <div className="grid grid-cols-12 text-[10px] text-slate-500 pb-1 border-b border-slate-800">
              <span className="col-span-3">PID</span>
              <span className="col-span-5">COMMAND</span>
              <span className="col-span-2 text-right">CPU</span>
              <span className="col-span-2 text-right">MEM</span>
            </div>
            {metrics && metrics.topProcesses.length > 0 ? (
              metrics.topProcesses.map((proc) => (
                <div
                  key={proc.pid}
                  className="grid grid-cols-12 items-center py-0.5 text-slate-300"
                >
                  <span className="col-span-3 text-slate-500">{proc.pid}</span>
                  <span className="col-span-5 truncate" title={proc.command}>
                    {proc.command}
                  </span>
                  <span className="col-span-2 text-right text-cyan-300">
                    {proc.cpuPct.toFixed(1)}%
                  </span>
                  <span className="col-span-2 text-right text-slate-400">
                    {proc.memPct.toFixed(1)}%
                  </span>
                </div>
              ))
            ) : (
              <div className="py-3 text-center text-slate-500 text-[11px]">
                Waiting for telemetry snapshot...
              </div>
            )}
          </div>
        </div>
      </div>
    </aside>
  );
};
