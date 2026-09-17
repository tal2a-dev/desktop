import { useCallback, useEffect, useState, type ReactNode } from "react";
import {
  ChevronDown,
  FolderSearch,
  HardDrive,
  Loader2,
  Save,
  ScrollText,
} from "lucide-react";
import { toast } from "sonner";
import { AboutSection, useToolVersions } from "@/components/settings/AboutSection.tsx";
import { AppUpdater } from "@/components/AppUpdater.tsx";
import {
  DirectorySettings,
  InstallFoldersPanel,
} from "@/components/settings/DirectorySettings.tsx";
import { ApiKeyPicker } from "@/components/settings/ApiKeyPicker.tsx";
import { LogConfigPanel } from "@/components/settings/LogConfigPanel.tsx";
import {
  DIR_SETTING_KEY,
  EMPTY_RESOLVED,
  mergeSettings,
  normalizeResolved,
  sanitizeSettings,
  type AppSettings,
  type DirectoryAppId,
  type LogLevel,
  type ResolvedDirectories,
} from "@/components/settings/types.ts";
import { Button } from "@/components/ui/button.tsx";
import { Switch } from "@/components/ui/switch.tsx";
import { cn } from "@/lib/utils.ts";
import { invoke } from "../lib/tauri.ts";

type SettingsTab = "general" | "advanced" | "about";
type AccordionId = "directory" | "install" | "logs";

const TABS: { id: SettingsTab; label: string }[] = [
  { id: "general", label: "General" },
  { id: "advanced", label: "Advanced" },
  { id: "about", label: "About" },
];

export function SettingsPage() {
  const [tab, setTab] = useState<SettingsTab>("general");
  const [settings, setSettings] = useState<AppSettings | null>(null);
  const [resolvedDirs, setResolvedDirs] =
    useState<ResolvedDirectories>(EMPTY_RESOLVED);
  const [loading, setLoading] = useState(true);
  const [saving, setSaving] = useState(false);
  const [openSections, setOpenSections] = useState<Set<AccordionId>>(
    () => new Set(["directory"]),
  );
  const { tools, loading: toolsLoading, refresh } = useToolVersions();

  const loadResolved = useCallback(async () => {
    try {
      const raw = await invoke<Record<string, unknown>>(
        "get_resolved_directories",
      );
      setResolvedDirs(normalizeResolved(raw));
    } catch (e) {
      toast.error(String(e));
    }
  }, []);

  useEffect(() => {
    let cancelled = false;
    Promise.all([
      invoke<Partial<AppSettings>>("get_app_settings"),
      invoke<Record<string, unknown>>("get_resolved_directories").catch(
        () => ({}),
      ),
    ])
      .then(([rawSettings, rawDirs]) => {
        if (cancelled) return;
        setSettings(mergeSettings(rawSettings));
        setResolvedDirs(normalizeResolved(rawDirs));
      })
      .catch((e) => {
        if (!cancelled) toast.error(String(e));
      })
      .finally(() => {
        if (!cancelled) setLoading(false);
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const save = async (next: AppSettings, message = "Settings saved") => {
    const sanitized = sanitizeSettings(next);
    setSettings(sanitized);
    setSaving(true);
    try {
      await invoke("save_app_settings", { settings: sanitized });
      toast.success(message);
      await loadResolved();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setSaving(false);
    }
  };

  const patch = (updates: Partial<AppSettings>, message?: string) => {
    if (!settings) return;
    void save({ ...settings, ...updates }, message);
  };

  const pickDir = async (defaultPath?: string): Promise<string | undefined> => {
    try {
      const picked = await invoke<string | null>("pick_directory", {
        defaultPath,
      });
      return picked?.trim() ? picked : undefined;
    } catch (e) {
      toast.error(String(e));
      return undefined;
    }
  };

  const browseAppConfig = async () => {
    if (!settings) return;
    const picked = await pickDir(
      settings.appConfigDir ?? resolvedDirs.appConfig,
    );
    if (picked) patch({ appConfigDir: picked });
  };

  const resetAppConfig = async () => {
    patch({ appConfigDir: undefined });
  };

  const browseDirectory = async (app: DirectoryAppId) => {
    if (!settings) return;
    const key = DIR_SETTING_KEY[app];
    const current = (settings[key] as string | undefined) ?? resolvedDirs[app];
    const picked = await pickDir(current);
    if (picked) patch({ [key]: picked });
  };

  const resetDirectory = async (app: DirectoryAppId) => {
    patch({ [DIR_SETTING_KEY[app]]: undefined });
  };

  const openLogs = async () => {
    try {
      const path = await invoke<string>("open_logs_dir");
      if (path) toast.success(`Opened ${path}`);
    } catch (e) {
      toast.error(String(e));
    }
  };

  const toggleSection = (id: AccordionId) => {
    setOpenSections((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  if (loading || !settings) {
    return (
      <p className="px-6 py-4 text-sm text-muted-foreground">
        Loading settings…
      </p>
    );
  }

  return (
    <div className="flex h-full min-h-0 flex-col overflow-hidden px-6 py-4">
      <div
        className="mb-6 grid w-full grid-cols-3 gap-1 rounded-lg glass p-1"
        role="tablist"
        aria-label="Settings sections"
      >
        {TABS.map((item) => (
          <button
            key={item.id}
            type="button"
            role="tab"
            aria-selected={tab === item.id}
            className={cn(
              "inline-flex items-center justify-center whitespace-nowrap rounded-md px-3 py-1.5 text-sm font-medium transition-all",
              tab === item.id
                ? "bg-blue-500 text-white shadow-sm dark:bg-blue-600"
                : "text-muted-foreground opacity-60 hover:bg-muted/50 hover:opacity-100",
            )}
            onClick={() => setTab(item.id)}
          >
            {item.label}
          </button>
        ))}
      </div>

      <div className="min-h-0 flex-1 overflow-y-auto overflow-x-hidden pr-1">
        {tab === "general" && (
          <div className="space-y-4">
          <ApiKeyPicker />
          <section className="space-y-4 rounded-xl border border-border p-4 glass-card">
            <h2 className="text-sm font-medium">Claude Code integration</h2>
            <label className="flex items-start justify-between gap-4">
              <span>
                <span className="block text-sm">Plugin integration</span>
                <span className="text-xs text-muted-foreground">
                  Writes primaryApiKey=any into ~/.claude/config.json so Claude
                  Code accepts the NAPI key.
                </span>
              </span>
              <Switch
                checked={settings.enableClaudePluginIntegration}
                onCheckedChange={(v) =>
                  patch(
                    { enableClaudePluginIntegration: v },
                    "Settings saved — applied on next Overwrite",
                  )
                }
              />
            </label>
            <label className="flex items-start justify-between gap-4">
              <span>
                <span className="block text-sm">Skip onboarding</span>
                <span className="text-xs text-muted-foreground">
                  Sets hasCompletedOnboarding in ~/.claude.json so the first-run
                  wizard does not block the key.
                </span>
              </span>
              <Switch
                checked={settings.skipClaudeOnboarding}
                onCheckedChange={(v) =>
                  patch(
                    { skipClaudeOnboarding: v },
                    "Settings saved — applied on next Overwrite",
                  )
                }
              />
            </label>
          </section>
          </div>
        )}

        {tab === "advanced" && (
          <div className="space-y-4 pb-4">
            <AccordionSection
              open={openSections.has("directory")}
              onToggle={() => toggleSection("directory")}
              icon={<FolderSearch className="h-5 w-5 text-primary" />}
              title="Configuration Directory"
              description="Manage storage paths for tal2a and coding app configurations"
            >
              <DirectorySettings
                settings={settings}
                resolvedDirs={resolvedDirs}
                onAppConfigChange={(value) =>
                  setSettings({ ...settings, appConfigDir: value })
                }
                onBrowseAppConfig={browseAppConfig}
                onResetAppConfig={resetAppConfig}
                onDirectoryChange={(app, value) =>
                  setSettings({
                    ...settings,
                    [DIR_SETTING_KEY[app]]: value,
                  })
                }
                onBrowseDirectory={browseDirectory}
                onResetDirectory={resetDirectory}
              />
            </AccordionSection>

            <AccordionSection
              open={openSections.has("install")}
              onToggle={() => toggleSection("install")}
              icon={<HardDrive className="h-5 w-5 text-blue-500" />}
              title="Application Installation Folders"
              description="Show the detected install path for each coding CLI"
            >
              <InstallFoldersPanel tools={tools} loading={toolsLoading} />
            </AccordionSection>

            <AccordionSection
              open={openSections.has("logs")}
              onToggle={() => toggleSection("logs")}
              icon={<ScrollText className="h-5 w-5 text-cyan-500" />}
              title="Log Management"
              description="Control diagnostic log output and open the logs directory"
            >
              <LogConfigPanel
                enabled={settings.logEnabled}
                level={settings.logLevel}
                onEnabledChange={(logEnabled) => patch({ logEnabled })}
                onLevelChange={(logLevel: LogLevel) => patch({ logLevel })}
                onOpenLogs={() => void openLogs()}
              />
            </AccordionSection>
          </div>
        )}

        {tab === "about" && (
          <div className="space-y-4">
            <AppUpdater />
            <AboutSection
              tools={tools}
              loading={toolsLoading}
              onRefresh={refresh}
            />
          </div>
        )}
      </div>

      {tab === "advanced" && (
        <div className="flex-shrink-0 border-t border-border pt-4">
          <div className="flex items-center justify-end">
            <Button
              onClick={() => void save(settings)}
              disabled={saving}
            >
              {saving ? (
                <span className="inline-flex items-center gap-2">
                  <Loader2 className="h-4 w-4 animate-spin" />
                  Saving
                </span>
              ) : (
                <>
                  <Save className="mr-2 h-4 w-4" />
                  Save
                </>
              )}
            </Button>
          </div>
        </div>
      )}
    </div>
  );
}

function AccordionSection({
  open,
  onToggle,
  icon,
  title,
  description,
  children,
}: {
  open: boolean;
  onToggle: () => void;
  icon: ReactNode;
  title: string;
  description: string;
  children: ReactNode;
}) {
  return (
    <div className="overflow-hidden rounded-xl glass-card">
      <button
        type="button"
        className="flex w-full items-center justify-between px-6 py-4 text-left hover:bg-muted/50"
        aria-expanded={open}
        onClick={onToggle}
      >
        <div className="flex items-center gap-3">
          {icon}
          <div>
            <h3 className="text-base font-semibold">{title}</h3>
            <p className="text-sm font-normal text-muted-foreground">
              {description}
            </p>
          </div>
        </div>
        <ChevronDown
          className={cn(
            "h-4 w-4 shrink-0 transition-transform duration-200",
            open && "rotate-180",
          )}
        />
      </button>
      {open ? (
        <div className="border-t border-border/50 px-6 pb-6 pt-4">
          {children}
        </div>
      ) : null}
    </div>
  );
}
