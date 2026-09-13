import { useState, useEffect, useCallback } from "react";
import { invoke } from "@tauri-apps/api/core";

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

export function McpLibraryPage() {
  const [tab, setTab] = useState<Tab>("installed");
  const [servers, setServers] = useState<McpServer[]>([]);
  const [q, setQ] = useState("");
  const [loading, setLoading] = useState(true);

  const [registry, setRegistry] = useState<RegistryServer[]>([]);
  const [cursor, setCursor] = useState<string | null>(null);
  const [regLoading, setRegLoading] = useState(false);
  const [regError, setRegError] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [toast, setToast] = useState<string | null>(null);

  const scan = useCallback(
    () =>
      invoke<McpServer[]>("scan_mcp_servers")
        .then(setServers)
        .catch(() => setServers([])),
    [],
  );

  useEffect(() => {
    scan().finally(() => setLoading(false));
  }, [scan]);

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
    if (tab === "registry" && registry.length === 0 && !regError) {
      loadRegistry(null);
    }
  }, [tab, registry.length, regError, loadRegistry]);

  const install = async (s: RegistryServer) => {
    setBusy(s.name);
    try {
      const msg = await invoke<string>("install_mcp_server", {
        name: s.name,
        url: s.url,
        transport: s.transport,
      });
      setToast(msg);
      await scan();
      // refresh the installed flag on the clicked row
      setRegistry((prev) =>
        prev.map((r) => (r.name === s.name ? { ...r, installed: true } : r)),
      );
    } catch (e) {
      setToast(String(e));
    } finally {
      setBusy(null);
      setTimeout(() => setToast(null), 3500);
    }
  };

  if (loading)
    return <p style={{ color: "var(--color-text-muted)" }}>Loading…</p>;

  const needle = q.trim().toLowerCase();
  const match = (t: string) => t.toLowerCase().includes(needle);

  const shown = needle
    ? servers.filter((s) => match(s.name) || match(s.agent) || match(s.target))
    : servers;

  const byAgent = shown.reduce<Record<string, McpServer[]>>((acc, s) => {
    (acc[s.agent] ||= []).push(s);
    return acc;
  }, {});

  const regShown = needle
    ? registry.filter(
        (r) => match(r.name) || match(r.title) || match(r.description),
      )
    : registry;

  return (
    <div>
      <div className="page-head">
        <div>
          <h2>MCP Library</h2>
          <p className="page-sub">
            {tab === "installed"
              ? `${servers.length} servers configured across ${
                  Object.keys(
                    servers.reduce<Record<string, 1>>(
                      (a, s) => ((a[s.agent] = 1), a),
                      {},
                    ),
                  ).length
                } agents`
              : `${registry.length} listed · public MCP registry`}
          </p>
        </div>
        <input
          className="search-input"
          placeholder="Filter…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </div>

      <div className="subtabs">
        <button
          className={`subtab ${tab === "installed" ? "active" : ""}`}
          onClick={() => setTab("installed")}
        >
          Installed
        </button>
        <button
          className={`subtab ${tab === "registry" ? "active" : ""}`}
          onClick={() => setTab("registry")}
        >
          Registry
        </button>
      </div>

      {tab === "installed" && (
        <>
          {servers.length === 0 && (
            <p className="page-sub">
              No MCP servers found in any agent config.
            </p>
          )}
          {Object.entries(byAgent).map(([agent, list]) => (
            <div key={agent} className="lib-group">
              <h3 className="section-heading">
                {agent} <span className="lib-count">{list.length}</span>
              </h3>
              <div className="lib-list">
                {list.map((s) => (
                  <div className="lib-row" key={`${agent}-${s.name}`}>
                    <span className="lib-name">{s.name}</span>
                    <span className={`lib-kind ${s.kind}`}>{s.kind}</span>
                    <span className="lib-target">{s.target}</span>
                  </div>
                ))}
              </div>
            </div>
          ))}
        </>
      )}

      {tab === "registry" && (
        <>
          {regError && <div className="error-banner">{regError}</div>}
          {!regError && registry.length === 0 && regLoading && (
            <p className="page-sub">Loading registry…</p>
          )}
          <div className="lib-list wide">
            {regShown.map((r) => (
              <div className="lib-row reg" key={r.name}>
                <div className="reg-main">
                  <span className="lib-name">{r.title || r.name}</span>
                  {r.installed && (
                    <span className="lib-kind http">installed</span>
                  )}
                  <span className="lib-kind">{r.transport || "—"}</span>
                  <span className="lib-target">
                    {r.name} · v{r.version}
                  </span>
                  {r.description && (
                    <span className="reg-desc">{r.description}</span>
                  )}
                </div>
                <button
                  className="btn-inline primary"
                  disabled={!r.url || busy === r.name || r.installed}
                  onClick={() => install(r)}
                >
                  {r.installed
                    ? "Installed"
                    : busy === r.name
                      ? "Installing…"
                      : "Install"}
                </button>
              </div>
            ))}
          </div>
          {cursor && (
            <button
              className="btn-inline load-more"
              disabled={regLoading}
              onClick={() => loadRegistry(cursor)}
            >
              {regLoading ? "Loading…" : "Load more"}
            </button>
          )}
        </>
      )}

      {toast && <div className="status-toast success">{toast}</div>}
    </div>
  );
}
