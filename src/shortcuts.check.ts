// Self-check for the shortcut keymap. Run: node src/shortcuts.check.ts
import {
  SHORTCUT_DEFS,
  comboFromEvent,
  findConflict,
  formatCombo,
  isApplePlatform,
  isValidCombo,
  matchShortcut,
  parseShortcutOverrides,
  resolveBindings,
  setShortcutOverrides,
  shortcutHint,
  withShortcutHint,
} from "./shortcuts.ts";

function check(cond: boolean, why: string): void {
  if (!cond) throw new Error(`FAIL: ${why}`);
}

const key = (combo: string) => {
  const parts = combo.split("+");
  return {
    code: parts.pop()!,
    ctrlKey: parts.includes("Ctrl"),
    altKey: parts.includes("Alt"),
    shiftKey: parts.includes("Shift"),
    metaKey: parts.includes("Meta"),
  } as KeyboardEvent;
};

for (const mac of [true, false]) {
  const bindings = resolveBindings({}, mac);
  const seen = new Set<string>();
  for (const d of SHORTCUT_DEFS) {
    const combo = bindings[d.id];
    if (!combo) continue;
    check(comboFromEvent(key(combo)) === combo, `${d.id} default "${combo}" is not canonical, so it can never match`);
    check(isValidCombo(combo), `${d.id} default "${combo}" would swallow plain typing`);
    check(!seen.has(combo), `two actions share "${combo}" on ${mac ? "mac" : "win"}; one would never fire`);
    seen.add(combo);
  }
}

// Bare Ctrl+letter is the shell's (^C interrupt, ^D EOF, ^W delete-word...). Only paste keeps
// Ctrl+V, the Windows behaviour users already rely on.
for (const d of SHORTCUT_DEFS) {
  if (d.id !== "paste") check(!/^Ctrl\+Key[A-Z]$/.test(d.win), `${d.id} steals ${d.win} from the shell on Windows`);
}

check(comboFromEvent({ code: "ShiftLeft", shiftKey: true } as KeyboardEvent) === null, "pressing Shift alone must not record a binding");
check(!isValidCombo("KeyA") && !isValidCombo("Shift+KeyA"), "letters with no Ctrl/Alt/Cmd are typing, not shortcuts");
check(isValidCombo("F5"), "function keys don't type text, so they may be bound bare");

const parsed = parseShortcutOverrides({ copy: "Alt+KeyC", paste: "Alt+KeyV", bogus: "Ctrl+KeyB", find: "KeyF", clear: 3, lineStart: "" });
check(JSON.stringify(parsed) === JSON.stringify({ copy: "Alt+KeyC", lineStart: "" }), "settings.json is user-editable: drop junk, keep valid overrides and unbinds");

const custom = resolveBindings({ copy: "Alt+KeyC", lineStart: "", paste: "Alt+KeyV" }, true);
check(custom.copy === "Alt+KeyC", "an override must replace the default");
check(custom.lineStart === "", "an empty override means the user unbound it");
check(custom.paste === "Meta+KeyV", "paste stays on the native key even if settings say otherwise");
check(findConflict(custom, "Meta+KeyF", "copy") === "find", "recording a key already in use must report who holds it");
check(findConflict(custom, "Alt+KeyC", "copy") === null, "re-recording an action's own key is not a conflict");

setShortcutOverrides({ copy: "Alt+KeyC", find: "" });
check(matchShortcut(key("Alt+KeyC")) === "copy", "the live keymap must follow overrides");
check(shortcutHint("copy") === formatCombo("Alt+KeyC"), "UI hints must show the key the user picked, not the default");
check(withShortcutHint("Find", "find") === "Find", "an unbound action must not show an empty hint like \"Find ()\"");
const oldCopy = isApplePlatform ? "Meta+KeyC" : "Ctrl+Shift+KeyC";
check(matchShortcut(key(oldCopy)) === null, "the replaced default must stop firing");
setShortcutOverrides({});

check(formatCombo("Shift+Meta+KeyD", true) === "⇧⌘D", "mac shows the ⌃⌥⇧⌘ symbols in Apple's order");
check(formatCombo("Ctrl+Shift+BracketRight", false) === "Ctrl+Shift+]", "Windows spells modifiers out");

console.log("shortcuts: all checks passed");
