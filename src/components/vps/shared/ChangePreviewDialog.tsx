import React from "react";
import { AppI18nStrings } from "../../../i18n";

export interface PendingChange {
  label: string;
  from: string;
  to: string;
}

export const ChangePreviewDialog: React.FC<{
  t: AppI18nStrings;
  open: boolean;
  changes: PendingChange[];
  blockedReason?: string | null;
  applying: boolean;
  onCancel: () => void;
  onApply: () => void;
}> = ({ t, open, changes, blockedReason, applying, onCancel, onApply }) => {
  if (!open) return null;
  return (
    <div className="ui-modal-backdrop fixed inset-0 z-50 flex items-center justify-center bg-black/70 backdrop-blur-xs p-4">
      <div className="ui-modal-content w-full max-w-lg max-h-[85vh] flex flex-col rounded-2xl bg-popover border border-border shadow-2xl">
        <div className="px-5 py-3.5 border-b border-border">
          <h3 className="text-sm font-semibold text-foreground">{t.vpsCommonPreviewTitle}</h3>
        </div>
        <div className="px-5 py-4 space-y-3 overflow-y-auto">
          <ul className="divide-y divide-border/60 rounded-xl border border-border/80">
            {changes.map((c) => (
              <li key={c.label} className="px-3 py-2 space-y-0.5">
                <div className="text-xs font-medium text-foreground">{c.label}</div>
                <div className="font-mono text-xs text-muted-foreground break-all">
                  {c.from || "—"} → <span className="text-foreground">{c.to || "—"}</span>
                </div>
              </li>
            ))}
          </ul>
          {blockedReason && (
            <div className="rounded-xl border border-danger/40 bg-danger/10 px-3 py-2 text-xs text-danger">
              {blockedReason}
            </div>
          )}
        </div>
        <div className="px-5 py-3 border-t border-border flex justify-end gap-2">
          <button
            type="button"
            disabled={applying}
            onClick={onCancel}
            className="px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 cursor-pointer"
          >
            {t.vpsCommonCancel}
          </button>
          <button
            type="button"
            disabled={applying || !!blockedReason}
            onClick={onApply}
            className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-primary text-primary-foreground hover:opacity-90 cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
          >
            {t.vpsCommonApply}
          </button>
        </div>
      </div>
    </div>
  );
};
