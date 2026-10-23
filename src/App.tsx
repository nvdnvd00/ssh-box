import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import {
  Activity,
  Code2,
  Columns,
  Container,
  Globe,
  HardDrive,
  KeyRound,
  Laptop,
  LayoutDashboard,
  PanelLeft,
  Plus,
  Rows,
  Server,
  Settings,
  Terminal as TerminalIcon,
  UserCheck,
  Waypoints,
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
  ResourcesStoreData,
  ServerMetrics,
  splitPaneInTree,
  TabResources,
  updateLeafInTree,
  updateSplitRatioInTree,
  WorkspaceTab,
} from "./types";
import {
  applyAppThemeToRoot,
  AppSettingsConfig,
  loadAppSettings,
  loadThemeConfig,
  saveAppSettings,
  saveThemeConfig,
  ThemeConfig,
} from "./themes";
import { SplitWorkspace } from "./components/SplitWorkspace";
import { TelemetryDrawer } from "./components/TelemetryDrawer";
import { ResourcesWorkspace } from "./components/ResourcesWorkspace";
import { DashboardWorkspace } from "./components/DashboardWorkspace";
import { SettingsTabId, SettingsWorkspace } from "./components/SettingsWorkspace";

type WorkspaceViewMode = "dashboard" | "resources" | "terminal" | "settings";

const EMPTY_RESOURCES: ResourcesStoreData = {
  groups: [],
  tags: [],
  identities: [],
  sshKeys: [],
  proxyProfiles: [],
  portForwardingRules: [],
  snippetPackages: [],
  snippets: [],
  snippetHistory: [],
};

export default function App() {
  const [themeConfig, setThemeConfig] = useState<ThemeConfig>(() => {
    const initial = loadThemeConfig();
    applyAppThemeToRoot(initial);
    return initial;
  });
  const [appSettings, setAppSettings] = useState<AppSettingsConfig>(() =>
    loadAppSettings(),
  );

  const [hosts, setHosts] = useState<HostEntry[]>([]);
  const [resources, setResources] =
    useState<ResourcesStoreData>(EMPTY_RESOURCES);

  // Resource & Dashboard are the core default experience; Terminal is opened on-demand
  const [workspaceView, setWorkspaceView] =
    useState<WorkspaceViewMode>("dashboard");
  const [resourceSubTab, setResourceSubTab] =
    useState<TabResources>("machines");
  const [settingsSection, setSettingsSection] =
    useState<SettingsTabId>("terminal");
  const [inspectedHostId, setInspectedHostId] = useState<string | null>(null);
  const [openCreateMachineSignal, setOpenCreateMachineSignal] = useState(0);

  // Terminal tabs start empty and are spawned only when SSH-ing into a machine or opening a shell
  const [tabs, setTabs] = useState<WorkspaceTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string>("");
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(() =>
    typeof window !== "undefined" ? window.innerWidth >= 768 : true,
  );
  const [telemetryDrawerOpen, setTelemetryDrawerOpen] = useState(false);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const isMacDesktop = useMemo(() => {
    if (typeof navigator === "undefined") return false;
    const ua = navigator.userAgent || "";
    const isIos =
      /iPhone|iPad|iPod/i.test(ua) ||
      (/Mac/i.test(navigator.platform) && (navigator.maxTouchPoints ?? 0) > 1);
    return /Mac/i.test(navigator.platform) && !isIos;
  }, []);

  useEffect(() => {
    const win = getCurrentWindow();
    let timer1: ReturnType<typeof setTimeout> | undefined;
    let timer2: ReturnType<typeof setTimeout> | undefined;

    const checkFullscreen = () => {
      win
        .isFullscreen()
        .then((fs) => setIsFullscreen(fs))
        .catch(() => undefined);
    };

    const handleResize = () => {
      checkFullscreen();
      clearTimeout(timer1);
      clearTimeout(timer2);
      timer1 = setTimeout(checkFullscreen, 150);
      timer2 = setTimeout(checkFullscreen, 600);
    };

    checkFullscreen();
    window.addEventListener("resize", handleResize);
    let unlisten: (() => void) | undefined;
    win
      .onResized(handleResize)
      .then((fn) => {
        unlisten = fn;
      })
      .catch(() => undefined);

    return () => {
      clearTimeout(timer1);
      clearTimeout(timer2);
      window.removeEventListener("resize", handleResize);
      unlisten?.();
    };
  }, []);

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

  useEffect(() => {
    applyAppThemeToRoot(themeConfig);
    saveThemeConfig(themeConfig);
  }, [themeConfig]);

  useEffect(() => {
    saveAppSettings(appSettings);
  }, [appSettings]);

  const handleUpdateTheme = useCallback(
    (updater: (prev: ThemeConfig) => ThemeConfig) => {
      setThemeConfig((prev) => updater(prev));
    },
    [],
  );

  const handleUpdateAppSettings = useCallback(
    (updater: (prev: AppSettingsConfig) => AppSettingsConfig) => {
      setAppSettings((prev) => updater(prev));
    },
    [],
  );

  const loadHosts = useCallback(() => {
    invoke<HostEntry[]>("list_hosts")
      .then((list) => setHosts(list))
      .catch(() => undefined);
  }, []);

  const loadResources = useCallback(() => {
    invoke<ResourcesStoreData>("get_resources")
      .then((data) => setResources(data))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    loadHosts();
    loadResources();
  }, [loadHosts, loadResources]);

  const activeTab = useMemo(
    () => tabs.find((t) => t.id === activeTabId) ?? tabs[0] ?? null,
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

  const activeTerminalHostId = useMemo(() => {
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

  // Determine which host (if any) needs live telemetry right now:
  // - If viewing Dashboard overview: poll "local" for the Localhost card
  // - If inspecting a specific machine in Dashboard Detail Monitor: poll `inspectedHostId`
  // - If inside Terminal view AND `telemetryDrawerOpen` is true: poll `activeTerminalHostId`
  const monitoredHostId = useMemo(() => {
    if (workspaceView === "dashboard") {
      return inspectedHostId ?? "local";
    }
    if (workspaceView === "terminal" && telemetryDrawerOpen && activeLeaf) {
      if (activeLeaf.kind === "ssh" && activeLeaf.status !== "connected") {
        return null;
      }
      return activeTerminalHostId;
    }
    return null;
  }, [
    workspaceView,
    inspectedHostId,
    telemetryDrawerOpen,
    activeLeaf,
    activeTerminalHostId,
  ]);

  useEffect(() => {
    if (!monitoredHostId) {
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
      hostId: monitoredHostId,
      onMetrics: channel,
    }).catch(() => undefined);

    return () => {
      invoke("unsubscribe_telemetry", { hostId: monitoredHostId }).catch(
        () => undefined,
      );
    };
  }, [monitoredHostId]);

  const handleOpenLocalTab = useCallback(() => {
    const leaf = createLocalLeaf("Local Shell");
    const newTab: WorkspaceTab = {
      id: `tab-${leaf.paneId}`,
      title: leaf.title,
      root: leaf,
      activePaneId: leaf.paneId,
    };
    setTabs((prev) => [...prev, newTab]);
    setActiveTabId(newTab.id);
    setWorkspaceView("terminal");
  }, []);

  const handleConnectHostTab = useCallback(
    (host: HostEntry, passwordOverride?: string) => {
      const leaf = createSshLeaf(host, passwordOverride);
      const newTab: WorkspaceTab = {
        id: `tab-${leaf.paneId}`,
        title: host.alias || `${host.user}@${host.hostname}`,
        root: leaf,
        activePaneId: leaf.paneId,
      };
      setTabs((prev) => [...prev, newTab]);
      setActiveTabId(newTab.id);
      if (appSettings.autoOpenMonitorOnSsh) {
        setTelemetryDrawerOpen(true);
      }
      setWorkspaceView("terminal");
    },
    [appSettings.autoOpenMonitorOnSsh],
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
          : createLocalLeaf("Local Shell");

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
      setWorkspaceView("terminal");
    },
    [activeTabId],
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
            setActiveTabId("");
            setTelemetryDrawerOpen(false);
            setWorkspaceView("dashboard");
            return [];
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
          setActiveTabId("");
          setTelemetryDrawerOpen(false);
          if (workspaceView === "terminal") {
            setWorkspaceView("dashboard");
          }
          return [];
        }
        if (tabId === activeTabId) {
          setActiveTabId(remaining[remaining.length - 1].id);
        }
        return remaining;
      });
    },
    [tabs, activeTabId, workspaceView],
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

  const handleRunSnippetInActivePane = useCallback(
    (code: string) => {
      if (!activeLeaf) return;
      const payload = code.endsWith("\n") ? code : `${code}\n`;
      const bytes = Array.from(new TextEncoder().encode(payload));
      invoke("write_pane_input", {
        paneId: activeLeaf.paneId,
        data: bytes,
      }).catch(() => undefined);
      setWorkspaceView("terminal");
    },
    [activeLeaf],
  );

  // Global keyboard shortcuts
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      const key = e.key.toLowerCase();

      if (key === "d" && !e.shiftKey) {
        e.preventDefault();
        if (activeLeaf && workspaceView === "terminal") {
          handleSplitPane(activeLeaf.paneId, "vertical", activeLeaf);
        }
      } else if (key === "d" && e.shiftKey) {
        e.preventDefault();
        if (activeLeaf && workspaceView === "terminal") {
          handleSplitPane(activeLeaf.paneId, "horizontal", activeLeaf);
        }
      } else if (key === "w") {
        e.preventDefault();
        if (activeLeaf && workspaceView === "terminal") {
          handleClosePane(activeLeaf.paneId);
        }
      } else if (key === "t") {
        e.preventDefault();
        handleOpenLocalTab();
      } else if (key === "i") {
        e.preventDefault();
        if (workspaceView === "terminal") {
          setTelemetryDrawerOpen((prev) => !prev);
        }
      } else if (key === ",") {
        e.preventDefault();
        setWorkspaceView((prev) =>
          prev === "settings" ? "dashboard" : "settings",
        );
      }
    };

    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [
    activeLeaf,
    workspaceView,
    handleSplitPane,
    handleClosePane,
    handleOpenLocalTab,
  ]);

  const currentTerminalMetrics = metricsMap[activeTerminalHostId] ?? null;
  const totalPanesInActiveTab = activeTab
    ? collectLeaves(activeTab.root).length
    : 1;

  const resourceItems: {
    id: TabResources;
    label: string;
    count?: number;
    icon: React.ReactNode;
  }[] = [
    {
      id: "machines",
      label: "Machine",
      count: hosts.length,
      icon: <Server className="w-4 h-4 text-emerald-400" />,
    },
    {
      id: "identities",
      label: "Identity",
      count: resources.identities.length,
      icon: <UserCheck className="w-4 h-4 text-violet-400" />,
    },
    {
      id: "keys",
      label: "SSH Key",
      count: resources.sshKeys.length,
      icon: <KeyRound className="w-4 h-4 text-amber-400" />,
    },
    {
      id: "connectionProxyProfiles",
      label: "Proxy",
      count: resources.proxyProfiles.length,
      icon: <Globe className="w-4 h-4 text-sky-400" />,
    },
  ];

  const toolboxItems: {
    id: TabResources;
    label: string;
    count?: number;
    icon: React.ReactNode;
  }[] = [
    {
      id: "sftp",
      label: "SFTP",
      icon: <HardDrive className="w-4 h-4 text-amber-400" />,
    },
    {
      id: "snippets",
      label: "Snippet",
      count: resources.snippets.length,
      icon: <Code2 className="w-4 h-4 text-pink-400" />,
    },
    {
      id: "docker",
      label: "Docker",
      icon: <Container className="w-4 h-4 text-cyan-400" />,
    },
    {
      id: "portForwarding",
      label: "Port Forwarding",
      count: resources.portForwardingRules.length,
      icon: <Waypoints className="w-4 h-4 text-indigo-400" />,
    },
  ];

  return (
    <div className="flex flex-col h-screen w-screen bg-background text-foreground overflow-hidden select-none transition-colors duration-200">
      {/* Top Titlebar (Adapts to macOS Overlay, Windows, iPadOS & Mobile) */}
      <header
        data-tauri-drag-region
        className={`h-11 shrink-0 ${
          isMacDesktop ? (isFullscreen ? "pl-[25px]" : "pl-20") : "pl-3"
        } pr-4 bg-header/95 backdrop-blur-md border-b border-border/80 flex items-center justify-between gap-3 z-30 transition-[padding] duration-200`}
      >
        <div className="flex items-center gap-2 min-w-0 flex-1">
          <button
            type="button"
            onClick={() => setSidebarOpen((p) => !p)}
            title="Toggle Navigation Sidebar"
            className={`p-2 rounded-xl transition-all duration-150 cursor-pointer ${
              sidebarOpen
                ? "bg-secondary text-primary border border-border/80 shadow-2xs"
                : "text-muted-foreground hover:bg-accent hover:text-foreground border border-transparent"
            }`}
          >
            <PanelLeft className="w-4 h-4" />
          </button>

          {/* When inside Terminal view, show active Terminal session tabs in the top bar */}
          {workspaceView === "terminal" && tabs.length > 0 && (
            <div className="flex items-center gap-1.5 min-w-0 overflow-x-auto no-scrollbar pl-1">
              {tabs.map((tab) => {
                const isTabActive = tab.id === activeTab?.id;
                const leaves = collectLeaves(tab.root);
                const primaryLeaf =
                  findLeafById(tab.root, tab.activePaneId) ?? leaves[0];
                const isSsh = primaryLeaf?.kind === "ssh";

                return (
                  <div
                    key={tab.id}
                    onClick={() => {
                      setActiveTabId(tab.id);
                      setWorkspaceView("terminal");
                    }}
                    className={`group relative flex items-center gap-2 px-3.5 py-1.5 rounded-xl text-xs font-medium cursor-pointer transition-all duration-150 max-w-[230px] ${
                      isTabActive
                        ? "bg-secondary text-foreground border border-primary/40 shadow-xs"
                        : "bg-card/50 text-muted-foreground hover:bg-accent/70 hover:text-foreground border border-transparent"
                    }`}
                  >
                    {isSsh ? (
                      <Server className="w-3.5 h-3.5 text-primary shrink-0" />
                    ) : (
                      <TerminalIcon className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                    )}
                    <span className="truncate">
                      {primaryLeaf?.title ?? tab.title}
                    </span>
                    {leaves.length > 1 && (
                      <span className="px-1.5 py-0.2 rounded-full bg-primary/15 text-[10px] font-mono text-primary font-semibold">
                        {leaves.length}
                      </span>
                    )}
                    <button
                      type="button"
                      onClick={(e) => handleCloseTab(tab.id, e)}
                      className="opacity-60 group-hover:opacity-100 hover:bg-rose-500/20 hover:text-rose-400 p-0.5 rounded-md transition-all cursor-pointer"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                );
              })}
            </div>
          )}
        </div>

        {/* Right Window Action Controls */}
        <div className="flex items-center gap-2 shrink-0">
          {workspaceView === "terminal" && activeLeaf && (
            <>
              <button
                type="button"
                onClick={() =>
                  handleSplitPane(activeLeaf.paneId, "vertical", activeLeaf)
                }
                title="Split Vertically (Cmd+D)"
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-secondary/70 hover:bg-accent text-xs font-medium text-foreground border border-border/70 transition-all cursor-pointer"
              >
                <Columns className="w-3.5 h-3.5 text-primary" />
                <span className="hidden md:inline">Split V</span>
              </button>

              <button
                type="button"
                onClick={() =>
                  handleSplitPane(activeLeaf.paneId, "horizontal", activeLeaf)
                }
                title="Split Horizontally (Cmd+Shift+D)"
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-secondary/70 hover:bg-accent text-xs font-medium text-foreground border border-border/70 transition-all cursor-pointer"
              >
                <Rows className="w-3.5 h-3.5 text-primary" />
                <span className="hidden md:inline">Split H</span>
              </button>

              <button
                type="button"
                onClick={() => {
                  if (activeLeaf.kind === "ssh" && activeLeaf.host) {
                    setInspectedHostId(activeLeaf.host.id);
                  }
                  setResourceSubTab("sftp");
                  setWorkspaceView("resources");
                }}
                title="Open SFTP File Manager"
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-secondary/70 hover:bg-accent text-xs font-medium text-foreground border border-border/70 transition-all cursor-pointer"
              >
                <HardDrive className="w-3.5 h-3.5 text-amber-400" />
                <span className="hidden md:inline">SFTP</span>
              </button>

              <button
                type="button"
                onClick={() => setTelemetryDrawerOpen((p) => !p)}
                title="Toggle Side Monitor Drawer (Cmd+I)"
                className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold transition-all duration-150 cursor-pointer border ${
                  telemetryDrawerOpen
                    ? "bg-primary/15 text-primary border-primary/40 shadow-2xs"
                    : "bg-secondary/70 text-muted-foreground hover:bg-accent hover:text-foreground border-border/70"
                }`}
              >
                <Activity className="w-3.5 h-3.5" />
                <span>Monitor</span>
              </button>
            </>
          )}
        </div>
      </header>

      {/* Main Workspace Area with Unified Left Sidebar */}
      <div className="relative flex flex-1 min-h-0 min-w-0 overflow-hidden">
        {sidebarOpen && (
          <div
            onClick={() => setSidebarOpen(false)}
            className="md:hidden fixed inset-0 z-20 bg-black/50 backdrop-blur-xs"
          />
        )}
        {sidebarOpen && (
          <aside className="fixed md:static inset-y-0 left-0 z-30 w-60 shrink-0 bg-header/95 md:bg-header/85 border-r border-border/80 flex flex-col justify-between p-3 overflow-y-auto select-none">
            <div className="space-y-5">
              {/* 1. Dashboard Top Item */}
              <div>
                <button
                  type="button"
                  onClick={() => {
                    setInspectedHostId(null);
                    setWorkspaceView("dashboard");
                  }}
                  className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-[13.5px] font-semibold transition-all duration-200 cursor-pointer ${
                    workspaceView === "dashboard" && !inspectedHostId
                      ? "bg-primary/15 text-foreground border border-primary/35 shadow-xs translate-x-0.5"
                      : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                  }`}
                >
                  <span className="flex items-center gap-3">
                    <span className="p-1.5 rounded-xl bg-card border border-border/70 shadow-2xs">
                      <LayoutDashboard className="w-4 h-4 text-emerald-400" />
                    </span>
                    <span>Dashboard</span>
                  </span>
                </button>
              </div>

              {/* 2. Resources Section */}
              <div className="space-y-1">
                <div className="px-3 pb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/75">
                  Resources
                </div>
                {resourceItems.map((item) => {
                  const active =
                    workspaceView === "resources" &&
                    resourceSubTab === item.id;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => {
                        setResourceSubTab(item.id);
                        setWorkspaceView("resources");
                      }}
                      className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-[13px] font-medium transition-all duration-200 cursor-pointer ${
                        active
                          ? "bg-primary/15 text-foreground border border-primary/35 shadow-xs translate-x-0.5"
                          : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                      }`}
                    >
                      <span className="flex items-center gap-3">
                        <span className="p-1.5 rounded-xl bg-card border border-border/70 shadow-2xs">
                          {item.icon}
                        </span>
                        <span>{item.label}</span>
                      </span>
                      {typeof item.count === "number" && item.count > 0 && (
                        <span className="px-2 py-0.5 rounded-full bg-secondary text-[11px] font-mono text-muted-foreground">
                          {item.count}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>

              {/* 3. Toolbox Section */}
              <div className="space-y-1">
                <div className="px-3 pb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/75">
                  Toolbox
                </div>
                {toolboxItems.map((item) => {
                  const active =
                    workspaceView === "resources" &&
                    resourceSubTab === item.id;
                  return (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => {
                        setResourceSubTab(item.id);
                        setWorkspaceView("resources");
                      }}
                      className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-[13px] font-medium transition-all duration-200 cursor-pointer ${
                        active
                          ? "bg-primary/15 text-foreground border border-primary/35 shadow-xs translate-x-0.5"
                          : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                      }`}
                    >
                      <span className="flex items-center gap-3">
                        <span className="p-1.5 rounded-xl bg-card border border-border/70 shadow-2xs">
                          {item.icon}
                        </span>
                        <span>{item.label}</span>
                      </span>
                      {typeof item.count === "number" && item.count > 0 && (
                        <span className="px-2 py-0.5 rounded-full bg-secondary text-[11px] font-mono text-muted-foreground">
                          {item.count}
                        </span>
                      )}
                    </button>
                  );
                })}
              </div>

              {/* 4. On-Demand Terminal Sessions Section */}
              <div className="space-y-1.5">
                <div className="px-3 flex items-center justify-between">
                  <span className="text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/75">
                    Terminal
                  </span>
                  <button
                    type="button"
                    onClick={handleOpenLocalTab}
                    title="Open Local Terminal Session (Cmd+T)"
                    className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent transition-colors cursor-pointer"
                  >
                    <Plus className="w-3.5 h-3.5" />
                  </button>
                </div>

                {tabs.length === 0 ? (
                  <div className="px-3 py-2 text-[11px] text-muted-foreground/70 italic">
                    Connect to a machine to open a terminal
                  </div>
                ) : (
                  <div className="space-y-1">
                    {tabs.map((tab) => {
                      const isTabActive =
                        workspaceView === "terminal" &&
                        tab.id === activeTab?.id;
                      const leaves = collectLeaves(tab.root);
                      const primaryLeaf =
                        findLeafById(tab.root, tab.activePaneId) ?? leaves[0];
                      const status = primaryLeaf?.status ?? "connecting";

                      return (
                        <div
                          key={tab.id}
                          onClick={() => {
                            setActiveTabId(tab.id);
                            setWorkspaceView("terminal");
                          }}
                          className={`group flex items-center justify-between px-3 py-2 rounded-xl text-xs font-medium cursor-pointer transition-all duration-150 ${
                            isTabActive
                              ? "bg-primary/15 text-foreground border border-primary/35 shadow-2xs"
                              : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                          }`}
                        >
                          <div className="flex items-center gap-2.5 min-w-0">
                            <span
                              className={`w-2 h-2 rounded-full shrink-0 ${
                                status === "connected"
                                  ? "bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.75)]"
                                  : status === "connecting"
                                    ? "bg-amber-400 animate-pulse"
                                    : "bg-rose-500"
                              }`}
                            />
                            <span className="truncate">
                              {primaryLeaf?.title ?? tab.title}
                            </span>
                          </div>
                          <button
                            type="button"
                            onClick={(e) => handleCloseTab(tab.id, e)}
                            className="opacity-0 group-hover:opacity-100 p-1 rounded-md hover:bg-rose-500/20 hover:text-rose-400 transition-all cursor-pointer"
                          >
                            <X className="w-3 h-3" />
                          </button>
                        </div>
                      );
                    })}
                  </div>
                )}
              </div>
            </div>

            {/* Bottom Settings Entry */}
            <div className="pt-3 border-t border-border/70 space-y-1.5">
              <button
                type="button"
                onClick={handleOpenLocalTab}
                className="w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-[13px] font-medium text-muted-foreground hover:bg-accent/60 hover:text-foreground transition-all cursor-pointer"
              >
                <span className="p-1.5 rounded-xl bg-card border border-border/70">
                  <Laptop className="w-4 h-4 text-emerald-400" />
                </span>
                <span>Local Terminal</span>
              </button>

              <button
                type="button"
                onClick={() => {
                  setSettingsSection("terminal");
                  setWorkspaceView("settings");
                }}
                className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-[13px] font-medium transition-all cursor-pointer ${
                  workspaceView === "settings"
                    ? "bg-primary/15 text-foreground border border-primary/35 shadow-xs"
                    : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                }`}
              >
                <span className="flex items-center gap-3">
                  <span className="p-1.5 rounded-xl bg-card border border-border/70">
                    <Settings className="w-4 h-4 text-primary" />
                  </span>
                  <span>Settings</span>
                </span>
                <kbd className="text-[10px] font-mono text-muted-foreground">
                  ⌘,
                </kbd>
              </button>
            </div>
          </aside>
        )}

        {/* Main Content Viewport */}
        <main className="flex-1 min-w-0 min-h-0 bg-background overflow-hidden relative">
          {/* Terminal View (kept mounted when tabs exist so xterm sessions stay alive) */}
          <div
            className={
              workspaceView === "terminal" && activeTab
                ? "h-full w-full"
                : "hidden"
            }
          >
            {activeTab && (
              <SplitWorkspace
                node={activeTab.root}
                activePaneId={activeTab.activePaneId}
                totalPanes={totalPanesInActiveTab}
                themeConfig={themeConfig}
                onFocusPane={handleFocusPane}
                onUpdateLeafStatus={handleUpdateLeafStatus}
                onTriggerReconnect={handleTriggerReconnect}
                onSplitPane={handleSplitPane}
                onClosePane={handleClosePane}
                onResizeSplit={handleResizeSplit}
              />
            )}
          </div>

          {/* Dashboard & Full-Screen Machine Monitor Detail View */}
          {workspaceView === "dashboard" && (
            <DashboardWorkspace
              hosts={hosts}
              resources={resources}
              metricsMap={metricsMap}
              selectedHostId={inspectedHostId}
              appSettings={appSettings}
              onSelectHost={(hostId) => setInspectedHostId(hostId)}
              onConnectHostTerminal={handleConnectHostTab}
              onOpenLocalTerminal={handleOpenLocalTab}
              onOpenAddMachine={() => {
                setResourceSubTab("machines");
                setWorkspaceView("resources");
                setOpenCreateMachineSignal((c) => c + 1);
              }}
              onOpenDockerManager={(hostId) => {
                setInspectedHostId(hostId);
                setResourceSubTab("docker");
                setWorkspaceView("resources");
              }}
              onOpenSftpManager={(hostId) => {
                setInspectedHostId(hostId);
                setResourceSubTab("sftp");
                setWorkspaceView("resources");
              }}
            />
          )}

          {/* Resources & Toolbox View */}
          {workspaceView === "resources" && (
            <ResourcesWorkspace
              hosts={hosts}
              resources={resources}
              activeHostId={inspectedHostId || activeTerminalHostId}
              initialTab={resourceSubTab}
              hideSidebar={true}
              openCreateMachineSignal={openCreateMachineSignal}
              onRefreshHosts={() => {
                loadHosts();
                loadResources();
              }}
              onUpdateResources={setResources}
              onSaveHost={handleSaveHost}
              onDeleteHost={handleDeleteHost}
              onConnectHost={handleConnectHostTab}
              onInspectMachine={(hostId) => {
                setInspectedHostId(hostId);
                setWorkspaceView("dashboard");
              }}
              onOpenLocalTerminal={handleOpenLocalTab}
              onRunSnippetInActivePane={handleRunSnippetInActivePane}
            />
          )}

          {/* Dedicated Settings View */}
          {workspaceView === "settings" && (
            <SettingsWorkspace
              themeConfig={themeConfig}
              appSettings={appSettings}
              resources={resources}
              initialSection={settingsSection}
              onChangeTheme={handleUpdateTheme}
              onChangeAppSettings={handleUpdateAppSettings}
              onUpdateResources={setResources}
            />
          )}
        </main>

        {/* On-Demand Side Monitor Drawer (Only when inside Terminal view and explicitly toggled open) */}
        {workspaceView === "terminal" && (
          <TelemetryDrawer
            isOpen={telemetryDrawerOpen}
            hostId={activeTerminalHostId}
            hostLabel={activeTelemetryLabel}
            accentColor={themeConfig.accentColor}
            metrics={currentTerminalMetrics}
            cpuHistory={cpuHistoryMap[activeTerminalHostId] ?? []}
            memHistory={memHistoryMap[activeTerminalHostId] ?? []}
            rxHistory={rxHistoryMap[activeTerminalHostId] ?? []}
            txHistory={txHistoryMap[activeTerminalHostId] ?? []}
            onClose={() => setTelemetryDrawerOpen(false)}
            onOpenDockerPanel={() => {
              setResourceSubTab("docker");
              setWorkspaceView("resources");
            }}
          />
        )}
      </div>
    </div>
  );
}
