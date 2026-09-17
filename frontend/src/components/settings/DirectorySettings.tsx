import { useMemo } from "react";
import { FolderOpen, FolderSearch, Undo2 } from "lucide-react";
import { toast } from "sonner";
import { AgentIcon } from "@/components/AgentIcon.tsx";
import { Button } from "@/components/ui/button.tsx";
import { Input } from "@/components/ui/input.tsx";
import { invoke } from "@/lib/tauri.ts";
import {
  DIR_SETTING_KEY,
  TOOL_IDS,
  TOOL_LABELS,
  type AppSettings,
  type DirectoryAppId,
  type ResolvedDirectories,
  type ToolVersion,
} from "./types.ts";

const DIR_PLACEHOLDERS: Record<DirectoryAppId, string> = {
  claude: "e.g. ~/.claude",
  codex: "e.g. ~/.codex",
  gemini: "e.g. ~/.gemini",
  grok: "e.g. ~/.grok",
  opencode: "e.g. ~/.config/opencode",
  openclaw: "e.g. ~/.openclaw",
  hermes: "e.g. ~/.hermes",
  pi: "e.g. ~/.pi/agent",
};

interface DirectorySettingsProps {
  settings: AppSettings;
  resolvedDirs: ResolvedDirectories;
  onAppConfigChange: (value?: string) => void;
  onBrowseAppConfig: () => Promise<void>;
  onResetAppConfig: () => Promise<void>;
  onDirectoryChange: (app: DirectoryAppId, value?: string) => void;
  onBrowseDirectory: (app: DirectoryAppId) => Promise<void>;
  onResetDirectory: (app: DirectoryAppId) => Promise<void>;
}

export function DirectorySettings({
  settings,
  resolvedDirs,
  onAppConfigChange,
  onBrowseAppConfig,
  onResetAppConfig,
  onDirectoryChange,
  onBrowseDirectory,
  onResetDirectory,
}: DirectorySettingsProps) {
  return (
    <div className="space-y-6">
      <section className="space-y-4">
        <header className="space-y-1">
          <h3 className="text-sm font-medium">
            tal2a Configuration Directory
          </h3>
          <p className="text-xs text-muted-foreground">
            Override the default location of the tal2a configuration
            directory
          </p>
        </header>
        <DirectoryInput
          value={settings.appConfigDir}
          resolvedValue={resolvedDirs.appConfig}
          placeholder="e.g. ~/.config/tal2a"
          onChange={onAppConfigChange}
          onBrowse={onBrowseAppConfig}
          onReset={onResetAppConfig}
        />
      </section>

      <section className="space-y-4">
        <header className="space-y-1">
          <h3 className="text-sm font-medium">
            Configuration Directory Override (Advanced)
          </h3>
          <p className="text-xs text-muted-foreground">
            Override the default configuration directory for Claude, Codex, and
            other apps. Leave empty to use the system default.
          </p>
        </header>

        {TOOL_IDS.map((app) => (
          <DirectoryInput
            key={app}
            label={`${TOOL_LABELS[app]} Configuration Directory`}
            value={settings[DIR_SETTING_KEY[app]] as string | undefined}
            resolvedValue={resolvedDirs[app]}
            placeholder={DIR_PLACEHOLDERS[app]}
            onChange={(val) => onDirectoryChange(app, val)}
            onBrowse={() => onBrowseDirectory(app)}
            onReset={() => onResetDirectory(app)}
          />
        ))}
      </section>
    </div>
  );
}

interface DirectoryInputProps {
  label?: string;
  value?: string;
  resolvedValue: string;
  placeholder?: string;
  onChange: (value?: string) => void;
  onBrowse: () => Promise<void>;
  onReset: () => Promise<void>;
}

function DirectoryInput({
  label,
  value,
  resolvedValue,
  placeholder,
  onChange,
  onBrowse,
  onReset,
}: DirectoryInputProps) {
  const displayValue = useMemo(
    () => value ?? resolvedValue ?? "",
    [value, resolvedValue],
  );

  return (
    <div className="space-y-1.5">
      {label ? (
        <p className="text-xs font-medium text-foreground">{label}</p>
      ) : null}
      <div className="flex items-center gap-2">
        <Input
          value={displayValue}
          placeholder={placeholder}
          className="text-xs"
          onChange={(event) => {
            const next = event.target.value;
            onChange(next.trim().length > 0 ? next : undefined);
          }}
        />
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={() => void onBrowse()}
          title="Browse directory"
        >
          <FolderSearch className="h-4 w-4" />
        </Button>
        <Button
          type="button"
          variant="outline"
          size="icon"
          onClick={() => void onReset()}
          title="Reset to default"
        >
          <Undo2 className="h-4 w-4" />
        </Button>
      </div>
    </div>
  );
}

interface InstallFoldersPanelProps {
  tools: ToolVersion[];
  loading: boolean;
}

export function InstallFoldersPanel({
  tools,
  loading,
}: InstallFoldersPanelProps) {
  const byName = useMemo(() => {
    const map = new Map<string, ToolVersion>();
    for (const tool of tools) map.set(tool.name, tool);
    return map;
  }, [tools]);

  const openFolder = async (path: string) => {
    try {
      await invoke("open_path", { path });
    } catch (e) {
      toast.error(String(e));
    }
  };

  return (
    <div className="space-y-3">
      <p className="text-xs text-muted-foreground">
        Resolved install location for each CLI. Open the folder to inspect the
        binary.
      </p>
      {TOOL_IDS.map((id) => {
        const tool = byName.get(id);
        const path = tool?.install_path ?? "";
        return (
          <div
            key={id}
            className="flex items-center gap-3 rounded-lg border border-border/60 bg-background/40 px-3 py-2.5"
          >
            <AgentIcon id={id} name={TOOL_LABELS[id]} size={18} />
            <div className="min-w-0 flex-1">
              <p className="text-xs font-medium">{TOOL_LABELS[id]}</p>
              <p
                className="truncate font-mono text-[11px] text-muted-foreground"
                title={path || undefined}
              >
                {loading && !path ? "Detecting…" : path || "Not installed"}
              </p>
            </div>
            <Button
              type="button"
              variant="outline"
              size="sm"
              className="h-7 shrink-0 gap-1.5 text-xs"
              disabled={!path || loading}
              onClick={() => void openFolder(path)}
            >
              <FolderOpen className="h-3.5 w-3.5" />
              Open folder
            </Button>
          </div>
        );
      })}
    </div>
  );
}
