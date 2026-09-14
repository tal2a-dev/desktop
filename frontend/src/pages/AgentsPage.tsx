import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "../lib/tauri.ts";
import { useAuth } from "../lib/auth";

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

interface Toast {
  id: number;
  msg: string;
  kind: "success" | "error";
}

const TOAST_ICON: Record<Toast["kind"], string> = {
  success: "✓",
  error: "✕",
};

function ToastStack({
  toasts,
  onDismiss,
}: {
  toasts: Toast[];
  onDismiss: (id: number) => void;
}) {
  if (toasts.length === 0) return null;
  return (
    <div className="toasts" aria-live="polite">
      {toasts.map((t) => (
        <div key={t.id} className={`toast toast-${t.kind}`} role="status">
          <span aria-hidden="true">{TOAST_ICON[t.kind]}</span>
          <span>{t.msg}</span>
          <button
            className="toast-x"
            onClick={() => onDismiss(t.id)}
            aria-label="Dismiss notification"
          >
            X
          </button>
        </div>
      ))}
    </div>
  );
}

function impactOf(a: AgentConfig, baseUrl: string): string {
  const st = statusOf(a);
  if (st === "guide") return "Skipped — manual setup";
  if (st === "configured") return `Overwrite with endpoint ${baseUrl}`;
  if (st === "not-installed") return "Not installed — setup will be attempted";
  return `Will configure with endpoint ${baseUrl}`;
}

export function AgentsPage() {
  const { state } = useAuth();
  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [openId, setOpenId] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [q, setQ] = useState("");
  const [filter, setFilter] = useState<Filter>("all");
  const [loading, setLoading] = useState(true);
  const [scanError, setScanError] = useState<string | null>(null);
  const [toasts, setToasts] = useState<Toast[]>([]);
  const [confirmOpen, setConfirmOpen] = useState(false);
  const [copied, setCopied] = useState<string | null>(null);
  const [lastRun, setLastRun] = useState<Record<string, string>>({});
  const [inlineErr, setInlineErr] = useState<Record<string, string>>({});
  const toastId = useRef(0);

  const pushToast = useCallback((msg: string, kind: Toast["kind"]) => {
    toastId.current += 1;
    const id = toastId.current;
    // Stack, never overwrite; cap at 3, oldest drops first.
    setToasts((prev) => [...prev.slice(-2), { id, msg, kind }]);
    setTimeout(
      () => setToasts((prev) => prev.filter((t) => t.id !== id)),
      6000,
    );
  }, []);

  const dismissToast = useCallback(
    (id: number) => setToasts((prev) => prev.filter((t) => t.id !== id)),
    [],
  );

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
    scan();
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
        agentName: a.name,
        baseUrl: state.baseUrl,
      });
      pushToast(msg, "success");
      setLastRun((m) => ({ ...m, [a.id]: msg }));
      await scan();
    } catch (e) {
      pushToast(String(e), "error");
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
      pushToast(`Configured ${msgs.length} agents`, "success");
      setConfirmOpen(false);
      await scan();
    } catch (e) {
      pushToast(String(e), "error");
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
      pushToast(`Launched ${a.binary_name}`, "success");
      setLastRun((m) => ({ ...m, [a.id]: `Launched ${a.binary_name}` }));
    } catch (e) {
      pushToast(String(e), "error");
      setInlineErr((m) => ({ ...m, [a.id]: String(e) }));
    }
  };

  const copyPath = async (a: AgentConfig) => {
    try {
      await navigator.clipboard.writeText(a.config_path);
      setCopied(a.id);
      window.setTimeout(
        () => setCopied((cur) => (cur === a.id ? null : cur)),
        1500,
      );
    } catch {
      // Clipboard unavailable — the path text stays selectable.
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
    <div>
      <div className="page-head">
        <div>
          <h2>CLI Tools</h2>
          <p className="page-sub">
            {counts.configured} of {agents.length} configured · endpoint{" "}
            {state.baseUrl}
          </p>
        </div>
        <button
          className="btn-secondary"
          onClick={() => setConfirmOpen(true)}
          disabled={busyAny || loading || agents.length === 0}
        >
          Set up all
        </button>
      </div>

      <div
        style={{
          display: "flex",
          flexWrap: "wrap",
          gap: 8,
          alignItems: "center",
          marginBottom: 16,
        }}
      >
        <input
          className="search-input"
          type="search"
          aria-label="Filter agents by name, description, or config path"
          placeholder="Filter agents…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
        <div
          className="subtabs"
          role="group"
          aria-label="Filter agents by status"
          style={{ marginBottom: 0 }}
        >
          {FILTERS.map((f) => (
            <button
              key={f.id}
              className={`subtab ${filter === f.id ? "active" : ""}`}
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
        <div className="tool-grid">
          {filtered.map((a) => {
            const st = statusOf(a);
            const isOpen = openId === a.id;
            const panelId = `agent-panel-${a.id}`;
            return (
              <div
                key={a.id}
                className={`tool-card ${st} ${isOpen ? "open" : ""}`}
              >
                <div className="tool-card-top">
                  <span className="tool-ident">
                    <span
                      className="tool-tile"
                      style={{ background: a.color }}
                      aria-hidden="true"
                    >
                      {a.name.charAt(0)}
                    </span>
                    <span className="tool-name">{a.name}</span>
                  </span>
                  <span style={{ flex: 1 }} />
                  <span className={PILL_CLASS[st]}>{STATUS_LABEL[st]}</span>
                  <button
                    className="chev-btn"
                    aria-expanded={isOpen}
                    aria-controls={panelId}
                    aria-label={
                      isOpen
                        ? `Collapse ${a.name} details`
                        : `Expand ${a.name} details`
                    }
                    title={isOpen ? "Collapse" : "Expand"}
                    onClick={() => setOpenId(isOpen ? null : a.id)}
                  >
                    {isOpen ? "▾" : "▸"}
                  </button>
                </div>

                {isOpen && (
                  <div className="tool-body" id={panelId}>
                    <p className="tool-desc" title={a.description}>
                      {a.description}
                    </p>
                    <div
                      style={{
                        display: "flex",
                        alignItems: "flex-start",
                        gap: 8,
                      }}
                    >
                      <span className="tool-path" style={{ flex: 1 }}>
                        {a.config_path}
                      </span>
                      <button
                        className="btn-inline"
                        title="Copy config path"
                        onClick={() => copyPath(a)}
                      >
                        {copied === a.id ? "Copied" : "Copy"}
                      </button>
                    </div>
                    <div className="tool-path" style={{ marginTop: 6 }}>
                      endpoint {state.baseUrl}
                    </div>

                    {st === "guide" ? (
                      <div className="tool-note">
                        {a.name} manages credentials in its own settings UI —
                        configure it manually with endpoint {state.baseUrl}
                      </div>
                    ) : (
                      <div className="tool-actions">
                        <button
                          className="btn-primary"
                          style={{ minHeight: 32, padding: "5px 14px", fontSize: 12 }}
                          onClick={() => quickSetup(a)}
                          disabled={busy === a.id || busy === "__all__"}
                        >
                          {busy === a.id ? (
                            <span className="spinner" aria-hidden="true" />
                          ) : st === "configured" ? (
                            "Overwrite"
                          ) : (
                            "Quick Setup"
                          )}
                        </button>
                        {a.installed && (
                          <button
                            className="btn-secondary"
                            style={{ minHeight: 32, padding: "5px 14px", fontSize: 12 }}
                            onClick={() => launch(a)}
                          >
                            Launch in Terminal
                          </button>
                        )}
                      </div>
                    )}
                    {lastRun[a.id] && (
                      <p className="page-sub" role="status" style={{ marginTop: 8 }}>
                        Last run: {lastRun[a.id]}
                      </p>
                    )}
                    {inlineErr[a.id] && (
                      <div
                        className="error-banner"
                        role="alert"
                        style={{ marginTop: 8 }}
                      >
                        {inlineErr[a.id]}
                      </div>
                    )}
                  </div>
                )}
              </div>
            );
          })}
        </div>
      )}

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
              Each config is overwritten with endpoint {state.baseUrl}. The API
              key is read from the keyring — nothing is typed.
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

      <ToastStack toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
