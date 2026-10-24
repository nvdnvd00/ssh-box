import type { ITheme } from "@xterm/xterm";

export type AppThemeId =
  | "zinc-dark"
  | "obsidian-dark"
  | "stone-dark"
  | "midnight-dark"
  | "zinc-light";

export type AccentColorId =
  | "emerald"
  | "cyan"
  | "violet"
  | "amber"
  | "rose"
  | "monokai";

export type TerminalPresetId =
  | "monokai"
  | "tokyo-night"
  | "catppuccin-mocha"
  | "dracula"
  | "one-dark"
  | "nord"
  | "obsidian"
  | "solarized-light";

export type TerminalCursorStyle = "block" | "bar" | "underline";

export type TerminalFontFamilyId =
  | "jetbrains"
  | "sf-mono"
  | "menlo"
  | "fira-code"
  | "system-mono";

export interface AppThemeConfig {
  appTheme: AppThemeId;
  accentColor: AccentColorId;
}

export interface TerminalThemeConfig {
  terminalPreset: TerminalPresetId;
  fontFamily: TerminalFontFamilyId;
  fontSize: number;
  lineHeight: number;
  letterSpacing: number;
  cursorStyle: TerminalCursorStyle;
}

export type AppLanguage = "vi" | "en";

export interface AppSettingsConfig {
  language: AppLanguage;
  fastSyncEnabled: boolean;
  connectionTimeoutSecs: number;
  snippetTimeoutSecs: number;
  keepAliveEnabled: boolean;
  keepAliveIntervalSecs: number;
  dashboardRefreshSecs: number;
  temperatureUnit: "C" | "F";
  autoOpenMonitorOnSsh: boolean;
  copyOnSelect: boolean;
  /** Shell id from `list_local_shells`; "" = platform default */
  localShell: string;
  tmuxSshEnabled: boolean;
  tmuxLocalEnabled: boolean;
}

export interface ThemeConfig extends AppThemeConfig, TerminalThemeConfig {}

export interface AppThemeMeta {
  id: AppThemeId;
  name: string;
  subtitle: string;
  mode: "dark" | "light";
  bgHex: string;
  cardHex: string;
  borderHex: string;
  fgHex: string;
}

export interface AccentColorMeta {
  id: AccentColorId;
  name: string;
  swatchHex: string;
  chartStroke: string;
  chartFill: string;
}

export interface TerminalPresetMeta {
  id: TerminalPresetId;
  name: string;
  badge?: string;
  mode: "dark" | "light";
  swatches: [string, string, string, string, string];
  theme: ITheme;
}

export const TERMINAL_FONT_FAMILIES: {
  id: TerminalFontFamilyId;
  label: string;
  cssValue: string;
}[] = [
  {
    id: "sf-mono",
    label: "SF Mono",
    cssValue:
      '"SF Mono", "JetBrainsMono Nerd Font", "JetBrainsMono Nerd Font Mono", "Hack Nerd Font", Menlo, Monaco, "Courier New", monospace',
  },
  {
    id: "jetbrains",
    label: "JetBrains Mono",
    cssValue:
      '"JetBrainsMono Nerd Font", "JetBrainsMono Nerd Font Mono", "JetBrains Mono", "SF Mono", "Hack Nerd Font", Menlo, Monaco, "Courier New", monospace',
  },
  {
    id: "menlo",
    label: "Menlo Regular",
    cssValue:
      'Menlo, "JetBrainsMono Nerd Font", "Hack Nerd Font", Monaco, "SF Mono", "Courier New", monospace',
  },
  {
    id: "fira-code",
    label: "Fira Code",
    cssValue:
      '"FiraCode Nerd Font", "Fira Code", "Hack Nerd Font", "JetBrainsMono Nerd Font", "JetBrains Mono", Menlo, monospace',
  },
  {
    id: "system-mono",
    label: "System Monospace",
    cssValue:
      'ui-monospace, SFMono-Regular, "SF Mono", "JetBrainsMono Nerd Font", "Hack Nerd Font", Menlo, Monaco, Consolas, monospace',
  },
];

export function resolveTerminalFontCss(id?: TerminalFontFamilyId): string {
  return (
    TERMINAL_FONT_FAMILIES.find((f) => f.id === id)?.cssValue ??
    TERMINAL_FONT_FAMILIES[0].cssValue
  );
}

export const APP_THEMES: AppThemeMeta[] = [
  {
    id: "zinc-dark",
    name: "Zinc",
    subtitle: "Modern Neutral Dark",
    mode: "dark",
    bgHex: "#09090b",
    cardHex: "#121216",
    borderHex: "#27272a",
    fgHex: "#fafafa",
  },
  {
    id: "obsidian-dark",
    name: "Obsidian",
    subtitle: "Deep Slate Dark",
    mode: "dark",
    bgHex: "#0b0e14",
    cardHex: "#101520",
    borderHex: "#1e293b",
    fgHex: "#f1f5f9",
  },
  {
    id: "stone-dark",
    name: "Carbon",
    subtitle: "Warm Graphite Dark",
    mode: "dark",
    bgHex: "#111010",
    cardHex: "#181615",
    borderHex: "#292524",
    fgHex: "#fafaf9",
  },
  {
    id: "midnight-dark",
    name: "Midnight",
    subtitle: "Deep Indigo Dark",
    mode: "dark",
    bgHex: "#0b0a12",
    cardHex: "#12101e",
    borderHex: "#26213d",
    fgHex: "#f5f3ff",
  },
  {
    id: "zinc-light",
    name: "Daylight",
    subtitle: "Clean Light Surface",
    mode: "light",
    bgHex: "#f8fafc",
    cardHex: "#ffffff",
    borderHex: "#e2e8f0",
    fgHex: "#0f172a",
  },
];

export const ACCENT_COLORS: AccentColorMeta[] = [
  {
    id: "emerald",
    name: "Emerald",
    swatchHex: "#10b981",
    chartStroke: "#34d399",
    chartFill: "rgba(52, 211, 153, 0.15)",
  },
  {
    id: "cyan",
    name: "Ocean Cyan",
    swatchHex: "#06b6d4",
    chartStroke: "#22d3ee",
    chartFill: "rgba(34, 211, 238, 0.15)",
  },
  {
    id: "violet",
    name: "Royal Violet",
    swatchHex: "#8b5cf6",
    chartStroke: "#a78bfa",
    chartFill: "rgba(167, 139, 250, 0.15)",
  },
  {
    id: "amber",
    name: "Solar Amber",
    swatchHex: "#f59e0b",
    chartStroke: "#fbbf24",
    chartFill: "rgba(251, 191, 36, 0.15)",
  },
  {
    id: "rose",
    name: "Crimson Rose",
    swatchHex: "#f43f5e",
    chartStroke: "#fb7185",
    chartFill: "rgba(251, 113, 133, 0.15)",
  },
  {
    id: "monokai",
    name: "Lime Green",
    swatchHex: "#a9dc76",
    chartStroke: "#a9dc76",
    chartFill: "rgba(169, 220, 118, 0.16)",
  },
];

export const TERMINAL_PRESETS: TerminalPresetMeta[] = [
  {
    id: "tokyo-night",
    name: "Tokyo Night",
    badge: "Default",
    mode: "dark",
    swatches: ["#f7768e", "#9ece6a", "#e0af68", "#bb9af7", "#7aa2f7"],
    theme: {
      background: "#16161e",
      foreground: "#c0caf5",
      cursor: "#c0caf5",
      cursorAccent: "#16161e",
      selectionBackground: "rgba(122, 162, 247, 0.28)",
      black: "#15161e",
      red: "#f7768e",
      green: "#9ece6a",
      yellow: "#e0af68",
      blue: "#7aa2f7",
      magenta: "#bb9af7",
      cyan: "#7dcfff",
      white: "#a9b1d6",
      brightBlack: "#414868",
      brightRed: "#f7768e",
      brightGreen: "#9ece6a",
      brightYellow: "#e0af68",
      brightBlue: "#7aa2f7",
      brightMagenta: "#bb9af7",
      brightCyan: "#7dcfff",
      brightWhite: "#c0caf5",
    },
  },
  {
    id: "monokai",
    name: "Monokai Pro",
    mode: "dark",
    swatches: ["#ff6188", "#a9dc76", "#ffd866", "#ab9df2", "#78dce8"],
    theme: {
      background: "#19181a",
      foreground: "#fcfcfa",
      cursor: "#ffd866",
      cursorAccent: "#19181a",
      selectionBackground: "rgba(255, 216, 102, 0.24)",
      black: "#221f22",
      red: "#ff6188",
      green: "#a9dc76",
      yellow: "#ffd866",
      blue: "#fc9867",
      magenta: "#ab9df2",
      cyan: "#78dce8",
      white: "#fcfcfa",
      brightBlack: "#727072",
      brightRed: "#ff6188",
      brightGreen: "#a9dc76",
      brightYellow: "#ffd866",
      brightBlue: "#fc9867",
      brightMagenta: "#ab9df2",
      brightCyan: "#78dce8",
      brightWhite: "#ffffff",
    },
  },
  {
    id: "catppuccin-mocha",
    name: "Catppuccin Mocha",
    mode: "dark",
    swatches: ["#f38ba8", "#a6e3a1", "#f9e2af", "#cba6f7", "#89b4fa"],
    theme: {
      background: "#1e1e2e",
      foreground: "#cdd6f4",
      cursor: "#f5e0dc",
      cursorAccent: "#1e1e2e",
      selectionBackground: "rgba(203, 166, 247, 0.25)",
      black: "#45475a",
      red: "#f38ba8",
      green: "#a6e3a1",
      yellow: "#f9e2af",
      blue: "#89b4fa",
      magenta: "#f5c2e7",
      cyan: "#94e2d5",
      white: "#bac2de",
      brightBlack: "#585b70",
      brightRed: "#f38ba8",
      brightGreen: "#a6e3a1",
      brightYellow: "#f9e2af",
      brightBlue: "#89b4fa",
      brightMagenta: "#cba6f7",
      brightCyan: "#89dceb",
      brightWhite: "#ffffff",
    },
  },
  {
    id: "dracula",
    name: "Dracula",
    mode: "dark",
    swatches: ["#ff5555", "#50fa7b", "#f1fa8c", "#ff79c6", "#8be9fd"],
    theme: {
      background: "#1e1f29",
      foreground: "#f8f8f2",
      cursor: "#ff79c6",
      cursorAccent: "#1e1f29",
      selectionBackground: "rgba(189, 147, 249, 0.28)",
      black: "#21222c",
      red: "#ff5555",
      green: "#50fa7b",
      yellow: "#f1fa8c",
      blue: "#bd93f9",
      magenta: "#ff79c6",
      cyan: "#8be9fd",
      white: "#f8f8f2",
      brightBlack: "#6272a4",
      brightRed: "#ff6e6e",
      brightGreen: "#69ff94",
      brightYellow: "#ffffa5",
      brightBlue: "#d6acff",
      brightMagenta: "#ff92df",
      brightCyan: "#a4ffff",
      brightWhite: "#ffffff",
    },
  },
  {
    id: "one-dark",
    name: "One Dark Pro",
    mode: "dark",
    swatches: ["#e06c75", "#98c379", "#e5c07b", "#c678dd", "#61afef"],
    theme: {
      background: "#1e222a",
      foreground: "#abb2bf",
      cursor: "#528bff",
      cursorAccent: "#1e222a",
      selectionBackground: "rgba(97, 175, 239, 0.25)",
      black: "#282c34",
      red: "#e06c75",
      green: "#98c379",
      yellow: "#e5c07b",
      blue: "#61afef",
      magenta: "#c678dd",
      cyan: "#56b6c2",
      white: "#abb2bf",
      brightBlack: "#5c6370",
      brightRed: "#e06c75",
      brightGreen: "#98c379",
      brightYellow: "#d19a66",
      brightBlue: "#61afef",
      brightMagenta: "#c678dd",
      brightCyan: "#56b6c2",
      brightWhite: "#ffffff",
    },
  },
  {
    id: "nord",
    name: "Nord Frost",
    mode: "dark",
    swatches: ["#bf616a", "#a3be8c", "#ebcb8b", "#b48ead", "#88c0d0"],
    theme: {
      background: "#2e3440",
      foreground: "#d8dee9",
      cursor: "#88c0d0",
      cursorAccent: "#2e3440",
      selectionBackground: "rgba(136, 192, 208, 0.25)",
      black: "#3b4252",
      red: "#bf616a",
      green: "#a3be8c",
      yellow: "#ebcb8b",
      blue: "#81a1c1",
      magenta: "#b48ead",
      cyan: "#88c0d0",
      white: "#e5e9f0",
      brightBlack: "#4c566a",
      brightRed: "#bf616a",
      brightGreen: "#a3be8c",
      brightYellow: "#ebcb8b",
      brightBlue: "#81a1c1",
      brightMagenta: "#b48ead",
      brightCyan: "#8fbcbb",
      brightWhite: "#eceff4",
    },
  },
  {
    id: "obsidian",
    name: "Obsidian Cyber",
    mode: "dark",
    swatches: ["#f87171", "#4ade80", "#facc15", "#c084fc", "#38bdf8"],
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
  },
  {
    id: "solarized-light",
    name: "Solarized Light",
    mode: "light",
    swatches: ["#dc322f", "#859900", "#b58900", "#d33682", "#268bd2"],
    theme: {
      background: "#fdf6e3",
      foreground: "#586e75",
      cursor: "#268bd2",
      cursorAccent: "#fdf6e3",
      selectionBackground: "rgba(38, 139, 210, 0.2)",
      black: "#073642",
      red: "#dc322f",
      green: "#859900",
      yellow: "#b58900",
      blue: "#268bd2",
      magenta: "#d33682",
      cyan: "#2aa198",
      white: "#eee8d5",
      brightBlack: "#657b83",
      brightRed: "#cb4b16",
      brightGreen: "#859900",
      brightYellow: "#b58900",
      brightBlue: "#268bd2",
      brightMagenta: "#6c71c4",
      brightCyan: "#2aa198",
      brightWhite: "#002b36",
    },
  },
];

const APP_THEME_STORAGE_KEY = "ssh-box:app-theme-v2";
const TERM_THEME_STORAGE_KEY = "ssh-box:terminal-theme-v2";
const APP_SETTINGS_STORAGE_KEY = "ssh-box:app-settings-v1";

export const DEFAULT_APP_THEME: AppThemeConfig = {
  appTheme: "obsidian-dark",
  accentColor: "emerald",
};

export const DEFAULT_TERMINAL_THEME: TerminalThemeConfig = {
  terminalPreset: "tokyo-night",
  fontFamily: "sf-mono",
  fontSize: 13,
  lineHeight: 1.2,
  letterSpacing: 0,
  cursorStyle: "block",
};

export const DEFAULT_APP_SETTINGS: AppSettingsConfig = {
  language: "vi",
  fastSyncEnabled: true,
  connectionTimeoutSecs: 15,
  snippetTimeoutSecs: 300,
  keepAliveEnabled: true,
  keepAliveIntervalSecs: 60,
  dashboardRefreshSecs: 2,
  temperatureUnit: "C",
  autoOpenMonitorOnSsh: false,
  copyOnSelect: false,
  localShell: "",
  tmuxSshEnabled: false,
  tmuxLocalEnabled: false,
};

export const DEFAULT_THEME_CONFIG: ThemeConfig = {
  ...DEFAULT_APP_THEME,
  ...DEFAULT_TERMINAL_THEME,
};

export function parseThemeConfig(
  parsedApp: Partial<AppThemeConfig> = {},
  parsedTerm: Partial<TerminalThemeConfig> = {},
): ThemeConfig {
  return {
    appTheme: APP_THEMES.some((t) => t.id === parsedApp.appTheme)
      ? (parsedApp.appTheme as AppThemeId)
      : DEFAULT_APP_THEME.appTheme,
    accentColor: ACCENT_COLORS.some((a) => a.id === parsedApp.accentColor)
      ? (parsedApp.accentColor as AccentColorId)
      : DEFAULT_APP_THEME.accentColor,
    terminalPreset: TERMINAL_PRESETS.some(
      (p) => p.id === parsedTerm.terminalPreset,
    )
      ? (parsedTerm.terminalPreset as TerminalPresetId)
      : DEFAULT_TERMINAL_THEME.terminalPreset,
    fontFamily: TERMINAL_FONT_FAMILIES.some(
      (f) => f.id === parsedTerm.fontFamily,
    )
      ? (parsedTerm.fontFamily as TerminalFontFamilyId)
      : DEFAULT_TERMINAL_THEME.fontFamily,
    fontSize:
      typeof parsedTerm.fontSize === "number" &&
      parsedTerm.fontSize >= 10 &&
      parsedTerm.fontSize <= 22
        ? parsedTerm.fontSize
        : DEFAULT_TERMINAL_THEME.fontSize,
    lineHeight:
      typeof parsedTerm.lineHeight === "number" &&
      parsedTerm.lineHeight >= 1.0 &&
      parsedTerm.lineHeight <= 2.0
        ? parsedTerm.lineHeight
        : DEFAULT_TERMINAL_THEME.lineHeight,
    letterSpacing:
      typeof parsedTerm.letterSpacing === "number" &&
      parsedTerm.letterSpacing >= -2 &&
      parsedTerm.letterSpacing <= 6
        ? parsedTerm.letterSpacing
        : DEFAULT_TERMINAL_THEME.letterSpacing,
    cursorStyle:
      parsedTerm.cursorStyle === "block" ||
      parsedTerm.cursorStyle === "bar" ||
      parsedTerm.cursorStyle === "underline"
        ? parsedTerm.cursorStyle
        : DEFAULT_TERMINAL_THEME.cursorStyle,
  };
}

export function loadThemeConfig(): ThemeConfig {
  try {
    const rawApp = localStorage.getItem(APP_THEME_STORAGE_KEY);
    const rawTerm = localStorage.getItem(TERM_THEME_STORAGE_KEY);
    const parsedApp = rawApp
      ? (JSON.parse(rawApp) as Partial<AppThemeConfig>)
      : {};
    const parsedTerm = rawTerm
      ? (JSON.parse(rawTerm) as Partial<TerminalThemeConfig>)
      : {};

    return parseThemeConfig(parsedApp, parsedTerm);
  } catch {
    return DEFAULT_THEME_CONFIG;
  }
}

export function saveThemeConfig(config: ThemeConfig): void {
  try {
    const appCfg: AppThemeConfig = {
      appTheme: config.appTheme,
      accentColor: config.accentColor,
    };
    const termCfg: TerminalThemeConfig = {
      terminalPreset: config.terminalPreset,
      fontFamily: config.fontFamily,
      fontSize: config.fontSize,
      lineHeight: config.lineHeight,
      letterSpacing: config.letterSpacing,
      cursorStyle: config.cursorStyle,
    };
    localStorage.setItem(APP_THEME_STORAGE_KEY, JSON.stringify(appCfg));
    localStorage.setItem(TERM_THEME_STORAGE_KEY, JSON.stringify(termCfg));
  } catch {
    // Ignore storage quota errors
  }
}

export function parseAppSettings(
  parsed: Partial<AppSettingsConfig> = {},
): AppSettingsConfig {
  return {
    language:
      parsed.language === "en" || parsed.language === "vi"
        ? parsed.language
        : DEFAULT_APP_SETTINGS.language,
    fastSyncEnabled:
      typeof parsed.fastSyncEnabled === "boolean"
        ? parsed.fastSyncEnabled
        : DEFAULT_APP_SETTINGS.fastSyncEnabled,
    connectionTimeoutSecs:
      typeof parsed.connectionTimeoutSecs === "number" &&
      parsed.connectionTimeoutSecs >= 3 &&
      parsed.connectionTimeoutSecs <= 120
        ? parsed.connectionTimeoutSecs
        : DEFAULT_APP_SETTINGS.connectionTimeoutSecs,
    snippetTimeoutSecs:
      typeof parsed.snippetTimeoutSecs === "number" &&
      parsed.snippetTimeoutSecs >= 10 &&
      parsed.snippetTimeoutSecs <= 3600
        ? parsed.snippetTimeoutSecs
        : DEFAULT_APP_SETTINGS.snippetTimeoutSecs,
    keepAliveEnabled:
      typeof parsed.keepAliveEnabled === "boolean"
        ? parsed.keepAliveEnabled
        : DEFAULT_APP_SETTINGS.keepAliveEnabled,
    keepAliveIntervalSecs:
      typeof parsed.keepAliveIntervalSecs === "number" &&
      parsed.keepAliveIntervalSecs >= 5 &&
      parsed.keepAliveIntervalSecs <= 600
        ? parsed.keepAliveIntervalSecs
        : DEFAULT_APP_SETTINGS.keepAliveIntervalSecs,
    dashboardRefreshSecs:
      typeof parsed.dashboardRefreshSecs === "number" &&
      parsed.dashboardRefreshSecs >= 1 &&
      parsed.dashboardRefreshSecs <= 30
        ? parsed.dashboardRefreshSecs
        : DEFAULT_APP_SETTINGS.dashboardRefreshSecs,
    temperatureUnit:
      parsed.temperatureUnit === "F" || parsed.temperatureUnit === "C"
        ? parsed.temperatureUnit
        : DEFAULT_APP_SETTINGS.temperatureUnit,
    autoOpenMonitorOnSsh:
      typeof parsed.autoOpenMonitorOnSsh === "boolean"
        ? parsed.autoOpenMonitorOnSsh
        : DEFAULT_APP_SETTINGS.autoOpenMonitorOnSsh,
    copyOnSelect:
      typeof parsed.copyOnSelect === "boolean"
        ? parsed.copyOnSelect
        : DEFAULT_APP_SETTINGS.copyOnSelect,
    localShell:
      typeof parsed.localShell === "string"
        ? parsed.localShell
        : DEFAULT_APP_SETTINGS.localShell,
    tmuxSshEnabled:
      typeof parsed.tmuxSshEnabled === "boolean"
        ? parsed.tmuxSshEnabled
        : DEFAULT_APP_SETTINGS.tmuxSshEnabled,
    tmuxLocalEnabled:
      typeof parsed.tmuxLocalEnabled === "boolean"
        ? parsed.tmuxLocalEnabled
        : DEFAULT_APP_SETTINGS.tmuxLocalEnabled,
  };
}

export function loadAppSettings(): AppSettingsConfig {
  try {
    const raw = localStorage.getItem(APP_SETTINGS_STORAGE_KEY);
    if (!raw) return DEFAULT_APP_SETTINGS;
    return parseAppSettings(JSON.parse(raw) as Partial<AppSettingsConfig>);
  } catch {
    return DEFAULT_APP_SETTINGS;
  }
}

export function saveAppSettings(settings: AppSettingsConfig): void {
  try {
    localStorage.setItem(APP_SETTINGS_STORAGE_KEY, JSON.stringify(settings));
  } catch {
    // Ignore storage errors
  }
}

export function applyAppThemeToRoot(config: AppThemeConfig): void {
  if (typeof document === "undefined") return;
  const root = document.documentElement;
  root.setAttribute("data-app-theme", config.appTheme);
  root.setAttribute("data-accent", config.accentColor);
}

export function resolveXtermTheme(config: TerminalThemeConfig): ITheme {
  const preset =
    TERMINAL_PRESETS.find((p) => p.id === config.terminalPreset) ??
    TERMINAL_PRESETS[0];
  return { ...preset.theme };
}
