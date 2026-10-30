import React, { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { getI18n } from "../../i18n";
import { HostOs, InstallAction, InstallerScan, SearchHit, ToolView } from "../../types";
import { StatusBadge } from "./shared/StatusBadge";
import { ConfirmButton } from "./shared/ConfirmButton";
import { Notice } from "./shared/Notice";
import { parseVpsError, vpsErrorText } from "./shared/errors";
import { useVpsResource, VpsCtx } from "./shared/useVpsResource";

const CATEGORIES = ["security", "apps", "ai", "network", "utils"];
type Tab = "missing" | "installed" | "attention";
type RunStatus = "waiting" | "running" | "done" | "failed";
interface BatchItem { label: string; action: InstallAction; status: RunStatus; output: string }

const BTN = "shrink-0 px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 transition-all cursor-pointer disabled:cursor-not-allowed disabled:opacity-50";
const PRIMARY = "px-3 py-1.5 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:opacity-90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50";
const STATUS_TONE = { waiting: "neutral", running: "warn", done: "ok", failed: "danger" } as const;

const needsAttention = (tool: ToolView) =>
  tool.status.kind !== "notInstalled" && !!tool.serviceState && !(tool.serviceState.enabled && tool.serviceState.active);

export const InstallerStudio: React.FC<{ vps: VpsCtx; os: HostOs }> = ({ vps, os }) => {
  const { t, hostId } = vps;
  const en = t === getI18n("en");
  const res = useVpsResource<InstallerScan>(vps, () => invoke<InstallerScan>("installer_scan", { hostId, os }));
  const [tab, setTab] = useState<Tab>("missing");
  const [sel, setSel] = useState<Set<string>>(new Set());
  const [hits, setHits] = useState<Record<string, SearchHit>>({});
  const [via, setVia] = useState<Record<string, string>>({});
  const [batch, setBatch] = useState<BatchItem[]>([]);
  const [busy, setBusy] = useState(false);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const scan = res.data;
  const present = scan?.managers.filter((m) => m.present).map((m) => m.id) ?? [];
  const missingMgrs = scan?.managers.filter((m) => !m.present).map((m) => m.id) ?? [];

  useEffect(() => {
    const q = query.trim();
    if (q.length < 2 || !scan) return setResults([]);
    const id = window.setTimeout(() => {
      invoke<SearchHit[]>("installer_search", { hostId, os, present, query: q })
        .then(setResults)
        .catch((e) => res.setError(vpsErrorText(t, e)));
    }, 400);
    return () => window.clearTimeout(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [query, scan]);

  const toggle = (key: string, on?: boolean) =>
    setSel((s) => {
      const n = new Set(s);
      if (on ?? !n.has(key)) n.add(key);
      else {
        n.delete(key);
        if (key.startsWith("mgr:")) scan?.tools.forEach((tl) => tl.blockedBy === key.slice(4) && n.delete(`tool:${tl.id}`));
      }
      return n;
    });
  const pickHit = (h: SearchHit) => {
    const key = `hit:${h.manager}:${h.package}`;
    setHits((m) => ({ ...m, [key]: h }));
    toggle(key);
  };

  const verb = (a: InstallAction): string => {
    switch (a.kind) {
      case "install": return `${a.manager}: ${t.instInstall.toLowerCase()} ${a.package}`;
      case "uninstall": return `${a.manager}: ${t.instUninstall.toLowerCase()} ${a.package}`;
      case "enableService": return `${t.instEnable} ${a.service}`;
      case "installManager": return `${t.instInstall} ${a.manager}`;
      case "refresh": return t.instRefresh;
    }
  };

  // Selection → ordered actions: Refresh (Linux) → managers → tools/search hits. Tools whose manager isn't ticked are dropped.
  const buildActions = (): InstallAction[] => {
    if (!scan) return [];
    const mgrs = missingMgrs.filter((m) => sel.has(`mgr:${m}`));
    const tools = scan.tools
      .filter((tl) => sel.has(`tool:${tl.id}`) && tl.install && (!tl.blockedBy || mgrs.includes(tl.blockedBy)))
      .map((tl): InstallAction => ({ kind: "install", manager: tl.install!.manager, package: tl.install!.package }));
    const extra = Object.entries(hits)
      .filter(([k]) => sel.has(k))
      .map(([, h]): InstallAction => ({ kind: "install", manager: h.manager, package: h.package }));
    const body = [...mgrs.map((m): InstallAction => ({ kind: "installManager", manager: m })), ...tools, ...extra];
    return body.length && os === "linux" ? [{ kind: "refresh" }, ...body] : body;
  };
  const count = buildActions().filter((a) => a.kind !== "refresh").length;

  const runBatch = async (actions: InstallAction[], clearSelection = false) => {
    const items: BatchItem[] = actions.map((a) => ({ label: verb(a), action: a, status: "waiting", output: "" }));
    setBatch(items);
    setBusy(true);
    const patch = (i: number, p: Partial<BatchItem>) => setBatch((b) => b.map((x, j) => (j === i ? { ...x, ...p } : x)));
    let stopped = false;
    for (let i = 0; i < items.length; i++) {
      patch(i, { status: "running" });
      try {
        const output = await invoke<string>("installer_run", { hostId, os, action: items[i].action, sudoPassword: vps.sudoPassword ?? null });
        patch(i, { status: "done", output });
      } catch (e) {
        if (parseVpsError(e).code === "SUDO_REQUIRED") {
          patch(i, { status: "waiting" });
          vps.requestSudo();
          stopped = true;
          break;
        }
        patch(i, { status: "failed", output: vpsErrorText(t, e) });
      }
    }
    setBusy(false);
    if (clearSelection && !stopped) setSel(new Set());
    await res.refresh();
  };

  if (res.loading && !scan) return <p className="text-sm text-muted-foreground">{t.vpsCommonLoading}</p>;
  if (!scan) return res.error ? <Notice tone="danger">{res.error}</Notice> : null;

  const inTab = (tl: ToolView) =>
    tab === "missing" ? tl.status.kind === "notInstalled" : tab === "installed" ? tl.status.kind !== "notInstalled" : needsAttention(tl);
  const tabs: [Tab, string, number][] = [
    ["missing", t.instTabMissing, scan.tools.filter((tl) => tl.status.kind === "notInstalled").length + missingMgrs.length],
    ["installed", t.instTabInstalled, scan.tools.filter((tl) => tl.status.kind !== "notInstalled").length + present.length],
    ["attention", t.instTabAttention, scan.tools.filter(needsAttention).length],
  ];
  const mgrRows = tab === "attention" ? [] : scan.managers.filter((m) => m.present === (tab === "installed"));

  const toolRow = (tl: ToolView) => {
    const warn = needsAttention(tl);
    const vias = tl.status.kind === "installed" ? tl.status.via : [];
    const chosen = vias.find((o) => o.manager === via[tl.id]) ?? vias[0];
    return (
      <div key={tl.id} className={`flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between ${warn ? "bg-warning/5" : ""}`}>
        <label className="min-w-0 flex items-start gap-3">
          {tl.status.kind === "notInstalled" && (
            <input
              type="checkbox"
              className="mt-1"
              disabled={busy || !tl.install || (!!tl.blockedBy && !sel.has(`mgr:${tl.blockedBy}`))}
              checked={sel.has(`tool:${tl.id}`)}
              onChange={() => toggle(`tool:${tl.id}`)}
            />
          )}
          <span className="min-w-0 space-y-0.5">
            <span className="flex flex-wrap items-center gap-2 text-sm font-medium text-foreground">
              {tl.name}
              {tl.recommended && tl.status.kind === "notInstalled" && <StatusBadge tone="ok">{t.instRecommended}</StatusBadge>}
              {warn && <StatusBadge tone="warn">{t.instNotRunning}</StatusBadge>}
              {vias.length > 0 && <span className="text-xs text-muted-foreground">· {vias.map((o) => o.manager).join(", ")}</span>}
              {tl.status.kind === "external" && <span className="text-xs text-muted-foreground">· {t.instExternal}</span>}
            </span>
            <span className="block text-xs text-muted-foreground">{en ? tl.descriptionEn : tl.descriptionVi}</span>
          </span>
        </label>
        <div className="flex flex-wrap items-center gap-2 sm:justify-end">
          {tl.blockedBy && tl.status.kind === "notInstalled" && !sel.has(`mgr:${tl.blockedBy}`) && (
            <button type="button" className={BTN} disabled={busy} onClick={() => toggle(`mgr:${tl.blockedBy}`, true)}>
              {t.instNeedsManager(tl.blockedBy)}
            </button>
          )}
          {warn && tl.service && (
            <button type="button" className={PRIMARY} disabled={busy} onClick={() => runBatch([{ kind: "enableService", service: tl.service! }])}>
              {t.instEnable}
            </button>
          )}
          {vias.length > 1 && (
            <select className="ui-input text-xs w-auto" value={chosen?.manager} onChange={(e) => setVia({ ...via, [tl.id]: e.target.value })}>
              {vias.map((o) => <option key={o.manager}>{o.manager}</option>)}
            </select>
          )}
          {chosen && (
            <ConfirmButton
              label={t.instUninstall}
              confirmLabel={t.instUninstallConfirm(tl.name, chosen.manager)}
              cancelLabel={t.vpsCommonCancel}
              disabled={busy}
              onConfirm={() => runBatch([{ kind: "uninstall", manager: chosen.manager, package: chosen.package }])}
            />
          )}
        </div>
      </div>
    );
  };

  const group = (key: string, rows: React.ReactNode[]) =>
    rows.length > 0 && (
      <section key={key} className="space-y-2">
        <h2 className="text-sm font-semibold text-foreground px-1">{t.instCat[key] ?? key}</h2>
        <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60">{rows}</div>
      </section>
    );

  const hitGroups = results.reduce<Record<string, SearchHit[]>>((g, h) => ({ ...g, [h.manager]: [...(g[h.manager] ?? []), h] }), {});

  return (
    <div className="space-y-5 pb-16">
      {res.error && <Notice tone="danger">{res.error}</Notice>}
      {hostId === "local" && os === "windows" && <Notice tone="warn">{t.instWindowsPopupNote}</Notice>}

      <div className="flex flex-wrap gap-2">
        {tabs.map(([id, label, n]) => (
          <button
            key={id}
            type="button"
            onClick={() => setTab(id)}
            className={`px-3 py-1.5 rounded-xl text-xs font-medium border transition-all cursor-pointer ${tab === id ? "bg-primary text-primary-foreground border-primary" : "text-muted-foreground border-border/60 hover:bg-accent/70"}`}
          >
            {label} ({n})
          </button>
        ))}
      </div>

      {group(
        "managers",
        mgrRows.map((m) => (
          <div key={m.id} className="flex flex-wrap items-center justify-between gap-2 px-4 py-3">
            <label className="flex items-center gap-3 text-sm font-medium text-foreground">
              {!m.present && <input type="checkbox" disabled={busy} checked={sel.has(`mgr:${m.id}`)} onChange={() => toggle(`mgr:${m.id}`)} />}
              {m.id}
            </label>
            {m.present ? (
              <StatusBadge tone="ok">{t.instTabInstalled}</StatusBadge>
            ) : (
              <button type="button" className={BTN} disabled={busy} onClick={() => runBatch([{ kind: "installManager", manager: m.id }])}>
                {t.instInstall}
              </button>
            )}
          </div>
        )),
      )}
      {CATEGORIES.map((c) => group(c, scan.tools.filter((tl) => tl.category === c && inTab(tl)).map(toolRow)))}

      <section className="space-y-2">
        <input
          className="ui-input"
          value={query}
          placeholder={t.instSearchPlaceholder.replace("…", present.join(", "))}
          onChange={(e) => setQuery(e.target.value)}
        />
        {query.trim().length >= 2 && missingMgrs.length > 0 && (
          <p className="px-1 text-xs text-muted-foreground">{t.instSearchSkipped(missingMgrs.join(", "))}</p>
        )}
        {Object.entries(hitGroups).map(([m, list]) =>
          group(
            m,
            list.map((h) => (
              <label key={h.package} className="flex items-start gap-3 px-4 py-2">
                <input type="checkbox" className="mt-1" disabled={busy} checked={sel.has(`hit:${m}:${h.package}`)} onChange={() => pickHit(h)} />
                <span className="min-w-0">
                  <span className="block font-mono text-sm text-foreground break-all">{h.package}</span>
                  <span className="block text-xs text-muted-foreground">{h.description}</span>
                </span>
              </label>
            )),
          ),
        )}
      </section>

      {batch.length > 0 && (
        <section className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60">
          {batch.map((b, i) => (
            <details key={i} className="px-4 py-2">
              <summary className="flex flex-wrap items-center justify-between gap-2 cursor-pointer text-sm">
                <span className="font-mono text-xs text-foreground break-all">{b.label}</span>
                <StatusBadge tone={STATUS_TONE[b.status]}>{t.instStatus[b.status]}</StatusBadge>
              </summary>
              {b.output && (
                <pre className="mt-2 max-h-64 overflow-auto whitespace-pre-wrap break-all text-xs text-muted-foreground">
                  {b.output.split("\n").slice(-40).join("\n")}
                </pre>
              )}
            </details>
          ))}
        </section>
      )}

      {count > 0 && (
        <div className="sticky bottom-0 flex justify-end rounded-2xl border border-border/80 bg-card/95 px-4 py-3">
          <button type="button" className={PRIMARY} disabled={busy} onClick={() => runBatch(buildActions(), true)}>
            {t.instInstallSelected(count)}
          </button>
        </div>
      )}
    </div>
  );
};
