import type { McpAppId } from "@/types/mcp.ts";

export interface McpAppVisual {
  label: string;
  short: string;
  activeClass: string;
  badgeClass: string;
}

export const MCP_APP_VISUAL: Record<McpAppId, McpAppVisual> = {
  claude: {
    label: "Claude",
    short: "CL",
    activeClass:
      "bg-orange-500/10 ring-1 ring-orange-500/20 hover:bg-orange-500/20 text-orange-600 dark:text-orange-400",
    badgeClass:
      "bg-orange-500/10 text-orange-700 dark:text-orange-300 hover:bg-orange-500/20 border-0 gap-1.5",
  },
  cursor: {
    label: "Cursor",
    short: "CU",
    activeClass:
      "bg-sky-500/10 ring-1 ring-sky-500/20 hover:bg-sky-500/20 text-sky-600 dark:text-sky-400",
    badgeClass:
      "bg-sky-500/10 text-sky-700 dark:text-sky-300 hover:bg-sky-500/20 border-0 gap-1.5",
  },
  codex: {
    label: "Codex",
    short: "CX",
    activeClass:
      "bg-green-500/10 ring-1 ring-green-500/20 hover:bg-green-500/20 text-green-600 dark:text-green-400",
    badgeClass:
      "bg-green-500/10 text-green-700 dark:text-green-300 hover:bg-green-500/20 border-0 gap-1.5",
  },
  gemini: {
    label: "Gemini",
    short: "GM",
    activeClass:
      "bg-blue-500/10 ring-1 ring-blue-500/20 hover:bg-blue-500/20 text-blue-600 dark:text-blue-400",
    badgeClass:
      "bg-blue-500/10 text-blue-700 dark:text-blue-300 hover:bg-blue-500/20 border-0 gap-1.5",
  },
  grokbuild: {
    label: "Grok",
    short: "GK",
    activeClass:
      "bg-cyan-500/10 ring-1 ring-cyan-500/20 hover:bg-cyan-500/20 text-cyan-700 dark:text-cyan-300",
    badgeClass:
      "bg-cyan-500/10 text-cyan-700 dark:text-cyan-300 hover:bg-cyan-500/20 border-0 gap-1.5",
  },
  opencode: {
    label: "OpenCode",
    short: "OC",
    activeClass:
      "bg-violet-500/10 ring-1 ring-violet-500/20 hover:bg-violet-500/20 text-violet-600 dark:text-violet-400",
    badgeClass:
      "bg-violet-500/10 text-violet-700 dark:text-violet-300 hover:bg-violet-500/20 border-0 gap-1.5",
  },
  hermes: {
    label: "Hermes",
    short: "HM",
    activeClass:
      "bg-amber-500/10 ring-1 ring-amber-500/20 hover:bg-amber-500/20 text-amber-700 dark:text-amber-300",
    badgeClass:
      "bg-amber-500/10 text-amber-700 dark:text-amber-300 hover:bg-amber-500/20 border-0 gap-1.5",
  },
};
