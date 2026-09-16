import type { ReactNode } from "react";

export function ListItemRow({
  isLast,
  children,
}: {
  isLast?: boolean;
  children: ReactNode;
}) {
  return (
    <div
      className={`group flex items-center gap-3 px-4 py-2.5 hover:bg-muted/50 transition-colors ${
        !isLast ? "border-b border-border-default" : ""
      }`}
    >
      {children}
    </div>
  );
}
