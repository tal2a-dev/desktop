import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface McpServer {
  name: string;
  agent: string;
  kind: string;
  target: string;
}

export function McpLibraryPage() {
  const [servers, setServers] = useState<McpServer[]>([]);
  const [q, setQ] = useState("");
  const [loading, setLoading] = useState(true);

  useEffect(() => {
    invoke<McpServer[]>("scan_mcp_servers")
      .then(setServers)
      .catch(() => setServers([]))
      .finally(() => setLoading(false));
  }, []);

  if (loading)
    return <p style={{ color: "var(--color-text-muted)" }}>Loading…</p>;

  const needle = q.trim().toLowerCase();
  const shown = needle
    ? servers.filter(
        (s) =>
          s.name.toLowerCase().includes(needle) ||
          s.agent.toLowerCase().includes(needle) ||
          s.target.toLowerCase().includes(needle),
      )
    : servers;

  const byAgent = shown.reduce<Record<string, McpServer[]>>((acc, s) => {
    (acc[s.agent] ||= []).push(s);
    return acc;
  }, {});

  return (
    <div>
      <div className="page-head">
        <div>
          <h2>MCP Library</h2>
          <p className="page-sub">
            {servers.length} servers configured across{" "}
            {
              Object.keys(
                servers.reduce<Record<string, 1>>(
                  (a, s) => ((a[s.agent] = 1), a),
                  {},
                ),
              ).length
            }{" "}
            agents
          </p>
        </div>
        <input
          className="search-input"
          placeholder="Filter servers…"
          value={q}
          onChange={(e) => setQ(e.target.value)}
        />
      </div>

      {servers.length === 0 && (
        <p className="page-sub">No MCP servers found in any agent config.</p>
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
    </div>
  );
}
