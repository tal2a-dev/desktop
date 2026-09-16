export const TOOL_IDS = [
  "claude",
  "codex",
  "gemini",
  "grok",
  "opencode",
  "openclaw",
  "hermes",
  "pi",
] as const;

export type ToolId = (typeof TOOL_IDS)[number];

export type DirectoryAppId = ToolId;

export type LogLevel = "error" | "warn" | "info" | "debug" | "trace";

export const LOG_LEVELS: LogLevel[] = [
  "error",
  "warn",
  "info",
  "debug",
  "trace",
];

export interface AppSettings {
  enableClaudePluginIntegration: boolean;
  skipClaudeOnboarding: boolean;
  claudeConfigDir?: string;
  codexConfigDir?: string;
  geminiConfigDir?: string;
  grokConfigDir?: string;
  opencodeConfigDir?: string;
  openclawConfigDir?: string;
  hermesConfigDir?: string;
  piConfigDir?: string;
  appConfigDir?: string;
  logEnabled: boolean;
  logLevel: LogLevel;
  selectedTokenId?: number;
}

export interface ResolvedDirectories {
  appConfig: string;
  claude: string;
  codex: string;
  gemini: string;
  grok: string;
  opencode: string;
  openclaw: string;
  hermes: string;
  pi: string;
}

export interface ToolVersion {
  name: string;
  version: string | null;
  latest_version: string | null;
  error: string | null;
  installed_but_broken: boolean;
  install_path: string | null;
}

export const TOOL_LABELS: Record<ToolId, string> = {
  claude: "Claude Code",
  codex: "Codex",
  gemini: "Gemini CLI",
  grok: "Grok CLI",
  opencode: "OpenCode",
  openclaw: "OpenClaw",
  hermes: "Hermes",
  pi: "Pi",
};

export const DIR_SETTING_KEY: Record<DirectoryAppId, keyof AppSettings> = {
  claude: "claudeConfigDir",
  codex: "codexConfigDir",
  gemini: "geminiConfigDir",
  grok: "grokConfigDir",
  opencode: "opencodeConfigDir",
  openclaw: "openclawConfigDir",
  hermes: "hermesConfigDir",
  pi: "piConfigDir",
};

export const EMPTY_RESOLVED: ResolvedDirectories = {
  appConfig: "",
  claude: "",
  codex: "",
  gemini: "",
  grok: "",
  opencode: "",
  openclaw: "",
  hermes: "",
  pi: "",
};

function asString(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function asOptionalString(value: unknown): string | undefined {
  if (typeof value !== "string") return undefined;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : undefined;
}

function asNullableString(value: unknown): string | null {
  if (value == null) return null;
  if (typeof value !== "string") return null;
  const trimmed = value.trim();
  return trimmed.length > 0 ? trimmed : null;
}

export function isLogLevel(value: unknown): value is LogLevel {
  return LOG_LEVELS.includes(value as LogLevel);
}

export function mergeSettings(raw: Partial<AppSettings> | null | undefined): AppSettings {
  const src = raw ?? {};
  return {
    enableClaudePluginIntegration: src.enableClaudePluginIntegration ?? true,
    skipClaudeOnboarding: src.skipClaudeOnboarding ?? true,
    claudeConfigDir: asOptionalString(src.claudeConfigDir),
    codexConfigDir: asOptionalString(src.codexConfigDir),
    geminiConfigDir: asOptionalString(src.geminiConfigDir),
    grokConfigDir: asOptionalString(src.grokConfigDir),
    opencodeConfigDir: asOptionalString(src.opencodeConfigDir),
    openclawConfigDir: asOptionalString(src.openclawConfigDir),
    hermesConfigDir: asOptionalString(src.hermesConfigDir),
    piConfigDir: asOptionalString(src.piConfigDir),
    appConfigDir: asOptionalString(src.appConfigDir),
    logEnabled: src.logEnabled ?? true,
    logLevel: isLogLevel(src.logLevel) ? src.logLevel : "info",
    selectedTokenId:
      typeof src.selectedTokenId === "number" && src.selectedTokenId > 0
        ? src.selectedTokenId
        : undefined,
  };
}

/** Drop empty override strings so they are not persisted. */
export function sanitizeSettings(settings: AppSettings): AppSettings {
  return mergeSettings(settings);
}

export function normalizeResolved(
  raw: Partial<ResolvedDirectories> | Record<string, unknown> | null | undefined,
): ResolvedDirectories {
  const src = (raw ?? {}) as Record<string, unknown>;
  return {
    appConfig: asString(src.appConfig ?? src.app_config),
    claude: asString(src.claude),
    codex: asString(src.codex),
    gemini: asString(src.gemini),
    grok: asString(src.grok),
    opencode: asString(src.opencode),
    openclaw: asString(src.openclaw),
    hermes: asString(src.hermes),
    pi: asString(src.pi),
  };
}

export function normalizeToolVersion(raw: unknown): ToolVersion {
  const src = (raw ?? {}) as Record<string, unknown>;
  return {
    name: asString(src.name),
    version: asNullableString(src.version),
    latest_version: asNullableString(src.latest_version ?? src.latestVersion),
    error: asNullableString(src.error),
    installed_but_broken: Boolean(
      src.installed_but_broken ?? src.installedButBroken,
    ),
    install_path: asNullableString(src.install_path ?? src.installPath),
  };
}

export function isUpdateAvailable(
  current: string | null | undefined,
  latest: string | null | undefined,
): boolean {
  if (!current || !latest) return false;
  const parse = (v: string): [number, number, number] | null => {
    const m = v.trim().match(/^v?(\d+)\.(\d+)(?:\.(\d+))?/);
    if (!m) return null;
    return [Number(m[1]), Number(m[2]), Number(m[3] ?? 0)];
  };
  const a = parse(current);
  const b = parse(latest);
  if (!a || !b) return current.trim() !== latest.trim();
  for (let i = 0; i < 3; i++) {
    if (b[i] !== a[i]) return b[i] > a[i];
  }
  return false;
}
