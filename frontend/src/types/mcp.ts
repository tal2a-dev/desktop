export interface McpServerSpec {
  type?: "stdio" | "http" | "sse";
  command?: string;
  args?: string[];
  env?: Record<string, string>;
  cwd?: string;
  url?: string;
  headers?: Record<string, string>;
  [key: string]: unknown;
}

export type McpAppId =
  | "claude"
  | "cursor"
  | "codex"
  | "gemini"
  | "grokbuild"
  | "opencode"
  | "hermes";

export interface McpApps {
  claude: boolean;
  cursor: boolean;
  codex: boolean;
  gemini: boolean;
  grokbuild: boolean;
  opencode: boolean;
  hermes: boolean;
}

export interface McpServer {
  id: string;
  name: string;
  server: McpServerSpec;
  apps: McpApps;
  description?: string;
  tags?: string[];
  homepage?: string;
  docs?: string;
}

export interface ScannedMcpServer {
  name: string;
  agent: string;
  kind: string;
  target: string;
}

export const EMPTY_APPS: McpApps = {
  claude: false,
  cursor: false,
  codex: false,
  gemini: false,
  grokbuild: false,
  opencode: false,
  hermes: false,
};

export const MCP_APP_IDS: McpAppId[] = [
  "claude",
  "cursor",
  "codex",
  "gemini",
  "grokbuild",
  "opencode",
  "hermes",
];

export function isMcpAppId(id: string): id is McpAppId {
  return (MCP_APP_IDS as string[]).includes(id);
}

export function defaultEnabledApps(): McpApps {
  return {
    ...EMPTY_APPS,
    claude: true,
    cursor: true,
    codex: true,
    gemini: true,
    grokbuild: true,
  };
}
