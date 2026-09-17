# tal2a App — Comprehensive Build Prompt

## 1. Project State

### What Exists (Verified)

- **Tauri v2 scaffold**: `src-tauri/` with `Cargo.toml`, `tauri.conf.json`, `build.rs`, `src/lib.rs`, `src/main.rs`, icons, capabilities/default.json
- **React 19 + Vite frontend**: `frontend/` with `package.json`, `vite.config.ts`, `tsconfig*.json`, `src/App.tsx`, `src/main.tsx`, `index.html`, built dist/
- **Rust backend stubs** in `src-tauri/src/lib.rs`: `scan_agents()`, `reconfigure_agent()`, `fetch_subscription()` — all functional but incomplete
- **Dependencies**: `tauri 2`, `serde 1`, `serde_json 1`, `reqwest 0.12 (json)`, `regex 1`
- **Frontend deps**: `react 19.3`, `react-dom 19.3`, `@tauri-apps/api ^2.11.1`, `@tauri-apps/cli ^2.11.4`, `vite 8.3`, `typescript ~5.8`
- **Lockfiles**: `frontend/bun.lock` (bun), `package-lock.json` (npm root)
- **Build commands**: `bun run dev`, `bun run build`, `tauri dev/build` configured in `tauri.conf.json`

### What Does NOT Exist (Must Be Built)

- Login/auth UI and flow
- Agent scanner UI
- Agent config rewriter UI
- Native agent launcher
- Subscription/quota dashboard UI
- MCP library sync UI (stub only)
- Skill library sync UI (stub only)
- Secure credential storage (currently plaintext in config files)
- Error boundaries, input validation, atomic writes
- Routing (single-page app with no router)
- Any CSS framework or design system
- Tests of any kind

### Exact File Listing (Source Files Only)

```
src-tauri/Cargo.toml
src-tauri/Cargo.lock
src-tauri/build.rs
src-tauri/tauri.conf.json
src-tauri/capabilities/default.json
src-tauri/src/lib.rs          ← HAS STUBS: scan_agents, reconfigure_agent, fetch_subscription
src-tauri/src/main.rs
frontend/package.json
frontend/vite.config.ts
frontend/tsconfig.json
frontend/tsconfig.app.json
frontend/tsconfig.node.json
frontend/index.html
frontend/src/main.tsx
frontend/src/App.tsx
frontend/src/App.css
frontend/src/index.css
frontend/src/vite-env.d.ts
```

---

## 2. Backend API Reference

Base URL: `https://tal2a.app`

| Endpoint                          | Method | Auth         | Status           | Response Shape                                                                                                                                                                 | Notes                                                                                                                                                          |
| --------------------------------- | ------ | ------------ | ---------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `/api/status`                     | GET    | None         | 200              | `{data: {announcements: [], api_info: [{url, description, color}], ...}}`                                                                                                      | Health check, no auth needed                                                                                                                                   |
| `/api/pricing`                    | GET    | None         | 200              | `{auto_groups: [], data: [{model_name, description, icon, tags, vendor_id, quota_type, model_ratio, completion_ratio, cache_ratio, enable_groups, supported_endpoint_types}]}` | Public pricing/models list                                                                                                                                     |
| `/api/models`                     | GET    | Bearer token | 401 without auth | Unknown (auth-gated)                                                                                                                                                           | Requires valid token                                                                                                                                           |
| `/api/user/login`                 | POST   | None         | 200 on failure   | `{message: string, success: boolean}`                                                                                                                                          | Body: `{username, password}`. Returns `success:false` on bad creds. Token location in success response UNVERIFIED — must inspect actual success response shape |
| `/api/user/self`                  | GET    | Bearer token | Assumed 200      | `{data: {role, used_quota, quota, reset_at}}`                                                                                                                                  | Used by existing `fetch_subscription` stub. Field names assumed from new-api convention                                                                        |
| `/dashboard/billing/subscription` | GET    | Bearer token | 401 without auth | `{error: {code, message, type}}`                                                                                                                                               | Subscription details                                                                                                                                           |
| `/dashboard/billing/usage`        | GET    | Bearer token | 401 without auth | `{error: {code, message, type}}`                                                                                                                                               | Usage history                                                                                                                                                  |
| `/api/subscription/plans`         | GET    | Bearer token | 401 without auth | Unknown                                                                                                                                                                        | Plan catalog                                                                                                                                                   |

### Critical Unknowns

- **Login success response shape**: The `/api/user/login` endpoint returns `{success: false, message}` on failure. The success response structure (where the token lives) was NOT captured. The agent MUST test login with valid credentials or inspect new-api source to determine if token is in `response.data.token`, `response.token`, or returned as a cookie.
- **Rate limits**: Not documented. Assume standard rate limiting; implement exponential backoff on 429.
- **Token format**: Bearer token. Storage format unknown (JWT vs opaque). Treat as opaque string.

---

## 3. Agent Configuration Matrix

### Claude Code

| Field            | Value                                                                                                                  |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------- |
| Config format    | JSON                                                                                                                   |
| macOS path       | `~/.claude/settings.json`                                                                                              |
| Windows path     | `%USERPROFILE%\.claude\settings.json`                                                                                  |
| API key field    | `env.ANTHROPIC_API_KEY` (NOT `ANTHROPIC_AUTH_TOKEN` — fix existing stub)                                               |
| Base URL field   | `env.ANTHROPIC_BASE_URL`                                                                                               |
| Install check    | `which claude && claude --version`                                                                                     |
| Launch command   | `claude`                                                                                                               |
| Rewrite strategy | Parse JSON → set `env.ANTHROPIC_API_KEY` and `env.ANTHROPIC_BASE_URL` → atomic write                                   |
| Notes            | Key is env var, not top-level field. settings.json handles permissions/hooks/MCP. Never store key outside `env` block. |

### Codex

| Field            | Value                                                                                                                                    |
| ---------------- | ---------------------------------------------------------------------------------------------------------------------------------------- |
| Config format    | TOML                                                                                                                                     |
| macOS path       | `~/.codex/config.toml`                                                                                                                   |
| Windows path     | `%USERPROFILE%\.codex\config.toml`                                                                                                       |
| API key field    | Env var via `env_key` in `[model_providers.<id>]` (default: `OPENAI_API_KEY`)                                                            |
| Base URL field   | `base_url` in `[model_providers.<id>]` or `openai_base_url` for built-in                                                                 |
| Install check    | `which codex && codex --version`                                                                                                         |
| Launch command   | `codex`                                                                                                                                  |
| Rewrite strategy | TOML parse/edit (use `toml_edit` crate, NOT regex — fix existing stub). Set `base_url` under correct provider section. Key stays in env. |
| Notes            | Existing regex rewrite in lib.rs is fragile. Replace with proper TOML editing. Keys NEVER in config file.                                |

### Gemini CLI

| Field            | Value                                                                                       |
| ---------------- | ------------------------------------------------------------------------------------------- |
| Config format    | JSON                                                                                        |
| macOS path       | `~/.gemini/settings.json`                                                                   |
| Windows path     | `%USERPROFILE%\.gemini\settings.json`                                                       |
| API key field    | `GEMINI_API_KEY` env var ONLY                                                               |
| Base URL field   | UNVERIFIED — docs truncated                                                                 |
| Install check    | `which gemini`                                                                              |
| Launch command   | `gemini`                                                                                    |
| Rewrite strategy | STUB ONLY. OAuth-based auth, no static key injection. Display "configure manually" message. |
| Notes            | Cannot programmatically configure. Show user instructions.                                  |

### Cursor

| Field            | Value                                                                                         |
| ---------------- | --------------------------------------------------------------------------------------------- |
| Config format    | JSON (`cli-config.json`)                                                                      |
| macOS path       | `~/.cursor/cli-config.json`                                                                   |
| Windows path     | `%USERPROFILE%\.cursor\cli-config.json`                                                       |
| API key field    | IDE Settings UI only, NOT in config file                                                      |
| Base URL field   | IDE Settings UI only                                                                          |
| Install check    | `which agent && agent --version`                                                              |
| Launch command   | `agent`                                                                                       |
| Rewrite strategy | STUB ONLY. Keys managed via IDE GUI. Display "configure in Cursor Settings > Models" message. |
| Notes            | cli-config.json has NO credential fields. Binary name is `agent`, not `cursor`.               |

### Cline

| Field            | Value                                                                                              |
| ---------------- | -------------------------------------------------------------------------------------------------- |
| Config format    | JSON                                                                                               |
| macOS path       | `~/.cline/data/settings/providers.json`                                                            |
| Windows path     | `%USERPROFILE%\.cline\data\settings\providers.json`                                                |
| API key field    | `apiKey` per-provider object in providers.json                                                     |
| Base URL field   | `baseUrl` per-provider object                                                                      |
| Install check    | `which cline && cline --version`                                                                   |
| Launch command   | `cline`                                                                                            |
| Rewrite strategy | Parse providers.json → find/create NAPI provider entry → set `apiKey` and `baseUrl` → atomic write |
| Notes            | Env var support UNVERIFIED. File-based config confirmed. Use CLI `-k` flag for session override.   |

### OpenCode (detected in existing stub)

| Field            | Value                                               |
| ---------------- | --------------------------------------------------- |
| Config format    | JSON                                                |
| macOS path       | `~/.config/opencode/opencode.json`                  |
| Windows path     | `%APPDATA%\opencode\opencode.json`                  |
| API key field    | `provider_env.OPENAI_API_KEY`                       |
| Base URL field   | `provider_env.OPENAI_BASE_URL`                      |
| Rewrite strategy | Parse JSON → set provider_env fields → atomic write |

---

## 4. Architecture Decisions

### Tauri Commands Needed

| Command                 | Purpose                                | Status                                      |
| ----------------------- | -------------------------------------- | ------------------------------------------- |
| `scan_agents`           | Detect installed agents + config paths | EXISTS, needs field corrections             |
| `reconfigure_agent`     | Write base_url + key to agent config   | EXISTS, needs TOML fix + atomic writes      |
| `fetch_subscription`    | Get quota/plan from API                | EXISTS, needs response shape verification   |
| `login`                 | POST /api/user/login, return token     | MISSING                                     |
| `launch_agent`          | Spawn agent process natively           | MISSING                                     |
| `check_agent_installed` | Verify agent binary exists             | MISSING (currently only checks config file) |
| `store_credential`      | Save token to OS keychain              | MISSING                                     |
| `load_credential`       | Load token from OS keychain            | MISSING                                     |
| `get_pricing`           | GET /api/pricing (public)              | MISSING                                     |
| `get_status`            | GET /api/status (health)               | MISSING                                     |

### Secure Storage

- **Use `keyring` crate** (cross-platform OS keychain: macOS Keychain, Windows Credential Manager, Linux Secret Service)
- Add to Cargo.toml: `keyring = "3"`
- Store: service name `"tal2a"`, username `"user-token"`
- NEVER log, print, or serialize tokens to disk outside keychain
- Frontend never sees raw token — pass through Tauri commands that use it server-side

### State Management

- **React Context + useReducer** for auth state, agent list, subscription info
- No external state library (Zustand/Jotai/Redux) — YAGNI for this scope
- Persist auth state: token in keychain, last-used agent in localStorage

### Routing

- **No router library**. App has <5 screens. Use conditional rendering based on auth state + tab navigation.
- Screens: Login → Dashboard (tabs: Agents, Subscription, MCP Library, Skills Library)

### Design Palette

- **Teal/slate palette** for developer tooling aesthetic. Primary: `#0d9488` (teal-600). Background: `#0f172a` (slate-900). Surface: `#1e293b` (slate-800). Text: `#f1f5f9` (slate-100). Accent: `#f59e0b` (amber-500) for warnings/quota.
- Dark mode default. No purple/indigo.

---

## 5. Feature Spec vs API Reality

| Requirement                  | API Available                            | Implementation                                                                                                       |
| ---------------------------- | ---------------------------------------- | -------------------------------------------------------------------------------------------------------------------- |
| Dashboard login              | `/api/user/login` (POST, no auth)        | Build login form → Tauri `login` command → store token in keychain                                                   |
| Auth against tal2a.app | Verified endpoints accept Bearer         | All authenticated calls go through Tauri commands that inject token from keychain                                    |
| Agent scanner                | N/A (local filesystem)                   | Fix existing `scan_agents`: correct Claude key field, add binary existence check                                     |
| Per-agent config rewriter    | N/A (local filesystem)                   | Fix existing `reconfigure_agent`: replace regex TOML with `toml_edit`, add atomic writes, add Cline/OpenCode support |
| Native agent launcher        | N/A (local process)                      | New `launch_agent` command: `std::process::Command` with platform-appropriate spawn                                  |
| Subscription/plan view       | `/api/user/self`, `/dashboard/billing/*` | Fix `fetch_subscription`, add billing endpoints                                                                      |
| Quota display                | `/api/user/self` (used_quota, quota)     | Part of subscription view                                                                                            |
| MCP library sync             | NO ENDPOINT                              | Stub UI: "Coming soon" placeholder with disabled state                                                               |
| Skill library sync           | NO ENDPOINT                              | Stub UI: "Coming soon" placeholder with disabled state                                                               |
| Credential safety            | N/A                                      | `keyring` crate, no logging, atomic writes                                                                           |
| Cross-platform               | Tauri v2                                 | Platform conditionals for paths, keyring handles OS differences                                                      |
| Atomic writes                | N/A                                      | Write to `.tmp` → rename (Rust `std::fs::rename` is atomic on same filesystem)                                       |
| Error boundaries             | N/A                                      | React ErrorBoundary component wrapping each screen                                                                   |
| Input validation             | N/A                                      | Zod schemas for login form, agent config inputs                                                                      |

---

## 6. Build Order

Each step is independently assignable. Dependencies noted.

### Step 1: Fix Rust Backend Foundation

- Correct `scan_agents`: Claude key field → `env.ANTHROPIC_API_KEY` (not `ANTHROPIC_AUTH_TOKEN`)
- Add `keyring = "3"` and `toml_edit = "0.22"` to Cargo.toml
- Replace regex TOML rewrite in `reconfigure_agent` with `toml_edit`
- Implement atomic write helper: write to `{path}.tmp` → `std::fs::rename`
- Add `login` command: POST `/api/user/login`, extract token from response, store in keyring
- Add `store_credential` / `load_credential` commands using keyring
- Add `check_agent_installed`: verify binary exists via `which`/`where`
- Add `launch_agent`: `std::process::Command` with detached spawn
- Add `get_pricing` and `get_status` commands
- **Depends on**: Nothing
- **Verification**: `cargo build` succeeds, all commands callable from frontend

### Step 2: Frontend Auth Flow

- Create `frontend/src/lib/auth.ts`: auth context, login/logout actions, token management via Tauri commands
- Create `frontend/src/pages/LoginPage.tsx`: username/password form, Zod validation, error display
- Create `frontend/src/components/ErrorBoundary.tsx`
- Wire App.tsx: show LoginPage when unauthenticated, Dashboard when authenticated
- Style with teal/slate palette in `index.css`
- **Depends on**: Step 1 (login command)
- **Verification**: Can log in with valid credentials, token stored in keychain, persists across restart

### Step 3: Agent Scanner + Config Rewriter UI

- Create `frontend/src/pages/AgentsPage.tsx`: list detected agents, status badges, reconfigure form per agent
- For each agent: show detected/not-detected, config path, edit base_url + API key fields
- On save: call `reconfigure_agent`, show success/error toast
- For stub agents (Gemini, Cursor): show manual instructions instead of edit form
- Add "Launch" button per agent → `launch_agent` command
- **Depends on**: Step 1 (scan_agents, reconfigure_agent fixed)
- **Verification**: Can scan agents, reconfigure Claude/Codex/Cline, launch detected agents

### Step 4: Subscription + Quota Dashboard

- Create `frontend/src/pages/SubscriptionPage.tsx`: plan name, quota bar (used/limit), reset date, pricing table
- Call `fetch_subscription` on mount, `get_pricing` for model list
- Handle loading/error states
- **Depends on**: Step 1 (fetch_subscription, get_pricing)
- **Verification**: Shows real quota data when logged in, graceful error when API fails

### Step 5: MCP + Skills Library Stubs

- Create `frontend/src/pages/McpLibraryPage.tsx`: disabled card grid with "Coming Soon" overlay
- Create `frontend/src/pages/SkillsLibraryPage.tsx`: same pattern
- Add tab navigation to Dashboard
- **Depends on**: Step 2 (auth flow for protected route)
- **Verification**: Pages render, clearly marked as unavailable

### Step 6: Polish + Safety

- Add input sanitization to all forms
- Add rate-limit handling (429 → exponential backoff)
- Add network error retry logic
- Ensure no token appears in console.log, error messages, or UI
- Test atomic writes: kill mid-write → config not corrupted
- Cross-platform path testing (Windows paths via cfg! macros)
- **Depends on**: Steps 1-5
- **Verification**: Manual security audit passes, no credential leaks

---

## 7. Constraints & Safety Rules

### Credential Handling

- Tokens stored ONLY in OS keychain via `keyring` crate
- NEVER log tokens: mask in all error messages (`format!("Bearer {}...{}", &t[..4], &t[t.len()-4..])`)
- NEVER pass tokens to frontend as strings — Tauri commands use them internally
- Config file rewrites: atomic (tmp + rename), never partial writes
- API keys in agent configs: written only to designated fields, never duplicated

### Atomic Writes

- Pattern: `write_to_temp(path) → fs::rename(temp, path)`
- Temp file: `{original_path}.napi-tmp`
- Rename is atomic on POSIX and NTFS when same filesystem
- On failure: clean up temp file, return error, original untouched

### Cross-Platform

- Paths: use `dirs` crate or env vars (`HOME`/`USERPROFILE`/`APPDATA`)
- Line endings: preserve original file's line endings when rewriting configs
- Process spawn: `Command::new("cmd").args(["/c", "start", ...])` on Windows, `Command::new("open")` on macOS, `xdg-open` on Linux
- Keyring: works cross-platform automatically

### No Hardcoded Secrets

- Base URL configurable (default `https://tal2a.app`)
- No API keys in source code
- No test credentials committed

### Error Handling

- All Tauri commands return `Result<T, String>` with user-friendly error messages
- Frontend: ErrorBoundary wraps every screen
- Network errors: retry with backoff, max 3 attempts
- Config parse errors: show original content backup option

---

## 8. Testing Strategy

### Unit Tests (Rust)

- `scan_agents`: mock HOME, verify correct paths and detection
- `reconfigure_agent`: round-trip test (read → modify → read back → assert fields changed)
- Atomic write: interrupt simulation, verify no corruption
- Keyring: store/load/delete cycle
- Target: 80% coverage on lib.rs

### Integration Tests (Frontend)

- Login flow: valid creds → authenticated state, invalid creds → error display
- Agent scan: renders detected agents correctly
- Config rewrite: form submission → success feedback
- Subscription: loading state → data display → error fallback
- Target: critical paths covered

### Manual Verification Checklist

- [ ] Login with real credentials succeeds
- [ ] Token persists after app restart
- [ ] Claude Code config rewritten correctly (JSON valid, env block intact)
- [ ] Codex config rewritten correctly (TOML valid, provider section intact)
- [ ] Agent launch opens terminal/process
- [ ] Quota displays real numbers
- [ ] MCP/Skills pages show stub state
- [ ] No tokens in DevTools console, network tab shows Bearer header only
- [ ] Kill during config write → original file intact
- [ ] Works on macOS AND Windows (or document platform gaps)

### What NOT to Test

- E2E browser automation (overkill for desktop app)
- Visual regression (no design system to regress against)
- Performance benchmarks (desktop app, not latency-sensitive)
