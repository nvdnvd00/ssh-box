import React, { ReactNode } from "react";

export type StatusTone = "ok" | "warn" | "danger" | "neutral";

// The only place that maps status to color (tokens --success/--warning/--danger per theme).
const TONE: Record<StatusTone, string> = {
  ok: "text-success border-success/40 bg-success/10",
  warn: "text-warning border-warning/40 bg-warning/10",
  danger: "text-danger border-danger/40 bg-danger/10",
  neutral: "text-muted-foreground border-border bg-secondary/60",
};

export const StatusBadge: React.FC<{ tone: StatusTone; children: ReactNode }> = ({ tone, children }) => (
  <span className={`inline-flex items-center gap-1 rounded-lg border px-2 py-0.5 text-xs font-medium ${TONE[tone]}`}>
    {children}
  </span>
);
