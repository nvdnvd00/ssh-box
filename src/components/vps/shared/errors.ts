import { AppI18nStrings } from "../../../i18n";

export type VpsErrorCode =
  | "SUDO_REQUIRED"
  | "SUDO_DENIED"
  | "LOCKOUT_BLOCKED"
  | "COMMAND_FAILED"
  | "VERIFY_FAILED"
  | "UNSUPPORTED"
  | "TIMEOUT"
  | "WINGET_MANUAL";

const CODES: VpsErrorCode[] = [
  "SUDO_REQUIRED",
  "SUDO_DENIED",
  "LOCKOUT_BLOCKED",
  "COMMAND_FAILED",
  "VERIFY_FAILED",
  "UNSUPPORTED",
  "TIMEOUT",
  "WINGET_MANUAL",
];

// Backend errors are "CODE: detail" (src-tauri/src/vps_config.rs vps_err).
export function parseVpsError(err: unknown): { code: VpsErrorCode | null; detail: string } {
  const raw = typeof err === "string" ? err : err instanceof Error ? err.message : String(err);
  const i = raw.indexOf(":");
  const head = i >= 0 ? raw.slice(0, i) : raw;
  const code = CODES.find((c) => c === head.trim()) ?? null;
  if (!code) return { code: null, detail: raw };
  return { code, detail: i >= 0 ? raw.slice(i + 1).trim() : "" };
}

export function vpsErrorText(t: AppI18nStrings, err: unknown): string {
  const { code, detail } = parseVpsError(err);
  switch (code) {
    case "SUDO_REQUIRED":
      return t.vpsErrSudoRequired;
    case "SUDO_DENIED":
      return t.vpsErrSudoDenied;
    case "LOCKOUT_BLOCKED":
      return t.vpsLockoutReasons[detail] ?? `${t.vpsErrLockoutBlocked} ${detail}`;
    case "UNSUPPORTED":
      return t.vpsUnsupportedReasons[detail] ?? `${t.vpsErrUnsupported} ${detail}`;
    case "COMMAND_FAILED":
      return t.vpsCommandReasons[detail] ?? `${t.vpsErrCommandFailed} ${detail}`;
    case "VERIFY_FAILED":
      return `${t.vpsErrVerifyFailed} ${detail}`;
    case "TIMEOUT":
      return `${t.vpsErrTimeout} ${detail}`;
    case "WINGET_MANUAL":
      return t.instWingetManual;
    default:
      return detail;
  }
}
