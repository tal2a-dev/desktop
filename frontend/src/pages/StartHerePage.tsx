import {
  useCallback,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import {
  AlertCircle,
  ArrowRight,
  BarChart2,
  CheckCircle2,
  Circle,
  Download,
  Loader2,
  Wrench,
} from "lucide-react";
import { toast } from "sonner";
import { AgentIcon } from "@/components/AgentIcon.tsx";
import { ChatgptDesktopDownload } from "@/components/ChatgptDesktopDownload.tsx";
import { McpIcon } from "@/components/BrandIcons.tsx";
import { Button } from "@/components/ui/button.tsx";
import { cn } from "@/lib/utils.ts";
import { invoke } from "@/lib/tauri.ts";
import { useAuth } from "@/lib/auth.tsx";

interface AgentConfig {
  id: string;
  name: string;
  installed: boolean;
  configured: boolean;
  installable: boolean;
  binary_name: string;
  detected?: boolean;
  config_type?: string;
  description?: string;
}

type Readiness = "ready" | "needs-install" | "needs-overwrite" | "guide";

function readinessOf(a: AgentConfig): Readiness {
  if (a.config_type === "guide") return "guide";
  if (!a.installed && !a.detected) return "needs-install";
  if (a.configured) return "ready";
  return "needs-overwrite";
}

const STATUS_LABEL: Record<Readiness, string> = {
  ready: "Ready",
  "needs-install": "Needs install",
  "needs-overwrite": "Needs overwrite",
  guide: "Manual setup",
};

function iconId(id: string): string {
  if (id === "claude-code" || id === "claude-cli") return "claude";
  return id;
}

function hostOf(url: string): string {
  try {
    return new URL(url).host;
  } catch {
    return url.replace(/^https?:\/\//, "");
  }
}

function go(hash: string) {
  window.location.hash = hash;
}

export function StartHerePage() {
  const { state } = useAuth();
  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [loading, setLoading] = useState(true);
  const [scanError, setScanError] = useState<string | null>(null);
  const host = hostOf(state.baseUrl);

  const scan = useCallback(async () => {
    setLoading(true);
    setScanError(null);
    try {
      setAgents(await invoke<AgentConfig[]>("scan_agents"));
    } catch (e) {
      setAgents([]);
      setScanError(String(e));
    } finally {
      setLoading(false);
    }
  }, []);

  useEffect(() => {
    void scan();
  }, [scan]);

  useEffect(() => {
    const onVis = () => {
      if (document.visibilityState === "visible") void scan();
    };
    document.addEventListener("visibilitychange", onVis);
    window.addEventListener("focus", onVis);
    return () => {
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("focus", onVis);
    };
  }, [scan]);

  const counts = useMemo(() => {
    const next = {
      ready: 0,
      "needs-install": 0,
      "needs-overwrite": 0,
      guide: 0,
    };
    for (const a of agents) next[readinessOf(a)] += 1;
    return next;
  }, [agents]);

  const enable = async (a: AgentConfig) => {
    setBusy(a.id);
    try {
      const msg = await invoke<string>("configure_agent", {
        agentName: a.id,
        baseUrl: state.baseUrl,
      });
      toast.success(msg || `Pointed ${a.name} at NAPI`);
      await scan();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(null);
    }
  };

  const install = async (a: AgentConfig) => {
    setBusy(a.id);
    try {
      const msg = await invoke<string>("install_agent", { agentId: a.id });
      toast.success(msg || `Installed ${a.name}`);
      await scan();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(null);
    }
  };

  const signedIn = state.authed;
  const hasReady = counts.ready > 0;
  const hasInstall = agents.length > 0;

  return (
    <div className="h-full min-h-0 overflow-y-auto px-6 py-4">
      <div className="mb-6">
        <p className="text-[11px] font-medium uppercase tracking-wide text-muted-foreground">
          Workspace readiness
        </p>
        <h2 className="text-lg font-semibold">Connect your coding apps</h2>
        <p className="max-w-2xl text-sm text-muted-foreground">
          NAPI Desktop installs and points local CLIs at{" "}
          <span className="font-medium text-foreground">{host}</span>. That
          host is the NAPI backend — not a provider catalog.
        </p>
      </div>

      <div className="mb-6 grid gap-3 sm:grid-cols-3">
        <SummaryCard
          label="Ready"
          value={counts.ready}
          hint="Pointed at NAPI"
          tone="ready"
        />
        <SummaryCard
          label="Needs install"
          value={counts["needs-install"]}
          hint="CLI missing on this machine"
          tone="install"
        />
        <SummaryCard
          label="Needs overwrite"
          value={counts["needs-overwrite"]}
          hint="Installed, not yet using NAPI"
          tone="overwrite"
        />
      </div>

      <div className="mb-6 rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 shadow-sm">
        <p className="mb-3 text-sm font-medium">Getting started</p>
        <ol className="space-y-2 text-sm">
          <CheckRow done={signedIn} label="Sign in to NAPI Desktop" />
          <CheckRow
            done={hasInstall && counts["needs-install"] < agents.length}
            label="Install the coding apps you use"
          />
          <CheckRow
            done={hasReady}
            label={`Point at least one app at ${host}`}
          />
        </ol>
      </div>

      <section className="mb-8">
        <div className="mb-3 flex items-center justify-between gap-3">
          <h3 className="text-sm font-medium">Coding apps</h3>
          <Button
            variant="link"
            size="sm"
            className="h-auto px-0"
            onClick={() => go("#/agents")}
          >
            Manage agents
            <ArrowRight className="h-3.5 w-3.5" />
          </Button>
        </div>

        {loading && agents.length === 0 && (
          <div
            className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3"
            role="status"
            aria-label="Scanning coding apps"
          >
            {Array.from({ length: 6 }).map((_, i) => (
              <div
                key={i}
                className="min-h-[132px] rounded-xl border border-border bg-card/40 p-4"
              >
                <div className="skeleton h-7 w-7 rounded-md" />
                <div className="skeleton mt-3 h-4 w-32" />
                <div className="skeleton mt-2 h-3 w-24" />
              </div>
            ))}
          </div>
        )}

        {!loading && scanError && (
          <div className="rounded-xl border border-border bg-card p-8 text-center">
            <AlertCircle className="mx-auto mb-2 h-8 w-8 text-muted-foreground" />
            <h3 className="text-base font-medium">Couldn’t scan coding apps</h3>
            <p className="mt-1 text-sm text-muted-foreground">{scanError}</p>
            <Button
              className="mt-4"
              variant="outline"
              size="sm"
              onClick={() => void scan()}
            >
              Retry
            </Button>
          </div>
        )}

        {!loading && !scanError && agents.length === 0 && (
          <div className="rounded-xl border border-border bg-card p-8 text-center">
            <Circle className="mx-auto mb-2 h-8 w-8 text-muted-foreground" />
            <h3 className="text-base font-medium">No coding apps found</h3>
            <p className="mt-1 text-sm text-muted-foreground">
              No known CLI configs were detected on this machine.
            </p>
            <Button
              className="mt-4"
              variant="outline"
              size="sm"
              onClick={() => void scan()}
            >
              Rescan
            </Button>
          </div>
        )}

        {agents.length > 0 && (
          <div className="grid gap-3 sm:grid-cols-2 xl:grid-cols-3">
            {agents.map((a) => {
              const st = readinessOf(a);
              const live = st === "ready";
              const working = busy === a.id;
              return (
                <div
                  key={a.id}
                  className={cn(
                    "flex min-h-[150px] flex-col gap-3 rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 shadow-sm transition-colors hover:border-primary/30",
                    live && "border-blue-500/50 shadow-blue-500/10",
                  )}
                >
                  <div className="flex items-start justify-between gap-3">
                    <div className="flex min-w-0 items-center gap-2">
                      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-border bg-background/80">
                        <AgentIcon
                          id={iconId(a.id)}
                          name={a.name}
                          size={20}
                        />
                      </span>
                      <div className="min-w-0">
                        <p className="truncate text-sm font-medium">{a.name}</p>
                        <p className="truncate text-[11px] text-muted-foreground">
                          {st === "ready"
                            ? `Connected to ${host}`
                            : st === "needs-overwrite"
                              ? "Installed · not using NAPI yet"
                              : st === "guide"
                                ? "Configure inside the app"
                                : "CLI not found"}
                        </p>
                      </div>
                    </div>
                    {st === "ready" ? (
                      <CheckCircle2 className="mt-1 h-4 w-4 shrink-0 text-green-500" />
                    ) : st === "needs-install" ? (
                      <AlertCircle className="mt-1 h-4 w-4 shrink-0 text-yellow-500" />
                    ) : (
                      <Circle className="mt-1 h-4 w-4 shrink-0 text-blue-400" />
                    )}
                  </div>

                  <div className="mt-auto flex items-center justify-between gap-2">
                    <span
                      className={cn(
                        "rounded-full border px-1.5 py-0.5 text-[10px] font-medium",
                        st === "ready" &&
                          "border-emerald-500/30 bg-emerald-500/10 text-emerald-600 dark:text-emerald-400",
                        st === "needs-install" &&
                          "border-yellow-500/30 bg-yellow-500/10 text-yellow-700 dark:text-yellow-400",
                        st === "needs-overwrite" &&
                          "border-blue-500/30 bg-blue-500/10 text-blue-600 dark:text-blue-400",
                        st === "guide" &&
                          "border-border text-muted-foreground",
                      )}
                    >
                      {STATUS_LABEL[st]}
                    </span>
                    <div className="flex items-center gap-1">
                      {st === "needs-install" && a.installable && (
                        <Button
                          size="sm"
                          variant="outline"
                          className="h-7 gap-1.5 text-xs"
                          disabled={working}
                          onClick={() => void install(a)}
                        >
                          {working ? (
                            <Loader2 className="h-3.5 w-3.5 animate-spin" />
                          ) : (
                            <Download className="h-3.5 w-3.5" />
                          )}
                          Install
                        </Button>
                      )}
                      {st === "needs-overwrite" && (
                        <Button
                          size="sm"
                          className="h-7 text-xs"
                          disabled={working || !a.installed}
                          onClick={() => void enable(a)}
                        >
                          {working ? (
                            <Loader2 className="h-3.5 w-3.5 animate-spin" />
                          ) : null}
                          Enable
                        </Button>
                      )}
                      {st === "ready" && (
                        <span className="text-xs text-muted-foreground">
                          Ready
                        </span>
                      )}
                      {(st === "guide" ||
                        (st === "needs-install" && !a.installable)) && (
                        <Button
                          variant="link"
                          size="sm"
                          className="h-auto px-0 text-xs"
                          onClick={() => go("#/agents")}
                        >
                          Open Agents
                        </Button>
                      )}
                    </div>
                  </div>
                </div>
              );
            })}
          </div>
        )}
        <ChatgptDesktopDownload />
      </section>

      <section className="pb-8">
        <h3 className="mb-3 text-sm font-medium">Also set up</h3>
        <div className="grid gap-3 sm:grid-cols-3">
          <ShortcutCard
            title="MCP"
            body="Connect tools the agents can call."
            icon={<McpIcon size={16} />}
            onClick={() => go("#/mcp")}
          />
          <ShortcutCard
            title="Skills"
            body="Install reusable agent skills."
            icon={<Wrench className="h-4 w-4" />}
            onClick={() => go("#/skills")}
          />
          <ShortcutCard
            title="Usage"
            body="Watch quota against this NAPI account."
            icon={<BarChart2 className="h-4 w-4" />}
            onClick={() => go("#/subscription")}
          />
        </div>
      </section>
    </div>
  );
}

function SummaryCard({
  label,
  value,
  hint,
  tone,
}: {
  label: string;
  value: number;
  hint: string;
  tone: "ready" | "install" | "overwrite";
}) {
  return (
    <div className="rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 shadow-sm">
      <p className="text-xs text-muted-foreground">{label}</p>
      <p
        className={cn(
          "mt-1 text-2xl font-semibold tabular-nums",
          tone === "ready" && "text-emerald-500",
          tone === "install" && "text-yellow-500",
          tone === "overwrite" && "text-blue-500",
        )}
      >
        {value}
      </p>
      <p className="mt-1 text-[11px] text-muted-foreground">{hint}</p>
    </div>
  );
}

function CheckRow({ done, label }: { done: boolean; label: string }) {
  return (
    <li className="flex items-center gap-2">
      {done ? (
        <CheckCircle2 className="h-4 w-4 shrink-0 text-emerald-500" />
      ) : (
        <Circle className="h-4 w-4 shrink-0 text-muted-foreground" />
      )}
      <span className={done ? "text-foreground" : "text-muted-foreground"}>
        {label}
      </span>
    </li>
  );
}

function ShortcutCard({
  title,
  body,
  icon,
  onClick,
}: {
  title: string;
  body: string;
  icon: ReactNode;
  onClick: () => void;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      className="flex items-start gap-3 rounded-xl border border-border bg-gradient-to-br from-card/80 to-card/40 p-4 text-left shadow-sm transition-colors hover:border-primary/30"
    >
      <span className="flex h-8 w-8 shrink-0 items-center justify-center rounded-lg border border-border bg-background/80 text-muted-foreground">
        {icon}
      </span>
      <span className="min-w-0 flex-1">
        <span className="flex items-center justify-between gap-2">
          <span className="text-sm font-medium">{title}</span>
          <ArrowRight className="h-3.5 w-3.5 text-muted-foreground" />
        </span>
        <span className="mt-0.5 block text-xs text-muted-foreground">
          {body}
        </span>
      </span>
    </button>
  );
}
