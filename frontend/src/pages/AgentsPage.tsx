import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
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

export function AgentsPage() {
  const { state } = useAuth();
  const [agents, setAgents] = useState<AgentConfig[]>([]);
  const [openId, setOpenId] = useState<string | null>(null);
  const [busy, setBusy] = useState<string | null>(null);
  const [toast, setToast] = useState<{
    msg: string;
    type: "success" | "error";
  } | null>(null);

  const scan = async () =>
    setAgents(await invoke<AgentConfig[]>("scan_agents"));

  useEffect(() => {
    scan().catch(() => {});
  }, []);

  const showToast = (msg: string, type: "success" | "error") => {
    setToast({ msg, type });
    setTimeout(() => setToast(null), 3500);
  };

  // ponytail: one click — the API key is read from the keyring in Rust, never typed.
  const quickSetup = async (a: AgentConfig) => {
    setBusy(a.id);
    try {
      const msg = await invoke<string>("configure_agent", {
        agentName: a.name,
        baseUrl: state.baseUrl,
      });
      showToast(msg, "success");
      await scan();
    } catch (e) {
      showToast(String(e), "error");
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
      showToast(`Configured ${msgs.length} agents`, "success");
      await scan();
    } catch (e) {
      showToast(String(e), "error");
    } finally {
      setBusy(null);
    }
  };

  const launch = async (a: AgentConfig) => {
    try {
      await invoke("launch_agent", { binaryName: a.binary_name });
      showToast(`Launched ${a.binary_name}`, "success");
    } catch (e) {
      showToast(String(e), "error");
    }
  };

  const configured = agents.filter((a) => statusOf(a) === "configured").length;

  return (
    <div>
      <div className="page-head">
        <div>
          <h2>CLI Tools</h2>
          <p className="page-sub">
            {configured} of {agents.length} configured · endpoint{" "}
            {state.baseUrl}
          </p>
        </div>
        <button
          className="btn-inline"
          onClick={setupAll}
          disabled={busy !== null}
        >
          {busy === "__all__" ? "Configuring…" : "Set up all"}
        </button>
      </div>

      <div className="tool-grid">
        {agents.map((a) => {
          const st = statusOf(a);
          const isOpen = openId === a.id;
          return (
            <div
              key={a.id}
              className={`tool-card ${st} ${isOpen ? "open" : ""}`}
              onClick={() => setOpenId(isOpen ? null : a.id)}
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
                <span className={`tool-dot ${st}`} />
              </div>
              <div className="tool-status">{STATUS_LABEL[st]}</div>

              {isOpen && (
                <div className="tool-body" onClick={(e) => e.stopPropagation()}>
                  <p className="tool-desc">{a.description}</p>
                  <div className="tool-path">{a.config_path}</div>

                  {st === "guide" ? (
                    <div className="tool-note">
                      {a.name} manages credentials in its own settings UI —
                      configure it manually with endpoint {state.baseUrl}
                    </div>
                  ) : (
                    <div className="tool-actions">
                      <button
                        className="btn-inline primary"
                        onClick={() => quickSetup(a)}
                        disabled={busy === a.id}
                      >
                        {busy === a.id
                          ? "Configuring…"
                          : st === "configured"
                            ? "Re-apply"
                            : "Quick Setup"}
                      </button>
                      {a.installed && (
                        <button
                          className="btn-inline"
                          onClick={() => launch(a)}
                        >
                          Launch
                        </button>
                      )}
                    </div>
                  )}
                </div>
              )}
            </div>
          );
        })}
      </div>

      {toast && <div className={`status-toast ${toast.type}`}>{toast.msg}</div>}
    </div>
  );
}
