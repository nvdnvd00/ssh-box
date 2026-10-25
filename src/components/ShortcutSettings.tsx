import React, { useEffect, useState } from "react";
import { RotateCcw } from "lucide-react";
import { AppLanguage, AppSettingsConfig } from "../themes";
import {
  comboFromEvent,
  findConflict,
  formatCombo,
  isValidCombo,
  resolveBindings,
  SHORTCUT_DEFS,
  ShortcutDef,
  ShortcutId,
} from "../shortcuts";

interface ShortcutSettingsProps {
  lang: AppLanguage;
  overrides: Record<string, string>;
  onChangeAppSettings: (
    updater: (prev: AppSettingsConfig) => AppSettingsConfig,
  ) => void;
}

const GROUPS: { id: ShortcutDef["group"]; label: [string, string] }[] = [
  { id: "terminal", label: ["Terminal", "Terminal"] },
  { id: "tabs", label: ["Tab & Pane", "Tabs & Panes"] },
  { id: "app", label: ["Ứng dụng", "App"] },
];

const DEFAULTS = resolveBindings({});

export const ShortcutSettings: React.FC<ShortcutSettingsProps> = ({
  lang,
  overrides,
  onChangeAppSettings,
}) => {
  const vi = lang === "vi";
  const [recordingId, setRecordingId] = useState<ShortcutId | null>(null);
  const [error, setError] = useState("");
  const labelOf = (id: ShortcutId) =>
    SHORTCUT_DEFS.find((d) => d.id === id)!.label[vi ? 0 : 1];

  const setBinding = (id: ShortcutId, combo: string | null) => {
    onChangeAppSettings((prev) => {
      const next = { ...prev.shortcuts };
      // Store only real overrides so future default changes still reach this user.
      if (combo === null || combo === DEFAULTS[id]) delete next[id];
      else next[id] = combo;
      return { ...prev, shortcuts: next };
    });
    setRecordingId(null);
    setError("");
  };

  useEffect(() => {
    if (!recordingId) return;
    // Capture phase on window: runs before App's shortcut listener and the terminal.
    const onKeyDown = (e: KeyboardEvent) => {
      e.preventDefault();
      e.stopPropagation();
      const combo = comboFromEvent(e);
      if (!combo) return; // a modifier on its own: keep waiting
      if (combo === "Escape") {
        setRecordingId(null);
        setError("");
        return;
      }
      if (combo === "Backspace") return setBinding(recordingId, "");
      if (!isValidCombo(combo)) {
        setError(
          vi
            ? "Cần kèm Ctrl, Alt hoặc ⌘ để không chặn việc gõ chữ trong terminal."
            : "Add Ctrl, Alt or ⌘ so the key doesn't block typing in the terminal.",
        );
        return;
      }
      const owner = findConflict(resolveBindings(overrides), combo, recordingId);
      if (owner) {
        setError(
          vi
            ? `${formatCombo(combo)} đang dùng cho "${labelOf(owner)}". Đổi phím đó trước.`
            : `${formatCombo(combo)} is used by "${labelOf(owner)}". Change that one first.`,
        );
        return;
      }
      setBinding(recordingId, combo);
    };
    window.addEventListener("keydown", onKeyDown, true);
    return () => window.removeEventListener("keydown", onKeyDown, true);
  }, [recordingId, overrides]);

  const bindings = resolveBindings(overrides);
  const kbdClass =
    "min-w-[64px] px-2.5 py-1 rounded-lg border font-mono text-xs text-center";

  return (
    <section className="space-y-4">
      <div className="flex items-start justify-between gap-4 px-1">
        <p className="text-[12px] text-muted-foreground">
          {vi
            ? "Bấm vào phím để đổi, rồi nhấn tổ hợp mới. Esc để huỷ, Backspace để bỏ gán."
            : "Click a key to change it, then press the new combo. Esc cancels, Backspace unbinds."}
        </p>
        {Object.keys(overrides).length > 0 && (
          <button
            type="button"
            onClick={() => {
              onChangeAppSettings((prev) => ({ ...prev, shortcuts: {} }));
              setRecordingId(null);
              setError("");
            }}
            className="shrink-0 px-3 py-1.5 rounded-lg border border-border bg-secondary/70 text-xs text-foreground hover:bg-accent transition-colors cursor-pointer"
          >
            {vi ? "Khôi phục tất cả" : "Reset all"}
          </button>
        )}
      </div>
      {error && <p className="px-1 text-[12px] text-red-400">{error}</p>}

      {GROUPS.map((group) => (
        <div
          key={group.id}
          className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden"
        >
          <div className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
            {group.label[vi ? 0 : 1]}
          </div>
          {SHORTCUT_DEFS.filter((d) => d.group === group.id).map((d) => {
            const recording = recordingId === d.id;
            const combo = bindings[d.id];
            return (
              <div
                key={d.id}
                className="flex items-center justify-between px-4 py-2.5 text-[13px]"
              >
                <span className="text-foreground">{d.label[vi ? 0 : 1]}</span>
                <div className="flex items-center gap-1.5">
                  {d.id in overrides && (
                    <button
                      type="button"
                      title={vi ? "Về mặc định" : "Reset to default"}
                      onClick={() => setBinding(d.id, null)}
                      className="p-1 rounded-md text-muted-foreground hover:text-foreground hover:bg-accent transition-colors cursor-pointer"
                    >
                      <RotateCcw className="w-3.5 h-3.5" />
                    </button>
                  )}
                  {d.fixed ? (
                    <kbd
                      title={vi ? "Phím dán của hệ thống, không đổi được" : "System paste key, not editable"}
                      className={`${kbdClass} bg-secondary/40 border-border/60 text-muted-foreground/70`}
                    >
                      {formatCombo(combo)}
                    </kbd>
                  ) : (
                    <button
                      type="button"
                      onClick={() => {
                        setError("");
                        setRecordingId(recording ? null : d.id);
                      }}
                      className={`${kbdClass} cursor-pointer transition-colors ${
                        recording
                          ? "bg-primary/15 border-primary text-primary"
                          : "bg-secondary border-border text-muted-foreground hover:border-primary/60"
                      }`}
                    >
                      {recording
                        ? vi
                          ? "Bấm phím…"
                          : "Press keys…"
                        : formatCombo(combo) || (vi ? "Chưa gán" : "Unbound")}
                    </button>
                  )}
                </div>
              </div>
            );
          })}
        </div>
      ))}
    </section>
  );
};
