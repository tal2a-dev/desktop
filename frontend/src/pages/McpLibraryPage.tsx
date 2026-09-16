import { useState, useEffect, useCallback, useRef } from "react";
import { invoke } from "../lib/tauri.ts";
import type { StatusDetail } from "../App.tsx";
import { UnifiedMcpPanel } from "@/components/mcp/UnifiedMcpPanel.tsx";
import type { ScannedMcpServer } from "@/types/mcp.ts";

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
  const [managedCount, setManagedCount] = useState(0);
  const [qReg, setQReg] = useState("");
  const [registry, setRegistry] = useState<RegistryServer[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [regLoading, setRegLoading] = useState(false);
  const [regError, setRegError] = useState<string | null>(null);
  const [busy, setBusy] = useState<Record<string, boolean>>({});
  const [toasts, setToasts] = useState<Toast[]>([]);
  const toastId = useRef(0);

  const pushToast = useCallback((msg: string, kind: Toast["kind"]) => {
    toastId.current += 1;
    const id = toastId.current;
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

  useEffect(() => {
    const detail: StatusDetail = {
      mcp: `${managedCount} MCP`,
      backendError: regError ?? null,
    };
    window.dispatchEvent(new CustomEvent("napi:status", { detail }));
  }, [managedCount, regError]);

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

  useEffect(() => {
    if (tab === "registry" && registry.length === 0 && !regError && !regLoading) {
      void loadRegistry(null);
    }
  }, [tab, registry.length, regError, regLoading, loadRegistry]);

  const refreshRegistry = useCallback(() => {
    setCursor(null);
    setRegistry([]);
    setRegError(null);
    void loadRegistry(null);
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
      setRegistry((prev) =>
        prev.map((r) => (r.name === s.name ? { ...r, installed: true } : r)),
      );
      try {
        const scanned = await invoke<ScannedMcpServer[]>("scan_mcp_servers");
        setManagedCount(scanned.length);
      } catch {
        /* ignore */
      }
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

  const match = (t: string, needle: string) => t.toLowerCase().includes(needle);
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
    <div className="flex min-h-0 flex-1 flex-col px-6 py-4">
      <div className="mb-4 flex items-start justify-between gap-3">
        <div>
          <p className="text-sm text-muted-foreground">
            {tab === "installed"
              ? `${managedCount} managed servers · writes agent mcpServers`
              : `${registry.length} listed · public MCP registry`}
          </p>
        </div>
        {tab === "registry" ? (
          <div style={{ display: "flex", gap: 8, flexShrink: 0 }}>
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
              title="Reload the registry from the first page"
            >
              {regLoading && registry.length === 0 ? (
                <span className="spinner" aria-hidden="true" />
              ) : (
                "Refresh"
              )}
            </button>
          </div>
        ) : null}
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

      {tab === "installed" ? (
        <UnifiedMcpPanel onCountChange={setManagedCount} />
      ) : (
        <>
          {regError && (
            <>
              <div className="error-banner">{regError}</div>
              <div style={{ marginTop: 12 }}>
                <button className="btn-secondary" onClick={() => void loadRegistry(null)}>
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
                      <span className="lib-kind" title="Endpoint transport type">
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
                      onClick={() => void install(r)}
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
              {" · "}Install writes ~/.claude.json and the unified MCP library.
            </p>
          )}
          {cursor && !regError && (
            <button
              className="btn-inline load-more"
              disabled={regLoading}
              onClick={() => void loadRegistry(cursor)}
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
