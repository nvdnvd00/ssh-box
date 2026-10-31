import React, { ReactNode } from "react";

// One setting per row; stacks label above control below 640px.
export const SettingRow: React.FC<{
  id?: string;
  label: string;
  description?: string;
  directive?: string;
  effective?: string;
  children: ReactNode;
}> = ({ id, label, description, directive, effective, children }) => (
  <div id={id} className="flex flex-col gap-2 px-4 py-3 sm:flex-row sm:items-center sm:justify-between">
    <div className="min-w-0 space-y-0.5">
      <div className="text-sm font-medium text-foreground">{label}</div>
      {description && <div className="text-xs text-muted-foreground">{description}</div>}
      {(directive || effective) && (
        <div className="font-mono text-xs text-muted-foreground break-all">
          {directive}
          {directive && effective ? " " : ""}
          {effective}
        </div>
      )}
    </div>
    <div className="shrink-0 sm:max-w-[60%]">{children}</div>
  </div>
);

export const SettingGroup: React.FC<{ title: string; children: ReactNode }> = ({ title, children }) => (
  <section className="space-y-2">
    <h2 className="text-sm font-semibold text-foreground px-1">{title}</h2>
    <div className="rounded-2xl border border-border/80 bg-card/65 divide-y divide-border/60">{children}</div>
  </section>
);
