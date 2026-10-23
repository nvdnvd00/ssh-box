import React, { useEffect, useRef, useState } from "react";
import {
  Check,
  Minus,
  Monitor,
  Palette,
  Plus,
  RotateCcw,
  Terminal as TerminalIcon,
  Type,
} from "lucide-react";
import {
  ACCENT_COLORS,
  APP_THEMES,
  DEFAULT_APP_THEME,
  DEFAULT_TERMINAL_THEME,
  TERMINAL_PRESETS,
  TerminalCursorStyle,
  ThemeConfig,
} from "../themes";

interface ThemePopoverProps {
  config: ThemeConfig;
  isOpen: boolean;
  onToggle: () => void;
  onClose: () => void;
  onChange: (updater: (prev: ThemeConfig) => ThemeConfig) => void;
}

export const ThemePopover: React.FC<ThemePopoverProps> = ({
  config,
  isOpen,
  onToggle,
  onClose,
  onChange,
}) => {
  const wrapperRef = useRef<HTMLDivElement | null>(null);
  const [activeTab, setActiveTab] = useState<"app" | "terminal">("app");

  useEffect(() => {
    if (!isOpen) return;

    const handlePointerDown = (e: MouseEvent) => {
      if (
        wrapperRef.current &&
        !wrapperRef.current.contains(e.target as Node)
      ) {
        onClose();
      }
    };

    const handleKeyDown = (e: KeyboardEvent) => {
      if (e.key === "Escape") {
        onClose();
      }
    };

    window.addEventListener("mousedown", handlePointerDown);
    window.addEventListener("keydown", handleKeyDown);
    return () => {
      window.removeEventListener("mousedown", handlePointerDown);
      window.removeEventListener("keydown", handleKeyDown);
    };
  }, [isOpen, onClose]);

  const activeAppTheme =
    APP_THEMES.find((t) => t.id === config.appTheme) ?? APP_THEMES[0];
  const activePreset =
    TERMINAL_PRESETS.find((p) => p.id === config.terminalPreset) ??
    TERMINAL_PRESETS[0];

  return (
    <div ref={wrapperRef} className="relative">
      <button
        type="button"
        onClick={onToggle}
        title="Appearance & Terminal Settings (Cmd+,)"
        className={`inline-flex items-center gap-1.5 px-2.5 py-1.5 rounded-lg text-xs font-medium transition-all duration-150 cursor-pointer border ${
          isOpen
            ? "bg-primary/15 text-primary border-primary/40 shadow-xs"
            : "text-muted-foreground hover:bg-accent hover:text-foreground border-transparent"
        }`}
      >
        <Palette className="w-3.5 h-3.5 text-primary" />
        <span className="hidden sm:inline text-xs">Theme</span>
      </button>

      {isOpen && (
        <div className="ui-popover-panel absolute right-0 top-full mt-2 w-[370px] rounded-2xl bg-popover/98 backdrop-blur-xl border border-border shadow-2xl z-50 overflow-hidden select-none">
          {/* Segmented Header: App Theme vs Terminal Theme */}
          <div className="p-2.5 border-b border-border bg-card/60 flex items-center justify-between gap-2">
            <div className="grid grid-cols-2 flex-1 rounded-xl bg-secondary p-1 border border-border/80">
              <button
                type="button"
                onClick={() => setActiveTab("app")}
                className={`inline-flex items-center justify-center gap-1.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                  activeTab === "app"
                    ? "bg-card text-foreground shadow-xs"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                <Monitor className="w-3.5 h-3.5 text-primary" />
                App Theme
              </button>
              <button
                type="button"
                onClick={() => setActiveTab("terminal")}
                className={`inline-flex items-center justify-center gap-1.5 py-1.5 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                  activeTab === "terminal"
                    ? "bg-card text-foreground shadow-xs"
                    : "text-muted-foreground hover:text-foreground"
                }`}
              >
                <TerminalIcon className="w-3.5 h-3.5 text-primary" />
                Terminal Theme
              </button>
            </div>

            <button
              type="button"
              onClick={() =>
                onChange((prev) =>
                  activeTab === "app"
                    ? { ...prev, ...DEFAULT_APP_THEME }
                    : { ...prev, ...DEFAULT_TERMINAL_THEME },
                )
              }
              title="Reset current tab to default"
              className="p-2 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent transition-colors cursor-pointer"
            >
              <RotateCcw className="w-3.5 h-3.5" />
            </button>
          </div>

          <div className="p-4 space-y-4 max-h-[78vh] overflow-y-auto text-xs">
            {activeTab === "app" ? (
              <>
                {/* 1. App UI Surface */}
                <div className="space-y-2.5">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-foreground">
                      Interface Surface
                    </span>
                    <span className="text-[11px] text-muted-foreground">
                      {activeAppTheme.subtitle}
                    </span>
                  </div>

                  <div className="grid grid-cols-5 gap-2">
                    {APP_THEMES.map((theme) => {
                      const selected = config.appTheme === theme.id;
                      return (
                        <button
                          key={theme.id}
                          type="button"
                          onClick={() =>
                            onChange((prev) => ({
                              ...prev,
                              appTheme: theme.id,
                            }))
                          }
                          className={`group flex flex-col items-center gap-1.5 p-1.5 rounded-xl border transition-all cursor-pointer ${
                            selected
                              ? "border-primary bg-primary/10 shadow-xs"
                              : "border-border/70 bg-card/50 hover:border-muted-foreground/40 hover:bg-accent/50"
                          }`}
                        >
                          <div
                            className="w-full h-8 rounded-lg border flex overflow-hidden relative"
                            style={{
                              backgroundColor: theme.bgHex,
                              borderColor: theme.borderHex,
                            }}
                          >
                            <div
                              className="w-2.5 h-full border-r"
                              style={{
                                backgroundColor: theme.cardHex,
                                borderColor: theme.borderHex,
                              }}
                            />
                            <div className="flex-1 p-1 flex flex-col justify-between">
                              <span
                                className="w-3 h-1 rounded-full opacity-80"
                                style={{ backgroundColor: theme.fgHex }}
                              />
                              <span
                                className="w-2 h-2 rounded-full self-end"
                                style={{ backgroundColor: "var(--primary)" }}
                              />
                            </div>
                          </div>
                          <span
                            className={`text-[11px] font-medium truncate w-full text-center ${
                              selected
                                ? "text-foreground font-semibold"
                                : "text-muted-foreground group-hover:text-foreground"
                            }`}
                          >
                            {theme.name}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* 2. Accent Color */}
                <div className="space-y-2.5 pt-3 border-t border-border">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-foreground">
                      Brand & Accent Color
                    </span>
                    <span className="text-[11px] text-muted-foreground">
                      {
                        ACCENT_COLORS.find((a) => a.id === config.accentColor)
                          ?.name
                      }
                    </span>
                  </div>

                  <div className="grid grid-cols-6 gap-2">
                    {ACCENT_COLORS.map((accent) => {
                      const selected = config.accentColor === accent.id;
                      return (
                        <button
                          key={accent.id}
                          type="button"
                          onClick={() =>
                            onChange((prev) => ({
                              ...prev,
                              accentColor: accent.id,
                            }))
                          }
                          title={accent.name}
                          className={`h-8 rounded-xl border flex items-center justify-center transition-all cursor-pointer ${
                            selected
                              ? "border-foreground scale-105 shadow-xs bg-accent"
                              : "border-border/70 bg-card/50 hover:bg-accent/60"
                          }`}
                        >
                          <span
                            className="w-4 h-4 rounded-full flex items-center justify-center shadow-xs"
                            style={{ backgroundColor: accent.swatchHex }}
                          >
                            {selected && (
                              <Check className="w-2.5 h-2.5 text-slate-950 stroke-[3]" />
                            )}
                          </span>
                        </button>
                      );
                    })}
                  </div>
                </div>
              </>
            ) : (
              <>
                {/* Terminal Color Presets */}
                <div className="space-y-2.5">
                  <div className="flex items-center justify-between">
                    <span className="text-xs font-semibold text-foreground">
                      Color Scheme
                    </span>
                    <span className="text-[11px] text-muted-foreground">
                      {activePreset.name}
                    </span>
                  </div>

                  <div className="grid grid-cols-2 gap-2">
                    {TERMINAL_PRESETS.map((preset) => {
                      const selected = config.terminalPreset === preset.id;
                      return (
                        <button
                          key={preset.id}
                          type="button"
                          onClick={() =>
                            onChange((prev) => ({
                              ...prev,
                              terminalPreset: preset.id,
                            }))
                          }
                          className={`group flex flex-col gap-1.5 p-2.5 rounded-xl border text-left transition-all cursor-pointer ${
                            selected
                              ? "border-primary bg-primary/10 shadow-xs"
                              : "border-border/80 bg-card/50 hover:border-muted-foreground/40 hover:bg-accent/50"
                          }`}
                        >
                          <div className="flex items-center justify-between gap-1">
                            <span className="text-xs font-medium text-foreground truncate">
                              {preset.name}
                            </span>
                            {preset.badge && (
                              <span className="px-1.5 py-0.5 rounded-full bg-primary/20 text-primary text-[9px] font-semibold">
                                {preset.badge}
                              </span>
                            )}
                          </div>

                          <div
                            className="w-full h-7 rounded-lg border px-2 flex items-center justify-between font-mono text-[10px]"
                            style={{
                              backgroundColor: preset.theme.background,
                              borderColor: "rgba(128,128,128,0.22)",
                              color: preset.theme.foreground,
                            }}
                          >
                            <span>
                              <span style={{ color: preset.swatches[1] }}>
                                ~
                              </span>{" "}
                              <span style={{ color: preset.swatches[0] }}>
                                $
                              </span>
                              <span
                                className="inline-block w-1.5 h-2.5 ml-0.5 align-middle"
                                style={{
                                  backgroundColor: preset.theme.cursor,
                                }}
                              />
                            </span>
                            <span className="flex items-center gap-1">
                              {preset.swatches.map((hex, i) => (
                                <span
                                  key={i}
                                  className="w-2 h-2 rounded-full"
                                  style={{ backgroundColor: hex }}
                                />
                              ))}
                            </span>
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* Terminal Typography & Cursor */}
                <div className="pt-3 border-t border-border grid grid-cols-2 gap-3">
                  <div className="space-y-1.5">
                    <span className="inline-flex items-center gap-1 text-xs font-semibold text-foreground">
                      <Type className="w-3.5 h-3.5 text-primary" />
                      Font Size
                    </span>
                    <div className="flex items-center justify-between rounded-xl border border-border bg-secondary/70 p-1">
                      <button
                        type="button"
                        onClick={() =>
                          onChange((prev) => ({
                            ...prev,
                            fontSize: Math.max(11, prev.fontSize - 1),
                          }))
                        }
                        className="p-1 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                      >
                        <Minus className="w-3.5 h-3.5" />
                      </button>
                      <span className="text-xs font-semibold text-foreground">
                        {config.fontSize}px
                      </span>
                      <button
                        type="button"
                        onClick={() =>
                          onChange((prev) => ({
                            ...prev,
                            fontSize: Math.min(18, prev.fontSize + 1),
                          }))
                        }
                        className="p-1 rounded-lg hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                      >
                        <Plus className="w-3.5 h-3.5" />
                      </button>
                    </div>
                  </div>

                  <div className="space-y-1.5">
                    <span className="block text-xs font-semibold text-foreground">
                      Cursor Style
                    </span>
                    <div className="grid grid-cols-3 rounded-xl border border-border bg-secondary/70 p-1 gap-1">
                      {(
                        ["block", "bar", "underline"] as TerminalCursorStyle[]
                      ).map((style) => {
                        const active = config.cursorStyle === style;
                        return (
                          <button
                            key={style}
                            type="button"
                            onClick={() =>
                              onChange((prev) => ({
                                ...prev,
                                cursorStyle: style,
                              }))
                            }
                            className={`py-1 rounded-lg text-[11px] font-medium capitalize transition-all cursor-pointer ${
                              active
                                ? "bg-card text-foreground shadow-xs border border-border/60"
                                : "text-muted-foreground hover:text-foreground"
                            }`}
                          >
                            {style}
                          </button>
                        );
                      })}
                    </div>
                  </div>
                </div>
              </>
            )}
          </div>
        </div>
      )}
    </div>
  );
};
