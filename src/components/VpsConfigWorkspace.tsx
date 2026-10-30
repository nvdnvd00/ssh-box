import React, { useCallback, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { RefreshCw, Server } from "lucide-react";
import { HostEntry, PrivilegeLevel, ResourcesStoreData, VpsProbe, VpsStudioTab } from "../types";
import { AppSettingsConfig } from "../themes";
import { getI18n } from "../i18n";
import { SelectField } from "./SelectField";
import { StatusBadge, StatusTone } from "./vps/shared/StatusBadge";
import { vpsErrorText } from "./vps/shared/errors";
import { VpsCtx } from "./vps/shared/useVpsResource";
import { Notice } from "./vps/shared/Notice";
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

const PRIVILEGE_TONE: Record<PrivilegeLevel, StatusTone> = {
  root: "ok",
  sudoNoPassword: "ok",
  sudoPassword: "ok",
  none: "warn",
};

export const VpsConfigWorkspace: React.FC<VpsConfigWorkspaceProps> = ({
  hosts,
  resources,
  activeHostId,
  appSettings,
  onSelectHost,
  onSaveHost,
}) => {
  const t = getI18n(appSettings?.language);
  const pickInitial = () =>
    hosts.some((h) => h.id === activeHostId) ? activeHostId : (hosts[0]?.id ?? "");
  const [selectedHostId, setSelectedHostId] = useState<string>(pickInitial);
  useEffect(() => {
    if (activeHostId && hosts.some((h) => h.id === activeHostId)) setSelectedHostId(activeHostId);
  }, [activeHostId, hosts]);

  const [activeStudio, setActiveStudio] = useState<VpsStudioTab>("ssh");
  const [probes, setProbes] = useState<Record<string, VpsProbe>>({});
  // Sudo passwords live only in this component's memory (spec §5).
  const [sudoPasswords, setSudoPasswords] = useState<Record<string, string>>({});
  const [probing, setProbing] = useState(false);
  const [probeError, setProbeError] = useState<string | null>(null);
  const [sudoOpen, setSudoOpen] = useState(false);
  const [sudoInput, setSudoInput] = useState("");
  const [sudoError, setSudoError] = useState<string | null>(null);

  const host = hosts.find((h) => h.id === selectedHostId) ?? null;
  const isLocal = !host || host.id === "local";
  const probe = host ? probes[host.id] : undefined;
  const sudoPassword = host ? sudoPasswords[host.id] : undefined;

  const runProbe = useCallback(
    async (hostId: string, password?: string) => {
      setProbing(true);
      setProbeError(null);
      try {
        const result = await invoke<VpsProbe>("vps_probe", { hostId, sudoPassword: password ?? null });
        setProbes((prev) => ({ ...prev, [hostId]: result }));
        return result;
      } finally {
        setProbing(false);
      }
    },
    [],
  );

  useEffect(() => {
    if (!host || isLocal) return;
    runProbe(host.id, sudoPasswords[host.id]).catch((e) => setProbeError(vpsErrorText(t, e)));
    // Re-probe only when the host changes; unlocking sudo probes explicitly.
  }, [host?.id, isLocal]);

  const unlockSudo = async (e: React.FormEvent) => {
    e.preventDefault();
    if (!host || !sudoInput) return;
    setSudoError(null);
    try {
      const result = await runProbe(host.id, sudoInput);
      if (result.privilege === "none") {
        setSudoError(t.vpsErrSudoDenied);
        return;
      }
      setSudoPasswords((prev) => ({ ...prev, [host.id]: sudoInput }));
      setSudoOpen(false);
      setSudoInput("");
    } catch (err) {
      setSudoError(vpsErrorText(t, err));
    }
  };

  const vps: VpsCtx = useMemo(
    () => ({ hostId: host?.id ?? "", sudoPassword, requestSudo: () => setSudoOpen(true), t }),
    [host?.id, sudoPassword, t],
  );

  const privilegeLabel: Record<PrivilegeLevel, string> = {
    root: t.vpsPrivRoot,
    sudoNoPassword: t.vpsPrivSudoNoPassword,
    sudoPassword: t.vpsPrivSudoPassword,
    none: t.vpsPrivNone,
  };
  const tabs: { id: VpsStudioTab; label: string }[] = [
    { id: "ssh", label: t.vpsTabSsh },
    { id: "firewall", label: t.vpsTabFirewall },
    { id: "access", label: t.vpsTabAccess },
  ];

  if (hosts.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center h-full p-8 text-center gap-3">
        <Server className="w-8 h-8 text-muted-foreground" />
        <p className="text-sm text-muted-foreground">{t.vpsShellNoHosts}</p>
      </div>
    );
  }

  return (
    <div className="flex flex-col h-full min-w-0">
      <div className="border-b border-border/70 px-4 py-3 flex flex-wrap items-center gap-x-3 gap-y-2">
        <SelectField
          size="sm"
          className="w-full sm:w-64"
          value={selectedHostId}
          placeholder={t.vpsShellSelectHost}
          options={hosts.map((h) => ({ value: h.id, label: h.alias || h.hostname }))}
          onChange={(id) => {
            setSelectedHostId(id);
            onSelectHost?.(id);
          }}
        />
        {host && !isLocal && (
          <span className="font-mono text-xs text-muted-foreground break-all">
            {host.user}@{host.hostname}:{host.port}
            {probe ? ` · ${probe.osId} ${probe.osVersion}` : ""}
          </span>
        )}
        {probe && <StatusBadge tone={PRIVILEGE_TONE[probe.privilege]}>{privilegeLabel[probe.privilege]}</StatusBadge>}
        {probe?.privilege === "none" && (
          <button
            type="button"
            onClick={() => setSudoOpen(true)}
            className="px-3 py-1.5 rounded-xl text-xs font-medium border border-border/60 hover:bg-accent/70 cursor-pointer"
          >
            {t.vpsSudoOpen}
          </button>
        )}
        {host && !isLocal && (
          <button
            type="button"
            aria-label={t.vpsCommonReload}
            disabled={probing}
            onClick={() => runProbe(host.id, sudoPassword).catch((e) => setProbeError(vpsErrorText(t, e)))}
            className="ml-auto p-2 rounded-xl border border-border/60 hover:bg-accent/70 cursor-pointer disabled:opacity-50"
          >
            <RefreshCw className={`w-4 h-4 ${probing ? "animate-spin" : ""}`} />
          </button>
        )}
      </div>

      <div className="border-b border-border/70 px-2 flex gap-1 overflow-x-auto">
        {tabs.map((tab) => (
          <button
            key={tab.id}
            type="button"
            onClick={() => setActiveStudio(tab.id)}
            className={`shrink-0 px-3 py-2.5 text-sm border-b-2 cursor-pointer ${
              activeStudio === tab.id
                ? "border-primary text-foreground font-medium"
                : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            {tab.label}
          </button>
        ))}
      </div>

      <div className="flex-1 overflow-y-auto select-text">
        <div className="max-w-4xl mx-auto w-full p-4 sm:p-6 space-y-4">
          {isLocal && <Notice tone="warn">{t.vpsUnsupportedReasons.linux_only}</Notice>}
          {!isLocal && probeError && <Notice tone="danger">{probeError}</Notice>}
          {!isLocal && probing && !probe && <p className="text-sm text-muted-foreground">{t.vpsShellProbing}</p>}
          {!isLocal && probe && host && !probeError && activeStudio === "ssh" && (
            <SshHardeningStudio key={host.id} vps={vps} probe={probe} host={host} onSaveHost={onSaveHost} />
          )}
          {!isLocal && probe && host && !probeError && activeStudio === "firewall" && (
            <FirewallStudio key={host.id} vps={vps} probe={probe} host={host} onSaveHost={onSaveHost} />
          )}
          {!isLocal && probe && host && !probeError && activeStudio === "access" && (
            <AccessKeysStudio
              key={host.id}
              vps={vps}
              probe={probe}
              host={host}
              onSaveHost={onSaveHost}
              sshKeys={resources.sshKeys ?? []}
            />
          )}
        </div>
      </div>

      {sudoOpen && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <form
            onSubmit={unlockSudo}
            className="ui-modal-content w-full max-w-sm rounded-2xl bg-popover border border-border shadow-2xl"
          >
            <div className="px-5 py-3.5 border-b border-border">
              <h3 className="text-sm font-semibold text-foreground">{t.vpsSudoTitle}</h3>
            </div>
            <div className="px-5 py-4 space-y-2">
              <label className="ui-label" htmlFor="vps-sudo-password">
                {host ? `${host.user}@${host.hostname}` : ""}
              </label>
              <input
                id="vps-sudo-password"
                type="password"
                autoFocus
                autoComplete="current-password"
                className="ui-input"
                value={sudoInput}
                onChange={(e) => setSudoInput(e.target.value)}
              />
              <p className="text-xs text-muted-foreground">{t.vpsSudoHint}</p>
              {sudoError && <p className="text-xs text-danger">{sudoError}</p>}
            </div>
            <div className="px-5 py-3 border-t border-border flex justify-end gap-2">
              <button
                type="button"
                onClick={() => {
                  setSudoOpen(false);
                  setSudoError(null);
                }}
                className="px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 cursor-pointer"
              >
                {t.vpsCommonCancel}
              </button>
              <button
                type="submit"
                disabled={!sudoInput || probing}
                className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:opacity-90 cursor-pointer disabled:opacity-50"
              >
                {t.vpsSudoUnlock}
              </button>
            </div>
          </form>
        </div>
      )}
    </div>
  );
};

