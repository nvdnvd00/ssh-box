import React, { ReactNode } from "react";

const TONE = {
  warn: "border-warning/40 bg-warning/10 text-warning",
  danger: "border-danger/40 bg-danger/10 text-danger",
};

export const Notice: React.FC<{ tone: keyof typeof TONE; children: ReactNode }> = ({ tone, children }) => (
  <div className={`rounded-xl border px-4 py-3 text-sm ${TONE[tone]}`}>{children}</div>
);
