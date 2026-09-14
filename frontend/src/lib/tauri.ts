import { invoke as tauriInvoke } from "@tauri-apps/api/core";

// ponytail: browser demo bridge. In the Tauri shell this re-exports the real
// invoke; in a plain browser (vite dev/preview) it serves mock data so the
// redesigned UI is clickable without a Rust backend. No new dependencies.
const IN_TAURI =
  typeof window !== "undefined" && "__TAURI_INTERNALS__" in window;

interface AgentConfig {
  id: string;
  name: string;
  description: string;
  config_path: string;
  key_field: string;
  base_url_field: string;
  format: string;
  detected: boolean;
  installed: boolean;
  configured: boolean;
  config_type: string;
  color: string;
  binary_name: string;
}

const MOCK_AGENTS: AgentConfig[] = [
  {
    id: "claude-code",
    name: "Claude Code",
    description: "Anthropic's CLI for agentic coding with Claude.",
    config_path: "~/.claude/settings.json",
    key_field: "apiKey",
    base_url_field: "baseUrl",
    format: "JSON",
    detected: true,
    installed: true,
    configured: true,
    config_type: "auto",
    color: "#d97757",
    binary_name: "claude",
  },
  {
    id: "codex",
    name: "OpenAI Codex CLI",
    description: "OpenAI's terminal agent. Key comes from OPENAI_API_KEY.",
    config_path: "~/.codex/config.toml",
    key_field: "",
    base_url_field: "base_url",
    format: "TOML",
    detected: true,
    installed: true,
    configured: false,
    config_type: "auto",
    color: "#10a37f",
    binary_name: "codex",
  },
  {
    id: "opencode",
    name: "OpenCode",
    description: "Open-source terminal coding agent.",
    config_path: "~/.config/opencode/opencode.json",
    key_field: "apiKey",
    base_url_field: "baseUrl",
    format: "JSON",
    detected: false,
    installed: false,
    configured: false,
    config_type: "auto",
    color: "#7c3aed",
    binary_name: "opencode",
  },
  {
    id: "cursor",
    name: "Cursor",
    description: "AI-first code editor. Credentials live in its own settings UI.",
    config_path: "Cursor settings UI",
    key_field: "",
    base_url_field: "",
    format: "GUI",
    detected: true,
    installed: true,
    configured: false,
    config_type: "guide",
    color: "#38bdf8",
    binary_name: "cursor",
  },
];

const mockState = {
  authed: false,
  subscription: {
    plan: "Pro",
    used: 42.5,
    limit: 100,
    reset_date: new Date(Date.now() + 12 * 86400000).toISOString(),
  },
  pricing: {
    data: [
      {
        model_name: "claude-opus-5",
        description: "Most capable model for complex reasoning.",
        tags: "flagship",
        supported_endpoint_types: ["anthropic", "openai"],
      },
      {
        model_name: "claude-sonnet-5",
        description: "Best balance of speed and intelligence.",
        tags: "balanced",
        supported_endpoint_types: ["anthropic", "openai"],
      },
    ],
  },
  mcp: [
    {
      name: "filesystem",
      agent: "Claude Code",
      kind: "stdio",
      target: "npx @modelcontextprotocol/server-filesystem",
    },
    {
      name: "github",
      agent: "Claude Code",
      kind: "http",
      target: "https://api.githubcopilot.com/mcp/",
    },
  ],
  registry: {
    servers: [
      {
        name: "io.github.ragaeiou/playwright",
        title: "Playwright",
        description: "Browser automation for agents.",
        version: "1.2.0",
        url: "https://mcp.example.com/playwright",
        transport: "http",
        installed: false,
      },
      {
        name: "io.github.local/fetch",
        title: "Fetch",
        description: "Fetch URLs and convert to markdown.",
        version: "0.9.1",
        url: "",
        transport: "stdio",
        installed: true,
      },
    ],
    next_cursor: null,
  },
  skills: [
    { name: "commit", agent: "Claude Code" },
    { name: "review", agent: "Claude Code" },
    { name: "plan", agent: "OpenCode" },
  ],
};

const delay = (ms = 400) => new Promise((r) => setTimeout(r, ms));

async function mockInvoke<T>(cmd: string, args?: unknown): Promise<T> {
  await delay();
  const a = (args ?? {}) as Record<string, unknown>;
  switch (cmd) {
    case "load_credential":
      return mockState.authed as T;
    case "login":
      if (!a.username || !a.password) throw new Error("Enter a username and password.");
      if (a.password === "2fa") return "2FA_REQUIRED:demo-flow-token" as T;
      mockState.authed = true;
      return "" as T;
    case "verify_2fa":
      if (String(a.code).trim().length < 6) throw new Error("Invalid code.");
      mockState.authed = true;
      return "" as T;
    case "github_oauth":
      mockState.authed = true;
      return "" as T;
    case "clear_credential":
      mockState.authed = false;
      return undefined as T;
    case "auto_setup":
      return [`Demo mode — ${mockState.mcp.length} MCP servers, no configs touched.`] as T;
    case "scan_agents":
      return MOCK_AGENTS as T;
    case "configure_agent":
      return `Demo mode — ${a.agentName} not actually reconfigured.` as T;
    case "auto_configure_all":
      return ["Demo mode — nothing was written."] as T;
    case "launch_agent":
      return undefined as T;
    case "fetch_subscription":
      return mockState.subscription as T;
    case "get_pricing":
      return mockState.pricing as T;
    case "scan_mcp_servers":
      return mockState.mcp as T;
    case "fetch_registry_servers":
      return mockState.registry as T;
    case "install_mcp_server":
      return `Demo mode — ${a.name} not actually installed.` as T;
    case "scan_skills":
      return mockState.skills as T;
    default:
      throw new Error(`Unknown command in demo mode: ${cmd}`);
  }
}

export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (IN_TAURI) return tauriInvoke<T>(cmd, args);
  return mockInvoke<T>(cmd, args);
}

export const IS_DEMO = !IN_TAURI;
