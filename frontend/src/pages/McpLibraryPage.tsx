import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "../lib/tauri.ts";
import type { StatusDetail } from "../App";

interface McpServer {
  name: string;
  agent: string;
  kind: string;
  target: string;
}

interface RegistryServer {
  name: string;
  title: string;
  description: string;
  version: string;
  url: string;
  transport: string;
  installed: boolean;
}

interface RegistryPage {
  servers: RegistryServer[];
  next_cursor: string | null;
}

type Tab = "installed" | "registry";

interface Toast {
  id: number;
  msg: string;
  kind: "success" | "error" | "info";
}

const TOAST_ICON: Record<Toast["kind"], string> = {
  success: "✓",
  error: "✕",
  info: "ℹ",
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

function SkeletonList({ label }: { label: string }) {
  return (
    <div className="skeleton-list" aria-label={label} role="status">
      {Array.from({ length: 6 }).map((_, i) => (
        <div key={i} className="skeleton" style={{ height: 32 }} />
      ))}
    </div>
  );
}

const subFromHash = (): Tab =>
  window.location.hash.startsWith("#/mcp/registry") ? "registry" : "installed";

export function McpLibraryPage() {
  const [tab, setTab] = useState<Tab>(subFromHash);
  const [servers, setServers] = useState<McpServer[]>([]);
  // Own filter state per subtab so switching tabs and Load-more keep the query.
  const [qInst, setQInst] = useState("");
  const [qReg, setQReg] = useState("");
  const [loading, setLoading] = useState(true);
  const [scanError, setScanError] = useState<string | null>(null);
  const [open, setOpen] = useState<string | null>(null);

  const [registry, setRegistry] = useState<RegistryServer[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [regLoading, setRegLoading] = useState(false);
  const [regError, setRegError] = useState<string | null>(null);
  // Per-row parallel busy: concurrent installs never lock each other out.
  const [busy, setBusy] = useState<Record<string, boolean>>({});
  const [toasts, setToasts] = useState<Toast[]>([]);
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

  const scan = useCallback(
    () =>
      invoke<McpServer[]>("scan_mcp_servers")
        .then((s) => {
          setServers(s);
          setScanError(null);
        })
        .catch((e) => {
          setServers([]);
          setScanError(String(e));
        }),
    [],
  );

  useEffect(() => {
    scan().finally(() => setLoading(false));
  }, [scan]);

  const retryScan = useCallback(() => {
    setLoading(true);
    setScanError(null);
    scan().finally(() => setLoading(false));
  }, [scan]);

  // Subtabs persist to #/mcp/installed | #/mcp/registry (App matches by prefix).
  useEffect(() => {
    const onHash = () => {
      if (window.location.hash.startsWith("#/mcp")) setTab(subFromHash());
    };
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  const pick = (t: Tab) => {
    setTab(t);
    if (window.location.hash.startsWith("#/mcp"))
      window.location.hash = t === "registry" ? "#/mcp/registry" : "#/mcp/installed";
  };

  // Statusline: MCP count + backend errors.
  useEffect(() => {
    const detail: StatusDetail = {
      mcp: `${servers.length} MCP`,
      backendError: scanError ?? regError ?? null,
    };
    window.dispatchEvent(new CustomEvent("napi:status", { detail }));
  }, [servers.length, scanError, regError]);

  const loadRegistry = useCallback(async (next?: string | null) => {
    setRegLoading(true);
    setRegError(null);
    try {
      const page = await invoke<RegistryPage>("fetch_registry_servers", {
        cursor: next ?? null,
        limit: 30,
      });
      setRegistry((prev) => (next ? [...prev, ...page.servers] : page.servers));
      setCursor(page.next_cursor);
    } catch (e) {
      setRegError(String(e));
    } finally {
      setRegLoading(false);
    }
  }, []);

  // Lazy-once: fetch on first visit to the Registry subtab only.
  useEffect(() => {
    if (
      tab === "registry" &&
      registry.length === 0 &&
      !regError &&
      !regLoading
    ) {
      loadRegistry(null);
    }
  }, [tab, registry.length, regError, regLoading, loadRegistry]);

  const refreshRegistry = useCallback(() => {
    setCursor(null);
    setRegistry([]);
    setRegError(null);
    loadRegistry(null);
  }, [loadRegistry]);

  const install = async (s: RegistryServer) => {
    setBusy((b) => ({ ...b, [s.name]: true }));
    try {
      const msg = await invoke<string>("install_mcp_server", {
        name: s.name,
        url: s.url,
        transport: s.transport,
      });
      pushToast(msg, "success");
      await scan();
      // refresh the installed flag on the clicked row
      setRegistry((prev) =>
        prev.map((r) => (r.name === s.name ? { ...r, installed: true } : r)),
      );
    } catch (e) {
      pushToast(String(e), "error");
    } finally {
      setBusy((b) => {
        const next = { ...b };
        delete next[s.name];
        return next;
      });
    }
  };

  if (loading)
    return (
      <div>
        <div className="page-head">
          <div>
            <h2>MCP Library</h2>
            <p className="page-sub">Scanning agent configs…</p>
          </div>
        </div>
        <SkeletonList label="Loading MCP servers" />
        <ToastStack toasts={toasts} onDismiss={dismissToast} />
      </div>
    );

  const match = (t: string, needle: string) =>
    t.toLowerCase().includes(needle);

  const instNeedle = qInst.trim().toLowerCase();
  const shown = instNeedle
    ? servers.filter(
        (s) =>
          match(s.name, instNeedle) ||
          match(s.agent, instNeedle) ||
          match(s.target, instNeedle),
      )
    : servers;

  const agentCount = new Set(servers.map((s) => s.agent)).size;

  const byAgent = shown.reduce<Record<string, McpServer[]>>((acc, s) => {
    (acc[s.agent] ||= []).push(s);
    return acc;
  }, {});

  const regNeedle = qReg.trim().toLowerCase();
  const regShown = regNeedle
    ? registry.filter(
        (r) =>
          match(r.name, regNeedle) ||
          match(r.title, regNeedle) ||
          match(r.description, regNeedle),
      )
    : registry;

  return (
    <div>
      <div className="page-head">
        <div>
          <h2>MCP Library</h2>
          <p className="page-sub">
            {tab === "installed"
              ? `${servers.length} servers across ${agentCount} agents`
              : `${registry.length} listed · public MCP registry`}
          </p>
          <div className="lib-chips" style={{ margin: "8px 0 0" }}>
            {tab === "installed" ? (
              <span className="chip">
                <b>{servers.length}</b>&nbsp;({shown.length} shown)
              </span>
            ) : (
              <span className="chip">
                showing&nbsp;<b>{regShown.length}</b>&nbsp;of {registry.length}{" "}
                loaded
              </span>
            )}
          </div>
        </div>
        <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
          {tab === "installed" ? (
            <>
              <input
                className="search-input"
                placeholder="Filter servers…"
                value={qInst}
                onChange={(e) => setQInst(e.target.value)}
              />
              <button
                className="btn-secondary"
                style={{ minHeight: 32, padding: "5px 14px", fontSize: 12 }}
                onClick={retryScan}
                title="Re-scan agent configs"
              >
                Refresh
              </button>
            </>
          ) : (
            <>
              <input
                className="search-input"
                placeholder="Filter registry…"
                value={qReg}
                onChange={(e) => setQReg(e.target.value)}
              />
              <button
                className="btn-secondary"
                style={{ minHeight: 32, padding: "5px 14px", fontSize: 12 }}
                onClick={refreshRegistry}
                disabled={regLoading}
                title="Reload the registry from the first page (keeps your filter)"
              >
                {regLoading && registry.length === 0 ? (
                  <span className="spinner" aria-hidden="true" />
                ) : (
                  "Refresh"
                )}
              </button>
            </>
          )}
        </div>
      </div>

      <div className="subtabs" role="tablist" aria-label="MCP Library views">
        <button
          role="tab"
          aria-selected={tab === "installed"}
          className={`subtab ${tab === "installed" ? "active" : ""}`}
          onClick={() => pick("installed")}
        >
          Installed
        </button>
        <button
          role="tab"
          aria-selected={tab === "registry"}
          className={`subtab ${tab === "registry" ? "active" : ""}`}
          onClick={() => pick("registry")}
        >
          Registry
        </button>
      </div>

      {tab === "installed" && (
        <>
          {scanError && servers.length === 0 && (
            <>
              <div className="error-banner">{scanError}</div>
              <div style={{ marginTop: 12 }}>
                <button className="btn-secondary" onClick={retryScan}>
                  Retry scan
                </button>
              </div>
            </>
          )}
          {!scanError && servers.length === 0 && (
            <div className="empty-state">
              <div className="empty-state-icon" aria-hidden="true">
                ▣
              </div>
              <h3>No MCP servers found</h3>
              <p>No agent config on this machine exposes MCP servers yet.</p>
              <button className="btn-secondary" onClick={retryScan}>
                Retry scan
              </button>
            </div>
          )}
          {servers.length > 0 && shown.length === 0 && (
            <div className="empty-state">
              <div className="empty-state-icon" aria-hidden="true">
                ⌕
              </div>
              <h3>No matches</h3>
              <p>No installed server matches “{qInst.trim()}”.</p>
              <button className="btn-secondary" onClick={() => setQInst("")}>
                Clear filter
              </button>
            </div>
          )}
          {Object.entries(byAgent).map(([agent, list]) => (
            <div key={agent} className="lib-group">
              <h3 className="section-heading">
                {agent} <span className="lib-count">{list.length}</span>
              </h3>
              <div className="lib-list">
                {list.map((s) => {
                  const key = `${agent}-${s.name}`;
                  const isOpen = open === key;
                  return (
                    <div key={key}>
                      <div className="lib-row">
                        <span className="lib-name" title={s.name}>
                          {s.name}
                        </span>
                        <span
                          className={`lib-kind ${s.kind}`}
                          title="Transport kind"
                        >
                          {s.kind || "—"}
                        </span>
                        <span style={{ flex: 1 }} />
                        <button
                          className="chev-btn"
                          aria-expanded={isOpen}
                          aria-label={
                            isOpen
                              ? `Collapse ${s.name} details`
                              : `Expand ${s.name} details`
                          }
                          title={isOpen ? "Collapse" : "Expand"}
                          onClick={() => setOpen(isOpen ? null : key)}
                        >
                          {isOpen ? "▾" : "▸"}
                        </button>
                      </div>
                      {isOpen && (
                        <div
                          className="lib-row compact"
                          style={{ alignItems: "flex-start" }}
                        >
                          <span
                            className="lib-target"
                            title={s.target || "No target recorded"}
                            style={{ flex: 1 }}
                          >
                            {s.target || "—"}
                          </span>
                        </div>
                      )}
                    </div>
                  );
                })}
              </div>
            </div>
          ))}
          {servers.length > 0 && (
            <p className="page-sub" style={{ marginTop: 12 }}>
              Targets exclude credentials — the backend strips query strings
              and fragments.
            </p>
          )}
        </>
      )}

      {tab === "registry" && (
        <>
          {regError && (
            <>
              <div className="error-banner">{regError}</div>
              <div style={{ marginTop: 12 }}>
                <button
                  className="btn-secondary"
                  onClick={() => loadRegistry(null)}
                >
                  Retry
                </button>
              </div>
            </>
          )}
          {!regError && regLoading && registry.length === 0 && (
            <SkeletonList label="Loading registry" />
          )}
          {!regError && !regLoading && registry.length === 0 && (
              <div className="empty-state">
                <div className="empty-state-icon" aria-hidden="true">
                  ▣
                </div>
                <h3>Registry is empty</h3>
                <p>The public registry returned no servers.</p>
                <button className="btn-secondary" onClick={refreshRegistry}>
                  Refresh registry
                </button>
              </div>
            )}
          {!regError && registry.length > 0 && regShown.length === 0 && (
            <div className="empty-state">
              <div className="empty-state-icon" aria-hidden="true">
                ⌕
              </div>
              <h3>No matches</h3>
              <p>No registry server matches “{qReg.trim()}”.</p>
              <button className="btn-secondary" onClick={() => setQReg("")}>
                Clear filter
              </button>
            </div>
          )}
          {regShown.length > 0 && (
            <div className="lib-list wide">
              {regShown.map((r) => {
                const isBusy = !!busy[r.name];
                const why = r.installed
                  ? "Already installed"
                  : !r.url
                    ? "No remote endpoint listed"
                    : isBusy
                      ? "Installing…"
                      : "Install into Claude Code";
                return (
                  <div className="lib-row reg" key={r.name}>
                    <div className="reg-main">
                      <span className="lib-name" title={r.name}>
                        {r.title || r.name}
                      </span>
                      {r.installed && (
                        <span className="pill pill-configured">Installed</span>
                      )}
                      <span
                        className="lib-kind"
                        title="Endpoint transport type"
                      >
                        {r.transport || "—"}
                      </span>
                      <span className="lib-target">
                        {r.name} · v{r.version}
                      </span>
                      {r.description && (
                        <span className="reg-desc">{r.description}</span>
                      )}
                    </div>
                    <button
                      className="btn-inline primary"
                      disabled={!r.url || isBusy || r.installed}
                      title={why}
                      aria-label={`${r.installed ? "Installed" : "Install"} ${r.title || r.name}`}
                      onClick={() => install(r)}
                    >
                      {r.installed ? (
                        "Installed"
                      ) : isBusy ? (
                        <span className="spinner" aria-hidden="true" />
                      ) : (
                        "Install"
                      )}
                    </button>
                  </div>
                );
              })}
            </div>
          )}
          {regShown.length > 0 && (
            <p className="page-sub" style={{ marginTop: 12 }}>
              {cursor
                ? `showing ${registry.length} loaded · more available`
                : `${registry.length} shown`}
              {" · "}Install writes to the Claude Code config
              (~/.claude.json) only. Registry data is public — no key required.
            </p>
          )}
          {cursor && !regError && (
            <button
              className="btn-inline load-more"
              disabled={regLoading}
              onClick={() => loadRegistry(cursor)}
            >
              {regLoading ? (
                <span className="spinner" aria-hidden="true" />
              ) : (
                "Load more"
              )}
            </button>
          )}
        </>
      )}

      <ToastStack toasts={toasts} onDismiss={dismissToast} />
    </div>
  );
}
