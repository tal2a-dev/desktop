# Release secrets

The updater will not ship until these GitHub Actions secrets exist on `tal2a-dev/desktop`.

| Secret | Value |
| --- | --- |
| `TAURI_SIGNING_PRIVATE_KEY` | Full contents of local `.tauri/tal2a.key` (gitignored). Generate once with `bunx tauri signer generate -w .tauri/tal2a.key --ci`. The matching **public** key is already in `src-tauri/tauri.conf.json`. |
| `TAURI_SIGNING_PRIVATE_KEY_PASSWORD` | Only if the key was generated with a password. Empty is fine. |

```bash
gh secret set TAURI_SIGNING_PRIVATE_KEY --repo tal2a-dev/desktop < .tauri/tal2a.key
```

Optional Apple notarization (unsigned macOS builds still download; Gatekeeper needs right-click → Open):

| Secret | Value |
| --- | --- |
| `APPLE_CERTIFICATE` | Base64 `.p12` of a Developer ID Application cert |
| `APPLE_CERTIFICATE_PASSWORD` | `.p12` password |
| `APPLE_ID` | Apple ID email |
| `APPLE_PASSWORD` | App-specific password |
| `APPLE_TEAM_ID` | Team ID |

`GITHUB_TOKEN` is provided by Actions. The workflow needs `contents: write` (already set).
