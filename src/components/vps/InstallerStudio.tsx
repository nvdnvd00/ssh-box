import React, { useEffect, useRef, useState } from "react";
import { Channel, invoke } from "@tauri-apps/api/core";
import { Package, Terminal, X } from "lucide-react";
import { getI18n } from "../../i18n";
import { HostOs, InstallAction, InstallerScan, SearchHit, ToolView } from "../../types";
import { StatusBadge } from "./shared/StatusBadge";
import { ConfirmButton } from "./shared/ConfirmButton";
import { Notice } from "./shared/Notice";
import { parseVpsError, vpsErrorText } from "./shared/errors";
import { useVpsResource, VpsCtx } from "./shared/useVpsResource";
import { LOGOS } from "./installerLogos";

const CATEGORIES = ["security", "apps", "ai", "network", "utils"];
type Tab = "missing" | "installed" | "attention";
type RunStatus = "waiting" | "running" | "done" | "failed";
interface BatchItem { label: string; action: InstallAction; status: RunStatus; output: string }
interface HostJob { batch: BatchItem[]; busy: boolean; startedAt: number; elapsed: number }

// Keep running jobs across tab switches so leaving and returning never hides an active install or re-enables Install.
const JOBS = new Map<string, HostJob>();
const LISTENERS = new Set<() => void>();
const getJob = (hostId: string): HostJob => JOBS.get(hostId) ?? { batch: [], busy: false, startedAt: 0, elapsed: 0 };
const setJob = (hostId: string, fn: (j: HostJob) => HostJob) => {
  JOBS.set(hostId, fn(getJob(hostId)));
  LISTENERS.forEach((l) => l());
};

const stepPct = (b: BatchItem): number => {
  if (b.status === "done") return 100;
  if (b.status === "waiting") return 0;
  const out = b.output;
  if (!out) return b.status === "running" ? 5 : 0;
  const pcts = [...out.matchAll(/(\d{1,3})%/g)].map((m) => Number(m[1])).filter((n) => n <= 100);
  if (pcts.length) return Math.min(99, Math.max(8, pcts[pcts.length - 1]));
  if (out.includes("Processing triggers")) return 95;
  const setups = (out.match(/Setting up /g) ?? []).length;
  if (setups) return Math.min(92, 75 + setups * 2);
  const unpacks = (out.match(/Unpacking |Preparing to unpack/g) ?? []).length;
  if (unpacks) return Math.min(74, 45 + unpacks);
  if (/Fetched |Building dependency tree|Reading state information/.test(out)) return 40;
  const gets = (out.match(/Get:|Hit:/g) ?? []).length;
  if (gets) return Math.min(35, 12 + gets);
  const lines = out.trim().split("\n").length;
  return Math.min(90, 8 + lines * 2);
};

const lastLine = (s: string): string => {
  const lines = s.split(/[\r\n]+/).map((l) => l.trim()).filter(Boolean);
  return lines[lines.length - 1] ?? "";
};

const fmtTime = (secs: number): string =>
  `${String(Math.floor(secs / 60)).padStart(2, "0")}:${String(secs % 60).padStart(2, "0")}`;

const BTN = "shrink-0 px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 transition-all cursor-pointer disabled:cursor-not-allowed disabled:opacity-50";
const PRIMARY = "px-3 py-1.5 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:opacity-90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50";
const STATUS_TONE = { waiting: "neutral", running: "warn", done: "ok", failed: "danger" } as const;

// Brand tile with a black or white glyph, whichever reads better on the brand colour.
const Logo: React.FC<{ id: string }> = ({ id }) => {
  const logo = LOGOS[id];
  if (!logo) return <span className="grid size-8 shrink-0 place-items-center rounded-lg bg-muted text-muted-foreground"><Package className="size-4" /></span>;
  const n = parseInt(logo.hex, 16);
  const light = 0.299 * (n >> 16) + 0.587 * ((n >> 8) & 255) + 0.114 * (n & 255) > 160;
  return (
    <span className="grid size-8 shrink-0 place-items-center rounded-lg" style={{ background: `#${logo.hex}` }}>
      <svg viewBox="0 0 24 24" className="size-4" fill={light ? "#000" : "#fff"} aria-hidden="true"><path d={logo.path} /></svg>
    </span>
  );
};

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
  const [, forceRender] = useState(0);
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchHit[]>([]);
  const termRef = useRef<HTMLPreElement | null>(null);
  const panelRef = useRef<HTMLElement | null>(null);
  const { batch, busy, startedAt, elapsed } = getJob(hostId);
  const scan = res.data;
  const present = scan?.managers.filter((m) => m.present).map((m) => m.id) ?? [];
  const missingMgrs = scan?.managers.filter((m) => !m.present).map((m) => m.id) ?? [];

  useEffect(() => {
    const l = () => forceRender((n) => n + 1);
    LISTENERS.add(l);
    return () => { LISTENERS.delete(l); };
  }, []);

  useEffect(() => {
    if (!busy || !startedAt) return;
    const id = window.setInterval(() => {
      setJob(hostId, (j) => ({ ...j, elapsed: Math.max(0, Math.floor((Date.now() - j.startedAt) / 1000)) }));
    }, 1000);
    return () => window.clearInterval(id);
  }, [busy, startedAt, hostId]);

  const totalOutLen = batch.reduce((n, b) => n + b.output.length, 0);
  useEffect(() => {
    if (termRef.current) termRef.current.scrollTop = termRef.current.scrollHeight;
  }, [totalOutLen, batch.length]);

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
    if (getJob(hostId).busy) return;
    const items: BatchItem[] = actions.map((a) => ({ label: verb(a), action: a, status: "waiting", output: "" }));
    setJob(hostId, () => ({ batch: items, busy: true, startedAt: Date.now(), elapsed: 0 }));
    window.setTimeout(() => panelRef.current?.scrollIntoView({ behavior: "smooth", block: "nearest" }), 50);
    const patch = (i: number, fn: (b: BatchItem) => BatchItem) =>
      setJob(hostId, (j) => ({ ...j, batch: j.batch.map((x, k) => (k === i ? fn(x) : x)) }));
    let stopped = false;
    for (let i = 0; i < items.length; i++) {
      patch(i, (b) => ({ ...b, status: "running" }));
      const onOutput = new Channel<string>();
      onOutput.onmessage = (chunk) => patch(i, (b) => ({ ...b, output: b.output + chunk }));
      try {
        const output = await invoke<string>("installer_run", {
          hostId,
          os,
          action: items[i].action,
          sudoPassword: vps.sudoPassword ?? null,
          onOutput,
        });
        patch(i, (b) => ({ ...b, status: "done", output: b.output || output }));
      } catch (e) {
        if (["SUDO_REQUIRED", "SUDO_DENIED"].includes(parseVpsError(e).code ?? "")) {
          patch(i, (b) => ({ ...b, status: "waiting" }));
          vps.requestSudo();
          stopped = true;
          break;
        }
        const errText = vpsErrorText(t, e);
        patch(i, (b) => ({ ...b, status: "failed", output: b.output ? `${b.output}\n${errText}` : errText }));
        stopped = true;
        break;
      }
    }
    setJob(hostId, (j) => ({ ...j, busy: false }));
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
          <Logo id={tl.id} />
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
  const pct = batch.length ? Math.round(batch.reduce((s, b) => s + stepPct(b), 0) / batch.length) : 0;
  const activeIdx = batch.findIndex((b) => b.status === "running");
  const failedItem = batch.find((b) => b.status === "failed");
  const currentItem = activeIdx >= 0 ? batch[activeIdx] : failedItem ?? batch[batch.length - 1];
  const overallStatus: RunStatus = busy ? "running" : failedItem ? "failed" : "done";
  const liveLine = currentItem ? lastLine(currentItem.output) : "";

  return (
    <div className="space-y-5 pb-16">
      {res.error && <Notice tone="danger">{res.error}</Notice>}
      {hostId === "local" && os === "windows" && <Notice tone="warn">{t.instWindowsPopupNote}</Notice>}

      {batch.length > 0 && (
        <section ref={panelRef} className="rounded-2xl border border-border/80 bg-card/90 overflow-hidden shadow-sm">
          <div className="px-4 py-3 border-b border-border/60 space-y-2.5">
            <div className="flex flex-wrap items-center justify-between gap-2">
              <div className="flex items-center gap-2 min-w-0">
                <Terminal className="size-4 shrink-0 text-primary" />
                <span className="font-mono text-xs font-medium text-foreground truncate">
                  {activeIdx >= 0 ? `[${activeIdx + 1}/${batch.length}] ${currentItem?.label}` : currentItem?.label}
                </span>
              </div>
              <div className="flex items-center gap-2 shrink-0">
                <span className="font-mono text-xs text-muted-foreground">{fmtTime(elapsed)}</span>
                <span className="font-mono text-xs font-semibold text-foreground">{pct}%</span>
                <StatusBadge tone={STATUS_TONE[overallStatus]}>{t.instStatus[overallStatus]}</StatusBadge>
                {!busy && (
                  <button
                    type="button"
                    onClick={() => setJob(hostId, () => ({ batch: [], busy: false, startedAt: 0, elapsed: 0 }))}
                    className="p-1 rounded-lg text-muted-foreground hover:text-foreground hover:bg-accent/70 cursor-pointer"
                  >
                    <X className="size-3.5" />
                  </button>
                )}
              </div>
            </div>

            <div className="h-2 w-full rounded-full bg-muted overflow-hidden">
              <div
                className={`h-full transition-all duration-300 ${
                  overallStatus === "failed" ? "bg-destructive" : overallStatus === "done" ? "bg-emerald-500" : "bg-primary"
                }`}
                style={{ width: `${pct}%` }}
              />
            </div>

            <p className="font-mono text-[11px] text-muted-foreground truncate">
              {liveLine || (busy ? `${t.instStatus.running}…` : t.instStatus[overallStatus])}
            </p>
          </div>

          <pre
            ref={termRef}
            className="bg-zinc-950 text-zinc-100 font-mono text-xs p-3.5 max-h-72 overflow-y-auto whitespace-pre-wrap break-all leading-relaxed"
          >
            {batch.map((b, i) => (
              <div key={i} className={i > 0 ? "mt-3 pt-2 border-t border-zinc-800" : ""}>
                <div className="text-emerald-400 font-semibold">
                  $ [{i + 1}/{batch.length}] {b.label} ({t.instStatus[b.status]})
                </div>
                {b.output ? (
                  <div className="mt-1 text-zinc-300">{b.output}</div>
                ) : b.status === "running" ? (
                  <div className="mt-1 text-zinc-500 animate-pulse">{t.instStatus.running}…</div>
                ) : null}
              </div>
            ))}
          </pre>
        </section>
      )}

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
                <Logo id="" />
                <span className="min-w-0">
                  <span className="block font-mono text-sm text-foreground break-all">{h.package}</span>
                  <span className="block text-xs text-muted-foreground">{h.description}</span>
                </span>
              </label>
            )),
          ),
        )}
      </section>

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
            <label className="min-w-0 flex items-center gap-3">
              {!m.present && <input type="checkbox" disabled={busy} checked={sel.has(`mgr:${m.id}`)} onChange={() => toggle(`mgr:${m.id}`)} />}
              <Logo id={m.id} />
              <span className="min-w-0">
                <span className="block text-sm font-medium text-foreground">{m.id}</span>
                {t.instMgrDesc[m.id] && <span className="block text-xs text-muted-foreground">{t.instMgrDesc[m.id]}</span>}
              </span>
            </label>
            {m.present ? (
              <StatusBadge tone="ok">{t.instTabInstalled}</StatusBadge>
            ) : (
              <button
                type="button"
                className={BTN}
                disabled={busy}
                onClick={() =>
                  runBatch(os === "linux" ? [{ kind: "refresh" }, { kind: "installManager", manager: m.id }] : [{ kind: "installManager", manager: m.id }])
                }
              >
                {t.instInstall}
              </button>
            )}
            {m.id === "brew" && !m.present && <p className="w-full text-xs text-muted-foreground">{t.instBrewAdminNote}</p>}
          </div>
        )),
      )}
      {CATEGORIES.map((c) => group(c, scan.tools.filter((tl) => tl.category === c && inTab(tl)).map(toolRow)))}

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
