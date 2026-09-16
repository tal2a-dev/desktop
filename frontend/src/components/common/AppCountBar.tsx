import { Badge, badgeVariants } from "@/components/ui/badge.tsx";
import { MCP_APP_VISUAL } from "@/config/mcpApps.tsx";
import { cn } from "@/lib/utils.ts";
import { MCP_APP_IDS, type McpAppId } from "@/types/mcp.ts";

export function AppCountBar({
  totalLabel,
  counts,
  totalCount,
  onToggleAll,
  pendingApp,
  disabled = false,
}: {
  totalLabel: string;
  counts: Partial<Record<McpAppId, number>>;
  totalCount: number;
  onToggleAll: (app: McpAppId, enabled: boolean) => void | Promise<void>;
  pendingApp?: McpAppId | null;
  disabled?: boolean;
}) {
  const hasPending = pendingApp != null;

  return (
    <div className="mb-4 flex flex-shrink-0 items-center gap-4 rounded-xl border border-white/10 px-4 py-3 glass">
      <Badge
        variant="outline"
        className="h-7 shrink-0 whitespace-nowrap bg-background/50 px-3"
      >
        {totalLabel}
      </Badge>
      <div className="min-w-0 flex-1 overflow-x-auto">
        <div className="ml-auto flex w-max min-w-full items-center justify-end gap-2">
          {MCP_APP_IDS.map((app) => {
            const count = counts[app] ?? 0;
            const allEnabled = totalCount > 0 && count >= totalCount;
            const partiallyEnabled = count > 0 && count < totalCount;
            const pending = pendingApp === app;
            const actionLabel = allEnabled
              ? `Disable all for ${MCP_APP_VISUAL[app].label}`
              : `Enable all for ${MCP_APP_VISUAL[app].label}`;

            return (
              <button
                key={app}
                type="button"
                role="checkbox"
                aria-checked={partiallyEnabled ? "mixed" : allEnabled}
                aria-busy={pending}
                aria-label={actionLabel}
                title={actionLabel}
                disabled={disabled || totalCount === 0 || hasPending}
                onClick={() => void onToggleAll(app, !allEnabled)}
                className={cn(
                  badgeVariants({ variant: "secondary" }),
                  MCP_APP_VISUAL[app].badgeClass,
                  "shrink-0 cursor-pointer select-none whitespace-nowrap disabled:cursor-not-allowed",
                  pending && "cursor-wait",
                )}
              >
                <span className="opacity-75">{MCP_APP_VISUAL[app].label}:</span>
                <span className="ml-1 font-bold">{count}</span>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}
