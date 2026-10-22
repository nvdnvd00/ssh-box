import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import {
  Activity,
  ArrowDownRight,
  ArrowUpRight,
  Columns,
  Cpu,
  HardDrive,
  Laptop,
  MemoryStick,
  PanelLeft,
  Plus,
  Rows,
  Server,
  Terminal as TerminalIcon,
  X,
} from "lucide-react";
import {
  collectLeaves,
  createLocalLeaf,
  createSshLeaf,
  findLeafById,
  HostEntry,
  LeafPaneNode,
  PaneStatus,
  removePaneFromTree,
  ServerMetrics,
  splitPaneInTree,
  updateLeafInTree,
  updateSplitRatioInTree,
  WorkspaceTab,
} from "./types";
import { HostSidebar } from "./components/HostSidebar";
import { SplitWorkspace } from "./components/SplitWorkspace";
import { TelemetryDrawer } from "./components/TelemetryDrawer";

function createInitialTab(): WorkspaceTab {
  const leaf = createLocalLeaf("Local");
  return {
    id: `tab-${leaf.paneId}`,
    title: leaf.title,
    root: leaf,
    activePaneId: leaf.paneId,
  };
}

export default function App() {
  const [hosts, setHosts] = useState<HostEntry[]>([]);
  const [tabs, setTabs] = useState<WorkspaceTab[]>(() => [createInitialTab()]);
  const [activeTabId, setActiveTabId] = useState<string>(
    () => tabs[0]?.id ?? "",
  );
  const [sidebarOpen, setSidebarOpen] = useState(true);
  const [telemetryOpen, setTelemetryOpen] = useState(true);

  const [metricsMap, setMetricsMap] = useState<Record<string, ServerMetrics>>(
    {},
  );
  const [cpuHistoryMap, setCpuHistoryMap] = useState<Record<string, number[]>>(
    {},
  );
  const [memHistoryMap, setMemHistoryMap] = useState<Record<string, number[]>>(
    {},
  );
  const [rxHistoryMap, setRxHistoryMap] = useState<Record<string, number[]>>(
    {},
  );
  const [txHistoryMap, setTxHistoryMap] = useState<Record<string, number[]>>(
    {},
  );

  const loadHosts = useCallback(() => {
    invoke<HostEntry[]>("list_hosts")
      .then((list) => setHosts(list))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    loadHosts();
  }, [loadHosts]);

  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? tabs[0],
    [tabs, activeTabId],
  );

  const activeLeaf = useMemo(() => {
    if (!activeTab) return null;
    return (
      findLeafById(activeTab.root, activeTab.activePaneId) ??
      collectLeaves(activeTab.root)[0] ??
      null
    );
  }, [activeTab]);

  const activeTelemetryHostId = useMemo(() => {
    if (!activeLeaf) return "local";
    if (activeLeaf.kind === "ssh" && activeLeaf.host) {
      return activeLeaf.host.id;
    }
    return "local";
  }, [activeLeaf]);

  const activeTelemetryLabel = useMemo(() => {
    if (activeLeaf?.kind === "ssh" && activeLeaf.host) {
      return `${activeLeaf.host.user}@${activeLeaf.host.alias || activeLeaf.host.hostname}`;
    }
    return "localhost";
  }, [activeLeaf]);

  // Subscribe to 2-second Telemetry stream for active host (or local machine)
  useEffect(() => {
    const targetId = activeTelemetryHostId;
    if (
      targetId !== "local" &&
      activeLeaf?.kind === "ssh" &&
      activeLeaf.status !== "connected"
    ) {
      return;
    }

    const channel = new Channel<ServerMetrics>();
    channel.onmessage = (m) => {
      setMetricsMap((prev) => ({ ...prev, [m.hostId]: m }));
      setCpuHistoryMap((prev) => ({
        ...prev,
        [m.hostId]: [...(prev[m.hostId] ?? []).slice(-29), m.cpuPercent],
      }));
      const memPct =
        m.memTotalMb > 0 ? (m.memUsedMb / m.memTotalMb) * 100 : 0;
      setMemHistoryMap((prev) => ({
        ...prev,
        [m.hostId]: [...(prev[m.hostId] ?? []).slice(-29), memPct],
      }));
      setRxHistoryMap((prev) => ({
        ...prev,
        [m.hostId]: [...(prev[m.hostId] ?? []).slice(-29), m.netRxKbps],
      }));
      setTxHistoryMap((prev) => ({
        ...prev,
        [m.hostId]: [...(prev[m.hostId] ?? []).slice(-29), m.netTxKbps],
      }));
    };

    invoke("subscribe_telemetry", {
      hostId: targetId,
      onMetrics: channel,
    }).catch(() => undefined);
  }, [activeTelemetryHostId, activeLeaf?.kind, activeLeaf?.status]);

  const handleOpenLocalTab = useCallback(() => {
    const newTab = createInitialTab();
    setTabs((prev) => [...prev, newTab]);
    setActiveTabId(newTab.id);
  }, []);

  const handleConnectHostTab = useCallback(
    (host: HostEntry, passwordOverride?: string) => {
      const leaf = createSshLeaf(host, passwordOverride);
      const newTab: WorkspaceTab = {
        id: `tab-${leaf.paneId}`,
        title: leaf.title,
        root: leaf,
        activePaneId: leaf.paneId,
      };
      setTabs((prev) => [...prev, newTab]);
      setActiveTabId(newTab.id);
    },
    [],
  );

  const handleSplitPane = useCallback(
    (
      paneId: string,
      direction: "horizontal" | "vertical",
      sourceLeaf: LeafPaneNode,
    ) => {
      const newLeaf =
        sourceLeaf.kind === "ssh" && sourceLeaf.host
          ? createSshLeaf(sourceLeaf.host, sourceLeaf.passwordOverride)
          : createLocalLeaf("Local");

      setTabs((prev) =>
        prev.map((tab) => {
          if (tab.id !== activeTabId) return tab;
          return {
            ...tab,
            root: splitPaneInTree(tab.root, paneId, direction, newLeaf),
            activePaneId: newLeaf.paneId,
          };
        }),
      );
    },
    [activeTabId],
  );

  const handleSplitWithHost = useCallback(
    (host: HostEntry) => {
      if (!activeTab || !activeLeaf) {
        handleConnectHostTab(host);
        return;
      }
      const newLeaf = createSshLeaf(host);
      setTabs((prev) =>
        prev.map((tab) => {
          if (tab.id !== activeTab.id) return tab;
          return {
            ...tab,
            root: splitPaneInTree(
              tab.root,
              activeLeaf.paneId,
              "vertical",
              newLeaf,
            ),
            activePaneId: newLeaf.paneId,
          };
        }),
      );
    },
    [activeTab, activeLeaf, handleConnectHostTab],
  );

  const handleClosePane = useCallback(
    (paneId: string) => {
      invoke("close_pane", { paneId }).catch(() => undefined);

      setTabs((prev) => {
        const currentTab = prev.find((t) => t.id === activeTabId);
        if (!currentTab) return prev;

        const nextRoot = removePaneFromTree(currentTab.root, paneId);
        if (!nextRoot) {
          const remainingTabs = prev.filter((t) => t.id !== activeTabId);
          if (remainingTabs.length === 0) {
            const fallback = createInitialTab();
            setActiveTabId(fallback.id);
            return [fallback];
          }
          setActiveTabId(remainingTabs[remainingTabs.length - 1].id);
          return remainingTabs;
        }

        const remainingLeaves = collectLeaves(nextRoot);
        const nextActivePaneId = remainingLeaves.some(
          (l) => l.paneId === currentTab.activePaneId,
        )
          ? currentTab.activePaneId
          : (remainingLeaves[0]?.paneId ?? "");

        return prev.map((t) =>
          t.id === activeTabId
            ? {
                ...t,
                root: nextRoot,
                activePaneId: nextActivePaneId,
              }
            : t,
        );
      });
    },
    [activeTabId],
  );

  const handleCloseTab = useCallback(
    (tabId: string, e: React.MouseEvent) => {
      e.stopPropagation();
      const targetTab = tabs.find((t) => t.id === tabId);
      if (targetTab) {
        for (const leaf of collectLeaves(targetTab.root)) {
          invoke("close_pane", { paneId: leaf.paneId }).catch(() => undefined);
        }
      }
      setTabs((prev) => {
        const remaining = prev.filter((t) => t.id !== tabId);
        if (remaining.length === 0) {
          const fallback = createInitialTab();
          setActiveTabId(fallback.id);
          return [fallback];
        }
        if (tabId === activeTabId) {
          setActiveTabId(remaining[remaining.length - 1].id);
        }
        return remaining;
      });
    },
    [tabs, activeTabId],
  );

  const handleFocusPane = useCallback(
    (paneId: string) => {
      setTabs((prev) =>
        prev.map((t) =>
          t.id === activeTabId ? { ...t, activePaneId: paneId } : t,
        ),
      );
    },
    [activeTabId],
  );

  const handleUpdateLeafStatus = useCallback(
    (
      paneId: string,
      status: PaneStatus,
      extra?: {
        errorMessage?: string;
        fingerprint?: string;
        passwordOverride?: string;
      },
    ) => {
      setTabs((prev) =>
        prev.map((t) => ({
          ...t,
          root: updateLeafInTree(t.root, paneId, (leaf) => ({
            ...leaf,
            status,
            errorMessage:
              extra && "errorMessage" in extra
                ? extra.errorMessage
                : leaf.errorMessage,
            fingerprint: extra?.fingerprint ?? leaf.fingerprint,
            passwordOverride: extra?.passwordOverride ?? leaf.passwordOverride,
          })),
        })),
      );
    },
    [],
  );

  const handleTriggerReconnect = useCallback(
    (paneId: string, passwordOverride?: string) => {
      setTabs((prev) =>
        prev.map((t) => ({
          ...t,
          root: updateLeafInTree(t.root, paneId, (leaf) => ({
            ...leaf,
            status: "connecting",
            errorMessage: undefined,
            passwordOverride: passwordOverride ?? leaf.passwordOverride,
            reconnectNonce: leaf.reconnectNonce + 1,
          })),
        })),
      );
    },
    [],
  );

  const handleResizeSplit = useCallback(
    (splitId: string, ratio: number) => {
      setTabs((prev) =>
        prev.map((t) =>
          t.id === activeTabId
            ? { ...t, root: updateSplitRatioInTree(t.root, splitId, ratio) }
            : t,
        ),
      );
    },
    [activeTabId],
  );

  const handleSaveHost = useCallback(
    async (host: HostEntry, secret?: string) => {
      const updated = await invoke<HostEntry[]>("save_host", {
        host,
        secret: secret ?? null,
      });
      setHosts(updated);
    },
    [],
  );

  const handleDeleteHost = useCallback(async (hostId: string) => {
    const updated = await invoke<HostEntry[]>("delete_host", { hostId });
    setHosts(updated);
  }, []);

  // Global iTerm2-style keyboard shortcuts
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      const key = e.key.toLowerCase();

      if (key === "d" && !e.shiftKey) {
        e.preventDefault();
        if (activeLeaf) {
          handleSplitPane(activeLeaf.paneId, "vertical", activeLeaf);
        }
      } else if (key === "d" && e.shiftKey) {
        e.preventDefault();
        if (activeLeaf) {
          handleSplitPane(activeLeaf.paneId, "horizontal", activeLeaf);
        }
      } else if (key === "w") {
        e.preventDefault();
        if (activeLeaf) {
          handleClosePane(activeLeaf.paneId);
        }
      } else if (key === "t") {
        e.preventDefault();
        handleOpenLocalTab();
      } else if (key === "i") {
        e.preventDefault();
        setTelemetryOpen((prev) => !prev);
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [activeLeaf, handleSplitPane, handleClosePane, handleOpenLocalTab]);

  const currentMetrics = metricsMap[activeTelemetryHostId] ?? null;
  const totalPanesInActiveTab = activeTab
    ? collectLeaves(activeTab.root).length
    : 1;

  return (
    <div className="flex flex-col h-screen w-screen bg-[#0b0e14] text-slate-100 overflow-hidden select-none">
      {/* Top macOS Overlay Titlebar + Unified Tab Bar */}
      <header
        data-tauri-drag-region
        className="h-10 shrink-0 pl-20 pr-3 bg-[#090c12]/95 border-b border-slate-800/80 flex items-center justify-between gap-2"
      >
        <div className="flex items-center gap-1.5 min-w-0 flex-1 overflow-x-auto no-scrollbar">
          <button
            type="button"
            onClick={() => setSidebarOpen((p) => !p)}
            title="Toggle Host Sidebar"
            className={`p-1.5 rounded-md transition-colors cursor-pointer ${
              sidebarOpen
                ? "bg-slate-800 text-cyan-400"
                : "text-slate-400 hover:bg-slate-800/60 hover:text-slate-200"
            }`}
          >
            <PanelLeft className="w-3.5 h-3.5" />
          </button>

          {/* Workspace Tabs */}
          <div className="flex items-center gap-1 min-w-0 overflow-x-auto">
            {tabs.map((tab) => {
              const isTabActive = tab.id === activeTab?.id;
              const leaves = collectLeaves(tab.root);
              const primaryLeaf =
                findLeafById(tab.root, tab.activePaneId) ?? leaves[0];
              const isSsh = primaryLeaf?.kind === "ssh";

              return (
                <div
                  key={tab.id}
                  onClick={() => setActiveTabId(tab.id)}
                  className={`group flex items-center gap-2 px-3 py-1 rounded-md text-xs font-medium cursor-pointer transition-all max-w-[210px] ${
                    isTabActive
                      ? "bg-[#141b29] text-slate-100 border border-cyan-500/35 shadow-xs"
                      : "bg-[#0e131d]/70 text-slate-400 hover:bg-[#131926] hover:text-slate-200 border border-transparent"
                  }`}
                >
                  {isSsh ? (
                    <Server className="w-3.5 h-3.5 text-cyan-400 shrink-0" />
                  ) : (
                    <TerminalIcon className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                  )}
                  <span className="truncate">
                    {primaryLeaf?.title ?? tab.title}
                  </span>
                  {leaves.length > 1 && (
                    <span className="px-1 py-0.2 rounded bg-slate-800 text-[10px] font-mono text-cyan-300">
                      {leaves.length}
                    </span>
                  )}
                  <button
                    type="button"
                    onClick={(e) => handleCloseTab(tab.id, e)}
                    className="opacity-60 group-hover:opacity-100 hover:text-rose-400 p-0.5 rounded cursor-pointer"
                  >
                    <X className="w-3 h-3" />
                  </button>
                </div>
              );
            })}

            <button
              type="button"
              onClick={handleOpenLocalTab}
              title="New Local Terminal Tab (Cmd+T)"
              className="p-1.5 rounded-md text-slate-400 hover:bg-slate-800 hover:text-slate-100 transition-colors cursor-pointer shrink-0"
            >
              <Plus className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        {/* Right Window Action Controls */}
        <div className="flex items-center gap-1 shrink-0">
          <button
            type="button"
            onClick={() =>
              activeLeaf &&
              handleSplitPane(activeLeaf.paneId, "vertical", activeLeaf)
            }
            title="Split Vertically (Cmd+D)"
            className="inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs text-slate-300 hover:bg-slate-800 hover:text-slate-100 transition-colors cursor-pointer"
          >
            <Columns className="w-3.5 h-3.5 text-cyan-400" />
            <span className="hidden md:inline text-[11px]">Split V</span>
          </button>

          <button
            type="button"
            onClick={() =>
              activeLeaf &&
              handleSplitPane(activeLeaf.paneId, "horizontal", activeLeaf)
            }
            title="Split Horizontally (Cmd+Shift+D)"
            className="inline-flex items-center gap-1 px-2 py-1 rounded-md text-xs text-slate-300 hover:bg-slate-800 hover:text-slate-100 transition-colors cursor-pointer"
          >
            <Rows className="w-3.5 h-3.5 text-cyan-400" />
            <span className="hidden md:inline text-[11px]">Split H</span>
          </button>

          <button
            type="button"
            onClick={() => setTelemetryOpen((p) => !p)}
            title="Toggle System Monitor (Cmd+I)"
            className={`inline-flex items-center gap-1.5 px-2.5 py-1 rounded-md text-xs font-medium transition-colors cursor-pointer ${
              telemetryOpen
                ? "bg-cyan-500/15 text-cyan-300 border border-cyan-500/30"
                : "text-slate-400 hover:bg-slate-800 hover:text-slate-200 border border-transparent"
            }`}
          >
            <Activity className="w-3.5 h-3.5" />
            <span className="hidden sm:inline text-[11px]">Monitor</span>
          </button>
        </div>
      </header>

      {/* Main Workspace Area */}
      <div className="flex flex-1 min-h-0 min-w-0 overflow-hidden">
        {sidebarOpen && (
          <HostSidebar
            hosts={hosts}
            activeHostId={activeLeaf?.host?.id}
            onConnectHost={handleConnectHostTab}
            onSplitWithHost={handleSplitWithHost}
            onOpenLocalTerminal={handleOpenLocalTab}
            onSaveHost={handleSaveHost}
            onDeleteHost={handleDeleteHost}
            onRefreshHosts={loadHosts}
          />
        )}

        <main className="flex-1 min-w-0 min-h-0 bg-[#0b0e14] overflow-hidden">
          {activeTab && (
            <SplitWorkspace
              node={activeTab.root}
              activePaneId={activeTab.activePaneId}
              totalPanes={totalPanesInActiveTab}
              onFocusPane={handleFocusPane}
              onUpdateLeafStatus={handleUpdateLeafStatus}
              onTriggerReconnect={handleTriggerReconnect}
              onSplitPane={handleSplitPane}
              onClosePane={handleClosePane}
              onResizeSplit={handleResizeSplit}
            />
          )}
        </main>

        <TelemetryDrawer
          isOpen={telemetryOpen}
          hostLabel={activeTelemetryLabel}
          metrics={currentMetrics}
          cpuHistory={cpuHistoryMap[activeTelemetryHostId] ?? []}
          memHistory={memHistoryMap[activeTelemetryHostId] ?? []}
          rxHistory={rxHistoryMap[activeTelemetryHostId] ?? []}
          txHistory={txHistoryMap[activeTelemetryHostId] ?? []}
          onClose={() => setTelemetryOpen(false)}
        />
      </div>

      {/* Bottom Status Bar */}
      <footer className="h-6 shrink-0 px-3 bg-[#080a0f] border-t border-slate-800/80 flex items-center justify-between text-[11px] text-slate-400 font-mono">
        <div className="flex items-center gap-3 min-w-0">
          <span className="inline-flex items-center gap-1.5 text-slate-200">
            {activeLeaf?.kind === "ssh" ? (
              <>
                <Server className="w-3 h-3 text-cyan-400" />
                <span>
                  {activeLeaf.host
                    ? `${activeLeaf.host.hostname}:${activeLeaf.host.port}`
                    : "SSH"}
                </span>
              </>
            ) : (
              <>
                <Laptop className="w-3 h-3 text-emerald-400" />
                <span>Local</span>
              </>
            )}
          </span>

          <span className="text-slate-600">|</span>

          <span className="inline-flex items-center gap-1">
            <Cpu className="w-3 h-3 text-cyan-400" />
            {currentMetrics
              ? `${currentMetrics.cpuPercent.toFixed(1)}%`
              : "0.0%"}
          </span>

          <span className="inline-flex items-center gap-1">
            <MemoryStick className="w-3 h-3 text-violet-400" />
            {currentMetrics
              ? `${currentMetrics.memUsedMb}/${currentMetrics.memTotalMb}MB`
              : "0/0MB"}
          </span>

          <span className="hidden sm:inline-flex items-center gap-1">
            <HardDrive className="w-3 h-3 text-sky-400" />
            {currentMetrics
              ? `${currentMetrics.diskPercent.toFixed(0)}%`
              : "0%"}
          </span>

          <span className="hidden md:inline-flex items-center gap-2">
            <span className="inline-flex items-center text-emerald-400">
              <ArrowDownRight className="w-3 h-3" />
              {currentMetrics
                ? `${currentMetrics.netRxKbps.toFixed(1)}K/s`
                : "0.0K/s"}
            </span>
            <span className="inline-flex items-center text-amber-400">
              <ArrowUpRight className="w-3 h-3" />
              {currentMetrics
                ? `${currentMetrics.netTxKbps.toFixed(1)}K/s`
                : "0.0K/s"}
            </span>
          </span>
        </div>

        <div className="hidden lg:flex items-center gap-3 text-[10px] text-slate-500">
          <span>⌘D Split V</span>
          <span>⇧⌘D Split H</span>
          <span>⌘W Close</span>
          <span>⌘T Tab</span>
          <span>⌘I Monitor</span>
        </div>
      </footer>
    </div>
  );
}
