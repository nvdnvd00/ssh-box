import React, { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  ArrowUpDown,
  Box,
  Check,
  Code2,
  Container,
  Copy,
  Download,
  Edit3,
  FileKey,
  FolderGit2,
  Globe,
  HardDrive,
  History,
  KeyRound,
  Laptop,
  LayoutGrid,
  Layers,
  List,
  Network,
  Package,
  Play,
  Plus,
  RefreshCw,
  RotateCw,
  Search,
  Server,
  Shield,
  Square,
  Tag,
  Terminal,
  Trash2,
  Upload,
  UserCheck,
  Waypoints,
  X,
} from "lucide-react";
import {
  ConnectionProxyProfile,
  DockerFullSnapshot,
  DockerPanelTab,
  HostEntry,
  IdentityItem,
  KnownHostEntry,
  MachineAuthMethod,
  MachineGroup,
  MachineTag,
  PortForwardingRule,
  ResourceDisplayStyle,
  ResourceSortStyle,
  ResourcesStoreData,
  SnippetItem,
  SnippetPackage,
  SshKeyItem,
  TabResources,
} from "../types";
import { AppSettingsConfig } from "../themes";
import { getI18n } from "../i18n";
import { SelectField } from "./SelectField";
import { SftpWorkspace } from "./SftpWorkspace";

interface ResourcesWorkspaceProps {
  hosts: HostEntry[];
  resources: ResourcesStoreData;
  appSettings?: AppSettingsConfig;
  activeHostId: string;
  initialTab?: TabResources;
  hideSidebar?: boolean;
  openCreateMachineSignal?: number;
  onRefreshHosts: () => void;
  onUpdateResources: (next: ResourcesStoreData) => void;
  onSaveHost: (host: HostEntry, secret?: string) => Promise<void>;
  onDeleteHost: (hostId: string) => Promise<void>;
  onConnectHost: (host: HostEntry) => void;
  onInspectMachine?: (hostId: string) => void;
  onOpenLocalTerminal?: () => void;
  onOpenCommandTab?: (
    hostId: string,
    title: string,
    startupCommand: string,
  ) => void;
  onRunSnippetInActivePane?: (code: string, snippetName?: string) => void;
}

const TAG_PALETTE = [
  "#38bdf8",
  "#34d399",
  "#a78bfa",
  "#fbbf24",
  "#f87171",
  "#fb923c",
  "#ec4899",
];

function downloadJson(fileName: string, data: unknown) {
  const blob = new Blob([JSON.stringify(data, null, 2)], {
    type: "application/json",
  });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = fileName;
  a.click();
  URL.revokeObjectURL(url);
}

// Exported machines carry no identity/SSH key links.
function toPortableMachine(h: HostEntry): HostEntry {
  return {
    ...h,
    source: "custom",
    authMethod: h.authMethod === "identity" ? "passwordOrKey" : h.authMethod,
    identityId: null,
    sshKeyId: null,
  };
}

function formatUnixDate(ts?: number | null): string {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleString();
}

export const ResourcesWorkspace: React.FC<ResourcesWorkspaceProps> = ({
  hosts,
  resources,
  appSettings,
  activeHostId,
  initialTab = "machines",
  hideSidebar = false,
  openCreateMachineSignal = 0,
  onRefreshHosts,
  onUpdateResources,
  onSaveHost,
  onDeleteHost,
  onConnectHost,
  onInspectMachine,
  onOpenLocalTerminal,
  onOpenCommandTab,
  onRunSnippetInActivePane,
}) => {
  const t = getI18n(appSettings?.language);
  const [activeSubTab, setActiveSubTab] = useState<TabResources>(initialTab);
  const [knownHosts, setKnownHosts] = useState<KnownHostEntry[]>([]);
  const [exportKeyModal, setExportKeyModal] = useState<{
    keyItem: SshKeyItem;
    targetHostId: string;
    password: string;
    busy: boolean;
    successMsg?: string;
  } | null>(null);

  const loadKnownHosts = useCallback(() => {
    invoke<KnownHostEntry[]>("list_known_hosts")
      .then((list) => setKnownHosts(list))
      .catch(() => undefined);
  }, []);

  useEffect(() => {
    if (activeSubTab === "knownHosts") {
      loadKnownHosts();
    }
  }, [activeSubTab, loadKnownHosts]);
  const [sftpTargetHostId, setSftpTargetHostId] = useState<string>(activeHostId);
  const [displayStyle, setDisplayStyle] =
    useState<ResourceDisplayStyle>("grid");
  const [sortStyle, setSortStyle] = useState<ResourceSortStyle>("name");
  const [searchQuery, setSearchQuery] = useState("");
  const [errorBanner, setErrorBanner] = useState<string | null>(null);
  const [dockerStateFilter, setDockerStateFilter] = useState<
    "all" | "running" | "stopped" | "paused"
  >("all");

  useEffect(() => {
    if (initialTab) {
      setActiveSubTab(initialTab);
    }
  }, [initialTab]);

  useEffect(() => {
    if (activeHostId && activeHostId !== "local") {
      setSftpTargetHostId(activeHostId);
    }
  }, [activeHostId]);

  useEffect(() => {
    if (openCreateMachineSignal > 0) {
      setActiveSubTab("machines");
      setEditingHost({
        id: "",
        alias: "",
        hostname: "",
        user: "root",
        port: 22,
        identityFile: null,
        group: "Production",
        source: "custom",
        remark: "",
        tags: [],
        authMethod: "passwordOrKey",
        identityId: null,
        sshKeyId: null,
        proxyProfileId: null,
        jumpHostId: null,
        showStatus: true,
        defaultSftpPath: "/",
      });
      setHostSecret("");
    }
  }, [openCreateMachineSignal]);

  // Machine filters & modals
  const [selectedGroupFilter, setSelectedGroupFilter] = useState<string>("ALL");
  const [selectedTagFilter, setSelectedTagFilter] = useState<string>("ALL");
  const [editingHost, setEditingHost] = useState<HostEntry | null>(null);
  const [hostSecret, setHostSecret] = useState("");
  const [testingHost, setTestingHost] = useState(false);
  const [hostTestResult, setHostTestResult] = useState<{
    ok: boolean;
    message: string;
  } | null>(null);
  // A result only describes the values it was run with
  useEffect(() => setHostTestResult(null), [editingHost, hostSecret]);
  const [showGroupModal, setShowGroupModal] = useState(false);
  const [groupForm, setGroupForm] = useState<MachineGroup>({
    id: "",
    name: "",
    remark: "",
    createdDate: 0,
  });
  const [showTagModal, setShowTagModal] = useState(false);
  const [tagForm, setTagForm] = useState<MachineTag>({
    id: "",
    name: "",
    color: TAG_PALETTE[0],
    createdDate: 0,
  });

  // Identity modal
  const [editingIdentity, setEditingIdentity] = useState<IdentityItem | null>(
    null,
  );
  const [identityPassword, setIdentityPassword] = useState("");

  // SSH Key modals
  const [showKeyGenerateModal, setShowKeyGenerateModal] = useState(false);
  const [keyGenName, setKeyGenName] = useState("");
  const [keyGenRemark, setKeyGenRemark] = useState("");
  const [keyGenType, setKeyGenType] = useState<"ed25519" | "rsa" | "ecdsa">(
    "ed25519",
  );
  const [keyGenPassphrase, setKeyGenPassphrase] = useState("");

  const [showKeyImportModal, setShowKeyImportModal] = useState(false);
  const [keyImportName, setKeyImportName] = useState("");
  const [keyImportRemark, setKeyImportRemark] = useState("");
  const [keyImportPrivate, setKeyImportPrivate] = useState("");
  const [keyImportPublic, setKeyImportPublic] = useState("");
  const [keyImportPassphrase, setKeyImportPassphrase] = useState("");
  const [copiedKeyId, setCopiedKeyId] = useState<string | null>(null);

  // Proxy modal
  const [editingProxy, setEditingProxy] =
    useState<ConnectionProxyProfile | null>(null);
  const [proxyPassword, setProxyPassword] = useState("");

  // Port Forwarding modal
  const [editingPfRule, setEditingPfRule] = useState<PortForwardingRule | null>(
    null,
  );
  const [togglingPfId, setTogglingPfId] = useState<string | null>(null);

  // Snippets state
  const [selectedPackageId, setSelectedPackageId] = useState<string>("ALL");
  const [showSnippetHistory, setShowSnippetHistory] = useState(false);
  const [editingPackage, setEditingPackage] = useState<SnippetPackage | null>(
    null,
  );
  const [editingSnippet, setEditingSnippet] = useState<SnippetItem | null>(
    null,
  );
  const [batchSnippet, setBatchSnippet] = useState<SnippetItem | null>(null);
  const [batchMachineIds, setBatchMachineIds] = useState<string[]>([]);
  const [batchGroupNames, setBatchGroupNames] = useState<string[]>([]);
  const [runningBatch, setRunningBatch] = useState(false);

  // Docker state
  const [dockerHostId, setDockerHostId] = useState<string>(
    activeHostId || "local",
  );
  const [dockerTab, setDockerTab] = useState<DockerPanelTab>("overview");
  const [dockerSnap, setDockerSnap] = useState<DockerFullSnapshot | null>(null);
  const [dockerLoading, setDockerLoading] = useState(false);
  const [dockerLogsModal, setDockerLogsModal] = useState<{
    containerName: string;
    logs: string;
  } | null>(null);

  const loadDockerSnapshot = useCallback(async (targetHostId: string) => {
    setDockerLoading(true);
    setErrorBanner(null);
    try {
      const snap = await invoke<DockerFullSnapshot>("fetch_docker_snapshot", {
        hostId: targetHostId,
      });
      setDockerSnap(snap);
    } catch (err) {
      setErrorBanner(String(err));
    } finally {
      setDockerLoading(false);
    }
  }, []);

  useEffect(() => {
    if (activeSubTab === "docker") {
      loadDockerSnapshot(dockerHostId);
    }
  }, [activeSubTab, dockerHostId, loadDockerSnapshot]);

  const sortItems = useCallback(
    <T extends { createdDate?: number }>(
      items: T[],
      getName: (item: T) => string,
    ): T[] => {
      const copy = [...items];
      if (sortStyle === "date") {
        copy.sort((a, b) => (b.createdDate ?? 0) - (a.createdDate ?? 0));
      } else {
        copy.sort((a, b) => getName(a).localeCompare(getName(b)));
      }
      return copy;
    },
    [sortStyle],
  );

  const allGroupNames = useMemo(() => {
    const set = new Set<string>();
    for (const g of resources.groups) {
      if (g.name.trim()) set.add(g.name.trim());
    }
    for (const h of hosts) {
      if (h.group.trim()) set.add(h.group.trim());
    }
    return Array.from(set).sort();
  }, [resources.groups, hosts]);

  const filteredMachines = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const list = hosts.filter((h) => {
      if (selectedGroupFilter !== "ALL" && h.group !== selectedGroupFilter) {
        return false;
      }
      if (
        selectedTagFilter !== "ALL" &&
        !(h.tags ?? []).includes(selectedTagFilter)
      ) {
        return false;
      }
      if (!q) return true;
      return (
        h.alias.toLowerCase().includes(q) ||
        h.hostname.toLowerCase().includes(q) ||
        h.user.toLowerCase().includes(q) ||
        (h.remark ?? "").toLowerCase().includes(q)
      );
    });
    return sortItems(list, (h) => h.alias || h.hostname);
  }, [hosts, searchQuery, selectedGroupFilter, selectedTagFilter, sortItems]);

  const filteredIdentities = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const list = resources.identities.filter(
      (i) =>
        !q ||
        i.name.toLowerCase().includes(q) ||
        i.userName.toLowerCase().includes(q) ||
        i.remark.toLowerCase().includes(q),
    );
    return sortItems(list, (i) => i.name);
  }, [resources.identities, searchQuery, sortItems]);

  const filteredKeys = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const list = resources.sshKeys.filter(
      (k) =>
        !q ||
        k.name.toLowerCase().includes(q) ||
        k.keyType.toLowerCase().includes(q) ||
        k.fingerprint.toLowerCase().includes(q) ||
        k.remark.toLowerCase().includes(q),
    );
    return sortItems(list, (k) => k.name);
  }, [resources.sshKeys, searchQuery, sortItems]);

  const filteredProxies = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const list = resources.proxyProfiles.filter(
      (p) =>
        !q ||
        p.name.toLowerCase().includes(q) ||
        p.host.toLowerCase().includes(q) ||
        p.remark.toLowerCase().includes(q),
    );
    return sortItems(list, (p) => p.name);
  }, [resources.proxyProfiles, searchQuery, sortItems]);

  const filteredPfRules = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const list = resources.portForwardingRules.filter(
      (r) =>
        !q ||
        r.name.toLowerCase().includes(q) ||
        r.mode.toLowerCase().includes(q) ||
        r.targetHost.toLowerCase().includes(q) ||
        r.remark.toLowerCase().includes(q),
    );
    return sortItems(list, (r) => r.name);
  }, [resources.portForwardingRules, searchQuery, sortItems]);

  const filteredSnippets = useMemo(() => {
    const q = searchQuery.trim().toLowerCase();
    const list = resources.snippets.filter((s) => {
      if (selectedPackageId === "UNGROUPED" && s.packageId) return false;
      if (
        selectedPackageId !== "ALL" &&
        selectedPackageId !== "UNGROUPED" &&
        s.packageId !== selectedPackageId
      ) {
        return false;
      }
      if (!q) return true;
      return (
        s.name.toLowerCase().includes(q) ||
        s.code.toLowerCase().includes(q) ||
        s.remark.toLowerCase().includes(q)
      );
    });
    return sortItems(list, (s) => s.name);
  }, [resources.snippets, selectedPackageId, searchQuery, sortItems]);

  // Handlers
  const handleSaveHostForm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingHost || !editingHost.hostname.trim()) return;
    setErrorBanner(null);
    try {
      await onSaveHost(
        {
          ...editingHost,
          alias: editingHost.alias.trim() || editingHost.hostname.trim(),
          hostname: editingHost.hostname.trim(),
          user: editingHost.user.trim() || "root",
          group: editingHost.group.trim() || "Production",
          source: "custom",
        },
        hostSecret.trim() ? hostSecret.trim() : undefined,
      );
      setEditingHost(null);
      setHostSecret("");
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleTestHostConnection = async () => {
    if (!editingHost || !editingHost.hostname.trim()) return;
    setTestingHost(true);
    try {
      const fingerprint = await invoke<string>("test_ssh_connection", {
        host: {
          ...editingHost,
          hostname: editingHost.hostname.trim(),
          user: editingHost.user.trim() || "root",
        },
        secret: hostSecret.trim() || null,
      });
      setHostTestResult({ ok: true, message: `Connected · ${fingerprint}` });
    } catch (err) {
      setHostTestResult({ ok: false, message: String(err) });
    } finally {
      setTestingHost(false);
    }
  };

  const handleSaveGroup = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!groupForm.name.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>("save_machine_group", {
        group: groupForm,
      });
      onUpdateResources(next);
      setGroupForm({ id: "", name: "", remark: "", createdDate: 0 });
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleDeleteGroup = async (groupId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>("delete_machine_group", {
        groupId,
      });
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleSaveTag = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!tagForm.name.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>("save_machine_tag", {
        tag: tagForm,
      });
      onUpdateResources(next);
      setTagForm({
        id: "",
        name: "",
        color: TAG_PALETTE[0],
        createdDate: 0,
      });
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleDeleteTag = async (tagId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>("delete_machine_tag", {
        tagId,
      });
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleSaveIdentityForm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingIdentity || !editingIdentity.name.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>("save_identity", {
        identity: editingIdentity,
        password: identityPassword.trim() ? identityPassword.trim() : null,
      });
      onUpdateResources(next);
      setEditingIdentity(null);
      setIdentityPassword("");
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleDeleteIdentity = async (identityId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>("delete_identity", {
        identityId,
      });
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleGenerateKey = async (e: React.FormEvent) => {
    e.preventDefault();
    try {
      const next = await invoke<ResourcesStoreData>("generate_ssh_key", {
        name: keyGenName,
        remark: keyGenRemark,
        keyType: keyGenType,
        passphrase: keyGenPassphrase.trim() ? keyGenPassphrase.trim() : null,
      });
      onUpdateResources(next);
      setShowKeyGenerateModal(false);
      setKeyGenName("");
      setKeyGenRemark("");
      setKeyGenPassphrase("");
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleImportKey = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!keyImportName.trim() || !keyImportPrivate.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>("import_ssh_key", {
        keyId: null,
        name: keyImportName,
        remark: keyImportRemark,
        privateKey: keyImportPrivate,
        publicKey: keyImportPublic.trim() ? keyImportPublic.trim() : null,
        passphrase: keyImportPassphrase.trim()
          ? keyImportPassphrase.trim()
          : null,
      });
      onUpdateResources(next);
      setShowKeyImportModal(false);
      setKeyImportName("");
      setKeyImportRemark("");
      setKeyImportPrivate("");
      setKeyImportPublic("");
      setKeyImportPassphrase("");
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleDeleteKey = async (keyId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>("delete_ssh_key", {
        keyId,
      });
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleCopyPubKey = (item: SshKeyItem) => {
    navigator.clipboard?.writeText(item.publicKey);
    setCopiedKeyId(item.id);
    setTimeout(() => setCopiedKeyId(null), 1500);
  };

  // Machine export/import: the user picks which machines go in or out.
  const machineFileInputRef = useRef<HTMLInputElement | null>(null);
  const [machineTransfer, setMachineTransfer] = useState<{
    mode: "export" | "import";
    items: HostEntry[];
    selected: boolean[];
  } | null>(null);

  const openMachineTransfer = (
    mode: "export" | "import",
    items: HostEntry[],
  ) => setMachineTransfer({ mode, items, selected: items.map(() => true) });

  const handlePickMachineFile = async (
    e: React.ChangeEvent<HTMLInputElement>,
  ) => {
    const file = e.target.files?.[0];
    e.target.value = "";
    if (!file) return;
    setErrorBanner(null);
    try {
      const data = JSON.parse(await file.text());
      if (data?.type !== "ssh-box/machines" || !Array.isArray(data.items)) {
        throw new Error("Not an ssh-box machines export file");
      }
      openMachineTransfer("import", data.items.map(toPortableMachine));
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleConfirmMachineTransfer = async () => {
    if (!machineTransfer) return;
    const { mode, items, selected } = machineTransfer;
    const picked = items.filter((_, i) => selected[i]);
    setMachineTransfer(null);
    setErrorBanner(null);
    try {
      if (mode === "export") {
        downloadJson(
          `ssh-box-machines-${new Date().toISOString().slice(0, 10)}.json`,
          { type: "ssh-box/machines", items: picked.map(toPortableMachine) },
        );
        return;
      }
      for (const m of picked) {
        // Same id already here: update it but keep its local identity/key links.
        const existing = hosts.find((h) => h.id === m.id);
        await onSaveHost(
          existing
            ? {
                ...m,
                authMethod: existing.authMethod,
                identityId: existing.identityId,
                sshKeyId: existing.sshKeyId,
              }
            : m,
        );
      }
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleSaveProxyForm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingProxy || !editingProxy.name.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>("save_proxy_profile", {
        profile: editingProxy,
        password: proxyPassword.trim() ? proxyPassword.trim() : null,
      });
      onUpdateResources(next);
      setEditingProxy(null);
      setProxyPassword("");
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleDeleteProxy = async (profileId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>("delete_proxy_profile", {
        profileId,
      });
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleSavePfForm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingPfRule || !editingPfRule.name.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>(
        "save_port_forwarding_rule",
        {
          rule: editingPfRule,
        },
      );
      onUpdateResources(next);
      setEditingPfRule(null);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleTogglePfRule = async (rule: PortForwardingRule) => {
    setTogglingPfId(rule.id);
    setErrorBanner(null);
    try {
      const nextActive = rule.status !== "running";
      const next = await invoke<ResourcesStoreData>(
        "toggle_port_forwarding_rule",
        {
          ruleId: rule.id,
          active: nextActive,
        },
      );
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
      const refreshed = await invoke<ResourcesStoreData>("get_resources").catch(
        () => null,
      );
      if (refreshed) onUpdateResources(refreshed);
    } finally {
      setTogglingPfId(null);
    }
  };

  // Sequential so each card shows its own spinner and errors stay per rule
  const handleToggleAllPfRules = async (active: boolean) => {
    setErrorBanner(null);
    const errors: string[] = [];
    for (const rule of resources.portForwardingRules) {
      if ((rule.status === "running") === active) continue;
      setTogglingPfId(rule.id);
      try {
        await invoke("toggle_port_forwarding_rule", {
          ruleId: rule.id,
          active,
        });
      } catch (err) {
        errors.push(`${rule.name}: ${err}`);
      }
    }
    setTogglingPfId(null);
    const refreshed = await invoke<ResourcesStoreData>("get_resources").catch(
      () => null,
    );
    if (refreshed) onUpdateResources(refreshed);
    if (errors.length > 0) setErrorBanner(errors.join(" · "));
  };

  const handleDeletePfRule = async (ruleId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>(
        "delete_port_forwarding_rule",
        { ruleId },
      );
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleSaveSnippetPackageForm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingPackage || !editingPackage.name.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>("save_snippet_package", {
        pkg: editingPackage,
      });
      onUpdateResources(next);
      setEditingPackage(null);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleDeleteSnippetPackage = async (packageId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>("delete_snippet_package", {
        packageId,
      });
      onUpdateResources(next);
      if (selectedPackageId === packageId) {
        setSelectedPackageId("ALL");
      }
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleSaveSnippetForm = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editingSnippet || !editingSnippet.name.trim()) return;
    try {
      const next = await invoke<ResourcesStoreData>("save_snippet", {
        snippet: editingSnippet,
      });
      onUpdateResources(next);
      setEditingSnippet(null);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleDeleteSnippet = async (snippetId: string) => {
    try {
      const next = await invoke<ResourcesStoreData>("delete_snippet", {
        snippetId,
      });
      onUpdateResources(next);
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const handleRunBatchSnippet = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!batchSnippet) return;
    setRunningBatch(true);
    setErrorBanner(null);
    try {
      const next = await invoke<ResourcesStoreData>("execute_snippet_batch", {
        snippetId: batchSnippet.id,
        machineIds: batchMachineIds,
        groupNames: batchGroupNames,
      });
      onUpdateResources(next);
      setBatchSnippet(null);
      setShowSnippetHistory(true);
    } catch (err) {
      setErrorBanner(String(err));
    } finally {
      setRunningBatch(false);
    }
  };

  const handleDockerAction = async (
    scope: string,
    action: string,
    target: string,
  ) => {
    setDockerLoading(true);
    setErrorBanner(null);
    try {
      const snap = await invoke<DockerFullSnapshot>("docker_action", {
        hostId: dockerHostId,
        scope,
        action,
        target,
      });
      setDockerSnap(snap);
    } catch (err) {
      setErrorBanner(String(err));
    } finally {
      setDockerLoading(false);
    }
  };

  const handleOpenDockerLogs = async (
    containerId: string,
    containerName: string,
  ) => {
    try {
      const logs = await invoke<string>("docker_logs", {
        hostId: dockerHostId,
        containerId,
        tail: 250,
      });
      setDockerLogsModal({ containerName, logs });
    } catch (err) {
      setErrorBanner(String(err));
    }
  };

  const navItems: {
    id: TabResources;
    label: string;
    count?: number;
    icon: React.ReactNode;
  }[] = [
    {
      id: "machines",
      label: "Machines",
      count: hosts.length,
      icon: <Server className="w-4 h-4" />,
    },
    {
      id: "identities",
      label: "Identities",
      count: resources.identities.length,
      icon: <UserCheck className="w-4 h-4" />,
    },
    {
      id: "keys",
      label: "SSH Keys",
      count: resources.sshKeys.length,
      icon: <KeyRound className="w-4 h-4" />,
    },
    {
      id: "connectionProxyProfiles",
      label: "Proxies",
      count: resources.proxyProfiles.length,
      icon: <Globe className="w-4 h-4" />,
    },
    {
      id: "portForwarding",
      label: "Port Forwarding",
      count: resources.portForwardingRules.length,
      icon: <Waypoints className="w-4 h-4" />,
    },
    {
      id: "snippets",
      label: "Snippets",
      count: resources.snippets.length,
      icon: <Code2 className="w-4 h-4" />,
    },
    {
      id: "docker",
      label: "Docker",
      icon: <Container className="w-4 h-4" />,
    },
  ];

  const gridClass =
    displayStyle === "grid"
      ? "grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-3.5"
      : "flex flex-col gap-2.5";

  return (
    <div className="flex h-full w-full bg-background text-foreground overflow-hidden">
      {/* Left Navigation Sidebar (hidden when unified sidebar is active) */}
      {!hideSidebar && (
        <div className="w-56 shrink-0 border-r border-border bg-card/60 flex flex-col justify-between p-3">
          <div className="space-y-1">
            <div className="px-3 py-1.5 text-[11px] font-semibold uppercase tracking-wider text-muted-foreground">
              Resource Center
            </div>
            {navItems.map((item) => {
              const active = activeSubTab === item.id;
              return (
                <button
                  key={item.id}
                  type="button"
                  onClick={() => setActiveSubTab(item.id)}
                  className={`w-full flex items-center justify-between px-3 py-2.5 rounded-xl text-[13px] font-medium transition-all cursor-pointer ${
                    active
                      ? "bg-primary/15 text-primary border border-primary/30 shadow-2xs"
                      : "text-muted-foreground hover:bg-accent hover:text-foreground border border-transparent"
                  }`}
                >
                  <span className="flex items-center gap-2.5">
                    {item.icon}
                    <span>{item.label}</span>
                  </span>
                  {typeof item.count === "number" && (
                    <span className="px-2 py-0.5 rounded-full bg-secondary text-[11px] font-medium text-muted-foreground">
                      {item.count}
                    </span>
                  )}
                </button>
              );
            })}
          </div>

          <div className="space-y-2">
            {onOpenLocalTerminal && (
              <button
                type="button"
                onClick={onOpenLocalTerminal}
                className="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-xl bg-secondary hover:bg-accent text-foreground border border-border text-xs font-medium transition-colors cursor-pointer"
              >
                <Laptop className="w-3.5 h-3.5 text-primary" />
                Open Local Terminal
              </button>
            )}

            <div className="p-3 rounded-xl bg-secondary/40 border border-border/70 text-[11px] text-muted-foreground space-y-1">
              <div className="font-semibold text-foreground flex items-center gap-1.5">
                <Shield className="w-3.5 h-3.5 text-primary" />
                Keychain Protected
              </div>
              <p className="text-[11px] leading-relaxed">
                Credentials and key passphrases are encrypted in your system
                Keychain.
              </p>
            </div>
          </div>
        </div>
      )}

      {/* Main Content Area */}
      <div className="flex-1 flex flex-col min-w-0 overflow-hidden">
        {/* Top Toolbar (Hidden when in dedicated SFTP dual-pane view) */}
        {activeSubTab !== "sftp" && (
        <div className="px-5 py-3 border-b border-border bg-card/40 flex flex-wrap items-center justify-between gap-3 shrink-0">
          <div className="flex items-center gap-2 flex-1 min-w-[220px] max-w-md">
            <div className="relative flex-1">
              <Search className="w-4 h-4 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                placeholder={`Search ${activeSubTab}...`}
                className="ui-input pl-9!"
              />
            </div>
          </div>

          <div className="flex items-center gap-2">
            {activeSubTab !== "docker" && (
              <>
                <button
                  type="button"
                  onClick={onRefreshHosts}
                  title="Reload Resources"
                  className="h-[38px] w-[38px] inline-flex items-center justify-center rounded-[10px] bg-secondary hover:bg-accent border border-border text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
                >
                  <RefreshCw className="w-4 h-4" />
                </button>

                <button
                  type="button"
                  onClick={() =>
                    setSortStyle((s) => (s === "name" ? "date" : "name"))
                  }
                  title="Sort by Name or Created Date"
                  className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] bg-secondary hover:bg-accent border border-border text-xs font-medium text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
                >
                  <ArrowUpDown className="w-3.5 h-3.5 text-primary" />
                  <span className="capitalize">{sortStyle}</span>
                </button>

                <div className="h-[38px] inline-flex items-center rounded-[10px] bg-secondary p-1 border border-border">
                  <button
                    type="button"
                    onClick={() => setDisplayStyle("grid")}
                    title="Grid View"
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
                    onClick={() => setDisplayStyle("list")}
                    title="List View"
                    className={`p-1.5 rounded-lg cursor-pointer transition-colors ${
                      displayStyle === "list"
                        ? "bg-card text-primary shadow-2xs"
                        : "text-muted-foreground hover:text-foreground"
                    }`}
                  >
                    <List className="w-3.5 h-3.5" />
                  </button>
                </div>
              </>
            )}

            {/* Context-specific primary buttons */}
            {activeSubTab === "machines" && (
              <>
                <button
                  type="button"
                  onClick={() => machineFileInputRef.current?.click()}
                  title="Import machines from a JSON file"
                  className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] bg-secondary hover:bg-accent border border-border text-xs font-medium text-foreground cursor-pointer transition-colors"
                >
                  <Upload className="w-3.5 h-3.5 text-primary" />
                  Import
                </button>
                <button
                  type="button"
                  onClick={() =>
                    openMachineTransfer(
                      "export",
                      // ~/.ssh/config hosts already live in a portable file.
                      hosts.filter((h) => h.source === "custom"),
                    )
                  }
                  title="Export machines to a JSON file"
                  className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] bg-secondary hover:bg-accent border border-border text-xs font-medium text-foreground cursor-pointer transition-colors"
                >
                  <Download className="w-3.5 h-3.5 text-primary" />
                  Export
                </button>
                <input
                  ref={machineFileInputRef}
                  type="file"
                  accept=".json,application/json"
                  className="hidden"
                  onChange={handlePickMachineFile}
                />
                <button
                  type="button"
                  onClick={() => setShowGroupModal(true)}
                  className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] bg-secondary hover:bg-accent border border-border text-xs font-medium text-foreground cursor-pointer transition-colors"
                >
                  <FolderGit2 className="w-3.5 h-3.5 text-primary" />
                  Groups
                </button>
                <button
                  type="button"
                  onClick={() => setShowTagModal(true)}
                  className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] bg-secondary hover:bg-accent border border-border text-xs font-medium text-foreground cursor-pointer transition-colors"
                >
                  <Tag className="w-3.5 h-3.5 text-primary" />
                  Tags
                </button>
                <button
                  type="button"
                  onClick={() => {
                    setEditingHost({
                      id: "",
                      alias: "",
                      hostname: "",
                      user: "root",
                      port: 22,
                      identityFile: null,
                      group:
                        selectedGroupFilter !== "ALL"
                          ? selectedGroupFilter
                          : "Production",
                      source: "custom",
                      remark: "",
                      tags: [],
                      authMethod: "passwordOrKey",
                      identityId: null,
                      sshKeyId: null,
                      proxyProfileId: null,
                      jumpHostId: null,
                      showStatus: true,
                      defaultSftpPath: "/",
                    });
                    setHostSecret("");
                  }}
                  className="h-[38px] inline-flex items-center gap-1.5 px-3.5 rounded-[10px] bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 cursor-pointer transition-opacity"
                >
                  <Plus className="w-4 h-4" />
                  New Machine
                </button>
              </>
            )}

            {activeSubTab === "identities" && (
              <button
                type="button"
                onClick={() => {
                  setEditingIdentity({
                    id: "",
                    name: "",
                    remark: "",
                    userName: "root",
                    sshKeyId: null,
                    createdDate: 0,
                  });
                  setIdentityPassword("");
                }}
                className="h-[38px] inline-flex items-center gap-1.5 px-3.5 rounded-[10px] bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 cursor-pointer"
              >
                <Plus className="w-4 h-4" />
                New Identity
              </button>
            )}

            {activeSubTab === "keys" && (
              <>
                <button
                  type="button"
                  onClick={() => setShowKeyImportModal(true)}
                  className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] bg-secondary hover:bg-accent border border-border text-xs font-medium text-foreground cursor-pointer"
                >
                  <FileKey className="w-3.5 h-3.5 text-primary" />
                  Import Key
                </button>
                <button
                  type="button"
                  onClick={() => setShowKeyGenerateModal(true)}
                  className="h-[38px] inline-flex items-center gap-1.5 px-3.5 rounded-[10px] bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 cursor-pointer"
                >
                  <Plus className="w-4 h-4" />
                  Generate Key
                </button>
              </>
            )}

            {activeSubTab === "connectionProxyProfiles" && (
              <button
                type="button"
                onClick={() => {
                  setEditingProxy({
                    id: "",
                    name: "",
                    remark: "",
                    proxyType: "socks5",
                    host: "127.0.0.1",
                    port: 1080,
                    authenticationMethod: "none",
                    username: "",
                    createdDate: 0,
                  });
                  setProxyPassword("");
                }}
                className="h-[38px] inline-flex items-center gap-1.5 px-3.5 rounded-[10px] bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 cursor-pointer"
              >
                <Plus className="w-4 h-4" />
                New Proxy
              </button>
            )}

            {activeSubTab === "portForwarding" &&
              resources.portForwardingRules.length > 0 && (
                <>
                  <button
                    type="button"
                    disabled={togglingPfId !== null}
                    onClick={() => handleToggleAllPfRules(true)}
                    className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] border bg-secondary hover:bg-accent border-border text-foreground text-xs font-medium cursor-pointer disabled:opacity-50"
                  >
                    <Play className="w-3.5 h-3.5" />
                    Start All
                  </button>
                  <button
                    type="button"
                    disabled={togglingPfId !== null}
                    onClick={() => handleToggleAllPfRules(false)}
                    className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] border bg-secondary hover:bg-accent border-border text-foreground text-xs font-medium cursor-pointer disabled:opacity-50"
                  >
                    <Square className="w-3.5 h-3.5" />
                    Stop All
                  </button>
                </>
              )}

            {activeSubTab === "portForwarding" && (
              <button
                type="button"
                onClick={() => {
                  setEditingPfRule({
                    id: "",
                    name: "",
                    remark: "",
                    mode: "local",
                    machineId: hosts[0]?.id ?? "",
                    localHost: "127.0.0.1",
                    localPort: 8080,
                    remoteHost: "127.0.0.1",
                    remotePort: 8080,
                    targetHost: "127.0.0.1",
                    targetPort: 80,
                    connectionProxyProfileId: null,
                    autoStart: false,
                    createdDate: 0,
                    status: "idle",
                  });
                }}
                className="h-[38px] inline-flex items-center gap-1.5 px-3.5 rounded-[10px] bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 cursor-pointer"
              >
                <Plus className="w-4 h-4" />
                New Forwarding Rule
              </button>
            )}

            {activeSubTab === "snippets" && (
              <>
                <button
                  type="button"
                  onClick={() => setShowSnippetHistory((p) => !p)}
                  className={`h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] border text-xs font-medium cursor-pointer ${
                    showSnippetHistory
                      ? "bg-primary/15 text-primary border-primary/35"
                      : "bg-secondary hover:bg-accent border-border text-foreground"
                  }`}
                >
                  <History className="w-3.5 h-3.5" />
                  History ({resources.snippetHistory.length})
                </button>
                <button
                  type="button"
                  onClick={() =>
                    setEditingPackage({
                      id: "",
                      name: "",
                      remark: "",
                      createdDate: 0,
                    })
                  }
                  className="h-[38px] inline-flex items-center gap-1.5 px-3 rounded-[10px] bg-secondary hover:bg-accent border border-border text-xs font-medium text-foreground cursor-pointer"
                >
                  <Package className="w-3.5 h-3.5 text-primary" />
                  New Package
                </button>
                <button
                  type="button"
                  onClick={() =>
                    setEditingSnippet({
                      id: "",
                      name: "",
                      remark: "",
                      code: "#!/usr/bin/env sh\nuptime\n",
                      packageId:
                        selectedPackageId !== "ALL" &&
                        selectedPackageId !== "UNGROUPED"
                          ? selectedPackageId
                          : null,
                      createdDate: 0,
                    })
                  }
                  className="h-[38px] inline-flex items-center gap-1.5 px-3.5 rounded-[10px] bg-primary text-primary-foreground text-xs font-semibold hover:opacity-90 cursor-pointer"
                >
                  <Plus className="w-4 h-4" />
                  New Snippet
                </button>
              </>
            )}
          </div>
        </div>
        )}

        {errorBanner && (
          <div className="mx-5 mt-3 px-3.5 py-2.5 rounded-xl bg-rose-500/15 border border-rose-500/35 text-rose-300 text-xs flex items-center justify-between">
            <span>{errorBanner}</span>
            <button
              type="button"
              onClick={() => setErrorBanner(null)}
              className="p-1 hover:bg-rose-500/20 rounded-lg cursor-pointer"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* Sub-tab Body */}
        <div className="flex-1 overflow-y-auto p-5 space-y-4">
          {/* 1. MACHINES TAB */}
          {activeSubTab === "machines" && (
            <div className="space-y-4">
              {/* Group & Tag Filter Bar */}
              <div className="flex flex-wrap items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => setSelectedGroupFilter("ALL")}
                  className={`px-3 py-1.5 rounded-full text-xs font-medium cursor-pointer border transition-colors ${
                    selectedGroupFilter === "ALL"
                      ? "bg-primary/15 text-primary border-primary/40"
                      : "bg-secondary/60 text-muted-foreground border-border hover:text-foreground"
                  }`}
                >
                  All Groups ({hosts.length})
                </button>
                {allGroupNames.map((gName) => (
                  <button
                    key={gName}
                    type="button"
                    onClick={() => setSelectedGroupFilter(gName)}
                    className={`px-3 py-1.5 rounded-full text-xs font-medium cursor-pointer border inline-flex items-center gap-1.5 transition-colors ${
                      selectedGroupFilter === gName
                        ? "bg-primary/15 text-primary border-primary/40"
                        : "bg-secondary/60 text-muted-foreground border-border hover:text-foreground"
                    }`}
                  >
                    <FolderGit2 className="w-3.5 h-3.5" />
                    {gName}
                  </button>
                ))}

                {resources.tags.length > 0 && (
                  <span className="mx-1 text-border">|</span>
                )}
                {resources.tags.map((t) => {
                  const active = selectedTagFilter === t.name;
                  return (
                    <button
                      key={t.id}
                      type="button"
                      onClick={() =>
                        setSelectedTagFilter(active ? "ALL" : t.name)
                      }
                      className={`px-2.5 py-1 rounded-full text-xs font-medium cursor-pointer border inline-flex items-center gap-1.5 transition-colors ${
                        active
                          ? "bg-secondary text-foreground border-primary"
                          : "bg-card text-muted-foreground border-border hover:text-foreground"
                      }`}
                    >
                      <span
                        className="w-2 h-2 rounded-full"
                        style={{ backgroundColor: t.color }}
                      />
                      {t.name}
                    </button>
                  );
                })}
              </div>

              <div className={gridClass}>
                {filteredMachines.map((h) => {
                  const identity = resources.identities.find(
                    (i) => i.id === h.identityId,
                  );
                  const sshKey = resources.sshKeys.find(
                    (k) => k.id === h.sshKeyId,
                  );
                  const proxy = resources.proxyProfiles.find(
                    (p) => p.id === h.proxyProfileId,
                  );
                  return (
                    <div
                      key={h.id}
                      className="p-4 rounded-2xl bg-card border border-border hover:border-primary/40 transition-all flex flex-col justify-between gap-3.5 shadow-2xs"
                    >
                      <div className="space-y-2">
                        <div className="flex items-start justify-between gap-2">
                          <div className="flex items-center gap-2.5 min-w-0">
                            <div className="p-2.5 rounded-xl bg-primary/12 text-primary shrink-0">
                              <Server className="w-4 h-4" />
                            </div>
                            <div className="min-w-0">
                              <div className="text-[13px] font-semibold text-foreground truncate">
                                {h.alias}
                              </div>
                              <div className="text-xs text-muted-foreground truncate">
                                {identity ? identity.userName : h.user}@
                                {h.hostname}:{h.port}
                              </div>
                            </div>
                          </div>
                          <span className="px-2.5 py-0.5 rounded-full bg-secondary text-[11px] text-muted-foreground font-medium shrink-0">
                            {h.group || "Default"}
                          </span>
                        </div>

                        {h.remark && (
                          <p className="text-xs text-muted-foreground line-clamp-2">
                            {h.remark}
                          </p>
                        )}

                        <div className="flex flex-wrap items-center gap-1.5 pt-1">
                          {identity && (
                            <span className="px-2 py-0.5 rounded-md bg-violet-500/15 text-violet-300 text-[11px] font-medium inline-flex items-center gap-1">
                              <UserCheck className="w-3 h-3" />
                              {identity.name}
                            </span>
                          )}
                          {sshKey && (
                            <span className="px-2 py-0.5 rounded-md bg-emerald-500/15 text-emerald-300 text-[11px] font-medium inline-flex items-center gap-1">
                              <KeyRound className="w-3 h-3" />
                              {sshKey.name}
                            </span>
                          )}
                          {proxy && (
                            <span className="px-2 py-0.5 rounded-md bg-sky-500/15 text-sky-300 text-[11px] font-medium inline-flex items-center gap-1">
                              <Globe className="w-3 h-3" />
                              {proxy.name}
                            </span>
                          )}
                          {(h.tags ?? []).map((tagName) => {
                            const tagObj = resources.tags.find(
                              (t) => t.name === tagName,
                            );
                            return (
                              <span
                                key={tagName}
                                className="px-2 py-0.5 rounded-full bg-secondary text-[11px] inline-flex items-center gap-1.5"
                              >
                                <span
                                  className="w-1.5 h-1.5 rounded-full"
                                  style={{
                                    backgroundColor:
                                      tagObj?.color ?? TAG_PALETTE[0],
                                  }}
                                />
                                {tagName}
                              </span>
                            );
                          })}
                        </div>
                      </div>

                      <div className="flex items-center justify-between gap-2 pt-2.5 border-t border-border/60">
                        <div className="flex items-center gap-1.5 min-w-0">
                          <button
                            type="button"
                            onClick={() => onConnectHost(h)}
                            className="h-8 inline-flex items-center gap-1.5 px-2.5 rounded-xl bg-primary text-primary-foreground text-xs font-semibold whitespace-nowrap shrink-0 hover:opacity-95 cursor-pointer transition-all shadow-2xs active:scale-97"
                          >
                            <Terminal className="w-3.5 h-3.5 shrink-0" />
                            <span>Terminal</span>
                          </button>
                          {onInspectMachine && (
                            <button
                              type="button"
                              onClick={() => onInspectMachine(h.id)}
                              className="h-8 inline-flex items-center gap-1.5 px-2.5 rounded-xl bg-secondary hover:bg-accent text-foreground border border-border/80 text-xs font-medium whitespace-nowrap shrink-0 cursor-pointer transition-all"
                              title="Open Full Machine Monitor"
                            >
                              <Activity className="w-3.5 h-3.5 text-emerald-400 shrink-0" />
                              <span>Monitor</span>
                            </button>
                          )}
                          <button
                            type="button"
                            onClick={() => {
                              setSftpTargetHostId(h.id);
                              setActiveSubTab("sftp");
                            }}
                            className="h-8 inline-flex items-center gap-1.5 px-2.5 rounded-xl bg-secondary hover:bg-accent text-foreground border border-border/80 text-xs font-medium whitespace-nowrap shrink-0 cursor-pointer transition-all"
                            title="Open SFTP File Manager"
                          >
                            <HardDrive className="w-3.5 h-3.5 text-amber-400 shrink-0" />
                            <span>SFTP</span>
                          </button>
                        </div>
                        <div className="flex items-center gap-1 shrink-0">
                          <button
                            type="button"
                            onClick={() => {
                              setEditingHost({
                                ...h,
                                authMethod: h.authMethod ?? "passwordOrKey",
                                tags: h.tags ?? [],
                              });
                              setHostSecret("");
                            }}
                            className="p-1.5 rounded-lg hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
                            title="Edit Machine"
                          >
                            <Edit3 className="w-3.5 h-3.5" />
                          </button>
                          {h.source === "custom" && (
                            <button
                              type="button"
                              onClick={() => onDeleteHost(h.id)}
                              className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer transition-colors"
                              title="Delete Machine"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          )}
                        </div>
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          )}

          {/* 2. IDENTITIES TAB */}
          {activeSubTab === "identities" && (
            <div className={gridClass}>
              {filteredIdentities.length === 0 ? (
                <div className="col-span-full py-12 text-center text-xs text-muted-foreground">
                  No saved identities yet. Click{" "}
                  <span className="text-primary font-semibold">
                    + New Identity
                  </span>{" "}
                  to create reusable credentials across multiple machines.
                </div>
              ) : (
                filteredIdentities.map((idItem) => {
                  const linkedKey = resources.sshKeys.find(
                    (k) => k.id === idItem.sshKeyId,
                  );
                  const usedByCount = hosts.filter(
                    (h) => h.identityId === idItem.id,
                  ).length;
                  return (
                    <div
                      key={idItem.id}
                      className="p-4 rounded-2xl bg-card border border-border hover:border-primary/40 transition-all flex flex-col justify-between gap-3"
                    >
                      <div className="space-y-2">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2.5">
                            <div className="p-2.5 rounded-xl bg-violet-500/15 text-violet-400">
                              <UserCheck className="w-4 h-4" />
                            </div>
                            <div>
                              <div className="text-[13px] font-semibold text-foreground">
                                {idItem.name}
                              </div>
                              <div className="text-xs text-muted-foreground">
                                User: {idItem.userName}
                              </div>
                            </div>
                          </div>
                          <span className="px-2.5 py-0.5 rounded-full bg-secondary text-[11px] font-medium text-muted-foreground">
                            {usedByCount} machines
                          </span>
                        </div>
                        {idItem.remark && (
                          <p className="text-xs text-muted-foreground">
                            {idItem.remark}
                          </p>
                        )}
                        <div className="flex flex-wrap gap-1.5 pt-1 text-[11px] font-medium">
                          {idItem.hasPassword && (
                            <span className="px-2 py-0.5 rounded-md bg-emerald-500/15 text-emerald-300">
                              Keychain Password
                            </span>
                          )}
                          {linkedKey && (
                            <span className="px-2 py-0.5 rounded-md bg-sky-500/15 text-sky-300">
                              Key: {linkedKey.name}
                            </span>
                          )}
                        </div>
                      </div>
                      <div className="flex items-center justify-between pt-2.5 border-t border-border/60 text-[11px] text-muted-foreground">
                        <span>{formatUnixDate(idItem.createdDate)}</span>
                        <div className="flex items-center gap-1">
                          <button
                            type="button"
                            onClick={() => {
                              setEditingIdentity(idItem);
                              setIdentityPassword("");
                            }}
                            className="p-1.5 rounded-lg hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer"
                          >
                            <Edit3 className="w-3.5 h-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={() => handleDeleteIdentity(idItem.id)}
                            className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          )}

          {/* 3. SSH KEYS TAB */}
          {activeSubTab === "keys" && (
            <div className={gridClass}>
              {filteredKeys.length === 0 ? (
                <div className="col-span-full py-12 text-center text-xs text-muted-foreground">
                  No SSH keys in vault. Click{" "}
                  <span className="text-primary font-semibold">
                    Generate Key
                  </span>{" "}
                  or{" "}
                  <span className="text-primary font-semibold">Import Key</span>{" "}
                  above.
                </div>
              ) : (
                filteredKeys.map((k) => (
                  <div
                    key={k.id}
                    className="p-4 rounded-2xl bg-card border border-border hover:border-primary/40 transition-all flex flex-col justify-between gap-3"
                  >
                    <div className="space-y-2">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2.5 min-w-0">
                          <div className="p-2.5 rounded-xl bg-emerald-500/15 text-emerald-400">
                            <KeyRound className="w-4 h-4" />
                          </div>
                          <div className="min-w-0">
                            <div className="text-[13px] font-semibold text-foreground truncate">
                              {k.name}
                            </div>
                            <div className="text-[11px] text-muted-foreground truncate">
                              {k.fingerprint}
                            </div>
                          </div>
                        </div>
                        <span className="px-2.5 py-0.5 rounded-full bg-secondary uppercase text-[10px] font-semibold text-primary">
                          {k.keyType}
                        </span>
                      </div>
                      {k.remark && (
                        <p className="text-xs text-muted-foreground">
                          {k.remark}
                        </p>
                      )}
                      {k.publicKey && (
                        <div className="p-2.5 rounded-xl bg-secondary/70 border border-border font-mono text-[11px] text-muted-foreground truncate">
                          {k.publicKey}
                        </div>
                      )}
                    </div>
                    <div className="flex items-center justify-between pt-2.5 border-t border-border/60 gap-2">
                      <div className="flex items-center gap-1.5 flex-wrap">
                        <button
                          type="button"
                          onClick={() => handleCopyPubKey(k)}
                          className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-secondary hover:bg-accent text-xs font-medium text-foreground cursor-pointer"
                        >
                          {copiedKeyId === k.id ? (
                            <>
                              <Check className="w-3.5 h-3.5 text-emerald-400" />
                              Copied
                            </>
                          ) : (
                            <>
                              <Copy className="w-3.5 h-3.5 text-primary" />
                              Copy Public Key
                            </>
                          )}
                        </button>
                        <button
                          type="button"
                          onClick={() =>
                            setExportKeyModal({
                              keyItem: k,
                              targetHostId: hosts[0]?.id ?? "",
                              password: "",
                              busy: false,
                            })
                          }
                          className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg bg-primary/15 hover:bg-primary/25 text-xs font-semibold text-primary border border-primary/30 cursor-pointer"
                        >
                          <Server className="w-3.5 h-3.5" />
                          <span>{t.exportKeyToHostBtn}</span>
                        </button>
                      </div>
                      <button
                        type="button"
                        onClick={() => handleDeleteKey(k.id)}
                        className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                        title="Delete SSH Key"
                      >
                        <Trash2 className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}

          {/* 3B. KNOWN HOSTS MANAGER TAB */}
          {activeSubTab === "knownHosts" && (
            <div className="space-y-4">
              <div className="p-4 rounded-2xl bg-card border border-border flex flex-wrap items-center justify-between gap-3">
                <div>
                  <h3 className="text-sm font-semibold text-foreground flex items-center gap-2">
                    <Shield className="w-4 h-4 text-teal-400" />
                    <span>{t.knownHostsTitle}</span>
                  </h3>
                  <p className="text-xs text-muted-foreground mt-0.5">
                    {t.knownHostsSubtitle}
                  </p>
                </div>
                <div className="flex items-center gap-2">
                  <button
                    type="button"
                    onClick={loadKnownHosts}
                    className="px-3 py-1.5 rounded-xl bg-secondary hover:bg-accent text-xs font-medium text-foreground inline-flex items-center gap-1.5 cursor-pointer"
                  >
                    <RefreshCw className="w-3.5 h-3.5" />
                    <span>{t.sftpRefresh}</span>
                  </button>
                  {knownHosts.length > 0 && (
                    <button
                      type="button"
                      onClick={async () => {
                        try {
                          const next =
                            await invoke<KnownHostEntry[]>("clear_known_hosts");
                          setKnownHosts(next);
                        } catch (err) {
                          setErrorBanner(String(err));
                        }
                      }}
                      className="px-3 py-1.5 rounded-xl bg-rose-500/15 hover:bg-rose-500/25 text-xs font-semibold text-rose-300 border border-rose-500/30 inline-flex items-center gap-1.5 cursor-pointer"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                      <span>{t.knownHostsClearAll}</span>
                    </button>
                  )}
                </div>
              </div>

              {knownHosts.length === 0 ? (
                <div className="py-12 text-center text-xs text-muted-foreground rounded-2xl bg-card/50 border border-border/60">
                  {t.knownHostsEmpty}
                </div>
              ) : (
                <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
                  {knownHosts
                    .filter((kh) => {
                      const q = searchQuery.trim().toLowerCase();
                      if (!q) return true;
                      return (
                        kh.hostPattern.toLowerCase().includes(q) ||
                        kh.keyType.toLowerCase().includes(q) ||
                        kh.fingerprint.toLowerCase().includes(q)
                      );
                    })
                    .map((kh) => (
                      <div
                        key={kh.id}
                        className="p-4 rounded-2xl bg-card border border-border hover:border-teal-500/40 transition-all flex items-center justify-between gap-3"
                      >
                        <div className="min-w-0 space-y-1">
                          <div className="flex items-center gap-2">
                            <span className="text-xs font-bold font-mono text-foreground truncate">
                              {kh.hostPattern}
                            </span>
                            <span className="px-2 py-0.5 rounded-full bg-teal-500/15 text-teal-300 border border-teal-500/30 font-mono text-[10px]">
                              {kh.keyType}
                            </span>
                          </div>
                          <div className="text-[11px] font-mono text-emerald-400 truncate">
                            {kh.fingerprint}
                          </div>
                        </div>
                        <div className="flex items-center gap-1 shrink-0">
                          <button
                            type="button"
                            onClick={() => {
                              navigator.clipboard
                                .writeText(kh.fingerprint)
                                .catch(() => undefined);
                            }}
                            title="Copy SHA-256 Fingerprint"
                            className="p-1.5 rounded-lg hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer"
                          >
                            <Copy className="w-3.5 h-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={async () => {
                              try {
                                const next = await invoke<KnownHostEntry[]>(
                                  "delete_known_host",
                                  {
                                    hostPattern: kh.hostPattern,
                                    keyType: kh.keyType,
                                  },
                                );
                                setKnownHosts(next);
                              } catch (err) {
                                setErrorBanner(String(err));
                              }
                            }}
                            title={t.knownHostsDelete}
                            className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    ))}
                </div>
              )}
            </div>
          )}

          {/* 4. PROXIES TAB */}
          {activeSubTab === "connectionProxyProfiles" && (
            <div className={gridClass}>
              {filteredProxies.length === 0 ? (
                <div className="col-span-full py-12 text-center text-xs text-muted-foreground">
                  No connection proxy profiles configured. Click{" "}
                  <span className="text-primary font-semibold">
                    + New Proxy
                  </span>{" "}
                  to add SOCKS5 or HTTP Connect proxies.
                </div>
              ) : (
                filteredProxies.map((p) => (
                  <div
                    key={p.id}
                    className="p-4 rounded-2xl bg-card border border-border hover:border-primary/40 transition-all flex flex-col justify-between gap-3"
                  >
                    <div className="space-y-1.5">
                      <div className="flex items-center justify-between">
                        <div className="flex items-center gap-2.5">
                          <div className="p-2.5 rounded-xl bg-sky-500/15 text-sky-400">
                            <Globe className="w-4 h-4" />
                          </div>
                          <div>
                            <div className="text-[13px] font-semibold text-foreground">
                              {p.name}
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {p.host}:{p.port}
                            </div>
                          </div>
                        </div>
                        <span className="px-2.5 py-0.5 rounded-full bg-secondary text-[10px] font-semibold uppercase text-sky-400">
                          {p.proxyType}
                        </span>
                      </div>
                      {p.remark && (
                        <p className="text-xs text-muted-foreground">
                          {p.remark}
                        </p>
                      )}
                    </div>
                    <div className="flex items-center justify-between pt-2.5 border-t border-border/60 text-xs text-muted-foreground">
                      <span>
                        Auth:{" "}
                        {p.authenticationMethod === "usernamePassword"
                          ? p.username
                          : "None"}
                      </span>
                      <div className="flex items-center gap-1">
                        <button
                          type="button"
                          onClick={() => {
                            setEditingProxy(p);
                            setProxyPassword("");
                          }}
                          className="p-1.5 rounded-lg hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer"
                        >
                          <Edit3 className="w-3.5 h-3.5" />
                        </button>
                        <button
                          type="button"
                          onClick={() => handleDeleteProxy(p.id)}
                          className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                        >
                          <Trash2 className="w-3.5 h-3.5" />
                        </button>
                      </div>
                    </div>
                  </div>
                ))
              )}
            </div>
          )}

          {/* 5. PORT FORWARDING TAB */}
          {activeSubTab === "portForwarding" && (
            <div className={gridClass}>
              {filteredPfRules.length === 0 ? (
                <div className="col-span-full py-12 text-center text-xs text-muted-foreground">
                  No port forwarding rules yet. Click{" "}
                  <span className="text-primary font-semibold">
                    + New Forwarding Rule
                  </span>{" "}
                  to create Local (-L), Dynamic SOCKS5 (-D), or Remote (-R)
                  tunnels.
                </div>
              ) : (
                filteredPfRules.map((rule) => {
                  const machine = hosts.find((h) => h.id === rule.machineId);
                  const isRunning = rule.status === "running";
                  return (
                    <div
                      key={rule.id}
                      className="p-4 rounded-2xl bg-card border border-border hover:border-primary/40 transition-all flex flex-col justify-between gap-3"
                    >
                      <div className="space-y-2.5">
                        <div className="flex items-center justify-between">
                          <div className="flex items-center gap-2.5">
                            <div
                              className={`p-2.5 rounded-xl ${
                                isRunning
                                  ? "bg-emerald-500/15 text-emerald-400"
                                  : "bg-secondary text-muted-foreground"
                              }`}
                            >
                              <Waypoints className="w-4 h-4" />
                            </div>
                            <div>
                              <div className="text-[13px] font-semibold text-foreground">
                                {rule.name}
                              </div>
                              <div className="text-xs text-muted-foreground">
                                via {machine?.alias ?? rule.machineId}
                              </div>
                            </div>
                          </div>
                          <span
                            className={`px-2.5 py-0.5 rounded-full text-[10px] font-semibold uppercase ${
                              isRunning
                                ? "bg-emerald-500/15 text-emerald-300 border border-emerald-500/30"
                                : rule.status === "failed"
                                  ? "bg-rose-500/15 text-rose-300"
                                  : "bg-secondary text-muted-foreground"
                            }`}
                          >
                            {rule.mode} • {rule.status}
                          </span>
                        </div>

                        <div className="p-2.5 rounded-xl bg-secondary/60 border border-border text-xs text-foreground">
                          {rule.mode === "dynamic" ? (
                            <span>
                              SOCKS5 {rule.localHost}:{rule.localPort} →{" "}
                              {machine?.alias ?? "SSH"}
                            </span>
                          ) : rule.mode === "remote" ? (
                            <span>
                              {machine?.alias ?? "SSH"} {rule.remoteHost}:
                              {rule.remotePort} → {rule.targetHost}:
                              {rule.targetPort}
                            </span>
                          ) : (
                            <span>
                              {rule.localHost}:{rule.localPort} →{" "}
                              {rule.targetHost}:{rule.targetPort}
                            </span>
                          )}
                        </div>

                        {rule.errorMessage && (
                          <div className="text-xs text-rose-400">
                            {rule.errorMessage}
                          </div>
                        )}
                      </div>

                      <div className="flex items-center justify-between pt-2.5 border-t border-border/60">
                        <button
                          type="button"
                          disabled={togglingPfId === rule.id}
                          onClick={() => handleTogglePfRule(rule)}
                          className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-semibold cursor-pointer ${
                            isRunning
                              ? "bg-rose-500/15 text-rose-300 hover:bg-rose-500/25"
                              : "bg-emerald-500/15 text-emerald-300 hover:bg-emerald-500/25"
                          }`}
                        >
                          {isRunning ? (
                            <>
                              <Square className="w-3 h-3" />
                              Stop Tunnel
                            </>
                          ) : (
                            <>
                              <Play className="w-3 h-3" />
                              Start Tunnel
                            </>
                          )}
                        </button>

                        <div className="flex items-center gap-1">
                          <button
                            type="button"
                            onClick={() => setEditingPfRule(rule)}
                            className="p-1.5 rounded-lg hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer"
                          >
                            <Edit3 className="w-3.5 h-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={() => handleDeletePfRule(rule.id)}
                            className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>
                  );
                })
              )}
            </div>
          )}

          {/* 6. SNIPPETS & PACKAGES TAB */}
          {activeSubTab === "snippets" && (
            <div className="space-y-4">
              {/* Package Filter Pills */}
              <div className="flex flex-wrap items-center gap-1.5">
                <button
                  type="button"
                  onClick={() => setSelectedPackageId("ALL")}
                  className={`px-3 py-1.5 rounded-full text-xs font-medium cursor-pointer border ${
                    selectedPackageId === "ALL"
                      ? "bg-primary/15 text-primary border-primary/40"
                      : "bg-secondary/60 text-muted-foreground border-border"
                  }`}
                >
                  All Snippets ({resources.snippets.length})
                </button>
                <button
                  type="button"
                  onClick={() => setSelectedPackageId("UNGROUPED")}
                  className={`px-3 py-1.5 rounded-full text-xs font-medium cursor-pointer border ${
                    selectedPackageId === "UNGROUPED"
                      ? "bg-primary/15 text-primary border-primary/40"
                      : "bg-secondary/60 text-muted-foreground border-border"
                  }`}
                >
                  Ungrouped
                </button>
                {resources.snippetPackages.map((pkg) => (
                  <div
                    key={pkg.id}
                    className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-full text-xs font-medium border ${
                      selectedPackageId === pkg.id
                        ? "bg-primary/15 text-primary border-primary/40"
                        : "bg-secondary/60 text-muted-foreground border-border"
                    }`}
                  >
                    <button
                      type="button"
                      onClick={() => setSelectedPackageId(pkg.id)}
                      className="inline-flex items-center gap-1.5 cursor-pointer"
                    >
                      <Package className="w-3.5 h-3.5" />
                      {pkg.name}
                    </button>
                    <button
                      type="button"
                      onClick={() => handleDeleteSnippetPackage(pkg.id)}
                      title="Delete Package (keeps snippets)"
                      className="ml-1 hover:text-rose-400 cursor-pointer"
                    >
                      <X className="w-3 h-3" />
                    </button>
                  </div>
                ))}
              </div>

              {showSnippetHistory ? (
                <div className="space-y-2.5">
                  <div className="text-xs font-semibold text-foreground flex items-center gap-2">
                    <History className="w-4 h-4 text-primary" />
                    Snippet Execution History
                  </div>
                  {resources.snippetHistory.length === 0 ? (
                    <div className="py-8 text-center text-xs text-muted-foreground">
                      No snippet execution history recorded yet.
                    </div>
                  ) : (
                    resources.snippetHistory.map((rec) => (
                      <div
                        key={rec.id}
                        className="p-4 rounded-2xl bg-card border border-border space-y-2"
                      >
                        <div className="flex items-center justify-between text-xs">
                          <div className="font-semibold text-foreground">
                            {rec.snippetName}{" "}
                            <span className="text-muted-foreground font-normal">
                              on
                            </span>{" "}
                            <span className="text-primary">
                              {rec.machineName}
                            </span>
                          </div>
                          <div className="flex items-center gap-2">
                            <span
                              className={`px-2 py-0.5 rounded-full text-[10px] font-semibold uppercase ${
                                rec.status === "finished"
                                  ? "bg-emerald-500/15 text-emerald-300"
                                  : "bg-rose-500/15 text-rose-300"
                              }`}
                            >
                              {rec.status}
                            </span>
                            <span className="text-[11px] text-muted-foreground">
                              {formatUnixDate(rec.executionDate)}
                            </span>
                          </div>
                        </div>
                        <pre className="p-3 rounded-xl bg-secondary/70 border border-border text-xs font-mono text-foreground overflow-x-auto max-h-40">
                          {rec.result}
                        </pre>
                      </div>
                    ))
                  )}
                </div>
              ) : (
                <div className={gridClass}>
                  {filteredSnippets.map((snip) => {
                    const pkg = resources.snippetPackages.find(
                      (p) => p.id === snip.packageId,
                    );
                    return (
                      <div
                        key={snip.id}
                        className="p-4 rounded-2xl bg-card border border-border hover:border-primary/40 transition-all flex flex-col justify-between gap-3"
                      >
                        <div className="space-y-2">
                          <div className="flex items-center justify-between">
                            <div className="text-[13px] font-semibold text-foreground">
                              {snip.name}
                            </div>
                            {pkg && (
                              <span className="px-2.5 py-0.5 rounded-full bg-secondary text-[11px] font-medium text-primary">
                                {pkg.name}
                              </span>
                            )}
                          </div>
                          {snip.remark && (
                            <p className="text-xs text-muted-foreground">
                              {snip.remark}
                            </p>
                          )}
                          <pre className="p-3 rounded-xl bg-secondary/70 border border-border text-xs font-mono text-foreground overflow-x-auto max-h-28">
                            {snip.code}
                          </pre>
                        </div>

                        <div className="flex items-center justify-between pt-2.5 border-t border-border/60">
                          <div className="flex items-center gap-1.5">
                            {onRunSnippetInActivePane && (
                              <button
                                type="button"
                                onClick={() =>
                                  onRunSnippetInActivePane(snip.code)
                                }
                                className="inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-xl bg-secondary hover:bg-accent text-xs font-medium text-foreground cursor-pointer"
                                title="Send to Active Terminal"
                              >
                                <Terminal className="w-3.5 h-3.5 text-primary" />
                                Send to Shell
                              </button>
                            )}
                            <button
                              type="button"
                              onClick={() => {
                                setBatchSnippet(snip);
                                setBatchMachineIds(["local"]);
                                setBatchGroupNames([]);
                              }}
                              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-primary/15 hover:bg-primary/25 text-xs text-primary font-semibold cursor-pointer"
                            >
                              <Play className="w-3.5 h-3.5" />
                              Batch Run
                            </button>
                          </div>

                          <div className="flex items-center gap-1">
                            <button
                              type="button"
                              onClick={() => setEditingSnippet(snip)}
                              className="p-1.5 rounded-lg hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer"
                            >
                              <Edit3 className="w-3.5 h-3.5" />
                            </button>
                            <button
                              type="button"
                              onClick={() => handleDeleteSnippet(snip.id)}
                              className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          </div>
                        </div>
                      </div>
                    );
                  })}
                </div>
              )}
            </div>
          )}

          {/* 7. DOCKER PANEL TAB */}
          {activeSubTab === "docker" && (
            <div className="space-y-4">
              {/* Target Host Selector + Docker Sub-Tabs */}
              <div className="flex flex-wrap items-center justify-between gap-3 p-3.5 rounded-2xl bg-card border border-border">
                <div className="flex items-center gap-2.5">
                  <Container className="w-4 h-4 text-sky-400" />
                  <span className="text-xs font-medium text-muted-foreground">
                    Target Host:
                  </span>
                  <div className="w-64">
                    <SelectField
                      value={dockerHostId}
                      onChange={setDockerHostId}
                      options={[
                        {
                          value: "local",
                          label: "Local Machine (localhost)",
                        },
                        ...hosts.map((h) => ({
                          value: h.id,
                          label: `${h.alias} (${h.user}@${h.hostname})`,
                        })),
                      ]}
                    />
                  </div>
                  <button
                    type="button"
                    onClick={() => loadDockerSnapshot(dockerHostId)}
                    className="h-[38px] w-[38px] inline-flex items-center justify-center rounded-[10px] bg-secondary hover:bg-accent border border-border text-muted-foreground hover:text-foreground cursor-pointer"
                    title="Refresh Docker Status"
                  >
                    <RefreshCw
                      className={`w-4 h-4 ${
                        dockerLoading ? "animate-spin text-primary" : ""
                      }`}
                    />
                  </button>
                </div>

                <div className="flex items-center gap-1">
                  {(
                    [
                      ["overview", "Overview", Layers],
                      ["containers", "Containers", Box],
                      ["compose", "Compose", Package],
                      ["images", "Images", HardDrive],
                      ["volumes", "Volumes", HardDrive],
                      ["networks", "Networks", Network],
                    ] as const
                  ).map(([id, label, Icon]) => (
                    <button
                      key={id}
                      type="button"
                      onClick={() => setDockerTab(id)}
                      className={`inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl text-xs font-medium cursor-pointer transition-colors ${
                        dockerTab === id
                          ? "bg-primary/15 text-primary border border-primary/35"
                          : "text-muted-foreground hover:bg-secondary hover:text-foreground"
                      }`}
                    >
                      <Icon className="w-3.5 h-3.5" />
                      {label}
                    </button>
                  ))}
                </div>
              </div>

              {!dockerSnap || !dockerSnap.installed ? (
                <div className="p-12 rounded-2xl bg-card border border-border text-center text-xs text-muted-foreground">
                  {dockerLoading
                    ? "Querying Docker engine..."
                    : "Docker is not installed or daemon is not running on this machine."}
                </div>
              ) : (
                <>
                  {dockerTab === "overview" && (
                    <div className="space-y-4">
                      {/* Engine Connected Banner */}
                      <div className="p-4 rounded-2xl bg-card border border-border flex flex-wrap items-center justify-between gap-4">
                        <div className="flex items-center gap-3">
                          <div className="w-10 h-10 rounded-xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400">
                            <Container className="w-5 h-5" />
                          </div>
                          <div>
                            <div className="text-sm font-bold text-foreground flex items-center gap-2">
                              <span>Docker Engine Connected</span>
                              <span className="px-2 py-0.5 rounded-full bg-emerald-500/15 text-emerald-400 text-[10px] font-mono">
                                v{dockerSnap.serverVersion}
                              </span>
                            </div>
                            <div className="text-xs text-muted-foreground mt-0.5">
                              {dockerSnap.os} • {dockerSnap.arch}
                            </div>
                          </div>
                        </div>
                      </div>

                      {/* 4 Summary Metric Cards */}
                      <div className="grid grid-cols-2 md:grid-cols-4 gap-3.5">
                        <div className="p-4 rounded-2xl bg-card border border-border flex items-center justify-between">
                          <div>
                            <div className="text-2xl font-bold font-mono text-foreground">
                              {dockerSnap.totalContainers}
                            </div>
                            <div className="text-xs text-muted-foreground mt-0.5">
                              Containers
                            </div>
                          </div>
                          <div className="p-2.5 rounded-xl bg-sky-500/15 text-sky-400">
                            <Box className="w-5 h-5" />
                          </div>
                        </div>

                        <div className="p-4 rounded-2xl bg-card border border-border flex items-center justify-between">
                          <div>
                            <div className="text-2xl font-bold font-mono text-emerald-400">
                              {dockerSnap.runningContainers}
                            </div>
                            <div className="text-xs text-muted-foreground mt-0.5">
                              Running
                            </div>
                          </div>
                          <div className="p-2.5 rounded-xl bg-emerald-500/15 text-emerald-400">
                            <Play className="w-5 h-5" />
                          </div>
                        </div>

                        <div className="p-4 rounded-2xl bg-card border border-border flex items-center justify-between">
                          <div>
                            <div className="text-2xl font-bold font-mono text-violet-400">
                              {dockerSnap.images.length}
                            </div>
                            <div className="text-xs text-muted-foreground mt-0.5">
                              Images
                            </div>
                          </div>
                          <div className="p-2.5 rounded-xl bg-violet-500/15 text-violet-400">
                            <Layers className="w-5 h-5" />
                          </div>
                        </div>

                        <div className="p-4 rounded-2xl bg-card border border-border flex items-center justify-between">
                          <div>
                            <div className="text-2xl font-bold font-mono text-amber-400">
                              {dockerSnap.volumes.length}
                            </div>
                            <div className="text-xs text-muted-foreground mt-0.5">
                              Volumes
                            </div>
                          </div>
                          <div className="p-2.5 rounded-xl bg-amber-500/15 text-amber-400">
                            <HardDrive className="w-5 h-5" />
                          </div>
                        </div>
                      </div>

                      {/* Container State Bar */}
                      <div className="p-5 rounded-2xl bg-card border border-border space-y-3">
                        <div className="text-xs font-semibold text-foreground">
                          Container State
                        </div>
                        <div className="w-full h-3 rounded-full bg-secondary overflow-hidden flex">
                          {dockerSnap.totalContainers > 0 && (
                            <>
                              <div
                                className="h-full bg-emerald-500 transition-all"
                                style={{
                                  width: `${(dockerSnap.runningContainers / dockerSnap.totalContainers) * 100}%`,
                                }}
                              />
                              <div
                                className="h-full bg-amber-400 transition-all"
                                style={{
                                  width: `${(dockerSnap.pausedContainers / dockerSnap.totalContainers) * 100}%`,
                                }}
                              />
                              <div
                                className="h-full bg-rose-500 transition-all"
                                style={{
                                  width: `${(dockerSnap.stoppedContainers / dockerSnap.totalContainers) * 100}%`,
                                }}
                              />
                            </>
                          )}
                        </div>
                        <div className="flex items-center gap-6 text-xs text-muted-foreground">
                          <span className="inline-flex items-center gap-1.5">
                            <span className="w-2.5 h-2.5 rounded-full bg-emerald-500" />
                            Running: {dockerSnap.runningContainers}
                          </span>
                          <span className="inline-flex items-center gap-1.5">
                            <span className="w-2.5 h-2.5 rounded-full bg-rose-500" />
                            Stopped: {dockerSnap.stoppedContainers}
                          </span>
                          <span className="inline-flex items-center gap-1.5">
                            <span className="w-2.5 h-2.5 rounded-full bg-amber-400" />
                            Paused: {dockerSnap.pausedContainers}
                          </span>
                        </div>
                      </div>
                    </div>
                  )}

                  {dockerTab === "containers" && (
                    <div className="space-y-4">
                      {/* State Filter Pills */}
                      <div className="flex items-center gap-2">
                        {(
                          [
                            ["all", `All (${dockerSnap.containers.length})`],
                            [
                              "running",
                              `Running (${dockerSnap.runningContainers})`,
                            ],
                            [
                              "stopped",
                              `Stopped (${dockerSnap.stoppedContainers})`,
                            ],
                            [
                              "paused",
                              `Paused (${dockerSnap.pausedContainers})`,
                            ],
                          ] as const
                        ).map(([st, label]) => (
                          <button
                            key={st}
                            type="button"
                            onClick={() => setDockerStateFilter(st)}
                            className={`px-3.5 py-1.5 rounded-full text-xs font-semibold transition-all cursor-pointer ${
                              dockerStateFilter === st
                                ? "bg-primary text-primary-foreground shadow-2xs"
                                : "bg-secondary/80 text-muted-foreground hover:text-foreground"
                            }`}
                          >
                            {label}
                          </button>
                        ))}
                      </div>

                      <div className="grid grid-cols-1 md:grid-cols-2 xl:grid-cols-3 gap-4">
                        {dockerSnap.containers
                          .filter((c) => {
                            if (dockerStateFilter === "all") return true;
                            if (dockerStateFilter === "running")
                              return c.state === "running";
                            if (dockerStateFilter === "paused")
                              return c.state === "paused";
                            return (
                              c.state !== "running" && c.state !== "paused"
                            );
                          })
                          .map((c) => {
                            const running = c.state === "running";
                            const cpuNum = Math.min(
                              100,
                              Math.max(
                                0,
                                parseFloat(
                                  (c.cpuPerc || "0").replace(/[^0-9.]/g, ""),
                                ) || 0,
                              ),
                            );
                            const memNum = Math.min(
                              100,
                              Math.max(
                                0,
                                parseFloat(
                                  (c.memPerc || "0").replace(/[^0-9.]/g, ""),
                                ) || 0,
                              ),
                            );
                            const netParts = (c.netIo || "0B / 0B").split("/");
                            const blockParts = (c.blockIo || "0B / 0B").split(
                              "/",
                            );
                            const r = 24;
                            const circ = 2 * Math.PI * r;

                            return (
                              <div
                                key={c.id}
                                className="p-4 rounded-2xl bg-card border border-border hover:border-primary/40 flex flex-col justify-between gap-4 transition-all shadow-2xs"
                              >
                                <div className="space-y-2">
                                  <div className="flex items-start justify-between gap-2">
                                    <div className="min-w-0">
                                      <div className="flex items-center gap-2">
                                        <span
                                          className={`w-2.5 h-2.5 rounded-full shrink-0 ${
                                            running
                                              ? "bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.75)]"
                                              : "bg-zinc-500"
                                          }`}
                                        />
                                        <span className="text-sm font-bold text-foreground truncate">
                                          {c.name}
                                        </span>
                                      </div>
                                      <div className="text-[11px] text-muted-foreground font-mono truncate mt-0.5 pl-4">
                                        {c.image}
                                      </div>
                                    </div>
                                    <span className="px-2 py-0.5 rounded-full bg-secondary text-[10px] font-mono text-muted-foreground shrink-0">
                                      {c.status}
                                    </span>
                                  </div>

                                  {c.ports && (
                                    <div className="flex flex-wrap gap-1 pl-4">
                                      {c.ports
                                        .split(",")
                                        .slice(0, 3)
                                        .map((p, idx) => (
                                          <span
                                            key={idx}
                                            className="px-2 py-0.5 rounded-md bg-primary/10 border border-primary/25 text-primary font-mono text-[10px]"
                                          >
                                            {p.trim()}
                                          </span>
                                        ))}
                                    </div>
                                  )}
                                </div>

                                {/* Circular CPU & RAM Gauges */}
                                <div className="flex items-center justify-around py-2 border-y border-border/60">
                                  <div className="flex flex-col items-center gap-1">
                                    <div className="relative w-14 h-14 flex items-center justify-center">
                                      <svg
                                        width={56}
                                        height={56}
                                        className="-rotate-90"
                                      >
                                        <circle
                                          cx={28}
                                          cy={28}
                                          r={r}
                                          fill="transparent"
                                          stroke="currentColor"
                                          strokeWidth={5}
                                          className="text-secondary"
                                        />
                                        <circle
                                          cx={28}
                                          cy={28}
                                          r={r}
                                          fill="transparent"
                                          stroke="#22c55e"
                                          strokeWidth={5}
                                          strokeDasharray={circ}
                                          strokeDashoffset={
                                            circ - (cpuNum / 100) * circ
                                          }
                                          strokeLinecap="round"
                                        />
                                      </svg>
                                      <span className="absolute text-[10px] font-bold font-mono">
                                        {Math.round(cpuNum)}%
                                      </span>
                                    </div>
                                    <span className="text-[10px] text-muted-foreground">
                                      CPU
                                    </span>
                                  </div>

                                  <div className="flex flex-col items-center gap-1">
                                    <div className="relative w-14 h-14 flex items-center justify-center">
                                      <svg
                                        width={56}
                                        height={56}
                                        className="-rotate-90"
                                      >
                                        <circle
                                          cx={28}
                                          cy={28}
                                          r={r}
                                          fill="transparent"
                                          stroke="currentColor"
                                          strokeWidth={5}
                                          className="text-secondary"
                                        />
                                        <circle
                                          cx={28}
                                          cy={28}
                                          r={r}
                                          fill="transparent"
                                          stroke="#38bdf8"
                                          strokeWidth={5}
                                          strokeDasharray={circ}
                                          strokeDashoffset={
                                            circ - (memNum / 100) * circ
                                          }
                                          strokeLinecap="round"
                                        />
                                      </svg>
                                      <span className="absolute text-[10px] font-bold font-mono">
                                        {Math.round(memNum)}%
                                      </span>
                                    </div>
                                    <span className="text-[10px] text-muted-foreground">
                                      RAM
                                    </span>
                                  </div>
                                </div>

                                <div className="grid grid-cols-2 gap-2 text-[11px] font-mono">
                                  <div>
                                    <div className="text-muted-foreground font-sans">
                                      Network
                                    </div>
                                    <div className="text-emerald-400">
                                      ↑ {(netParts[0] || "0B").trim()}
                                    </div>
                                    <div className="text-sky-400">
                                      ↓ {(netParts[1] || "0B").trim()}
                                    </div>
                                  </div>
                                  <div className="text-right">
                                    <div className="text-muted-foreground font-sans">
                                      Block IO
                                    </div>
                                    <div className="text-foreground">
                                      R {(blockParts[0] || "0B").trim()}
                                    </div>
                                    <div className="text-foreground">
                                      W {(blockParts[1] || "0B").trim()}
                                    </div>
                                  </div>
                                </div>

                                <div className="flex items-center justify-between pt-2 border-t border-border/60">
                                  <div className="flex items-center gap-1.5">
                                    {running ? (
                                      <button
                                        type="button"
                                        onClick={() =>
                                          handleDockerAction(
                                            "container",
                                            "stop",
                                            c.id,
                                          )
                                        }
                                        className="px-3 py-1.5 rounded-xl bg-amber-500/15 hover:bg-amber-500/25 text-amber-300 text-xs font-semibold cursor-pointer transition-colors"
                                      >
                                        Stop
                                      </button>
                                    ) : (
                                      <button
                                        type="button"
                                        onClick={() =>
                                          handleDockerAction(
                                            "container",
                                            "start",
                                            c.id,
                                          )
                                        }
                                        className="px-3 py-1.5 rounded-xl bg-emerald-500/15 hover:bg-emerald-500/25 text-emerald-300 text-xs font-semibold cursor-pointer transition-colors"
                                      >
                                        Start
                                      </button>
                                    )}
                                    <button
                                      type="button"
                                      onClick={() =>
                                        handleDockerAction(
                                          "container",
                                          "restart",
                                          c.id,
                                        )
                                      }
                                      title="Restart Container"
                                      className="p-1.5 rounded-xl bg-secondary hover:bg-accent text-foreground cursor-pointer"
                                    >
                                      <RotateCw className="w-3.5 h-3.5" />
                                    </button>
                                    <button
                                      type="button"
                                      onClick={() =>
                                        handleOpenDockerLogs(c.id, c.name)
                                      }
                                      className="px-2.5 py-1.5 rounded-xl bg-secondary hover:bg-accent text-foreground text-xs font-medium cursor-pointer"
                                    >
                                      Logs
                                    </button>
                                    {/* Local shells are hidden on mobile, so is this */}
                                    {running &&
                                      onOpenCommandTab &&
                                      (dockerHostId !== "local" ||
                                        onOpenLocalTerminal) && (
                                        <button
                                          type="button"
                                          onClick={() =>
                                            onOpenCommandTab(
                                              dockerHostId,
                                              c.name,
                                              // c.id is the hex --no-trunc ID, safe unquoted
                                              `docker exec -it ${c.id} sh -c "command -v bash >/dev/null && exec bash || exec sh"`,
                                            )
                                          }
                                          title="Open Shell in Container"
                                          className="px-2.5 py-1.5 rounded-xl bg-secondary hover:bg-accent text-foreground text-xs font-medium cursor-pointer inline-flex items-center gap-1"
                                        >
                                          <Terminal className="w-3.5 h-3.5" />
                                          Shell
                                        </button>
                                      )}
                                  </div>
                                  <button
                                    type="button"
                                    onClick={() =>
                                      handleDockerAction(
                                        "container",
                                        "rm",
                                        c.id,
                                      )
                                    }
                                    title="Remove Container"
                                    className="p-1.5 rounded-xl hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                                  >
                                    <Trash2 className="w-3.5 h-3.5" />
                                  </button>
                                </div>
                              </div>
                            );
                          })}
                      </div>
                    </div>
                  )}

                  {dockerTab === "compose" && (
                    <div className="space-y-2.5">
                      {dockerSnap.composeProjects.map((proj) => (
                        <div
                          key={proj.name}
                          className="p-3.5 rounded-2xl bg-card border border-border flex items-center justify-between"
                        >
                          <div>
                            <div className="text-[13px] font-semibold text-foreground">
                              {proj.name}
                            </div>
                            <div className="text-xs text-muted-foreground">
                              {proj.status} • {proj.configFiles}
                            </div>
                          </div>
                          <div className="flex items-center gap-1.5">
                            <button
                              type="button"
                              onClick={() =>
                                handleDockerAction("compose", "start", proj.name)
                              }
                              className="px-3 py-1 rounded-lg bg-emerald-500/15 text-emerald-300 text-xs font-medium cursor-pointer"
                            >
                              Start
                            </button>
                            <button
                              type="button"
                              onClick={() =>
                                handleDockerAction(
                                  "compose",
                                  "restart",
                                  proj.name,
                                )
                              }
                              className="px-3 py-1 rounded-lg bg-secondary text-foreground text-xs font-medium cursor-pointer"
                            >
                              Restart
                            </button>
                            <button
                              type="button"
                              onClick={() =>
                                handleDockerAction("compose", "stop", proj.name)
                              }
                              className="px-3 py-1 rounded-lg bg-rose-500/15 text-rose-300 text-xs font-medium cursor-pointer"
                            >
                              Stop
                            </button>
                          </div>
                        </div>
                      ))}
                    </div>
                  )}

                  {dockerTab === "images" && (
                    <div className="space-y-2.5">
                      {dockerSnap.images.map((img) => (
                        <div
                          key={`${img.id}-${img.repository}-${img.tag}`}
                          className="p-3.5 rounded-2xl bg-card border border-border flex items-center justify-between text-xs"
                        >
                          <div>
                            <div className="font-semibold text-foreground">
                              {img.repository}:{img.tag}
                            </div>
                            <div className="text-[11px] text-muted-foreground">
                              {img.id.slice(0, 12)} • {img.size} •{" "}
                              {img.createdSince}
                            </div>
                          </div>
                          <button
                            type="button"
                            onClick={() =>
                              handleDockerAction("image", "rm", img.id)
                            }
                            className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      ))}
                    </div>
                  )}

                  {dockerTab === "volumes" && (
                    <div className="space-y-2.5">
                      <div className="flex justify-end">
                        <button
                          type="button"
                          onClick={() =>
                            handleDockerAction("volume", "prune", "")
                          }
                          className="px-3 py-1.5 rounded-xl bg-rose-500/15 text-rose-300 text-xs font-medium cursor-pointer"
                        >
                          Prune Unused Volumes
                        </button>
                      </div>
                      {dockerSnap.volumes.map((v) => (
                        <div
                          key={v.name}
                          className="p-3.5 rounded-2xl bg-card border border-border flex items-center justify-between text-xs"
                        >
                          <div>
                            <div className="font-semibold text-foreground">
                              {v.name}
                            </div>
                            <div className="text-[11px] text-muted-foreground">
                              {v.driver} • {v.mountpoint}
                            </div>
                          </div>
                          <button
                            type="button"
                            onClick={() =>
                              handleDockerAction("volume", "rm", v.name)
                            }
                            className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      ))}
                    </div>
                  )}

                  {dockerTab === "networks" && (
                    <div className="space-y-2.5">
                      {dockerSnap.networks.map((n) => (
                        <div
                          key={n.id}
                          className="p-3.5 rounded-2xl bg-card border border-border flex items-center justify-between text-xs"
                        >
                          <div>
                            <div className="font-semibold text-foreground">
                              {n.name}
                            </div>
                            <div className="text-[11px] text-muted-foreground">
                              {n.id.slice(0, 12)} • driver: {n.driver} • scope:{" "}
                              {n.scope}
                            </div>
                          </div>
                          {!["bridge", "host", "none"].includes(n.name) && (
                            <button
                              type="button"
                              onClick={() =>
                                handleDockerAction("network", "rm", n.id)
                              }
                              className="p-1.5 rounded-lg hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
                            >
                              <Trash2 className="w-3.5 h-3.5" />
                            </button>
                          )}
                        </div>
                      ))}
                    </div>
                  )}
                </>
              )}
            </div>
          )}

          {/* 8. SFTP DUAL-PANE FILE MANAGER TAB */}
          {activeSubTab === "sftp" && (
            <div className="h-full min-h-[520px] flex flex-col">
              <SftpWorkspace
                hosts={hosts}
                initialRemoteHostId={sftpTargetHostId || activeHostId}
                language={appSettings?.language}
                defaultFastSync={appSettings?.fastSyncEnabled}
              />
            </div>
          )}
        </div>
      </div>

      {/* === MODALS === */}

      {/* Machine Create/Edit Modal (Grouped macOS Form Rows) */}
      {editingHost && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-lg rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-6 py-4 border-b border-border/80 flex items-center justify-between">
              <h3 className="text-base font-bold text-foreground">
                {editingHost.id ? "Edit Machine" : "Create New Machine"}
              </h3>
              <button
                type="button"
                onClick={() => setEditingHost(null)}
                className="p-1.5 rounded-xl hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer transition-colors"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleSaveHostForm}
              className="p-6 space-y-5 text-xs max-h-[82vh] overflow-y-auto"
            >
              {/* 1. Machine Information Group */}
              <div className="space-y-1.5">
                <div className="text-xs font-semibold text-foreground px-1">
                  Machine Information
                </div>
                <div className="rounded-2xl border border-border/80 bg-card/70 divide-y divide-border/60">
                  <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                    <span className="text-[13px] font-medium text-foreground shrink-0">
                      Name
                    </span>
                    <input
                      type="text"
                      value={editingHost.alias}
                      onChange={(e) =>
                        setEditingHost({
                          ...editingHost,
                          alias: e.target.value,
                        })
                      }
                      placeholder="prod-node-01"
                      className="w-60 px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-right text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
                    />
                  </div>

                  <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                    <span className="text-[13px] font-medium text-foreground shrink-0">
                      Host
                    </span>
                    <input
                      type="text"
                      required
                      value={editingHost.hostname}
                      onChange={(e) =>
                        setEditingHost({
                          ...editingHost,
                          hostname: e.target.value,
                        })
                      }
                      placeholder="10.0.1.25 or example.com"
                      className="w-60 px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-right text-[13px] font-mono text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
                    />
                  </div>

                  <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                    <span className="text-[13px] font-medium text-foreground shrink-0">
                      Port
                    </span>
                    <input
                      type="number"
                      value={editingHost.port}
                      onChange={(e) =>
                        setEditingHost({
                          ...editingHost,
                          port: Number.parseInt(e.target.value, 10) || 22,
                        })
                      }
                      className="w-28 px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                    />
                  </div>

                  <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                    <span className="text-[13px] font-medium text-foreground shrink-0">
                      Group
                    </span>
                    <input
                      type="text"
                      list="machine-groups-list"
                      value={editingHost.group}
                      onChange={(e) =>
                        setEditingHost({
                          ...editingHost,
                          group: e.target.value,
                        })
                      }
                      placeholder="Production"
                      className="w-60 px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-right text-[13px] text-foreground focus:outline-none focus:border-primary"
                    />
                    <datalist id="machine-groups-list">
                      {allGroupNames.map((g) => (
                        <option key={g} value={g} />
                      ))}
                    </datalist>
                  </div>

                  <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                    <span className="text-[13px] font-medium text-foreground shrink-0">
                      Default SFTP Path
                    </span>
                    <input
                      type="text"
                      value={editingHost.defaultSftpPath ?? "/"}
                      onChange={(e) =>
                        setEditingHost({
                          ...editingHost,
                          defaultSftpPath: e.target.value,
                        })
                      }
                      placeholder="/ or ~"
                      className="w-60 px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                    />
                  </div>
                </div>
              </div>

              {/* 2. Jump Host Group */}
              <div className="space-y-1.5">
                <div className="rounded-2xl border border-border/80 bg-card/70 divide-y divide-border/60">
                  <div className="flex items-center justify-between px-4 py-3">
                    <span className="text-[13px] font-medium text-foreground">
                      Jump Host
                    </span>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(editingHost.jumpHostId)}
                      onClick={() => {
                        const candidates = hosts.filter(
                          (h) => h.id !== editingHost.id,
                        );
                        setEditingHost({
                          ...editingHost,
                          jumpHostId: editingHost.jumpHostId
                            ? null
                            : (candidates[0]?.id ?? null),
                        });
                      }}
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ${
                        editingHost.jumpHostId ? "bg-primary" : "bg-secondary"
                      }`}
                    >
                      <span
                        className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md transition duration-200 ${
                          editingHost.jumpHostId
                            ? "translate-x-5"
                            : "translate-x-0"
                        }`}
                      />
                    </button>
                  </div>

                  {editingHost.jumpHostId !== null &&
                    editingHost.jumpHostId !== undefined && (
                      <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                        <span className="text-[13px] font-medium text-muted-foreground">
                          Bastion Machine
                        </span>
                        <div className="w-60">
                          <SelectField
                            value={editingHost.jumpHostId ?? ""}
                            onChange={(val) =>
                              setEditingHost({
                                ...editingHost,
                                jumpHostId: val || null,
                              })
                            }
                            options={[
                              { value: "", label: "None" },
                              ...hosts
                                .filter((h) => h.id !== editingHost.id)
                                .map((h) => ({
                                  value: h.id,
                                  label: `${h.alias} (${h.hostname})`,
                                })),
                            ]}
                          />
                        </div>
                      </div>
                    )}
                </div>
              </div>

              {/* 3. Connection Proxy Group */}
              <div className="space-y-1.5">
                <div className="text-xs font-semibold text-foreground px-1">
                  Connection Proxy
                </div>
                <div className="rounded-2xl border border-border/80 bg-card/70">
                  <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                    <span className="text-[13px] font-medium text-foreground">
                      Proxy
                    </span>
                    <div className="w-60">
                      <SelectField
                        value={editingHost.proxyProfileId ?? ""}
                        onChange={(val) =>
                          setEditingHost({
                            ...editingHost,
                            proxyProfileId: val || null,
                          })
                        }
                        options={[
                          { value: "", label: "None (Direct)" },
                          ...resources.proxyProfiles.map((p) => ({
                            value: p.id,
                            label: `${p.name} (${p.proxyType})`,
                          })),
                        ]}
                      />
                    </div>
                  </div>
                </div>
              </div>

              {/* 4. Authentication Group */}
              <div className="space-y-1.5">
                <div className="text-xs font-semibold text-foreground px-1">
                  Authentication
                </div>
                <div className="rounded-2xl border border-border/80 bg-card/70 divide-y divide-border/60">
                  <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                    <span className="text-[13px] font-medium text-foreground">
                      Method
                    </span>
                    <div className="w-60">
                      <SelectField
                        value={editingHost.authMethod ?? "passwordOrKey"}
                        onChange={(val) =>
                          setEditingHost({
                            ...editingHost,
                            authMethod: val as MachineAuthMethod,
                          })
                        }
                        options={[
                          { value: "identity", label: "Identity" },
                          {
                            value: "passwordOrKey",
                            label: "Password / SSH Key",
                          },
                          {
                            value: "keyboardInteractive",
                            label: "Keyboard Interactive",
                          },
                        ]}
                      />
                    </div>
                  </div>

                  {editingHost.authMethod === "identity" ? (
                    <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                      <span className="text-[13px] font-medium text-foreground">
                        Identity
                      </span>
                      <div className="w-60">
                        <SelectField
                          value={editingHost.identityId ?? ""}
                          onChange={(val) =>
                            setEditingHost({
                              ...editingHost,
                              identityId: val || null,
                            })
                          }
                          placeholder="Select Identity..."
                          options={[
                            { value: "", label: "None" },
                            ...resources.identities.map((idItem) => ({
                              value: idItem.id,
                              label: `${idItem.name} (${idItem.userName})`,
                            })),
                          ]}
                        />
                      </div>
                    </div>
                  ) : (
                    <>
                      <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                        <span className="text-[13px] font-medium text-foreground">
                          Username
                        </span>
                        <input
                          type="text"
                          value={editingHost.user}
                          onChange={(e) =>
                            setEditingHost({
                              ...editingHost,
                              user: e.target.value,
                            })
                          }
                          placeholder="root"
                          className="w-60 px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                        />
                      </div>

                      <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                        <span className="text-[13px] font-medium text-foreground">
                          SSH Key
                        </span>
                        <div className="w-60">
                          <SelectField
                            value={editingHost.sshKeyId ?? ""}
                            onChange={(val) =>
                              setEditingHost({
                                ...editingHost,
                                sshKeyId: val || null,
                              })
                            }
                            options={[
                              { value: "", label: "Default (~/.ssh)" },
                              ...resources.sshKeys.map((k) => ({
                                value: k.id,
                                label: `${k.name} (${k.keyType})`,
                              })),
                            ]}
                          />
                        </div>
                      </div>

                      <div className="flex items-center justify-between px-4 py-2.5 gap-4">
                        <span className="text-[13px] font-medium text-foreground">
                          Password
                        </span>
                        <input
                          type="password"
                          value={hostSecret}
                          onChange={(e) => setHostSecret(e.target.value)}
                          placeholder="Stored in Keychain"
                          className="w-60 px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-right text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
                        />
                      </div>
                    </>
                  )}
                </div>
              </div>

              {/* 5. Advanced SSH Security & Startup Automation (Agent Forwarding, Startup Command, Env Vars) */}
              <div className="space-y-1.5">
                <div className="text-xs font-semibold text-foreground px-1">
                  SSH Automation & Agent Forwarding
                </div>
                <div className="rounded-2xl border border-border/80 bg-card/70 divide-y divide-border/60">
                  <div className="flex items-center justify-between px-4 py-3 gap-4">
                    <div>
                      <div className="text-[13px] font-medium text-foreground">
                        {t.hostAgentForwarding}
                      </div>
                      <div className="text-[11px] text-muted-foreground">
                        {t.hostAgentForwardingDesc}
                      </div>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={Boolean(editingHost.agentForwarding)}
                      onClick={() =>
                        setEditingHost({
                          ...editingHost,
                          agentForwarding: !editingHost.agentForwarding,
                        })
                      }
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ${
                        editingHost.agentForwarding
                          ? "bg-primary"
                          : "bg-secondary"
                      }`}
                    >
                      <span
                        className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md transition duration-200 ${
                          editingHost.agentForwarding
                            ? "translate-x-5"
                            : "translate-x-0"
                        }`}
                      />
                    </button>
                  </div>

                  <div className="px-4 py-2.5 space-y-1.5">
                    <div className="text-[12px] font-medium text-foreground">
                      {t.hostStartupCommand}
                    </div>
                    <input
                      type="text"
                      value={editingHost.startupCommand ?? ""}
                      onChange={(e) =>
                        setEditingHost({
                          ...editingHost,
                          startupCommand: e.target.value,
                        })
                      }
                      placeholder={t.hostStartupCommandPlaceholder}
                      className="w-full px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
                    />
                  </div>

                  <div className="px-4 py-2.5 space-y-1.5">
                    <div className="text-[12px] font-medium text-foreground">
                      {t.hostEnvVars}
                    </div>
                    <textarea
                      rows={2}
                      value={editingHost.envVars ?? ""}
                      onChange={(e) =>
                        setEditingHost({
                          ...editingHost,
                          envVars: e.target.value,
                        })
                      }
                      placeholder={t.hostEnvVarsPlaceholder}
                      className="w-full px-3 py-1.5 rounded-xl bg-secondary/75 border border-border/80 text-xs font-mono text-foreground focus:outline-none focus:border-primary resize-none"
                    />
                  </div>
                </div>
              </div>

              {resources.tags.length > 0 && (
                <div className="space-y-1.5">
                  <div className="text-xs font-semibold text-foreground px-1">
                    Color Tags
                  </div>
                  <div className="flex flex-wrap gap-1.5 px-1">
                    {resources.tags.map((t) => {
                      const checked = (editingHost.tags ?? []).includes(t.name);
                      return (
                        <button
                          key={t.id}
                          type="button"
                          onClick={() => {
                            const cur = editingHost.tags ?? [];
                            const nextTags = checked
                              ? cur.filter((x) => x !== t.name)
                              : [...cur, t.name];
                            setEditingHost({ ...editingHost, tags: nextTags });
                          }}
                          className={`px-3 py-1.5 rounded-full text-xs font-medium border inline-flex items-center gap-1.5 cursor-pointer transition-all ${
                            checked
                              ? "bg-primary/15 text-primary border-primary"
                              : "bg-secondary text-muted-foreground border-border"
                          }`}
                        >
                          <span
                            className="w-2 h-2 rounded-full"
                            style={{ backgroundColor: t.color }}
                          />
                          {t.name}
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}

              <div className="flex items-center justify-end gap-2.5 pt-3 border-t border-border/80">
                {hostTestResult && (
                  <span
                    className={`mr-auto min-w-0 text-[11px] font-mono break-all ${
                      hostTestResult.ok ? "text-emerald-400" : "text-rose-400"
                    }`}
                  >
                    {hostTestResult.message}
                  </span>
                )}
                <button
                  type="button"
                  onClick={handleTestHostConnection}
                  disabled={testingHost || !editingHost.hostname.trim()}
                  className="h-10 px-4 rounded-xl bg-secondary hover:bg-accent text-foreground text-xs font-semibold cursor-pointer transition-colors inline-flex items-center gap-1.5 shrink-0 disabled:opacity-50 disabled:cursor-not-allowed"
                >
                  {testingHost ? (
                    <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                  ) : (
                    <Activity className="w-3.5 h-3.5" />
                  )}
                  Test connection
                </button>
                <button
                  type="button"
                  onClick={() => setEditingHost(null)}
                  className="h-10 px-5 rounded-xl bg-secondary hover:bg-accent text-foreground text-xs font-semibold cursor-pointer transition-colors"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-10 px-5 rounded-xl bg-primary hover:opacity-95 text-primary-foreground text-xs font-semibold cursor-pointer transition-opacity shadow-xs"
                >
                  {editingHost.id ? "Save Changes" : "Create"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Groups Management Modal */}
      {machineTransfer && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">
                {machineTransfer.mode === "export"
                  ? "Export Machines"
                  : "Import Machines"}
              </h3>
              <button
                type="button"
                onClick={() => setMachineTransfer(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-5 space-y-3.5 text-xs">
              <label className="flex items-center gap-2.5 cursor-pointer font-medium">
                <input
                  type="checkbox"
                  checked={machineTransfer.selected.every(Boolean)}
                  onChange={(e) =>
                    setMachineTransfer((prev) =>
                      prev && {
                        ...prev,
                        selected: prev.selected.map(() => e.target.checked),
                      },
                    )
                  }
                />
                <span>
                  Select all ({machineTransfer.selected.filter(Boolean).length}/
                  {machineTransfer.items.length})
                </span>
              </label>
              <div className="space-y-1.5 max-h-64 overflow-y-auto p-3 rounded-xl bg-input border border-border">
                {machineTransfer.items.length === 0 ? (
                  <div className="text-muted-foreground">No machines.</div>
                ) : (
                  machineTransfer.items.map((m, i) => (
                    <label
                      key={i}
                      className="flex items-center gap-2.5 cursor-pointer py-1"
                    >
                      <input
                        type="checkbox"
                        checked={machineTransfer.selected[i]}
                        onChange={(e) =>
                          setMachineTransfer((prev) =>
                            prev && {
                              ...prev,
                              selected: prev.selected.map((v, j) =>
                                j === i ? e.target.checked : v,
                              ),
                            },
                          )
                        }
                      />
                      <span className="min-w-0 truncate">
                        {m.alias}{" "}
                        <span className="text-muted-foreground">
                          {m.user}@{m.hostname}:{m.port}
                        </span>
                      </span>
                      {machineTransfer.mode === "import" &&
                        hosts.some((h) => h.id === m.id) && (
                          <span className="ml-auto shrink-0 text-[10px] text-amber-400">
                            update
                          </span>
                        )}
                    </label>
                  ))
                )}
              </div>
              <p className="text-muted-foreground">
                Identity and SSH key links are not included.
              </p>
              <button
                type="button"
                disabled={!machineTransfer.selected.some(Boolean)}
                onClick={handleConfirmMachineTransfer}
                className="w-full h-[38px] rounded-[10px] bg-primary text-primary-foreground font-semibold cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {machineTransfer.mode === "export" ? "Export" : "Import"} (
                {machineTransfer.selected.filter(Boolean).length})
              </button>
            </div>
          </div>
        </div>
      )}

      {showGroupModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">Machine Groups</h3>
              <button
                type="button"
                onClick={() => setShowGroupModal(false)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-5 space-y-3.5 text-xs">
              <form onSubmit={handleSaveGroup} className="flex gap-2">
                <input
                  type="text"
                  required
                  value={groupForm.name}
                  onChange={(e) =>
                    setGroupForm({ ...groupForm, name: e.target.value })
                  }
                  placeholder="New group name..."
                  className="ui-input flex-1"
                />
                <button
                  type="submit"
                  className="h-[38px] px-4 rounded-[10px] bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Add
                </button>
              </form>
              <div className="space-y-1.5 max-h-56 overflow-y-auto">
                {resources.groups.map((g) => (
                  <div
                    key={g.id}
                    className="flex items-center justify-between px-3 py-2 rounded-xl bg-secondary/50"
                  >
                    <span className="font-medium">{g.name}</span>
                    <button
                      type="button"
                      onClick={() => handleDeleteGroup(g.id)}
                      className="text-muted-foreground hover:text-rose-400 cursor-pointer"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Color Tags Modal */}
      {showTagModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">Machine Color Tags</h3>
              <button
                type="button"
                onClick={() => setShowTagModal(false)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-5 space-y-3.5 text-xs">
              <form onSubmit={handleSaveTag} className="space-y-2.5">
                <div className="flex gap-2">
                  <input
                    type="text"
                    required
                    value={tagForm.name}
                    onChange={(e) =>
                      setTagForm({ ...tagForm, name: e.target.value })
                    }
                    placeholder="Tag name (e.g. k8s, gpu, prod)..."
                    className="ui-input flex-1"
                  />
                  <button
                    type="submit"
                    className="h-[38px] px-4 rounded-[10px] bg-primary text-primary-foreground font-semibold cursor-pointer"
                  >
                    Add Tag
                  </button>
                </div>
                <div className="flex items-center gap-2">
                  {TAG_PALETTE.map((c) => (
                    <button
                      key={c}
                      type="button"
                      onClick={() => setTagForm({ ...tagForm, color: c })}
                      className={`w-5 h-5 rounded-full cursor-pointer border-2 ${
                        tagForm.color === c
                          ? "border-white scale-110"
                          : "border-transparent"
                      }`}
                      style={{ backgroundColor: c }}
                    />
                  ))}
                </div>
              </form>
              <div className="space-y-1.5 max-h-56 overflow-y-auto">
                {resources.tags.map((t) => (
                  <div
                    key={t.id}
                    className="flex items-center justify-between px-3 py-2 rounded-xl bg-secondary/50"
                  >
                    <span className="inline-flex items-center gap-2">
                      <span
                        className="w-2.5 h-2.5 rounded-full"
                        style={{ backgroundColor: t.color }}
                      />
                      {t.name}
                    </span>
                    <button
                      type="button"
                      onClick={() => handleDeleteTag(t.id)}
                      className="text-muted-foreground hover:text-rose-400 cursor-pointer"
                    >
                      <Trash2 className="w-3.5 h-3.5" />
                    </button>
                  </div>
                ))}
              </div>
            </div>
          </div>
        </div>
      )}

      {/* Identity Modal */}
      {editingIdentity && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">
                {editingIdentity.id ? "Edit Identity" : "New Identity"}
              </h3>
              <button
                type="button"
                onClick={() => setEditingIdentity(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleSaveIdentityForm}
              className="p-5 space-y-4 text-xs"
            >
              <div>
                <label className="ui-label">Identity Name *</label>
                <input
                  type="text"
                  required
                  value={editingIdentity.name}
                  onChange={(e) =>
                    setEditingIdentity({
                      ...editingIdentity,
                      name: e.target.value,
                    })
                  }
                  placeholder="Production Root / Deploy User"
                  className="ui-input"
                />
              </div>
              <div>
                <label className="ui-label">SSH Username *</label>
                <input
                  type="text"
                  required
                  value={editingIdentity.userName}
                  onChange={(e) =>
                    setEditingIdentity({
                      ...editingIdentity,
                      userName: e.target.value,
                    })
                  }
                  className="ui-input"
                />
              </div>
              <div>
                <label className="ui-label">Linked SSH Key (Optional)</label>
                <SelectField
                  value={editingIdentity.sshKeyId ?? ""}
                  onChange={(val) =>
                    setEditingIdentity({
                      ...editingIdentity,
                      sshKeyId: val || null,
                    })
                  }
                  options={[
                    { value: "", label: "None" },
                    ...resources.sshKeys.map((k) => ({
                      value: k.id,
                      label: `${k.name} (${k.keyType})`,
                    })),
                  ]}
                />
              </div>
              <div>
                <label className="ui-label">Password (Stored in Keychain)</label>
                <input
                  type="password"
                  value={identityPassword}
                  onChange={(e) => setIdentityPassword(e.target.value)}
                  placeholder={
                    editingIdentity.hasPassword
                      ? "Leave blank to keep existing password"
                      : "Optional password"
                  }
                  className="ui-input"
                />
              </div>
              <div>
                <label className="ui-label">Remark</label>
                <input
                  type="text"
                  value={editingIdentity.remark}
                  onChange={(e) =>
                    setEditingIdentity({
                      ...editingIdentity,
                      remark: e.target.value,
                    })
                  }
                  className="ui-input"
                />
              </div>
              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setEditingIdentity(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Save Identity
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Generate SSH Key Modal */}
      {showKeyGenerateModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">Generate New SSH Key</h3>
              <button
                type="button"
                onClick={() => setShowKeyGenerateModal(false)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleGenerateKey}
              className="p-5 space-y-4 text-xs"
            >
              <div>
                <label className="ui-label">Key Name *</label>
                <input
                  type="text"
                  required
                  value={keyGenName}
                  onChange={(e) => setKeyGenName(e.target.value)}
                  placeholder="macbook-ed25519"
                  className="ui-input"
                />
              </div>
              <div>
                <label className="ui-label">Algorithm</label>
                <SelectField
                  value={keyGenType}
                  onChange={(val) =>
                    setKeyGenType(val as "ed25519" | "rsa" | "ecdsa")
                  }
                  options={[
                    { value: "ed25519", label: "Ed25519 (Recommended)" },
                    { value: "rsa", label: "RSA 4096-bit" },
                    { value: "ecdsa", label: "ECDSA" },
                  ]}
                />
              </div>
              <div>
                <label className="ui-label">Passphrase (Optional)</label>
                <input
                  type="password"
                  value={keyGenPassphrase}
                  onChange={(e) => setKeyGenPassphrase(e.target.value)}
                  placeholder="Optional passphrase"
                  className="ui-input"
                />
              </div>
              <div>
                <label className="ui-label">Remark</label>
                <input
                  type="text"
                  value={keyGenRemark}
                  onChange={(e) => setKeyGenRemark(e.target.value)}
                  placeholder="Optional description"
                  className="ui-input"
                />
              </div>
              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setShowKeyGenerateModal(false)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Generate
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Import SSH Key Modal */}
      {showKeyImportModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-lg rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">Import SSH Key</h3>
              <button
                type="button"
                onClick={() => setShowKeyImportModal(false)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form onSubmit={handleImportKey} className="p-5 space-y-4 text-xs">
              <div>
                <label className="ui-label">Key Name *</label>
                <input
                  type="text"
                  required
                  value={keyImportName}
                  onChange={(e) => setKeyImportName(e.target.value)}
                  placeholder="prod-cluster-key"
                  className="ui-input"
                />
              </div>
              <div>
                <label className="ui-label">
                  Private Key (PEM / OpenSSH) *
                </label>
                <textarea
                  required
                  rows={5}
                  value={keyImportPrivate}
                  onChange={(e) => setKeyImportPrivate(e.target.value)}
                  placeholder="-----BEGIN OPENSSH PRIVATE KEY-----..."
                  className="ui-textarea font-mono"
                />
              </div>
              <div>
                <label className="ui-label">
                  Public Key (Optional — auto-derived if omitted)
                </label>
                <input
                  type="text"
                  value={keyImportPublic}
                  onChange={(e) => setKeyImportPublic(e.target.value)}
                  placeholder="ssh-ed25519 AAAA..."
                  className="ui-input font-mono"
                />
              </div>
              <div className="grid grid-cols-2 gap-3.5">
                <div>
                  <label className="ui-label">Passphrase (If encrypted)</label>
                  <input
                    type="password"
                    value={keyImportPassphrase}
                    onChange={(e) => setKeyImportPassphrase(e.target.value)}
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">Remark</label>
                  <input
                    type="text"
                    value={keyImportRemark}
                    onChange={(e) => setKeyImportRemark(e.target.value)}
                    className="ui-input"
                  />
                </div>
              </div>
              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setShowKeyImportModal(false)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Import Key
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Proxy Profile Modal */}
      {editingProxy && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">
                {editingProxy.id ? "Edit Proxy Profile" : "New Proxy Profile"}
              </h3>
              <button
                type="button"
                onClick={() => setEditingProxy(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleSaveProxyForm}
              className="p-5 space-y-4 text-xs"
            >
              <div className="grid grid-cols-2 gap-3.5">
                <div>
                  <label className="ui-label">Name *</label>
                  <input
                    type="text"
                    required
                    value={editingProxy.name}
                    onChange={(e) =>
                      setEditingProxy({ ...editingProxy, name: e.target.value })
                    }
                    placeholder="Corp SOCKS5"
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">Proxy Type</label>
                  <SelectField
                    value={editingProxy.proxyType}
                    onChange={(val) =>
                      setEditingProxy({
                        ...editingProxy,
                        proxyType: val as "socks5" | "httpConnect",
                      })
                    }
                    options={[
                      { value: "socks5", label: "SOCKS5" },
                      { value: "httpConnect", label: "HTTP Connect" },
                    ]}
                  />
                </div>
              </div>
              <div className="grid grid-cols-3 gap-3.5">
                <div className="col-span-2">
                  <label className="ui-label">Host *</label>
                  <input
                    type="text"
                    required
                    value={editingProxy.host}
                    onChange={(e) =>
                      setEditingProxy({ ...editingProxy, host: e.target.value })
                    }
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">Port *</label>
                  <input
                    type="number"
                    required
                    value={editingProxy.port}
                    onChange={(e) =>
                      setEditingProxy({
                        ...editingProxy,
                        port: Number.parseInt(e.target.value, 10) || 1080,
                      })
                    }
                    className="ui-input"
                  />
                </div>
              </div>
              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setEditingProxy(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Save Proxy
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Port Forwarding Rule Modal */}
      {editingPfRule && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">Port Forwarding Rule</h3>
              <button
                type="button"
                onClick={() => setEditingPfRule(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form onSubmit={handleSavePfForm} className="p-5 space-y-4 text-xs">
              <div className="grid grid-cols-2 gap-3.5">
                <div>
                  <label className="ui-label">Rule Name *</label>
                  <input
                    type="text"
                    required
                    value={editingPfRule.name}
                    onChange={(e) =>
                      setEditingPfRule({
                        ...editingPfRule,
                        name: e.target.value,
                      })
                    }
                    placeholder="Postgres Tunnel"
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">Mode</label>
                  <SelectField
                    value={editingPfRule.mode}
                    onChange={(val) =>
                      setEditingPfRule({
                        ...editingPfRule,
                        mode: val as "local" | "dynamic" | "remote",
                      })
                    }
                    options={[
                      { value: "local", label: "Local Forwarding (-L)" },
                      { value: "dynamic", label: "Dynamic SOCKS5 (-D)" },
                      { value: "remote", label: "Remote Forwarding (-R)" },
                    ]}
                  />
                </div>
              </div>
              <div>
                <label className="ui-label">SSH Machine *</label>
                <SelectField
                  value={editingPfRule.machineId}
                  onChange={(val) =>
                    setEditingPfRule({
                      ...editingPfRule,
                      machineId: val,
                    })
                  }
                  placeholder="Select Machine..."
                  options={hosts.map((h) => ({
                    value: h.id,
                    label: `${h.alias} (${h.user}@${h.hostname})`,
                  }))}
                />
              </div>
              <div className="grid grid-cols-2 gap-3.5">
                {/* -R binds on the SSH server, -L/-D bind on this machine */}
                <div>
                  <label className="ui-label">
                    {editingPfRule.mode === "remote"
                      ? "Remote Bind Host"
                      : "Local Bind Host"}
                  </label>
                  <input
                    type="text"
                    value={
                      editingPfRule.mode === "remote"
                        ? editingPfRule.remoteHost
                        : editingPfRule.localHost
                    }
                    onChange={(e) =>
                      setEditingPfRule({
                        ...editingPfRule,
                        [editingPfRule.mode === "remote"
                          ? "remoteHost"
                          : "localHost"]: e.target.value,
                      })
                    }
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">
                    {editingPfRule.mode === "remote" ? "Remote Port" : "Local Port"}
                  </label>
                  <input
                    type="number"
                    value={
                      editingPfRule.mode === "remote"
                        ? editingPfRule.remotePort
                        : editingPfRule.localPort
                    }
                    onChange={(e) =>
                      setEditingPfRule({
                        ...editingPfRule,
                        [editingPfRule.mode === "remote"
                          ? "remotePort"
                          : "localPort"]:
                          Number.parseInt(e.target.value, 10) || 8080,
                      })
                    }
                    className="ui-input"
                  />
                </div>
              </div>
              {editingPfRule.mode !== "dynamic" && (
                <div className="grid grid-cols-2 gap-3.5">
                  <div>
                    <label className="ui-label">Target Host</label>
                    <input
                      type="text"
                      value={editingPfRule.targetHost}
                      onChange={(e) =>
                        setEditingPfRule({
                          ...editingPfRule,
                          targetHost: e.target.value,
                        })
                      }
                      className="ui-input"
                    />
                  </div>
                  <div>
                    <label className="ui-label">Target Port</label>
                    <input
                      type="number"
                      value={editingPfRule.targetPort}
                      onChange={(e) =>
                        setEditingPfRule({
                          ...editingPfRule,
                          targetPort: Number.parseInt(e.target.value, 10) || 80,
                        })
                      }
                      className="ui-input"
                    />
                  </div>
                </div>
              )}
              <label className="flex items-center gap-2.5 cursor-pointer font-medium">
                <input
                  type="checkbox"
                  checked={editingPfRule.autoStart}
                  onChange={(e) =>
                    setEditingPfRule({
                      ...editingPfRule,
                      autoStart: e.target.checked,
                    })
                  }
                />
                <span>Start automatically when the app opens</span>
              </label>
              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setEditingPfRule(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Save Rule
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Snippet Package Modal */}
      {editingPackage && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">New Snippet Package</h3>
              <button
                type="button"
                onClick={() => setEditingPackage(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleSaveSnippetPackageForm}
              className="p-5 space-y-4 text-xs"
            >
              <div>
                <label className="ui-label">Package Name *</label>
                <input
                  type="text"
                  required
                  value={editingPackage.name}
                  onChange={(e) =>
                    setEditingPackage({
                      ...editingPackage,
                      name: e.target.value,
                    })
                  }
                  placeholder="Kubernetes / Nginx / Diagnostics"
                  className="ui-input"
                />
              </div>
              <div>
                <label className="ui-label">Remark</label>
                <input
                  type="text"
                  value={editingPackage.remark}
                  onChange={(e) =>
                    setEditingPackage({
                      ...editingPackage,
                      remark: e.target.value,
                    })
                  }
                  className="ui-input"
                />
              </div>
              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setEditingPackage(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Save Package
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Snippet Create/Edit Modal */}
      {editingSnippet && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-lg rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">
                {editingSnippet.id ? "Edit Snippet" : "New Snippet"}
              </h3>
              <button
                type="button"
                onClick={() => setEditingSnippet(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleSaveSnippetForm}
              className="p-5 space-y-4 text-xs"
            >
              <div className="grid grid-cols-2 gap-3.5">
                <div>
                  <label className="ui-label">Name *</label>
                  <input
                    type="text"
                    required
                    value={editingSnippet.name}
                    onChange={(e) =>
                      setEditingSnippet({
                        ...editingSnippet,
                        name: e.target.value,
                      })
                    }
                    placeholder="Check Disk & Memory"
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">Package</label>
                  <SelectField
                    value={editingSnippet.packageId ?? ""}
                    onChange={(val) =>
                      setEditingSnippet({
                        ...editingSnippet,
                        packageId: val || null,
                      })
                    }
                    options={[
                      { value: "", label: "Ungrouped" },
                      ...resources.snippetPackages.map((p) => ({
                        value: p.id,
                        label: p.name,
                      })),
                    ]}
                  />
                </div>
              </div>
              <div>
                <label className="ui-label">Shell Script / Command *</label>
                <textarea
                  required
                  rows={6}
                  value={editingSnippet.code}
                  onChange={(e) =>
                    setEditingSnippet({
                      ...editingSnippet,
                      code: e.target.value,
                    })
                  }
                  className="ui-textarea font-mono"
                />
              </div>
              <div>
                <label className="ui-label">Remark</label>
                <input
                  type="text"
                  value={editingSnippet.remark}
                  onChange={(e) =>
                    setEditingSnippet({
                      ...editingSnippet,
                      remark: e.target.value,
                    })
                  }
                  className="ui-input"
                />
              </div>
              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setEditingSnippet(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Save Snippet
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Batch Execute Snippet Modal */}
      {batchSnippet && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">
                Execute "{batchSnippet.name}"
              </h3>
              <button
                type="button"
                onClick={() => setBatchSnippet(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleRunBatchSnippet}
              className="p-5 space-y-4 text-xs"
            >
              <div>
                <div className="ui-label">Select Target Machines</div>
                <div className="space-y-1.5 max-h-44 overflow-y-auto p-3 rounded-xl bg-input border border-border">
                  <label className="flex items-center gap-2.5 cursor-pointer py-1">
                    <input
                      type="checkbox"
                      checked={batchMachineIds.includes("local")}
                      onChange={(e) =>
                        setBatchMachineIds((prev) =>
                          e.target.checked
                            ? [...prev, "local"]
                            : prev.filter((id) => id !== "local"),
                        )
                      }
                    />
                    <span>Local Machine (localhost)</span>
                  </label>
                  {hosts.map((h) => (
                    <label
                      key={h.id}
                      className="flex items-center gap-2.5 cursor-pointer py-1"
                    >
                      <input
                        type="checkbox"
                        checked={batchMachineIds.includes(h.id)}
                        onChange={(e) =>
                          setBatchMachineIds((prev) =>
                            e.target.checked
                              ? [...prev, h.id]
                              : prev.filter((id) => id !== h.id),
                          )
                        }
                      />
                      <span>
                        {h.alias}{" "}
                        <span className="text-muted-foreground">
                          ({h.user}@{h.hostname})
                        </span>
                      </span>
                    </label>
                  ))}
                </div>
              </div>

              {allGroupNames.length > 0 && (
                <div>
                  <div className="ui-label">Or Entire Machine Groups</div>
                  <div className="flex flex-wrap gap-1.5">
                    {allGroupNames.map((g) => {
                      const active = batchGroupNames.includes(g);
                      return (
                        <button
                          key={g}
                          type="button"
                          onClick={() =>
                            setBatchGroupNames((prev) =>
                              active
                                ? prev.filter((x) => x !== g)
                                : [...prev, g],
                            )
                          }
                          className={`px-3 py-1 rounded-full text-xs border cursor-pointer ${
                            active
                              ? "bg-primary/15 text-primary border-primary"
                              : "bg-secondary text-muted-foreground border-border"
                          }`}
                        >
                          {g}
                        </button>
                      );
                    })}
                  </div>
                </div>
              )}

              <div className="flex justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setBatchSnippet(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={
                    runningBatch ||
                    (batchMachineIds.length === 0 &&
                      batchGroupNames.length === 0)
                  }
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer disabled:opacity-50"
                >
                  {runningBatch ? "Executing..." : "Run Now"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* Docker Container Logs Modal */}
      {dockerLogsModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-3xl rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">
                Logs: {dockerLogsModal.containerName}
              </h3>
              <button
                type="button"
                onClick={() => setDockerLogsModal(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-5">
              <pre className="p-3.5 rounded-xl bg-background border border-border font-mono text-xs text-foreground overflow-auto max-h-[65vh] whitespace-pre-wrap">
                {dockerLogsModal.logs || "(No logs output)"}
              </pre>
            </div>
          </div>
        </div>
      )}

      {/* 1-Click Export SSH Key to Host Modal (ssh-copy-id) */}
      {exportKeyModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <div className="flex items-center gap-2">
                <KeyRound className="w-4 h-4 text-primary" />
                <h3 className="text-sm font-semibold">
                  {t.exportKeyModalTitle}
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setExportKeyModal(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={async (e) => {
                e.preventDefault();
                if (!exportKeyModal.targetHostId) return;
                setExportKeyModal({
                  ...exportKeyModal,
                  busy: true,
                  successMsg: undefined,
                });
                setErrorBanner(null);
                try {
                  await invoke("export_ssh_key_to_host", {
                    keyId: exportKeyModal.keyItem.id,
                    hostId: exportKeyModal.targetHostId,
                    password: exportKeyModal.password.trim() || null,
                  });
                  onRefreshHosts();
                  setExportKeyModal({
                    ...exportKeyModal,
                    busy: false,
                    successMsg: t.exportKeySuccess,
                  });
                } catch (err) {
                  setExportKeyModal({ ...exportKeyModal, busy: false });
                  setErrorBanner(String(err));
                }
              }}
              className="p-5 space-y-4 text-xs"
            >
              <p className="text-muted-foreground leading-relaxed">
                {t.exportKeyModalSubtitle}
              </p>

              <div className="p-2.5 rounded-xl bg-secondary/60 border border-border font-mono text-[11px] truncate">
                {exportKeyModal.keyItem.name} ({exportKeyModal.keyItem.keyType})
              </div>

              <div className="space-y-1.5">
                <label className="ui-label">Target SSH Host *</label>
                <SelectField
                  value={exportKeyModal.targetHostId}
                  onChange={(val) =>
                    setExportKeyModal({
                      ...exportKeyModal,
                      targetHostId: val,
                    })
                  }
                  options={hosts.map((h) => ({
                    value: h.id,
                    label: `${h.alias || h.hostname} (${h.user}@${h.hostname}:${h.port})`,
                  }))}
                />
              </div>

              <div className="space-y-1.5">
                <label className="ui-label">
                  Host Password (optional if already in Keychain)
                </label>
                <input
                  type="password"
                  value={exportKeyModal.password}
                  onChange={(e) =>
                    setExportKeyModal({
                      ...exportKeyModal,
                      password: e.target.value,
                    })
                  }
                  placeholder="One-time password for ssh-copy-id..."
                  className="ui-input"
                />
              </div>

              {exportKeyModal.successMsg && (
                <div className="p-3 rounded-xl bg-emerald-500/15 border border-emerald-500/35 text-emerald-300 font-medium flex items-center gap-2">
                  <Check className="w-4 h-4 shrink-0" />
                  <span>{exportKeyModal.successMsg}</span>
                </div>
              )}

              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setExportKeyModal(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  {t.termCancelBtn}
                </button>
                <button
                  type="submit"
                  disabled={exportKeyModal.busy || !exportKeyModal.targetHostId}
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold disabled:opacity-50 cursor-pointer"
                >
                  {exportKeyModal.busy
                    ? "Exporting..."
                    : t.exportKeyToHostBtn}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
