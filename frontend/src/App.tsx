import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";

interface AgentConfig {
  name: string;
  config_path: string;
  key_field: string;
  base_url_field: string;
  format: string;
  detected: boolean;
}

interface SubscriptionInfo {
  plan: string;
  used: number;
  limit: number;
  reset_date: string;
}

function App() {
  const [token, setToken] = useState("");
  const [baseUrl, setBaseUrl] = useState("https://napi.mikawi.org");
  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [sub, setSub] = useState<SubscriptionInfo | null>(null);
  const [status, setStatus] = useState("");

  const scan = async () => {
    const result = await invoke<AgentConfig[]>("scan_agents");
    setAgents(result);
  };

  const reconfigure = async (name: string) => {
    try {
      const msg = await invoke<string>("reconfigure_agent", {
        agentName: name,
        apiKey: token,
        baseUrl,
      });
      setStatus(msg);
    } catch (e) {
      setStatus(`Error: ${e}`);
    }
  };

  const fetchSub = async () => {
    try {
      const info = await invoke<SubscriptionInfo>("fetch_subscription", {
        token,
        baseUrl,
      });
      setSub(info);
    } catch (e) {
      setStatus(`Subscription error: ${e}`);
    }
  };

  useEffect(() => {
    scan();
  }, []);

  return (
    <div
      style={{
        padding: 24,
        fontFamily: "system-ui",
        maxWidth: 600,
        margin: "0 auto",
      }}
    >
      <h1>Napi Desktop</h1>

      <section>
        <h2>Login</h2>
        <input
          placeholder="API Token"
          value={token}
          onChange={(e) => setToken(e.target.value)}
          style={{ width: "100%", padding: 8, marginBottom: 8 }}
        />
        <input
          placeholder="Base URL"
          value={baseUrl}
          onChange={(e) => setBaseUrl(e.target.value)}
          style={{ width: "100%", padding: 8, marginBottom: 8 }}
        />
        <button onClick={fetchSub} style={{ padding: "8px 16px" }}>
          Load Subscription
        </button>
      </section>

      {sub && (
        <section style={{ marginTop: 24 }}>
          <h2>Subscription</h2>
          <p>Plan: {sub.plan}</p>
          <p>
            Used: {sub.used} / {sub.limit}
          </p>
          <p>Reset: {sub.reset_date}</p>
        </section>
      )}

      <section style={{ marginTop: 24 }}>
        <h2>Detected Agents</h2>
        <button
          onClick={scan}
          style={{ padding: "8px 16px", marginBottom: 12 }}
        >
          Rescan
        </button>
        {agents.map((a) => (
          <div
            key={a.name}
            style={{
              border: "1px solid #ccc",
              borderRadius: 8,
              padding: 12,
              marginBottom: 8,
              opacity: a.detected ? 1 : 0.5,
            }}
          >
            <strong>{a.name}</strong> {a.detected ? "✅" : "❌ not found"}
            <br />
            <small>{a.config_path}</small>
            {a.detected && (
              <button
                onClick={() => reconfigure(a.name)}
                style={{ marginTop: 8, padding: "4px 12px" }}
              >
                Reconfigure
              </button>
            )}
          </div>
        ))}
      </section>

      {status && (
        <p
          style={{
            marginTop: 16,
            color: status.startsWith("Error") ? "red" : "green",
          }}
        >
          {status}
        </p>
      )}
    </div>
  );
}

export default App;
