import { AgentIcon } from "@/components/AgentIcon.tsx";
import { MCP_APP_VISUAL } from "@/config/mcpApps.tsx";
import { MCP_APP_IDS, type McpAppId, type McpApps } from "@/types/mcp.ts";

export function AppToggleGroup({
  apps,
  onToggle,
  disabled = false,
}: {
  apps: Partial<McpApps>;
  onToggle: (app: McpAppId, enabled: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <div className="flex flex-shrink-0 items-center gap-1.5">
      {MCP_APP_IDS.map((app) => {
        const { label, activeClass } = MCP_APP_VISUAL[app];
        const enabled = Boolean(apps[app]);
        return (
          <button
            key={app}
            type="button"
            onClick={() => onToggle(app, !enabled)}
            disabled={disabled}
            aria-label={label}
            aria-pressed={enabled}
            title={`${label}${enabled ? " on" : " off"}`}
            className={`flex h-7 w-7 items-center justify-center rounded-lg text-[10px] font-semibold transition-all ${
              enabled ? activeClass : "opacity-35 hover:opacity-70"
            } disabled:cursor-not-allowed`}
          >
            <AgentIcon id={app} name={label} size={14} />
          </button>
        );
      })}
    </div>
  );
}
