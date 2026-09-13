# NAPI Desktop

A desktop control panel for the [NAPI](https://napi.mikawi.org) AI gateway. It finds the
coding agents already installed on your machine, points them at NAPI in one click, launches
them, and shows your plan and quota — so you stop hand-editing JSON and TOML config files.

Built with **Tauri v2** (Rust) + **React 19** + **TypeScript**.

---

## What it does

| Feature               | Detail                                                                                                                                                                         |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| **Agent detection**   | Scans 15 coding agents — checks both the CLI binary on `PATH` and the config file on disk                                                                                      |
| **One-click setup**   | Rewrites each agent's config to point at your NAPI endpoint, using the API key already in your OS keyring                                                                      |
| **Agent launcher**    | Starts any installed agent CLI from the app                                                                                                                                    |
| **Subscription view** | Live plan, quota bar, reset date, and the model catalog                                                                                                                        |
| **MCP inspector**     | Read-only scan of MCP servers configured across your agents                                                                                                                    |
| **Skills inspector**  | Read-only scan of skills installed across your agents                                                                                                                          |
| **Credential safety** | Tokens live in the OS keychain — macOS Keychain, Windows Credential Manager, Linux Secret Service. They never cross the UI boundary and are never written to disk in plaintext |

Config writes are **atomic** (temp file + rename) so an interrupted write can't corrupt an
agent's config.

---

## Requirements

- **Node.js 20+** and **[Bun](https://bun.sh)** (package manager and script runner)
- **Rust** stable toolchain — [rustup.rs](https://rustup.rs)
- Platform build deps for Tauri v2 — see [tauri.app/start/prerequisites](https://tauri.app/start/prerequisites/)
  - macOS: Xcode Command Line Tools
  - Windows: Microsoft C++ Build Tools + WebView2
  - Linux: `webkit2gtk`, `libayatana-appindicator`, `librsvg2-dev`

---

## Quick start

```bash
git clone https://github.com/api-tal2a/desktop.git
cd desktop

bun install
cp .env.example .env      # then fill in GITHUB_CLIENT_SECRET — see below

bun run tauri dev
```

For a release build:

```bash
bun run tauri build
```

---

## Configuration

`.env` is **gitignored**. `.env.example` is the committed template.

| Variable               | Required                | Purpose                                                   |
| ---------------------- | ----------------------- | --------------------------------------------------------- |
| `GITHUB_CLIENT_SECRET` | Only for GitHub sign-in | OAuth client secret for the "Continue with GitHub" button |

`build.rs` reads `.env` from the repo root and injects values as compile-time environment
variables. Nothing is read at runtime, and **no secret is ever written into source**. If the
variable is missing the app still builds — the GitHub button reports a clear error and
username/password sign-in keeps working.

The client ID and gateway URL are fetched at runtime from `/api/status`, so they are not
duplicated in `.env`.

> **Desktop OAuth note.** A desktop binary is decompilable, so an embedded OAuth client
> secret is extractable by anyone who has the app. That is expected for installed-app OAuth
> flows. If that trade-off is unacceptable, the exchange must move server-side.

---

## Supported agents

**Auto-configured** — the app writes the config for you:

| Agent            | Config file                             | Format |
| ---------------- | --------------------------------------- | ------ |
| Claude Code      | `~/.claude/settings.json`               | JSON   |
| OpenAI Codex CLI | `~/.codex/config.toml`                  | TOML   |
| Cline            | `~/.cline/data/settings/providers.json` | JSON   |
| OpenCode         | `~/.config/opencode/opencode.json`      | JSON   |

**Guided** — these store credentials in their own GUI or a proprietary store, so the app
detects them and tells you the endpoint to enter manually:

Gemini CLI · Cursor · Continue · Goose · Factory Droid · Grok CLI · Roo Code ·
Kilo Code · Hermes Agent · Qwen Code · Windsurf

---

## Sign-in

Two methods:

1. **Username + password** — supports the gateway's two-factor flow. When the server returns
   a `flow_token` with `require_verification: true`, the app prompts for a 2FA code and
   completes the exchange.
2. **GitHub OAuth** — opens the system browser, catches the redirect on a local loopback
   server (`http://localhost:9876/callback`), and exchanges the authorization code for a
   token.

Both paths store the resulting token in the OS keyring.

> **GitHub sign-in is incomplete.** The client-secret exchange against GitHub succeeds — a
> valid GitHub access token is returned — but the gateway has no endpoint that converts a
> GitHub access token into a NAPI session token. `/api/oauth/github` requires a server-side
> `state` value that only the web flow can produce. Completing this needs a new endpoint:
> `POST /api/oauth/github/desktop { code, redirect_uri }`.
>
> Username/password sign-in is fully functional.

---

## Architecture

```
desktop/
├── frontend/                  # React source (Vite root)
│   └── src/
│       ├── components/        # ErrorBoundary
│       ├── lib/auth.tsx       # Auth context, reducer, Tauri bridge
│       └── pages/             # Login, Agents, Subscription, MCP, Skills
├── src-tauri/
│   ├── src/lib.rs             # 19 Tauri commands — the entire backend
│   ├── build.rs               # Loads .env into compile-time env vars
│   └── tauri.conf.json
├── vite.config.ts             # root: "frontend", outDir: "../dist"
└── .env.example
```

All privileged work happens in Rust and is exposed to the UI as Tauri commands:

```
scan_agents          reconfigure_agent     configure_agent      auto_configure_all
auto_setup           check_agent_installed launch_agent
login                verify_2fa            github_oauth
store_credential     load_credential       clear_credential     ensure_api_key
fetch_subscription   get_pricing           get_status
scan_mcp_servers     scan_skills
```

The frontend never receives an API key or session token. Commands that need a credential read
it from the keyring themselves.

---

## Security

- Secrets are read from `.env` at **build time** only — never committed, never in source
- Session tokens and API keys live in the **OS keychain**, not files
- Tokens never cross the IPC boundary to the UI
- Agent config writes are **atomic** (`.napi-tmp` + rename); a failure leaves the original intact
- HTTP calls retry with exponential backoff on `429` and transient failures
- `base_url` is validated before use; agent binary names are charset-restricted before being
  passed to a shell
- `.gitignore` excludes `.env`, build output, and machine-local agent state

---

## Known limitations

- **MCP and Skills pages are read-only inspectors**, not sync clients. They report what is
  configured locally. The gateway exposes no MCP or skill registry endpoints.
- **GitHub sign-in does not complete** — see the note above. Blocked on a server endpoint.
- **11 of 15 agents are guided, not automatic**, because they keep credentials somewhere the
  app cannot safely write.
- **Codex API keys are not written by the app.** Codex reads its key from the
  `OPENAI_API_KEY` environment variable; the app writes only `base_url` and tells you to
  export the key.

---

## Development

```bash
bun run dev          # Vite dev server only (frontend, no Rust shell)
bun run tauri dev    # full app with hot reload
bun run build        # type-check + production frontend build
bun run tauri build  # packaged desktop binary

cd src-tauri && cargo check   # Rust only
cd src-tauri && cargo test    # Rust unit tests
```

Rust unit tests (14) cover atomic-write failure handling, base-URL validation, binary-name
injection rejection, token masking, URL credential redaction, the keyring
store/load/clear cycle, a Claude config round-trip, quota conversion, and
credential non-leakage in the MCP scan.

---

## License

Not yet specified.
