# llm_wiki-web

This single-user, loopback Web host assembles the original React UI and Rust business modules from pinned upstream `e08cfce65aad8b76b13546a081c1a8d4422b70c3`. All 475 original files remain unchanged. It runs the original Wiki import and Agent logic without a desktop or noVNC.

The host now has two distinct model modes:

- `live`: uses the model saved through the original Settings UI. A missing configuration is an error; no synthetic answer or model fallback is substituted.
- `fixture`: starts the deterministic local protocol model for isolated regression checks. Only its own exact endpoint is allowed.

## Repository layout and upstream dependency

This repository contains only the external Web host, adapters and checks. The original desktop source is a build dependency, not part of the current checkout. `prepare.py` fetches the pinned upstream commit into `.cache/upstream.git` on first use, then exports and verifies it in the output directory. Subsequent assemblies reuse the cache. Git and network access to GitHub are required for the initial fetch.

For an offline build, pass `--source /path/to/local/llm_wiki` containing the pinned commit. A custom `--upstream-cache /path/to/cache.git` keeps the cache on another disk. No dependency on a specific sibling checkout is required.

- `templates/`: external Rust host and browser adapters
- `prepare.py`, `Dockerfile`: dependency assembly and builds
- `run_probe.py`: runtime launcher
- `verify_*.py`, `test_workspace_mode.py`: validation
- `CONTAINER.md`, `VALIDATION.md`: deployment and evidence

## Assemble and build

From the repository root, choose a new output directory on a disk with sufficient space:

```sh
python3 prepare.py --output /path/to/new-host
cd /path/to/new-host
npm ci
npm run typecheck
npm run build
cargo build --manifest-path backend/Cargo.toml --release --bin external-host-smoke
python3 verify_source.py
python3 run_probe.py --mode live --workspace /path/to/private-workspace --port 18902
```

Use the one-use URL printed at startup to sign in. The listener is loopback-only. A new workspace starts with an empty `Web Wiki` project. Open **Settings → LLM Models**, configure and activate an HTTP provider, and leave Chat/Ingest on **Use active model** for this version. Settings are saved atomically with mode 0600. Do not commit the workspace or credentials to source control.

For local Ollama, set its base URL and an already installed model name. Reasoning can be set separately for Chat and Ingest through the original UI. The real-model check used `qwen3-wiki:latest` (4B) for Wiki generation and `qwen3-8b:latest` for Agent Q&A; thinking was turned off. The 4B model did not reliably follow the Agent's JSON action protocol. A small model's successful connection test does not prove Agent compatibility.

Run `run_probe.py --mode fixture` with a separate workspace for the isolated synthetic harness. The runner records its mode and rejects accidental cross-mode reuse, including legacy live credentials, instead of resetting them. The script keeps fixture mode as its CLI default for compatibility; the Docker image explicitly defaults to live mode.

For development, `prepare.py --node-modules /absolute/path/to/node_modules` explicitly reuses a dependency cache instead of `npm ci`. Linux native Tauri/GTK/WebKit/protobuf dependencies are required. See [CONTAINER.md](CONTAINER.md) for Docker and build-proxy configuration.

## Shared model configuration and transport

Browser generation requests and backend Agent turns use the current saved `llmConfig`. Each Agent turn captures one configuration snapshot. The authenticated HTTP relay accepts only the exact generation URL(s) derived from that configuration, disables redirects, preserves response streaming/status/Retry-After, and rejects truncated responses rather than fabricating completion. Model credentials and required headers are rebuilt server-side; browser-supplied credentials, cookies and hop-by-hop headers cannot replace them.

The external build graph includes the original provider implementation and appends a small helper in that module's scope, so URL normalization and authentication reuse the author's private helpers. The original provider file is not edited or replaced. The external file-sync scheduler similarly reuses the original file-sync module. Original source hashes are checked at build/startup.

## Validation and current boundaries

The real-model browser workflow passed: import a synthetic text → original analysis/generation → six Wiki files → native Agent `wiki.read_page` → correct random verification code → follow-up answer from the same source → history restored after page reload. See [VALIDATION.md](VALIDATION.md) for the evidence and the initial model failures that were observed.

Transport regression checks cover saved credentials, live/fixture separation, exact destination checks, UTF-8 streaming, HTTP errors/rate limits, redirects, truncated upstream streams, concurrent UI RPC, dynamic Agent configuration and process restart. No paid provider or existing knowledge base was used for these checks.

This is not yet complete desktop parity. Remaining work includes task-specific provider overrides, complete browser upload/download/assets, CLI providers, cloud parsing services, project lifecycle edge cases, interrupted-job recovery and long-running/public deployment. Only the active HTTP model is bridged for now. The relay and original backend Agent currently use a 180-second request timeout; the longer timeout setting shown in the original UI is not yet bridged. Existing upload/asset/clipper/API controls that lack a host route return explicit unsupported errors; they do not silently succeed. Import remains browser-driven and currently uses server-side files.

The HTTP provider families supported by the original Rust provider are wired through their original helpers, but real remote OpenAI/Anthropic/Google/Azure/MiniMax endpoints still require separate credential-backed validation. Current real-model evidence is local Ollama only. Keep user data and keys separate from image/source artifacts; workspace file operations are confined to the configured root.

In the assembled output, the original source and LICENSE remain under `upstream/`. Keep upstream attribution and license files when distributing artifacts. New host files are under `backend/` and `platform/` in the assembled layout.

## Attribution and license

Built on [nashsu/llm_wiki](https://github.com/nashsu/llm_wiki), pinned to the revision above. Distributed under GNU GPL v3; see [LICENSE](LICENSE). The original application is fetched unchanged during assembly.
