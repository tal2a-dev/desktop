import { useState, type FormEvent } from "react";
import { useAuth } from "../lib/auth.tsx";

export function LoginPage() {
  const { login, verify2fa, githubOAuth, state } = useAuth();
  const [username, setUsername] = useState("");
  const [password, setPassword] = useState("");
  const [code, setCode] = useState("");

  const handleLogin = async (e: FormEvent) => {
    e.preventDefault();
    if (!username.trim() || !password) return;
    await login(username.trim(), password);
  };

  const handleVerify = async (e: FormEvent) => {
    e.preventDefault();
    if (!code.trim()) return;
    await verify2fa(code.trim());
  };

  if (state.flowToken) {
    return (
      <div className="login-container">
        <form onSubmit={handleVerify} className="login-card">
          <h1>NAPI Desktop</h1>
          <p className="subtitle">Two-factor authentication required</p>
          <label htmlFor="code">Verification Code</label>
          <input
            id="code"
            type="text"
            value={code}
            onChange={(e) => setCode(e.target.value)}
            autoComplete="one-time-code"
            autoFocus
            required
            placeholder="000000"
          />
          {state.error && <div className="error-banner">{state.error}</div>}
          <button type="submit" disabled={state.loading}>
            {state.loading ? "Verifying…" : "Verify"}
          </button>
        </form>
      </div>
    );
  }

  return (
    <div className="login-container">
      <form onSubmit={handleLogin} className="login-card">
        <h1>NAPI Desktop</h1>
        <p className="subtitle">Sign in to your account</p>
        <label htmlFor="username">Username</label>
        <input
          id="username"
          type="text"
          value={username}
          onChange={(e) => setUsername(e.target.value)}
          autoComplete="username"
          autoFocus
          required
        />
        <label htmlFor="password">Password</label>
        <input
          id="password"
          type="password"
          value={password}
          onChange={(e) => setPassword(e.target.value)}
          autoComplete="current-password"
          required
        />
        {state.error && <div className="error-banner">{state.error}</div>}
        <button type="submit" disabled={state.loading}>
          {state.loading ? "Signing in…" : "Sign In"}
        </button>
        <div className="oauth-divider">or</div>
        <button
          type="button"
          className="btn-github"
          onClick={githubOAuth}
          disabled={state.loading}
        >
          {state.loading ? "Opening GitHub…" : "Continue with GitHub"}
        </button>
      </form>
    </div>
  );
}
