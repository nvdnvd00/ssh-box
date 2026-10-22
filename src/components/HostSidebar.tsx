import React, { useMemo, useState } from "react";
import {
  Columns,
  FolderGit2,
  KeyRound,
  Laptop,
  Lock,
  Plus,
  RefreshCw,
  Search,
  Server,
  Trash2,
  Zap,
} from "lucide-react";
import { HostEntry } from "../types";

interface HostSidebarProps {
  hosts: HostEntry[];
  activeHostId?: string;
  onConnectHost: (host: HostEntry, passwordOverride?: string) => void;
  onSplitWithHost: (host: HostEntry) => void;
  onOpenLocalTerminal: () => void;
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
  hosts,
  activeHostId,
  onConnectHost,
  onSplitWithHost,
  onOpenLocalTerminal,
  onSaveHost,
  onDeleteHost,
  onRefreshHosts,
}) => {
  const [search, setSearch] = useState("");
  const [quickInput, setQuickInput] = useState("");
  const [showModal, setShowModal] = useState(false);

  const [alias, setAlias] = useState("");
  const [hostname, setHostname] = useState("");
  const [user, setUser] = useState("root");
  const [port, setPort] = useState("22");
  const [identityFile, setIdentityFile] = useState("");
  const [group, setGroup] = useState("Production");
  const [secret, setSecret] = useState("");
  const [saving, setSaving] = useState(false);

  const filteredHosts = useMemo(() => {
    const q = search.trim().toLowerCase();
    if (!q) return hosts;
    return hosts.filter(
      (h) =>
        h.alias.toLowerCase().includes(q) ||
        h.hostname.toLowerCase().includes(q) ||
        h.user.toLowerCase().includes(q) ||
        h.group.toLowerCase().includes(q),
    );
  }, [hosts, search]);

  const groupedHosts = useMemo(() => {
    const map = new Map<string, HostEntry[]>();
    for (const h of filteredHosts) {
      const g = h.group || (h.source === "sshconfig" ? "~/.ssh/config" : "Custom");
      const list = map.get(g) ?? [];
      list.push(h);
      map.set(g, list);
    }
    return Array.from(map.entries());
  }, [filteredHosts]);

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
        },
        secret.trim() ? secret.trim() : undefined,
      );
      setShowModal(false);
      setAlias("");
      setHostname("");
      setUser("root");
      setPort("22");
      setIdentityFile("");
      setSecret("");
    } finally {
      setSaving(false);
    }
  };

  return (
    <aside className="w-64 shrink-0 h-full bg-[#0d1119]/95 border-r border-slate-800/80 flex flex-col select-none">
      {/* Quick Connect Box */}
      <div className="p-3 border-b border-slate-800/80 space-y-2">
        <form onSubmit={handleQuickSubmit} className="relative">
          <Zap className="w-3.5 h-3.5 text-cyan-400 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
          <input
            type="text"
            value={quickInput}
            onChange={(e) => setQuickInput(e.target.value)}
            placeholder="ssh user@host:port..."
            className="w-full pl-8 pr-2.5 py-1.5 rounded-md bg-[#131926] border border-slate-700/70 text-xs text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-cyan-500/80 font-mono"
          />
        </form>

        <div className="flex items-center gap-1.5">
          <button
            type="button"
            onClick={onOpenLocalTerminal}
            className="flex-1 inline-flex items-center justify-center gap-1.5 px-2.5 py-1.5 rounded-md bg-slate-800/90 hover:bg-slate-700/90 text-slate-100 border border-slate-700/60 text-xs font-medium transition-colors cursor-pointer"
          >
            <Laptop className="w-3.5 h-3.5 text-cyan-400" />
            Local Shell
          </button>
          <button
            type="button"
            onClick={() => setShowModal(true)}
            className="inline-flex items-center justify-center gap-1 px-2.5 py-1.5 rounded-md bg-cyan-500/15 hover:bg-cyan-500/25 text-cyan-300 border border-cyan-500/30 text-xs font-medium transition-colors cursor-pointer"
            title="Add SSH Host"
          >
            <Plus className="w-3.5 h-3.5" />
            Host
          </button>
        </div>
      </div>

      {/* Filter Bar */}
      <div className="px-3 py-2 border-b border-slate-800/60 flex items-center gap-1.5">
        <div className="relative flex-1">
          <Search className="w-3.5 h-3.5 text-slate-500 absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
          <input
            type="text"
            value={search}
            onChange={(e) => setSearch(e.target.value)}
            placeholder="Filter hosts..."
            className="w-full pl-8 pr-2 py-1 rounded bg-[#111622] border border-slate-800 text-xs text-slate-200 placeholder:text-slate-500 focus:outline-none focus:border-slate-600"
          />
        </div>
        <button
          type="button"
          onClick={onRefreshHosts}
          title="Reload Hosts"
          className="p-1.5 rounded hover:bg-slate-800 text-slate-400 hover:text-slate-200 transition-colors cursor-pointer"
        >
          <RefreshCw className="w-3.5 h-3.5" />
        </button>
      </div>

      {/* Host Groups */}
      <div className="flex-1 overflow-y-auto px-2 py-2 space-y-4">
        {groupedHosts.length === 0 ? (
          <div className="px-3 py-8 text-center text-xs text-slate-500 space-y-1">
            <p className="font-medium text-slate-400">No SSH hosts found</p>
            <p className="text-[11px]">
              Add entries to <code className="text-slate-300">~/.ssh/config</code> or click{" "}
              <span className="text-cyan-400">+ Host</span> above.
            </p>
          </div>
        ) : (
          groupedHosts.map(([groupName, groupItems]) => (
            <div key={groupName} className="space-y-1">
              <div className="flex items-center justify-between px-2 py-0.5 text-[10px] font-semibold uppercase tracking-wider text-slate-400">
                <span className="flex items-center gap-1.5 truncate">
                  <FolderGit2 className="w-3 h-3 text-slate-500" />
                  {groupName}
                </span>
                <span className="text-slate-600 font-mono">{groupItems.length}</span>
              </div>

              <div className="space-y-0.5">
                {groupItems.map((h) => {
                  const isSelected = h.id === activeHostId;
                  return (
                    <div
                      key={h.id}
                      onClick={() => onConnectHost(h)}
                      className={`group flex items-center justify-between px-2.5 py-2 rounded-md cursor-pointer transition-colors ${
                        isSelected
                          ? "bg-cyan-500/15 border border-cyan-500/30 text-slate-100"
                          : "hover:bg-slate-800/60 border border-transparent text-slate-300"
                      }`}
                    >
                      <div className="min-w-0 flex-1">
                        <div className="flex items-center gap-1.5">
                          <Server
                            className={`w-3.5 h-3.5 shrink-0 ${
                              isSelected ? "text-cyan-400" : "text-slate-400"
                            }`}
                          />
                          <span className="text-xs font-medium truncate">
                            {h.alias}
                          </span>
                          {h.source === "sshconfig" ? (
                            <span className="px-1 py-0.2 rounded bg-slate-800 text-slate-400 text-[9px] font-mono">
                              config
                            </span>
                          ) : (
                            <span
                              title="Saved Host"
                              className="inline-flex items-center text-emerald-400/80"
                            >
                              <KeyRound className="w-2.5 h-2.5" />
                            </span>
                          )}
                        </div>
                        <div className="text-[11px] text-slate-400 font-mono truncate pl-5">
                          {h.user}@{h.hostname}
                          {h.port !== 22 ? `:${h.port}` : ""}
                        </div>
                      </div>

                      <div className="hidden group-hover:flex items-center gap-1 ml-1 shrink-0">
                        <button
                          type="button"
                          onClick={(e) => {
                            e.stopPropagation();
                            onSplitWithHost(h);
                          }}
                          title="Split Current Pane with this Host"
                          className="p-1 rounded hover:bg-slate-700 text-slate-300 hover:text-cyan-300"
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
                            title="Delete Custom Host"
                            className="p-1 rounded hover:bg-rose-500/20 text-slate-400 hover:text-rose-300"
                          >
                            <Trash2 className="w-3 h-3" />
                          </button>
                        )}
                      </div>
                    </div>
                  );
                })}
              </div>
            </div>
          ))
        )}
      </div>

      {/* Add Custom Host Modal */}
      {showModal && (
        <div className="fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="w-full max-w-md rounded-xl bg-[#111622] border border-slate-700/80 shadow-2xl overflow-hidden">
            <div className="px-4 py-3 border-b border-slate-800 flex items-center justify-between">
              <div className="flex items-center gap-2">
                <Server className="w-4 h-4 text-cyan-400" />
                <h3 className="text-sm font-semibold text-slate-100">
                  New SSH Host
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setShowModal(false)}
                className="text-xs text-slate-400 hover:text-slate-200 cursor-pointer"
              >
                ESC
              </button>
            </div>

            <form onSubmit={handleCreateHost} className="p-4 space-y-3 text-xs">
              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-slate-400 mb-1">Alias / Name</label>
                  <input
                    type="text"
                    value={alias}
                    onChange={(e) => setAlias(e.target.value)}
                    placeholder="prod-api-01"
                    className="w-full px-2.5 py-1.5 rounded bg-[#0b0e14] border border-slate-700 text-slate-100 focus:outline-none focus:border-cyan-500"
                  />
                </div>
                <div>
                  <label className="block text-slate-400 mb-1">Group</label>
                  <input
                    type="text"
                    value={group}
                    onChange={(e) => setGroup(e.target.value)}
                    placeholder="Production"
                    className="w-full px-2.5 py-1.5 rounded bg-[#0b0e14] border border-slate-700 text-slate-100 focus:outline-none focus:border-cyan-500"
                  />
                </div>
              </div>

              <div className="grid grid-cols-3 gap-3">
                <div className="col-span-2">
                  <label className="block text-slate-400 mb-1">
                    Hostname / IP *
                  </label>
                  <input
                    type="text"
                    required
                    value={hostname}
                    onChange={(e) => setHostname(e.target.value)}
                    placeholder="10.0.1.25"
                    className="w-full px-2.5 py-1.5 rounded bg-[#0b0e14] border border-slate-700 text-slate-100 font-mono focus:outline-none focus:border-cyan-500"
                  />
                </div>
                <div>
                  <label className="block text-slate-400 mb-1">Port</label>
                  <input
                    type="number"
                    value={port}
                    onChange={(e) => setPort(e.target.value)}
                    className="w-full px-2.5 py-1.5 rounded bg-[#0b0e14] border border-slate-700 text-slate-100 font-mono focus:outline-none focus:border-cyan-500"
                  />
                </div>
              </div>

              <div className="grid grid-cols-2 gap-3">
                <div>
                  <label className="block text-slate-400 mb-1">Username</label>
                  <input
                    type="text"
                    value={user}
                    onChange={(e) => setUser(e.target.value)}
                    placeholder="ubuntu"
                    className="w-full px-2.5 py-1.5 rounded bg-[#0b0e14] border border-slate-700 text-slate-100 font-mono focus:outline-none focus:border-cyan-500"
                  />
                </div>
                <div>
                  <label className="block text-slate-400 mb-1">
                    Identity File (optional)
                  </label>
                  <input
                    type="text"
                    value={identityFile}
                    onChange={(e) => setIdentityFile(e.target.value)}
                    placeholder="~/.ssh/id_ed25519"
                    className="w-full px-2.5 py-1.5 rounded bg-[#0b0e14] border border-slate-700 text-slate-100 font-mono focus:outline-none focus:border-cyan-500"
                  />
                </div>
              </div>

              <div>
                <label className="flex items-center gap-1.5 text-slate-400 mb-1">
                  <Lock className="w-3 h-3 text-emerald-400" />
                  Password / Passphrase
                </label>
                <input
                  type="password"
                  value={secret}
                  onChange={(e) => setSecret(e.target.value)}
                  placeholder="Optional"
                  className="w-full px-2.5 py-1.5 rounded bg-[#0b0e14] border border-slate-700 text-slate-100 focus:outline-none focus:border-cyan-500"
                />
              </div>

              <div className="flex items-center justify-end gap-2 pt-2 border-t border-slate-800">
                <button
                  type="button"
                  onClick={() => setShowModal(false)}
                  className="px-3 py-1.5 rounded bg-slate-800 hover:bg-slate-700 text-slate-300 cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={saving}
                  className="px-3.5 py-1.5 rounded bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-semibold cursor-pointer disabled:opacity-50"
                >
                  {saving ? "Saving..." : "Save Host"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </aside>
  );
};
