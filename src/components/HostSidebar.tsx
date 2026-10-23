import React, { useMemo, useState } from "react";
import {
  ChevronDown,
  Columns,
  FolderGit2,
  KeyRound,
  Laptop,
  Layers,
  Lock,
  Plus,
  RefreshCw,
  Search,
  Server,
  Trash2,
  UserCheck,
  X,
  Zap,
} from "lucide-react";
import { HostEntry, MachineAuthMethod, ResourcesStoreData } from "../types";
import { SelectField } from "./SelectField";

interface HostSidebarProps {
  isOpen: boolean;
  hosts: HostEntry[];
  resources?: ResourcesStoreData;
  activeHostId?: string;
  onConnectHost: (host: HostEntry, passwordOverride?: string) => void;
  onSplitWithHost: (host: HostEntry) => void;
  onOpenLocalTerminal: () => void;
  onOpenResources?: () => void;
  onSaveHost: (host: HostEntry, secret?: string) => Promise<void>;
  onDeleteHost: (hostId: string) => Promise<void>;
  onRefreshHosts: () => void;
}

function parseQuickConnectTarget(raw: string): HostEntry | null {
  const trimmed = raw.trim().replace(/^ssh\s+/i, "");
  if (!trimmed) return null;

  let user = "root";
  let hostPart = trimmed;
  if (trimmed.includes("@")) {
    const [u, rest] = trimmed.split("@", 2);
    if (u.trim()) user = u.trim();
    hostPart = rest.trim();
  }

  let hostname = hostPart;
  let port = 22;
  if (hostPart.includes(":")) {
    const [h, p] = hostPart.split(":", 2);
    hostname = h.trim();
    const parsedPort = Number.parseInt(p.trim(), 10);
    if (!Number.isNaN(parsedPort) && parsedPort > 0 && parsedPort <= 65535) {
      port = parsedPort;
    }
  }

  if (!hostname) return null;

  return {
    id: `quick:${user}@${hostname}:${port}`,
    alias: hostname,
    hostname,
    user,
    port,
    identityFile: null,
    group: "Quick Connect",
    source: "custom",
  };
}

export const HostSidebar: React.FC<HostSidebarProps> = ({
  isOpen,
  hosts,
  resources,
  activeHostId,
  onConnectHost,
  onSplitWithHost,
  onOpenLocalTerminal,
  onOpenResources,
  onSaveHost,
  onDeleteHost,
  onRefreshHosts,
}) => {
  const [search, setSearch] = useState("");
  const [selectedTag, setSelectedTag] = useState<string>("ALL");
  const [quickInput, setQuickInput] = useState("");
  const [showModal, setShowModal] = useState(false);
  const [collapsedGroups, setCollapsedGroups] = useState<
    Record<string, boolean>
  >({});

  const [alias, setAlias] = useState("");
  const [hostname, setHostname] = useState("");
  const [user, setUser] = useState("root");
  const [port, setPort] = useState("22");
  const [authMethod, setAuthMethod] =
    useState<MachineAuthMethod>("passwordOrKey");
  const [identityId, setIdentityId] = useState<string>("");
  const [sshKeyId, setSshKeyId] = useState<string>("");
  const [identityFile, setIdentityFile] = useState("");
  const [group, setGroup] = useState("Production");
  const [secret, setSecret] = useState("");
  const [saving, setSaving] = useState(false);

  const filteredHosts = useMemo(() => {
    const q = search.trim().toLowerCase();
    return hosts.filter((h) => {
      if (selectedTag !== "ALL" && !(h.tags ?? []).includes(selectedTag)) {
        return false;
      }
      if (!q) return true;
      return (
        h.alias.toLowerCase().includes(q) ||
        h.hostname.toLowerCase().includes(q) ||
        h.user.toLowerCase().includes(q) ||
        h.group.toLowerCase().includes(q) ||
        (h.remark ?? "").toLowerCase().includes(q)
      );
    });
  }, [hosts, search, selectedTag]);

  const groupedHosts = useMemo(() => {
    const map = new Map<string, HostEntry[]>();
    for (const h of filteredHosts) {
      const g =
        h.group || (h.source === "sshconfig" ? "~/.ssh/config" : "Custom");
      const list = map.get(g) ?? [];
      list.push(h);
      map.set(g, list);
    }
    return Array.from(map.entries());
  }, [filteredHosts]);

  const toggleGroup = (groupName: string) => {
    setCollapsedGroups((prev) => ({
      ...prev,
      [groupName]: !prev[groupName],
    }));
  };

  const handleQuickSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    const target = parseQuickConnectTarget(quickInput);
    if (!target) return;
    onConnectHost(target);
    setQuickInput("");
  };

  const handleCreateHost = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!hostname.trim()) return;
    setSaving(true);
    try {
      const parsedPort = Number.parseInt(port, 10);
      await onSaveHost(
        {
          id: "",
          alias: alias.trim() || hostname.trim(),
          hostname: hostname.trim(),
          user: user.trim() || "root",
          port:
            !Number.isNaN(parsedPort) && parsedPort > 0 && parsedPort <= 65535
              ? parsedPort
              : 22,
          identityFile: identityFile.trim() ? identityFile.trim() : null,
          group: group.trim() || "Production",
          source: "custom",
          authMethod,
          identityId:
            authMethod === "identity" && identityId ? identityId : null,
          sshKeyId: sshKeyId || null,
        },
        secret.trim() ? secret.trim() : undefined,
      );
      setShowModal(false);
      setAlias("");
      setHostname("");
      setUser("root");
      setPort("22");
      setIdentityFile("");
      setIdentityId("");
      setSshKeyId("");
      setSecret("");
    } finally {
      setSaving(false);
    }
  };

  return (
    <aside
      aria-hidden={!isOpen}
      className={`ui-drawer-shell shrink-0 h-full bg-card/95 border-border overflow-hidden select-none ${
        isOpen
          ? "w-64 opacity-100 border-r"
          : "w-0 opacity-0 border-r-0 pointer-events-none"
      }`}
    >
      <div className="w-64 h-full flex flex-col">
        {/* Quick Connect Box */}
        <div className="p-3 border-b border-border space-y-2">
          <form onSubmit={handleQuickSubmit} className="relative group">
            <Zap className="w-3.5 h-3.5 text-primary absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none transition-transform duration-150 group-focus-within:scale-110" />
            <input
              type="text"
              value={quickInput}
              onChange={(e) => setQuickInput(e.target.value)}
              placeholder="Quick connect user@host:port"
              className="w-full h-8 pl-8 pr-2.5 rounded-xl bg-input border border-border text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary transition-all"
            />
          </form>

          <div className="flex items-center gap-1.5">
            <button
              type="button"
              onClick={onOpenLocalTerminal}
              className="flex-1 inline-flex items-center justify-center gap-1.5 px-2.5 py-1.5 rounded-xl bg-secondary hover:bg-accent active:scale-[0.98] text-foreground border border-border text-xs font-medium transition-all duration-150 cursor-pointer shadow-2xs"
            >
              <Laptop className="w-3.5 h-3.5 text-primary" />
              Local Shell
            </button>
            <button
              type="button"
              onClick={() => setShowModal(true)}
              className="inline-flex items-center justify-center gap-1 px-2.5 py-1.5 rounded-xl bg-primary/15 hover:bg-primary/25 active:scale-[0.98] text-primary border border-primary/30 text-xs font-semibold transition-all duration-150 cursor-pointer"
              title="Add SSH Host"
            >
              <Plus className="w-3.5 h-3.5" />
              Host
            </button>
          </div>
        </div>

        {/* Filter Bar + Tag Pills */}
        <div className="px-3 py-2 border-b border-border/70 space-y-1.5">
          <div className="flex items-center gap-1.5">
            <div className="relative flex-1">
              <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Filter machines..."
                className="w-full h-7 pl-8 pr-2 rounded-lg bg-input border border-border text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary/60 transition-colors"
              />
            </div>
            <button
              type="button"
              onClick={onRefreshHosts}
              title="Reload Hosts"
              className="p-1.5 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground active:rotate-90 transition-all duration-200 cursor-pointer"
            >
              <RefreshCw className="w-3.5 h-3.5" />
            </button>
          </div>

          {resources && resources.tags.length > 0 && (
            <div className="flex items-center gap-1 overflow-x-auto no-scrollbar pt-0.5">
              <button
                type="button"
                onClick={() => setSelectedTag("ALL")}
                className={`px-2 py-0.5 rounded-full text-[11px] shrink-0 cursor-pointer ${
                  selectedTag === "ALL"
                    ? "bg-primary/15 text-primary font-medium"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                All
              </button>
              {resources.tags.map((t) => (
                <button
                  key={t.id}
                  type="button"
                  onClick={() =>
                    setSelectedTag(selectedTag === t.name ? "ALL" : t.name)
                  }
                  className={`px-2 py-0.5 rounded-full text-[11px] shrink-0 inline-flex items-center gap-1 cursor-pointer ${
                    selectedTag === t.name
                      ? "bg-secondary text-foreground font-medium"
                      : "text-muted-foreground hover:text-foreground"
                  }`}
                >
                  <span
                    className="w-1.5 h-1.5 rounded-full"
                    style={{ backgroundColor: t.color }}
                  />
                  {t.name}
                </button>
              ))}
            </div>
          )}
        </div>

        {/* Host Groups */}
        <div className="flex-1 overflow-y-auto px-2 py-2 space-y-3">
          {groupedHosts.length === 0 ? (
            <div className="px-3 py-8 text-center text-xs text-muted-foreground space-y-1">
              <p className="font-medium text-foreground/80">
                No SSH machines found
              </p>
              <p className="text-[11px]">
                Add entries to{" "}
                <code className="text-foreground">~/.ssh/config</code> or click{" "}
                <span className="text-primary font-medium">+ Host</span> above.
              </p>
            </div>
          ) : (
            groupedHosts.map(([groupName, groupItems]) => {
              const isCollapsed = Boolean(collapsedGroups[groupName]);
              return (
                <div key={groupName} className="space-y-1">
                  <button
                    type="button"
                    onClick={() => toggleGroup(groupName)}
                    className="w-full flex items-center justify-between px-2 py-1 rounded-lg text-[11px] font-semibold uppercase tracking-wider text-muted-foreground hover:text-foreground hover:bg-accent/50 transition-colors cursor-pointer"
                  >
                    <span className="flex items-center gap-1.5 truncate">
                      <ChevronDown
                        className={`w-3 h-3 text-muted-foreground transition-transform duration-200 ${
                          isCollapsed ? "-rotate-90" : "rotate-0"
                        }`}
                      />
                      <FolderGit2 className="w-3 h-3 text-primary/80" />
                      {groupName}
                    </span>
                    <span className="px-1.5 py-0.2 rounded-full bg-secondary text-muted-foreground text-[10px]">
                      {groupItems.length}
                    </span>
                  </button>

                  {!isCollapsed && (
                    <div className="space-y-0.5 pl-1">
                      {groupItems.map((h) => {
                        const isSelected = h.id === activeHostId;
                        const identity = resources?.identities.find(
                          (i) => i.id === h.identityId,
                        );
                        return (
                          <div
                            key={h.id}
                            onClick={() => onConnectHost(h)}
                            className={`group flex items-center justify-between px-2.5 py-2 rounded-xl cursor-pointer transition-all duration-150 ${
                              isSelected
                                ? "bg-primary/14 border border-primary/35 text-foreground shadow-2xs"
                                : "hover:bg-accent/70 border border-transparent text-card-foreground"
                            }`}
                          >
                            <div className="min-w-0 flex-1">
                              <div className="flex items-center gap-1.5">
                                <Server
                                  className={`w-3.5 h-3.5 shrink-0 transition-colors ${
                                    isSelected
                                      ? "text-primary"
                                      : "text-muted-foreground group-hover:text-foreground"
                                  }`}
                                />
                                <span className="text-xs font-medium truncate">
                                  {h.alias}
                                </span>
                                {identity ? (
                                  <span
                                    title={`Identity: ${identity.name}`}
                                    className="inline-flex items-center text-violet-400"
                                  >
                                    <UserCheck className="w-2.5 h-2.5" />
                                  </span>
                                ) : h.source === "sshconfig" ? (
                                  <span className="px-1.5 py-0.2 rounded bg-secondary border border-border/70 text-muted-foreground text-[10px]">
                                    config
                                  </span>
                                ) : (
                                  <span
                                    title="Saved Machine"
                                    className="inline-flex items-center text-emerald-400/90"
                                  >
                                    <KeyRound className="w-2.5 h-2.5" />
                                  </span>
                                )}
                              </div>
                              <div className="flex items-center gap-1.5 pl-5">
                                <span className="text-[11px] text-muted-foreground truncate">
                                  {identity ? identity.userName : h.user}@
                                  {h.hostname}
                                  {h.port !== 22 ? `:${h.port}` : ""}
                                </span>
                                {(h.tags ?? []).slice(0, 2).map((tName) => {
                                  const tObj = resources?.tags.find(
                                    (t) => t.name === tName,
                                  );
                                  return (
                                    <span
                                      key={tName}
                                      className="w-1.5 h-1.5 rounded-full shrink-0"
                                      title={tName}
                                      style={{
                                        backgroundColor:
                                          tObj?.color ?? "#38bdf8",
                                      }}
                                    />
                                  );
                                })}
                              </div>
                            </div>

                            <div className="opacity-0 group-hover:opacity-100 flex items-center gap-1 ml-1 shrink-0 transition-opacity duration-150">
                              <button
                                type="button"
                                onClick={(e) => {
                                  e.stopPropagation();
                                  onSplitWithHost(h);
                                }}
                                title="Split Current Pane with this Machine"
                                className="p-1 rounded-md hover:bg-secondary text-muted-foreground hover:text-primary transition-colors cursor-pointer"
                              >
                                <Columns className="w-3 h-3" />
                              </button>
                              {h.source === "custom" && (
                                <button
                                  type="button"
                                  onClick={(e) => {
                                    e.stopPropagation();
                                    onDeleteHost(h.id);
                                  }}
                                  title="Delete Machine"
                                  className="p-1 rounded-md hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 transition-colors cursor-pointer"
                                >
                                  <Trash2 className="w-3 h-3" />
                                </button>
                              )}
                            </div>
                          </div>
                        );
                      })}
                    </div>
                  )}
                </div>
              );
            })
          )}
        </div>

        {/* Bottom shortcut to Resource Center */}
        {onOpenResources && (
          <div className="p-2.5 border-t border-border bg-card/60">
            <button
              type="button"
              onClick={onOpenResources}
              className="w-full flex items-center justify-center gap-2 px-3 py-2 rounded-xl bg-primary/12 hover:bg-primary/20 text-primary border border-primary/25 text-xs font-semibold transition-colors cursor-pointer"
            >
              <Layers className="w-3.5 h-3.5" />
              Resource Center
            </button>
          </div>
        )}
      </div>

      {/* Add Custom Host Modal */}
      {showModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-visible">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Server className="w-4 h-4 text-primary" />
                <h3 className="text-sm font-semibold text-foreground">
                  New SSH Machine
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setShowModal(false)}
                className="p-1 rounded-lg hover:bg-secondary text-muted-foreground hover:text-foreground cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>

            <form
              onSubmit={handleCreateHost}
              className="p-5 space-y-4 text-xs"
            >
              <div className="grid grid-cols-2 gap-3.5">
                <div>
                  <label className="ui-label">Alias / Name</label>
                  <input
                    type="text"
                    value={alias}
                    onChange={(e) => setAlias(e.target.value)}
                    placeholder="prod-api-01"
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">Group</label>
                  <input
                    type="text"
                    value={group}
                    onChange={(e) => setGroup(e.target.value)}
                    placeholder="Production"
                    className="ui-input"
                  />
                </div>
              </div>

              <div className="grid grid-cols-3 gap-3.5">
                <div className="col-span-2">
                  <label className="ui-label">Hostname / IP *</label>
                  <input
                    type="text"
                    required
                    value={hostname}
                    onChange={(e) => setHostname(e.target.value)}
                    placeholder="10.0.1.25"
                    className="ui-input"
                  />
                </div>
                <div>
                  <label className="ui-label">Port</label>
                  <input
                    type="number"
                    value={port}
                    onChange={(e) => setPort(e.target.value)}
                    className="ui-input"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3.5">
                <div>
                  <label className="ui-label">Auth Method</label>
                  <SelectField
                    value={authMethod}
                    onChange={(val) =>
                      setAuthMethod(val as MachineAuthMethod)
                    }
                    options={[
                      { value: "passwordOrKey", label: "Password / SSH Key" },
                      { value: "identity", label: "Saved Identity" },
                    ]}
                  />
                </div>

                {authMethod === "identity" ? (
                  <div>
                    <label className="ui-label">Identity</label>
                    <SelectField
                      value={identityId}
                      onChange={setIdentityId}
                      options={[
                        { value: "", label: "Select Identity..." },
                        ...(resources?.identities ?? []).map((i) => ({
                          value: i.id,
                          label: `${i.name} (${i.userName})`,
                        })),
                      ]}
                    />
                  </div>
                ) : (
                  <div>
                    <label className="ui-label">Username</label>
                    <input
                      type="text"
                      value={user}
                      onChange={(e) => setUser(e.target.value)}
                      placeholder="ubuntu"
                      className="ui-input"
                    />
                  </div>
                )}
              </div>

              {authMethod !== "identity" && (
                <>
                  <div className="grid grid-cols-2 gap-3.5">
                    <div>
                      <label className="ui-label">Vault SSH Key</label>
                      <SelectField
                        value={sshKeyId}
                        onChange={setSshKeyId}
                        options={[
                          { value: "", label: "Default / File Path" },
                          ...(resources?.sshKeys ?? []).map((k) => ({
                            value: k.id,
                            label: `${k.name} (${k.keyType})`,
                          })),
                        ]}
                      />
                    </div>
                    <div>
                      <label className="ui-label">Identity File Path</label>
                      <input
                        type="text"
                        value={identityFile}
                        onChange={(e) => setIdentityFile(e.target.value)}
                        placeholder="~/.ssh/id_ed25519"
                        className="ui-input"
                      />
                    </div>
                  </div>

                  <div>
                    <label className="ui-label flex items-center gap-1.5">
                      <Lock className="w-3 h-3 text-emerald-400" />
                      Password / Passphrase
                    </label>
                    <input
                      type="password"
                      value={secret}
                      onChange={(e) => setSecret(e.target.value)}
                      placeholder="Optional"
                      className="ui-input"
                    />
                  </div>
                </>
              )}

              <div className="flex items-center justify-end gap-2.5 pt-3 border-t border-border">
                <button
                  type="button"
                  onClick={() => setShowModal(false)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent text-foreground transition-colors cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={saving}
                  className="h-9 px-4 rounded-xl bg-primary hover:opacity-90 text-primary-foreground font-semibold transition-opacity cursor-pointer disabled:opacity-50"
                >
                  {saving ? "Saving..." : "Save Machine"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </aside>
  );
};
