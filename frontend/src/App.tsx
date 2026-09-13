import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { AuthProvider, useAuth } from "./lib/auth.tsx";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { LoginPage } from "./pages/LoginPage";
import { AgentsPage } from "./pages/AgentsPage";
import { SubscriptionPage } from "./pages/SubscriptionPage";
import { McpLibraryPage } from "./pages/McpLibraryPage";
import { SkillsLibraryPage } from "./pages/SkillsLibraryPage";

type Tab = "agents" | "subscription" | "mcp" | "skills";

const TABS: { id: Tab; label: string }[] = [
  { id: "agents", label: "Agents" },
  { id: "subscription", label: "Subscription" },
  { id: "mcp", label: "MCP Library" },
  { id: "skills", label: "Skills" },
];

function Dashboard() {
  const { logout } = useAuth();
  const [tab, setTab] = useState<Tab>("agents");

  return (
    <div className="dashboard">
      <div className="dashboard-header">
        <h1>NAPI Desktop</h1>
        <button className="logout-btn" onClick={logout}>
          Sign Out
        </button>
      </div>

      <nav className="tabs">
        {TABS.map((t) => (
          <button
            key={t.id}
            className={`tab ${tab === t.id ? "active" : ""}`}
            onClick={() => setTab(t.id)}
          >
            {t.label}
          </button>
        ))}
      </nav>

      {tab === "agents" && <AgentsPage />}
      {tab === "subscription" && <SubscriptionPage />}
      {tab === "mcp" && <McpLibraryPage />}
      {tab === "skills" && <SkillsLibraryPage />}
    </div>
  );
}

function AppContent() {
  const { state } = useAuth();
  const [setupMsg, setSetupMsg] = useState<string | null>(null);
  const [setupDone, setSetupDone] = useState(false);

  useEffect(() => {
    if (!state.authed || setupDone) return;
    setSetupDone(true);
    // ponytail: fully automatic — login then self-provision the API key and
    // rewrite every detected agent config. The user touches nothing.
    invoke<string[]>("auto_setup", { baseUrl: state.baseUrl })
      .then((msgs) => setSetupMsg(msgs.join(" · ")))
      .catch((e) => setSetupMsg(String(e)));
  }, [state.authed, state.baseUrl, setupDone]);

  if (state.loading) {
    return (
      <div className="login-container">
        <p style={{ color: "var(--color-text-muted)" }}>Loading…</p>
      </div>
    );
  }

  return state.authed ? (
    <>
      {setupMsg && <div className="setup-banner">{setupMsg}</div>}
      <Dashboard />
    </>
  ) : (
    <LoginPage />
  );
}

function App() {
  return (
    <ErrorBoundary>
      <AuthProvider>
        <AppContent />
      </AuthProvider>
    </ErrorBoundary>
  );
}

export default App;
