// Keyboard shortcuts: one table of actions with per-platform defaults.
// A binding is a combo string "Ctrl+Alt+Shift+Meta+<KeyboardEvent.code>" (modifiers in that
// order). Users override bindings in Settings; only the overrides are persisted, so changing a
// default here still reaches everyone who never customized that action.

export const isApplePlatform =
  typeof navigator !== "undefined" && /Mac|iPhone|iPad|iPod/.test(navigator.platform);

type Digit = "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9";

export type ShortcutId =
  | "zoomIn"
  | "zoomOut"
  | "zoomReset"
  | "copy"
  | "paste"
  | "selectAll"
  | "find"
  | "clear"
  | "lineStart"
  | "lineEnd"
  | "deleteLine"
  | "newLocalTab"
  | "closePane"
  | "splitVertical"
  | "splitHorizontal"
  | "nextTab"
  | "prevTab"
  | `goToTab${Digit}`
  | "nextPane"
  | "prevPane"
  | "commandPalette"
  | "toggleMonitor"
  | "openSettings";

export interface ShortcutDef {
  id: ShortcutId;
  group: "terminal" | "tabs" | "app";
  label: [vi: string, en: string];
  mac: string;
  win: string;
  /** Rides the OS-native paste path; reading the clipboard from JS prompts in WKWebView. */
  fixed?: boolean;
}

const DIGITS: Digit[] = ["1", "2", "3", "4", "5", "6", "7", "8", "9"];

export const SHORTCUT_DEFS: ShortcutDef[] = [
  { id: "zoomIn", group: "terminal", label: ["Phóng to chữ", "Zoom in"], mac: "Meta+Equal", win: "Ctrl+Equal" },
  { id: "zoomOut", group: "terminal", label: ["Thu nhỏ chữ", "Zoom out"], mac: "Meta+Minus", win: "Ctrl+Minus" },
  { id: "zoomReset", group: "terminal", label: ["Cỡ chữ mặc định", "Reset zoom"], mac: "Meta+Digit0", win: "Ctrl+Digit0" },
  { id: "copy", group: "terminal", label: ["Copy", "Copy"], mac: "Meta+KeyC", win: "Ctrl+Shift+KeyC" },
  { id: "paste", group: "terminal", label: ["Dán", "Paste"], mac: "Meta+KeyV", win: "Ctrl+KeyV", fixed: true },
  { id: "selectAll", group: "terminal", label: ["Chọn tất cả", "Select all"], mac: "Meta+KeyA", win: "Ctrl+Shift+KeyA" },
  { id: "find", group: "terminal", label: ["Tìm trong terminal", "Find"], mac: "Meta+KeyF", win: "Ctrl+Shift+KeyF" },
  { id: "clear", group: "terminal", label: ["Xoá màn hình", "Clear screen"], mac: "Shift+Meta+KeyK", win: "Ctrl+Shift+KeyK" },
  { id: "lineStart", group: "terminal", label: ["Về đầu dòng", "Go to line start"], mac: "Meta+ArrowLeft", win: "" },
  { id: "lineEnd", group: "terminal", label: ["Về cuối dòng", "Go to line end"], mac: "Meta+ArrowRight", win: "" },
  { id: "deleteLine", group: "terminal", label: ["Xoá cả dòng", "Delete line"], mac: "Meta+Backspace", win: "" },
  { id: "newLocalTab", group: "tabs", label: ["Mở tab local mới", "New local tab"], mac: "Meta+KeyT", win: "Ctrl+Shift+KeyT" },
  { id: "closePane", group: "tabs", label: ["Đóng pane", "Close pane"], mac: "Meta+KeyW", win: "Ctrl+Shift+KeyW" },
  { id: "splitVertical", group: "tabs", label: ["Chia dọc", "Split vertically"], mac: "Meta+KeyD", win: "Alt+Shift+Equal" },
  { id: "splitHorizontal", group: "tabs", label: ["Chia ngang", "Split horizontally"], mac: "Shift+Meta+KeyD", win: "Alt+Shift+Minus" },
  { id: "nextTab", group: "tabs", label: ["Tab kế tiếp", "Next tab"], mac: "Shift+Meta+BracketRight", win: "Ctrl+Tab" },
  { id: "prevTab", group: "tabs", label: ["Tab trước", "Previous tab"], mac: "Shift+Meta+BracketLeft", win: "Ctrl+Shift+Tab" },
  ...DIGITS.map(
    (n): ShortcutDef => ({
      id: `goToTab${n}`,
      group: "tabs",
      label: [`Tới tab ${n}`, `Go to tab ${n}`],
      mac: `Meta+Digit${n}`,
      win: `Ctrl+Alt+Digit${n}`,
    }),
  ),
  { id: "nextPane", group: "tabs", label: ["Pane kế tiếp", "Next pane"], mac: "Meta+BracketRight", win: "Alt+ArrowRight" },
  { id: "prevPane", group: "tabs", label: ["Pane trước", "Previous pane"], mac: "Meta+BracketLeft", win: "Alt+ArrowLeft" },
  { id: "commandPalette", group: "app", label: ["Bảng lệnh nhanh", "Command palette"], mac: "Meta+KeyK", win: "Ctrl+Shift+KeyP" },
  { id: "toggleMonitor", group: "app", label: ["Bật/tắt bảng giám sát", "Toggle monitor"], mac: "Meta+KeyI", win: "Ctrl+Shift+KeyI" },
  { id: "openSettings", group: "app", label: ["Mở Settings", "Open Settings"], mac: "Meta+Comma", win: "Ctrl+Comma" },
];

const DEF_BY_ID = new Map(SHORTCUT_DEFS.map((d) => [d.id, d]));

export function comboFromEvent(
  e: Pick<KeyboardEvent, "ctrlKey" | "altKey" | "shiftKey" | "metaKey" | "code">,
): string | null {
  if (!e.code || /^(Control|Alt|Shift|Meta|OS)(Left|Right)?$/.test(e.code)) return null;
  const mods = [
    e.ctrlKey && "Ctrl",
    e.altKey && "Alt",
    e.shiftKey && "Shift",
    e.metaKey && "Meta",
  ].filter(Boolean);
  return [...mods, e.code].join("+");
}

/** A bare key (or Shift+key) would swallow normal typing in the terminal. */
export function isValidCombo(combo: string): boolean {
  return /(^|\+)(Ctrl|Alt|Meta)\+/.test(combo) || /^(Shift\+)?F\d{1,2}$/.test(combo);
}

export function resolveBindings(
  overrides: Record<string, string>,
  mac = isApplePlatform,
): Record<ShortcutId, string> {
  const out = {} as Record<ShortcutId, string>;
  for (const d of SHORTCUT_DEFS) {
    const own = d.fixed ? undefined : overrides[d.id];
    out[d.id] = own ?? (mac ? d.mac : d.win);
  }
  return out;
}

/** Returns the action already holding `combo`, ignoring `exceptId`. */
export function findConflict(
  bindings: Record<ShortcutId, string>,
  combo: string,
  exceptId: ShortcutId,
): ShortcutId | null {
  for (const d of SHORTCUT_DEFS) {
    if (d.id !== exceptId && bindings[d.id] === combo) return d.id;
  }
  return null;
}

/** Drops unknown/fixed actions and malformed combos read back from disk. "" = unbound. */
export function parseShortcutOverrides(raw: unknown): Record<string, string> {
  const out: Record<string, string> = {};
  if (!raw || typeof raw !== "object") return out;
  for (const [id, combo] of Object.entries(raw)) {
    const def = DEF_BY_ID.get(id as ShortcutId);
    if (!def || def.fixed || typeof combo !== "string") continue;
    if (combo === "" || isValidCombo(combo)) out[id] = combo;
  }
  return out;
}

// The live keymap. TerminalPane attaches its key handler once per pane, so it reads this
// module state instead of a prop that would go stale inside that closure.
let current = resolveBindings({});
let byCombo = new Map<string, ShortcutId>();

export function setShortcutOverrides(overrides: Record<string, string>): void {
  current = resolveBindings(overrides);
  byCombo = new Map();
  for (const d of SHORTCUT_DEFS) {
    const combo = current[d.id];
    if (combo && !byCombo.has(combo)) byCombo.set(combo, d.id);
  }
}
setShortcutOverrides({});

export function matchShortcut(e: KeyboardEvent): ShortcutId | null {
  const combo = comboFromEvent(e);
  return combo ? (byCombo.get(combo) ?? null) : null;
}

const KEY_LABELS: Record<string, string> = {
  Equal: "=",
  Minus: "-",
  BracketLeft: "[",
  BracketRight: "]",
  Comma: ",",
  Period: ".",
  Slash: "/",
  Backslash: "\\",
  Semicolon: ";",
  Quote: "'",
  Backquote: "`",
  ArrowLeft: "←",
  ArrowRight: "→",
  ArrowUp: "↑",
  ArrowDown: "↓",
  Backspace: "⌫",
  Enter: "↵",
  Escape: "Esc",
  Delete: "Del",
};
const MAC_MODS: Record<string, string> = { Ctrl: "⌃", Alt: "⌥", Shift: "⇧", Meta: "⌘" };

export function formatCombo(combo: string, mac = isApplePlatform): string {
  if (!combo) return "";
  const parts = combo.split("+");
  const code = parts.pop()!;
  const key = KEY_LABELS[code] ?? code.replace(/^(Key|Digit|Numpad)/, "");
  return mac
    ? parts.map((m) => MAC_MODS[m]).join("") + key
    : [...parts, key].join("+");
}

/** Display label of the key currently bound to `id`; "" when unbound. */
export function shortcutHint(id: ShortcutId): string {
  return formatCombo(current[id]);
}

/** "Find (⌘F)" for tooltips; just `text` when the action is unbound. */
export function withShortcutHint(text: string, id: ShortcutId): string {
  const hint = shortcutHint(id);
  return hint ? `${text} (${hint})` : text;
}
