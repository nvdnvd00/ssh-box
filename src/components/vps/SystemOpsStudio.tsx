import React, { useCallback, useEffect, useId, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  Activity,
  AlertTriangle,
  CalendarClock,
  CheckCircle2,
  Clock,
  Database,
  FileText,
  Globe,
  HardDrive,
  PauseCircle,
  Play,
  Plus,
  RefreshCw,
  RotateCw,
  Search,
  Server,
  ShieldCheck,
  Sparkles,
  Square,
  Terminal,
  Trash2,
  Wrench,
  X,
  Zap,
} from "lucide-react";
import {
  CronJobItem,
  HostEntry,
  SystemdServiceItem,
  SystemOpsSnapshot,
  VpsPrivilegeInfo,
} from "../../types";
import { AppLanguage } from "../../themes";

interface SystemOpsStudioProps {
  host: HostEntry;
  sudoPassword?: string;
  privilegeInfo: VpsPrivilegeInfo | null;
  language?: AppLanguage;
  onRequireSudoUnlock?: () => void;
}

type ServiceFilter = "core" | "active" | "failed" | "all";

const CORE_DAEMON_PREFIXES = [
  "ssh.service",
  "sshd.service",
  "nginx.service",
  "caddy.service",
  "apache2.service",
  "httpd.service",
  "haproxy.service",
  "traefik.service",
  "docker.service",
  "containerd.service",
  "ufw.service",
  "fail2ban.service",
  "postgresql.service",
  "mariadb.service",
  "mysql.service",
  "redis.service",
  "redis-server.service",
  "valkey.service",
  "mongodb.service",
  "mongod.service",
  "cron.service",
  "crond.service",
  "systemd-timesyncd.service",
  "chrony.service",
  "unattended-upgrades.service",
  "tailscaled.service",
  "wg-quick@",
];

function isCoreDaemon(unit: string): boolean {
  const lower = unit.toLowerCase();
  return CORE_DAEMON_PREFIXES.some(
    (prefix) => lower === prefix || lower.startsWith(prefix),
  );
}

function formatMbHuman(mb: number): string {
  if (mb <= 0) return "0 MB";
  if (mb >= 1024) {
    const gb = mb / 1024;
    return Number.isInteger(gb) ? `${gb} GB` : `${gb.toFixed(1)} GB`;
  }
  return `${mb} MB`;
}

const WEEKDAY_NAMES_EN: Record<string, string> = {
  "0": "Sunday",
  "1": "Monday",
  "2": "Tuesday",
  "3": "Wednesday",
  "4": "Thursday",
  "5": "Friday",
  "6": "Saturday",
  "7": "Sunday",
  sun: "Sunday",
  mon: "Monday",
  tue: "Tuesday",
  wed: "Wednesday",
  thu: "Thursday",
  fri: "Friday",
  sat: "Saturday",
};

const WEEKDAY_NAMES_VI: Record<string, string> = {
  "0": "Chủ nhật",
  "1": "Thứ Hai",
  "2": "Thứ Ba",
  "3": "Thứ Tư",
  "4": "Thứ Năm",
  "5": "Thứ Sáu",
  "6": "Thứ Bảy",
  "7": "Chủ nhật",
  sun: "Chủ nhật",
  mon: "Thứ Hai",
  tue: "Thứ Ba",
  wed: "Thứ Tư",
  thu: "Thứ Năm",
  fri: "Thứ Sáu",
  sat: "Thứ Bảy",
};

export function translateCronSchedule(schedule: string, isVi: boolean): string {
  const clean = schedule.trim();
  if (!clean) {
    return isVi ? "Chưa có lịch chạy" : "No schedule specified";
  }

  const lower = clean.toLowerCase();
  if (lower === "@reboot") {
    return isVi
      ? "Chạy một lần khi máy chủ khởi động (@reboot)"
      : "Runs once at system startup (@reboot)";
  }
  if (lower === "@hourly") {
    return isVi
      ? "Chạy mỗi giờ vào phút 00 (@hourly)"
      : "Runs every hour at minute 00 (@hourly)";
  }
  if (lower === "@daily" || lower === "@midnight") {
    return isVi
      ? "Chạy hàng ngày lúc 00:00 (@daily)"
      : "Runs daily at 00:00 midnight (@daily)";
  }
  if (lower === "@weekly") {
    return isVi
      ? "Chạy hàng tuần vào 00:00 Chủ nhật (@weekly)"
      : "Runs weekly on Sunday at 00:00 (@weekly)";
  }
  if (lower === "@monthly") {
    return isVi
      ? "Chạy hàng tháng vào ngày 1 lúc 00:00 (@monthly)"
      : "Runs monthly on day 1 at 00:00 (@monthly)";
  }
  if (lower === "@yearly" || lower === "@annually") {
    return isVi
      ? "Chạy hàng năm vào ngày 01/01 lúc 00:00 (@yearly)"
      : "Runs yearly on Jan 1 at 00:00 (@yearly)";
  }

  const parts = clean.split(/\s+/);
  if (parts.length !== 5) {
    return isVi
      ? `Biểu thức Cron tùy chỉnh (${clean})`
      : `Custom cron expression (${clean})`;
  }

  const [min, hour, dom, mon, dow] = parts;
  const pad2 = (v: string) => (/^\d+$/.test(v) ? v.padStart(2, "0") : v);

  if (min === "*" && hour === "*" && dom === "*" && mon === "*" && dow === "*") {
    return isVi
      ? "Chạy mỗi phút (* * * * *)"
      : "Runs every single minute (* * * * *)";
  }

  if (
    min.startsWith("*/") &&
    hour === "*" &&
    dom === "*" &&
    mon === "*" &&
    dow === "*"
  ) {
    const step = min.slice(2);
    return isVi
      ? `Chạy định kỳ mỗi ${step} phút`
      : `Runs every ${step} minutes`;
  }

  if (
    /^\d+$/.test(min) &&
    hour === "*" &&
    dom === "*" &&
    mon === "*" &&
    dow === "*"
  ) {
    return isVi
      ? `Chạy mỗi giờ vào phút :${pad2(min)}`
      : `Runs hourly at minute :${pad2(min)}`;
  }

  if (
    /^\d+$/.test(min) &&
    hour.startsWith("*/") &&
    dom === "*" &&
    mon === "*" &&
    dow === "*"
  ) {
    const hStep = hour.slice(2);
    return isVi
      ? `Chạy mỗi ${hStep} giờ vào phút :${pad2(min)}`
      : `Runs every ${hStep} hours at minute :${pad2(min)}`;
  }

  if (
    /^\d+$/.test(min) &&
    /^\d+$/.test(hour) &&
    dom === "*" &&
    mon === "*" &&
    dow === "*"
  ) {
    return isVi
      ? `Chạy hàng ngày lúc ${pad2(hour)}:${pad2(min)}`
      : `Runs daily at ${pad2(hour)}:${pad2(min)}`;
  }

  if (
    /^\d+$/.test(min) &&
    /^\d+$/.test(hour) &&
    dom === "*" &&
    mon === "*" &&
    dow !== "*"
  ) {
    const dayLabel = isVi
      ? WEEKDAY_NAMES_VI[dow.toLowerCase()] || `thứ (${dow})`
      : WEEKDAY_NAMES_EN[dow.toLowerCase()] || `weekday (${dow})`;
    return isVi
      ? `Chạy hàng tuần vào ${dayLabel} lúc ${pad2(hour)}:${pad2(min)}`
      : `Runs weekly on ${dayLabel} at ${pad2(hour)}:${pad2(min)}`;
  }

  if (
    /^\d+$/.test(min) &&
    /^\d+$/.test(hour) &&
    /^\d+$/.test(dom) &&
    mon === "*" &&
    dow === "*"
  ) {
    return isVi
      ? `Chạy hàng tháng vào ngày ${dom} lúc ${pad2(hour)}:${pad2(min)}`
      : `Runs monthly on day ${dom} at ${pad2(hour)}:${pad2(min)}`;
  }

  return isVi
    ? `Lịch chạy: phút [${min}] giờ [${hour}] ngày [${dom}] tháng [${mon}] thứ [${dow}]`
    : `Schedule: min [${min}] hour [${hour}] day [${dom}] month [${mon}] weekday [${dow}]`;
}

const TIMEZONE_PRESETS = [
  "UTC",
  "Asia/Ho_Chi_Minh",
  "Asia/Singapore",
  "Asia/Tokyo",
  "Asia/Seoul",
  "Europe/London",
  "Europe/Frankfurt",
  "America/New_York",
  "America/Los_Angeles",
];

const DNS_PRESETS = [
  {
    id: "cloudflare",
    label: "Cloudflare (1.1.1.1)",
    servers: ["1.1.1.1", "1.0.0.1"],
    badge: "Fastest Anycast",
  },
  {
    id: "google",
    label: "Google Public DNS",
    servers: ["8.8.8.8", "8.8.4.4"],
    badge: "Global Standard",
  },
  {
    id: "quad9",
    label: "Quad9 Security",
    servers: ["9.9.9.9", "149.112.112.112"],
    badge: "Malware Blocking",
  },
];

const SWAP_SIZE_PILLS = [
  { sizeMb: 0, labelVi: "Tắt Swap (0 MB)", labelEn: "Disable Swap (0 MB)" },
  { sizeMb: 512, labelVi: "512 MB", labelEn: "512 MB" },
  { sizeMb: 1024, labelVi: "1 GB", labelEn: "1 GB" },
  {
    sizeMb: 2048,
    labelVi: "2 GB (Khuyên dùng)",
    labelEn: "2 GB (Recommended)",
  },
  { sizeMb: 4096, labelVi: "4 GB", labelEn: "4 GB" },
  { sizeMb: 8192, labelVi: "8 GB", labelEn: "8 GB" },
];

const CRON_PRESETS = [
  {
    id: "every_min",
    labelVi: "Mỗi phút",
    labelEn: "Every Minute",
    schedule: "* * * * *",
    fields: ["*", "*", "*", "*", "*"] as [string, string, string, string, string],
  },
  {
    id: "every_5m",
    labelVi: "Mỗi 5 phút",
    labelEn: "Every 5 Mins",
    schedule: "*/5 * * * *",
    fields: ["*/5", "*", "*", "*", "*"] as [string, string, string, string, string],
  },
  {
    id: "hourly",
    labelVi: "Mỗi giờ",
    labelEn: "Hourly",
    schedule: "0 * * * *",
    fields: ["0", "*", "*", "*", "*"] as [string, string, string, string, string],
  },
  {
    id: "daily_2am",
    labelVi: "02:00 Hàng ngày",
    labelEn: "Daily 02:00",
    schedule: "0 2 * * *",
    fields: ["0", "2", "*", "*", "*"] as [string, string, string, string, string],
  },
  {
    id: "weekly_sun",
    labelVi: "Chủ nhật 03:00",
    labelEn: "Weekly Sun",
    schedule: "0 3 * * 0",
    fields: ["0", "3", "*", "*", "0"] as [string, string, string, string, string],
  },
  {
    id: "reboot",
    labelVi: "Khi khởi động (@reboot)",
    labelEn: "On Reboot (@reboot)",
    schedule: "@reboot",
    fields: null,
  },
];

export const SystemOpsStudio: React.FC<SystemOpsStudioProps> = ({
  host,
  sudoPassword,
  language = "vi",
  onRequireSudoUnlock,
}) => {
  const isVi = language === "vi";
  const systemdTableCaptionId = useId();

  const [snapshot, setSnapshot] = useState<SystemOpsSnapshot | null>(null);
  const [loading, setLoading] = useState(false);
  const [busyAction, setBusyAction] = useState<string | null>(null);
  const [errorMsg, setErrorMsg] = useState<string | null>(null);
  const [toastMsg, setToastMsg] = useState<string | null>(null);

  // Section 1A: Swap Allocator state
  const [targetSwapMb, setTargetSwapMb] = useState<number>(2048);

  // Section 1B: Host Identity, Timezone & DNS state
  const [draftHostname, setDraftHostname] = useState("");
  const [draftTimezone, setDraftTimezone] = useState("UTC");
  const [draftDnsServers, setDraftDnsServers] = useState<string[]>([]);
  const [customDnsInput, setCustomDnsInput] = useState("");

  // Section 2: Systemd Service Deck & Live Journalctl Inspector state
  const [serviceFilter, setServiceFilter] = useState<ServiceFilter>("core");
  const [serviceSearch, setServiceSearch] = useState("");
  const [inspectedUnit, setInspectedUnit] = useState<string | null>(null);
  const [logLinesCount, setLogLinesCount] = useState<number>(150);
  const [logSearch, setLogSearch] = useState("");
  const [logContent, setLogContent] = useState<string>("");
  const [loadingLogs, setLoadingLogs] = useState(false);

  // Section 3: Cron Studio Composer state
  const [isRebootSchedule, setIsRebootSchedule] = useState(false);
  const [cronMin, setCronMin] = useState("0");
  const [cronHour, setCronHour] = useState("2");
  const [cronDom, setCronDom] = useState("*");
  const [cronMon, setCronMon] = useState("*");
  const [cronDow, setCronDow] = useState("*");
  const [cronCommand, setCronCommand] = useState("");
  const [cronComment, setCronComment] = useState("");

  const syncFromSnapshot = useCallback((res: SystemOpsSnapshot) => {
    setSnapshot(res);
    setDraftHostname(res.hostname || "");
    setDraftTimezone(res.timezone || "UTC");
    setDraftDnsServers(res.dnsServers.length > 0 ? res.dnsServers : ["1.1.1.1", "1.0.0.1"]);
    const swapfileItem = res.swaps.find((s) => s.filename === "/swapfile");
    if (swapfileItem) {
      setTargetSwapMb(swapfileItem.sizeMb);
    } else if (res.swapTotalMb > 0) {
      setTargetSwapMb(res.swapTotalMb);
    } else {
      setTargetSwapMb(2048);
    }
  }, []);

  const fetchSnapshot = useCallback(async () => {
    setLoading(true);
    setErrorMsg(null);
    try {
      const res = await invoke<SystemOpsSnapshot>("vps_get_system_ops_snapshot", {
        hostId: host.id,
        sudoPassword: sudoPassword || null,
      });
      syncFromSnapshot(res);
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setLoading(false);
    }
  }, [host.id, sudoPassword, onRequireSudoUnlock, syncFromSnapshot]);

  useEffect(() => {
    fetchSnapshot();
  }, [fetchSnapshot]);

  useEffect(() => {
    if (!toastMsg) return;
    const timer = setTimeout(() => setToastMsg(null), 5000);
    return () => clearTimeout(timer);
  }, [toastMsg]);

  const handleConfigureSwap = async () => {
    setBusyAction("swap");
    setErrorMsg(null);
    try {
      const normalized =
        targetSwapMb === 0 ? 0 : Math.max(256, Math.min(32768, Math.round(targetSwapMb)));
      const res = await invoke<SystemOpsSnapshot>("vps_configure_swapfile", {
        hostId: host.id,
        sizeMb: normalized,
        sudoPassword: sudoPassword || null,
      });
      syncFromSnapshot(res);
      setToastMsg(
        normalized === 0
          ? isVi
            ? "Đã tắt và xóa /swapfile khỏi /etc/fstab!"
            : "Disabled and removed /swapfile from /etc/fstab!"
          : isVi
            ? `Đã cấp phát /swapfile (${formatMbHuman(normalized)}) và ghi bền vững vào /etc/fstab!`
            : `Allocated /swapfile (${formatMbHuman(normalized)}) and persisted in /etc/fstab!`,
      );
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handleSaveHostIdentity = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanHost = draftHostname.trim();
    const cleanTz = draftTimezone.trim();
    if (!cleanHost || !cleanTz) return;
    setBusyAction("identity");
    setErrorMsg(null);
    try {
      const res = await invoke<SystemOpsSnapshot>("vps_update_host_identity", {
        hostId: host.id,
        hostname: cleanHost,
        timezone: cleanTz,
        dnsServers: draftDnsServers,
        sudoPassword: sudoPassword || null,
      });
      syncFromSnapshot(res);
      setToastMsg(
        isVi
          ? `Đã cập nhật Hostname (${cleanHost}), Múi giờ (${cleanTz}) và DNS Resolvers!`
          : `Updated Hostname (${cleanHost}), Timezone (${cleanTz}), and DNS Resolvers!`,
      );
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handleAddCustomDns = () => {
    const clean = customDnsInput.trim();
    if (!clean) return;
    if (!draftDnsServers.includes(clean)) {
      setDraftDnsServers((prev) => [...prev, clean]);
    }
    setCustomDnsInput("");
  };

  const handleRemoveDns = (ip: string) => {
    setDraftDnsServers((prev) => prev.filter((item) => item !== ip));
  };

  const fetchServiceLogs = useCallback(
    async (unit: string, lines: number) => {
      setLoadingLogs(true);
      setErrorMsg(null);
      try {
        const out = await invoke<string>("vps_systemd_logs", {
          hostId: host.id,
          unit,
          lines,
          sudoPassword: sudoPassword || null,
        });
        setLogContent(out || (isVi ? "(Không có dòng nhật ký nào)" : "(No journal entries found)"));
      } catch (err) {
        const msg = String(err);
        setErrorMsg(msg);
        if (msg.toLowerCase().includes("sudo password required")) {
          onRequireSudoUnlock?.();
        }
      } finally {
        setLoadingLogs(false);
      }
    },
    [host.id, sudoPassword, isVi, onRequireSudoUnlock],
  );

  const handleOpenLogs = (unit: string) => {
    setInspectedUnit(unit);
    fetchServiceLogs(unit, logLinesCount);
  };

  const handleSystemdAction = async (unit: string, action: string) => {
    const busyKey = `svc:${unit}:${action}`;
    setBusyAction(busyKey);
    setErrorMsg(null);
    try {
      const res = await invoke<SystemOpsSnapshot>("vps_systemd_action", {
        hostId: host.id,
        unit,
        action,
        sudoPassword: sudoPassword || null,
      });
      syncFromSnapshot(res);
      setToastMsg(
        isVi
          ? `Đã thực thi systemctl ${action} ${unit}!`
          : `Executed systemctl ${action} ${unit}!`,
      );
      if (inspectedUnit === unit && (action === "restart" || action === "start" || action === "stop")) {
        fetchServiceLogs(unit, logLinesCount);
      }
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const composedSchedule = useMemo(() => {
    if (isRebootSchedule) return "@reboot";
    const f = [cronMin, cronHour, cronDom, cronMon, cronDow].map(
      (part) => part.trim() || "*",
    );
    return f.join(" ");
  }, [isRebootSchedule, cronMin, cronHour, cronDom, cronMon, cronDow]);

  const naturalSchedulePreview = useMemo(
    () => translateCronSchedule(composedSchedule, isVi),
    [composedSchedule, isVi],
  );

  const saveCronJobList = async (
    nextJobs: CronJobItem[],
    successToast: string,
    busyKey: string,
  ) => {
    setBusyAction(busyKey);
    setErrorMsg(null);
    try {
      const res = await invoke<SystemOpsSnapshot>("vps_save_cron_jobs", {
        hostId: host.id,
        jobs: nextJobs,
        sudoPassword: sudoPassword || null,
      });
      syncFromSnapshot(res);
      setToastMsg(successToast);
    } catch (err) {
      const msg = String(err);
      setErrorMsg(msg);
      if (msg.toLowerCase().includes("sudo password required")) {
        onRequireSudoUnlock?.();
      }
    } finally {
      setBusyAction(null);
    }
  };

  const handleAddCronJob = async (e: React.FormEvent) => {
    e.preventDefault();
    const cleanCmd = cronCommand.trim();
    if (!cleanCmd || !snapshot) return;
    const newJob: CronJobItem = {
      index: snapshot.cronJobs.length + 1,
      schedule: composedSchedule,
      command: cleanCmd,
      comment: cronComment.trim(),
      paused: false,
      rawLine: `${composedSchedule} ${cleanCmd}`,
    };
    const nextJobs = [...snapshot.cronJobs, newJob];
    await saveCronJobList(
      nextJobs,
      isVi
        ? `Đã thêm tác vụ Cron (${composedSchedule}) vào Crontab hệ thống!`
        : `Added Cron job (${composedSchedule}) to system crontab!`,
      "cron:add",
    );
    setCronCommand("");
    setCronComment("");
  };

  const handleTogglePauseCronJob = async (jobIndex: number) => {
    if (!snapshot) return;
    const nextJobs = snapshot.cronJobs.map((j) =>
      j.index === jobIndex ? { ...j, paused: !j.paused } : j,
    );
    const target = nextJobs.find((j) => j.index === jobIndex);
    await saveCronJobList(
      nextJobs,
      target?.paused
        ? isVi
          ? `Đã tạm dừng tác vụ Cron #${jobIndex} (# [SSHBOX_PAUSED])`
          : `Paused Cron job #${jobIndex} (# [SSHBOX_PAUSED])`
        : isVi
          ? `Đã kích hoạt lại tác vụ Cron #${jobIndex}`
          : `Resumed Cron job #${jobIndex}`,
      `cron:toggle:${jobIndex}`,
    );
  };

  const handleDeleteCronJob = async (jobIndex: number) => {
    if (!snapshot) return;
    const nextJobs = snapshot.cronJobs.filter((j) => j.index !== jobIndex);
    await saveCronJobList(
      nextJobs,
      isVi
        ? `Đã xóa tác vụ Cron #${jobIndex} khỏi Crontab!`
        : `Deleted Cron job #${jobIndex} from crontab!`,
      `cron:del:${jobIndex}`,
    );
  };

  const serviceCounts = useMemo(() => {
    const list = snapshot?.services ?? [];
    return {
      core: list.filter((s) => isCoreDaemon(s.unit) || s.activeState === "failed").length,
      active: list.filter((s) => s.activeState === "active").length,
      failed: list.filter((s) => s.activeState === "failed").length,
      all: list.length,
    };
  }, [snapshot?.services]);

  const filteredServices = useMemo(() => {
    const list = snapshot?.services ?? [];
    const q = serviceSearch.trim().toLowerCase();
    return list
      .filter((svc) => {
        if (serviceFilter === "core") {
          if (!isCoreDaemon(svc.unit) && svc.activeState !== "failed") return false;
        } else if (serviceFilter === "active") {
          if (svc.activeState !== "active") return false;
        } else if (serviceFilter === "failed") {
          if (svc.activeState !== "failed") return false;
        }
        if (!q) return true;
        return (
          svc.unit.toLowerCase().includes(q) ||
          svc.description.toLowerCase().includes(q) ||
          svc.subState.toLowerCase().includes(q)
        );
      })
      .sort((a, b) => {
        // Put failed first, then core daemons, then alphabetical
        const aFailed = a.activeState === "failed" ? 0 : 1;
        const bFailed = b.activeState === "failed" ? 0 : 1;
        if (aFailed !== bFailed) return aFailed - bFailed;
        const aCore = isCoreDaemon(a.unit) ? 0 : 1;
        const bCore = isCoreDaemon(b.unit) ? 0 : 1;
        if (aCore !== bCore) return aCore - bCore;
        return a.unit.localeCompare(b.unit);
      });
  }, [snapshot?.services, serviceFilter, serviceSearch]);

  const filteredLogLines = useMemo(() => {
    if (!logContent) return "";
    const q = logSearch.trim().toLowerCase();
    if (!q) return logContent;
    const matched = logContent
      .split("\n")
      .filter((line) => line.toLowerCase().includes(q));
    return matched.length > 0
      ? matched.join("\n")
      : isVi
        ? `(Không có dòng log nào khớp với "${logSearch}")`
        : `(No journal lines match "${logSearch}")`;
  }, [logContent, logSearch, isVi]);

  const renderServiceStatusBadge = (svc: SystemdServiceItem) => {
    if (svc.activeState === "failed" || svc.subState === "failed") {
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-rose-500/15 border border-rose-500/35 text-[10px] font-mono font-bold text-rose-300">
          <span className="w-1.5 h-1.5 rounded-full bg-rose-400 animate-pulse" />
          <span>FAILED</span>
        </span>
      );
    }
    if (svc.activeState === "active" && svc.subState === "running") {
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-emerald-500/15 border border-emerald-500/35 text-[10px] font-mono font-bold text-emerald-300">
          <span className="w-1.5 h-1.5 rounded-full bg-emerald-400" />
          <span>RUNNING</span>
        </span>
      );
    }
    if (svc.activeState === "active" && svc.subState === "exited") {
      return (
        <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-cyan-500/15 border border-cyan-500/35 text-[10px] font-mono font-bold text-cyan-300">
          <span className="w-1.5 h-1.5 rounded-full bg-cyan-400" />
          <span>EXITED</span>
        </span>
      );
    }
    return (
      <span className="inline-flex items-center gap-1.5 px-2.5 py-0.5 rounded-full bg-secondary border border-border/70 text-[10px] font-mono font-semibold text-muted-foreground">
        <span className="w-1.5 h-1.5 rounded-full bg-slate-400" />
        <span>{svc.subState.toUpperCase() || "INACTIVE"}</span>
      </span>
    );
  };

  if (loading && !snapshot) {
    return (
      <div className="rounded-2xl border border-border/80 bg-card/70 p-10 flex flex-col items-center justify-center gap-3 text-center">
        <RefreshCw className="w-6 h-6 text-amber-400 animate-spin" />
        <div className="text-sm font-bold text-foreground">
          {isVi
            ? "Đang thu thập trạng thái Swap, Systemd Daemons & Crontab..."
            : "Inspecting Swap Allocation, Systemd Services & Crontab..."}
        </div>
        <p className="text-xs text-muted-foreground max-w-md font-mono">
          free -m · /proc/swaps · timedatectl · systemctl · crontab -l
        </p>
      </div>
    );
  }

  const ramPct =
    snapshot && snapshot.ramTotalMb > 0
      ? Math.min(100, Math.round((snapshot.ramUsedMb / snapshot.ramTotalMb) * 100))
      : 0;
  const swapPct =
    snapshot && snapshot.swapTotalMb > 0
      ? Math.min(100, Math.round((snapshot.swapUsedMb / snapshot.swapTotalMb) * 100))
      : 0;
  const diskUsedMb =
    snapshot && snapshot.diskTotalMb > snapshot.diskFreeMb
      ? snapshot.diskTotalMb - snapshot.diskFreeMb
      : 0;
  const diskPct =
    snapshot && snapshot.diskTotalMb > 0
      ? Math.min(100, Math.round((diskUsedMb / snapshot.diskTotalMb) * 100))
      : 0;

  return (
    <div className="space-y-5 max-w-[1600px] mx-auto pb-8">
      {/* Error / Toast Notification Banners */}
      {errorMsg && (
        <div className="ui-banner-slide rounded-2xl border border-rose-500/45 bg-rose-500/12 px-4 py-3 flex items-start justify-between gap-3">
          <div className="flex items-start gap-2.5 text-xs text-rose-200">
            <AlertTriangle className="w-4 h-4 text-rose-400 shrink-0 mt-0.5" />
            <div className="space-y-0.5">
              <div className="font-bold">
                {isVi ? "Lỗi thực thi tác vụ hệ thống" : "System Operation Failed"}
              </div>
              <div className="font-mono text-[11px] opacity-90 break-all">
                {errorMsg}
              </div>
            </div>
          </div>
          <button
            type="button"
            onClick={() => setErrorMsg(null)}
            className="p-1 rounded-lg text-rose-300 hover:bg-rose-500/20 cursor-pointer"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}

      {toastMsg && (
        <div className="ui-banner-slide rounded-2xl border border-emerald-500/45 bg-emerald-500/12 px-4 py-3 flex items-center justify-between gap-3">
          <div className="flex items-center gap-2.5 text-xs font-semibold text-emerald-200">
            <CheckCircle2 className="w-4 h-4 text-emerald-400 shrink-0" />
            <span>{toastMsg}</span>
          </div>
          <button
            type="button"
            onClick={() => setToastMsg(null)}
            className="p-1 rounded-lg text-emerald-300 hover:bg-emerald-500/20 cursor-pointer"
          >
            <X className="w-3.5 h-3.5" />
          </button>
        </div>
      )}

      {/* =====================================================================
          SECTION 1: TOP BENTO DECK — SWAP ELASTICITY ALLOCATOR & HOST IDENTITY
          ===================================================================== */}
      {snapshot && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-stretch">
          {/* CARD A: Visual Swap & Memory Elasticity Allocator (6 cols) */}
          <section className="xl:col-span-6 rounded-2xl border border-border/80 bg-card/75 p-5 flex flex-col justify-between space-y-4 shadow-xs">
            <div className="space-y-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2.5">
                  <div className="w-9 h-9 rounded-xl bg-amber-500/15 border border-amber-500/35 flex items-center justify-center text-amber-400">
                    <HardDrive className="w-4.5 h-4.5" />
                  </div>
                  <div>
                    <h2 className="text-sm font-bold text-foreground">
                      {isVi
                        ? "Bộ Cấp Phát Swap Động & Đàn Hồi Bộ Nhớ"
                        : "Dynamic Swap & Memory Elasticity Allocator"}
                    </h2>
                    <p className="text-[11px] text-muted-foreground font-mono">
                      fallocate /swapfile · mkswap · swapon · /etc/fstab
                    </p>
                  </div>
                </div>

                <span
                  className={`px-2.5 py-1 rounded-xl border text-[11px] font-mono font-bold inline-flex items-center gap-1.5 ${
                    snapshot.swapTotalMb > 0
                      ? "bg-emerald-500/15 border-emerald-500/35 text-emerald-300"
                      : "bg-amber-500/15 border-amber-500/35 text-amber-300"
                  }`}
                >
                  <Database className="w-3.5 h-3.5" />
                  <span>
                    {snapshot.swapTotalMb > 0
                      ? `SWAP ACTIVE: ${formatMbHuman(snapshot.swapTotalMb)}`
                      : isVi
                        ? "CHƯA CÓ SWAP (0 MB)"
                        : "NO ACTIVE SWAP"}
                  </span>
                </span>
              </div>

              {/* Multi-Segment Capacity Bar: Physical RAM vs Active Swap vs Free Root Disk */}
              <div className="grid grid-cols-1 sm:grid-cols-3 gap-3">
                {/* Physical RAM */}
                <div className="rounded-xl border border-border/75 bg-background/55 p-3 space-y-2">
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-muted-foreground">
                      {isVi ? "RAM Vật lý" : "Physical RAM"}
                    </span>
                    <span className="font-mono font-bold text-cyan-300">
                      {ramPct}%
                    </span>
                  </div>
                  <div className="font-mono text-xs font-extrabold text-foreground">
                    {formatMbHuman(snapshot.ramUsedMb)} /{" "}
                    {formatMbHuman(snapshot.ramTotalMb)}
                  </div>
                  <div className="h-2 rounded-full bg-secondary overflow-hidden">
                    <div
                      className="h-full rounded-full bg-cyan-400 transition-all"
                      style={{ width: `${ramPct}%` }}
                    />
                  </div>
                </div>

                {/* Active Swap */}
                <div className="rounded-xl border border-border/75 bg-background/55 p-3 space-y-2">
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-muted-foreground">
                      {isVi ? "Swap Đang Bật" : "Active Swap"}
                    </span>
                    <span className="font-mono font-bold text-amber-300">
                      {snapshot.swapTotalMb > 0 ? `${swapPct}%` : "OFF"}
                    </span>
                  </div>
                  <div className="font-mono text-xs font-extrabold text-foreground">
                    {formatMbHuman(snapshot.swapUsedMb)} /{" "}
                    {formatMbHuman(snapshot.swapTotalMb)}
                  </div>
                  <div className="h-2 rounded-full bg-secondary overflow-hidden">
                    <div
                      className="h-full rounded-full bg-amber-400 transition-all"
                      style={{ width: `${swapPct}%` }}
                    />
                  </div>
                </div>

                {/* Available Root Disk */}
                <div className="rounded-xl border border-border/75 bg-background/55 p-3 space-y-2">
                  <div className="flex items-center justify-between text-[11px]">
                    <span className="font-semibold text-muted-foreground">
                      {isVi ? "Đĩa trống (/)" : "Root Disk Free"}
                    </span>
                    <span className="font-mono font-bold text-emerald-300">
                      {formatMbHuman(snapshot.diskFreeMb)}
                    </span>
                  </div>
                  <div className="font-mono text-xs font-extrabold text-foreground">
                    {isVi ? "Tổng:" : "Total:"} {formatMbHuman(snapshot.diskTotalMb)}
                  </div>
                  <div className="h-2 rounded-full bg-secondary overflow-hidden">
                    <div
                      className="h-full rounded-full bg-emerald-400 transition-all"
                      style={{ width: `${diskPct}%` }}
                    />
                  </div>
                </div>
              </div>

              {/* Active Swap Devices List */}
              <div className="rounded-xl border border-border/70 bg-background/45 px-3.5 py-2.5 space-y-1.5">
                <div className="text-[10px] font-bold uppercase tracking-wider text-muted-foreground">
                  {isVi
                    ? "Thiết bị Swap đang kích hoạt (/proc/swaps)"
                    : "Active Swap Devices (/proc/swaps)"}
                </div>
                {snapshot.swaps.length === 0 ? (
                  <div className="text-xs text-muted-foreground italic">
                    {isVi
                      ? "Chưa có phân vùng hoặc tệp /swapfile nào được kích hoạt. Máy chủ RAM thấp dễ bị OOM Killer dừng dịch vụ."
                      : "No active swapfile or swap partition detected. Low-RAM instances risk sudden OOM-killer crashes."}
                  </div>
                ) : (
                  <div className="flex flex-wrap gap-2">
                    {snapshot.swaps.map((sw) => (
                      <div
                        key={sw.filename}
                        className="inline-flex items-center gap-2 px-2.5 py-1 rounded-lg bg-secondary/90 border border-border/80 text-xs font-mono"
                      >
                        <span className="font-bold text-amber-300">
                          {sw.filename}
                        </span>
                        <span className="text-[10px] px-1.5 py-0.2 rounded bg-background/80 text-muted-foreground">
                          {sw.swapType}
                        </span>
                        <span className="text-foreground">
                          {formatMbHuman(sw.usedMb)} / {formatMbHuman(sw.sizeMb)}
                        </span>
                        <span className="text-[10px] text-muted-foreground">
                          prio:{sw.priority}
                        </span>
                      </div>
                    ))}
                  </div>
                )}
              </div>

              {/* 1-Click Swap Size Selector Pills */}
              <div className="space-y-2">
                <div className="text-xs font-bold text-foreground">
                  {isVi
                    ? "Chọn dung lượng /swapfile (Tự động cấp phát & lưu /etc/fstab):"
                    : "1-Click Swapfile Size Presets (Auto-allocates & persists in /etc/fstab):"}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {SWAP_SIZE_PILLS.map((pill) => {
                    const active = targetSwapMb === pill.sizeMb;
                    return (
                      <button
                        key={pill.sizeMb}
                        type="button"
                        onClick={() => setTargetSwapMb(pill.sizeMb)}
                        className={`px-3 py-1.5 rounded-xl text-xs font-mono font-semibold border transition-all cursor-pointer ${
                          active
                            ? "bg-amber-500/20 text-amber-300 border-amber-500/50 shadow-2xs"
                            : "bg-secondary/75 text-muted-foreground border-border/70 hover:text-foreground hover:bg-accent/50"
                        }`}
                      >
                        {isVi ? pill.labelVi : pill.labelEn}
                      </button>
                    );
                  })}
                </div>
              </div>
            </div>

            {/* Custom MB input + Allocate & Persist Button */}
            <div className="pt-2 flex flex-wrap items-center gap-2.5">
              <div className="flex items-center gap-2 bg-background/60 border border-border/80 rounded-xl px-3 py-1.5">
                <span className="text-[11px] font-semibold text-muted-foreground">
                  {isVi ? "Dung lượng (MB):" : "Custom (MB):"}
                </span>
                <input
                  type="number"
                  min={0}
                  max={32768}
                  step={256}
                  value={targetSwapMb}
                  onChange={(e) =>
                    setTargetSwapMb(Math.max(0, Number(e.target.value) || 0))
                  }
                  aria-label="Swap size in MB"
                  className="w-20 bg-transparent text-xs font-mono font-bold text-foreground outline-none"
                />
              </div>

              <button
                type="button"
                disabled={busyAction !== null}
                onClick={handleConfigureSwap}
                className="flex-1 min-w-[220px] py-2 px-4 rounded-xl bg-primary text-primary-foreground text-xs font-bold inline-flex items-center justify-center gap-2 hover:opacity-95 disabled:opacity-50 cursor-pointer transition-opacity shadow-xs"
              >
                {busyAction === "swap" ? (
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Zap className="w-3.5 h-3.5" />
                )}
                <span>
                  {targetSwapMb === 0
                    ? isVi
                      ? "Tắt & Xóa /swapfile (/etc/fstab)"
                      : "Disable & Remove /swapfile (/etc/fstab)"
                    : isVi
                      ? `Cấp phát & Lưu Swap (${formatMbHuman(targetSwapMb)}) vào /etc/fstab`
                      : `Allocate & Persist Swap (${formatMbHuman(targetSwapMb)}) in /etc/fstab`}
                </span>
              </button>
            </div>
          </section>

          {/* CARD B: Host Identity, Timezone & DNS Resolver Deck (6 cols) */}
          <form
            onSubmit={handleSaveHostIdentity}
            className="xl:col-span-6 rounded-2xl border border-border/80 bg-card/75 p-5 flex flex-col justify-between space-y-4 shadow-xs"
          >
            <div className="space-y-4">
              <div className="flex flex-wrap items-center justify-between gap-2">
                <div className="flex items-center gap-2.5">
                  <div className="w-9 h-9 rounded-xl bg-cyan-500/15 border border-cyan-500/35 flex items-center justify-center text-cyan-300">
                    <Globe className="w-4.5 h-4.5" />
                  </div>
                  <div>
                    <h2 className="text-sm font-bold text-foreground">
                      {isVi
                        ? "Định Danh Máy Chủ, Múi Giờ & Máy Chủ DNS"
                        : "Host Identity, Timezone & DNS Resolver Deck"}
                    </h2>
                    <p className="text-[11px] text-muted-foreground font-mono">
                      hostnamectl · timedatectl · /etc/resolv.conf
                    </p>
                  </div>
                </div>

                <div className="flex flex-wrap items-center gap-1.5">
                  <span
                    className={`px-2.5 py-1 rounded-xl border text-[11px] font-mono font-bold inline-flex items-center gap-1.5 ${
                      snapshot.ntpSynchronized
                        ? "bg-emerald-500/15 border-emerald-500/35 text-emerald-300"
                        : "bg-amber-500/15 border-amber-500/35 text-amber-300"
                    }`}
                  >
                    <Clock className="w-3.5 h-3.5" />
                    <span>
                      {snapshot.ntpSynchronized ? "NTP SYNCED" : "NTP UNSYNCED"}
                    </span>
                  </span>

                  <span
                    className={`px-2.5 py-1 rounded-xl border text-[11px] font-mono font-semibold inline-flex items-center gap-1.5 ${
                      snapshot.unattendedUpgradesEnabled
                        ? "bg-cyan-500/15 border-cyan-500/35 text-cyan-300"
                        : "bg-secondary border-border/70 text-muted-foreground"
                    }`}
                  >
                    <ShieldCheck className="w-3.5 h-3.5" />
                    <span>
                      {snapshot.unattendedUpgradesEnabled
                        ? "Auto-Upgrades ON"
                        : "Auto-Upgrades OFF"}
                    </span>
                  </span>
                </div>
              </div>

              {/* Hostname + Timezone Row */}
              <div className="grid grid-cols-1 sm:grid-cols-2 gap-3">
                <div className="space-y-1.5">
                  <label className="text-xs font-bold text-foreground block">
                    {isVi ? "Tên máy chủ (Hostname)" : "System Hostname"}
                  </label>
                  <input
                    type="text"
                    value={draftHostname}
                    onChange={(e) => setDraftHostname(e.target.value)}
                    placeholder="e.g. prod-edge-01"
                    className="ui-input !h-9 font-mono text-xs font-bold"
                  />
                </div>

                <div className="space-y-1.5">
                  <label className="text-xs font-bold text-foreground block">
                    {isVi ? "Múi giờ hệ thống (Timezone)" : "System Timezone"}
                  </label>
                  <input
                    type="text"
                    value={draftTimezone}
                    onChange={(e) => setDraftTimezone(e.target.value)}
                    placeholder="e.g. Asia/Ho_Chi_Minh"
                    className="ui-input !h-9 font-mono text-xs font-bold"
                  />
                </div>
              </div>

              {/* Quick Timezone Picker Pills */}
              <div className="space-y-1.5">
                <div className="text-[11px] font-semibold text-muted-foreground">
                  {isVi ? "Chọn nhanh Múi giờ:" : "Quick Timezone Picker:"}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {TIMEZONE_PRESETS.map((tz) => {
                    const active = draftTimezone === tz;
                    return (
                      <button
                        key={tz}
                        type="button"
                        onClick={() => setDraftTimezone(tz)}
                        className={`px-2.5 py-1 rounded-lg text-[11px] font-mono border transition-colors cursor-pointer ${
                          active
                            ? "bg-cyan-500/20 text-cyan-300 border-cyan-500/45 font-bold"
                            : "bg-secondary/70 text-muted-foreground border-border/70 hover:text-foreground"
                        }`}
                      >
                        {tz}
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* DNS Resolver Matrix */}
              <div className="rounded-xl border border-border/75 bg-background/50 p-3.5 space-y-3">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <span className="text-xs font-bold text-foreground">
                    {isVi
                      ? "Máy chủ phân giải DNS (/etc/resolv.conf)"
                      : "DNS Resolver Matrix (/etc/resolv.conf)"}
                  </span>
                  <div className="flex flex-wrap gap-1.5">
                    {DNS_PRESETS.map((preset) => {
                      const isCurrent =
                        preset.servers.every((ip) =>
                          draftDnsServers.includes(ip),
                        ) && draftDnsServers.length === preset.servers.length;
                      return (
                        <button
                          key={preset.id}
                          type="button"
                          onClick={() => setDraftDnsServers([...preset.servers])}
                          className={`px-2.5 py-1 rounded-lg text-[10px] font-mono font-semibold border cursor-pointer transition-colors ${
                            isCurrent
                              ? "bg-primary/20 text-primary border-primary/45 font-bold"
                              : "bg-secondary text-muted-foreground border-border/70 hover:text-foreground"
                          }`}
                        >
                          {preset.label}
                        </button>
                      );
                    })}
                  </div>
                </div>

                {/* Current Nameserver Chips + Add Custom */}
                <div className="flex flex-wrap items-center gap-2">
                  {draftDnsServers.map((ip) => (
                    <span
                      key={ip}
                      className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-lg bg-secondary border border-border/80 text-xs font-mono font-bold text-cyan-300"
                    >
                      <span>nameserver {ip}</span>
                      <button
                        type="button"
                        onClick={() => handleRemoveDns(ip)}
                        title="Remove DNS server"
                        className="text-muted-foreground hover:text-rose-400 cursor-pointer"
                      >
                        <X className="w-3 h-3" />
                      </button>
                    </span>
                  ))}

                  <div className="inline-flex items-center gap-1.5">
                    <input
                      type="text"
                      value={customDnsInput}
                      onChange={(e) => setCustomDnsInput(e.target.value)}
                      onKeyDown={(e) => {
                        if (e.key === "Enter") {
                          e.preventDefault();
                          handleAddCustomDns();
                        }
                      }}
                      placeholder={isVi ? "Thêm IP DNS..." : "Add DNS IP..."}
                      className="ui-input !w-32 !h-7 text-xs font-mono"
                    />
                    <button
                      type="button"
                      onClick={handleAddCustomDns}
                      className="px-2 py-1 rounded-lg bg-secondary hover:bg-accent border border-border/70 text-xs font-semibold text-foreground cursor-pointer"
                    >
                      <Plus className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>
              </div>
            </div>

            <div className="pt-2 flex items-center justify-end">
              <button
                type="submit"
                disabled={busyAction !== null || !draftHostname.trim()}
                className="w-full sm:w-auto py-2 px-5 rounded-xl bg-primary text-primary-foreground text-xs font-bold inline-flex items-center justify-center gap-2 hover:opacity-95 disabled:opacity-50 cursor-pointer transition-opacity shadow-xs"
              >
                {busyAction === "identity" ? (
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Sparkles className="w-3.5 h-3.5" />
                )}
                <span>
                  {isVi
                    ? "Lưu Định Danh, Múi Giờ & DNS"
                    : "Save Identity & DNS"}
                </span>
              </button>
            </div>
          </form>
        </div>
      )}

      {/* =====================================================================
          SPLIT WORKSPACE: SECTION 2 (LEFT SYSTEMD DECK + LOGS) & SECTION 3 (RIGHT CRON STUDIO)
          ===================================================================== */}
      {snapshot && (
        <div className="grid grid-cols-1 xl:grid-cols-12 gap-5 items-start">
          {/* SECTION 2: Left/Main Split — Systemd Service Deck + Live journalctl Log Drawer (7 cols) */}
          <div className="xl:col-span-7 space-y-4">
            <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs">
              <div className="flex flex-wrap items-center justify-between gap-3">
                <div className="flex items-center gap-2.5">
                  <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/35 flex items-center justify-center text-emerald-400">
                    <Server className="w-4 h-4" />
                  </div>
                  <div>
                    <h2
                      id={systemdTableCaptionId}
                      className="text-xs font-bold uppercase tracking-wider text-foreground"
                    >
                      {isVi
                        ? "Bảng Điều Khiển Dịch Vụ Systemd & Khởi Động Cùng Hệ Thống"
                        : "Systemd Service Deck & Boot Daemons"}
                    </h2>
                    <p className="text-[11px] text-muted-foreground">
                      {isVi
                        ? "Quản lý trạng thái chạy thực tế, bật/tắt tự khởi động cùng máy chủ và soi nhật ký journalctl."
                        : "Control daemon runtime state, toggle boot persistence, and inspect live journalctl streams."}
                    </p>
                  </div>
                </div>

                {/* Search input */}
                <div className="relative w-full sm:w-56">
                  <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
                  <input
                    type="text"
                    value={serviceSearch}
                    onChange={(e) => setServiceSearch(e.target.value)}
                    placeholder={
                      isVi ? "Tìm dịch vụ (nginx, ssh...)" : "Filter unit or description..."
                    }
                    aria-label="Filter systemd services"
                    className="ui-input !h-8 !pl-8 text-xs font-mono"
                  />
                </div>
              </div>

              {/* Filter Pills */}
              <div className="flex flex-wrap items-center gap-1.5">
                {(
                  [
                    {
                      id: "core",
                      label: isVi ? "Dịch vụ Cốt lõi (Pinned)" : "Core Daemons (Pinned)",
                      count: serviceCounts.core,
                    },
                    {
                      id: "active",
                      label: isVi ? "Đang chạy (Active)" : "Running (active)",
                      count: serviceCounts.active,
                    },
                    {
                      id: "failed",
                      label: isVi ? "Bị lỗi (Failed)" : "Failed (failed)",
                      count: serviceCounts.failed,
                    },
                    {
                      id: "all",
                      label: isVi ? "Tất cả dịch vụ" : "All Services",
                      count: serviceCounts.all,
                    },
                  ] as const
                ).map((pill) => {
                  const active = serviceFilter === pill.id;
                  return (
                    <button
                      key={pill.id}
                      type="button"
                      onClick={() => setServiceFilter(pill.id)}
                      className={`px-2.5 py-1 rounded-xl text-xs font-semibold border inline-flex items-center gap-1.5 transition-all cursor-pointer ${
                        active
                          ? "bg-primary text-primary-foreground border-primary shadow-2xs"
                          : "bg-secondary/70 text-muted-foreground border-border/70 hover:text-foreground"
                      }`}
                    >
                      <span>{pill.label}</span>
                      <span
                        className={`px-1.5 py-0.2 rounded-full text-[10px] font-mono ${
                          active
                            ? "bg-black/20 text-primary-foreground"
                            : "bg-background/70 text-foreground"
                        }`}
                      >
                        {pill.count}
                      </span>
                    </button>
                  );
                })}
              </div>

              {/* Scrollable Sticky Systemd Table */}
              <div
                role="region"
                tabIndex={0}
                aria-labelledby={systemdTableCaptionId}
                className="vps-table-wrapper rounded-xl border border-border/80 bg-background/50 max-h-[480px]"
              >
                <table className="w-full text-left text-xs">
                  <caption className="sr-only">
                    Systemd service units, runtime status, boot enable toggle, and service controls
                  </caption>
                  <thead className="bg-secondary/95 text-[11px] uppercase tracking-wider text-muted-foreground">
                    <tr>
                      <th className="py-2.5 px-3 font-semibold border-b border-border">
                        {isVi ? "Dịch vụ (Unit)" : "Service Unit"}
                      </th>
                      <th className="py-2.5 px-3 font-semibold border-b border-border">
                        {isVi ? "Trạng thái" : "Live Status"}
                      </th>
                      <th className="py-2.5 px-3 font-semibold border-b border-border text-center">
                        {isVi ? "Tự chạy khi Boot" : "Boot Enable"}
                      </th>
                      <th className="py-2.5 px-3 font-semibold border-b border-border text-right">
                        {isVi ? "Điều khiển 1-Click" : "1-Click Controls"}
                      </th>
                    </tr>
                  </thead>
                  <tbody className="divide-y divide-border/60">
                    {filteredServices.length === 0 ? (
                      <tr>
                        <td
                          colSpan={4}
                          className="py-8 px-4 text-center text-xs text-muted-foreground"
                        >
                          {isVi
                            ? "Không có dịch vụ Systemd nào khớp với bộ lọc hiện tại."
                            : "No systemd services match the current filter."}
                        </td>
                      </tr>
                    ) : (
                      filteredServices.map((svc) => {
                        const isInspecting = inspectedUnit === svc.unit;
                        const toggleBusy =
                          busyAction === `svc:${svc.unit}:enable` ||
                          busyAction === `svc:${svc.unit}:disable`;
                        return (
                          <tr
                            key={svc.unit}
                            className={`transition-colors ${
                              isInspecting
                                ? "bg-primary/10"
                                : "hover:bg-accent/30"
                            }`}
                          >
                            <th
                              scope="row"
                              className="py-2.5 px-3 font-normal text-left align-middle"
                            >
                              <div className="flex items-center gap-1.5">
                                <span className="font-mono font-bold text-foreground">
                                  {svc.unit}
                                </span>
                                {isCoreDaemon(svc.unit) && (
                                  <span className="px-1.5 py-0.2 rounded bg-primary/15 text-primary font-mono text-[9px] font-bold">
                                    CORE
                                  </span>
                                )}
                              </div>
                              <div className="text-[11px] text-muted-foreground line-clamp-1">
                                {svc.description}
                              </div>
                            </th>

                            <td className="py-2.5 px-3 align-middle whitespace-nowrap">
                              {renderServiceStatusBadge(svc)}
                            </td>

                            <td className="py-2.5 px-3 align-middle text-center">
                              <label className="inline-flex items-center justify-center cursor-pointer">
                                <span className="sr-only">
                                  Toggle boot enable for {svc.unit}
                                </span>
                                <input
                                  type="checkbox"
                                  {...({ switch: "" } as any)}
                                  checked={svc.enabled}
                                  disabled={busyAction !== null || toggleBusy}
                                  onChange={() =>
                                    handleSystemdAction(
                                      svc.unit,
                                      svc.enabled ? "disable" : "enable",
                                    )
                                  }
                                />
                              </label>
                            </td>

                            <td className="py-2.5 px-3 align-middle text-right whitespace-nowrap">
                              <div className="inline-flex items-center justify-end gap-1">
                                {svc.activeState !== "active" ? (
                                  <button
                                    type="button"
                                    disabled={busyAction !== null}
                                    onClick={() =>
                                      handleSystemdAction(svc.unit, "start")
                                    }
                                    title={`Start ${svc.unit}`}
                                    className="px-2 py-1 rounded-lg bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/35 text-emerald-300 text-[11px] font-mono font-semibold inline-flex items-center gap-1 cursor-pointer disabled:opacity-45"
                                  >
                                    <Play className="w-3 h-3" />
                                    <span>Start</span>
                                  </button>
                                ) : (
                                  <button
                                    type="button"
                                    disabled={busyAction !== null}
                                    onClick={() =>
                                      handleSystemdAction(svc.unit, "stop")
                                    }
                                    title={`Stop ${svc.unit}`}
                                    className="px-2 py-1 rounded-lg bg-rose-500/15 hover:bg-rose-500/25 border border-rose-500/35 text-rose-300 text-[11px] font-mono font-semibold inline-flex items-center gap-1 cursor-pointer disabled:opacity-45"
                                  >
                                    <Square className="w-3 h-3" />
                                    <span>Stop</span>
                                  </button>
                                )}

                                <button
                                  type="button"
                                  disabled={busyAction !== null}
                                  onClick={() =>
                                    handleSystemdAction(svc.unit, "restart")
                                  }
                                  title={`Restart ${svc.unit}`}
                                  className="px-2 py-1 rounded-lg bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/35 text-amber-300 text-[11px] font-mono font-semibold inline-flex items-center gap-1 cursor-pointer disabled:opacity-45"
                                >
                                  <RotateCw className="w-3 h-3" />
                                  <span>Restart</span>
                                </button>

                                <button
                                  type="button"
                                  onClick={() => handleOpenLogs(svc.unit)}
                                  className={`px-2 py-1 rounded-lg border text-[11px] font-mono font-semibold inline-flex items-center gap-1 cursor-pointer transition-colors ${
                                    isInspecting
                                      ? "bg-primary text-primary-foreground border-primary"
                                      : "bg-secondary hover:bg-accent text-foreground border-border/70"
                                  }`}
                                >
                                  <FileText className="w-3 h-3" />
                                  <span>Logs</span>
                                </button>
                              </div>
                            </td>
                          </tr>
                        );
                      })
                    )}
                  </tbody>
                </table>
              </div>
            </section>

            {/* Inline Live journalctl Inspector Panel */}
            {inspectedUnit && (
              <section className="ui-fade-slide rounded-2xl border border-primary/40 bg-card/90 p-5 space-y-3 shadow-lg">
                <div className="flex flex-wrap items-center justify-between gap-2">
                  <div className="flex items-center gap-2">
                    <div className="w-7 h-7 rounded-lg bg-primary/15 border border-primary/35 flex items-center justify-center text-primary">
                      <Terminal className="w-3.5 h-3.5" />
                    </div>
                    <div>
                      <div className="text-xs font-bold font-mono text-foreground">
                        journalctl -u {inspectedUnit}
                      </div>
                      <div className="text-[10px] text-muted-foreground">
                        {isVi
                          ? "Nhật ký thời gian thực định dạng ISO-8601 (--no-pager -o short-iso)"
                          : "Live systemd unit journal stream (--no-pager -o short-iso)"}
                      </div>
                    </div>
                  </div>

                  <div className="flex flex-wrap items-center gap-2">
                    {/* Line count selector (50, 150, 300) */}
                    <div className="inline-flex items-center rounded-xl bg-secondary p-0.5 border border-border/70">
                      {[50, 150, 300].map((cnt) => (
                        <button
                          key={cnt}
                          type="button"
                          onClick={() => {
                            setLogLinesCount(cnt);
                            fetchServiceLogs(inspectedUnit, cnt);
                          }}
                          className={`px-2 py-1 rounded-lg text-[10px] font-mono font-bold cursor-pointer ${
                            logLinesCount === cnt
                              ? "bg-primary text-primary-foreground"
                              : "text-muted-foreground hover:text-foreground"
                          }`}
                        >
                          {cnt} lines
                        </button>
                      ))}
                    </div>

                    <button
                      type="button"
                      onClick={() => fetchServiceLogs(inspectedUnit, logLinesCount)}
                      className="p-1.5 rounded-xl bg-secondary hover:bg-accent border border-border/70 text-foreground cursor-pointer"
                      title="Refresh journalctl logs"
                    >
                      <RefreshCw
                        className={`w-3.5 h-3.5 ${loadingLogs ? "animate-spin text-primary" : ""}`}
                      />
                    </button>

                    <button
                      type="button"
                      onClick={() => setInspectedUnit(null)}
                      className="p-1.5 rounded-xl bg-secondary hover:bg-accent border border-border/70 text-muted-foreground hover:text-foreground cursor-pointer"
                      title="Close log inspector"
                    >
                      <X className="w-3.5 h-3.5" />
                    </button>
                  </div>
                </div>

                {/* Search filter inside logs */}
                <div className="relative">
                  <Search className="w-3.5 h-3.5 text-muted-foreground absolute left-3 top-1/2 -translate-y-1/2 pointer-events-none" />
                  <input
                    type="text"
                    value={logSearch}
                    onChange={(e) => setLogSearch(e.target.value)}
                    placeholder={
                      isVi
                        ? "Lọc dòng nhật ký (error, warn, pid...)"
                        : "Filter journal lines (error, warn, pid...)"
                    }
                    className="ui-input !h-8 !pl-8 text-xs font-mono"
                  />
                </div>

                <pre className="rounded-xl border border-border/80 bg-background/90 p-3.5 text-[11px] font-mono text-foreground overflow-x-auto max-h-72 leading-relaxed whitespace-pre-wrap break-all">
                  {loadingLogs
                    ? isVi
                      ? "Đang tải nhật ký từ journalctl..."
                      : "Fetching unit logs from journalctl..."
                    : filteredLogLines}
                </pre>
              </section>
            )}
          </div>

          {/* SECTION 3: Right Split — Human-Readable Cron Studio (5 cols) */}
          <div className="xl:col-span-5 space-y-4">
            {/* Visual Cron Composer Card */}
            <form
              onSubmit={handleAddCronJob}
              className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-4 shadow-xs"
            >
              <div className="flex items-center gap-2.5">
                <div className="w-8 h-8 rounded-xl bg-violet-500/15 border border-violet-500/35 flex items-center justify-center text-violet-300">
                  <CalendarClock className="w-4 h-4" />
                </div>
                <div>
                  <h2 className="text-xs font-bold uppercase tracking-wider text-foreground">
                    {isVi
                      ? "Trình Soạn Thảo Cron Trực Quan & Dịch Ngôn Ngữ Tự Nhiên"
                      : "Visual Cron Composer & Natural Language Studio"}
                  </h2>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Tạo lịch tự động hóa với bản dịch tiếng Việt/Anh tức thời."
                      : "Compose scheduled jobs with real-time human-readable translation."}
                  </p>
                </div>
              </div>

              {/* Quick Schedule Presets */}
              <div className="space-y-1.5">
                <div className="text-[11px] font-semibold text-muted-foreground">
                  {isVi ? "Mẫu lịch chạy nhanh:" : "Quick Schedule Presets:"}
                </div>
                <div className="flex flex-wrap gap-1.5">
                  {CRON_PRESETS.map((preset) => {
                    const active = composedSchedule === preset.schedule;
                    return (
                      <button
                        key={preset.id}
                        type="button"
                        onClick={() => {
                          if (!preset.fields) {
                            setIsRebootSchedule(true);
                          } else {
                            setIsRebootSchedule(false);
                            setCronMin(preset.fields[0]);
                            setCronHour(preset.fields[1]);
                            setCronDom(preset.fields[2]);
                            setCronMon(preset.fields[3]);
                            setCronDow(preset.fields[4]);
                          }
                        }}
                        className={`px-2.5 py-1 rounded-xl text-[11px] font-mono font-semibold border transition-colors cursor-pointer ${
                          active
                            ? "bg-violet-500/20 text-violet-200 border-violet-500/45 font-bold"
                            : "bg-secondary/75 text-muted-foreground border-border/70 hover:text-foreground"
                        }`}
                      >
                        {isVi ? preset.labelVi : preset.labelEn}
                      </button>
                    );
                  })}
                </div>
              </div>

              {/* 5-Field Cron Editor */}
              {!isRebootSchedule && (
                <div className="grid grid-cols-5 gap-2">
                  {[
                    {
                      label: isVi ? "Phút (Min)" : "Minute",
                      val: cronMin,
                      set: setCronMin,
                    },
                    {
                      label: isVi ? "Giờ (Hour)" : "Hour",
                      val: cronHour,
                      set: setCronHour,
                    },
                    {
                      label: isVi ? "Ngày (Day)" : "Day (DOM)",
                      val: cronDom,
                      set: setCronDom,
                    },
                    {
                      label: isVi ? "Tháng (Mon)" : "Month",
                      val: cronMon,
                      set: setCronMon,
                    },
                    {
                      label: isVi ? "Thứ (DOW)" : "Weekday",
                      val: cronDow,
                      set: setCronDow,
                    },
                  ].map((field) => (
                    <div key={field.label} className="space-y-1">
                      <label className="text-[10px] font-mono font-semibold text-muted-foreground block text-center">
                        {field.label}
                      </label>
                      <input
                        type="text"
                        value={field.val}
                        onChange={(e) => field.set(e.target.value)}
                        className="ui-input !h-8 text-center font-mono text-xs font-bold"
                      />
                    </div>
                  ))}
                </div>
              )}

              {/* Real-Time Natural Language Schedule Translator Banner */}
              <div className="rounded-xl border border-violet-500/35 bg-violet-500/10 px-3.5 py-2.5 flex items-center justify-between gap-2">
                <div className="flex items-center gap-2 min-w-0">
                  <Activity className="w-4 h-4 text-violet-300 shrink-0" />
                  <div className="text-xs font-semibold text-violet-100 truncate">
                    {naturalSchedulePreview}
                  </div>
                </div>
                <span className="px-2 py-0.5 rounded-lg bg-background/80 border border-border/70 font-mono text-[11px] font-bold text-violet-300 shrink-0">
                  {composedSchedule}
                </span>
              </div>

              {/* Command + Comment Inputs */}
              <div className="space-y-2.5">
                <div className="space-y-1">
                  <div className="flex items-center justify-between gap-2">
                    <label className="text-xs font-bold text-foreground">
                      {isVi ? "Lệnh thực thi (Command)" : "Cron Command"}
                    </label>
                    <button
                      type="button"
                      onClick={() => {
                        const suffix = ">> /var/log/cron.log 2>&1";
                        if (!cronCommand.includes(suffix)) {
                          setCronCommand((prev) =>
                            prev.trim() ? `${prev.trim()} ${suffix}` : suffix,
                          );
                        }
                      }}
                      className="px-2 py-0.5 rounded-lg bg-secondary hover:bg-accent border border-border/70 text-[10px] font-mono text-cyan-300 cursor-pointer"
                    >
                      + Append &gt;&gt; /var/log/cron.log 2&gt;&amp;1
                    </button>
                  </div>
                  <input
                    type="text"
                    value={cronCommand}
                    onChange={(e) => setCronCommand(e.target.value)}
                    placeholder="/usr/local/bin/backup.sh >> /var/log/cron.log 2>&1"
                    className="ui-input !h-9 font-mono text-xs"
                  />
                </div>

                <div className="space-y-1">
                  <label className="text-xs font-bold text-foreground block">
                    {isVi
                      ? "Ghi chú / Nhãn tác vụ (Comment)"
                      : "Job Label / Comment"}
                  </label>
                  <input
                    type="text"
                    value={cronComment}
                    onChange={(e) => setCronComment(e.target.value)}
                    placeholder={
                      isVi
                        ? "VD: Sao lưu PostgreSQL hàng đêm"
                        : "e.g. Nightly PostgreSQL Backup"
                    }
                    className="ui-input !h-8 text-xs"
                  />
                </div>
              </div>

              <button
                type="submit"
                disabled={busyAction !== null || !cronCommand.trim()}
                className="w-full py-2 px-4 rounded-xl bg-primary text-primary-foreground text-xs font-bold inline-flex items-center justify-center gap-2 hover:opacity-95 disabled:opacity-50 cursor-pointer transition-opacity shadow-xs"
              >
                {busyAction === "cron:add" ? (
                  <RefreshCw className="w-3.5 h-3.5 animate-spin" />
                ) : (
                  <Plus className="w-3.5 h-3.5" />
                )}
                <span>
                  {isVi ? "Thêm Tác Vụ Cron" : "Add Cron Job"}
                </span>
              </button>
            </form>

            {/* Active Crontab Deck */}
            <section className="rounded-2xl border border-border/80 bg-card/75 p-5 space-y-3 shadow-xs">
              <div className="flex items-center justify-between">
                <div className="flex items-center gap-2">
                  <Wrench className="w-4 h-4 text-amber-400" />
                  <h3 className="text-xs font-bold uppercase tracking-wider text-foreground">
                    {isVi
                      ? `Danh Sách Crontab Đang Cài Đặt (${snapshot.cronJobs.length})`
                      : `Active Crontab Deck (${snapshot.cronJobs.length})`}
                  </h3>
                </div>
                <span className="text-[10px] font-mono text-muted-foreground">
                  crontab -l
                </span>
              </div>

              {snapshot.cronJobs.length === 0 ? (
                <div className="rounded-xl border border-border/70 bg-background/45 p-6 text-center space-y-1">
                  <div className="text-xs font-semibold text-foreground">
                    {isVi
                      ? "Chưa có tác vụ tự động nào trong Crontab"
                      : "No Scheduled Cron Jobs Configured"}
                  </div>
                  <p className="text-[11px] text-muted-foreground">
                    {isVi
                      ? "Sử dụng trình soạn thảo phía trên để thêm lịch sao lưu, dọn dẹp Docker hoặc gia hạn SSL."
                      : "Use the Visual Cron Composer above to schedule backups, pruning, or healthchecks."}
                  </p>
                </div>
              ) : (
                <div className="space-y-2.5 max-h-[420px] overflow-y-auto pr-1">
                  {snapshot.cronJobs.map((job) => (
                    <div
                      key={job.index}
                      className={`rounded-xl border p-3.5 space-y-2 transition-colors ${
                        job.paused
                          ? "border-amber-500/35 bg-amber-500/5 opacity-80"
                          : "border-border/80 bg-background/55"
                      }`}
                    >
                      <div className="flex items-start justify-between gap-2">
                        <div className="space-y-1 min-w-0">
                          <div className="flex flex-wrap items-center gap-1.5">
                            <span className="text-xs font-bold text-foreground">
                              {job.comment ||
                                (isVi
                                  ? `Tác vụ Cron #${job.index}`
                                  : `Cron Job #${job.index}`)}
                            </span>
                            <span className="px-2 py-0.5 rounded-md bg-secondary border border-border/70 font-mono text-[10px] font-bold text-violet-300">
                              {job.schedule}
                            </span>
                            {job.paused && (
                              <span className="inline-flex items-center gap-1 px-2 py-0.5 rounded-md bg-amber-500/15 border border-amber-500/35 text-[10px] font-mono font-bold text-amber-300">
                                <PauseCircle className="w-3 h-3" />
                                <span>PAUSED</span>
                              </span>
                            )}
                          </div>
                          <div className="text-[11px] text-emerald-300 font-medium">
                            {translateCronSchedule(job.schedule, isVi)}
                          </div>
                        </div>

                        <div className="flex items-center gap-2 shrink-0">
                          <label
                            className="inline-flex items-center gap-1.5 text-[10px] font-mono text-muted-foreground cursor-pointer"
                            title={
                              job.paused
                                ? "Resume cron job"
                                : "Pause cron job (# [SSHBOX_PAUSED])"
                            }
                          >
                            <span>{job.paused ? "Paused" : "Active"}</span>
                            <input
                              type="checkbox"
                              {...({ switch: "" } as any)}
                              checked={!job.paused}
                              disabled={busyAction !== null}
                              onChange={() => handleTogglePauseCronJob(job.index)}
                            />
                          </label>

                          <button
                            type="button"
                            disabled={busyAction !== null}
                            onClick={() => handleDeleteCronJob(job.index)}
                            title="Delete cron job"
                            className="p-1.5 rounded-lg bg-rose-500/12 hover:bg-rose-500/25 border border-rose-500/30 text-rose-300 cursor-pointer disabled:opacity-45"
                          >
                            <Trash2 className="w-3.5 h-3.5" />
                          </button>
                        </div>
                      </div>

                      <div className="rounded-lg bg-secondary/80 border border-border/60 px-2.5 py-1.5 font-mono text-[11px] text-foreground break-all">
                        {job.command}
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </section>
          </div>
        </div>
      )}
    </div>
  );
};
