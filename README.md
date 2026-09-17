# tal2a

A desktop control panel for the [NAPI](https://tal2a.app) AI gateway. It finds the
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
git clone https://github.com/tal2a-dev/desktop.git
cd desktop

bun install

bun run tauri dev
```

This repo often lives on ExFAT (`nvm_500`). macOS writes AppleDouble `._*` sidecars there; Tauri's build script then panics on `._default.toml` (not UTF-8). `bun run tauri` forces `CARGO_TARGET_DIR` onto APFS at `~/.cache/tal2a/target`. Override with `TAL2A_CARGO_TARGET_DIR` if needed.

For a release build:

```bash
bun run tauri build
```

## Releases and auto-update

Every push to `main` runs `.github/workflows/release.yml`:

| Platform | Artifact |
| --- | --- |
| macOS | universal `.dmg` / `.app` (Apple Silicon + Intel) |
| Windows | `.msi` / `.exe` |
| Linux | `.AppImage` / `.deb` |

The app checks `https://github.com/tal2a-dev/desktop/releases/latest/download/latest.json` and can install from Settings → About.

First release needs the updater signing secret — see `.github/SECRETS.md`.

---

## Configuration

There is none, and nothing is baked into the binary:

| Value                      | Where it comes from                                                 |
| -------------------------- | ------------------------------------------------------------------- |
| Gateway base URL           | Defaults to `https://tal2a.app`, editable in the app                |
| GitHub OAuth client ID     | Read at runtime from the gateway's `/api/status`                    |
| GitHub OAuth client secret | Never reaches this app — the gateway holds it and runs the exchange |

No `.env` file, no build-time secrets. A desktop binary is decompilable, so anything embedded
would be extractable by whoever has the app; keeping the client secret server-side avoids that
entirely.

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

Two methods, both ending in the same place:

1. **Username + password** — supports the gateway's two-factor flow. When the server returns a
   `flow_token` with `require_verification: true`, the app prompts for a 2FA code and completes
   the exchange.
2. **GitHub OAuth** — opens the system browser and listens for the redirect on a loopback
   server at `http://localhost:9876/callback`.

Both paths store the resulting session token in the OS keyring.

The OAuth flow is entirely server-driven, which is what makes it workable from a desktop app:

1. The app asks the gateway to mint a state — `POST /api/oauth/state` with
   `{ provider: "github", intent: "login" }`. The gateway stores it with a 10-minute TTL. The
   app cannot invent this value; assuming it could was the original bug.
2. The app opens GitHub's authorize URL carrying that state and the loopback redirect URI.
3. GitHub bounces the browser back to `localhost:9876`, and the app answers with a plain
   "signed in" page — on failure too, so a rejection never looks like a dead connection.
4. The app hands `code` + `state` to `GET /api/oauth/github`. **The gateway holds the client
   secret and performs the GitHub exchange itself**, then applies the same login policy it
   uses for a password sign-in — so a 2FA-protected account lands in the same code prompt.

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
│   ├── build.rs
│   └── tauri.conf.json
└── vite.config.ts             # root: "frontend", outDir: "../dist"
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

- **No secret is embedded in the binary.** The OAuth client secret stays on the gateway; the
  app only handles a client ID, which is public
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
