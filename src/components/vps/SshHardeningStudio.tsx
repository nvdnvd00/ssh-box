import React, { useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Check, Loader2, X } from "lucide-react";
import { SshdApplyStarted, SshdSettings, SshdView } from "../../types";
import { SelectField } from "../SelectField";
import { Switch } from "./shared/Switch";
import { SettingGroup, SettingRow } from "./shared/SettingRow";
import { StatusBadge } from "./shared/StatusBadge";
import { ChangePreviewDialog, PendingChange } from "./shared/ChangePreviewDialog";
import { parseVpsError, vpsErrorText } from "./shared/errors";
import { useVpsResource, VpsStudioProps } from "./shared/useVpsResource";
import { Notice } from "./shared/Notice";

type Phase = "applying" | "verifying" | "done" | "failed";
interface Progress {
  phase: Phase;
  deadline?: number;
  message?: string;
  failedAt?: number;
}

const NUMBER_FIELDS = ["maxAuthTries", "maxSessions", "loginGraceTime", "clientAliveInterval", "clientAliveCountMax"] as const;

export const SshHardeningStudio: React.FC<VpsStudioProps> = ({ vps, probe, host, onSaveHost }) => {
  const { t, hostId } = vps;
  // Non-admin Windows cannot read sshd -T: the backend refuses, and the notice below already says why.
  const skipRead = probe.support.ssh.reason === "windows_not_admin";
  const res = useVpsResource<SshdView | null>(vps, (pw) =>
    skipRead ? Promise.resolve(null) : invoke<SshdView>("vps_get_sshd_settings", { hostId, sudoPassword: pw ?? null }),
  );
  const current = res.data?.settings ?? null;
  const [draft, setDraft] = useState<SshdSettings | null>(null);
  // Raw text of the user lists, so typing a space before the next name is not swallowed.
  const [usersText, setUsersText] = useState({ allowUsers: "", denyUsers: "" });
  const reset = () => {
    setDraft(current);
    setUsersText({ allowUsers: current?.allowUsers.join(" ") ?? "", denyUsers: current?.denyUsers.join(" ") ?? "" });
  };
  useEffect(reset, [current]);
  const rolledBack = useRef(false);

  const [previewOpen, setPreviewOpen] = useState(false);
  const [blocked, setBlocked] = useState<string | null>(null);
  const [progress, setProgress] = useState<Progress | null>(null);
  const [now, setNow] = useState(Date.now());
  useEffect(() => {
    if (progress?.phase !== "verifying") return;
    const id = window.setInterval(() => setNow(Date.now()), 1000);
    return () => window.clearInterval(id);
  }, [progress?.phase]);

  const { ssh, sshRows, passwordCoupled } = probe.support;
  // macOS: password login also works through keyboard-interactive, so both fields move together.
  const passwordOn = (c: SshdSettings) => c.passwordAuthentication || (passwordCoupled && c.kbdInteractiveAuthentication);
  const readOnly = ssh.level === "readOnly" || !!res.data?.rollbackPending;
  const set = <K extends keyof SshdSettings>(key: K, value: SshdSettings[K]) =>
    setDraft((d) => (d ? { ...d, [key]: value } : d));

  const rootOptions = [
    { value: "yes", label: t.vpsSshRootYes },
    { value: "prohibit-password", label: t.vpsSshRootKeyOnly },
    { value: "forced-commands-only", label: t.vpsSshRootForced },
    { value: "no", label: t.vpsSshRootNo },
  ];
  const labels: Record<keyof SshdSettings, string> = {
    port: t.vpsSshPort,
    passwordAuthentication: t.vpsSshPassword,
    kbdInteractiveAuthentication: t.vpsSshPassword,
    pubkeyAuthentication: t.vpsSshPubkey,
    permitRootLogin: t.vpsSshRoot,
    allowUsers: t.vpsSshAllowUsers,
    denyUsers: t.vpsSshDenyUsers,
    maxAuthTries: t.vpsSshMaxAuthTries,
    maxSessions: t.vpsSshMaxSessions,
    loginGraceTime: t.vpsSshLoginGrace,
    clientAliveInterval: t.vpsSshAliveInterval,
    clientAliveCountMax: t.vpsSshAliveCount,
  };
  const show = (key: keyof SshdSettings, v: SshdSettings[keyof SshdSettings]): string => {
    if (typeof v === "boolean") return v ? "yes" : "no";
    if (Array.isArray(v)) return v.join(" ");
    if (key === "permitRootLogin") return rootOptions.find((o) => o.value === v)?.label ?? String(v);
    return String(v);
  };
  const changes: PendingChange[] = useMemo(() => {
    if (!current || !draft) return [];
    return (Object.keys(labels) as (keyof SshdSettings)[])
      .map((k) => ({ k, from: current[k], to: draft[k] }))
      .filter((c) => c.k !== "kbdInteractiveAuthentication" || passwordCoupled)
      .filter((c) => show(c.k, c.from) !== show(c.k, c.to))
      .map((c) => ({ label: c.k === "kbdInteractiveAuthentication" ? "KbdInteractiveAuthentication" : labels[c.k], from: show(c.k, c.from), to: show(c.k, c.to) }));
  }, [current, draft, t, passwordCoupled]);
  const valid =
    !!draft &&
    Number.isInteger(draft.port) && draft.port >= 1 && draft.port <= 65535 &&
    NUMBER_FIELDS.every((k) => Number.isInteger(draft[k]) && draft[k] >= 0) &&
    draft.maxAuthTries >= 1 &&
    draft.maxSessions >= 1;

  const apply = async () => {
    if (!draft) return;
    setBlocked(null);
    setProgress({ phase: "applying" });
    let started: SshdApplyStarted;
    try {
      started = await invoke<SshdApplyStarted>("vps_apply_sshd_settings", {
        hostId, desired: draft, sudoPassword: vps.sudoPassword ?? null,
      });
    } catch (e) {
      setProgress(null);
      if (parseVpsError(e).code === "LOCKOUT_BLOCKED") return setBlocked(vpsErrorText(t, e));
      setPreviewOpen(false);
      return setProgress({ phase: "failed", failedAt: 0, message: vpsErrorText(t, e) });
    }
    setPreviewOpen(false);
    setProgress({ phase: "verifying", deadline: Date.now() + (started.rollbackAt - started.serverNow) * 1000 });
    rolledBack.current = false;
    try {
      const view = await invoke<SshdView>("vps_finish_sshd_apply", {
        hostId, newPort: started.newPort, sudoPassword: vps.sudoPassword ?? null,
      });
      let message = t.vpsSshDone;
      // Save only a port the server is really on now (not after "Roll back now" or the timer).
      const newPort = started.newPort;
      if (newPort !== null && !rolledBack.current && view.settings.port === newPort) {
        await onSaveHost({ ...host, port: newPort });
        message += " " + t.vpsSshPortSaved.replace("{p}", String(newPort));
      }
      await res.mutate(async () => view);
      setProgress({ phase: "done", message });
    } catch (e) {
      setProgress({ phase: "failed", failedAt: 7, message: vpsErrorText(t, e) });
      await res.refresh();
    }
  };

  const rollbackNow = async () => {
    rolledBack.current = true;
    try {
      await res.mutate((pw) => invoke<SshdView>("vps_revert_sshd_settings", { hostId, sudoPassword: pw ?? null }));
      setProgress({ phase: "failed", failedAt: 7, message: t.vpsSshRolledBack });
    } catch (e) {
      setProgress({ phase: "failed", message: vpsErrorText(t, e) });
    }
  };

  if (res.loading && !res.data) return <p className="text-sm text-muted-foreground">{t.vpsCommonLoading}</p>;
  if (skipRead) return <Notice tone="warn">{t.vpsUnsupportedReasons[ssh.reason!] ?? ssh.reason}</Notice>;
  if (!draft || !current) return res.error ? <Notice tone="danger">{res.error}</Notice> : null;

  const checks = [
    { id: "row-password", label: t.vpsSshCheckPassword, ok: !passwordOn(current) },
    ...(sshRows.permitRootLogin ? [{ id: "row-root", label: t.vpsSshCheckRoot, ok: current.permitRootLogin !== "yes" }] : []),
    { id: "row-maxauth", label: t.vpsSshCheckMaxAuth, ok: current.maxAuthTries <= 4 },
  ];
  const numberInput = (key: "port" | (typeof NUMBER_FIELDS)[number]) => (
    <input
      type="number"
      inputMode="numeric"
      className="ui-input w-28"
      value={Number.isNaN(draft[key]) ? "" : draft[key]}
      onChange={(e) => set(key, e.target.value === "" ? NaN : Number(e.target.value))}
    />
  );
  const usersInput = (key: "allowUsers" | "denyUsers") => (
    <input
      className="ui-input w-full sm:w-72 font-mono"
      value={usersText[key]}
      onChange={(e) => {
        setUsersText((u) => ({ ...u, [key]: e.target.value }));
        set(key, e.target.value.split(/\s+/).filter(Boolean));
      }}
    />
  );
  const remaining = progress?.deadline ? Math.max(0, Math.ceil((progress.deadline - now) / 1000)) : 0;

  return (
    <div className="space-y-5">
      {res.error && <Notice tone="danger">{res.error}</Notice>}
      {ssh.reason && <Notice tone="warn">{t.vpsUnsupportedReasons[ssh.reason] ?? ssh.reason}</Notice>}
      {res.data?.rollbackPending && (
        <Notice tone="warn">
          {t.vpsSshPending}{" "}
          <button type="button" className="underline cursor-pointer" onClick={rollbackNow}>
            {t.vpsCommonRollbackNow}
          </button>
        </Notice>
      )}

      <SettingGroup title={t.vpsSshStatusTitle}>
        {checks.map((c) => (
          <SettingRow key={c.id} label={c.label}>
            <div className="flex items-center gap-2">
              <StatusBadge tone={c.ok ? "ok" : "warn"}>{c.ok ? t.vpsCommonPass : t.vpsCommonFail}</StatusBadge>
              {!c.ok && (
                <button
                  type="button"
                  className="text-xs text-primary hover:underline cursor-pointer"
                  onClick={() => document.getElementById(c.id)?.scrollIntoView({ behavior: "smooth", block: "center" })}
                >
                  {t.vpsCommonFix}
                </button>
              )}
            </div>
          </SettingRow>
        ))}
      </SettingGroup>

      <fieldset disabled={readOnly} className="space-y-5 min-w-0">
        <SettingGroup title={t.vpsSshGroupLogin}>
          {sshRows.port && (
            <SettingRow label={t.vpsSshPort} description={t.vpsSshPortHint} directive="Port" effective={String(current.port)}>
              {numberInput("port")}
            </SettingRow>
          )}
          <SettingRow id="row-password" label={t.vpsSshPassword} directive="PasswordAuthentication">
            <Switch label={t.vpsSshPassword} checked={passwordOn(draft)} onChange={(v) => setDraft((d) => d && { ...d, passwordAuthentication: v, ...(passwordCoupled && { kbdInteractiveAuthentication: v }) })} />
          </SettingRow>
          <SettingRow label={t.vpsSshPubkey} directive="PubkeyAuthentication">
            <Switch label={t.vpsSshPubkey} checked={draft.pubkeyAuthentication} onChange={(v) => set("pubkeyAuthentication", v)} />
          </SettingRow>
          {sshRows.permitRootLogin && (
            <SettingRow id="row-root" label={t.vpsSshRoot} directive="PermitRootLogin">
              <SelectField size="sm" className="w-full sm:w-52" value={draft.permitRootLogin} options={rootOptions} onChange={(v) => set("permitRootLogin", v)} />
            </SettingRow>
          )}
        </SettingGroup>

        {sshRows.allowDenyUsers && (
          <SettingGroup title={t.vpsSshGroupWho}>
            <SettingRow label={t.vpsSshAllowUsers} description={t.vpsSshUsersHint} directive="AllowUsers">
              {usersInput("allowUsers")}
            </SettingRow>
            <SettingRow label={t.vpsSshDenyUsers} directive="DenyUsers">
              {usersInput("denyUsers")}
            </SettingRow>
          </SettingGroup>
        )}

        <SettingGroup title={t.vpsSshGroupLimits}>
          <SettingRow id="row-maxauth" label={t.vpsSshMaxAuthTries} directive="MaxAuthTries">{numberInput("maxAuthTries")}</SettingRow>
          <SettingRow label={t.vpsSshMaxSessions} directive="MaxSessions">{numberInput("maxSessions")}</SettingRow>
          <SettingRow label={t.vpsSshLoginGrace} directive="LoginGraceTime">{numberInput("loginGraceTime")}</SettingRow>
          <SettingRow label={t.vpsSshAliveInterval} directive="ClientAliveInterval">{numberInput("clientAliveInterval")}</SettingRow>
          <SettingRow label={t.vpsSshAliveCount} directive="ClientAliveCountMax">{numberInput("clientAliveCountMax")}</SettingRow>
        </SettingGroup>
      </fieldset>

      <p className="text-xs text-muted-foreground font-mono break-all">
        {res.data?.writeTarget === "dropIn" ? t.vpsSshWriteDropIn : t.vpsSshWriteMain}
      </p>

      {changes.length > 0 && (
        <div className="sticky bottom-0 -mx-4 sm:mx-0 border-t sm:border border-border bg-popover/95 backdrop-blur sm:rounded-2xl px-4 py-3 flex flex-col gap-2 sm:flex-row sm:items-center sm:justify-between">
          <span className="text-sm text-foreground">
            {valid ? t.vpsCommonPendingChanges.replace("{n}", String(changes.length)) : t.vpsSshInvalid}
          </span>
          <div className="flex gap-2">
            <button type="button" onClick={reset} className="flex-1 sm:flex-none px-3 py-1.5 rounded-xl text-xs font-medium border border-border/60 hover:bg-accent/70 cursor-pointer">
              {t.vpsCommonDiscard}
            </button>
            <button
              type="button"
              disabled={!valid || readOnly}
              onClick={() => { setBlocked(null); setPreviewOpen(true); }}
              className="flex-1 sm:flex-none px-3 py-1.5 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:opacity-90 cursor-pointer disabled:opacity-50"
            >
              {t.vpsCommonPreviewApply}
            </button>
          </div>
        </div>
      )}

      <ChangePreviewDialog
        t={t}
        open={previewOpen}
        changes={changes}
        blockedReason={blocked}
        applying={progress?.phase === "applying"}
        onCancel={() => setPreviewOpen(false)}
        onApply={apply}
      />

      {progress && progress.phase !== "applying" && (
        <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
          <div className="ui-modal-content w-full max-w-md rounded-2xl bg-popover border border-border shadow-2xl">
            <div className="px-5 py-3.5 border-b border-border">
              <h3 className="text-sm font-semibold text-foreground">{t.vpsSshProgressTitle}</h3>
            </div>
            <ol className="px-5 py-4 space-y-1.5">
              {t.vpsSshSteps.map((label, i) => {
                const doneUpTo = progress.phase === "done" ? 9 : progress.phase === "verifying" ? 7 : (progress.failedAt ?? 0);
                const state = i < doneUpTo ? "done" : progress.phase === "verifying" && i === 7 ? "running" : progress.phase === "failed" && i === doneUpTo ? "failed" : "todo";
                return (
                  <li key={label} className="flex items-center gap-2 text-sm">
                    {state === "done" && <Check className="w-4 h-4 text-success" />}
                    {state === "running" && <Loader2 className="w-4 h-4 animate-spin text-primary" />}
                    {state === "failed" && <X className="w-4 h-4 text-danger" />}
                    {state === "todo" && <span className="w-4 h-4" />}
                    <span className={state === "todo" ? "text-muted-foreground" : "text-foreground"}>{label}</span>
                  </li>
                );
              })}
            </ol>
            <div className="px-5 pb-4 space-y-2 text-sm">
              {progress.phase === "verifying" && (
                <p className="text-muted-foreground">{t.vpsSshRollbackIn.replace("{s}", String(remaining))}</p>
              )}
              {progress.message && (
                <p className={progress.phase === "done" ? "text-success" : "text-danger"}>{progress.message}</p>
              )}
            </div>
            <div className="px-5 py-3 border-t border-border flex justify-end gap-2">
              {progress.phase === "verifying" ? (
                <button type="button" onClick={rollbackNow} className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-danger/15 text-danger border border-danger/40 cursor-pointer">
                  {t.vpsCommonRollbackNow}
                </button>
              ) : (
                <button type="button" onClick={() => setProgress(null)} className="px-3 py-1.5 rounded-xl text-xs font-medium border border-border/60 hover:bg-accent/70 cursor-pointer">
                  {t.vpsSshClose}
                </button>
              )}
            </div>
          </div>
        </div>
      )}
    </div>
  );
};

