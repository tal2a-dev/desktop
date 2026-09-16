import type { McpServer, McpServerSpec } from "@/types/mcp.ts";

export type McpPreset = Omit<McpServer, "apps" | "description"> & {
  description: string;
};

function npxCommand(
  packageName: string,
  extraArgs: string[] = [],
): { command: string; args: string[] } {
  const isWin =
    typeof navigator !== "undefined" && /windows/i.test(navigator.userAgent);
  if (isWin) {
    return { command: "cmd", args: ["/c", "npx", ...extraArgs, packageName] };
  }
  return { command: "npx", args: [...extraArgs, packageName] };
}

export const mcpPresets: McpPreset[] = [
  {
    id: "fetch",
    name: "mcp-server-fetch",
    tags: ["stdio", "http", "web"],
    description: "Fetch URLs and convert HTML to markdown.",
    server: {
      type: "stdio",
      command: "uvx",
      args: ["mcp-server-fetch"],
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/fetch",
  },
  {
    id: "time",
    name: "@modelcontextprotocol/server-time",
    tags: ["stdio", "time"],
    description: "Time and timezone conversion tools.",
    server: {
      type: "stdio",
      ...npxCommand("@modelcontextprotocol/server-time", ["-y"]),
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/time",
  },
  {
    id: "memory",
    name: "@modelcontextprotocol/server-memory",
    tags: ["stdio", "memory"],
    description: "Knowledge graph memory for agents.",
    server: {
      type: "stdio",
      ...npxCommand("@modelcontextprotocol/server-memory", ["-y"]),
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/memory",
  },
  {
    id: "sequential-thinking",
    name: "@modelcontextprotocol/server-sequential-thinking",
    tags: ["stdio", "reasoning"],
    description: "Step-by-step reasoning tools.",
    server: {
      type: "stdio",
      ...npxCommand("@modelcontextprotocol/server-sequential-thinking", ["-y"]),
    } as McpServerSpec,
    homepage: "https://github.com/modelcontextprotocol/servers",
    docs: "https://github.com/modelcontextprotocol/servers/tree/main/src/sequentialthinking",
  },
  {
    id: "context7",
    name: "@upstash/context7-mcp",
    tags: ["stdio", "docs"],
    description: "Up-to-date library documentation lookup.",
    server: {
      type: "stdio",
      ...npxCommand("@upstash/context7-mcp", ["-y"]),
    } as McpServerSpec,
    homepage: "https://context7.com",
    docs: "https://github.com/upstash/context7/blob/master/README.md",
  },
];
