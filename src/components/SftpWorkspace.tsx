import React, { useCallback, useEffect, useMemo, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import {
  ArrowLeft,
  ArrowRight,
  ArrowUp,
  Check,
  Edit3,
  Eye,
  EyeOff,
  File,
  FilePlus,
  FileText,
  Folder,
  FolderOpen,
  FolderPlus,
  Home,
  Laptop,
  Link2,
  Lock,
  RefreshCw,
  Search,
  Server,
  Trash2,
  Type,
  X,
  Zap,
} from "lucide-react";
import {
  HostEntry,
  SftpEntry,
  SftpListResponse,
  SftpTransferProgress,
  SftpTransferResult,
} from "../types";
import { AppLanguage } from "../themes";
import { getI18n } from "../i18n";
import { SelectField } from "./SelectField";

interface SftpWorkspaceProps {
  hosts: HostEntry[];
  initialRemoteHostId?: string;
  language?: AppLanguage;
  defaultFastSync?: boolean;
}

type Side = "local" | "remote";

function formatFileSize(bytes: number, kind: SftpEntry["kind"]): string {
  if (kind === "dir") return "—";
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  if (bytes < 1024 * 1024 * 1024) {
    return `${(bytes / (1024 * 1024)).toFixed(2)} MB`;
  }
  return `${(bytes / (1024 * 1024 * 1024)).toFixed(2)} GB`;
}

function formatModifiedDate(ts: number): string {
  if (!ts) return "—";
  return new Date(ts * 1000).toLocaleString(undefined, {
    month: "short",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}

function parentPosixPath(cwd: string): string {
  const trimmed = cwd.replace(/\/+$/, "");
  if (!trimmed || trimmed === "/") return "/";
  const idx = trimmed.lastIndexOf("/");
  if (idx <= 0) return "/";
  return trimmed.slice(0, idx);
}

function joinPath(base: string, name: string): string {
  const cleanName = name.replace(/^\/+/, "");
  if (!base || base === "/") return `/${cleanName}`;
  return `${base.replace(/\/+$/, "")}/${cleanName}`;
}

function octalToBits(octalStr: string): boolean[] {
  const clean = octalStr.trim().replace(/^0+/, "") || "0";
  const padded = clean.padStart(3, "0").slice(-3);
  const digits = padded.split("").map((ch) => Number.parseInt(ch, 8) || 0);
  return [
    Boolean(digits[0] & 4),
    Boolean(digits[0] & 2),
    Boolean(digits[0] & 1),
    Boolean(digits[1] & 4),
    Boolean(digits[1] & 2),
    Boolean(digits[1] & 1),
    Boolean(digits[2] & 4),
    Boolean(digits[2] & 2),
    Boolean(digits[2] & 1),
  ];
}

function bitsToOctal(bits: boolean[]): string {
  const d0 = (bits[0] ? 4 : 0) + (bits[1] ? 2 : 0) + (bits[2] ? 1 : 0);
  const d1 = (bits[3] ? 4 : 0) + (bits[4] ? 2 : 0) + (bits[5] ? 1 : 0);
  const d2 = (bits[6] ? 4 : 0) + (bits[7] ? 2 : 0) + (bits[8] ? 1 : 0);
  return `${d0}${d1}${d2}`;
}

export const SftpWorkspace: React.FC<SftpWorkspaceProps> = ({
  hosts,
  initialRemoteHostId,
  language = "vi",
  defaultFastSync = true,
}) => {
  const t = getI18n(language);
  const [fastSyncEnabled, setFastSyncEnabled] =
    useState<boolean>(defaultFastSync);
  const [transferProgress, setTransferProgress] =
    useState<SftpTransferProgress | null>(null);
  const [lastTransferResult, setLastTransferResult] =
    useState<SftpTransferResult | null>(null);
  // Local pane state
  const [localCwd, setLocalCwd] = useState<string>("~");
  const [localPathInput, setLocalPathInput] = useState<string>("~");
  const [localEntries, setLocalEntries] = useState<SftpEntry[]>([]);
  const [localLoading, setLocalLoading] = useState<boolean>(false);
  const [localError, setLocalError] = useState<string | null>(null);
  const [localSearch, setLocalSearch] = useState<string>("");
  const [localShowHidden, setLocalShowHidden] = useState<boolean>(false);
  const [localSelectedPath, setLocalSelectedPath] = useState<string | null>(
    null,
  );

  // Remote pane state
  const [remoteHostId, setRemoteHostId] = useState<string>(() => {
    if (
      initialRemoteHostId &&
      initialRemoteHostId !== "local" &&
      hosts.some((h) => h.id === initialRemoteHostId)
    ) {
      return initialRemoteHostId;
    }
    return hosts[0]?.id ?? "";
  });
  const [remoteCwd, setRemoteCwd] = useState<string>("");
  const [remotePathInput, setRemotePathInput] = useState<string>("");
  const [remoteEntries, setRemoteEntries] = useState<SftpEntry[]>([]);
  const [remoteLoading, setRemoteLoading] = useState<boolean>(false);
  const [remoteError, setRemoteError] = useState<string | null>(null);
  const [remoteSearch, setRemoteSearch] = useState<string>("");
  const [remoteShowHidden, setRemoteShowHidden] = useState<boolean>(false);
  const [remoteSelectedPath, setRemoteSelectedPath] = useState<string | null>(
    null,
  );

  // Status banner for transfers / actions
  const [transferBusy, setTransferBusy] = useState<boolean>(false);
  const [statusMessage, setStatusMessage] = useState<{
    type: "success" | "error";
    text: string;
  } | null>(null);

  // Modals state
  const [mkdirModal, setMkdirModal] = useState<{
    side: Side;
    baseDir: string;
    name: string;
  } | null>(null);

  const [renameModal, setRenameModal] = useState<{
    side: Side;
    entry: SftpEntry;
    newName: string;
  } | null>(null);

  const [chmodModal, setChmodModal] = useState<{
    side: Side;
    entry: SftpEntry;
    octal: string;
    bits: boolean[];
  } | null>(null);

  const [deleteModal, setDeleteModal] = useState<{
    side: Side;
    entry: SftpEntry;
  } | null>(null);

  const [editorModal, setEditorModal] = useState<{
    side: Side;
    path: string;
    fileName: string;
    isNew: boolean;
    content: string;
    saving: boolean;
  } | null>(null);

  // Sync initialRemoteHostId when prop changes
  useEffect(() => {
    if (
      initialRemoteHostId &&
      initialRemoteHostId !== "local" &&
      hosts.some((h) => h.id === initialRemoteHostId)
    ) {
      setRemoteHostId(initialRemoteHostId);
    } else if (!remoteHostId && hosts.length > 0) {
      setRemoteHostId(hosts[0].id);
    }
  }, [initialRemoteHostId, hosts, remoteHostId]);

  const loadLocalDir = useCallback(async (targetPath: string) => {
    setLocalLoading(true);
    setLocalError(null);
    try {
      const res = await invoke<SftpListResponse>("sftp_list_dir", {
        hostId: "local",
        path: targetPath,
      });
      setLocalCwd(res.cwd);
      setLocalPathInput(res.cwd);
      setLocalEntries(res.entries);
      setLocalSelectedPath(null);
    } catch (err) {
      setLocalError(String(err));
    } finally {
      setLocalLoading(false);
    }
  }, []);

  const loadRemoteDir = useCallback(
    async (hostId: string, targetPath: string) => {
      if (!hostId) return;
      setRemoteLoading(true);
      setRemoteError(null);
      try {
        const res = await invoke<SftpListResponse>("sftp_list_dir", {
          hostId,
          path: targetPath,
        });
        setRemoteCwd(res.cwd);
        setRemotePathInput(res.cwd);
        setRemoteEntries(res.entries);
        setRemoteSelectedPath(null);
      } catch (err) {
        setRemoteError(String(err));
      } finally {
        setRemoteLoading(false);
      }
    },
    [],
  );

  useEffect(() => {
    loadLocalDir("~");
  }, [loadLocalDir]);

  useEffect(() => {
    if (!remoteHostId) {
      setRemoteEntries([]);
      setRemoteCwd("");
      setRemotePathInput("");
      return;
    }
    const hostObj = hosts.find((h) => h.id === remoteHostId);
    const startPath = hostObj?.defaultSftpPath?.trim() || "~";
    loadRemoteDir(remoteHostId, startPath);
  }, [remoteHostId, hosts, loadRemoteDir]);

  const selectedLocalEntry = useMemo(
    () => localEntries.find((e) => e.path === localSelectedPath) ?? null,
    [localEntries, localSelectedPath],
  );

  const selectedRemoteEntry = useMemo(
    () => remoteEntries.find((e) => e.path === remoteSelectedPath) ?? null,
    [remoteEntries, remoteSelectedPath],
  );

  const filteredLocalEntries = useMemo(() => {
    const q = localSearch.trim().toLowerCase();
    return localEntries.filter((e) => {
      if (!localShowHidden && e.name.startsWith(".")) return false;
      if (!q) return true;
      return e.name.toLowerCase().includes(q);
    });
  }, [localEntries, localSearch, localShowHidden]);

  const filteredRemoteEntries = useMemo(() => {
    const q = remoteSearch.trim().toLowerCase();
    return remoteEntries.filter((e) => {
      if (!remoteShowHidden && e.name.startsWith(".")) return false;
      if (!q) return true;
      return e.name.toLowerCase().includes(q);
    });
  }, [remoteEntries, remoteSearch, remoteShowHidden]);

  const refreshSide = useCallback(
    async (side: Side) => {
      if (side === "local") {
        await loadLocalDir(localCwd || "~");
      } else if (remoteHostId) {
        await loadRemoteDir(remoteHostId, remoteCwd || "~");
      }
    },
    [localCwd, remoteCwd, remoteHostId, loadLocalDir, loadRemoteDir],
  );

  const handleTransfer = useCallback(
    async (direction: "upload" | "download", entryOverride?: SftpEntry) => {
      if (!remoteHostId) {
        setStatusMessage({
          type: "error",
          text: "Please select a remote SSH machine first.",
        });
        return;
      }

      const entry =
        direction === "upload"
          ? (entryOverride ?? selectedLocalEntry)
          : (entryOverride ?? selectedRemoteEntry);

      if (!entry) return;

      const sourceHostId = direction === "upload" ? "local" : remoteHostId;
      const targetHostId = direction === "upload" ? remoteHostId : "local";
      const targetDir = direction === "upload" ? remoteCwd || "/" : localCwd;

      setTransferBusy(true);
      setStatusMessage(null);
      setTransferProgress(null);
      try {
        const progressChannel = new Channel<SftpTransferProgress>();
        progressChannel.onmessage = (ev) => {
          setTransferProgress(ev);
        };
        const res = await invoke<SftpTransferResult>("sftp_transfer", {
          sourceHostId,
          sourcePath: entry.path,
          targetHostId,
          targetDir,
          preferFastSync: fastSyncEnabled,
          onProgress: progressChannel,
        });
        setLastTransferResult(res);
        setStatusMessage({
          type: "success",
          text:
            direction === "upload"
              ? `${t.sftpTransferCompleted}: "${entry.name}" → ${targetDir}`
              : `${t.sftpTransferCompleted}: "${entry.name}" → ${targetDir}`,
        });
        if (direction === "upload") {
          await loadRemoteDir(remoteHostId, remoteCwd || "~");
        } else {
          await loadLocalDir(localCwd || "~");
        }
      } catch (err) {
        setStatusMessage({
          type: "error",
          text: String(err),
        });
      } finally {
        setTransferBusy(false);
      }
    },
    [
      remoteHostId,
      selectedLocalEntry,
      selectedRemoteEntry,
      remoteCwd,
      localCwd,
      fastSyncEnabled,
      t.sftpTransferCompleted,
      loadRemoteDir,
      loadLocalDir,
    ],
  );

  const handleOpenTextEditor = useCallback(
    async (side: Side, entry: SftpEntry) => {
      const hostId = side === "local" ? "local" : remoteHostId;
      if (!hostId) return;
      setStatusMessage(null);
      try {
        const content = await invoke<string>("sftp_read_text", {
          hostId,
          path: entry.path,
        });
        setEditorModal({
          side,
          path: entry.path,
          fileName: entry.name,
          isNew: false,
          content,
          saving: false,
        });
      } catch (err) {
        setStatusMessage({
          type: "error",
          text: String(err),
        });
      }
    },
    [remoteHostId],
  );

  const handleCreateNewFile = useCallback(
    (side: Side) => {
      const baseDir = side === "local" ? localCwd : remoteCwd;
      if (!baseDir) return;
      setEditorModal({
        side,
        path: joinPath(baseDir, "untitled.txt"),
        fileName: "untitled.txt",
        isNew: true,
        content: "",
        saving: false,
      });
    },
    [localCwd, remoteCwd],
  );

  const handleSaveEditor = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!editorModal) return;
    const hostId = editorModal.side === "local" ? "local" : remoteHostId;
    if (!hostId) return;

    const baseDir = editorModal.side === "local" ? localCwd : remoteCwd;
    const finalPath = editorModal.isNew
      ? joinPath(baseDir, editorModal.fileName.trim() || "untitled.txt")
      : editorModal.path;

    setEditorModal({ ...editorModal, saving: true });
    try {
      await invoke("sftp_write_text", {
        hostId,
        path: finalPath,
        content: editorModal.content,
      });
      const side = editorModal.side;
      setEditorModal(null);
      setStatusMessage({
        type: "success",
        text: `Saved ${finalPath}`,
      });
      await refreshSide(side);
    } catch (err) {
      setEditorModal({ ...editorModal, saving: false });
      setStatusMessage({
        type: "error",
        text: String(err),
      });
    }
  };

  const handleConfirmMkdir = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!mkdirModal || !mkdirModal.name.trim()) return;
    const hostId = mkdirModal.side === "local" ? "local" : remoteHostId;
    const fullPath = joinPath(mkdirModal.baseDir, mkdirModal.name.trim());
    try {
      await invoke("sftp_mkdir", { hostId, path: fullPath });
      const side = mkdirModal.side;
      setMkdirModal(null);
      await refreshSide(side);
    } catch (err) {
      setStatusMessage({ type: "error", text: String(err) });
    }
  };

  const handleConfirmRename = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!renameModal || !renameModal.newName.trim()) return;
    const hostId = renameModal.side === "local" ? "local" : remoteHostId;
    const parent = parentPosixPath(renameModal.entry.path);
    const newPath = joinPath(parent, renameModal.newName.trim());
    try {
      await invoke("sftp_rename", {
        hostId,
        oldPath: renameModal.entry.path,
        newPath,
      });
      const side = renameModal.side;
      setRenameModal(null);
      await refreshSide(side);
    } catch (err) {
      setStatusMessage({ type: "error", text: String(err) });
    }
  };

  const handleConfirmChmod = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!chmodModal) return;
    const hostId = chmodModal.side === "local" ? "local" : remoteHostId;
    try {
      await invoke("sftp_chmod", {
        hostId,
        path: chmodModal.entry.path,
        mode: chmodModal.octal.trim(),
      });
      const side = chmodModal.side;
      setChmodModal(null);
      await refreshSide(side);
    } catch (err) {
      setStatusMessage({ type: "error", text: String(err) });
    }
  };

  const handleConfirmDelete = async () => {
    if (!deleteModal) return;
    const hostId = deleteModal.side === "local" ? "local" : remoteHostId;
    try {
      await invoke("sftp_delete", {
        hostId,
        path: deleteModal.entry.path,
      });
      const side = deleteModal.side;
      setDeleteModal(null);
      await refreshSide(side);
    } catch (err) {
      setStatusMessage({ type: "error", text: String(err) });
    }
  };

  const renderPane = (side: Side) => {
    const isLocal = side === "local";
    const cwd = isLocal ? localCwd : remoteCwd;
    const pathInput = isLocal ? localPathInput : remotePathInput;
    const setPathInput = isLocal ? setLocalPathInput : setRemotePathInput;
    const entries = isLocal ? filteredLocalEntries : filteredRemoteEntries;
    const loading = isLocal ? localLoading : remoteLoading;
    const error = isLocal ? localError : remoteError;
    const search = isLocal ? localSearch : remoteSearch;
    const setSearch = isLocal ? setLocalSearch : setRemoteSearch;
    const showHidden = isLocal ? localShowHidden : remoteShowHidden;
    const setShowHidden = isLocal ? setLocalShowHidden : setRemoteShowHidden;
    const selectedPath = isLocal ? localSelectedPath : remoteSelectedPath;
    const setSelectedPath = isLocal
      ? setLocalSelectedPath
      : setRemoteSelectedPath;
    const selectedEntry = isLocal ? selectedLocalEntry : selectedRemoteEntry;

    const navigateTo = (target: string) => {
      if (isLocal) {
        loadLocalDir(target);
      } else if (remoteHostId) {
        loadRemoteDir(remoteHostId, target);
      }
    };

    const disabledPane = !isLocal && !remoteHostId;

    return (
      <div className="flex flex-col min-w-0 min-h-0 flex-1 rounded-2xl border border-border/80 bg-card/65 overflow-hidden shadow-xs">
        {/* Pane Header */}
        <div className="h-12 px-3.5 border-b border-border/70 bg-header/60 flex items-center justify-between gap-2 shrink-0">
          <div className="flex items-center gap-2 min-w-0">
            <span className="p-1.5 rounded-xl bg-secondary border border-border/70 shrink-0">
              {isLocal ? (
                <Laptop className="w-4 h-4 text-emerald-400" />
              ) : (
                <Server className="w-4 h-4 text-primary" />
              )}
            </span>
            {isLocal ? (
              <div>
                <div className="text-xs font-semibold text-foreground">
                  Local System
                </div>
                <div className="text-[10px] text-muted-foreground font-mono truncate">
                  {cwd || "~"}
                </div>
              </div>
            ) : (
              <div className="w-60">
                <SelectField
                  value={remoteHostId}
                  onChange={(val) => setRemoteHostId(val)}
                  placeholder="Select SSH Machine..."
                  options={hosts.map((h) => ({
                    value: h.id,
                    label: `${h.alias || h.hostname} (${h.user}@${h.hostname})`,
                  }))}
                />
              </div>
            )}
          </div>

          {/* Primary Transfer Action */}
          <div className="flex items-center gap-1.5">
            {isLocal ? (
              <button
                type="button"
                disabled={!selectedEntry || !remoteHostId || transferBusy}
                onClick={() => handleTransfer("upload")}
                title="Upload selected item to Remote directory"
                className="h-8 px-3 rounded-xl bg-primary text-primary-foreground text-xs font-semibold inline-flex items-center gap-1.5 disabled:opacity-40 hover:opacity-95 cursor-pointer transition-all"
              >
                <span>Upload</span>
                <ArrowRight className="w-3.5 h-3.5" />
              </button>
            ) : (
              <button
                type="button"
                disabled={!selectedEntry || !remoteHostId || transferBusy}
                onClick={() => handleTransfer("download")}
                title="Download selected item to Local directory"
                className="h-8 px-3 rounded-xl bg-primary text-primary-foreground text-xs font-semibold inline-flex items-center gap-1.5 disabled:opacity-40 hover:opacity-95 cursor-pointer transition-all"
              >
                <ArrowLeft className="w-3.5 h-3.5" />
                <span>Download</span>
              </button>
            )}
          </div>
        </div>

        {/* Navigation & Path Bar */}
        <div className="px-3 py-2 border-b border-border/60 bg-background/40 flex items-center gap-1.5 shrink-0">
          <button
            type="button"
            disabled={disabledPane || loading}
            onClick={() => navigateTo(parentPosixPath(cwd || "/"))}
            title="Parent Directory (..)"
            className="p-1.5 rounded-lg bg-secondary/70 hover:bg-accent text-muted-foreground hover:text-foreground border border-border/60 disabled:opacity-40 cursor-pointer"
          >
            <ArrowUp className="w-3.5 h-3.5" />
          </button>
          <button
            type="button"
            disabled={disabledPane || loading}
            onClick={() => navigateTo("~")}
            title="Home Directory (~)"
            className="p-1.5 rounded-lg bg-secondary/70 hover:bg-accent text-muted-foreground hover:text-foreground border border-border/60 disabled:opacity-40 cursor-pointer"
          >
            <Home className="w-3.5 h-3.5" />
          </button>

          <form
            onSubmit={(e) => {
              e.preventDefault();
              navigateTo(pathInput.trim() || "/");
            }}
            className="flex-1 min-w-0"
          >
            <input
              type="text"
              disabled={disabledPane}
              value={pathInput}
              onChange={(e) => setPathInput(e.target.value)}
              placeholder="/path/to/directory"
              className="w-full h-7 px-2.5 rounded-lg bg-secondary/70 border border-border/70 text-xs font-mono text-foreground focus:outline-none focus:border-primary"
            />
          </form>

          <button
            type="button"
            disabled={disabledPane || loading}
            onClick={() => refreshSide(side)}
            title="Refresh Directory"
            className="p-1.5 rounded-lg bg-secondary/70 hover:bg-accent text-muted-foreground hover:text-foreground border border-border/60 disabled:opacity-40 cursor-pointer"
          >
            <RefreshCw
              className={`w-3.5 h-3.5 ${loading ? "animate-spin text-primary" : ""}`}
            />
          </button>
        </div>

        {/* Search & Action Toolbar */}
        <div className="px-3 py-2 border-b border-border/60 bg-card/40 flex flex-wrap items-center justify-between gap-2 shrink-0">
          <div className="flex items-center gap-1.5 flex-1 min-w-[140px]">
            <div className="relative flex-1">
              <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-2.5 top-1/2 -translate-y-1/2 pointer-events-none" />
              <input
                type="text"
                disabled={disabledPane}
                value={search}
                onChange={(e) => setSearch(e.target.value)}
                placeholder="Filter files..."
                className="w-full h-7 pl-8 pr-2.5 rounded-lg bg-secondary/60 border border-border/60 text-xs text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
              />
            </div>
            <button
              type="button"
              disabled={disabledPane}
              onClick={() => setShowHidden((v) => !v)}
              title={showHidden ? "Hide dotfiles" : "Show dotfiles (.*)"}
              className={`h-7 px-2 rounded-lg border text-[11px] font-medium inline-flex items-center gap-1 cursor-pointer transition-colors ${
                showHidden
                  ? "bg-primary/15 text-primary border-primary/40"
                  : "bg-secondary/60 text-muted-foreground border-border/60 hover:text-foreground"
              }`}
            >
              {showHidden ? (
                <Eye className="w-3.5 h-3.5" />
              ) : (
                <EyeOff className="w-3.5 h-3.5" />
              )}
              <span>.*</span>
            </button>
          </div>

          <div className="flex items-center gap-1">
            <button
              type="button"
              disabled={disabledPane || !cwd}
              onClick={() =>
                setMkdirModal({
                  side,
                  baseDir: cwd,
                  name: "",
                })
              }
              title="New Folder"
              className="h-7 px-2 rounded-lg bg-secondary/70 hover:bg-accent text-xs text-foreground border border-border/60 inline-flex items-center gap-1 disabled:opacity-40 cursor-pointer"
            >
              <FolderPlus className="w-3.5 h-3.5 text-amber-400" />
              <span className="hidden 2xl:inline">Folder</span>
            </button>

            <button
              type="button"
              disabled={disabledPane || !cwd}
              onClick={() => handleCreateNewFile(side)}
              title="New Text File"
              className="h-7 px-2 rounded-lg bg-secondary/70 hover:bg-accent text-xs text-foreground border border-border/60 inline-flex items-center gap-1 disabled:opacity-40 cursor-pointer"
            >
              <FilePlus className="w-3.5 h-3.5 text-emerald-400" />
              <span className="hidden 2xl:inline">File</span>
            </button>

            <button
              type="button"
              disabled={!selectedEntry || selectedEntry.kind === "dir"}
              onClick={() =>
                selectedEntry && handleOpenTextEditor(side, selectedEntry)
              }
              title="Edit Text File"
              className="h-7 px-2 rounded-lg bg-secondary/70 hover:bg-accent text-xs text-foreground border border-border/60 inline-flex items-center gap-1 disabled:opacity-40 cursor-pointer"
            >
              <Edit3 className="w-3.5 h-3.5 text-sky-400" />
            </button>

            <button
              type="button"
              disabled={!selectedEntry}
              onClick={() =>
                selectedEntry &&
                setRenameModal({
                  side,
                  entry: selectedEntry,
                  newName: selectedEntry.name,
                })
              }
              title="Rename"
              className="h-7 px-2 rounded-lg bg-secondary/70 hover:bg-accent text-xs text-foreground border border-border/60 inline-flex items-center gap-1 disabled:opacity-40 cursor-pointer"
            >
              <Type className="w-3.5 h-3.5 text-violet-400" />
            </button>

            <button
              type="button"
              disabled={!selectedEntry}
              onClick={() =>
                selectedEntry &&
                setChmodModal({
                  side,
                  entry: selectedEntry,
                  octal: selectedEntry.modeOctal,
                  bits: octalToBits(selectedEntry.modeOctal),
                })
              }
              title="Permissions (chmod)"
              className="h-7 px-2 rounded-lg bg-secondary/70 hover:bg-accent text-xs text-foreground border border-border/60 inline-flex items-center gap-1 disabled:opacity-40 cursor-pointer"
            >
              <Lock className="w-3.5 h-3.5 text-orange-400" />
            </button>

            <button
              type="button"
              disabled={!selectedEntry}
              onClick={() =>
                selectedEntry &&
                setDeleteModal({
                  side,
                  entry: selectedEntry,
                })
              }
              title="Delete"
              className="h-7 px-2 rounded-lg bg-secondary/70 hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 border border-border/60 inline-flex items-center gap-1 disabled:opacity-40 cursor-pointer"
            >
              <Trash2 className="w-3.5 h-3.5" />
            </button>
          </div>
        </div>

        {/* Error Alert inside Pane */}
        {error && (
          <div className="mx-3 mt-2 px-3 py-2 rounded-xl bg-rose-500/15 border border-rose-500/35 text-rose-300 text-xs flex items-center justify-between gap-2">
            <span className="truncate">{error}</span>
            <button
              type="button"
              onClick={() =>
                isLocal ? setLocalError(null) : setRemoteError(null)
              }
              className="p-0.5 rounded hover:bg-rose-500/20 cursor-pointer shrink-0"
            >
              <X className="w-3.5 h-3.5" />
            </button>
          </div>
        )}

        {/* File Table Region (Follows modern-web-guidance responsive-table) */}
        <div
          tabIndex={0}
          role="region"
          aria-label={
            isLocal ? "Local directory files" : "Remote directory files"
          }
          className="flex-1 overflow-auto min-h-0 @container"
        >
          {disabledPane ? (
            <div className="h-full flex flex-col items-center justify-center p-6 text-center text-muted-foreground text-xs gap-2">
              <Server className="w-8 h-8 text-muted-foreground/50" />
              <div>No SSH machine selected.</div>
              <div className="text-[11px] text-muted-foreground/75">
                Add a machine in the Machine tab or choose one from the selector
                above.
              </div>
            </div>
          ) : (
            <table className="w-full text-left border-separate border-spacing-0 text-xs">
              <thead className="sticky top-0 z-10 bg-header/95 backdrop-blur-xs text-[11px] uppercase tracking-wider text-muted-foreground">
                <tr>
                  <th
                    scope="col"
                    className="py-2 pl-3.5 pr-2 font-semibold border-b border-border/70"
                  >
                    Name
                  </th>
                  <th
                    scope="col"
                    className="py-2 px-2 font-semibold border-b border-border/70 text-right w-24"
                  >
                    Size
                  </th>
                  <th
                    scope="col"
                    className="py-2 px-2 font-semibold border-b border-border/70 w-28 hidden @md:table-cell"
                  >
                    Perms
                  </th>
                  <th
                    scope="col"
                    className="py-2 pl-2 pr-3.5 font-semibold border-b border-border/70 text-right w-32 hidden @lg:table-cell"
                  >
                    Modified
                  </th>
                </tr>
              </thead>
              <tbody className="divide-y divide-border/40">
                {cwd && cwd !== "/" && (
                  <tr
                    onDoubleClick={() => navigateTo(parentPosixPath(cwd))}
                    className="hover:bg-accent/50 cursor-pointer text-muted-foreground transition-colors"
                  >
                    <th
                      scope="row"
                      className="py-2 pl-3.5 pr-2 font-medium sticky left-0 bg-card/90 backdrop-blur-xs"
                    >
                      <div className="flex items-center gap-2">
                        <FolderOpen className="w-4 h-4 text-amber-400/80 shrink-0" />
                        <span className="font-mono">..</span>
                      </div>
                    </th>
                    <td className="py-2 px-2 text-right font-mono">—</td>
                    <td className="py-2 px-2 font-mono hidden @md:table-cell">
                      —
                    </td>
                    <td className="py-2 pl-2 pr-3.5 text-right hidden @lg:table-cell">
                      Parent
                    </td>
                  </tr>
                )}

                {entries.length === 0 && !loading ? (
                  <tr>
                    <td
                      colSpan={4}
                      className="py-10 text-center text-muted-foreground text-xs italic"
                    >
                      Directory is empty
                    </td>
                  </tr>
                ) : (
                  entries.map((entry) => {
                    const isSelected = selectedPath === entry.path;
                    return (
                      <tr
                        key={entry.path}
                        onClick={() => setSelectedPath(entry.path)}
                        onDoubleClick={() => {
                          if (entry.kind === "dir") {
                            navigateTo(entry.path);
                          } else {
                            handleOpenTextEditor(side, entry);
                          }
                        }}
                        className={`group cursor-pointer transition-colors ${
                          isSelected
                            ? "bg-primary/15 text-foreground"
                            : "hover:bg-accent/55 text-foreground/90"
                        }`}
                      >
                        <th
                          scope="row"
                          className={`py-2 pl-3.5 pr-2 font-medium sticky left-0 max-w-[260px] truncate ${
                            isSelected ? "bg-card" : "bg-card/90"
                          }`}
                        >
                          <div className="flex items-center justify-between gap-2 min-w-0">
                            <div className="flex items-center gap-2 min-w-0">
                              {entry.kind === "dir" ? (
                                <Folder className="w-4 h-4 text-amber-400 shrink-0" />
                              ) : entry.kind === "symlink" ? (
                                <Link2 className="w-4 h-4 text-sky-400 shrink-0" />
                              ) : (
                                <FileText className="w-4 h-4 text-muted-foreground shrink-0" />
                              )}
                              <span className="truncate" title={entry.name}>
                                {entry.name}
                              </span>
                            </div>

                            {/* Quick Hover Row Action */}
                            <div className="opacity-0 group-hover:opacity-100 flex items-center gap-1 shrink-0">
                              <button
                                type="button"
                                disabled={!remoteHostId || transferBusy}
                                onClick={(e) => {
                                  e.stopPropagation();
                                  handleTransfer(
                                    isLocal ? "upload" : "download",
                                    entry,
                                  );
                                }}
                                title={
                                  isLocal
                                    ? "Upload to Remote"
                                    : "Download to Local"
                                }
                                className="p-1 rounded bg-primary/20 text-primary hover:bg-primary hover:text-primary-foreground cursor-pointer transition-colors"
                              >
                                {isLocal ? (
                                  <ArrowRight className="w-3 h-3" />
                                ) : (
                                  <ArrowLeft className="w-3 h-3" />
                                )}
                              </button>
                            </div>
                          </div>
                        </th>
                        <td className="py-2 px-2 text-right font-mono tabular-nums text-[11px] text-muted-foreground">
                          {formatFileSize(entry.size, entry.kind)}
                        </td>
                        <td className="py-2 px-2 font-mono tabular-nums text-[11px] text-muted-foreground hidden @md:table-cell">
                          <span title={entry.permissions}>
                            {entry.modeOctal}
                          </span>{" "}
                          <span className="text-[10px] opacity-70">
                            {entry.permissions}
                          </span>
                        </td>
                        <td className="py-2 pl-2 pr-3.5 text-right font-mono tabular-nums text-[11px] text-muted-foreground hidden @lg:table-cell">
                          {formatModifiedDate(entry.modified)}
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          )}
        </div>

        {/* Pane Footer Status */}
        <div className="h-8 px-3.5 border-t border-border/60 bg-header/40 flex items-center justify-between text-[11px] text-muted-foreground shrink-0">
          <span>
            {entries.length} item{entries.length === 1 ? "" : "s"}
          </span>
          {selectedEntry && (
            <span className="font-mono truncate max-w-[65%]">
              {selectedEntry.name} ({selectedEntry.permissions})
            </span>
          )}
        </div>
      </div>
    );
  };

  return (
    <div className="flex flex-col h-full w-full overflow-hidden gap-3">
      {/* Fast Sync & Chunk Streaming Control Bar */}
      <div className="px-3.5 py-2 rounded-2xl border border-border/80 bg-card/70 flex flex-wrap items-center justify-between gap-2 shrink-0">
        <div className="flex items-center gap-2.5 min-w-0">
          <button
            type="button"
            onClick={() => setFastSyncEnabled((v) => !v)}
            className={`px-2.5 py-1 rounded-xl text-xs font-semibold inline-flex items-center gap-1.5 border transition-all cursor-pointer ${
              fastSyncEnabled
                ? "bg-emerald-500/15 text-emerald-300 border-emerald-500/40 shadow-2xs"
                : "bg-sky-500/15 text-sky-300 border-sky-500/40"
            }`}
          >
            <Zap
              className={`w-3.5 h-3.5 ${
                fastSyncEnabled ? "text-emerald-400" : "text-sky-400"
              }`}
            />
            <span>
              {fastSyncEnabled ? t.sftpFastSyncTag : t.sftpChunkStreamTag}
            </span>
          </button>
          <span className="text-xs text-muted-foreground truncate">
            {fastSyncEnabled
              ? t.sftpFastSyncActiveDesc
              : t.sftpChunkStreamActiveDesc}
          </span>
        </div>

        {(transferProgress || lastTransferResult) && (
          <div className="flex items-center gap-2 text-xs">
            {(() => {
              const activeMode =
                transferProgress?.mode ?? lastTransferResult?.mode;
              const activeDetail =
                transferProgress?.modeDetail ?? lastTransferResult?.modeDetail;
              const isFast = activeMode === "fast_sync";
              return (
                <span
                  className={`px-2 py-0.5 rounded-lg font-mono text-[10px] font-semibold inline-flex items-center gap-1 border ${
                    isFast
                      ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/40"
                      : "bg-sky-500/20 text-sky-300 border-sky-500/40"
                  }`}
                >
                  {isFast ? "⚡ FAST SYNC" : "📦 CHUNK STREAM"}
                  {activeDetail ? ` • ${activeDetail}` : ""}
                </span>
              );
            })()}
          </div>
        )}
      </div>

      {/* Live Chunk Transfer Progress & Status Banner */}
      {(statusMessage || transferBusy) && (
        <div
          className={`px-4 py-2.5 rounded-xl border text-xs flex flex-col gap-1.5 shrink-0 ${
            transferBusy
              ? "bg-primary/15 border-primary/35 text-primary"
              : statusMessage?.type === "error"
                ? "bg-rose-500/15 border-rose-500/35 text-rose-300"
                : "bg-emerald-500/15 border-emerald-500/35 text-emerald-300"
          }`}
        >
          <div className="flex items-center justify-between gap-2">
            <div className="flex items-center gap-2 truncate">
              {transferBusy ? (
                <RefreshCw className="w-3.5 h-3.5 animate-spin shrink-0" />
              ) : statusMessage?.type === "success" ? (
                <Check className="w-3.5 h-3.5 shrink-0" />
              ) : (
                <File className="w-3.5 h-3.5 shrink-0" />
              )}
              <span className="truncate font-medium">
                {transferBusy
                  ? `${t.sftpTransferProgress} ${transferProgress?.fileName ?? ""}`
                  : statusMessage?.text}
              </span>
              {transferProgress && (
                <span
                  className={`px-2 py-0.5 rounded-md font-mono text-[10px] font-bold border shrink-0 ${
                    transferProgress.mode === "fast_sync"
                      ? "bg-emerald-500/20 text-emerald-300 border-emerald-500/40"
                      : "bg-sky-500/20 text-sky-300 border-sky-500/40"
                  }`}
                >
                  {transferProgress.mode === "fast_sync"
                    ? t.sftpFastSyncTag
                    : t.sftpChunkStreamTag}
                </span>
              )}
            </div>
            <div className="flex items-center gap-2 shrink-0 font-mono text-[11px]">
              {transferProgress && (
                <span>
                  {formatFileSize(transferProgress.bytesTransferred, "file")} •{" "}
                  {formatFileSize(transferProgress.speedBytesPerSec, "file")}/s
                  ({transferProgress.percent}%)
                </span>
              )}
              {!transferBusy && (
                <button
                  type="button"
                  onClick={() => setStatusMessage(null)}
                  className="p-0.5 rounded hover:bg-black/20 cursor-pointer"
                >
                  <X className="w-3.5 h-3.5" />
                </button>
              )}
            </div>
          </div>
          {transferBusy && transferProgress && (
            <div className="w-full h-1.5 rounded-full bg-background/60 overflow-hidden">
              <div
                style={{ width: `${transferProgress.percent}%` }}
                className="h-full bg-primary transition-all duration-150"
              />
            </div>
          )}
        </div>
      )}

      {/* Dual-Pane Container */}
      <div className="flex-1 min-h-0 grid grid-cols-1 lg:grid-cols-2 gap-3.5">
        {renderPane("local")}
        {renderPane("remote")}
      </div>

      {/* 1. New Folder Modal */}
      {mkdirModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">Create New Folder</h3>
              <button
                type="button"
                onClick={() => setMkdirModal(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form onSubmit={handleConfirmMkdir} className="p-5 space-y-4 text-xs">
              <div>
                <label className="ui-label">Parent Directory</label>
                <div className="px-3 py-2 rounded-xl bg-secondary/50 border border-border/60 font-mono text-muted-foreground truncate">
                  {mkdirModal.baseDir}
                </div>
              </div>
              <div>
                <label className="ui-label">Folder Name *</label>
                <input
                  type="text"
                  required
                  autoFocus
                  value={mkdirModal.name}
                  onChange={(e) =>
                    setMkdirModal({ ...mkdirModal, name: e.target.value })
                  }
                  placeholder="new-folder"
                  className="ui-input"
                />
              </div>
              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setMkdirModal(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Create
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* 2. Rename Modal */}
      {renameModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">Rename Item</h3>
              <button
                type="button"
                onClick={() => setRenameModal(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleConfirmRename}
              className="p-5 space-y-4 text-xs"
            >
              <div>
                <label className="ui-label">New Name *</label>
                <input
                  type="text"
                  required
                  autoFocus
                  value={renameModal.newName}
                  onChange={(e) =>
                    setRenameModal({ ...renameModal, newName: e.target.value })
                  }
                  className="ui-input font-mono"
                />
              </div>
              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setRenameModal(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Rename
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* 3. Chmod Modal */}
      {chmodModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold">
                Permissions (chmod) — {chmodModal.entry.name}
              </h3>
              <button
                type="button"
                onClick={() => setChmodModal(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleConfirmChmod}
              className="p-5 space-y-4 text-xs"
            >
              <div className="grid grid-cols-3 gap-3">
                {(["Owner", "Group", "Others"] as const).map((groupLabel, gi) => (
                  <div
                    key={groupLabel}
                    className="p-3 rounded-xl bg-secondary/50 border border-border/60 space-y-2"
                  >
                    <div className="font-semibold text-foreground">
                      {groupLabel}
                    </div>
                    {(["Read", "Write", "Execute"] as const).map(
                      (permLabel, pi) => {
                        const bitIdx = gi * 3 + pi;
                        return (
                          <label
                            key={permLabel}
                            className="flex items-center gap-2 text-muted-foreground cursor-pointer"
                          >
                            <input
                              type="checkbox"
                              checked={chmodModal.bits[bitIdx]}
                              onChange={(e) => {
                                const nextBits = [...chmodModal.bits];
                                nextBits[bitIdx] = e.target.checked;
                                setChmodModal({
                                  ...chmodModal,
                                  bits: nextBits,
                                  octal: bitsToOctal(nextBits),
                                });
                              }}
                              className="rounded border-border"
                            />
                            <span>{permLabel}</span>
                          </label>
                        );
                      },
                    )}
                  </div>
                ))}
              </div>
              <div>
                <label className="ui-label">Octal Mode</label>
                <input
                  type="text"
                  required
                  value={chmodModal.octal}
                  onChange={(e) => {
                    const val = e.target.value;
                    setChmodModal({
                      ...chmodModal,
                      octal: val,
                      bits: octalToBits(val),
                    });
                  }}
                  placeholder="755"
                  className="ui-input font-mono"
                />
              </div>
              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setChmodModal(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  className="h-9 px-4 rounded-xl bg-primary text-primary-foreground font-semibold cursor-pointer"
                >
                  Apply
                </button>
              </div>
            </form>
          </div>
        </div>
      )}

      {/* 4. Delete Confirmation Modal */}
      {deleteModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between">
              <h3 className="text-sm font-semibold text-rose-400">
                Delete {deleteModal.entry.kind === "dir" ? "Directory" : "File"}
              </h3>
              <button
                type="button"
                onClick={() => setDeleteModal(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <div className="p-5 space-y-4 text-xs">
              <p className="text-muted-foreground leading-relaxed">
                Are you sure you want to permanently delete{" "}
                <strong className="text-foreground font-mono">
                  {deleteModal.entry.path}
                </strong>
                ?
              </p>
              <div className="flex justify-end gap-2 pt-2">
                <button
                  type="button"
                  onClick={() => setDeleteModal(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="button"
                  onClick={handleConfirmDelete}
                  className="h-9 px-4 rounded-xl bg-rose-500 text-white font-semibold hover:bg-rose-600 cursor-pointer"
                >
                  Delete
                </button>
              </div>
            </div>
          </div>
        </div>
      )}

      {/* 5. Text Editor Modal */}
      {editorModal && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-3xl rounded-2xl bg-popover border border-border shadow-2xl overflow-hidden flex flex-col max-h-[85vh]">
            <div className="px-5 py-3.5 border-b border-border flex items-center justify-between shrink-0">
              <div className="flex items-center gap-2 min-w-0">
                <FileText className="w-4 h-4 text-primary shrink-0" />
                <h3 className="text-sm font-semibold truncate">
                  {editorModal.isNew
                    ? `New File (${editorModal.side === "local" ? "Local" : "Remote"})`
                    : editorModal.path}
                </h3>
              </div>
              <button
                type="button"
                onClick={() => setEditorModal(null)}
                className="p-1 rounded-lg hover:bg-secondary cursor-pointer"
              >
                <X className="w-4 h-4" />
              </button>
            </div>
            <form
              onSubmit={handleSaveEditor}
              className="p-5 flex-1 flex flex-col gap-3.5 min-h-0 text-xs"
            >
              {editorModal.isNew && (
                <div>
                  <label className="ui-label">File Name *</label>
                  <input
                    type="text"
                    required
                    value={editorModal.fileName}
                    onChange={(e) =>
                      setEditorModal({
                        ...editorModal,
                        fileName: e.target.value,
                      })
                    }
                    className="ui-input font-mono"
                  />
                </div>
              )}
              <textarea
                value={editorModal.content}
                onChange={(e) =>
                  setEditorModal({ ...editorModal, content: e.target.value })
                }
                rows={18}
                spellCheck={false}
                className="ui-textarea font-mono text-xs leading-relaxed flex-1 min-h-[320px] resize-none"
              />
              <div className="flex justify-end gap-2 pt-2 border-t border-border shrink-0">
                <button
                  type="button"
                  onClick={() => setEditorModal(null)}
                  className="h-9 px-4 rounded-xl bg-secondary hover:bg-accent cursor-pointer"
                >
                  Cancel
                </button>
                <button
                  type="submit"
                  disabled={editorModal.saving}
                  className="h-9 px-5 rounded-xl bg-primary text-primary-foreground font-semibold disabled:opacity-50 cursor-pointer"
                >
                  {editorModal.saving ? "Saving..." : "Save File"}
                </button>
              </div>
            </form>
          </div>
        </div>
      )}
    </div>
  );
};
