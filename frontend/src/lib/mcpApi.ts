import { invoke } from "@/lib/tauri.ts";
import type { McpAppId, McpServer } from "@/types/mcp.ts";

export const mcpApi = {
  getAllServers: () => invoke<Record<string, McpServer>>("get_mcp_servers"),
  upsert: (server: McpServer) => invoke<void>("upsert_mcp_server", { server }),
  delete: (id: string) => invoke<boolean>("delete_mcp_server", { id }),
  toggleApp: (serverId: string, app: McpAppId, enabled: boolean) =>
    invoke<void>("toggle_mcp_app", { serverId, app, enabled }),
  importFromApps: () => invoke<number>("import_mcp_from_apps"),
  validateCommand: (cmd: string) =>
    invoke<boolean>("validate_mcp_command", { cmd }),
};
