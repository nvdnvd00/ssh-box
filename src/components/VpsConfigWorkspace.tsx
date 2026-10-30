import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  CheckCircle2,
  Flame,
  KeyRound,
  Lock,
  LockOpen,
  RefreshCw,
  Server,
  Shield,
  ShieldCheck,
  Sliders,
  Terminal as TerminalIcon,
  Users,
  X,
} from "lucide-react";
import {
  HostEntry,
  ResourcesStoreData,
  VpsPrivilegeInfo,
  VpsStudioTab,
} from "../types";
import { AppSettingsConfig } from "../themes";
import { SelectField } from "./SelectField";
import { SshHardeningStudio } from "./vps/SshHardeningStudio";
import { FirewallStudio } from "./vps/FirewallStudio";
import { AccessKeysStudio } from "./vps/AccessKeysStudio";

interface VpsConfigWorkspaceProps {
  hosts: HostEntry[];
  resources: ResourcesStoreData;
  activeHostId: string;
  appSettings?: AppSettingsConfig;
  onSelectHost?: (hostId: string) => void;
  onSaveHost: (host: HostEntry, secret?: string) => Promise<void>;
  onConnectHostTerminal?: (host: HostEntry) => void;
}

export const VpsConfigWorkspace: React.FC<VpsConfigWorkspaceProps> = ({
  hosts,
  resources,
  activeHostId,
  appSettings,
  onSelectHost,
  onSaveHost,
  onConnectHostTerminal,
}) => {
  const lang = appSettings?.language ?? "vi";
  const isVi = lang === "vi";

  const [selectedHostId, setSelectedHostId] = useState<string>(() => {
    if (activeHostId && activeHostId !== "local" && hosts.some((h) => h.id === activeHostId)) {
      return activeHostId;
    }
    return hosts[0]?.id ?? "";
  });

  useEffect(() => {
    if (
      activeHostId &&
      activeHostId !== "local" &&
      hosts.some((h) => h.id === activeHostId)
    ) {
      setSelectedHostId(activeHostId);
    } else if (!selectedHostId && hosts.length > 0) {
      setSelectedHostId(hosts[0].id);
    }
  }, [activeHostId, hosts, selectedHostId]);

  const [activeStudio, setActiveStudio] = useState<VpsStudioTab>("ssh");
  const [privilegeMap, setPrivilegeMap] = useState<
    Record<string, VpsPrivilegeInfo>
  >({});
  const [sudoPasswordMap, setSudoPasswordMap] = useState<
    Record<string, string>
  >({});
  const [probingPrivilege, setProbingPrivilege] = useState(false);
  const [sudoPopoverOpen, setSudoPopoverOpen] = useState(false);
  const [sudoInput, setSudoInput] = useState("");
  const [sudoUnlockError, setSudoUnlockError] = useState<string | null>(null);
  const [refreshNonce, setRefreshNonce] = useState(0);

  const selectedHost = useMemo(
    () => hosts.find((h) => h.id === selectedHostId) ?? hosts[0] ?? null,
    [hosts, selectedHostId],
  );

  const currentSudoPassword = selectedHost
    ? sudoPasswordMap[selectedHost.id]
    : undefined;
  const currentPrivilege = selectedHost
    ? privilegeMap[selectedHost.id] ?? null
    : null;

  const probeHostPrivilege = useCallback(
    async (hostId: string, candidatePassword?: string) => {
      if (!hostId) return null;
      setProbingPrivilege(true);
      try {
        const info = await invoke<VpsPrivilegeInfo>("vps_probe_privilege", {
          hostId,
          sudoPassword: candidatePassword ?? sudoPasswordMap[hostId] ?? null,
        });
        setPrivilegeMap((prev) => ({ ...prev, [hostId]: info }));
        return info;
      } catch {
        return null;
      } finally {
        setProbingPrivilege(false);
      }
    },
    [sudoPasswordMap],
  );

  useEffect(() => {
    if (selectedHost) {
      probeHostPrivilege(selectedHost.id);
    }
  }, [selectedHost, probeHostPrivilege]);

  const handleUnlockSudo = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!selectedHost) return;
    const trimmed = sudoInput.trim();
    if (!trimmed) return;
    setSudoUnlockError(null);
    const info = await probeHostPrivilege(selectedHost.id, trimmed);
    if (info && info.sudoUnlocked) {
      setSudoPasswordMap((prev) => ({ ...prev, [selectedHost.id]: trimmed }));
      setSudoPopoverOpen(false);
      setSudoInput("");
      setRefreshNonce((n) => n + 1);
    } else {
      setSudoUnlockError(
        isVi
          ? "Mật khẩu sudo không chính xác hoặc user không nằm trong nhóm sudoers."
          : "Sudo password rejected by server or user is not in sudoers.",
      );
    }
  };

  const hostOptions = useMemo(
    () =>
      hosts.map((h) => ({
        value: h.id,
        label: `${h.alias || h.hostname} (${h.user}@${h.hostname}:${h.port})`,
      })),
    [hosts],
  );

  const studioTabs: {
    id: VpsStudioTab;
    label: string;
    subtitle: string;
    icon: React.ReactNode;
  }[] = [
    {
      id: "ssh",
      label: isVi ? "Bảo mật SSH" : "SSH Hardening",
      subtitle: "sshd_config & Port",
      icon: <Shield className="w-4 h-4 text-emerald-400" />,
    },
    {
      id: "firewall",
      label: isVi ? "Tường lửa & Cổng" : "Firewall & Sockets",
      subtitle: "UFW & Listening Ports",
      icon: <Flame className="w-4 h-4 text-orange-400" />,
    },
    {
      id: "access",
      label: isVi ? "Users & Khóa SSH" : "Users & SSH Keys",
      subtitle: "Accounts & Vault Keys",
      icon: <Users className="w-4 h-4 text-cyan-400" />,
    },
  ];

  if (hosts.length === 0 || !selectedHost) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8 text-center space-y-3">
        <div className="w-12 h-12 rounded-2xl bg-primary/15 border border-primary/30 flex items-center justify-center text-primary">
          <Shield className="w-6 h-6" />
        </div>
        <h2 className="text-base font-bold text-foreground">
          {isVi
            ? "Chưa có máy chủ VPS nào trong danh bạ"
            : "No Remote VPS Hosts Configured"}
        </h2>
        <p className="text-xs text-muted-foreground max-w-md">
          {isVi
            ? "Hãy thêm ít nhất một máy chủ SSH trong mục Máy chủ (Hosts) để sử dụng bộ công cụ Gia cố Bảo mật & Cấu hình VPS."
            : "Add an SSH machine in the Machines tab to use the VPS Hardening & Configuration Center."}
        </p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full w-full bg-background text-foreground overflow-hidden select-none ui-fade-slide">
      {/* Top Server Center Command Bar */}
      <div className="px-6 py-3.5 border-b border-border/80 bg-header/60 backdrop-blur-md flex flex-wrap items-center justify-between gap-3 shrink-0 z-20">
        <div className="flex flex-wrap items-center gap-3 min-w-0">
          <div className="flex items-center gap-2">
            <div className="w-8 h-8 rounded-xl bg-emerald-500/15 border border-emerald-500/30 flex items-center justify-center text-emerald-400 shrink-0">
              <Sliders className="w-4 h-4" />
            </div>
            <div className="w-64 sm:w-72">
              <SelectField
                size="sm"
                value={selectedHost.id}
                options={hostOptions}
                onChange={(nextId) => {
                  setSelectedHostId(nextId);
                  onSelectHost?.(nextId);
                }}
              />
            </div>
          </div>

          {/* Host IP/Port Pill */}
          <span className="px-2.5 py-1 rounded-lg bg-secondary/90 border border-border/70 font-mono text-[11px] text-foreground">
            {selectedHost.user}@{selectedHost.hostname}:{selectedHost.port}
          </span>

          {/* OS Badge */}
          {currentPrivilege?.osPrettyName && (
            <span className="px-2.5 py-1 rounded-lg bg-card border border-border/70 text-[11px] font-medium text-muted-foreground inline-flex items-center gap-1.5">
              <Server className="w-3 h-3 text-primary" />
              <span>{currentPrivilege.osPrettyName}</span>
            </span>
          )}
        </div>

        {/* Right Controls: Privilege Status Pill + Refresh + Open Terminal */}
        <div className="flex items-center gap-2 relative">
          {/* Interactive Privilege Status Pill */}
          <div className="relative">
            {currentPrivilege?.isRoot ? (
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-emerald-500/15 border border-emerald-500/35 text-[11px] font-mono font-bold text-emerald-400">
                <ShieldCheck className="w-3.5 h-3.5" />
                <span>ROOT (UID 0)</span>
              </span>
            ) : currentPrivilege?.passwordlessSudo ? (
              <span className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-cyan-500/15 border border-cyan-500/35 text-[11px] font-mono font-bold text-cyan-400">
                <CheckCircle2 className="w-3.5 h-3.5" />
                <span>SUDO NOPASSWD</span>
              </span>
            ) : currentPrivilege?.sudoUnlocked ? (
              <button
                type="button"
                onClick={() => setSudoPopoverOpen((p) => !p)}
                title="Sudo session unlocked. Click to update sudo password."
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-emerald-500/15 hover:bg-emerald-500/25 border border-emerald-500/35 text-[11px] font-mono font-bold text-emerald-300 cursor-pointer transition-colors"
              >
                <LockOpen className="w-3.5 h-3.5 text-emerald-400" />
                <span>SUDO UNLOCKED ({currentPrivilege.user})</span>
              </button>
            ) : (
              <button
                type="button"
                onClick={() => setSudoPopoverOpen((p) => !p)}
                className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-amber-500/15 hover:bg-amber-500/25 border border-amber-500/40 text-[11px] font-mono font-bold text-amber-300 cursor-pointer transition-colors"
              >
                <Lock className="w-3.5 h-3.5 text-amber-400" />
                <span>
                  {probingPrivilege
                    ? "PROBING SUDO..."
                    : "SUDO PASSWORD REQUIRED"}
                </span>
              </button>
            )}

            {/* Inline Sudo Password Unlock Popover */}
            {sudoPopoverOpen && (
              <form
                onSubmit={handleUnlockSudo}
                className="ui-popover-panel absolute right-0 top-full mt-2 w-80 rounded-2xl bg-popover border border-border p-4 shadow-2xl z-50 space-y-3"
              >
                <div className="flex items-center justify-between">
                  <div className="flex items-center gap-2 text-xs font-bold text-foreground">
                    <KeyRound className="w-3.5 h-3.5 text-amber-400" />
                    <span>
                      {isVi
                        ? `Mở khóa Sudo (${selectedHost.user})`
                        : `Unlock Sudo (${selectedHost.user})`}
                    </span>
                  </div>
                  <button
                    type="button"
                    onClick={() => setSudoPopoverOpen(false)}
                    className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent cursor-pointer"
                  >
                    <X className="w-3.5 h-3.5" />
                  </button>
                </div>

                <p className="text-[11px] text-muted-foreground leading-relaxed">
                  {isVi
                    ? "Mật khẩu sudo chỉ được lưu tạm thời trong phiên làm việc này để chạy các tác vụ cấu hình hệ thống."
                    : "Cached in memory for this session to execute privileged system configuration scripts."}
                </p>

                <input
                  type="password"
                  autoFocus
                  value={sudoInput}
                  onChange={(e) => setSudoInput(e.target.value)}
                  placeholder={
                    isVi ? "Nhập mật khẩu sudo..." : "Enter sudo password..."
                  }
                  className="ui-input text-xs font-mono"
                />

                {sudoUnlockError && (
                  <div className="text-[11px] text-rose-400 font-medium">
                    {sudoUnlockError}
                  </div>
                )}

                <div className="flex items-center justify-end gap-2">
                  <button
                    type="button"
                    onClick={() => setSudoPopoverOpen(false)}
                    className="px-3 py-1.5 rounded-xl bg-secondary text-xs font-medium text-foreground hover:bg-accent cursor-pointer"
                  >
                    {isVi ? "Đóng" : "Cancel"}
                  </button>
                  <button
                    type="submit"
                    disabled={probingPrivilege || !sudoInput.trim()}
                    className="px-3.5 py-1.5 rounded-xl bg-primary text-primary-foreground text-xs font-semibold hover:opacity-95 disabled:opacity-50 cursor-pointer"
                  >
                    {probingPrivilege
                      ? isVi
                        ? "Đang kiểm tra..."
                        : "Verifying..."
                      : isVi
                        ? "Mở khóa"
                        : "Unlock Sudo"}
                  </button>
                </div>
              </form>
            )}
          </div>

          <button
            type="button"
            onClick={() => {
              probeHostPrivilege(selectedHost.id);
              setRefreshNonce((n) => n + 1);
            }}
            title="Refresh Server Configuration"
            className="p-2 rounded-xl bg-secondary/80 hover:bg-accent text-muted-foreground hover:text-foreground border border-border/70 transition-colors cursor-pointer"
          >
            <RefreshCw
              className={`w-3.5 h-3.5 ${probingPrivilege ? "animate-spin text-primary" : ""}`}
            />
          </button>

          {onConnectHostTerminal && (
            <button
              type="button"
              onClick={() => onConnectHostTerminal(selectedHost)}
              className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-xl bg-secondary/90 hover:bg-primary hover:text-primary-foreground text-xs font-semibold text-foreground border border-border/70 transition-colors cursor-pointer"
            >
              <TerminalIcon className="w-3.5 h-3.5" />
              <span>SSH</span>
            </button>
          )}
        </div>
      </div>

      {/* 6-Studio Switcher Bar */}
      <div className="px-6 pt-3 pb-2 border-b border-border/60 bg-background/70 flex items-center gap-2 overflow-x-auto no-scrollbar shrink-0">
        {studioTabs.map((tab) => {
          const active = activeStudio === tab.id;
          return (
            <button
              key={tab.id}
              type="button"
              onClick={() => setActiveStudio(tab.id)}
              className={`group flex items-center gap-2.5 px-3.5 py-2 rounded-xl border text-left transition-all cursor-pointer shrink-0 ${
                active
                  ? "bg-primary/14 border-primary/45 text-foreground shadow-2xs"
                  : "bg-card/60 border-border/70 text-muted-foreground hover:bg-accent/50 hover:text-foreground"
              }`}
            >
              <div className="shrink-0">{tab.icon}</div>
              <div>
                <div className="text-xs font-bold leading-tight">
                  {tab.label}
                </div>
                <div className="text-[10px] text-muted-foreground font-mono">
                  {tab.subtitle}
                </div>
              </div>
            </button>
          );
        })}
      </div>

      {/* Active Studio Canvas */}
      <div className="flex-1 overflow-y-auto p-6">
        {activeStudio === "ssh" && (
          <SshHardeningStudio
            key={`${selectedHost.id}-${refreshNonce}`}
            host={selectedHost}
            sudoPassword={currentSudoPassword}
            privilegeInfo={currentPrivilege}
            language={lang}
            onSaveHost={onSaveHost}
            onRequireSudoUnlock={() => setSudoPopoverOpen(true)}
          />
        )}

        {activeStudio === "firewall" && (
          <FirewallStudio
            key={`${selectedHost.id}-${refreshNonce}`}
            host={selectedHost}
            sudoPassword={currentSudoPassword}
            privilegeInfo={currentPrivilege}
            language={lang}
            onRequireSudoUnlock={() => setSudoPopoverOpen(true)}
          />
        )}

        {activeStudio === "access" && (
          <AccessKeysStudio
            key={`${selectedHost.id}-${refreshNonce}`}
            host={selectedHost}
            sshKeys={resources.sshKeys}
            sudoPassword={currentSudoPassword}
            privilegeInfo={currentPrivilege}
            language={lang}
            onRequireSudoUnlock={() => setSudoPopoverOpen(true)}
          />
        )}
      </div>
    </div>
  );
};
