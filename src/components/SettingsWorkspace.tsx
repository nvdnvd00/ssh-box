import React, { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  Check,
  Globe,
  Info,
  Keyboard,
  KeyRound,
  LayoutGrid,
  Minus,
  Moon,
  Plus,
  RotateCcw,
  ShieldCheck,
  Sliders,
  Sun,
  Tag,
  Terminal as TerminalIcon,
  Trash2,
  Wifi,
  Wrench,
  Zap,
} from "lucide-react";
import {
  ACCENT_COLORS,
  AccentColorId,
  APP_THEMES,
  AppLanguage,
  AppSettingsConfig,
  AppThemeId,
  DEFAULT_APP_SETTINGS,
  DEFAULT_THEME_CONFIG,
  resolveTerminalFontCss,
  TERMINAL_FONT_FAMILIES,
  TERMINAL_PRESETS,
  TerminalCursorStyle,
  TerminalFontFamilyId,
  TerminalPresetId,
  ThemeConfig,
} from "../themes";
import { getI18n } from "../i18n";
import { MachineTag, ResourcesStoreData } from "../types";
import { SelectField } from "./SelectField";

export type SettingsTabId =
  | "general"
  | "security"
  | "connections"
  | "dashboard"
  | "terminal"
  | "tags"
  | "diagnostics"
  | "shortcuts"
  | "about";

interface SettingsWorkspaceProps {
  themeConfig: ThemeConfig;
  appSettings: AppSettingsConfig;
  resources: ResourcesStoreData;
  activeTab: SettingsTabId;
  onChangeTheme: (updater: (prev: ThemeConfig) => ThemeConfig) => void;
  onChangeAppSettings: (
    updater: (prev: AppSettingsConfig) => AppSettingsConfig,
  ) => void;
  onUpdateResources: (next: ResourcesStoreData) => void;
}

const TAG_COLORS = [
  "#38bdf8",
  "#34d399",
  "#a78bfa",
  "#fbbf24",
  "#f87171",
  "#fb923c",
  "#ec4899",
  "#2dd4bf",
];

export function getSettingsNavGroups(lang: AppLanguage): {
  title: string;
  items: { id: SettingsTabId; label: string; icon: React.ReactNode }[];
}[] {
  const i18n = getI18n(lang);
  return [
    {
      title: lang === "vi" ? "Ứng dụng" : "App",
      items: [
        {
          id: "general",
          label: lang === "vi" ? "Tổng quan" : "General",
          icon: <Sliders className="w-4 h-4 text-sky-400" />,
        },
        {
          id: "security",
          label: lang === "vi" ? "Bảo mật & Sao lưu" : "Security & Privacy",
          icon: <ShieldCheck className="w-4 h-4 text-emerald-400" />,
        },
      ],
    },
    {
      title: lang === "vi" ? "Tính năng" : "Features",
      items: [
        {
          id: "connections",
          label: lang === "vi" ? "Kết nối" : "Connections",
          icon: <Wifi className="w-4 h-4 text-cyan-400" />,
        },
        {
          id: "dashboard",
          label: i18n.navDashboard,
          icon: <LayoutGrid className="w-4 h-4 text-indigo-400" />,
        },
        {
          id: "terminal",
          label: i18n.navTerminal,
          icon: <TerminalIcon className="w-4 h-4 text-amber-400" />,
        },
        {
          id: "tags",
          label: lang === "vi" ? "Nhãn máy chủ (Tags)" : "Machine Tags",
          icon: <Tag className="w-4 h-4 text-pink-400" />,
        },
      ],
    },
    {
      title: lang === "vi" ? "Trợ giúp" : "Help",
      items: [
        {
          id: "diagnostics",
          label: lang === "vi" ? "Chẩn đoán" : "Diagnostics",
          icon: <Wrench className="w-4 h-4 text-orange-400" />,
        },
        {
          id: "shortcuts",
          label: lang === "vi" ? "Phím tắt" : "Keyboard Shortcuts",
          icon: <Keyboard className="w-4 h-4 text-teal-400" />,
        },
        {
          id: "about",
          label: lang === "vi" ? "Giới thiệu" : "About",
          icon: <Info className="w-4 h-4 text-violet-400" />,
        },
      ],
    },
  ];
}

export const SettingsWorkspace: React.FC<SettingsWorkspaceProps> = ({
  themeConfig,
  appSettings,
  resources,
  activeTab,
  onChangeTheme,
  onChangeAppSettings,
  onUpdateResources,
}) => {
  const lang: AppLanguage = appSettings.language ?? "vi";
  const i18n = getI18n(lang);
  const [confirmReset, setConfirmReset] = useState(false);
  const [newTagName, setNewTagName] = useState("");
  const [newTagColor, setNewTagColor] = useState(TAG_COLORS[0]);
  const [shellOptions, setShellOptions] = useState<
    { id: string; label: string }[]
  >([]);
  const [appVersion, setAppVersion] = useState("");

  useEffect(() => {
    getVersion()
      .then(setAppVersion)
      .catch(() => undefined);
    invoke<{ id: string; label: string }[]>("list_local_shells")
      .then(setShellOptions)
      .catch(() => undefined);
  }, []);

  const activePreset =
    TERMINAL_PRESETS.find((p) => p.id === themeConfig.terminalPreset) ??
    TERMINAL_PRESETS[0];

  const handleAddTag = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!newTagName.trim()) return;
    try {
      const updated = await invoke<ResourcesStoreData>("save_machine_tag", {
        tag: {
          id: "",
          name: newTagName.trim(),
          color: newTagColor,
          createdDate: 0,
        } satisfies MachineTag,
      });
      onUpdateResources(updated);
      setNewTagName("");
    } catch {
      // ignore
    }
  };

  const handleDeleteTag = async (tagId: string) => {
    try {
      const updated = await invoke<ResourcesStoreData>("delete_machine_tag", {
        tagId,
      });
      onUpdateResources(updated);
    } catch {
      // ignore
    }
  };

  const currentTabTitle =
    getSettingsNavGroups(lang)
      .flatMap((g) => g.items)
      .find((i) => i.id === activeTab)?.label ?? "Settings";

  return (
    <div className="flex h-full w-full bg-background text-foreground overflow-hidden select-none">
      {/* Settings Content Surface */}
      <div className="flex-1 min-w-0 flex flex-col h-full overflow-hidden">
        <div className="h-14 shrink-0 px-8 border-b border-border/70 flex items-center justify-between bg-header/40 backdrop-blur-md">
          <h1 className="text-base font-semibold tracking-tight text-foreground">
            {currentTabTitle}
          </h1>
        </div>

        <div className="flex-1 overflow-y-auto px-8 py-6">
          <div className="max-w-2xl space-y-7 pb-10 ui-fade-slide">
            {/* GENERAL TAB: Language, Fast Sync, App Theme & Accent Color */}
            {activeTab === "general" && (
              <>
                <section className="space-y-2.5">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    {i18n.settingsLanguageLabel}
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <div className="flex items-start gap-3">
                        <div className="p-2 rounded-xl bg-sky-500/10 text-sky-400 border border-sky-500/20">
                          <Globe className="w-4 h-4" />
                        </div>
                        <div>
                          <div className="text-[13px] font-medium text-foreground">
                            {i18n.settingsLanguageLabel}
                          </div>
                          <div className="text-[11px] text-muted-foreground">
                            {i18n.settingsLanguageDesc}
                          </div>
                        </div>
                      </div>
                      <div className="flex items-center gap-1.5 bg-secondary/70 p-1 rounded-xl border border-border/70">
                        <button
                          type="button"
                          onClick={() =>
                            onChangeAppSettings((prev) => ({
                              ...prev,
                              language: "vi",
                            }))
                          }
                          className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                            lang === "vi"
                              ? "bg-primary text-primary-foreground shadow-xs"
                              : "text-muted-foreground hover:text-foreground"
                          }`}
                        >
                          Tiếng Việt
                        </button>
                        <button
                          type="button"
                          onClick={() =>
                            onChangeAppSettings((prev) => ({
                              ...prev,
                              language: "en",
                            }))
                          }
                          className={`px-3 py-1.5 rounded-lg text-xs font-semibold transition-all cursor-pointer ${
                            lang === "en"
                              ? "bg-primary text-primary-foreground shadow-xs"
                              : "text-muted-foreground hover:text-foreground"
                          }`}
                        >
                          English
                        </button>
                      </div>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <div className="flex items-start gap-3">
                        <div className="p-2 rounded-xl bg-amber-500/10 text-amber-400 border border-amber-500/20">
                          <Zap className="w-4 h-4" />
                        </div>
                        <div>
                          <div className="text-[13px] font-medium text-foreground flex items-center gap-2">
                            <span>{i18n.settingsFastSyncLabel}</span>
                            <span className="px-2 py-0.5 rounded-full text-[10px] font-bold bg-amber-500/20 text-amber-300 border border-amber-500/35">
                              {i18n.sftpFastSyncTag}
                            </span>
                          </div>
                          <div className="text-[11px] text-muted-foreground">
                            {i18n.settingsFastSyncDesc}
                          </div>
                        </div>
                      </div>
                      <button
                        type="button"
                        role="switch"
                        aria-checked={appSettings.fastSyncEnabled ?? true}
                        onClick={() =>
                          onChangeAppSettings((prev) => ({
                            ...prev,
                            fastSyncEnabled: !(prev.fastSyncEnabled ?? true),
                          }))
                        }
                        className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                          (appSettings.fastSyncEnabled ?? true)
                            ? "bg-primary"
                            : "bg-secondary"
                        }`}
                      >
                        <span
                          className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out ${
                            (appSettings.fastSyncEnabled ?? true)
                              ? "translate-x-5"
                              : "translate-x-0"
                          }`}
                        />
                      </button>
                    </div>
                  </div>
                </section>

                <section className="space-y-2.5">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    Application Surface Theme
                  </h2>
                  <p className="text-xs text-muted-foreground px-1">
                    Controls the application window, sidebar, cards, and modals
                    independently of the SSH/Local terminal canvas.
                  </p>
                  <div className="grid grid-cols-2 sm:grid-cols-3 gap-3 pt-1">
                    {APP_THEMES.map((t) => {
                      const active = themeConfig.appTheme === t.id;
                      return (
                        <button
                          key={t.id}
                          type="button"
                          onClick={() =>
                            onChangeTheme((prev) => ({
                              ...prev,
                              appTheme: t.id as AppThemeId,
                            }))
                          }
                          className={`group flex flex-col gap-2.5 p-3.5 rounded-2xl border text-left transition-all duration-200 cursor-pointer ${
                            active
                              ? "border-primary bg-primary/10 shadow-sm scale-[1.01]"
                              : "border-border/80 bg-card/70 hover:bg-accent/50 hover:border-border"
                          }`}
                        >
                          <div className="flex items-center justify-between w-full">
                            <div
                              className="w-10 h-6 rounded-lg border flex items-center justify-center shadow-inner"
                              style={{
                                backgroundColor: t.bgHex,
                                borderColor: t.borderHex,
                              }}
                            >
                              <div
                                className="w-5 h-2.5 rounded-xs"
                                style={{ backgroundColor: t.cardHex }}
                              />
                            </div>
                            {active && (
                              <span className="w-5 h-5 rounded-full bg-primary text-primary-foreground flex items-center justify-center">
                                <Check className="w-3 h-3" />
                              </span>
                            )}
                          </div>
                          <div>
                            <div className="text-[13px] font-semibold text-foreground">
                              {t.name}
                            </div>
                            <div className="text-[11px] text-muted-foreground">
                              {t.subtitle}
                            </div>
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </section>

                <section className="space-y-2.5">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    Interface Accent Color
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <div>
                        <div className="text-[13px] font-medium text-foreground">
                          Primary Highlight & Telemetry Accent
                        </div>
                        <div className="text-[11px] text-muted-foreground">
                          Applied to active navigation pills, switches, and
                          primary action buttons
                        </div>
                      </div>
                      <div className="flex items-center gap-2">
                        {ACCENT_COLORS.map((acc) => {
                          const active = themeConfig.accentColor === acc.id;
                          return (
                            <button
                              key={acc.id}
                              type="button"
                              title={acc.name}
                              onClick={() =>
                                onChangeTheme((prev) => ({
                                  ...prev,
                                  accentColor: acc.id as AccentColorId,
                                }))
                              }
                              className={`w-7 h-7 rounded-full flex items-center justify-center transition-all duration-150 cursor-pointer ${
                                active
                                  ? "ring-2 ring-offset-2 ring-offset-background ring-primary scale-110"
                                  : "opacity-80 hover:opacity-100 hover:scale-105"
                              }`}
                              style={{ backgroundColor: acc.swatchHex }}
                            >
                              {active && (
                                <Check className="w-3.5 h-3.5 text-black stroke-[2.5]" />
                              )}
                            </button>
                          );
                        })}
                      </div>
                    </div>
                  </div>
                </section>
                <section className="space-y-2.5">
                  <div className="rounded-2xl border border-border/80 bg-card/65 flex items-center justify-between gap-3 px-4 py-3.5">
                    <div className="text-xs text-muted-foreground">
                      {lang === "vi"
                        ? "Đưa theme và mọi cài đặt về mặc định"
                        : "Restore theme and all settings to defaults"}
                    </div>
                    {confirmReset ? (
                      <div className="flex items-center gap-2 shrink-0">
                        <button
                          type="button"
                          onClick={() => setConfirmReset(false)}
                          className="px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 transition-all cursor-pointer"
                        >
                          {lang === "vi" ? "Huỷ" : "Cancel"}
                        </button>
                        <button
                          type="button"
                          onClick={() => {
                            onChangeTheme(() => DEFAULT_THEME_CONFIG);
                            onChangeAppSettings(() => DEFAULT_APP_SETTINGS);
                            setConfirmReset(false);
                          }}
                          className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-rose-500/15 text-rose-400 border border-rose-500/40 hover:bg-rose-500/25 transition-all cursor-pointer"
                        >
                          {lang === "vi" ? "Xác nhận đặt lại" : "Confirm reset"}
                        </button>
                      </div>
                    ) : (
                      <button
                        type="button"
                        onClick={() => setConfirmReset(true)}
                        className="shrink-0 inline-flex items-center gap-2 px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 transition-all cursor-pointer"
                      >
                        <RotateCcw className="w-3.5 h-3.5" />
                        {lang === "vi" ? "Đặt lại tất cả" : "Reset all settings"}
                      </button>
                    )}
                  </div>
                </section>
              </>
            )}

            {/* SECURITY & PRIVACY TAB */}
            {activeTab === "security" && (
              <section className="space-y-4">
                <h2 className="text-[13px] font-semibold text-foreground px-1">
                  Credential & Host Key Security
                </h2>
                <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                  <div className="flex items-center justify-between px-4 py-4">
                    <div className="flex items-start gap-3">
                      <div className="p-2 rounded-xl bg-emerald-500/10 text-emerald-400 border border-emerald-500/20">
                        <KeyRound className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="text-[13px] font-medium text-foreground">
                          macOS Native Keychain Storage
                        </div>
                        <div className="text-xs text-muted-foreground mt-0.5">
                          SSH passwords, Identity secrets, Key passphrases, and
                          Proxy credentials are encrypted inside macOS Keychain
                          (never stored in plaintext JSON).
                        </div>
                      </div>
                    </div>
                    <span className="px-2.5 py-1 rounded-full text-[11px] font-medium bg-emerald-500/15 text-emerald-400 border border-emerald-500/30 shrink-0">
                      Active
                    </span>
                  </div>

                  <div className="flex items-center justify-between px-4 py-4">
                    <div className="flex items-start gap-3">
                      <div className="p-2 rounded-xl bg-sky-500/10 text-sky-400 border border-sky-500/20">
                        <ShieldCheck className="w-4 h-4" />
                      </div>
                      <div>
                        <div className="text-[13px] font-medium text-foreground">
                          Host Key Verification (TOFU)
                        </div>
                        <div className="text-xs text-muted-foreground mt-0.5">
                          Verifies remote server SHA-256 fingerprints against{" "}
                          <code className="font-mono text-foreground">
                            ~/.ssh/known_hosts
                          </code>{" "}
                          and blocks connections if a host key changes
                          unexpectedly.
                        </div>
                      </div>
                    </div>
                    <span className="px-2.5 py-1 rounded-full text-[11px] font-medium bg-sky-500/15 text-sky-400 border border-sky-500/30 shrink-0">
                      Enforced
                    </span>
                  </div>
                </div>
              </section>
            )}

            {/* CONNECTIONS TAB (Matches Reference Screenshot 10) */}
            {activeTab === "connections" && (
              <>
                <section className="space-y-2">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    Timeouts
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Connection Timeout
                      </span>
                      <div className="flex items-center gap-2">
                        <input
                          type="number"
                          min={3}
                          max={120}
                          value={appSettings.connectionTimeoutSecs}
                          onChange={(e) =>
                            onChangeAppSettings((prev) => ({
                              ...prev,
                              connectionTimeoutSecs: Math.max(
                                3,
                                Math.min(120, Number(e.target.value) || 15),
                              ),
                            }))
                          }
                          className="w-16 px-2.5 py-1.5 rounded-lg bg-secondary/80 border border-border text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                        />
                        <span className="text-xs text-muted-foreground w-14">
                          seconds
                        </span>
                      </div>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Snippet Execution Timeout
                      </span>
                      <div className="flex items-center gap-2">
                        <input
                          type="number"
                          min={10}
                          max={3600}
                          value={appSettings.snippetTimeoutSecs}
                          onChange={(e) =>
                            onChangeAppSettings((prev) => ({
                              ...prev,
                              snippetTimeoutSecs: Math.max(
                                10,
                                Math.min(3600, Number(e.target.value) || 300),
                              ),
                            }))
                          }
                          className="w-16 px-2.5 py-1.5 rounded-lg bg-secondary/80 border border-border text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                        />
                        <span className="text-xs text-muted-foreground w-14">
                          seconds
                        </span>
                      </div>
                    </div>
                  </div>
                </section>

                <section className="space-y-2">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    Terminal Sessions
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Keep Alive
                      </span>
                      <button
                        type="button"
                        role="switch"
                        aria-checked={appSettings.keepAliveEnabled}
                        onClick={() =>
                          onChangeAppSettings((prev) => ({
                            ...prev,
                            keepAliveEnabled: !prev.keepAliveEnabled,
                          }))
                        }
                        className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                          appSettings.keepAliveEnabled
                            ? "bg-primary"
                            : "bg-secondary"
                        }`}
                      >
                        <span
                          className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out ${
                            appSettings.keepAliveEnabled
                              ? "translate-x-5"
                              : "translate-x-0"
                          }`}
                        />
                      </button>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Alive Interval
                      </span>
                      <div className="flex items-center gap-2">
                        <input
                          type="number"
                          min={5}
                          max={600}
                          disabled={!appSettings.keepAliveEnabled}
                          value={appSettings.keepAliveIntervalSecs}
                          onChange={(e) =>
                            onChangeAppSettings((prev) => ({
                              ...prev,
                              keepAliveIntervalSecs: Math.max(
                                5,
                                Math.min(600, Number(e.target.value) || 60),
                              ),
                            }))
                          }
                          className="w-16 px-2.5 py-1.5 rounded-lg bg-secondary/80 border border-border text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary disabled:opacity-40"
                        />
                        <span className="text-xs text-muted-foreground w-14">
                          seconds
                        </span>
                      </div>
                    </div>
                  </div>
                  <p className="text-xs text-muted-foreground px-1 pt-0.5">
                    Sends periodic signals to prevent idle SSH sessions from
                    disconnecting.
                  </p>
                </section>
              </>
            )}

            {/* DASHBOARD TAB */}
            {activeTab === "dashboard" && (
              <section className="space-y-2">
                <h2 className="text-[13px] font-semibold text-foreground px-1">
                  Telemetry & Real-Time Metrics
                </h2>
                <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                  <div className="flex items-center justify-between px-4 py-3.5">
                    <div>
                      <div className="text-[13px] font-medium text-foreground">
                        Telemetry Refresh Interval
                      </div>
                      <div className="text-[11px] text-muted-foreground">
                        Frequency of live CPU, Memory, Disk, and Network polling
                      </div>
                    </div>
                    <div className="w-40">
                      <SelectField
                        value={String(appSettings.dashboardRefreshSecs)}
                        onChange={(v) =>
                          onChangeAppSettings((prev) => ({
                            ...prev,
                            dashboardRefreshSecs: Number(v) || 2,
                          }))
                        }
                        options={[
                          { value: "1", label: "1 second" },
                          { value: "2", label: "2 seconds" },
                          { value: "5", label: "5 seconds" },
                          { value: "10", label: "10 seconds" },
                        ]}
                      />
                    </div>
                  </div>

                  <div className="flex items-center justify-between px-4 py-3.5">
                    <div>
                      <div className="text-[13px] font-medium text-foreground">
                        Temperature Unit
                      </div>
                      <div className="text-[11px] text-muted-foreground">
                        Used for CPU and GPU thermal sensors
                      </div>
                    </div>
                    <div className="w-40">
                      <SelectField
                        value={appSettings.temperatureUnit}
                        onChange={(v) =>
                          onChangeAppSettings((prev) => ({
                            ...prev,
                            temperatureUnit: v === "F" ? "F" : "C",
                          }))
                        }
                        options={[
                          { value: "C", label: "Celsius (°C)" },
                          { value: "F", label: "Fahrenheit (°F)" },
                        ]}
                      />
                    </div>
                  </div>

                  <div className="flex items-center justify-between px-4 py-3.5">
                    <div>
                      <div className="text-[13px] font-medium text-foreground">
                        Show Side Monitor Drawer in Terminal View
                      </div>
                      <div className="text-[11px] text-muted-foreground">
                        Automatically open the side monitor drawer when
                        connecting to a machine via SSH
                      </div>
                    </div>
                    <button
                      type="button"
                      role="switch"
                      aria-checked={appSettings.autoOpenMonitorOnSsh}
                      onClick={() =>
                        onChangeAppSettings((prev) => ({
                          ...prev,
                          autoOpenMonitorOnSsh: !prev.autoOpenMonitorOnSsh,
                        }))
                      }
                      className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                        appSettings.autoOpenMonitorOnSsh
                          ? "bg-primary"
                          : "bg-secondary"
                      }`}
                    >
                      <span
                        className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out ${
                          appSettings.autoOpenMonitorOnSsh
                            ? "translate-x-5"
                            : "translate-x-0"
                        }`}
                      />
                    </button>
                  </div>
                </div>
              </section>
            )}

            {/* TERMINAL TAB (Matches Reference Screenshot 11 & 12) */}
            {activeTab === "terminal" && (
              <>
                <section className="space-y-2.5">
                  <div className="flex items-center justify-between px-1">
                    <h2 className="text-[13px] font-semibold text-foreground">
                      Preview
                    </h2>
                    <div className="inline-flex items-center p-0.5 rounded-xl bg-secondary/90 border border-border/80">
                      <button
                        type="button"
                        onClick={() =>
                          onChangeTheme((prev) => ({
                            ...prev,
                            terminalPreset: "solarized-light",
                          }))
                        }
                        className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                          activePreset.mode === "light"
                            ? "bg-card text-foreground shadow-2xs"
                            : "text-muted-foreground hover:text-foreground"
                        }`}
                      >
                        <Sun className="w-3.5 h-3.5" />
                        Light
                      </button>
                      <button
                        type="button"
                        onClick={() => {
                          if (activePreset.mode === "light") {
                            onChangeTheme((prev) => ({
                              ...prev,
                              terminalPreset: "tokyo-night",
                            }));
                          }
                        }}
                        className={`inline-flex items-center gap-1.5 px-3 py-1 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                          activePreset.mode === "dark"
                            ? "bg-card text-foreground shadow-2xs"
                            : "text-muted-foreground hover:text-foreground"
                        }`}
                      >
                        <Moon className="w-3.5 h-3.5" />
                        Dark
                      </button>
                    </div>
                  </div>

                  {/* Live ANSI Terminal Preview Box */}
                  <div
                    className="rounded-2xl border border-border/90 p-5 shadow-inner transition-colors duration-200 overflow-x-auto"
                    style={{
                      backgroundColor: activePreset.theme.background,
                      color: activePreset.theme.foreground,
                      fontFamily: resolveTerminalFontCss(
                        themeConfig.fontFamily,
                      ),
                      fontSize: `${themeConfig.fontSize}px`,
                      lineHeight: themeConfig.lineHeight,
                      letterSpacing: `${themeConfig.letterSpacing}px`,
                    }}
                  >
                    <div>
                      <span
                        style={{
                          color: activePreset.theme.green,
                          fontWeight: 600,
                        }}
                      >
                        user@host
                      </span>
                      <span>:</span>
                      <span
                        style={{
                          color: activePreset.theme.blue,
                          fontWeight: 600,
                        }}
                      >
                        ~
                      </span>
                      <span>$ ls -la</span>
                    </div>
                    <div>
                      <span>drwxr-xr-x  5 user staff  160B Mar 19 </span>
                      <span style={{ color: activePreset.theme.cyan }}>
                        .config
                      </span>
                    </div>
                    <div>
                      <span>-rwxr-xr-x  1 user staff  2.4K Mar 19 </span>
                      <span style={{ color: activePreset.theme.red }}>
                        deploy.sh
                      </span>
                    </div>
                    <div>
                      <span>-rw-r--r--  1 user staff  512B Mar 19 </span>
                      <span style={{ color: activePreset.theme.yellow }}>
                        config.yml
                      </span>
                    </div>
                    <div>
                      <span
                        style={{
                          color: activePreset.theme.green,
                          fontWeight: 600,
                        }}
                      >
                        user@host
                      </span>
                      <span>:</span>
                      <span
                        style={{
                          color: activePreset.theme.blue,
                          fontWeight: 600,
                        }}
                      >
                        ~
                      </span>
                      <span>$ echo </span>
                      <span style={{ color: activePreset.theme.magenta }}>
                        &quot;Ready to connect!&quot;
                      </span>
                    </div>
                    <div>Ready to connect!</div>
                    <div className="flex items-center">
                      <span
                        style={{
                          color: activePreset.theme.green,
                          fontWeight: 600,
                        }}
                      >
                        user@host
                      </span>
                      <span>:</span>
                      <span
                        style={{
                          color: activePreset.theme.blue,
                          fontWeight: 600,
                        }}
                      >
                        ~
                      </span>
                      <span>$&nbsp;</span>
                      <span
                        className="inline-block"
                        style={{
                          backgroundColor:
                            themeConfig.cursorStyle === "block"
                              ? activePreset.theme.cursor
                              : "transparent",
                          borderLeft:
                            themeConfig.cursorStyle === "bar"
                              ? `2px solid ${activePreset.theme.cursor}`
                              : undefined,
                          borderBottom:
                            themeConfig.cursorStyle === "underline"
                              ? `2px solid ${activePreset.theme.cursor}`
                              : undefined,
                          width:
                            themeConfig.cursorStyle === "bar" ? "2px" : "8px",
                          height: `${Math.round(themeConfig.fontSize * 1.15)}px`,
                        }}
                      />
                    </div>
                  </div>
                </section>

                {/* Color & Cursor Configuration Group */}
                <section className="space-y-2">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    Color
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Color Palette
                      </span>
                      <div className="w-56">
                        <SelectField
                          value={themeConfig.terminalPreset}
                          onChange={(val) =>
                            onChangeTheme((prev) => ({
                              ...prev,
                              terminalPreset: val as TerminalPresetId,
                            }))
                          }
                          options={TERMINAL_PRESETS.map((p) => ({
                            value: p.id,
                            label: p.name,
                          }))}
                        />
                      </div>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Cursor Style
                      </span>
                      <div className="inline-flex p-0.5 rounded-xl bg-secondary/90 border border-border/80">
                        {(
                          [
                            { id: "block", label: "Block" },
                            { id: "bar", label: "Bar" },
                            { id: "underline", label: "Underline" },
                          ] as { id: TerminalCursorStyle; label: string }[]
                        ).map((c) => (
                          <button
                            key={c.id}
                            type="button"
                            onClick={() =>
                              onChangeTheme((prev) => ({
                                ...prev,
                                cursorStyle: c.id,
                              }))
                            }
                            className={`px-3 py-1 rounded-lg text-xs font-medium transition-all cursor-pointer ${
                              themeConfig.cursorStyle === c.id
                                ? "bg-card text-foreground shadow-2xs"
                                : "text-muted-foreground hover:text-foreground"
                            }`}
                          >
                            {c.label}
                          </button>
                        ))}
                      </div>
                    </div>
                  </div>

                  {/* Visual Color Palette Grid */}
                  <div className="grid grid-cols-2 sm:grid-cols-4 gap-2.5 pt-2">
                    {TERMINAL_PRESETS.map((preset) => {
                      const active = themeConfig.terminalPreset === preset.id;
                      return (
                        <button
                          key={preset.id}
                          type="button"
                          onClick={() =>
                            onChangeTheme((prev) => ({
                              ...prev,
                              terminalPreset: preset.id,
                            }))
                          }
                          className={`flex flex-col gap-2 p-3 rounded-xl border text-left transition-all cursor-pointer ${
                            active
                              ? "border-primary bg-primary/10 shadow-xs"
                              : "border-border/70 bg-card/50 hover:bg-accent/40"
                          }`}
                        >
                          <div className="flex items-center justify-between">
                            <span className="text-xs font-medium text-foreground truncate">
                              {preset.name}
                            </span>
                            {active && (
                              <Check className="w-3.5 h-3.5 text-primary shrink-0" />
                            )}
                          </div>
                          <div
                            className="flex items-center gap-1.5 px-2 py-1.5 rounded-lg border border-white/10"
                            style={{
                              backgroundColor: preset.theme.background,
                            }}
                          >
                            {preset.swatches.map((hex, i) => (
                              <span
                                key={i}
                                className="w-3 h-3 rounded-full"
                                style={{ backgroundColor: hex }}
                              />
                            ))}
                          </div>
                        </button>
                      );
                    })}
                  </div>
                </section>

                {/* Font Configuration Group */}
                <section className="space-y-2">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    Font
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Font
                      </span>
                      <div className="w-52">
                        <SelectField
                          value={themeConfig.fontFamily}
                          onChange={(val) =>
                            onChangeTheme((prev) => ({
                              ...prev,
                              fontFamily: val as TerminalFontFamilyId,
                            }))
                          }
                          options={TERMINAL_FONT_FAMILIES.map((f) => ({
                            value: f.id,
                            label: f.label,
                          }))}
                        />
                      </div>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Font Size
                      </span>
                      <div className="flex items-center gap-1.5">
                        <input
                          type="number"
                          min={10}
                          max={22}
                          value={themeConfig.fontSize}
                          onChange={(e) =>
                            onChangeTheme((prev) => ({
                              ...prev,
                              fontSize: Math.max(
                                10,
                                Math.min(22, Number(e.target.value) || 13),
                              ),
                            }))
                          }
                          className="w-16 px-2.5 py-1.5 rounded-lg bg-secondary/80 border border-border text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                        />
                        <div className="inline-flex rounded-lg border border-border bg-secondary/70 overflow-hidden">
                          <button
                            type="button"
                            onClick={() =>
                              onChangeTheme((prev) => ({
                                ...prev,
                                fontSize: Math.max(10, prev.fontSize - 1),
                              }))
                            }
                            className="p-1.5 hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                          >
                            <Minus className="w-3.5 h-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={() =>
                              onChangeTheme((prev) => ({
                                ...prev,
                                fontSize: Math.min(22, prev.fontSize + 1),
                              }))
                            }
                            className="p-1.5 hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-l border-border"
                          >
                            <Plus className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Line Height
                      </span>
                      <div className="flex items-center gap-1.5">
                        <input
                          type="number"
                          step={0.1}
                          min={1.0}
                          max={2.0}
                          value={themeConfig.lineHeight}
                          onChange={(e) =>
                            onChangeTheme((prev) => ({
                              ...prev,
                              lineHeight: Number(
                                Math.max(
                                  1.0,
                                  Math.min(2.0, Number(e.target.value) || 1.2),
                                ).toFixed(1),
                              ),
                            }))
                          }
                          className="w-16 px-2.5 py-1.5 rounded-lg bg-secondary/80 border border-border text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                        />
                        <div className="inline-flex rounded-lg border border-border bg-secondary/70 overflow-hidden">
                          <button
                            type="button"
                            onClick={() =>
                              onChangeTheme((prev) => ({
                                ...prev,
                                lineHeight: Number(
                                  Math.max(1.0, prev.lineHeight - 0.1).toFixed(
                                    1,
                                  ),
                                ),
                              }))
                            }
                            className="p-1.5 hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                          >
                            <Minus className="w-3.5 h-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={() =>
                              onChangeTheme((prev) => ({
                                ...prev,
                                lineHeight: Number(
                                  Math.min(2.0, prev.lineHeight + 0.1).toFixed(
                                    1,
                                  ),
                                ),
                              }))
                            }
                            className="p-1.5 hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-l border-border"
                          >
                            <Plus className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <span className="text-[13px] font-medium text-foreground">
                        Letter Spacing
                      </span>
                      <div className="flex items-center gap-1.5">
                        <input
                          type="number"
                          step={0.5}
                          min={-2}
                          max={6}
                          value={themeConfig.letterSpacing}
                          onChange={(e) =>
                            onChangeTheme((prev) => ({
                              ...prev,
                              letterSpacing: Math.max(
                                -2,
                                Math.min(6, Number(e.target.value) || 0),
                              ),
                            }))
                          }
                          className="w-16 px-2.5 py-1.5 rounded-lg bg-secondary/80 border border-border text-right text-[13px] font-mono text-foreground focus:outline-none focus:border-primary"
                        />
                        <div className="inline-flex rounded-lg border border-border bg-secondary/70 overflow-hidden">
                          <button
                            type="button"
                            onClick={() =>
                              onChangeTheme((prev) => ({
                                ...prev,
                                letterSpacing: Math.max(
                                  -2,
                                  prev.letterSpacing - 0.5,
                                ),
                              }))
                            }
                            className="p-1.5 hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer"
                          >
                            <Minus className="w-3.5 h-3.5" />
                          </button>
                          <button
                            type="button"
                            onClick={() =>
                              onChangeTheme((prev) => ({
                                ...prev,
                                letterSpacing: Math.min(
                                  6,
                                  prev.letterSpacing + 0.5,
                                ),
                              }))
                            }
                            className="p-1.5 hover:bg-accent text-muted-foreground hover:text-foreground transition-colors cursor-pointer border-l border-border"
                          >
                            <Plus className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>
                    </div>
                  </div>
                </section>

                <section className="space-y-2">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    {i18n.settingsClipboardTitle}
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <div>
                        <div className="text-[13px] font-medium text-foreground">
                          {i18n.settingsCopyOnSelectLabel}
                        </div>
                        <div className="text-[11px] text-muted-foreground">
                          {i18n.settingsCopyOnSelectDesc}
                        </div>
                      </div>
                      <button
                        type="button"
                        role="switch"
                        aria-checked={appSettings.copyOnSelect}
                        onClick={() =>
                          onChangeAppSettings((prev) => ({
                            ...prev,
                            copyOnSelect: !prev.copyOnSelect,
                          }))
                        }
                        className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                          appSettings.copyOnSelect ? "bg-primary" : "bg-secondary"
                        }`}
                      >
                        <span
                          className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out ${
                            appSettings.copyOnSelect
                              ? "translate-x-5"
                              : "translate-x-0"
                          }`}
                        />
                      </button>
                    </div>
                  </div>
                </section>
                {/* Local Shell (empty list on mobile, where only the embedded shell exists) */}
                {shellOptions.length > 0 && (
                  <section className="space-y-2">
                    <h2 className="text-[13px] font-semibold text-foreground px-1">
                      Shell
                    </h2>
                    <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60">
                      <div className="flex items-center justify-between px-4 py-3.5">
                        <div>
                          <div className="text-[13px] font-medium text-foreground">
                            {i18n.settingsLocalShellLabel}
                          </div>
                          <div className="text-[11px] text-muted-foreground">
                            {i18n.settingsLocalShellDesc}
                          </div>
                        </div>
                        <div className="w-56">
                          <SelectField
                            value={appSettings.localShell}
                            onChange={(v) =>
                              onChangeAppSettings((prev) => ({
                                ...prev,
                                localShell: v,
                              }))
                            }
                            options={[
                              { value: "", label: i18n.settingsLocalShellAuto },
                              ...shellOptions.map((s) => ({
                                value: s.id,
                                label: s.label,
                              })),
                            ]}
                          />
                        </div>
                      </div>
                    </div>
                  </section>
                )}

                <section className="space-y-2">
                  <h2 className="text-[13px] font-semibold text-foreground px-1">
                    {i18n.settingsTmuxTitle}
                  </h2>
                  <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                    <div className="flex items-center justify-between px-4 py-3.5">
                      <div>
                        <div className="text-[13px] font-medium text-foreground">
                          {i18n.settingsTmuxSshLabel}
                        </div>
                        <div className="text-[11px] text-muted-foreground">
                          {i18n.settingsTmuxSshDesc}
                        </div>
                      </div>
                      <button
                        type="button"
                        role="switch"
                        aria-checked={appSettings.tmuxSshEnabled}
                        onClick={() =>
                          onChangeAppSettings((prev) => ({
                            ...prev,
                            tmuxSshEnabled: !prev.tmuxSshEnabled,
                          }))
                        }
                        className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                          appSettings.tmuxSshEnabled
                            ? "bg-primary"
                            : "bg-secondary"
                        }`}
                      >
                        <span
                          className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out ${
                            appSettings.tmuxSshEnabled
                              ? "translate-x-5"
                              : "translate-x-0"
                          }`}
                        />
                      </button>
                    </div>

                    <div className="flex items-center justify-between px-4 py-3.5">
                      <div>
                        <div className="text-[13px] font-medium text-foreground">
                          {i18n.settingsTmuxLocalLabel}
                        </div>
                        <div className="text-[11px] text-muted-foreground">
                          {i18n.settingsTmuxLocalDesc}
                        </div>
                      </div>
                      <button
                        type="button"
                        role="switch"
                        aria-checked={appSettings.tmuxLocalEnabled}
                        onClick={() =>
                          onChangeAppSettings((prev) => ({
                            ...prev,
                            tmuxLocalEnabled: !prev.tmuxLocalEnabled,
                          }))
                        }
                        className={`relative inline-flex h-6 w-11 shrink-0 cursor-pointer rounded-full border-2 border-transparent transition-colors duration-200 ease-in-out ${
                          appSettings.tmuxLocalEnabled
                            ? "bg-primary"
                            : "bg-secondary"
                        }`}
                      >
                        <span
                          className={`pointer-events-none inline-block h-5 w-5 transform rounded-full bg-white shadow-md ring-0 transition duration-200 ease-in-out ${
                            appSettings.tmuxLocalEnabled
                              ? "translate-x-5"
                              : "translate-x-0"
                          }`}
                        />
                      </button>
                    </div>
                  </div>
                </section>
              </>
            )}

            {/* MACHINE TAGS TAB */}
            {activeTab === "tags" && (
              <section className="space-y-4">
                <h2 className="text-[13px] font-semibold text-foreground px-1">
                  Manage Machine Tags
                </h2>
                <form
                  onSubmit={handleAddTag}
                  className="rounded-2xl border border-border/80 bg-card/65 p-4 flex flex-wrap items-center gap-3"
                >
                  <input
                    type="text"
                    value={newTagName}
                    onChange={(e) => setNewTagName(e.target.value)}
                    placeholder="New tag name (e.g. Production, Kubernetes, DB)..."
                    className="flex-1 min-w-[200px] px-3.5 py-2 rounded-xl bg-secondary/80 border border-border text-[13px] text-foreground placeholder:text-muted-foreground focus:outline-none focus:border-primary"
                  />
                  <div className="flex items-center gap-1.5">
                    {TAG_COLORS.map((hex) => (
                      <button
                        key={hex}
                        type="button"
                        onClick={() => setNewTagColor(hex)}
                        className={`w-6 h-6 rounded-full transition-transform cursor-pointer ${
                          newTagColor === hex
                            ? "scale-115 ring-2 ring-white"
                            : "opacity-75 hover:opacity-100"
                        }`}
                        style={{ backgroundColor: hex }}
                      />
                    ))}
                  </div>
                  <button
                    type="submit"
                    className="inline-flex items-center gap-1.5 px-4 py-2 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-95 transition-opacity cursor-pointer"
                  >
                    <Plus className="w-3.5 h-3.5" />
                    Add Tag
                  </button>
                </form>

                <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                  {resources.tags.length === 0 ? (
                    <div className="py-8 text-center text-xs text-muted-foreground">
                      No custom tags created yet.
                    </div>
                  ) : (
                    resources.tags.map((t) => (
                      <div
                        key={t.id}
                        className="flex items-center justify-between px-4 py-3"
                      >
                        <div className="flex items-center gap-3">
                          <span
                            className="w-3.5 h-3.5 rounded-full"
                            style={{ backgroundColor: t.color }}
                          />
                          <span className="text-[13px] font-medium text-foreground">
                            {t.name}
                          </span>
                        </div>
                        <button
                          type="button"
                          onClick={() => handleDeleteTag(t.id)}
                          className="p-1.5 rounded-lg text-muted-foreground hover:text-rose-400 hover:bg-rose-500/10 transition-colors cursor-pointer"
                        >
                          <Trash2 className="w-4 h-4" />
                        </button>
                      </div>
                    ))
                  )}
                </div>
              </section>
            )}

            {/* DIAGNOSTICS TAB */}
            {activeTab === "diagnostics" && (
              <section className="space-y-3">
                <h2 className="text-[13px] font-semibold text-foreground px-1">
                  Runtime & Transport Diagnostics
                </h2>
                <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                  <div className="flex items-center justify-between px-4 py-3.5">
                    <span className="text-[13px] text-muted-foreground">
                      SSH Transport Engine
                    </span>
                    <span className="text-[13px] font-mono text-foreground">
                      Async Tokio + Russh (Truecolor PTY)
                    </span>
                  </div>
                  <div className="flex items-center justify-between px-4 py-3.5">
                    <span className="text-[13px] text-muted-foreground">
                      Terminal Renderer
                    </span>
                    <span className="text-[13px] font-mono text-foreground">
                      Xterm.js Unified Canvas (Local &amp; Remote Parity)
                    </span>
                  </div>
                  <div className="flex items-center justify-between px-4 py-3.5">
                    <span className="text-[13px] text-muted-foreground">
                      Configured Identities / Keys / Proxies
                    </span>
                    <span className="text-[13px] font-mono text-foreground">
                      {resources.identities.length} Identities ·{" "}
                      {resources.sshKeys.length} Keys ·{" "}
                      {resources.proxyProfiles.length} Proxies
                    </span>
                  </div>
                </div>
              </section>
            )}

            {/* SHORTCUTS TAB */}
            {activeTab === "shortcuts" && (
              <section className="space-y-4">
                <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60 overflow-hidden">
                  <div className="px-4 py-3 text-xs font-semibold uppercase tracking-wider text-muted-foreground">
                    Keyboard Shortcuts
                  </div>
                  {[
                    { label: "Open Settings", keys: "⌘ ," },
                    { label: "Split Terminal Vertically", keys: "⌘ D" },
                    { label: "Split Terminal Horizontally", keys: "⇧ ⌘ D" },
                    { label: "Close Active Terminal Pane", keys: "⌘ W" },
                    { label: "Open Local Terminal Session", keys: "⌘ T" },
                  ].map((sc) => (
                    <div
                      key={sc.label}
                      className="flex items-center justify-between px-4 py-3 text-[13px]"
                    >
                      <span className="text-foreground">{sc.label}</span>
                      <kbd className="px-2.5 py-1 rounded-lg bg-secondary border border-border font-mono text-xs text-muted-foreground">
                        {sc.keys}
                      </kbd>
                    </div>
                  ))}
                </div>
              </section>
            )}

            {/* ABOUT TAB */}
            {activeTab === "about" && (
              <section className="space-y-4">
                <div className="rounded-2xl border border-border/80 bg-card/65 p-6 flex items-center gap-4">
                  <div className="w-14 h-14 rounded-2xl bg-primary/15 border border-primary/30 flex items-center justify-center text-primary shadow-sm">
                    <Activity className="w-7 h-7" />
                  </div>
                  <div>
                    <h2 className="text-base font-bold text-foreground">
                      SSH-Box
                    </h2>
                    <p className="text-xs text-muted-foreground mt-0.5">
                      Native macOS Server Resource Manager, Telemetry Dashboard,
                      Docker Controller &amp; SSH Workspace
                    </p>
                    <div className="inline-flex items-center gap-2 mt-2 px-2.5 py-0.5 rounded-full bg-secondary text-[11px] font-mono text-muted-foreground">
                      Version {appVersion} · Tauri v2 + Rust
                    </div>
                  </div>
                </div>
              </section>
            )}
          </div>
        </div>
      </div>
    </div>
  );
};
