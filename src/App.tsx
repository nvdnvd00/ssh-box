import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { check, Update } from "@tauri-apps/plugin-updater";
import { relaunch } from "@tauri-apps/plugin-process";
import {
  Activity,
  ArrowLeft,
  Code2,
  Container,
  Download,
  HardDrive,
  Home,
  KeyRound,
  LayoutDashboard,
  PanelLeft,
  Play,
  Plus,
  Radio,
  Search,
  Server,
  Settings,
  Terminal as TerminalIcon,
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
  SnippetItem,
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
import {
  extractSnippetVariables,
  getI18n,
  interpolateSnippetVariables,
} from "./i18n";
import { SplitWorkspace } from "./components/SplitWorkspace";
import { TelemetryDrawer } from "./components/TelemetryDrawer";
import { ResourcesWorkspace } from "./components/ResourcesWorkspace";
import { DashboardWorkspace } from "./components/DashboardWorkspace";
import {
  getSettingsNavGroups,
  SettingsTabId,
  SettingsWorkspace,
} from "./components/SettingsWorkspace";
type WorkspaceViewMode = "dashboard" | "resources" | "terminal" | "settings";

const KEYCHAIN_TABS: TabResources[] = [
  "identities",
  "keys",
  "knownHosts",
  "connectionProxyProfiles",
];

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
  // Remembers which Home screen to return to when leaving a terminal tab
  const lastHomeViewRef = useRef<WorkspaceViewMode>("dashboard");
  // ...and which screen the Settings "Back" button returns to
  const beforeSettingsViewRef = useRef<WorkspaceViewMode>("dashboard");
  useEffect(() => {
    if (workspaceView === "terminal") return;
    lastHomeViewRef.current = workspaceView;
    if (workspaceView !== "settings") beforeSettingsViewRef.current = workspaceView;
  }, [workspaceView]);
  const [resourceSubTab, setResourceSubTab] =
    useState<TabResources>("machines");
  const [settingsSection, setSettingsSection] =
    useState<SettingsTabId>("general");
  const [inspectedHostId, setInspectedHostId] = useState<string | null>(null);
  const [openCreateMachineSignal, setOpenCreateMachineSignal] = useState(0);

  // Terminal tabs start empty and are spawned only when SSH-ing into a machine or opening a shell
  const [tabs, setTabs] = useState<WorkspaceTab[]>([]);
  const [activeTabId, setActiveTabId] = useState<string>("");
  const [sidebarOpen, setSidebarOpen] = useState<boolean>(() =>
    typeof window !== "undefined" ? window.innerWidth >= 768 : true,
  );
  const [telemetryDrawerOpen, setTelemetryDrawerOpen] = useState(false);
  const [snippetsDrawerOpen, setSnippetsDrawerOpen] = useState(false);
  const [snippetDrawerSearch, setSnippetDrawerSearch] = useState("");
  const [broadcastMode, setBroadcastMode] = useState<"off" | "tab" | "all">(
    "off",
  );
  const [commandPaletteOpen, setCommandPaletteOpen] = useState(false);
  const [commandPaletteQuery, setCommandPaletteQuery] = useState("");
  const [pendingVarSnippet, setPendingVarSnippet] = useState<{
    name: string;
    code: string;
    vars: string[];
    values: Record<string, string>;
  } | null>(null);
  const [isFullscreen, setIsFullscreen] = useState(false);
  const [pendingUpdate, setPendingUpdate] = useState<Update | null>(null);
  const [isUpdating, setIsUpdating] = useState(false);
  const t = getI18n(appSettings.language);

  // Fails silently in dev (no endpoints) and on mobile (plugin not registered).
  useEffect(() => {
    check()
      .then(setPendingUpdate)
      .catch(() => undefined);
  }, []);

  const handleInstallUpdate = async () => {
    if (!pendingUpdate || !window.confirm(t.updateConfirm)) return;
    setIsUpdating(true);
    try {
      await pendingUpdate.downloadAndInstall();
      await relaunch();
    } catch (err) {
      setIsUpdating(false);
      window.alert(String(err));
    }
  };
  const { isMacDesktop, isMobileApp } = useMemo(() => {
    if (typeof navigator === "undefined") {
      return { isMacDesktop: false, isMobileApp: false };
    }
    const ua = navigator.userAgent || "";
    const isIos =
      /iPhone|iPad|iPod/i.test(ua) ||
      (/Mac/i.test(navigator.platform) && (navigator.maxTouchPoints ?? 0) > 1);
    return {
      isMacDesktop: /Mac/i.test(navigator.platform) && !isIos,
      // iOS/Android builds have no real PTY (only the embedded shell), so local tabs are disabled there
      isMobileApp: isIos || /Android/i.test(ua),
    };
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
  const openLocalTab = isMobileApp ? undefined : handleOpenLocalTab;

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
            setWorkspaceView(lastHomeViewRef.current);
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
            setWorkspaceView(lastHomeViewRef.current);
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

  const broadcastTargetPaneIds = useMemo(() => {
    if (broadcastMode === "off") return undefined;
    if (broadcastMode === "tab") {
      if (!activeTab) return undefined;
      return collectLeaves(activeTab.root).map((l) => l.paneId);
    }
    const allIds: string[] = [];
    for (const tab of tabs) {
      for (const l of collectLeaves(tab.root)) {
        allIds.push(l.paneId);
      }
    }
    return allIds;
  }, [broadcastMode, activeTab, tabs]);

  const sendCommandToTargetPanes = useCallback(
    (rawCode: string) => {
      const payload = rawCode.endsWith("\n") ? rawCode : `${rawCode}\n`;
      const bytes = Array.from(new TextEncoder().encode(payload));
      const targets =
        broadcastTargetPaneIds && broadcastTargetPaneIds.length > 0
          ? broadcastTargetPaneIds
          : activeLeaf
            ? [activeLeaf.paneId]
            : [];
      for (const paneId of targets) {
        invoke("write_pane_input", { paneId, data: bytes }).catch(
          () => undefined,
        );
      }
      setWorkspaceView("terminal");
    },
    [broadcastTargetPaneIds, activeLeaf],
  );

  const handleRunSnippetInActivePane = useCallback(
    (code: string, snippetName = "Snippet") => {
      const vars = extractSnippetVariables(code);
      if (vars.length > 0) {
        const initialValues: Record<string, string> = {};
        for (const v of vars) initialValues[v] = "";
        setPendingVarSnippet({
          name: snippetName,
          code,
          vars,
          values: initialValues,
        });
        return;
      }
      sendCommandToTargetPanes(code);
    },
    [sendCommandToTargetPanes],
  );

  // Global keyboard shortcuts
  useEffect(() => {
    const onKeyDown = (e: KeyboardEvent) => {
      if (!e.metaKey) return;
      const key = e.key.toLowerCase();

      if (key === "k") {
        e.preventDefault();
        setCommandPaletteOpen((prev) => !prev);
        setCommandPaletteQuery("");
        return;
      }

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
        openLocalTab?.();
      } else if (key === "i") {
        e.preventDefault();
        if (workspaceView === "terminal") {
          setTelemetryDrawerOpen((prev) => !prev);
        }
      } else if (key === ",") {
        e.preventDefault();
        setWorkspaceView((prev) =>
          prev === "settings" ? beforeSettingsViewRef.current : "settings",
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
    openLocalTab,
  ]);

  const currentTerminalMetrics = metricsMap[activeTerminalHostId] ?? null;

  const navItems: {
    id: TabResources;
    tabs?: TabResources[];
    label: string;
    count?: number;
    icon: React.ReactNode;
  }[] = [
    {
      id: "machines",
      label: t.navMachines,
      count: hosts.length,
      icon: <Server className="w-4 h-4 text-emerald-400" />,
    },
    {
      id: "identities",
      tabs: KEYCHAIN_TABS,
      label: t.navKeychain,
      count: resources.identities.length + resources.sshKeys.length,
      icon: <KeyRound className="w-4 h-4 text-amber-400" />,
    },
    {
      id: "snippets",
      label: t.navSnippets,
      count: resources.snippets.length,
      icon: <Code2 className="w-4 h-4 text-pink-400" />,
    },
    {
      id: "portForwarding",
      label: t.navPortForwarding,
      count: resources.portForwardingRules.length,
      icon: <Waypoints className="w-4 h-4 text-indigo-400" />,
    },
    {
      id: "sftp",
      label: t.navSftp,
      icon: <HardDrive className="w-4 h-4 text-amber-400" />,
    },
    {
      id: "docker",
      label: t.navDocker,
      icon: <Container className="w-4 h-4 text-cyan-400" />,
    },
  ];

  const keychainItems: { id: TabResources; label: string }[] = [
    { id: "identities", label: t.navIdentities },
    { id: "keys", label: t.navSshKeys },
    { id: "knownHosts", label: t.navKnownHosts },
    { id: "connectionProxyProfiles", label: t.navProxyProfiles },
  ];

  const toolBtnClass = (active: boolean) =>
    `inline-flex items-center p-2 rounded-xl border transition-all cursor-pointer ${
      active
        ? "bg-primary/15 text-primary border-primary/40"
        : "bg-secondary/70 text-muted-foreground hover:bg-accent hover:text-foreground border-border/70"
    }`;

  return (
    <div className="flex flex-col h-screen w-screen bg-background text-foreground overflow-hidden select-none transition-colors duration-200">
      {/* Top Tab Bar (Termius-style: pinned Home tab + terminal session tabs) */}
      <header
        data-tauri-drag-region
        className={`h-11 shrink-0 ${
          isMacDesktop ? (isFullscreen ? "pl-[25px]" : "pl-20") : "pl-3"
        } pr-4 bg-header/95 backdrop-blur-md border-b border-border/80 flex items-center justify-between gap-3 z-30 transition-[padding] duration-200`}
      >
        <div
          data-tauri-drag-region
          className="flex items-center gap-1.5 min-w-0 flex-1"
        >
          {workspaceView !== "terminal" && (
            <button
              type="button"
              onClick={() => setSidebarOpen((p) => !p)}
              title="Toggle Navigation Sidebar"
              className="md:hidden p-2 rounded-xl text-muted-foreground hover:bg-accent hover:text-foreground transition-all cursor-pointer"
            >
              <PanelLeft className="w-4 h-4" />
            </button>
          )}

          <button
            type="button"
            onClick={() => setWorkspaceView(lastHomeViewRef.current)}
            className={`flex items-center gap-2 px-3.5 py-1.5 rounded-xl text-xs font-medium cursor-pointer transition-all duration-150 shrink-0 ${
              workspaceView !== "terminal"
                ? "bg-secondary text-foreground border border-primary/40 shadow-xs"
                : "bg-card/50 text-muted-foreground hover:bg-accent/70 hover:text-foreground border border-transparent"
            }`}
          >
            <Home className="w-3.5 h-3.5 text-primary" />
            <span>{t.navHome}</span>
          </button>

          {tabs.length > 0 && (
            <div
              data-tauri-drag-region
              className="flex items-center gap-1.5 min-w-0 overflow-x-auto no-scrollbar"
            >
              {tabs.map((tab) => {
                const isTabActive =
                  workspaceView === "terminal" && tab.id === activeTab?.id;
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
                    className={`group relative flex items-center gap-2 px-3.5 py-1.5 rounded-xl text-xs font-medium cursor-pointer transition-all duration-150 max-w-[230px] shrink-0 ${
                      isTabActive
                        ? "bg-secondary text-foreground border border-primary/40 shadow-xs"
                        : "bg-card/50 text-muted-foreground hover:bg-accent/70 hover:text-foreground border border-transparent"
                    }`}
                  >
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

          <button
            type="button"
            onClick={() => {
              setCommandPaletteQuery("");
              setCommandPaletteOpen(true);
            }}
            title={t.quickConnectBtn}
            className="p-1.5 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent transition-colors cursor-pointer shrink-0"
          >
            <Plus className="w-4 h-4" />
          </button>
        </div>

        {/* Right: terminal tools (only inside a terminal tab) + quick search */}
        <div
          data-tauri-drag-region
          className="flex items-center gap-1.5 shrink-0"
        >
          {workspaceView === "terminal" && activeLeaf && (
            <>
              <button
                type="button"
                onClick={() =>
                  setBroadcastMode((prev) =>
                    prev === "off" ? "tab" : prev === "tab" ? "all" : "off",
                  )
                }
                title={t.termBroadcastActive}
                className={`inline-flex items-center gap-1.5 p-2 rounded-xl text-xs font-semibold border transition-all cursor-pointer ${
                  broadcastMode !== "off"
                    ? "bg-amber-500/15 text-amber-400 border-amber-500/40"
                    : "bg-secondary/70 text-muted-foreground hover:bg-accent hover:text-foreground border-border/70"
                }`}
              >
                <Radio
                  className={`w-3.5 h-3.5 ${
                    broadcastMode !== "off" ? "animate-pulse" : ""
                  }`}
                />
                {broadcastMode !== "off" && (
                  <span>{broadcastMode === "tab" ? "Tab" : "All"}</span>
                )}
              </button>

              <button
                type="button"
                onClick={() => setSnippetsDrawerOpen((p) => !p)}
                title={t.termSnippetsDrawer}
                className={toolBtnClass(snippetsDrawerOpen)}
              >
                <Code2 className="w-3.5 h-3.5 text-pink-400" />
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
                className={toolBtnClass(false)}
              >
                <HardDrive className="w-3.5 h-3.5 text-amber-400" />
              </button>

              <button
                type="button"
                onClick={() => setTelemetryDrawerOpen((p) => !p)}
                title="Toggle Side Monitor Drawer (Cmd+I)"
                className={toolBtnClass(telemetryDrawerOpen)}
              >
                <Activity className="w-3.5 h-3.5" />
              </button>

              <span className="w-px h-5 bg-border/80 mx-1" />
            </>
          )}

          {pendingUpdate && (
            <button
              type="button"
              onClick={handleInstallUpdate}
              disabled={isUpdating}
              title={t.updateConfirm}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-primary/15 text-primary border border-primary/40 text-xs font-semibold transition-all cursor-pointer disabled:opacity-60 disabled:cursor-wait"
            >
              <Download className="w-3.5 h-3.5" />
              <span>
                {isUpdating
                  ? t.updateInstalling
                  : `${t.updateBtn} v${pendingUpdate.version}`}
              </span>
            </button>
          )}

          <button
            type="button"
            onClick={() => {
              setCommandPaletteQuery("");
              setCommandPaletteOpen(true);
            }}
            title={t.quickConnectBtn}
            className="inline-flex items-center gap-2 px-3 py-1.5 rounded-xl bg-secondary/70 hover:bg-accent text-xs font-medium text-muted-foreground hover:text-foreground border border-border/70 transition-all cursor-pointer"
          >
            <Search className="w-3.5 h-3.5 text-primary" />
            <kbd className="px-1.5 py-0.5 rounded-md bg-background/80 border border-border text-[10px] font-mono text-muted-foreground">
              ⌘K
            </kbd>
          </button>
        </div>
      </header>

      {/* Main Workspace Area with Unified Left Sidebar */}
      <div className="relative flex flex-1 min-h-0 min-w-0 overflow-hidden">
        {workspaceView !== "terminal" && sidebarOpen && (
          <div
            onClick={() => setSidebarOpen(false)}
            className="md:hidden fixed inset-0 z-20 bg-black/50 backdrop-blur-xs"
          />
        )}
        {/* Sidebar lives only in the Home tab; terminal tabs get the full width */}
        {workspaceView !== "terminal" && (
          <aside
            className={`${
              sidebarOpen ? "flex" : "hidden md:flex"
            } fixed md:static inset-y-0 left-0 z-30 w-60 shrink-0 bg-header/95 md:bg-header/85 border-r border-border/80 flex-col justify-between p-3 overflow-y-auto select-none`}
          >
            {workspaceView === "settings" ? (
              <div className="space-y-5">
                <button
                  type="button"
                  onClick={() => setWorkspaceView(beforeSettingsViewRef.current)}
                  className="w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-[13px] font-medium text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent transition-all cursor-pointer"
                >
                  <ArrowLeft className="w-4 h-4" />
                  <span>{t.navBack}</span>
                </button>
                {getSettingsNavGroups(appSettings.language).map((group) => (
                  <div key={group.title} className="space-y-1">
                    <div className="px-3 pb-1 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground/75">
                      {group.title}
                    </div>
                    {group.items.map((item) => (
                      <button
                        key={item.id}
                        type="button"
                        onClick={() => setSettingsSection(item.id)}
                        className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-[13px] font-medium transition-all duration-200 cursor-pointer ${
                          settingsSection === item.id
                            ? "bg-primary/15 text-foreground border border-primary/35 shadow-xs"
                            : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                        }`}
                      >
                        {item.icon}
                        <span className="truncate">{item.label}</span>
                      </button>
                    ))}
                  </div>
                ))}
              </div>
            ) : (
              <>
              <div className="space-y-1">
                <button
                  type="button"
                  onClick={() => {
                    setInspectedHostId(null);
                    setWorkspaceView("dashboard");
                  }}
                  className={`w-full flex items-center gap-3 px-3.5 py-2.5 rounded-xl text-[13px] font-medium transition-all duration-200 cursor-pointer ${
                    workspaceView === "dashboard" && !inspectedHostId
                      ? "bg-primary/15 text-foreground border border-primary/35 shadow-xs"
                      : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                  }`}
                >
                  <LayoutDashboard className="w-4 h-4 text-emerald-400" />
                  <span>{t.navDashboard}</span>
                </button>

                {navItems.map((item) => {
                  const active =
                    workspaceView === "resources" &&
                    (item.tabs ?? [item.id]).includes(resourceSubTab);
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
                          ? "bg-primary/15 text-foreground border border-primary/35 shadow-xs"
                          : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                      }`}
                    >
                      <span className="flex items-center gap-3 min-w-0">
                        {item.icon}
                        <span className="truncate">{item.label}</span>
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

              <div className="pt-3 border-t border-border/70">
                <button
                  type="button"
                  onClick={() => setWorkspaceView("settings")}
                  className={`w-full flex items-center justify-between px-3.5 py-2.5 rounded-xl text-[13px] font-medium transition-all cursor-pointer ${
                    "text-muted-foreground hover:bg-accent/60 hover:text-foreground border border-transparent"
                  }`}
                >
                  <span className="flex items-center gap-3">
                    <Settings className="w-4 h-4 text-primary" />
                    <span>{t.navSettings}</span>
                  </span>
                  <kbd className="text-[10px] font-mono text-muted-foreground">
                    ⌘,
                  </kbd>
                </button>
              </div>
              </>
            )}
          </aside>
        )}

        {/* Main Content Viewport */}
        <main className="flex-1 min-w-0 min-h-0 bg-background overflow-hidden relative">
          {/* Terminal View: every tab stays mounted (inactive ones hidden) so switching tabs never tears down xterm or the PTY/SSH session */}
          <div
            className={
              workspaceView === "terminal" && activeTab
                ? "h-full w-full"
                : "hidden"
            }
          >
            {tabs.map((tab) => (
              <div
                key={tab.id}
                className={tab.id === activeTab?.id ? "h-full w-full" : "hidden"}
              >
                <SplitWorkspace
                  node={tab.root}
                  activePaneId={tab.activePaneId}
                  totalPanes={collectLeaves(tab.root).length}
                  themeConfig={themeConfig}
                  isVisible={
                    workspaceView === "terminal" && tab.id === activeTab?.id
                  }
                  language={appSettings.language}
                  copyOnSelect={appSettings.copyOnSelect}
                  broadcastTargetPaneIds={broadcastTargetPaneIds}
                  onFocusPane={handleFocusPane}
                  onUpdateLeafStatus={handleUpdateLeafStatus}
                  onTriggerReconnect={handleTriggerReconnect}
                  onSplitPane={handleSplitPane}
                  onClosePane={handleClosePane}
                  onResizeSplit={handleResizeSplit}
                />
              </div>
            ))}
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
              onOpenLocalTerminal={openLocalTab}
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

          {/* Resources & Toolbox View (Keychain groups several sub-tabs behind one sidebar item) */}
          {workspaceView === "resources" && (
            <div className="flex flex-col h-full">
              {KEYCHAIN_TABS.includes(resourceSubTab) && (
                <div className="px-5 pt-3 flex items-center gap-1.5 shrink-0">
                  {keychainItems.map((item) => (
                    <button
                      key={item.id}
                      type="button"
                      onClick={() => setResourceSubTab(item.id)}
                      className={`px-3 py-1.5 rounded-xl text-xs font-medium border transition-all cursor-pointer ${
                        resourceSubTab === item.id
                          ? "bg-primary/15 text-foreground border-primary/35"
                          : "text-muted-foreground hover:bg-accent/60 hover:text-foreground border-transparent"
                      }`}
                    >
                      {item.label}
                    </button>
                  ))}
                </div>
              )}
              <div className="flex-1 min-h-0">
                <ResourcesWorkspace
                  hosts={hosts}
                  resources={resources}
                  appSettings={appSettings}
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
                  onOpenLocalTerminal={openLocalTab}
                  onRunSnippetInActivePane={handleRunSnippetInActivePane}
                />
              </div>
            </div>
          )}

          {/* Dedicated Settings View */}
          {workspaceView === "settings" && (
            <SettingsWorkspace
              themeConfig={themeConfig}
              appSettings={appSettings}
              resources={resources}
              activeTab={settingsSection}
              onChangeTheme={handleUpdateTheme}
              onChangeAppSettings={handleUpdateAppSettings}
              onUpdateResources={setResources}
            />
          )}
        </main>

        {/* Quick Snippets Side Drawer inside Terminal View */}
        {workspaceView === "terminal" && snippetsDrawerOpen && (
          <aside className="w-72 shrink-0 bg-header/95 border-l border-border/80 flex flex-col h-full z-20">
            <div className="h-11 px-3.5 border-b border-border/70 flex items-center justify-between">
              <div className="flex items-center gap-2 text-xs font-semibold text-foreground">
                <Code2 className="w-4 h-4 text-pink-400" />
                <span>{t.termSnippetsDrawer}</span>
              </div>
              <button
                type="button"
                onClick={() => setSnippetsDrawerOpen(false)}
                className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent cursor-pointer"
              >
                <X className="w-3.5 h-3.5" />
              </button>
            </div>
            <div className="p-2.5 border-b border-border/60">
              <div className="relative flex items-center">
                <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-2.5 pointer-events-none" />
                <input
                  type="text"
                  value={snippetDrawerSearch}
                  onChange={(e) => setSnippetDrawerSearch(e.target.value)}
                  placeholder="Tìm kiếm snippet..."
                  className="w-full pl-8 pr-2.5 py-1.5 rounded-xl bg-background/80 border border-border/70 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
                />
              </div>
            </div>
            <div className="flex-1 overflow-y-auto p-2.5 space-y-2">
              {resources.snippets
                .filter((s: SnippetItem) => {
                  const q = snippetDrawerSearch.trim().toLowerCase();
                  if (!q) return true;
                  return (
                    s.name.toLowerCase().includes(q) ||
                    s.code.toLowerCase().includes(q)
                  );
                })
                .map((s: SnippetItem) => {
                  const vars = extractSnippetVariables(s.code);
                  return (
                    <div
                      key={s.id}
                      className="p-2.5 rounded-xl bg-card/80 border border-border/70 hover:border-primary/40 transition-all space-y-1.5"
                    >
                      <div className="flex items-center justify-between gap-2">
                        <span className="text-xs font-semibold text-foreground truncate">
                          {s.name}
                        </span>
                        <button
                          type="button"
                          onClick={() =>
                            handleRunSnippetInActivePane(s.code, s.name)
                          }
                          className="px-2 py-1 rounded-lg bg-primary text-primary-foreground text-[11px] font-semibold inline-flex items-center gap-1 hover:opacity-90 cursor-pointer shrink-0"
                        >
                          <Play className="w-3 h-3" />
                          <span>{t.termRunBtn}</span>
                        </button>
                      </div>
                      <pre className="text-[10px] font-mono text-muted-foreground bg-background/70 px-2 py-1 rounded-lg overflow-x-auto max-h-16">
                        {s.code}
                      </pre>
                      {vars.length > 0 && (
                        <div className="flex flex-wrap gap-1">
                          {vars.map((v) => (
                            <span
                              key={v}
                              className="px-1.5 py-0.5 rounded-md bg-pink-500/15 text-pink-300 border border-pink-500/30 font-mono text-[10px]"
                            >
                              {`{{${v}}}`}
                            </span>
                          ))}
                        </div>
                      )}
                    </div>
                  );
                })}
            </div>
          </aside>
        )}

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

      {/* Dynamic Snippet Variable Prompt Modal */}
      {pendingVarSnippet && (
        <div
          onClick={() => setPendingVarSnippet(null)}
          className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-center justify-center p-4"
        >
          <form
            onClick={(e) => e.stopPropagation()}
            onSubmit={(e) => {
              e.preventDefault();
              const finalCmd = interpolateSnippetVariables(
                pendingVarSnippet.code,
                pendingVarSnippet.values,
              );
              setPendingVarSnippet(null);
              sendCommandToTargetPanes(finalCmd);
            }}
            className="w-full max-w-md rounded-2xl bg-card border border-border p-5 shadow-2xl space-y-4"
          >
            <div className="flex items-center justify-between">
              <div>
                <h3 className="text-sm font-semibold text-foreground">
                  {t.termFillVariablesTitle}: {pendingVarSnippet.name}
                </h3>
                <p className="text-xs text-muted-foreground mt-0.5">
                  Điền giá trị cho các biến động trước khi gửi vào Terminal
                </p>
              </div>
              <button
                type="button"
                onClick={() => setPendingVarSnippet(null)}
                className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <div className="space-y-3">
              {pendingVarSnippet.vars.map((varName, idx) => (
                <div key={varName} className="space-y-1">
                  <label className="text-xs font-mono font-medium text-pink-400">
                    {`{{${varName}}}`}
                  </label>
                  <input
                    type="text"
                    autoFocus={idx === 0}
                    required
                    value={pendingVarSnippet.values[varName] ?? ""}
                    onChange={(e) =>
                      setPendingVarSnippet((prev) =>
                        prev
                          ? {
                              ...prev,
                              values: {
                                ...prev.values,
                                [varName]: e.target.value,
                              },
                            }
                          : null,
                      )
                    }
                    placeholder={`Nhập giá trị cho ${varName}...`}
                    className="w-full px-3 py-2 rounded-xl bg-background border border-border text-xs text-foreground focus:outline-none focus:border-primary"
                  />
                </div>
              ))}
            </div>

            <div className="p-2.5 rounded-xl bg-background/80 border border-border/70">
              <div className="text-[10px] uppercase tracking-wider text-muted-foreground mb-1">
                Preview
              </div>
              <pre className="text-xs font-mono text-emerald-400 whitespace-pre-wrap break-all">
                {interpolateSnippetVariables(
                  pendingVarSnippet.code,
                  pendingVarSnippet.values,
                )}
              </pre>
            </div>

            <div className="flex items-center justify-end gap-2 pt-1">
              <button
                type="button"
                onClick={() => setPendingVarSnippet(null)}
                className="px-3.5 py-2 rounded-xl bg-secondary text-xs font-medium text-foreground hover:bg-accent cursor-pointer"
              >
                {t.termCancelBtn}
              </button>
              <button
                type="submit"
                className="px-4 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 cursor-pointer"
              >
                {t.termRunBtn}
              </button>
            </div>
          </form>
        </div>
      )}

      {/* Global Command Palette (Cmd+K) */}
      {commandPaletteOpen && (
        <div
          onClick={() => setCommandPaletteOpen(false)}
          className="fixed inset-0 z-50 bg-black/60 backdrop-blur-xs flex items-start justify-center pt-20 p-4"
        >
          <div
            onClick={(e) => e.stopPropagation()}
            className="w-full max-w-xl rounded-2xl bg-card border border-border shadow-2xl overflow-hidden"
          >
            <div className="flex items-center gap-2.5 px-4 py-3 border-b border-border bg-header/70">
              <Search className="w-4 h-4 text-primary shrink-0" />
              <input
                type="text"
                autoFocus
                value={commandPaletteQuery}
                onChange={(e) => setCommandPaletteQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Escape") {
                    setCommandPaletteOpen(false);
                  }
                }}
                placeholder={t.cmdPalettePlaceholder}
                className="flex-1 bg-transparent text-sm text-foreground placeholder:text-muted-foreground focus:outline-none"
              />
              <kbd className="px-2 py-0.5 rounded-md bg-secondary text-[10px] font-mono text-muted-foreground">
                ESC
              </kbd>
            </div>

            <div className="max-h-96 overflow-y-auto p-3 space-y-4">
              {/* Quick actions section */}
              {openLocalTab &&
                (!commandPaletteQuery.trim() ||
                  t.newLocalShell
                    .toLowerCase()
                    .includes(commandPaletteQuery.trim().toLowerCase())) && (
                <div>
                  <div className="px-2 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                    {t.cmdPaletteSectionQuickActions}
                  </div>
                  <button
                    type="button"
                    onClick={() => {
                      setCommandPaletteOpen(false);
                      openLocalTab();
                    }}
                    className="w-full flex items-center justify-between px-3 py-2 rounded-xl hover:bg-accent/70 transition-colors cursor-pointer"
                  >
                    <span className="flex items-center gap-2.5 text-xs font-semibold text-foreground">
                      <TerminalIcon className="w-4 h-4 text-emerald-400" />
                      {t.newLocalShell}
                    </span>
                    <kbd className="px-1.5 py-0.5 rounded-md bg-secondary text-[10px] font-mono text-muted-foreground">
                      ⌘T
                    </kbd>
                  </button>
                </div>
              )}

              {/* Hosts section */}
              <div>
                <div className="px-2 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                  {t.cmdPaletteSectionHosts}
                </div>
                <div className="space-y-1">
                  {hosts
                    .filter((h) => {
                      const q = commandPaletteQuery.trim().toLowerCase();
                      if (!q) return true;
                      return (
                        h.alias.toLowerCase().includes(q) ||
                        h.hostname.toLowerCase().includes(q) ||
                        h.user.toLowerCase().includes(q) ||
                        h.group.toLowerCase().includes(q)
                      );
                    })
                    .map((h) => (
                      <div
                        key={h.id}
                        className="flex items-center justify-between px-3 py-2 rounded-xl hover:bg-accent/70 transition-colors"
                      >
                        <div className="flex items-center gap-2.5 min-w-0">
                          <Server className="w-4 h-4 text-emerald-400 shrink-0" />
                          <div className="min-w-0">
                            <div className="text-xs font-semibold text-foreground truncate">
                              {h.alias || h.hostname}
                            </div>
                            <div className="text-[11px] font-mono text-muted-foreground truncate">
                              {h.user}@{h.hostname}:{h.port}
                            </div>
                          </div>
                        </div>
                        <div className="flex items-center gap-1.5 shrink-0">
                          <button
                            type="button"
                            onClick={() => {
                              setCommandPaletteOpen(false);
                              handleConnectHostTab(h);
                            }}
                            className="px-2.5 py-1 rounded-lg bg-primary text-primary-foreground text-[11px] font-semibold hover:opacity-90 cursor-pointer"
                          >
                            {t.cmdActionOpenSsh}
                          </button>
                          <button
                            type="button"
                            onClick={() => {
                              setCommandPaletteOpen(false);
                              setInspectedHostId(h.id);
                              setResourceSubTab("sftp");
                              setWorkspaceView("resources");
                            }}
                            className="px-2.5 py-1 rounded-lg bg-secondary hover:bg-accent text-foreground text-[11px] font-medium border border-border/70 cursor-pointer"
                          >
                            {t.cmdActionOpenSftp}
                          </button>
                        </div>
                      </div>
                    ))}
                </div>
              </div>

              {/* Snippets section */}
              {resources.snippets.length > 0 && (
                <div>
                  <div className="px-2 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                    {t.cmdPaletteSectionSnippets}
                  </div>
                  <div className="space-y-1">
                    {resources.snippets
                      .filter((s) => {
                        const q = commandPaletteQuery.trim().toLowerCase();
                        if (!q) return true;
                        return (
                          s.name.toLowerCase().includes(q) ||
                          s.code.toLowerCase().includes(q)
                        );
                      })
                      .map((s) => (
                        <div
                          key={s.id}
                          className="flex items-center justify-between px-3 py-2 rounded-xl hover:bg-accent/70 transition-colors"
                        >
                          <div className="min-w-0 pr-3">
                            <div className="text-xs font-semibold text-foreground truncate">
                              {s.name}
                            </div>
                            <div className="text-[11px] font-mono text-muted-foreground truncate">
                              {s.code}
                            </div>
                          </div>
                          <button
                            type="button"
                            onClick={() => {
                              setCommandPaletteOpen(false);
                              handleRunSnippetInActivePane(s.code, s.name);
                            }}
                            className="px-2.5 py-1 rounded-lg bg-pink-500/20 text-pink-300 border border-pink-500/30 text-[11px] font-semibold hover:bg-pink-500/30 cursor-pointer shrink-0"
                          >
                            {t.cmdActionRunSnippet}
                          </button>
                        </div>
                      ))}
                  </div>
                </div>
              )}

              {/* Port Forwarding section */}
              {resources.portForwardingRules.length > 0 && (
                <div>
                  <div className="px-2 pb-1.5 text-[10px] font-semibold uppercase tracking-wider text-muted-foreground">
                    {t.cmdPaletteSectionPortForward}
                  </div>
                  <div className="space-y-1">
                    {resources.portForwardingRules
                      .filter((r) => {
                        const q = commandPaletteQuery.trim().toLowerCase();
                        if (!q) return true;
                        return r.name.toLowerCase().includes(q);
                      })
                      .map((r) => {
                        const isRunning = r.status === "running";
                        return (
                          <div
                            key={r.id}
                            className="flex items-center justify-between px-3 py-2 rounded-xl hover:bg-accent/70 transition-colors"
                          >
                            <div className="min-w-0">
                              <div className="text-xs font-semibold text-foreground truncate">
                                {r.name} ({r.mode.toUpperCase()})
                              </div>
                              <div className="text-[11px] font-mono text-muted-foreground truncate">
                                {r.localHost}:{r.localPort} → {r.targetHost}:
                                {r.targetPort}
                              </div>
                            </div>
                            <button
                              type="button"
                              onClick={async () => {
                                try {
                                  const next =
                                    await invoke<ResourcesStoreData>(
                                      "toggle_port_forwarding_rule",
                                      {
                                        ruleId: r.id,
                                        active: !isRunning,
                                      },
                                    );
                                  setResources(next);
                                } catch {
                                  // ignore
                                }
                              }}
                              className={`px-2.5 py-1 rounded-lg text-[11px] font-semibold cursor-pointer ${
                                isRunning
                                  ? "bg-emerald-500/20 text-emerald-300 border border-emerald-500/30"
                                  : "bg-secondary text-foreground border border-border"
                              }`}
                            >
                              {isRunning ? "Running (Stop)" : "Start Tunnel"}
                            </button>
                          </div>
                        );
                      })}
                  </div>
                </div>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
