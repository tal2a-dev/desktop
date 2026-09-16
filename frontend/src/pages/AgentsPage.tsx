import { useState, useEffect, useCallback } from "react";
import { toast } from "sonner";
import { Button } from "@/components/ui/button.tsx";
import { Input } from "@/components/ui/input.tsx";
import { cn } from "@/lib/utils.ts";
import { AgentIcon } from "@/components/AgentIcon.tsx";
import { ChatgptDesktopDownload } from "@/components/ChatgptDesktopDownload.tsx";
import { invoke } from "../lib/tauri.ts";
import { useAuth } from "../lib/auth";
import { settingsApi } from "@/lib/api.ts";
import { thisMachineDownloadUrl } from "@/config/chatgptDesktop.ts";

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

type Status = "not-installed" | "not-configured" | "configured" | "guide";

function statusOf(a: AgentConfig): Status {
  if (a.config_type === "guide") return "guide";
  if (!a.installed && !a.detected) return "not-installed";
  if (a.configured) return "configured";
  return "not-configured";
}

const STATUS_LABEL: Record<Status, string> = {
  "not-installed": "Not installed",
  "not-configured": "Not configured",
  configured: "Configured",
  guide: "Manual setup",
};

const PILL_CLASS: Record<Status, string> = {
  "not-installed": "pill pill-not-installed",
  "not-configured": "pill pill-not-configured",
  configured: "pill pill-configured",
  guide: "pill pill-guide",
};

type Filter = "all" | Exclude<Status, "guide">;

const FILTERS: { id: Filter; label: string }[] = [
  { id: "all", label: "All" },
  { id: "configured", label: "Configured" },
  { id: "not-configured", label: "Not configured" },
  { id: "not-installed", label: "Not installed" },
];

function impactOf(a: AgentConfig, baseUrl: string): string {
  const st = statusOf(a);
  if (st === "guide") return "Skipped — manual setup";
  if (st === "configured") return `Overwrite with endpoint ${baseUrl}`;
  if (st === "not-installed")
    return `Write NAPI config now (ready when ${a.name} is installed)`;
  return `Will configure with endpoint ${baseUrl}`;
}

export function AgentsPage() {
  const { state } = useAuth();
  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [busy, setBusy] = useState<string | null>(null);
  const [q, setQ] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [loading, setLoading] = useState(true);
  const [scanError, setScanError] = useState<string | null>(null);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [uninstallTarget, setUninstallTarget] = useState<AgentConfig | null>(
    null,
  );
  const [inlineErr, setInlineErr] = useState<Record<string, string>>({});

  const scan = useCallback(async () => {
    setLoading(true);
    setScanError(null);
    try {
      setAgents(await invoke<AgentConfig[]>("scan_agents"));
      setScanError(null);
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
    const onFocus = () => void scan();
    window.addEventListener("focus", onFocus);
    return () => {
      document.removeEventListener("visibilitychange", onVis);
      window.removeEventListener("focus", onFocus);
    };
  }, [scan]);

  // ponytail: one click — the API key is read from the keyring in Rust, never typed.
  const quickSetup = async (a: AgentConfig) => {
    setBusy(a.id);
    setInlineErr((m) => {
      const next = { ...m };
      delete next[a.id];
      return next;
    });
    try {
      const msg = await invoke<string>("configure_agent", {
        agentName: a.id,
        baseUrl: state.baseUrl,
      });
      toast.success(msg);
      await scan();
    } catch (e) {
      toast.error(String(e));
      setInlineErr((m) => ({ ...m, [a.id]: String(e) }));
    } finally {
      setBusy(null);
    }
  };

  const setupAll = async () => {
    setBusy("__all__");
    try {
      const msgs = await invoke<string[]>("auto_configure_all", {
        baseUrl: state.baseUrl,
      });
      toast.success(`Configured ${msgs.length} agents`);
      setConfirmOpen(false);
      await scan();
    } catch (e) {
      toast.error(String(e));
    } finally {
      setBusy(null);
    }
  };

  const installCli = async (a: AgentConfig) => {
    setBusy(a.id);
    setInlineErr((m) => {
      const next = { ...m };
      delete next[a.id];
      return next;
    });
    try {
      const msg = await invoke<string>("install_agent", { agentId: a.id });
      toast.success(msg || `Installed ${a.name}`);
      await scan();
    } catch (e) {
      toast.error(String(e));
      setInlineErr((m) => ({ ...m, [a.id]: String(e) }));
    } finally {
      setBusy(null);
    }
  };

  const uninstallCli = async (a: AgentConfig) => {
    setBusy(a.id);
    setUninstallTarget(null);
    try {
      const msg = await invoke<string>("uninstall_agent", {
        agentId: a.id,
        binaryName: a.binary_name,
      });
      toast.success(msg || `Uninstalled ${a.name}`);
      await scan();
    } catch (e) {
      toast.error(String(e));
      setInlineErr((m) => ({ ...m, [a.id]: String(e) }));
    } finally {
      setBusy(null);
    }
  };

  const launch = async (a: AgentConfig) => {
    setInlineErr((m) => {
      const next = { ...m };
      delete next[a.id];
      return next;
    });
    try {
      await invoke("launch_agent", { binaryName: a.binary_name });
      toast.success(`Launched ${a.binary_name}`);
    } catch (e) {
      toast.error(String(e));
      setInlineErr((m) => ({ ...m, [a.id]: String(e) }));
    }
  };

  useEffect(() => {
    if (!confirmOpen) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === "Escape") setConfirmOpen(false);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [confirmOpen]);

  const counts: Record<Filter, number> = {
    all: agents.length,
    configured: agents.filter((a) => statusOf(a) === "configured").length,
    "not-configured": agents.filter((a) => statusOf(a) === "not-configured")
      .length,
    "not-installed": agents.filter((a) => statusOf(a) === "not-installed")
      .length,
  };

  const needle = q.trim().toLowerCase();
  const filtered = agents.filter((a) => {
    if (filter !== "all" && statusOf(a) !== filter) return false;
    if (!needle) return true;
    return [a.name, a.description, a.config_path]
      .join(" ")
      .toLowerCase()
      .includes(needle);
  });

  const busyAny = busy !== null;

  return (
    <div className="px-6 py-4">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div>
          <p className="text-sm text-muted-foreground">
            {counts.configured} of {agents.length} pointed at {state.baseUrl}
          </p>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={() => void scan()}
          disabled={busyAny || loading}
        >
          {loading ? "Scanning…" : "Scan"}
        </Button>
        <Button
          variant="outline"
          size="sm"
          onClick={() => setConfirmOpen(true)}
          disabled={busyAny || loading || agents.length === 0}
        >
          Overwrite all
        </Button>
      </div>

      <div className="mb-4 flex flex-wrap items-center gap-2">
        <Input
          className="h-9 min-w-[220px] flex-1"
          type="search"
          aria-label="Filter agents by name, description, or config path"
          placeholder="Search agents…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <div
          className="flex flex-wrap gap-1"
          role="group"
          aria-label="Filter agents by status"
        >
          {FILTERS.map((f) => (
            <button
              key={f.id}
              className={`rounded-md px-2.5 py-1 text-xs ${filter === f.id ? "bg-muted text-foreground" : "text-muted-foreground hover:bg-muted"}`}
              aria-pressed={filter === f.id}
              onClick={() => setFilter(f.id)}
            >
              {f.label} · {counts[f.id]}
            </button>
          ))}
        </div>
      </div>

      {loading && agents.length === 0 && (
        <div role="status" aria-label="Scanning agents">
          <div className="tool-grid" aria-hidden="true">
            {Array.from({ length: 6 }).map((_, i) => (
              <div
                key={i}
                className="tool-card"
                style={{ cursor: "default" }}
              >
                <div className="tool-card-top">
                  <span className="tool-ident">
                    <span className="skeleton" style={{ width: 44, height: 44 }} />
                    <span
                      className="skeleton"
                      style={{ width: 120, height: 14 }}
                    />
                  </span>
                  <span
                    className="skeleton"
                    style={{ width: 72, height: 18, borderRadius: 999 }}
                  />
                </div>
                <div
                  className="skeleton"
                  style={{ height: 12, marginTop: 12 }}
                />
                <div
                  className="skeleton"
                  style={{ height: 12, width: "70%", marginTop: 8 }}
                />
              </div>
            ))}
          </div>
        </div>
      )}

      {!loading && scanError && agents.length === 0 && (
        <div className="empty-state">
          <div className="empty-state-icon" aria-hidden="true">
            ✕
          </div>
          <h3>Agent scan failed</h3>
          <p>{scanError}</p>
          <button className="btn-secondary" onClick={scan}>
            Retry
          </button>
        </div>
      )}

      {!loading && !scanError && agents.length === 0 && (
        <div className="empty-state">
          <div className="empty-state-icon" aria-hidden="true">
            ○
          </div>
          <h3>No agents found</h3>
          <p>No known CLI tool configs were detected on this machine.</p>
          <button className="btn-secondary" onClick={scan}>
            Rescan
          </button>
        </div>
      )}

      {!loading && !scanError && agents.length > 0 && filtered.length === 0 && (
        <div className="empty-state">
          <div className="empty-state-icon" aria-hidden="true">
            ⌕
          </div>
          <h3>No matches</h3>
          <p>
            No agents match{q.trim() ? ` “${q.trim()}”` : ""} in this filter.
          </p>
          <button
            className="btn-secondary"
            onClick={() => {
              setQ("");
              setFilter("all");
            }}
          >
            Clear filter
          </button>
        </div>
      )}

      {filtered.length > 0 && (
        <div className="flex flex-col gap-3">
          {filtered.map((a) => {
            const st = statusOf(a);
            const live = st === "configured";
            return (
              <div
                key={a.id}
                className={cn(
                  "relative overflow-hidden rounded-xl border border-border bg-card p-4 text-card-foreground transition-all duration-300",
                  live
                    ? "border-blue-500/60 shadow-sm shadow-blue-500/10"
                    : "hover:border-border-hover hover:shadow-sm",
                )}
              >
                {live && (
                  <div className="pointer-events-none absolute inset-0 bg-gradient-to-r from-blue-500/10 to-transparent" />
                )}
                <div className="relative flex flex-col gap-4 sm:flex-row sm:items-center sm:justify-between">
                  <div className="flex min-w-0 flex-1 items-center gap-3">
                    <div className="flex h-8 w-8 flex-shrink-0 items-center justify-center rounded-lg border border-border bg-muted">
                      <AgentIcon id={a.id} name={a.name} size={20} />
                    </div>
                    <div className="min-w-0 flex-1 space-y-1">
                      <div className="flex min-h-7 flex-wrap items-center gap-2">
                        <h3 className="truncate text-base font-semibold leading-none">
                          {a.name}
                        </h3>
                        <span className={PILL_CLASS[st]}>{STATUS_LABEL[st]}</span>
                      </div>
                      <p className="truncate text-sm text-muted-foreground">
                        {a.description || a.config_path}
                      </p>
                    </div>
                  </div>
                  <div className="flex shrink-0 items-center gap-2">
                    {a.id === "codex" && (
                      <Button
                        size="sm"
                        variant="outline"
                        disabled={busy === a.id || busy === "__all__"}
                        onClick={() => {
                          void settingsApi
                            .openExternal(thisMachineDownloadUrl())
                            .catch((e) => toast.error(String(e)));
                        }}
                      >
                        Download app
                      </Button>
                    )}
                    {!a.installed && a.installable && (
                      <Button
                        size="sm"
                        variant="default"
                        disabled={busy === a.id || busy === "__all__"}
                        onClick={() => void installCli(a)}
                      >
                        {busy === a.id ? "Installing…" : "Install"}
                      </Button>
                    )}
                    {a.config_type === "auto" && (
                      <Button
                        size="sm"
                        variant={live ? "outline" : "default"}
                        disabled={busy === a.id || busy === "__all__"}
                        onClick={() => void quickSetup(a)}
                      >
                        {busy === a.id
                          ? "Writing…"
                          : live
                            ? "Overwrite"
                            : "Enable"}
                      </Button>
                    )}
                    {a.installed && (
                      <>
                        <Button
                          size="sm"
                          variant="ghost"
                          onClick={() => void launch(a)}
                        >
                          Launch
                        </Button>
                        <Button
                          size="sm"
                          variant="ghost"
                          className="text-red-400 hover:text-red-300"
                          disabled={busy === a.id || busy === "__all__"}
                          onClick={() => setUninstallTarget(a)}
                        >
                          Uninstall
                        </Button>
                      </>
                    )}
                  </div>
                </div>
                {inlineErr[a.id] && (
                  <p className="relative mt-2 text-xs text-red-400">{inlineErr[a.id]}</p>
                )}
              </div>
            );
          })}
        </div>
      )}

      <ChatgptDesktopDownload />

      {confirmOpen && (
        <div
          className="modal-backdrop"
          onClick={() => (busyAny ? undefined : setConfirmOpen(false))}
        >
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            aria-labelledby="setup-all-title"
            onClick={(e) => e.stopPropagation()}
          >
            <h2 id="setup-all-title">Set up all agents?</h2>
            <p className="page-sub">
              Every known agent config is overwritten with endpoint{" "}
              {state.baseUrl}, including tools that are not installed on this
              machine. The API key is read from the keyring — nothing is typed.
            </p>
            <ul className="modal-impact">
              {agents.map((a) => (
                <li key={a.id}>
                  <strong>{a.name}</strong> — {impactOf(a, state.baseUrl)}
                </li>
              ))}
            </ul>
            <div className="modal-actions">
              <button
                className="btn-secondary"
                onClick={() => setConfirmOpen(false)}
                disabled={busyAny}
              >
                Cancel
              </button>
              <button
                className="btn-primary"
                onClick={setupAll}
                disabled={busyAny}
              >
                {busy === "__all__" ? (
                  <span className="spinner" aria-hidden="true" />
                ) : (
                  "Confirm"
                )}
              </button>
            </div>
          </div>
        </div>
      )}

      {uninstallTarget && (
        <div
          className="modal-backdrop"
          onClick={() => setUninstallTarget(null)}
        >
          <div
            className="modal"
            role="dialog"
            aria-modal="true"
            onClick={(e) => e.stopPropagation()}
          >
            <h2>Uninstall {uninstallTarget.name}?</h2>
            <p className="page-sub">
              Removes this harness&apos;s CLI and desktop app when we know them,
              and strips the NAPI overlay from its original config (Claude,
              Codex, OpenCode, Cursor, Gemini, Grok, Cline, Continue, Pi,
              Hermes, Qwen, Goose, Factory, Roo, Kilo, OpenClaw, Windsurf).
              History and project files stay.
            </p>
            <div className="mt-4 flex justify-end gap-2">
              <Button variant="outline" onClick={() => setUninstallTarget(null)}>
                Cancel
              </Button>
              <Button
                variant="destructive"
                onClick={() => void uninstallCli(uninstallTarget)}
              >
                Uninstall
              </Button>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}
