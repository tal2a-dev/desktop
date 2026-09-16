import { useState, type FormEvent } from "react";
import { BrandLockup } from "../components/BrandLockup.tsx";
import { useAuth } from "../lib/auth.tsx";
import { IS_DEMO } from "../lib/tauri.ts";

type BusyKind = "login" | "github" | "verify" | null;

const CLAMP_2 = {
  display: "-webkit-box",
  WebkitLineClamp: 2,
  WebkitBoxOrient: "vertical",
  overflow: "hidden",
} as const;

function ErrorDetail({ error }: { error: string }) {
  const [expanded, setExpanded] = useState(false);
  const [copied, setCopied] = useState(false);
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(error);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      /* clipboard unavailable — Expand still reveals the full text */
    }
  };
  return (
    <div className="error-banner" role="alert">
      <span style={expanded ? undefined : CLAMP_2}>{error}</span>
      <div style={{ display: "flex", gap: 8, marginTop: 8 }}>
        <button
          type="button"
          className="btn-inline"
          onClick={() => setExpanded((v) => !v)}
        >
          {expanded ? "Collapse" : "Expand"}
        </button>
        <button type="button" className="btn-inline" onClick={copy}>
          {copied ? "Copied" : "Copy"}
        </button>
      </div>
    </div>
  );
}

export function LoginPage() {
  const { login, verify2fa, githubOAuth, state } = useAuth();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [showPassword, setShowPassword] = useState(false);
  const [code, setCode] = useState("");
  const [usernameError, setUsernameError] = useState<string | null>(null);
  const [passwordError, setPasswordError] = useState<string | null>(null);
  const [codeError, setCodeError] = useState<string | null>(null);
  const [busy, setBusy] = useState<BusyKind>(null);
  // Back-to-password is local only (no dispatch): the flowToken stays alive so
  // Verify/Resend still work if the user returns to the 2FA step.
  const [backToPassword, setBackToPassword] = useState(false);

  const handleLogin = async (e: FormEvent) => {
    e.preventDefault();
    const uErr = username.trim() ? null : "Username is required";
    const pErr = password ? null : "Password is required";
    setUsernameError(uErr);
    setPasswordError(pErr);
    if (uErr || pErr) return;
    setBusy("login");
    try {
      await login(username.trim(), password);
      setBackToPassword(false);
    } finally {
      setBusy(null);
    }
  };

  const handleVerify = async (e: FormEvent) => {
    e.preventDefault();
    const cErr = code.trim() ? null : "Enter the 6-digit code";
    setCodeError(cErr);
    if (cErr) return;
    setBusy("verify");
    try {
      await verify2fa(code.trim());
    } finally {
      setBusy(null);
    }
  };

  const handleGithub = async () => {
    setBusy("github");
    try {
      await githubOAuth();
    } finally {
      setBusy(null);
    }
  };

  // Resend = re-run password login with the stored credentials for a fresh code.
  const handleResend = async () => {
    if (!username.trim() || !password) {
      setBackToPassword(true);
      return;
    }
    setBusy("login");
    try {
      await login(username.trim(), password);
    } finally {
      setBusy(null);
    }
  };

  const footer = (
    <div
      style={{
        marginTop: 20,
        display: "flex",
        justifyContent: "center",
        gap: 12,
        fontSize: 12,
      }}
    >
      <a href={state.baseUrl} target="_blank" rel="noreferrer">
        Forgot password?
      </a>
      <span aria-hidden="true" style={{ color: "var(--color-text-faint)" }}>
        ·
      </span>
      <a href={state.baseUrl} target="_blank" rel="noreferrer">
        Sign up
      </a>
    </div>
  );

  if (state.flowToken && !backToPassword) {
    return (
      <div className="login-container">
        <form onSubmit={handleVerify} className="login-card glass-card">
          <BrandLockup className="mb-2" heightClass="h-10" />
          <p className="subtitle">Two-factor authentication required</p>
          <div className="field">
            <label className="field-label" htmlFor="code">
              Verification Code
            </label>
            <input
              id="code"
              className="input input-otp"
              type="text"
              inputMode="numeric"
              value={code}
              onChange={(e) => {
                setCode(e.target.value);
                setCodeError(null);
              }}
              autoComplete="one-time-code"
              maxLength={6}
              autoFocus
              required
              placeholder="000000"
              aria-invalid={!!codeError}
              aria-describedby={codeError ? "code-error" : undefined}
            />
            {codeError && (
              <div className="field-error" id="code-error">
                {codeError}
              </div>
            )}
          </div>
          {state.error && <ErrorDetail key={state.error} error={state.error} />}
          <button
            type="submit"
            className="btn-primary"
            disabled={busy === "verify"}
            title={busy === "verify" ? "Verifying code…" : undefined}
            aria-busy={busy === "verify"}
            style={{ width: "100%" }}
          >
            {busy === "verify" ? (
              <>
                <span className="spinner" aria-hidden="true" />
                Verifying…
              </>
            ) : (
              "Verify"
            )}
          </button>
          <div style={{ display: "flex", gap: 8, marginTop: 12 }}>
            <button
              type="button"
              className="btn-secondary"
              style={{ flex: 1 }}
              onClick={() => setBackToPassword(true)}
            >
              Back to password
            </button>
            <button
              type="button"
              className="btn-inline"
              style={{ flex: 1 }}
              onClick={handleResend}
              disabled={busy !== null}
              title={
                busy !== null ? "A request is already in progress" : undefined
              }
            >
              {busy === "login" ? (
                <>
                  <span className="spinner" aria-hidden="true" />
                  Sending…
                </>
              ) : (
                "Resend code"
              )}
            </button>
          </div>
          {footer}
        </form>
      </div>
    );
  }

  return (
    <div className="login-container">
      <form onSubmit={handleLogin} className="login-card glass-card">
        <BrandLockup className="mb-2" heightClass="h-10" />
        <p className="subtitle">Sign in to your account</p>
        {IS_DEMO && (
          <div className="error-banner" role="note" style={{ marginBottom: 16 }}>
            Demo preview — any credentials sign in, password “2fa” shows the
            2FA step, and all data is mock.
          </div>
        )}
        <div className="field">
          <label className="field-label" htmlFor="username">
            Username
          </label>
          <input
            id="username"
            className="input"
            type="text"
            value={username}
            onChange={(e) => {
              setUsername(e.target.value);
              setUsernameError(null);
            }}
            autoComplete="username"
            autoFocus
            required
            aria-invalid={!!usernameError}
            aria-describedby={usernameError ? "username-error" : undefined}
          />
          {usernameError && (
            <div className="field-error" id="username-error">
              {usernameError}
            </div>
          )}
        </div>
        <div className="field">
          <label className="field-label" htmlFor="password">
            Password
          </label>
          <div className="input-wrap">
            <input
              id="password"
              className="input"
              type={showPassword ? "text" : "password"}
              value={password}
              onChange={(e) => {
                setPassword(e.target.value);
                setPasswordError(null);
              }}
              autoComplete="current-password"
              required
              aria-invalid={!!passwordError}
              aria-describedby={passwordError ? "password-error" : undefined}
            />
            <button
              type="button"
              className="eye-btn"
              onClick={() => setShowPassword((v) => !v)}
              aria-label={showPassword ? "Hide password" : "Show password"}
              title={showPassword ? "Hide password" : "Show password"}
            >
              {showPassword ? "Hide" : "Show"}
            </button>
          </div>
          {passwordError && (
            <div className="field-error" id="password-error">
              {passwordError}
            </div>
          )}
        </div>
        {state.error && <ErrorDetail key={state.error} error={state.error} />}
        <button
          type="submit"
          className="btn-primary"
          disabled={busy === "login"}
          title={busy === "login" ? "Signing in…" : undefined}
          aria-busy={busy === "login"}
          style={{ width: "100%" }}
        >
          {busy === "login" ? (
            <>
              <span className="spinner" aria-hidden="true" />
              Signing in…
            </>
          ) : (
            "Sign In"
          )}
        </button>
        <div className="oauth-divider">or</div>
        <button
          type="button"
          className="btn-github"
          onClick={handleGithub}
          disabled={busy === "github"}
          title={busy === "github" ? "Waiting for GitHub…" : undefined}
          aria-busy={busy === "github"}
        >
          {busy === "github" ? (
            <>
              <span className="spinner" aria-hidden="true" />
              Opening GitHub…
            </>
          ) : (
            "Continue with GitHub"
          )}
        </button>
        {footer}
      </form>
    </div>
  );
}
