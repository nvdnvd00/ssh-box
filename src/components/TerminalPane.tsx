import React, { useEffect, useRef, useState } from "react";
import { Terminal } from "@xterm/xterm";
import { FitAddon } from "@xterm/addon-fit";
import { Unicode11Addon } from "@xterm/addon-unicode11";
import { Channel, invoke } from "@tauri-apps/api/core";
import {
  ArrowDown,
  ArrowUp,
  Columns,
  Download,
  Keyboard,
  KeyRound,
  Radio,
  RefreshCw,
  Rows,
  Search,
  ShieldCheck,
  Terminal as TerminalIcon,
  X,
} from "lucide-react";
import { LeafPaneNode, PaneStatus } from "../types";
import {
  AppLanguage,
  loadAppSettings,
  resolveTerminalFontCss,
  resolveXtermTheme,
  ThemeConfig,
} from "../themes";
import { getI18n } from "../i18n";
import { isApplePlatform, matchShortcut, withShortcutHint } from "../shortcuts";

interface TerminalPaneProps {
  leaf: LeafPaneNode;
  isActive: boolean;
  showFocusBorder: boolean;
  themeConfig: ThemeConfig;
  isVisible?: boolean;
  language?: AppLanguage;
  copyOnSelect?: boolean;
  broadcastTargetPaneIds?: string[];
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
const textDecoder = new TextDecoder();

function stripAnsiCodes(raw: string): string {
  // eslint-disable-next-line no-control-regex
  return raw.replace(/\x1b\[[0-9;?]*[ -/]*[@-~]|\x1b\][^\x07]*(\x07|\x1b\\)/g, "");
}

export function computeImeDiff(
  prevRaw: string,
  nextRaw: string,
): { backspaces: number; insertText: string } {
  const prev = Array.from(prevRaw.normalize("NFC"));
  const next = Array.from(nextRaw.normalize("NFC"));

  let common = 0;
  const minLen = Math.min(prev.length, next.length);
  while (common < minLen && prev[common] === next[common]) {
    common++;
  }

  return {
    backspaces: prev.length - common,
    insertText: next.slice(common).join(""),
  };
}

export const TerminalPane: React.FC<TerminalPaneProps> = ({
  leaf,
  isActive,
  showFocusBorder,
  themeConfig,
  isVisible = true,
  language = "vi",
  copyOnSelect = false,
  broadcastTargetPaneIds,
  onFocus,
  onUpdateStatus,
  onTriggerReconnect,
  onSplitVertical,
  onSplitHorizontal,
  onClosePane,
}) => {
  const t = getI18n(language);
  const containerRef = useRef<HTMLDivElement | null>(null);
  const termRef = useRef<Terminal | null>(null);
  const fitRef = useRef<FitAddon | null>(null);
  const statusRef = useRef<PaneStatus>(leaf.status);
  const trackedInputRef = useRef<string>("");
  const broadcastTargetsRef = useRef<string[] | undefined>(
    broadcastTargetPaneIds,
  );
  broadcastTargetsRef.current = broadcastTargetPaneIds;
  const copyOnSelectRef = useRef(copyOnSelect);
  copyOnSelectRef.current = copyOnSelect;

  const [retryPassword, setRetryPassword] = useState("");
  const [searchOpen, setSearchOpen] = useState(false);
  const [searchQuery, setSearchQuery] = useState("");
  const [searchMatchInfo, setSearchMatchInfo] = useState<string>("");
  const searchCursorRef = useRef<{ row: number; col: number }>({
    row: -1,
    col: -1,
  });
  const searchInputRef = useRef<HTMLInputElement | null>(null);

  const [isRecordingLog, setIsRecordingLog] = useState(false);
  const isRecordingLogRef = useRef(false);
  isRecordingLogRef.current = isRecordingLog;
  const recordingFileNameRef = useRef<string>("");
  const [savedLogNotice, setSavedLogNotice] = useState<string | null>(null);
  const [showExtraKeys, setShowExtraKeys] = useState<boolean>(() => {
    if (typeof window === "undefined") return false;
    return (
      (navigator.maxTouchPoints ?? 0) > 0 ||
      window.matchMedia?.("(pointer: coarse)").matches ||
      window.innerWidth < 900
    );
  });
  const [ctrlArmed, setCtrlArmed] = useState(false);
  const [altArmed, setAltArmed] = useState(false);
  const ctrlArmedRef = useRef(false);
  const altArmedRef = useRef(false);
  const resolvedXtermTheme = resolveXtermTheme(themeConfig);

  ctrlArmedRef.current = ctrlArmed;
  altArmedRef.current = altArmed;
  statusRef.current = leaf.status;

  const sendVirtualKey = (seq: string) => {
    const term = termRef.current;
    if (term?.hasSelection()) {
      term.clearSelection();
    }
    let out = seq;
    if (ctrlArmedRef.current && seq.length === 1) {
      const code = seq.toUpperCase().charCodeAt(0);
      if (code >= 64 && code <= 95) {
        out = String.fromCharCode(code - 64);
      }
      setCtrlArmed(false);
    }
    if (altArmedRef.current) {
      out = "\x1b" + out;
      setAltArmed(false);
    }
    const bytes = Array.from(textEncoder.encode(out));
    const targets =
      broadcastTargetsRef.current && broadcastTargetsRef.current.length > 0
        ? broadcastTargetsRef.current
        : [leaf.paneId];
    for (const targetPaneId of targets) {
      invoke("write_pane_input", {
        paneId: targetPaneId,
        data: bytes,
      }).catch(() => undefined);
    }
    term?.focus();
  };

  const runBufferSearch = (query: string, direction: "next" | "prev") => {
    const term = termRef.current;
    if (!term || !query.trim()) {
      setSearchMatchInfo("");
      return;
    }
    const q = query.toLowerCase();
    const buf = term.buffer.active;
    const totalLines = buf.length;
    const matches: { row: number; col: number }[] = [];

    for (let r = 0; r < totalLines; r++) {
      const lineStr = buf.getLine(r)?.translateToString(true) ?? "";
      const lower = lineStr.toLowerCase();
      let idx = lower.indexOf(q);
      while (idx !== -1) {
        matches.push({ row: r, col: idx });
        idx = lower.indexOf(q, idx + 1);
      }
    }

    if (matches.length === 0) {
      setSearchMatchInfo("0/0");
      term.clearSelection();
      return;
    }

    const cur = searchCursorRef.current;
    let pickIdx = 0;
    if (direction === "next") {
      const found = matches.findIndex(
        (m) => m.row > cur.row || (m.row === cur.row && m.col > cur.col),
      );
      pickIdx = found !== -1 ? found : 0;
    } else {
      let found = -1;
      for (let i = matches.length - 1; i >= 0; i--) {
        if (
          matches[i].row < cur.row ||
          (matches[i].row === cur.row && matches[i].col < cur.col)
        ) {
          found = i;
          break;
        }
      }
      pickIdx = found !== -1 ? found : matches.length - 1;
    }

    const chosen = matches[pickIdx];
    searchCursorRef.current = chosen;
    setSearchMatchInfo(`${pickIdx + 1}/${matches.length}`);
    term.select(chosen.col, chosen.row, query.length);
    term.scrollToLine(Math.max(0, chosen.row - 4));
  };

  const handleToggleLiveLog = () => {
    if (isRecordingLog) {
      setIsRecordingLog(false);
    } else {
      const safeTitle = leaf.title.replace(/[^a-zA-Z0-9._-]/g, "_");
      recordingFileNameRef.current = `${safeTitle}-${Date.now()}.log`;
      setIsRecordingLog(true);
    }
  };

  const handleExportSessionLog = async () => {
    const term = termRef.current;
    if (!term) return;
    const buf = term.buffer.active;
    const lines: string[] = [];
    for (let r = 0; r < buf.length; r++) {
      const line = buf.getLine(r)?.translateToString(true) ?? "";
      lines.push(line);
    }
    const content = lines.join("\n").trimEnd() + "\n";
    try {
      const savedPath = await invoke<string>("save_session_log", {
        hostLabel: leaf.title,
        content,
        append: false,
      });
      setSavedLogNotice(savedPath);
      setTimeout(() => setSavedLogNotice(null), 4000);
    } catch {
      // ignore
    }
  };

  useEffect(() => {
    const el = containerRef.current;
    if (!el) return;

    const term = new Terminal({
      allowProposedApi: true,
      cursorBlink: true,
      cursorStyle: themeConfig.cursorStyle,
      fontSize: themeConfig.fontSize,
      lineHeight: themeConfig.lineHeight || 1.2,
      letterSpacing: themeConfig.letterSpacing ?? 0,
      fontFamily: resolveTerminalFontCss(themeConfig.fontFamily),
      scrollback: 10000,
      macOptionIsMeta: true,
      theme: resolveXtermTheme(themeConfig),
    });

    const fitAddon = new FitAddon();
    const unicode11Addon = new Unicode11Addon();
    term.loadAddon(fitAddon);
    term.loadAddon(unicode11Addon);
    term.unicode.activeVersion = "11";

    term.open(el);
    fitAddon.fit();
    termRef.current = term;
    fitRef.current = fitAddon;

    // Guard xterm.js SelectionService against macOS WKWebView swallowed mouseup events:
    // xterm's _handleMouseMove does not check `e.buttons`, so if WKWebView or NSTextInputContext
    // consumes `mouseup`, moving the cursor with no button pressed would keep highlighting.
    const selectionService = (
      term as unknown as {
        _core?: {
          _selectionService?: {
            _handleMouseMove?: (e: MouseEvent) => void;
            _addMouseDownListeners?: () => void;
            _removeMouseDownListeners?: () => void;
            refresh?: (isLinuxMouseSelection?: boolean) => void;
            _model?: {
              selectionStart?: [number, number];
              selectionEnd?: [number, number];
            };
          };
        };
      }
    )._core?._selectionService;

    if (selectionService) {
      if (typeof selectionService._addMouseDownListeners === "function") {
        const origAddListeners =
          selectionService._addMouseDownListeners.bind(selectionService);
        selectionService._addMouseDownListeners = () => {
          selectionService._removeMouseDownListeners?.();
          origAddListeners();
        };
      }

      if (typeof selectionService._handleMouseMove === "function") {
        const origMouseMove =
          selectionService._handleMouseMove.bind(selectionService);
        selectionService._handleMouseMove = (e: MouseEvent) => {
          if ((e.buttons & 1) === 0) {
            selectionService._removeMouseDownListeners?.();
            if (
              selectionService._model &&
              !selectionService._model.selectionEnd
            ) {
              selectionService._model.selectionStart = undefined;
              selectionService.refresh?.(false);
            }
            return;
          }
          origMouseMove(e);
        };
      }
    }

    const onWindowBlur = () => {
      selectionService?._removeMouseDownListeners?.();
    };
    window.addEventListener("blur", onWindowBlur);

    // Copy on mouseup rather than onSelectionChange: the latter fires on every drag tick
    // and on search highlights (term.select), which would clobber the clipboard.
    // hasSelection() can be true over blank cells, so check the text to avoid wiping the clipboard.
    const onMouseUpCopy = () => {
      const text = copyOnSelectRef.current ? term.getSelection() : "";
      if (text) {
        navigator.clipboard.writeText(text).catch(() => undefined);
      }
    };
    el.addEventListener("mouseup", onMouseUpCopy);

    const sendRawStringToPty = (str: string) => {
      if (!str) return;
      if (term.hasSelection()) {
        term.clearSelection();
      }
      let finalStr = str;
      if (ctrlArmedRef.current && str.length === 1) {
        const code = str.toUpperCase().charCodeAt(0);
        if (code >= 64 && code <= 95) {
          finalStr = String.fromCharCode(code - 64);
        }
        ctrlArmedRef.current = false;
        setCtrlArmed(false);
      }
      if (altArmedRef.current) {
        finalStr = "\x1b" + finalStr;
        altArmedRef.current = false;
        setAltArmed(false);
      }
      const bytes = Array.from(textEncoder.encode(finalStr));
      const targets =
        broadcastTargetsRef.current && broadcastTargetsRef.current.length > 0
          ? broadcastTargetsRef.current
          : [leaf.paneId];
      for (const targetPaneId of targets) {
        invoke("write_pane_input", {
          paneId: targetPaneId,
          data: bytes,
        }).catch(() => undefined);
      }
    };

    const resetTextareaBuffer = (resetImeContext = false) => {
      const hadText =
        trackedInputRef.current.length > 0 ||
        Boolean(term.textarea && term.textarea.value.length > 0);
      trackedInputRef.current = "";
      if (term.textarea) {
        term.textarea.value = "";
        if (resetImeContext && hadText) {
          term.textarea.blur();
          term.textarea.focus({ preventScroll: true });
        }
      }
    };

    // Intercept composition & input events in capture phase before xterm's CompositionHelper drops/locks them on macOS WKWebView
    const onCompositionEvent = (e: Event) => {
      if (e.target === term.textarea) {
        e.stopPropagation();
      }
    };

    const onTextareaInput = (e: Event) => {
      if (!term.textarea || e.target !== term.textarea) return;
      e.stopPropagation();

      // xterm's paste handler already sent the clipboard text; Chromium (WebView2) then also
      // inserts it into the textarea, which would send it a second time.
      if ((e as InputEvent).inputType === "insertFromPaste") {
        term.textarea.value = "";
        trackedInputRef.current = "";
        return;
      }

      const rawValue = term.textarea.value;
      if (rawValue.includes("\n") || rawValue.includes("\r")) {
        const cleanValue = rawValue.replace(/[\r\n]+/g, "");
        const { backspaces, insertText } = computeImeDiff(
          trackedInputRef.current,
          cleanValue,
        );
        const delta = "\x7f".repeat(backspaces) + insertText;
        if (delta) {
          sendRawStringToPty(delta);
        }
        resetTextareaBuffer(true);
        if (
          statusRef.current === "disconnected" ||
          statusRef.current === "error"
        ) {
          onTriggerReconnect();
        } else {
          sendRawStringToPty("\r");
        }
        return;
      }

      const { backspaces, insertText } = computeImeDiff(
        trackedInputRef.current,
        rawValue,
      );
      trackedInputRef.current = rawValue;

      const payload = "\x7f".repeat(backspaces) + insertText;
      if (payload) {
        sendRawStringToPty(payload);
      }

      if (rawValue.endsWith(" ")) {
        trackedInputRef.current = "";
        term.textarea.value = "";
      }
    };

    const onPasteReset = () => {
      trackedInputRef.current = "";
    };

    el.addEventListener("compositionstart", onCompositionEvent, true);
    el.addEventListener("compositionupdate", onCompositionEvent, true);
    el.addEventListener("compositionend", onCompositionEvent, true);
    el.addEventListener("input", onTextareaInput, true);
    el.addEventListener("paste", onPasteReset, true);

    term.attachCustomKeyEventHandler((ev: KeyboardEvent) => {
      // A matched shortcut never reaches xterm. Terminal actions run here; returning false
      // doesn't stop propagation, so App-level actions still reach App's window listener.
      const action = matchShortcut(ev);
      if (action) {
        if (ev.type === "keydown") {
          switch (action) {
            case "copy":
              ev.preventDefault();
              if (term.hasSelection()) {
                navigator.clipboard
                  .writeText(term.getSelection())
                  .catch(() => undefined);
                term.clearSelection();
              }
              break;
            case "selectAll":
              ev.preventDefault();
              term.selectAll();
              break;
            case "find":
              ev.preventDefault();
              setSearchOpen(true);
              setTimeout(() => searchInputRef.current?.focus(), 20);
              break;
            case "clear":
              ev.preventDefault();
              term.clear();
              break;
            case "lineStart":
            case "lineEnd":
            case "deleteLine":
              ev.preventDefault();
              resetTextareaBuffer(true);
              // readline: ^A start of line, ^E end of line, ^U kill to start
              sendRawStringToPty(
                { lineStart: "\x01", lineEnd: "\x05", deleteLine: "\x15" }[action],
              );
              break;
            // "paste" keeps the browser default: the OS pastes into xterm's textarea.
          }
        }
        return false;
      }
      if (ev.metaKey) return true;

      // Windows/Linux have no Cmd key: like Windows Terminal, Ctrl(+Shift)+C copies when text is
      // selected (otherwise ^C reaches the shell) and Ctrl(+Shift)+V pastes. Returning false skips
      // xterm, which would send ^V and preventDefault the browser's native paste.
      if (!isApplePlatform && ev.ctrlKey && !ev.altKey && ev.type === "keydown") {
        const key = ev.key.toLowerCase();
        if (key === "c" && term.hasSelection()) {
          navigator.clipboard
            .writeText(term.getSelection())
            .catch(() => undefined);
          term.clearSelection();
          return false;
        }
        if (key === "v") {
          return false;
        }
      }

      const isControlOrNavCode =
        ev.code === "Enter" ||
        ev.code === "NumpadEnter" ||
        ev.code === "Tab" ||
        ev.code === "Escape" ||
        ev.code === "ArrowUp" ||
        ev.code === "ArrowDown" ||
        ev.code === "ArrowLeft" ||
        ev.code === "ArrowRight" ||
        ev.code === "Home" ||
        ev.code === "End" ||
        ev.code === "PageUp" ||
        ev.code === "PageDown" ||
        ev.code === "Delete" ||
        /^F([1-9]|1[0-2])$/.test(ev.code);

      if (ev.ctrlKey || ev.altKey || isControlOrNavCode) {
        if (ev.type === "keydown") {
          resetTextareaBuffer(true);
          if (
            (ev.code === "Enter" || ev.code === "NumpadEnter") &&
            (ev.keyCode === 229 || ev.isComposing)
          ) {
            ev.preventDefault();
            if (
              statusRef.current === "disconnected" ||
              statusRef.current === "error"
            ) {
              onTriggerReconnect();
            } else {
              sendRawStringToPty("\r");
            }
            return false;
          }
        }
        return true;
      }

      const isBackspace =
        ev.code === "Backspace" || ev.key === "Backspace" || ev.keyCode === 8;
      if (isBackspace) {
        if (trackedInputRef.current.length > 0) {
          // Let WKWebView + Vietnamese Telex natively delete inside textarea and trigger `input` diff
          return false;
        }
        if (ev.type === "keydown" && (ev.keyCode === 229 || ev.isComposing)) {
          ev.preventDefault();
          sendRawStringToPty("\x7f");
          return false;
        }
        return true;
      }

      const isImeOrPrintable =
        ev.isComposing ||
        ev.keyCode === 229 ||
        ev.key === "Dead" ||
        ev.key.length === 1;

      if (isImeOrPrintable) {
        // Do NOT call preventDefault() in xterm.js; let macOS WKWebView & Telex update term.textarea natively
        return false;
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
      sendRawStringToPty(data);
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
      window.removeEventListener("blur", onWindowBlur);
      el.removeEventListener("mouseup", onMouseUpCopy);
      el.removeEventListener("compositionstart", onCompositionEvent, true);
      el.removeEventListener("compositionupdate", onCompositionEvent, true);
      el.removeEventListener("compositionend", onCompositionEvent, true);
      el.removeEventListener("input", onTextareaInput, true);
      el.removeEventListener("paste", onPasteReset, true);
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
      if (isRecordingLogRef.current && recordingFileNameRef.current) {
        const cleanText = stripAnsiCodes(
          textDecoder.decode(bytes, { stream: true }),
        );
        if (cleanText) {
          invoke("save_session_log", {
            hostLabel: leaf.title,
            content: cleanText,
            append: true,
            fileName: recordingFileNameRef.current,
          }).catch(() => undefined);
        }
      }
    };

    onUpdateStatus("connecting", { errorMessage: undefined });

    // "\r" is what Enter sends, so this also works in PowerShell/cmd via ConPTY
    const sendStartupCommand = () => {
      if (!leaf.startupCommand) return;
      invoke("write_pane_input", {
        paneId: leaf.paneId,
        data: Array.from(textEncoder.encode(`${leaf.startupCommand}\r`)),
      }).catch(() => undefined);
    };

    const currentSettings = loadAppSettings();

    if (leaf.kind === "local") {
      invoke("spawn_local_pty", {
        paneId: leaf.paneId,
        cols,
        rows,
        // Read at spawn time so changing the setting never restarts open panes
        shell: currentSettings.localShell || null,
        useTmux: !leaf.startupCommand && currentSettings.tmuxLocalEnabled,
        onData: channel,
      })
        .then(() => {
          if (!cancelled) {
            onUpdateStatus("connected");
            sendStartupCommand();
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
        useTmux: !leaf.startupCommand && currentSettings.tmuxSshEnabled,
        onData: channel,
      })
        .then((res) => {
          if (!cancelled) {
            onUpdateStatus("connected", {
              fingerprint: res.fingerprint,
              errorMessage: undefined,
            });
            sendStartupCommand();
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

  // Dynamically apply theme, font family, font size, line height, letter spacing, and cursor style changes without re-creating the PTY/SSH session
  useEffect(() => {
    const term = termRef.current;
    const fitAddon = fitRef.current;
    if (!term) return;

    const nextTheme = resolveXtermTheme(themeConfig);
    term.options.theme = { ...nextTheme };
    term.options.cursorStyle = themeConfig.cursorStyle;

    if (!isVisible) {
      return;
    }

    const nextFontCss = resolveTerminalFontCss(themeConfig.fontFamily);
    const nextLineHeight = themeConfig.lineHeight || 1.2;
    const nextLetterSpacing = themeConfig.letterSpacing ?? 0;

    const layoutChanged =
      term.options.fontSize !== themeConfig.fontSize ||
      term.options.fontFamily !== nextFontCss ||
      term.options.lineHeight !== nextLineHeight ||
      term.options.letterSpacing !== nextLetterSpacing;

    if (layoutChanged) {
      term.options.fontSize = themeConfig.fontSize;
      term.options.fontFamily = nextFontCss;
      term.options.lineHeight = nextLineHeight;
      term.options.letterSpacing = nextLetterSpacing;
    }

    try {
      term.refresh(0, Math.max(0, term.rows - 1));
    } catch {
      // ignore refresh error before open
    }

    requestAnimationFrame(() => {
      if (!fitAddon || !termRef.current) return;
      try {
        fitAddon.fit();
        termRef.current.refresh(0, Math.max(0, termRef.current.rows - 1));
        const cols = Math.max(20, termRef.current.cols);
        const rows = Math.max(5, termRef.current.rows);
        invoke("resize_pane", {
          paneId: leaf.paneId,
          cols,
          rows,
        }).catch(() => undefined);
      } catch {
        // ignore transient fit error
      }
    });
  }, [
    isVisible,
    themeConfig.terminalPreset,
    themeConfig.cursorStyle,
    themeConfig.fontFamily,
    themeConfig.fontSize,
    themeConfig.lineHeight,
    themeConfig.letterSpacing,
    leaf.paneId,
  ]);

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
      style={{ backgroundColor: resolvedXtermTheme.background }}
      className={`relative flex flex-col h-full w-full overflow-hidden transition-all duration-200 ${
        showFocusBorder
          ? isActive
            ? "ring-1 ring-inset ring-primary/65"
            : "ring-1 ring-inset ring-border/80 opacity-92"
          : ""
      }`}
    >
      {/* Pane Micro-Header */}
      <div className="flex items-center justify-between h-7 px-2.5 bg-card/90 backdrop-blur-xs border-b border-border text-[11px] text-muted-foreground shrink-0 transition-colors">
        <div className="flex items-center gap-2 min-w-0">
          <span className="relative flex h-2 w-2 shrink-0">
            {leaf.status === "connecting" && (
              <span className="animate-ping absolute inline-flex h-full w-full rounded-full bg-amber-400 opacity-75" />
            )}
            <span
              className={`relative inline-flex rounded-full h-2 w-2 transition-colors ${
                leaf.status === "connected"
                  ? "bg-emerald-400 shadow-[0_0_6px_rgba(52,211,153,0.75)]"
                  : leaf.status === "connecting"
                    ? "bg-amber-400"
                    : "bg-rose-500"
              }`}
            />
          </span>
          <TerminalIcon className="w-3 h-3 text-muted-foreground shrink-0" />
          <span className="font-medium text-foreground truncate">
            {leaf.title}
          </span>
          {leaf.fingerprint && (
            <span
              title={`Host Key: ${leaf.fingerprint}`}
              className="hidden sm:inline-flex items-center gap-1 px-1.5 py-0.5 rounded-md bg-emerald-500/10 text-emerald-400 border border-emerald-500/20 font-mono text-[10px] truncate max-w-[180px]"
            >
              <ShieldCheck className="w-2.5 h-2.5 shrink-0" />
              {leaf.fingerprint}
            </span>
          )}
        </div>

        <div className="flex items-center gap-0.5">
          {broadcastTargetPaneIds && broadcastTargetPaneIds.length > 1 && (
            <span className="inline-flex items-center gap-1 px-1.5 py-0.5 mr-1 rounded-md bg-amber-500/15 text-amber-400 border border-amber-500/30 font-semibold text-[10px]">
              <Radio className="w-2.5 h-2.5 animate-pulse" />
              {t.termBroadcastLabel}
            </span>
          )}
          {savedLogNotice && (
            <span
              title={savedLogNotice}
              className="hidden md:inline-flex items-center gap-1 px-1.5 py-0.5 mr-1 rounded-md bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 text-[10px] max-w-[200px] truncate"
            >
              {t.termSessionLogSaved}
            </span>
          )}
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setSearchOpen((p) => {
                const next = !p;
                if (next) {
                  setTimeout(() => searchInputRef.current?.focus(), 20);
                } else {
                  termRef.current?.clearSelection();
                  termRef.current?.focus();
                }
                return next;
              });
            }}
            title={withShortcutHint(t.termSearchPlaceholder, "find")}
            className={`p-1 rounded-md transition-colors cursor-pointer ${
              searchOpen
                ? "bg-primary/20 text-primary"
                : "hover:bg-accent hover:text-foreground"
            }`}
          >
            <Search className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleToggleLiveLog();
            }}
            title={
              isRecordingLog ? t.termSessionLogStop : t.termSessionLogStart
            }
            className={`px-1.5 py-0.5 rounded-md text-[10px] font-semibold inline-flex items-center gap-1 transition-colors cursor-pointer ${
              isRecordingLog
                ? "bg-rose-500/20 text-rose-400 border border-rose-500/40"
                : "hover:bg-accent hover:text-foreground"
            }`}
          >
            <span
              className={`w-1.5 h-1.5 rounded-full ${
                isRecordingLog
                  ? "bg-rose-400 animate-ping"
                  : "bg-muted-foreground"
              }`}
            />
            <span>REC</span>
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              handleExportSessionLog();
            }}
            title={t.termSessionLogExport}
            className="p-1 hover:bg-accent hover:text-foreground rounded-md transition-colors cursor-pointer"
          >
            <Download className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              setShowExtraKeys((p) => !p);
            }}
            title="Toggle Touch Extra Keys Bar"
            className={`p-1 rounded-md transition-colors cursor-pointer ${
              showExtraKeys
                ? "bg-primary/20 text-primary"
                : "hover:bg-accent hover:text-foreground"
            }`}
          >
            <Keyboard className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSplitVertical();
            }}
            title={withShortcutHint(t.termSplitVertical, "splitVertical")}
            className="p-1 hover:bg-accent hover:text-foreground rounded-md transition-colors cursor-pointer"
          >
            <Columns className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onSplitHorizontal();
            }}
            title={withShortcutHint(t.termSplitHorizontal, "splitHorizontal")}
            className="p-1 hover:bg-accent hover:text-foreground rounded-md transition-colors cursor-pointer"
          >
            <Rows className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={(e) => {
              e.stopPropagation();
              onClosePane();
            }}
            title={withShortcutHint(t.termClosePane, "closePane")}
            className="p-1 hover:bg-rose-500/20 hover:text-rose-400 rounded-md transition-colors cursor-pointer"
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      </div>

      {/* Inline Search Bar Overlay (Cmd+F) */}
      {searchOpen && (
        <div
          onMouseDown={(e) => e.stopPropagation()}
          className="absolute top-8 right-3 z-20 flex items-center gap-1.5 px-2.5 py-1.5 rounded-xl bg-card/95 border border-border shadow-xl backdrop-blur-md"
        >
          <Search className="w-3.5 h-3.5 text-muted-foreground shrink-0" />
          <input
            ref={searchInputRef}
            type="text"
            value={searchQuery}
            onChange={(e) => {
              const val = e.target.value;
              setSearchQuery(val);
              searchCursorRef.current = { row: -1, col: -1 };
              runBufferSearch(val, "next");
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                e.preventDefault();
                runBufferSearch(searchQuery, e.shiftKey ? "prev" : "next");
              } else if (e.key === "Escape") {
                e.preventDefault();
                setSearchOpen(false);
                termRef.current?.clearSelection();
                termRef.current?.focus();
              }
            }}
            placeholder={t.termSearchPlaceholder}
            className="w-48 bg-transparent text-xs text-foreground placeholder:text-muted-foreground focus:outline-none"
          />
          {searchMatchInfo && (
            <span className="text-[10px] font-mono text-muted-foreground px-1">
              {searchMatchInfo}
            </span>
          )}
          <button
            type="button"
            onClick={() => runBufferSearch(searchQuery, "prev")}
            className="p-1 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground cursor-pointer"
          >
            <ArrowUp className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={() => runBufferSearch(searchQuery, "next")}
            className="p-1 rounded-md hover:bg-accent text-muted-foreground hover:text-foreground cursor-pointer"
          >
            <ArrowDown className="w-3 h-3" />
          </button>
          <button
            type="button"
            onClick={() => {
              setSearchOpen(false);
              termRef.current?.clearSelection();
              termRef.current?.focus();
            }}
            className="p-1 rounded-md hover:bg-rose-500/20 text-muted-foreground hover:text-rose-400 cursor-pointer"
          >
            <X className="w-3 h-3" />
          </button>
        </div>
      )}

      {/* Reconnect / Auth Banner Overlay when Disconnected or Error */}
      {(leaf.status === "disconnected" || leaf.status === "error") && (
        <div className="ui-banner-slide z-10 flex flex-wrap items-center justify-between gap-2 px-3 py-2 bg-rose-950/90 border-b border-rose-500/40 text-xs text-rose-100 backdrop-blur-md">
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
                  className="pl-6 pr-2 py-1 rounded-md bg-background/90 border border-rose-400/30 text-[11px] text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary w-48"
                />
              </div>
            )}
            <button
              type="submit"
              className="inline-flex items-center gap-1 px-2.5 py-1 rounded-md bg-primary hover:opacity-90 text-primary-foreground font-semibold text-[11px] transition-opacity cursor-pointer"
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
        style={{
          backgroundColor: resolvedXtermTheme.background,
          color: resolvedXtermTheme.foreground,
        }}
        className="flex-1 w-full h-full px-1.5 py-1 overflow-hidden select-none transition-colors duration-150"
      />

      {/* Mobile / Touch Extra Keys Bar (iOS, iPadOS, Android & Touch Screens) */}
      {showExtraKeys && (
        <div
          onMouseDown={(e) => e.preventDefault()}
          className="flex items-center gap-1 px-2 py-1.5 bg-card/95 border-t border-border/80 overflow-x-auto no-scrollbar shrink-0 select-none"
        >
          <button
            type="button"
            onClick={() => sendVirtualKey("\x1b")}
            className="px-2 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono font-semibold text-foreground shrink-0 cursor-pointer"
          >
            ESC
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\t")}
            className="px-2 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono font-semibold text-foreground shrink-0 cursor-pointer"
          >
            TAB
          </button>
          <button
            type="button"
            onClick={() => setCtrlArmed((p) => !p)}
            className={`px-2 py-1 rounded-md text-[11px] font-mono font-semibold shrink-0 cursor-pointer border ${
              ctrlArmed
                ? "bg-primary text-primary-foreground border-primary"
                : "bg-secondary hover:bg-accent text-foreground border-transparent"
            }`}
          >
            CTRL
          </button>
          <button
            type="button"
            onClick={() => setAltArmed((p) => !p)}
            className={`px-2 py-1 rounded-md text-[11px] font-mono font-semibold shrink-0 cursor-pointer border ${
              altArmed
                ? "bg-primary text-primary-foreground border-primary"
                : "bg-secondary hover:bg-accent text-foreground border-transparent"
            }`}
          >
            ALT
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\x1b[D")}
            className="px-2.5 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono font-semibold text-foreground shrink-0 cursor-pointer"
          >
            ←
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\x1b[B")}
            className="px-2.5 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono font-semibold text-foreground shrink-0 cursor-pointer"
          >
            ↓
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\x1b[A")}
            className="px-2.5 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono font-semibold text-foreground shrink-0 cursor-pointer"
          >
            ↑
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\x1b[C")}
            className="px-2.5 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono font-semibold text-foreground shrink-0 cursor-pointer"
          >
            →
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\x03")}
            className="px-2 py-1 rounded-md bg-rose-500/15 hover:bg-rose-500/25 text-[11px] font-mono font-semibold text-rose-300 shrink-0 cursor-pointer"
          >
            ^C
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\x04")}
            className="px-2 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono text-foreground shrink-0 cursor-pointer"
          >
            ^D
          </button>
          <button
            type="button"
            onClick={() => sendVirtualKey("\x0c")}
            className="px-2 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono text-foreground shrink-0 cursor-pointer"
          >
            ^L
          </button>
          {["/", "-", "|", "~", "_"].map((sym) => (
            <button
              key={sym}
              type="button"
              onClick={() => sendVirtualKey(sym)}
              className="px-2.5 py-1 rounded-md bg-secondary hover:bg-accent text-[11px] font-mono text-foreground shrink-0 cursor-pointer"
            >
              {sym}
            </button>
          ))}
        </div>
      )}
    </div>
  );
};
