import React, { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { WebglAddon } from "@xterm/addon-webgl";
import { Channel, invoke } from "@tauri-apps/api/core";
import {
  Columns,
  KeyRound,
  RefreshCw,
  Rows,
  ShieldCheck,
  Terminal as TerminalIcon,
  X,
} from "lucide-react";
import { LeafPaneNode, PaneStatus } from "../types";

interface TerminalPaneProps {
  leaf: LeafPaneNode;
  isActive: boolean;
  showFocusBorder: boolean;
  onFocus: () => void;
  onUpdateStatus: (
    status: PaneStatus,
    extra?: {
      errorMessage?: string;
      fingerprint?: string;
      passwordOverride?: string;
    },
  ) => void;
  onTriggerReconnect: (passwordOverride?: string) => void;
  onSplitVertical: () => void;
  onSplitHorizontal: () => void;
  onClosePane: () => void;
}

const textEncoder = new TextEncoder();

export const TerminalPane: React.FC<TerminalPaneProps> = ({
  leaf,
  isActive,
  showFocusBorder,
  onFocus,
  onUpdateStatus,
  onTriggerReconnect,
  onSplitVertical,
  onSplitHorizontal,
  onClosePane,
}) => {
  const containerRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const statusRef = useRef<PaneStatus>(leaf.status);
  const [retryPassword, setRetryPassword] = useState("");

  statusRef.current = leaf.status;

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const term = new Terminal({
      allowProposedApi: true,
      cursorBlink: true,
      cursorStyle: "block",
      fontSize: 13,
      lineHeight: 1.22,
      fontFamily:
        '"JetBrains Mono", "SF Mono", Menlo, Monaco, "Courier New", monospace',
      scrollback: 10000,
      macOptionIsMeta: true,
      theme: {
        background: "#0b0e14",
        foreground: "#e2e8f0",
        cursor: "#38bdf8",
        cursorAccent: "#0b0e14",
        selectionBackground: "rgba(56, 189, 248, 0.28)",
        black: "#1e293b",
        red: "#f87171",
        green: "#4ade80",
        yellow: "#facc15",
        blue: "#60a5fa",
        magenta: "#c084fc",
        cyan: "#22d3ee",
        white: "#f1f5f9",
        brightBlack: "#475569",
        brightRed: "#fca5a5",
        brightGreen: "#86efac",
        brightYellow: "#fde047",
        brightBlue: "#93c5fd",
        brightMagenta: "#d8b4fe",
        brightCyan: "#67e8f9",
        brightWhite: "#ffffff",
      },
    });

    const fitAddon = new FitAddon();
    const unicode11Addon = new Unicode11Addon();
    term.loadAddon(fitAddon);
    term.loadAddon(unicode11Addon);
    term.unicode.activeVersion = "11";

    term.open(el);

    try {
      const webglAddon = new WebglAddon();
      webglAddon.onContextLoss(() => {
        webglAddon.dispose();
      });
      term.loadAddon(webglAddon);
    } catch {
      // Fallback to default renderer
    }

    fitAddon.fit();
    termRef.current = term;
    fitRef.current = fitAddon;

    term.attachCustomKeyEventHandler((ev: KeyboardEvent) => {
      if (ev.type === "keydown" && ev.metaKey) {
        const key = ev.key.toLowerCase();
        if (key === "d" || key === "w" || key === "t" || key === "i") {
          return false;
        }
        if (key === "c" && term.hasSelection()) {
          navigator.clipboard
            .writeText(term.getSelection())
            .catch(() => undefined);
          term.clearSelection();
          return false;
        }
      }
      return true;
    });

    const inputDisposable = term.onData((data: string) => {
      if (
        (statusRef.current === "disconnected" ||
          statusRef.current === "error") &&
        (data === "\r" || data === "\n")
      ) {
        onTriggerReconnect();
        return;
      }
      const bytes = Array.from(textEncoder.encode(data));
      invoke("write_pane_input", {
        paneId: leaf.paneId,
        data: bytes,
      }).catch(() => undefined);
    });

    const binaryDisposable = term.onBinary((bin: string) => {
      const bytes = Array.from(Uint8Array.from(bin, (c) => c.charCodeAt(0)));
      invoke("write_pane_input", {
        paneId: leaf.paneId,
        data: bytes,
      }).catch(() => undefined);
    });

    let rafId = 0;
    const resizeObserver = new ResizeObserver(() => {
      cancelAnimationFrame(rafId);
      rafId = requestAnimationFrame(() => {
        if (!fitRef.current || !termRef.current) return;
        try {
          fitRef.current.fit();
          const cols = Math.max(20, termRef.current.cols);
          const rows = Math.max(5, termRef.current.rows);
          invoke("resize_pane", {
            paneId: leaf.paneId,
            cols,
            rows,
          }).catch(() => undefined);
        } catch {
          // ignore transient layout errors
        }
      });
    });
    resizeObserver.observe(el);

    return () => {
      cancelAnimationFrame(rafId);
      resizeObserver.disconnect();
      inputDisposable.dispose();
      binaryDisposable.dispose();
      invoke("close_pane", { paneId: leaf.paneId }).catch(() => undefined);
      term.dispose();
      termRef.current = null;
      fitRef.current = null;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [leaf.paneId]);

  useEffect(() => {
    const term = termRef.current;
    const fitAddon = fitRef.current;
    if (!term || !fitAddon) return;

    let cancelled = false;
    fitAddon.fit();
    const cols = Math.max(20, term.cols || 80);
    const rows = Math.max(5, term.rows || 24);

    const channel = new Channel<number[] | Uint8Array>();
    channel.onmessage = (payload) => {
      if (cancelled || !termRef.current) return;
      const bytes =
        payload instanceof Uint8Array ? payload : new Uint8Array(payload);
      termRef.current.write(bytes);
    };

    onUpdateStatus("connecting", { errorMessage: undefined });

    if (leaf.kind === "local") {
      invoke("spawn_local_pty", {
        paneId: leaf.paneId,
        cols,
        rows,
        onData: channel,
      })
        .then(() => {
          if (!cancelled) {
            onUpdateStatus("connected");
            term.focus();
          }
        })
        .catch((err) => {
          if (!cancelled) {
            const msg = String(err);
            term.writeln(`\r\n\x1b[31m[Local PTY Error] ${msg}\x1b[0m`);
            onUpdateStatus("error", { errorMessage: msg });
          }
        });
    } else if (leaf.host) {
      if (leaf.reconnectNonce > 0) {
        term.writeln(
          `\r\n\x1b[36m[Reconnecting to ${leaf.host.user}@${leaf.host.hostname}:${leaf.host.port}...]\x1b[0m`,
        );
      } else {
        term.writeln(
          `\x1b[90mConnecting to ${leaf.host.user}@${leaf.host.hostname}:${leaf.host.port}...\x1b[0m`,
        );
      }
      invoke<{ fingerprint: string }>("connect_ssh_pane", {
        paneId: leaf.paneId,
        host: leaf.host,
        cols,
        rows,
        password: leaf.passwordOverride ?? null,
        onData: channel,
      })
        .then((res) => {
          if (!cancelled) {
            onUpdateStatus("connected", {
              fingerprint: res.fingerprint,
              errorMessage: undefined,
            });
            term.focus();
          }
        })
        .catch((err) => {
          if (!cancelled) {
            const msg = String(err);
            term.writeln(`\r\n\x1b[31m[SSH Error] ${msg}\x1b[0m`);
            onUpdateStatus("error", { errorMessage: msg });
          }
        });
    }

    return () => {
      cancelled = true;
    };
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [leaf.paneId, leaf.reconnectNonce]);

  useEffect(() => {
    if (isActive && termRef.current && leaf.status === "connected") {
      termRef.current.focus();
    }
  }, [isActive, leaf.status]);

  const handlePasswordReconnect = (e: React.FormEvent) => {
    e.preventDefault();
    onTriggerReconnect(retryPassword || undefined);
    setRetryPassword("");
  };

  return (
    <div
      onMouseDown={onFocus}
      className={`relative flex flex-col h-full w-full bg-[#0b0e14] overflow-hidden transition-colors ${
        showFocusBorder
          ? isActive
            ? "ring-1 ring-inset ring-cyan-500/60"
            : "ring-1 ring-inset ring-slate-800/80 opacity-90"
          : ""
      }`}
    >
      {/* Pane Micro-Header */}
      <div className="flex items-center justify-between h-7 px-2.5 bg-[#0f131c] border-b border-slate-800/80 text-[11px] text-slate-400 shrink-0">
        <div className="flex items-center gap-2 min-w-0">
          <span
            className={`h-2 w-2 rounded-full shrink-0 ${
              leaf.status === "connected"
                ? "bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.7)]"
                : leaf.status === "connecting"
                  ? "bg-amber-400 animate-pulse"
                  : "bg-rose-500"
            }`}
          />
          <TerminalIcon className="w-3 h-3 text-slate-400 shrink-0" />
          <span className="font-medium text-slate-200 truncate">
            {leaf.title}
          </span>
          {leaf.fingerprint && (
            <span
              title={`Host Key: ${leaf.fingerprint}`}
              className="hidden sm:inline-flex items-center gap-1 px-1.5 py-0.5 rounded bg-emerald-500/10 text-emerald-300 border border-emerald-500/20 font-mono text-[10px] truncate max-w-[180px]"
            >
              <ShieldCheck className="w-2.5 h-2.5 shrink-0" />
              {leaf.fingerprint}
            </span>
          )}
        </div>

        <div className="flex items-center gap-1">
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSplitVertical();
            }}
            title="Split Vertically (Cmd+D)"
            className="p-1 hover:bg-slate-800 hover:text-slate-100 rounded transition-colors"
          >
            <Columns className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSplitHorizontal();
            }}
            title="Split Horizontally (Cmd+Shift+D)"
            className="p-1 hover:bg-slate-800 hover:text-slate-100 rounded transition-colors"
          >
            <Rows className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onClosePane();
            }}
            title="Close Pane (Cmd+W)"
            className="p-1 hover:bg-rose-500/20 hover:text-rose-300 rounded transition-colors"
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      </div>

      {/* Reconnect / Auth Banner Overlay when Disconnected or Error */}
      {(leaf.status === "disconnected" || leaf.status === "error") && (
        <div className="z-10 flex flex-wrap items-center justify-between gap-2 px-3 py-2 bg-rose-950/90 border-b border-rose-500/40 text-xs text-rose-100 backdrop-blur-sm">
          <div className="flex-1 min-w-[200px] truncate">
            <span className="font-semibold text-rose-300 mr-1.5">
              Connection Interrupted:
            </span>
            <span className="text-rose-200/90">
              {leaf.errorMessage || "Session closed by remote host"}
            </span>
          </div>
          <form
            onSubmit={handlePasswordReconnect}
            className="flex items-center gap-1.5"
          >
            {leaf.kind === "ssh" && (
              <div className="relative flex items-center">
                <KeyRound className="w-3 h-3 text-rose-300/70 absolute left-2 pointer-events-none" />
                <input
                  type="password"
                  value={retryPassword}
                  onChange={(e) => setRetryPassword(e.target.value)}
                  placeholder="Password / Passphrase (optional)"
                  className="pl-6 pr-2 py-1 rounded bg-slate-900/90 border border-rose-400/30 text-[11px] text-slate-100 placeholder:text-slate-500 focus:outline-none focus:border-cyan-400 w-48"
                />
              </div>
            )}
            <button
              type="submit"
              className="inline-flex items-center gap-1 px-2.5 py-1 rounded bg-cyan-500 hover:bg-cyan-400 text-slate-950 font-semibold text-[11px] transition-colors cursor-pointer"
            >
              <RefreshCw className="w-3 h-3" />
              Reconnect (Enter)
            </button>
          </form>
        </div>
      )}

      {/* Xterm.js WebGL2 Surface */}
      <div
        ref={containerRef}
        className="flex-1 w-full h-full px-1.5 py-1 overflow-hidden select-text"
      />
    </div>
  );
};
