import { useEffect, useState } from "react";
import { ArrowLeft, BarChart2, LogOut, Settings, Wrench } from "lucide-react";
import { invoke } from "./lib/tauri.ts";
import { AuthProvider, useAuth } from "./lib/auth.tsx";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { Toaster } from "./components/ui/sonner.tsx";
import { Button } from "./components/ui/button.tsx";
import { LoginPage } from "./pages/LoginPage";
import { AgentsPage } from "./pages/AgentsPage";
import { SubscriptionPage } from "./pages/SubscriptionPage";
import { McpLibraryPage } from "./pages/McpLibraryPage";
import { SkillsLibraryPage } from "./pages/SkillsLibraryPage";
import { SettingsPage } from "./pages/SettingsPage";
import { StartHerePage } from "./pages/StartHerePage";
import { EnvWarningBanner, type EnvConflict } from "./components/EnvWarningBanner.tsx";
import { McpIcon } from "./components/BrandIcons.tsx";
import { BrandLockup } from "./components/BrandLockup.tsx";
import { AutoUpdateCheck } from "./components/AppUpdater.tsx";
import { cn } from "./lib/utils.ts";

export type Tab =
  | "start"
  | "agents"
  | "subscription"
  | "mcp"
  | "skills"
  | "settings";

export interface StatusDetail {
  configured?: string;
  quota?: string;
  mcp?: string;
  backendError?: string | null;
}

const HEADER_HEIGHT = 64;

const TABS: { id: Tab; label: string; hash: string; key: string }[] = [
  { id: "start", label: "Start Here", hash: "#/start", key: "h" },
  { id: "agents", label: "Agents", hash: "#/agents", key: "a" },
  { id: "subscription", label: "Usage", hash: "#/subscription", key: "s" },
  { id: "mcp", label: "MCP", hash: "#/mcp", key: "m" },
  { id: "skills", label: "Skills", hash: "#/skills", key: "k" },
  { id: "settings", label: "Settings", hash: "#/settings", key: "," },
];

function tabFromHash(): Tab {
  const h = window.location.hash;
  return TABS.find((t) => h.startsWith(t.hash))?.id ?? "start";
}

function friendlySetupLine(raw: string): string {
  if (/401/.test(raw) && /token/i.test(raw)) {
    return "Session expired — sign out and sign in again so NAPI can mint the API key.";
  }
  if (/not signed in/i.test(raw)) {
    return "Not signed in.";
  }
  if (/already exists in the keychain/i.test(raw) || /secure storage failure/i.test(raw)) {
    return "Keychain already has a NAPI Desktop login — retry to reuse it, or delete the napi-desktop items in Keychain Access.";
  }
  return raw;
}

const BANNER_DISMISSED_KEY = "napi:setup-banner-dismissed";

type BannerVariant = "success" | "error" | "info";

interface BannerProps {
  lines: string[] | null;
  variant: BannerVariant;
  onDismiss: () => void;
  onRetry: () => void;
}

function SetupBanner({ lines, variant, onDismiss, onRetry }: BannerProps) {
  if (!lines) return null;
  return (
    <div
      className={cn(
        "border-b px-4 py-2 text-sm",
        variant === "error" && "border-red-500/30 bg-red-500/10 text-red-300",
        variant === "success" && "border-emerald-500/30 bg-emerald-500/10 text-emerald-300",
        variant === "info" && "border-blue-500/30 bg-blue-500/10 text-blue-300",
      )}
      role="status"
    >
      <div className="flex items-start justify-between gap-3">
        <div className="flex max-h-32 min-w-0 flex-col gap-0.5 overflow-y-auto">
          {lines.map((l, i) => (
            <span key={i} className="font-mono text-xs">
              {l}
            </span>
          ))}
        </div>
        <div className="flex shrink-0 items-center gap-2">
          {variant === "error" && (
            <button
              className="rounded-md px-2 py-1 text-xs hover:bg-white/10"
              onClick={onRetry}
            >
              Retry
            </button>
          )}
          <button
            className="rounded-md px-2 py-1 text-xs hover:bg-white/10"
            onClick={onDismiss}
            aria-label="Dismiss"
          >
            X
          </button>
        </div>
      </div>
    </div>
  );
}

function Dashboard(props: BannerProps) {
  const { state, logout } = useAuth();
  const [tab, setTab] = useState<Tab>(tabFromHash);
  const [status, setStatus] = useState<StatusDetail>({});
  const [envConflicts, setEnvConflicts] = useState<EnvConflict[]>([]);
  const [envDismissed, setEnvDismissed] = useState(false);

  useEffect(() => {
    if (!window.location.hash) window.location.replace("#/start");
  }, []);

  const scanEnv = () => {
    Promise.all(
      ["claude", "codex", "gemini", "grok"].map((app) =>
        invoke<EnvConflict[]>("check_env_conflicts", { app }).catch(
          () => [] as EnvConflict[],
        ),
      ),
    ).then((groups) => {
      const seen = new Set<string>();
      const flat: EnvConflict[] = [];
      for (const g of groups) {
        for (const c of g) {
          const k = `${c.varName}:${c.sourcePath}`;
          if (!seen.has(k)) {
            seen.add(k);
            flat.push(c);
          }
        }
      }
      setEnvConflicts(flat);
    });
  };

  useEffect(() => {
    scanEnv();
  }, []);

  useEffect(() => {
    const onHash = () => setTab(tabFromHash());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  useEffect(() => {
    const onStatus = (e: Event) =>
      setStatus((s) => ({ ...s, ...(e as CustomEvent<StatusDetail>).detail }));
    window.addEventListener("napi:status", onStatus);
    return () => window.removeEventListener("napi:status", onStatus);
  }, []);

  useEffect(() => {
    let pendingG = false;
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement;
      const typing =
        /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) || el.isContentEditable;
      if (e.key === "Escape") {
        pendingG = false;
        return;
      }
      if (typing) return;
      if (pendingG) {
        pendingG = false;
        const t = TABS.find((t) => t.key === e.key);
        if (t) window.location.hash = t.hash;
        return;
      }
      if (e.key === "g") {
        pendingG = true;
        return;
      }
      if (e.key === "/") {
        e.preventDefault();
        (
          document.querySelector(
            ".app-main .search-input, .app-main input",
          ) as HTMLElement | null
        )?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  const title =
    tab === "skills"
      ? "Skills Management"
      : tab === "mcp"
        ? "MCP Server Management"
        : tab === "subscription"
          ? "Your usage"
          : tab === "settings"
            ? "Settings"
            : tab === "agents"
              ? "Coding agents"
              : tab === "start"
                ? "Start Here"
                : null;

  const go = (hash: string) => {
    window.location.hash = hash;
  };

  const iconBtn =
    "text-muted-foreground hover:text-foreground hover:bg-black/5 dark:hover:bg-white/5 w-8 px-2";

  return (
    <div className="flex h-screen flex-col overflow-hidden bg-background text-foreground">
      <header
        className="fixed z-50 w-full bg-background/80 backdrop-blur-md"
        data-tauri-drag-region
        style={{ top: 0, height: HEADER_HEIGHT }}
      >
        <div className="flex h-full items-center justify-between gap-2 px-6">
          <div
            className="flex items-center gap-2"
            style={{ WebkitAppRegion: "no-drag" } as Record<string, string>}
          >
            {tab !== "start" && title ? (
              <>
                <Button
                  variant="outline"
                  size="icon"
                  aria-label="Back"
                  onClick={() => go("#/start")}
                  className="mr-2 rounded-lg"
                >
                  <ArrowLeft className="h-4 w-4" />
                </Button>
                <h1 className="text-lg font-semibold">{title}</h1>
              </>
            ) : (
              <div className="flex items-center gap-2">
                <BrandLockup heightClass="h-7" />
                <span className="max-w-[240px] truncate text-xs text-muted-foreground">
                  {state.baseUrl.replace(/^https?:\/\//, "")}
                </span>
              </div>
            )}
          </div>

          <div
            className="flex items-center gap-1"
            style={{ WebkitAppRegion: "no-drag" } as Record<string, string>}
          >
            {status.quota && (
              <span className="mr-1 text-xs text-muted-foreground">{status.quota}</span>
            )}
            {tab === "start" && (
              <Button
                variant="ghost"
                size="sm"
                className={cn(iconBtn, "w-auto px-2")}
                onClick={() => go("#/start")}
              >
                Start Here
              </Button>
            )}
            {(tab === "start" || !title) && (
              <>
                <Button
                  variant="ghost"
                  size="sm"
                  className={iconBtn}
                  title="Skills"
                  onClick={() => go("#/skills")}
                >
                  <Wrench className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className={iconBtn}
                  title="MCP"
                  onClick={() => go("#/mcp")}
                >
                  <McpIcon size={16} />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className={iconBtn}
                  title="Usage"
                  onClick={() => go("#/subscription")}
                >
                  <BarChart2 className="h-4 w-4" />
                </Button>
                <Button
                  variant="ghost"
                  size="sm"
                  className={iconBtn}
                  title="Settings"
                  onClick={() => go("#/settings")}
                >
                  <Settings className="h-4 w-4" />
                </Button>
              </>
            )}
            <Button
              variant="ghost"
              size="sm"
              className={cn(iconBtn, "hover:text-red-400")}
              title="Sign out"
              onClick={logout}
            >
              <LogOut className="h-4 w-4" />
            </Button>
          </div>
        </div>
      </header>

      <div style={{ paddingTop: HEADER_HEIGHT }} className="flex min-h-0 flex-1 flex-col">
        {!envDismissed && (
          <EnvWarningBanner
            conflicts={envConflicts}
            onDismiss={() => setEnvDismissed(true)}
            onDeleted={() => {
              setEnvDismissed(false);
              scanEnv();
            }}
          />
        )}
        <SetupBanner {...props} />
        <main className="flex min-h-0 flex-1 flex-col overflow-y-auto">
          {tab === "start" && <StartHerePage />}
          {tab === "agents" && <AgentsPage />}
          {tab === "subscription" && (
            <div className="px-6 py-4">
              <SubscriptionPage />
            </div>
          )}
          {tab === "mcp" && <McpLibraryPage />}
          {tab === "skills" && <SkillsLibraryPage />}
          {tab === "settings" && <SettingsPage />}
        </main>
      </div>
    </div>
  );
}

function AppContent() {
  const { state, logout } = useAuth();
  const [lines, setLines] = useState<string[] | null>(null);
  const [variant, setVariant] = useState<BannerVariant>("info");
  const [dismissed, setDismissed] = useState(
    () => sessionStorage.getItem(BANNER_DISMISSED_KEY) === "1",
  );
  const [setupRun, setSetupRun] = useState(0);

  useEffect(() => {
    if (!state.authed) return;
    invoke<string[]>("auto_setup", { baseUrl: state.baseUrl })
      .then((msgs) => {
        setLines(msgs.map(friendlySetupLine));
        setVariant(msgs.some((m) => /skip|fail|error|401/i.test(m)) ? "info" : "success");
      })
      .catch((e) => {
        const msg = friendlySetupLine(String(e));
        if (/not signed in/i.test(msg)) {
          logout();
          return;
        }
        setLines([msg]);
        setVariant("error");
      });
  }, [state.authed, state.baseUrl, setupRun, logout]);

  if (state.loading) {
    return (
      <div className="flex h-screen items-center justify-center bg-background text-muted-foreground">
        Loading…
      </div>
    );
  }

  if (!state.authed) {
    return (
      <div className="min-h-dvh w-full overflow-y-auto bg-background">
        <LoginPage />
      </div>
    );
  }

  return (
    <>
      <AutoUpdateCheck />
      <Dashboard
        lines={dismissed ? null : lines}
        variant={variant}
        onDismiss={() => {
          sessionStorage.setItem(BANNER_DISMISSED_KEY, "1");
          setDismissed(true);
        }}
        onRetry={() => {
          sessionStorage.removeItem(BANNER_DISMISSED_KEY);
          setDismissed(false);
          setSetupRun((n) => n + 1);
        }}
      />
    </>
  );
}

function App() {
  return (
    <ErrorBoundary>
      <AuthProvider>
        <AppContent />
        <Toaster />
      </AuthProvider>
    </ErrorBoundary>
  );
}

export default App;
