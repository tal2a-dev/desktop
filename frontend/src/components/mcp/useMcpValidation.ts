import type { McpServerSpec } from "@/types/mcp.ts";

export function parseSmartMcpJson(jsonText: string): {
  id?: string;
  config: McpServerSpec;
} {
  let trimmed = jsonText.trim();
  if (!trimmed) {
    return { config: {} };
  }
  if (trimmed.startsWith('"') && !trimmed.startsWith("{")) {
    trimmed = `{${trimmed}}`;
  }
  const parsed: unknown = JSON.parse(trimmed);
  if (!parsed || typeof parsed !== "object" || Array.isArray(parsed)) {
    throw new Error("JSON must be an object");
  }
  const obj = parsed as Record<string, unknown>;
  if (Object.prototype.hasOwnProperty.call(obj, "mcpServers")) {
    throw new Error("Paste a single server object, not a full mcpServers map");
  }
  const keys = Object.keys(obj);
  if (
    keys.length === 1 &&
    obj[keys[0]] &&
    typeof obj[keys[0]] === "object" &&
    !Array.isArray(obj[keys[0]])
  ) {
    return {
      id: keys[0],
      config: obj[keys[0]] as McpServerSpec,
    };
  }
  return { config: obj as McpServerSpec };
}

export function useMcpValidation() {
  const validateJsonConfig = (value: string): string => {
    if (!value.trim()) return "";
    try {
      const result = parseSmartMcpJson(value);
      const typ = result.config.type ?? "stdio";
      if (typ === "stdio" && !result.config.command?.trim()) {
        return "stdio servers require a command";
      }
      if ((typ === "http" || typ === "sse") && !result.config.url?.trim()) {
        return "http/sse servers require a url";
      }
      return "";
    } catch (err) {
      return err instanceof Error ? err.message : "Invalid JSON";
    }
  };

  return { validateJsonConfig, parseSmartMcpJson };
}
