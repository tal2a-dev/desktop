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
  installable: boolean;
  uninstallable: boolean;
}

const MOCK_AGENTS: AgentConfig[] = [
  {
    id: "claude-code",
    name: "Claude Code",
    description: "Claude Code pointed at NAPI.",
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
    installable: true,
    uninstallable: true,
  },
  {
    id: "codex",
    name: "Codex",
    description: "Codex CLI, Desktop, and ChatGPT cowork (shared ~/.codex).",
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
    installable: true,
    uninstallable: true,
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
    installable: true,
    uninstallable: true,
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
    installable: false,
    uninstallable: false,
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
    case "install_agent":
      return `Demo mode — would install ${a.agentId}.` as T;
    case "update_agent":
      return `Demo mode — would update ${a.agentId}.` as T;
    case "uninstall_agent":
      return `Demo mode — would uninstall ${a.agentId}.` as T;
    case "check_env_conflicts":
      return [] as T;
    case "delete_env_vars":
      return { backupPath: "/tmp/demo", timestamp: "", conflicts: [] } as T;
    case "fetch_user_logs":
      return {
        success: true,
        message: "",
        data: {
          page: 1,
          page_size: 50,
          total: 128,
          items: [
            {
              id: 1,
              created_at: Math.floor(Date.now() / 1000) - 3600,
              model_name: "claude-sonnet-5",
              prompt_tokens: 1200,
              completion_tokens: 340,
              quota: 25000,
              token_name: "desktop",
              type: 2,
            },
            {
              id: 2,
              created_at: Math.floor(Date.now() / 1000) - 7200,
              model_name: "claude-opus-5",
              prompt_tokens: 800,
              completion_tokens: 210,
              quota: 80000,
              token_name: "desktop",
              type: 2,
            },
            {
              id: 3,
              created_at: Math.floor(Date.now() / 1000) - 86400,
              model_name: "",
              prompt_tokens: 0,
              completion_tokens: 0,
              quota: 500000,
              token_name: "",
              type: 1,
            },
          ],
        },
      } as T;
    case "get_status":
      return { success: true, data: { quota_per_unit: 500000 } } as T;
    case "auto_configure_all":
      return ["Demo mode — nothing was written."] as T;
    case "get_app_settings":
      return {
        enableClaudePluginIntegration: true,
        skipClaudeOnboarding: true,
        logEnabled: true,
        logLevel: "info",
        selectedTokenId: 1,
      } as T;
    case "list_api_keys":
      return [
        {
          id: 1,
          name: "tal2a-desktop",
          status: 1,
          remainQuota: 0,
          usedQuota: 12000,
          unlimitedQuota: true,
          expiredTime: -1,
          keyMasked: "napi**********ktop",
          selected: true,
        },
        {
          id: 2,
          name: "claude-code",
          status: 1,
          remainQuota: 800000,
          usedQuota: 200000,
          unlimitedQuota: false,
          expiredTime: -1,
          keyMasked: "clau**********code",
          selected: false,
        },
      ] as T;
    case "select_api_key":
      return {
        id: a.tokenId,
        name: "selected",
        status: 1,
        remainQuota: 0,
        usedQuota: 0,
        unlimitedQuota: true,
        expiredTime: -1,
        keyMasked: "sk-demo",
        selected: true,
      } as T;
    case "fetch_weekly_usage":
      return {
        start: Math.floor(Date.now() / 1000) - 7 * 86400,
        end: Math.floor(Date.now() / 1000),
        usedUsd: 12.4,
        usedQuota: 6_200_000,
        requestCount: 86,
        tokenUsed: 410_000,
        points: [],
      } as T;
    case "list_models":
      return ["grok-4", "claude-sonnet-5", "gpt-5.6-sol"] as T;
    case "get_agent_models":
      return {} as T;
    case "set_agent_model":
      return `Demo mode — ${a.agentName} model not actually changed.` as T;
    case "save_app_settings":
      return undefined as T;
    case "pick_directory":
      return ((a.defaultPath as string | undefined) ??
        "/Users/demo/.config/tal2a") as T;
    case "get_resolved_directories":
      return {
        appConfig: "/Users/demo/.config/tal2a",
        claude: "/Users/demo/.claude",
        codex: "/Users/demo/.codex",
        gemini: "/Users/demo/.gemini",
        grok: "/Users/demo/.grok",
        opencode: "/Users/demo/.config/opencode",
        openclaw: "/Users/demo/.openclaw",
        hermes: "/Users/demo/.hermes",
        pi: "/Users/demo/.pi/agent",
      } as T;
    case "get_tool_versions":
      return [
        {
          name: "claude",
          version: "2.0.14",
          latest_version: "2.1.0",
          error: null,
          installed_but_broken: false,
          install_path: "/usr/local/bin/claude",
        },
        {
          name: "codex",
          version: "0.42.0",
          latest_version: "0.42.0",
          error: null,
          installed_but_broken: false,
          install_path: "/usr/local/bin/codex",
        },
        {
          name: "gemini",
          version: null,
          latest_version: "0.9.1",
          error: "gemini: command not found",
          installed_but_broken: false,
          install_path: null,
        },
        {
          name: "grok",
          version: "1.2.0",
          latest_version: "1.3.0",
          error: null,
          installed_but_broken: false,
          install_path: "/usr/local/bin/grok",
        },
        {
          name: "opencode",
          version: "0.15.0",
          latest_version: "0.15.0",
          error: null,
          installed_but_broken: false,
          install_path: "/usr/local/bin/opencode",
        },
        {
          name: "openclaw",
          version: null,
          latest_version: "0.4.2",
          error: null,
          installed_but_broken: false,
          install_path: null,
        },
        {
          name: "hermes",
          version: "0.8.1",
          latest_version: "0.8.1",
          error: null,
          installed_but_broken: false,
          install_path: "/usr/local/bin/hermes",
        },
        {
          name: "pi",
          version: "0.3.0",
          latest_version: "0.3.0",
          error: null,
          installed_but_broken: true,
          install_path: "/usr/local/bin/pi",
        },
      ] as T;
    case "open_path":
      return undefined as T;
    case "open_logs_dir":
      return "/Users/demo/.config/tal2a/logs" as T;
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
    case "get_mcp_servers":
      return {
        filesystem: {
          id: "filesystem",
          name: "filesystem",
          server: {
            type: "stdio",
            command: "npx",
            args: ["-y", "@modelcontextprotocol/server-filesystem"],
          },
          apps: {
            claude: true,
            cursor: false,
            codex: false,
            gemini: false,
            grokbuild: false,
            opencode: false,
            hermes: false,
          },
          description: "Local filesystem tools",
        },
      } as T;
    case "upsert_mcp_server":
    case "toggle_mcp_app":
      return undefined as T;
    case "delete_mcp_server":
      return true as T;
    case "import_mcp_from_apps":
      return 2 as T;
    case "validate_mcp_command":
      return true as T;
    case "scan_skills":
      return mockState.skills as T;
    case "get_installed_skills":
      return [
        {
          id: "local:commit",
          name: "commit",
          description: "Write conventional commits",
          directory: "commit",
          apps: {
            claude: true,
            codex: false,
            gemini: false,
            grokbuild: false,
            opencode: true,
            openclaw: false,
            hermes: false,
            pi: false,
            mcode: false,
          },
          installedAt: 0,
          updatedAt: 0,
        },
      ] as T;
    case "get_skill_backups":
    case "scan_unmanaged_skills":
    case "discover_available_skills":
    case "check_skill_updates":
    case "get_skills":
      return [] as T;
    case "get_skill_repos":
      return [
        { owner: "anthropics", name: "skills", branch: "main", enabled: true },
      ] as T;
    case "search_skills_sh":
      return { skills: [], totalCount: 0, query: String(a.query ?? "") } as T;
    case "open_zip_file_dialog":
      return null as T;
    case "open_external":
      return undefined as T;
    case "delete_skill_backup":
    case "add_skill_repo":
    case "remove_skill_repo":
    case "toggle_skill_app":
      return true as T;
    default:
      throw new Error(`Unknown command in demo mode: ${cmd}`);
  }
}

export function invoke<T>(cmd: string, args?: Record<string, unknown>): Promise<T> {
  if (IN_TAURI) return tauriInvoke<T>(cmd, args);
  return mockInvoke<T>(cmd, args);
}

export const IS_DEMO = !IN_TAURI;
