import React, { useEffect, useState } from "react";

// Two-step inline confirm, same pattern as "Reset all settings" in SettingsWorkspace.
export const ConfirmButton: React.FC<{
  label: string;
  confirmLabel: string;
  cancelLabel: string;
  onConfirm: () => void | Promise<void>;
  disabled?: boolean;
}> = ({ label, confirmLabel, cancelLabel, onConfirm, disabled }) => {
  const [armed, setArmed] = useState(false);
  const [busy, setBusy] = useState(false);

  useEffect(() => {
    if (!armed || busy) return;
    const id = window.setTimeout(() => setArmed(false), 5000);
    return () => window.clearTimeout(id);
  }, [armed, busy]);

  if (!armed) {
    return (
      <button
        type="button"
        disabled={disabled}
        onClick={() => setArmed(true)}
        className="shrink-0 px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 transition-all cursor-pointer disabled:cursor-not-allowed disabled:opacity-50"
      >
        {label}
      </button>
    );
  }
  return (
    <div className="flex items-center gap-2 shrink-0">
      <button
        type="button"
        disabled={busy}
        onClick={() => setArmed(false)}
        className="px-3 py-1.5 rounded-xl text-xs font-medium text-muted-foreground hover:text-foreground hover:bg-accent/70 border border-border/60 transition-all cursor-pointer"
      >
        {cancelLabel}
      </button>
      <button
        type="button"
        disabled={busy}
        onClick={async () => {
          setBusy(true);
          try {
            await onConfirm();
          } finally {
            setBusy(false);
            setArmed(false);
          }
        }}
        className="px-3 py-1.5 rounded-xl text-xs font-semibold bg-danger/15 text-danger border border-danger/40 hover:bg-danger/25 transition-all cursor-pointer disabled:opacity-50"
      >
        {confirmLabel}
      </button>
    </div>
  );
};
