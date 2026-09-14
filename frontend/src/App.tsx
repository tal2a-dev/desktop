import { useEffect, useState } from "react";
import { invoke } from "./lib/tauri.ts";
import { AuthProvider, useAuth } from "./lib/auth.tsx";
import { ErrorBoundary } from "./components/ErrorBoundary";
import { LoginPage } from "./pages/LoginPage";
import { AgentsPage } from "./pages/AgentsPage";
import { SubscriptionPage } from "./pages/SubscriptionPage";
import { McpLibraryPage } from "./pages/McpLibraryPage";
import { SkillsLibraryPage } from "./pages/SkillsLibraryPage";

export type Tab = "agents" | "subscription" | "mcp" | "skills";

export interface StatusDetail {
  configured?: string;
  quota?: string;
  mcp?: string;
  backendError?: string | null;
}

const TABS: { id: Tab; label: string; hash: string; key: string }[] = [
  { id: "agents", label: "Agents", hash: "#/agents", key: "a" },
  { id: "subscription", label: "Subscription", hash: "#/subscription", key: "s" },
  { id: "mcp", label: "MCP Library", hash: "#/mcp", key: "m" },
  { id: "skills", label: "Skills", hash: "#/skills", key: "k" },
];

function tabFromHash(): Tab {
  const h = window.location.hash;
  return TABS.find((t) => t.id !== "agents" && h.startsWith(t.hash))?.id ?? "agents";
}

const BANNER_DISMISSED_KEY = "napi:setup-banner-dismissed";

type BannerVariant = "success" | "error" | "info";

interface BannerProps {
  lines: string[] | null;
  variant: BannerVariant;
  onDismiss: () => void;
  onRetry: () => void;
}

function SetupBanner({ lines, variant, onDismiss, onRetry }: BannerProps) {
  if (!lines) return null;
  return (
    <div className={`setup-banner is-${variant}`} role="status">
      <div className="setup-banner-inner">
        <div className="setup-banner-lines">
          {lines.map((l, i) => (
            <span key={i}>{l}</span>
          ))}
        </div>
        <div className="setup-banner-actions">
          {variant === "error" && (
            <button className="btn-inline" onClick={onRetry}>
              Retry
            </button>
          )}
          <button className="btn-inline" onClick={onDismiss} aria-label="Dismiss">
            X
          </button>
        </div>
      </div>
    </div>
  );
}

function Dashboard(props: BannerProps) {
  const { state, logout } = useAuth();
  const [tab, setTab] = useState<Tab>(tabFromHash);
  const [helpOpen, setHelpOpen] = useState(false);
  const [status, setStatus] = useState<StatusDetail>({});

  useEffect(() => {
    if (!window.location.hash) window.location.replace("#/agents");
  }, []);

  useEffect(() => {
    const onHash = () => setTab(tabFromHash());
    window.addEventListener("hashchange", onHash);
    return () => window.removeEventListener("hashchange", onHash);
  }, []);

  useEffect(() => {
    const onStatus = (e: Event) =>
      setStatus((s) => ({ ...s, ...(e as CustomEvent<StatusDetail>).detail }));
    window.addEventListener("napi:status", onStatus);
    return () => window.removeEventListener("napi:status", onStatus);
  }, []);

  // Keyboard: `/` focuses page filter, `g` + letter switches tabs, `?` help.
  useEffect(() => {
    let pendingG = false;
    const onKey = (e: KeyboardEvent) => {
      const el = e.target as HTMLElement;
      const typing =
        /^(INPUT|TEXTAREA|SELECT)$/.test(el.tagName) || el.isContentEditable;
      if (e.key === "Escape") {
        setHelpOpen(false);
        pendingG = false;
        return;
      }
      if (typing) return;
      if (e.key === "?") {
        e.preventDefault();
        setHelpOpen((v) => !v);
        return;
      }
      if (pendingG) {
        pendingG = false;
        const t = TABS.find((t) => t.key === e.key);
        if (t) window.location.hash = t.hash;
        return;
      }
      if (e.key === "g") {
        pendingG = true;
        return;
      }
      if (e.key === "/") {
        e.preventDefault();
        (
          document.querySelector(
            ".app-main .search-input, .app-main input",
          ) as HTMLElement | null
        )?.focus();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  return (
    <div className="napi">
      <header className="app-header">
        <div className="app-header-inner">
          <div className="brand">
            <span className="brand-mark">napi://desktop</span>
            <span className="env-pill" title={state.baseUrl}>
              {state.baseUrl}
            </span>
          </div>
          <span className="header-hint" aria-hidden="true">
            <kbd>/</kbd> filter <kbd>g</kbd> tabs <kbd>?</kbd> help
          </span>
          <div className="header-right">
            <span className="quota-mini" title="Quota used">
              <span className="quota-mini-track">
                <span className="quota-mini-fill" style={{ width: status.quota ?? "0%" }} />
              </span>
              {status.quota ?? "--"}
            </span>
            <button className="btn-danger-ghost" onClick={logout}>
              Sign Out
            </button>
          </div>
        </div>
      </header>

      <SetupBanner {...props} />

      <nav className="tabnav" aria-label="Primary">
        <div className="tabnav-inner">
          {TABS.map((t) => (
            <a
              key={t.id}
              href={t.hash}
              className={`tabnav-tab${tab === t.id ? " is-active" : ""}`}
              aria-current={tab === t.id ? "page" : undefined}
            >
              {t.label}
            </a>
          ))}
        </div>
      </nav>

      <main className="app-main">
        {tab === "agents" && <AgentsPage />}
        {tab === "subscription" && <SubscriptionPage />}
        {tab === "mcp" && <McpLibraryPage />}
        {tab === "skills" && <SkillsLibraryPage />}
      </main>

      <footer className="statusline" aria-live="polite">
        <span className="statusline-item">
          <span
            className={`dot ${status.backendError ? "dot-err" : "dot-ok"}`}
          />
          {state.baseUrl}
        </span>
        {status.configured && (
          <span className="statusline-item">{status.configured} configured</span>
        )}
        {status.quota && (
          <span className="statusline-item">{status.quota}</span>
        )}
        {status.mcp && <span className="statusline-item">{status.mcp}</span>}
        <span className="spacer" />
        {status.backendError ? (
          <span className="statusline-item">{status.backendError}</span>
        ) : (
          <span className="statusline-item">{tab}</span>
        )}
      </footer>

      {helpOpen && (
        <div className="kbd-help" role="dialog" aria-label="Keyboard shortcuts">
          <span>
            <kbd>/</kbd> focus page filter
          </span>
          <span>
            <kbd>g</kbd> then <kbd>a</kbd>/<kbd>s</kbd>/<kbd>m</kbd>/<kbd>k</kbd>{" "}
            switch tabs
          </span>
          <span>
            <kbd>Enter</kbd>/<kbd>Space</kbd> expand focused card
          </span>
          <span>
            <kbd>Esc</kbd> close dialog
          </span>
        </div>
      )}
    </div>
  );
}

function AppContent() {
  const { state, logout } = useAuth();
  const [lines, setLines] = useState<string[] | null>(null);
  const [variant, setVariant] = useState<BannerVariant>("info");
  const [dismissed, setDismissed] = useState(
    () => sessionStorage.getItem(BANNER_DISMISSED_KEY) === "1",
  );
  const [setupRun, setSetupRun] = useState(0);

  useEffect(() => {
    if (!state.authed) return;
    invoke<string[]>("auto_setup", { baseUrl: state.baseUrl })
      .then((msgs) => {
        setLines(msgs);
        setVariant("success");
      })
      .catch((e) => {
        const msg = String(e);
        // `authed` is only a cached boolean. If the keyring holds no credential
        // the session is gone — drop to sign-in rather than showing a storage
        // error over an empty dashboard.
        if (/not signed in/i.test(msg)) {
          logout();
          return;
        }
        setLines([msg]);
        setVariant("error");
      });
  }, [state.authed, state.baseUrl, setupRun, logout]);

  if (state.loading) {
    return (
      <div className="napi">
        <div className="login-container">
          <p style={{ color: "var(--color-text-muted)" }}>Loading…</p>
        </div>
      </div>
    );
  }

  if (!state.authed) {
    return (
      <div className="napi">
        <LoginPage />
      </div>
    );
  }

  return (
    <Dashboard
      lines={dismissed ? null : lines}
      variant={variant}
      onDismiss={() => {
        sessionStorage.setItem(BANNER_DISMISSED_KEY, "1");
        setDismissed(true);
      }}
      onRetry={() => {
        sessionStorage.removeItem(BANNER_DISMISSED_KEY);
        setDismissed(false);
        setSetupRun((n) => n + 1);
      }}
    />
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
