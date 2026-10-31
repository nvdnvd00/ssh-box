import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AccessKeysSnapshot, LinuxAccountItem, SshKeyItem } from "../../types";
import { SelectField } from "../SelectField";
import { SettingGroup, SettingRow } from "./shared/SettingRow";
import { StatusBadge } from "./shared/StatusBadge";
import { ConfirmButton } from "./shared/ConfirmButton";
import { ChangePreviewDialog, PendingChange } from "./shared/ChangePreviewDialog";
import { Notice } from "./shared/Notice";
import { parseVpsError, vpsErrorText } from "./shared/errors";
import { useVpsResource, VpsStudioProps } from "./shared/useVpsResource";

type SudoMode = "none" | "noPassword" | "withPassword";
const USERNAME = /^[a-z_][a-z0-9_-]{0,30}$/;

const sudoModeOf = (a: LinuxAccountItem): SudoMode =>
  !a.hasSudo ? "none" : a.nopasswdSudo ? "noPassword" : "withPassword";

interface Pending {
  change: PendingChange;
  run: (sudoPassword: string | null) => Promise<AccessKeysSnapshot>;
}

const buttonClass =
  "shrink-0 px-3 py-1.5 rounded-xl text-xs font-medium border border-border/60 hover:bg-accent/70 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed";

export const AccessKeysStudio: React.FC<VpsStudioProps & { sshKeys: SshKeyItem[] }> = ({ vps, probe, sshKeys }) => {
  const { t, hostId } = vps;
  const res = useVpsResource<AccessKeysSnapshot>(vps, (pw) =>
    invoke<AccessKeysSnapshot>("vps_get_access_keys_snapshot", { hostId, sudoPassword: pw ?? null }),
  );
  const [pending, setPending] = useState<Pending | null>(null);
  const [blocked, setBlocked] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [newUser, setNewUser] = useState({ name: "", mode: "none" as SudoMode, password: "" });
  const [drafts, setDrafts] = useState<Record<string, { mode: SudoMode; password: string; keyId: string }>>({});

  const { level, reason } = probe.support.access;
  const readOnly = level === "readOnly";
  // Presentation only: Windows accounts have no uid/shell or sudo modes, just administrator or standard.
  const isWindows = probe.osFamily === "windows";
  const ctx = probe.context;
  const appKey = ctx.auth.kind === "key" ? ctx.auth.publicKeyBody : null;
  const modeOptions = (["none", "noPassword", "withPassword"] as SudoMode[]).map((m) => ({ value: m, label: t.vpsUsersSudoModes[m] }));

  const ask = (change: PendingChange, run: Pending["run"]) => {
    setBlocked(null);
    setPending({ change, run });
  };
  const apply = async () => {
    if (!pending) return;
    setApplying(true);
    try {
      const next = await pending.run(vps.sudoPassword ?? null);
      await res.mutate(async () => next);
      setPending(null);
      setDrafts({});
      setNewUser({ name: "", mode: "none", password: "" });
    } catch (e) {
      if (parseVpsError(e).code === "LOCKOUT_BLOCKED") setBlocked(vpsErrorText(t, e));
      else {
        setPending(null);
        res.failWrite(e);
      }
    } finally {
      setApplying(false);
    }
  };
  const act = (cmd: string, args: Record<string, unknown>) =>
    res.mutate((pw) => invoke<AccessKeysSnapshot>(cmd, { hostId, ...args, sudoPassword: pw ?? null }));

  if (res.loading && !res.data) return <p className="text-sm text-muted-foreground">{t.vpsCommonLoading}</p>;
  if (!res.data) return res.error ? <Notice tone="danger">{res.error}</Notice> : null;

  const nameOk = USERNAME.test(newUser.name) && newUser.name !== "root";
  const passwordOk = (mode: SudoMode, pw: string) => mode !== "withPassword" || (pw.length > 0 && !/[\r\n]/.test(pw));
  const passwordInput = (value: string, onChange: (v: string) => void) => (
    <input type="password" autoComplete="new-password" className="ui-input" value={value} onChange={(e) => onChange(e.target.value)} />
  );

  return (
    <div className="space-y-5">
      {res.error && <Notice tone="danger">{res.error}</Notice>}
      {reason && <Notice tone="warn">{t.vpsUnsupportedReasons[reason] ?? reason}</Notice>}

      {!readOnly && (
      <SettingGroup title={t.vpsUsersCreateTitle}>
        <div className="px-4 py-3 grid gap-3 sm:grid-cols-2">
          <label className="space-y-1">
            <span className="ui-label">{t.vpsUsersName}</span>
            <input className="ui-input font-mono" value={newUser.name} onChange={(e) => setNewUser({ ...newUser, name: e.target.value })} />
            {newUser.name && !nameOk && <span className="text-xs text-danger">{t.vpsUsersInvalidName}</span>}
          </label>
          <label className="space-y-1">
            <span className="ui-label">{t.vpsUsersSudo}</span>
            <SelectField size="sm" value={newUser.mode} options={modeOptions} onChange={(m) => setNewUser({ ...newUser, mode: m as SudoMode })} />
          </label>
          {newUser.mode === "withPassword" && (
            <label className="space-y-1 sm:col-span-2">
              <span className="ui-label">{t.vpsUsersPassword}</span>
              {passwordInput(newUser.password, (password) => setNewUser({ ...newUser, password }))}
            </label>
          )}
          <div className="sm:col-span-2 flex justify-end">
            <button
              type="button"
              disabled={!nameOk || !passwordOk(newUser.mode, newUser.password)}
              className={buttonClass}
              onClick={() =>
                ask({ label: t.vpsUsersCreateTitle, from: "", to: `${newUser.name} · ${t.vpsUsersSudoModes[newUser.mode]}` }, (pw) =>
                  invoke<AccessKeysSnapshot>("vps_create_linux_user", {
                    hostId, username: newUser.name, sudo: newUser.mode,
                    password: newUser.mode === "withPassword" ? newUser.password : null, sudoPassword: pw,
                  }),
                )
              }
            >
              {t.vpsUsersCreate}
            </button>
          </div>
        </div>
      </SettingGroup>
      )}

      <fieldset disabled={readOnly} className="space-y-5 min-w-0">
      {res.data.accounts.map((a) => {
        const isRoot = a.username === "root";
        const current = sudoModeOf(a);
        const draft = drafts[a.username] ?? { mode: current, password: "", keyId: "" };
        const setDraft = (patch: Partial<typeof draft>) => setDrafts((d) => ({ ...d, [a.username]: { ...draft, ...patch } }));
        const grantKey = sshKeys.find((k) => k.id === draft.keyId);
        return (
          <SettingGroup key={a.username} title={a.username}>
            <SettingRow
              label={a.username}
              description={isRoot ? t.vpsUsersRootReadOnly : undefined}
              directive={isWindows ? undefined : `uid ${a.uid} · ${a.shell}`}
            >
              <div className="flex flex-wrap gap-2">
                <StatusBadge tone={current === "none" ? "neutral" : "ok"}>
                  {isWindows ? (a.hasSudo ? t.vpsPrivAdmin : t.vpsPrivStandard) : t.vpsUsersSudoModes[current]}
                </StatusBadge>
                {a.locked && <StatusBadge tone="warn">{t.vpsUsersLocked}</StatusBadge>}
              </div>
            </SettingRow>
            {!isRoot && !isWindows && (
              <>
                <SettingRow label={t.vpsUsersSudo}>
                  <div className="flex flex-col gap-2 sm:items-end">
                    <SelectField size="sm" className="w-full sm:w-56" value={draft.mode} options={modeOptions} onChange={(m) => setDraft({ mode: m as SudoMode })} />
                    {draft.mode !== current && draft.mode === "withPassword" && passwordInput(draft.password, (password) => setDraft({ password }))}
                    {draft.mode !== current && (
                      <button
                        type="button"
                        disabled={!passwordOk(draft.mode, draft.password)}
                        className={buttonClass}
                        onClick={() =>
                          ask(
                            { label: `${a.username} · ${t.vpsUsersSudo}`, from: t.vpsUsersSudoModes[current], to: t.vpsUsersSudoModes[draft.mode] },
                            (pw) =>
                              invoke<AccessKeysSnapshot>("vps_set_user_sudo", {
                                hostId, username: a.username, sudo: draft.mode,
                                password: draft.mode === "withPassword" ? draft.password : null, sudoPassword: pw,
                              }),
                          )
                        }
                      >
                        {t.vpsCommonPreviewApply}
                      </button>
                    )}
                  </div>
                </SettingRow>
                <SettingRow label={t.vpsUsersLock} description={t.vpsUsersLockHint}>
                  {a.locked ? (
                    <button type="button" className={buttonClass} onClick={() => act("vps_set_user_lock", { username: a.username, lock: false })}>
                      {t.vpsUsersUnlock}
                    </button>
                  ) : (
                    <ConfirmButton
                      label={t.vpsUsersLock}
                      confirmLabel={t.vpsUsersLockConfirm}
                      cancelLabel={t.vpsCommonCancel}
                      onConfirm={() => act("vps_set_user_lock", { username: a.username, lock: true }).then(() => undefined)}
                    />
                  )}
                </SettingRow>
              </>
            )}
            {a.authorizedKeys.length === 0 && (
              <p className="px-4 py-3 text-sm text-muted-foreground">{a.keysFileUnknown ? t.vpsUsersKeysFileUnknown : t.vpsUsersNoKeys}</p>
            )}
            {a.authorizedKeys.map((k) => (
              <SettingRow key={k.rawLine} label={k.comment || k.shortPreview} directive={`${k.keyType} ${k.shortPreview}`}>
                <div className="flex items-center gap-2">
                  {appKey === k.publicKeyBody && a.username === ctx.loginUser && <StatusBadge tone="ok">{t.vpsUsersKeyInUse}</StatusBadge>}
                  {!isRoot && (
                    <ConfirmButton
                      label={t.vpsUsersRevoke}
                      confirmLabel={t.vpsUsersRevokeConfirm}
                      cancelLabel={t.vpsCommonCancel}
                      onConfirm={() => act("vps_revoke_ssh_key", { username: a.username, keyBody: k.publicKeyBody }).then(() => undefined)}
                    />
                  )}
                </div>
              </SettingRow>
            ))}
            {a.keysFile && <p className="px-4 py-2 font-mono text-xs text-muted-foreground break-all">{a.keysFile}</p>}
            {!isRoot && !isWindows && (
              <SettingRow label={t.vpsUsersGrant} description={sshKeys.length === 0 ? t.vpsUsersNoLocalKeys : undefined}>
                {sshKeys.length > 0 && (
                  <div className="flex gap-2">
                    <SelectField
                      size="sm"
                      className="w-full sm:w-48"
                      value={draft.keyId}
                      placeholder={t.vpsUsersGrant}
                      options={sshKeys.map((k) => ({ value: k.id, label: k.name }))}
                      onChange={(keyId) => setDraft({ keyId })}
                    />
                    <button
                      type="button"
                      disabled={!grantKey}
                      className={buttonClass}
                      onClick={() =>
                        grantKey &&
                        ask({ label: `${a.username} · ${t.vpsUsersKeysTitle}`, from: "", to: grantKey.name }, (pw) =>
                          invoke<AccessKeysSnapshot>("vps_authorize_ssh_key", { hostId, username: a.username, keyLine: grantKey.publicKey, sudoPassword: pw }),
                        )
                      }
                    >
                      {t.vpsUsersGrantBtn}
                    </button>
                  </div>
                )}
              </SettingRow>
            )}
          </SettingGroup>
        );
      })}
      </fieldset>

      <ChangePreviewDialog
        t={t}
        open={!!pending}
        changes={pending ? [pending.change] : []}
        blockedReason={blocked}
        applying={applying}
        onCancel={() => setPending(null)}
        onApply={apply}
      />
    </div>
  );
};
