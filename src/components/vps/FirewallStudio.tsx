import React, { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { FirewallRuleItem, FirewallSnapshot } from "../../types";
import { SelectField } from "../SelectField";
import { Switch } from "./shared/Switch";
import { SettingGroup, SettingRow } from "./shared/SettingRow";
import { StatusBadge, StatusTone } from "./shared/StatusBadge";
import { ConfirmButton } from "./shared/ConfirmButton";
import { ChangePreviewDialog, PendingChange } from "./shared/ChangePreviewDialog";
import { Notice } from "./shared/Notice";
import { parseVpsError, vpsErrorText } from "./shared/errors";
import { useVpsResource, VpsStudioProps } from "./shared/useVpsResource";

const EXPOSURE_TONE: Record<string, StatusTone> = {
  exposed: "warn",
  restricted: "ok",
  blocked: "neutral",
  loopback: "neutral",
  unknown: "neutral",
};

interface Pending {
  change: PendingChange;
  run: (sudoPassword: string | null) => Promise<FirewallSnapshot>;
}

export const FirewallStudio: React.FC<VpsStudioProps> = ({ vps, probe }) => {
  const { t, hostId } = vps;
  // Non-admin Windows cannot read the firewall: the backend refuses, and the notice below already says why.
  const skipRead = probe.support.firewall.reason === "windows_not_admin";
  const res = useVpsResource<FirewallSnapshot | null>(vps, (pw) =>
    skipRead ? Promise.resolve(null) : invoke<FirewallSnapshot>("vps_get_firewall_snapshot", { hostId, sudoPassword: pw ?? null }),
  );
  const [pending, setPending] = useState<Pending | null>(null);
  const [blocked, setBlocked] = useState<string | null>(null);
  const [applying, setApplying] = useState(false);
  const [form, setForm] = useState({ action: "allow", source: "", port: "", proto: "tcp" });

  const engine = probe.firewallEngine;
  const { reason } = probe.support.firewall;
  const writable = probe.support.firewall.level === "full";
  const snap = res.data;
  const anywhere = (src: string) => (/^(any|anywhere|0\.0\.0\.0\/0|::\/0)?$/i.test(src.trim()) ? t.vpsFwAnywhere : src);

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
      setForm((f) => ({ ...f, source: "", port: "" }));
    } catch (e) {
      if (parseVpsError(e).code === "LOCKOUT_BLOCKED") setBlocked(vpsErrorText(t, e));
      else {
        setPending(null);
        res.setError(vpsErrorText(t, e));
      }
    } finally {
      setApplying(false);
    }
  };
  const remove = async (rule: FirewallRuleItem) => {
    await res.mutate((pw) => invoke<FirewallSnapshot>("vps_delete_firewall_rule", { hostId, rule, sudoPassword: pw ?? null }));
  };

  if (res.loading && !snap) return <p className="text-sm text-muted-foreground">{t.vpsCommonLoading}</p>;
  if (skipRead) return <Notice tone="warn">{t.vpsUnsupportedReasons[reason!] ?? reason}</Notice>;
  if (!snap) return res.error ? <Notice tone="danger">{res.error}</Notice> : null;

  // firewalld services/ports are listed once per address family; show each rule once.
  const rules = snap.rules.filter((r, i, all) => all.findIndex((o) => o.rawSpec === r.rawSpec) === i);
  // Windows Firewall only allows or blocks, and every rule needs a port: offer nothing that would always fail.
  const isWindows = engine === "windows";
  const portMissing = isWindows && !form.port.trim();
  const actionOptions = (isWindows ? ["allow", "deny"] : ["allow", "limit", "deny", "reject"]).map((a) => ({ value: a, label: t.vpsFwActions[a] }));
  const protoOptions = [
    { value: "tcp", label: "TCP" },
    { value: "udp", label: "UDP" },
    { value: "any", label: t.vpsFwAny },
  ];
  const describe = (action: string, port: string, proto: string, source: string) =>
    `${t.vpsFwActions[action.toLowerCase()] ?? action} ${port || t.vpsFwAllPorts}${port && proto !== "any" ? `/${proto}` : ""} · ${t.vpsFwFrom.replace("{s}", anywhere(source))}`;

  return (
    <div className="space-y-5">
      {res.error && <Notice tone="danger">{res.error}</Notice>}
      {engine === "none" && <Notice tone="warn">{t.vpsFwNoEngine}</Notice>}
      {reason && <Notice tone="warn">{t.vpsUnsupportedReasons[reason] ?? reason}</Notice>}
      {snap.enabled && !snap.ipv6Filtered && <Notice tone="warn">{t.vpsFwIpv6Off}</Notice>}

      <SettingGroup title={t.vpsFwStatusTitle}>
        <SettingRow label={t.vpsFwEnabled}>
          <Switch
            label={t.vpsFwEnabled}
            checked={snap.enabled}
            disabled={!writable}
            onChange={(enable) =>
              ask(
                { label: t.vpsFwEnabled, from: snap.enabled ? t.vpsFwOn : t.vpsFwOff, to: enable ? t.vpsFwOn : t.vpsFwOff },
                (pw) => invoke<FirewallSnapshot>("vps_set_firewall_state", { hostId, enable, sudoPassword: pw }),
              )
            }
          />
        </SettingRow>
        <SettingRow label={t.vpsFwEngine}>
          <StatusBadge tone={writable ? "ok" : "neutral"}>{engine}</StatusBadge>
        </SettingRow>
      </SettingGroup>

      <SettingGroup title={t.vpsFwPortsTitle}>
        {snap.listeningSockets.length === 0 && <p className="px-4 py-3 text-sm text-muted-foreground">{t.vpsFwNoPorts}</p>}
        {snap.listeningSockets.map((s) => (
          <SettingRow
            key={`${s.protocol}-${s.bindAddr}-${s.port}`}
            label={`${s.port}/${s.protocol}${s.processName ? ` · ${s.processName}` : ""}`}
            directive={s.bindAddr}
          >
            <StatusBadge tone={EXPOSURE_TONE[s.exposure] ?? "neutral"}>{t.vpsFwExposure[s.exposure] ?? s.exposure}</StatusBadge>
          </SettingRow>
        ))}
      </SettingGroup>

      <SettingGroup title={t.vpsFwRulesTitle}>
        {rules.length === 0 && <p className="px-4 py-3 text-sm text-muted-foreground">{t.vpsFwNoRules}</p>}
        {rules.map((r) => (
          <SettingRow
            key={r.rawSpec}
            label={describe(r.action, r.toPort === "Anywhere" ? "" : r.toPort.replace(/\/(tcp|udp)$/, ""), r.protocol, r.fromSource)}
            directive={r.rawSpec}
          >
            {writable && (engine !== "windows" || r.comment === "ssh-box") && (
              <ConfirmButton
                label={t.vpsFwDelete}
                confirmLabel={t.vpsFwDeleteConfirm}
                cancelLabel={t.vpsCommonCancel}
                onConfirm={() => remove(r)}
              />
            )}
          </SettingRow>
        ))}
      </SettingGroup>

      {writable && (
        <SettingGroup title={t.vpsFwAddTitle}>
          <div className="px-4 py-3 grid gap-3 sm:grid-cols-2">
            <label className="space-y-1">
              <span className="ui-label">{t.vpsFwAction}</span>
              <SelectField size="sm" value={form.action} options={actionOptions} onChange={(action) => setForm({ ...form, action })} />
            </label>
            <label className="space-y-1">
              <span className="ui-label">{t.vpsFwProto}</span>
              <SelectField size="sm" value={form.proto} options={protoOptions} onChange={(proto) => setForm({ ...form, proto })} />
            </label>
            <label className="space-y-1">
              <span className="ui-label">{t.vpsFwSource}</span>
              <div className="flex gap-2">
                <input className="ui-input font-mono min-w-0" value={form.source} onChange={(e) => setForm({ ...form, source: e.target.value })} />
                <button
                  type="button"
                  onClick={() => setForm({ ...form, source: probe.context.clientIp })}
                  className="shrink-0 px-3 rounded-xl text-xs border border-border/60 hover:bg-accent/70 cursor-pointer"
                >
                  {t.vpsFwMyIp}
                </button>
              </div>
            </label>
            <label className="space-y-1">
              <span className="ui-label">{isWindows ? t.vpsFwPortOnly : t.vpsFwPort}</span>
              <input className="ui-input font-mono" inputMode="numeric" value={form.port} onChange={(e) => setForm({ ...form, port: e.target.value })} />
              {portMissing && <span className="text-xs text-muted-foreground">{t.vpsUnsupportedReasons.windows_port_required}</span>}
            </label>
            <div className="sm:col-span-2 flex justify-end">
              <button
                type="button"
                disabled={portMissing}
                onClick={() =>
                  ask(
                    { label: t.vpsFwAddTitle, from: "", to: describe(form.action, form.port, form.proto, form.source) },
                    (pw) =>
                      invoke<FirewallSnapshot>("vps_add_firewall_rule", {
                        hostId, action: form.action, source: form.source || "any", port: form.port || "any", proto: form.proto, sudoPassword: pw,
                      }),
                  )
                }
                className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:opacity-90 cursor-pointer disabled:opacity-50 disabled:cursor-not-allowed"
              >
                {t.vpsFwAdd}
              </button>
            </div>
          </div>
        </SettingGroup>
      )}

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
