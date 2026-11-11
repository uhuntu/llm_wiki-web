# Validation — 2026-11-06

This document records checks for the independent source assembly, not a production-readiness claim.

| Check | Result |
|---|---|
| Export pinned Git revision into a new output directory | Passed; 475 original files |
| Refuse existing output without replacing its manifest | Passed |
| TypeScript check and Vite build in new assembly | Passed using explicitly supplied local node_modules cache |
| Rust build with upstream subtree mounted read-only | Passed, offline, using existing dependency/toolchain caches |
| Generate Rust module paths from actual build directory | Passed at `/verification`; no old experiment path |
| Run from a different root without build caches mounted | Passed at `/delivery`, non-root, network disabled |
| Original project/file commands and synthetic Agent smoke | Passed |
| Serve original compiled UI HTML/JS resources | Passed; HTTP checks, not a new browser flow certification |
| Reject file access outside fixture workspace | Passed |
| Runner forwards termination and permits immediate restart | Passed |
| Same project/settings/file survive process restart | Passed |
| Original file hashes after build and runtime | All 475 unchanged |
| Offline Docker scratch target and exported-image inspection | Passed with the local legacy Docker builder; synthetic env/cookie files, fixtures, dependency cache and untracked expanded-source files excluded; source archive hash unchanged |

The earlier local Rust executable build used `cargo rustc --offline --release --bin external-host-smoke -- -C lto=off -C opt-level=0` to shorten prototype feedback; these flags affect the host executable, not the pinned original source bytes. The existing build image supplies native dependencies. That cached check was followed by the fresh Docker verification below.

Earlier browser checks in the development layout covered original UI chat, file sync, restart and synthetic text import. This new layout separately passed source assembly and relocated runtime checks. Real providers, full parsing/assets/uploads/CLI and interrupted browser-driven ingest recovery remain pending.

No live data, credentials, cookies, compiled artifacts or existing runtime state were copied into this source experiment. No remote repository or shared server was changed.

The first context check detected an unknown synthetic `backend/private.env` in the image. The generated ignore rules now exclude directory contents before allowing individual files; the subsequent image export passed. No real credentials were involved. The initial attempt was blocked by system-disk space and registry connectivity. The fresh Docker verification below resolved those blockers. See [CONTAINER.md](CONTAINER.md) for the build definition and usage.

## Fresh Docker verification

Passed on Linux/amd64 in a new isolated Docker data directory on `/mnt/BACKUP/llmwiki-clean-build-20261106`. The existing Docker daemon remained running. No pre-existing npm, Cargo, Rust target, application-state or user-document directories were mounted into build stages. Retries reused stages produced during this verification run. This is a fresh dependency/container verification on the existing host, not a new physical-machine installation or a bit-for-bit reproducibility claim.

| Check | Result |
|---|---|
| Base images downloaded into the isolated daemon | Node 24.22.1-bookworm, Rust 1.99.1-bookworm, Debian bookworm-slim |
| npm ci from the original lockfile | Passed; 819 packages, followed by TypeScript and Vite checks |
| Native Debian build dependencies | Passed after adding libprotobuf-dev; the initial failure was missing google/protobuf/empty.proto with --no-install-recommends |
| cargo build --locked --release | Passed in 28m 01s with 2 jobs, original LTO, opt-level=s and panic=unwind settings |
| All Docker stages and final image | Passed; llmwiki-unchanged-host:clean |
| Final image with network disabled and read-only filesystem | Passed as UID/GID 10001, with a temporary data volume and /tmp writable, all capabilities dropped |
| Native project/file commands, Agent tools/events/session reload/cancellation | Passed using synthetic content and a loopback model fixture |
| Original UI resources, file RPC, outside-workspace rejection | Passed |
| Runner shutdown and project/settings/file persistence across process restart | Passed |
| Original source hashes inside final image after runtime | All 475 unchanged |
| Build CA and proxy environment in final image | Confirmed absent |
| Updated context exclusion check with optional build CA | Passed using synthetic markers |

Final image ID: `sha256:3e0caf87bc1b0d4861b40d941e591554d02f1b0979015ba0cf09bd702ce509bc`.

The build network required the host's public CA bundle and both uppercase/lowercase proxy build arguments; TLS verification remained enabled. The public CA bundle is confined to build stages. The final runtime requires neither the build proxy nor its CA. The image archive and checksum are retained under the build directory's `result/` folder.

These checks do not certify real model providers, complete parsing/import/upload/assets/CLI behavior, long-running deployment or full desktop parity. The new image's UI resource checks are HTTP checks; earlier browser flows remain separate evidence.

## Live-model milestone — 2026-11-10

The earlier synthetic-only limitation is superseded by the live mode added on this date. Full desktop parity is still pending. This milestone used new synthetic content in a separate workspace, never an existing knowledge base or paid API.

| Check | Result |
|---|---|
| Compile new host against unchanged original provider/Agent modules | Passed; all 475 original blobs unchanged |
| Default optimized release rebuild from this project's cached dependencies | Passed in 7m 45s, original LTO and opt-level=s; no new fresh-dependency claim |
| Fixture-mode native Agent/files/UI resources/restart regression | Passed offline, non-root, read-only root filesystem |
| Live-mode protocol integration | Passed: exact configured destinations, server-owned auth/custom headers, UTF-8 streams, 401/429/Retry-After, no redirects, truncated streams fail, UI RPC remains responsive during inference |
| Agent model selection and saved configuration restart | Passed using a local protocol server; no fixture fallback |
| Live/fixture workspace separation | Four runner tests passed, including preserving legacy live credentials and rejecting symlink markers |
| Original React Settings UI | Saved/activated local Ollama model; functional test returned the expected token |
| Original React source import with a real model | Passed with qwen3-wiki:latest (4B): analysis, generation and six committed Wiki files |
| Original ingest cache | Raw source SHA-256 matched the committed cache entry |
| Real backend Agent tool execution | qwen3-8b:latest called wiki.read_page on the generated source summary |
| Real two-turn browser Q&A | Correct random code JADE-B7D1FDFF, then maintenance interval 17 days, both with source references |
| Browser reload | Both turns and Agent read events restored |

The source was `quartz-beacon-67bbe6.txt`, describing a fictional device. The generated source summary preserves the random code, teal indicator, 17-day interval and six-second reset procedure. Native Agent and UI chat records both persisted under the new project's `.llm-wiki/` directory.

Observed limitations were retained as evidence: the first 4B analysis reached the 4096-token output limit and the original queue retried successfully; a duplicate source-summary block was rejected by the original guard. The 4B Agent produced a rejected non-JSON action and an incorrect answer. Switching through the original UI to the already installed 8B model, with thinking off, produced valid read-page actions and correct answers. No parser checks were relaxed, business prompts changed, or synthetic answers substituted.

Builds and test data are under `/mnt/BACKUP/llmwiki-clean-build-20261106/live-20261110`. Screenshots and verification records are also retained under the repository's `.runtime/live-model-20261110/`. The current full source-build Dockerfile defaults to live mode; the CLI runner retains fixture as its compatibility default and protects workspaces from cross-mode reuse.

Unverified or unsupported areas remain documented in README: real remote providers, per-task model overrides, browser uploads/downloads/assets, CLI and cloud parsing transports, project lifecycle edge cases, longer configured timeouts, public deployment and long-running reliability.

### Packaged runtime verification

The optimized image `llmwiki-unchanged-host:live-20261110` (`sha256:88f11d090f158d3a9e1a17eda34b9a929e5d081679819c75a3c1a534c9401d39`) passed both fixture and live transport suites inside its own runtime, offline with a read-only root filesystem. All 475 original source blobs passed verification again.

The preview was migrated to normal Docker container `llmwiki-web-live-preview`, UID 1000, read-only root filesystem, capabilities dropped and no-new-privileges. The temporary privileged builder is stopped. Existing Wiki pages, model settings, both correct answers and read events survived the container replacement. A third ambiguous reset follow-up received an incorrect claim that the source had no procedure; the source does contain it. This is retained as a real-model quality failure, not counted as successful Q&A.

Preview: http://localhost:18922/ (local authenticated session). Deliverables: `/mnt/BACKUP/llmwiki-clean-build-20261106/live-20261110/result/`, including runtime image archive, external-host source archive and SHA256SUMS. Data and build caches remain on the backup disk.

An explicit fourth request to reread the source triggered the original wiki.read_page tool and correctly returned the blue-button six-second reset procedure. The third-turn error and fourth-turn correction remain visible together in the packaged-runtime screenshot.

## Standalone repository layout — 2026-11-10

The external host now lives at repository root in `/home/hunt/work/llm_wiki-web`. The upstream application is absent from the current checkout and retained in Git history for attribution. Assembly accepts an explicit offline `--source`, or fetches the same pinned commit from the official repository into an ignored bare cache. This layout change does not alter runtime adapters or the previously tested image.

Layout checks passed: assembly from an explicit local upstream repository, assembly from a separate prepopulated bare cache, 475-blob integrity checks for both outputs, and four workspace-mode tests. First-time GitHub fetch did not complete in the current network and was stopped; online fetch is not claimed as verified. Fetches now have a 120-second timeout. No runtime rebuild was needed for path-only moves and dependency assembly changes.

## Browser upload implementation — 2026-11-10

Added authenticated POST `/_llmwiki/upload` to the external host, using the existing browser file chooser. Files stream into private, unique workspace staging directories, capped at 100 MiB per file and four active uploads. No project file is overwritten; unsuccessful staged uploads are removed.

The optimized release build passed in 8m 06s using cached dependencies. TypeScript checks and frontend production build passed. All 475 original source blobs remained unchanged. Packaged image `llmwiki-unchanged-host:upload-20261110` has ID `sha256:ca15c1a67eb2836807a6849cb1918449b327ab1c670596773260d11ab5d52451`.

`verify_uploads.py` passed inside the offline, non-root, read-only runtime: login and foreign-origin rejection; UTF-8 names/bytes; traversal and invalid filename rejection; unique same-name files; private permissions; empty files; a real streamed 100 MiB file; rejection above the limit; interrupted upload cleanup; staging symlink rejection; and reading the result through the original read_file command. Interrupted HTTP requests may close without a response; either a 400 or closed connection is accepted, but success or leftover partial files fail the check. Existing fixture/Agent/persistence and live model transport suites also passed on this image.

The localhost:18922 preview now runs this image. Its previous container remains stopped as `llmwiki-web-live-preview-preupload-20261110`, and the synthetic workspace was backed up before replacement. Artifacts are under `/mnt/BACKUP/llmwiki-clean-build-20261106/upload-20261110/result/`.

Browser inspection confirmed the original Sources → Import → From computer control and the file chooser opening. Automatic setFiles was blocked by the ChatGPT Chrome extension requiring Allow access to file URLs. No extension permission was changed. End-to-end browser upload/import is therefore pending user assistance, not claimed as passed. Upload staging retention is manual; downloads/assets, parser parity and other previously listed limitations remain pending.

### Browser upload/import verified after manual selection

The user manually selected the supplied synthetic text in Chrome without changing extension permissions. The authenticated upload and the original raw-source import both preserved all 173 bytes. Original Ollama analysis and generation completed, writing four files: the source summary, index, overview and log. The generated summary preserves COPPER-92F6 and the 23-day interval. Its successful ingest-cache SHA-256 matches the raw source.

The queue created three unresolved review suggestions (two missing-page suggestions and one enrichment suggestion); these are retained, not approved automatically. The source badge initially remained Not ingested after generation, then correctly showed Ingested after the built-in refresh. This stale display is recorded as a remaining UI refresh issue. The prior pending-browser statement above is superseded for this manually selected text workflow. Other formats and automatic chooser control remain unverified. Screenshot: `.runtime/upload-20261110/success.jpg` (local evidence, not committed).

## Authenticated asset delivery — 2026-11-11

Added the original `convertFileSrc` route. It serves regular files only from the canonical workspace, requires the same one-use-login cookie as other Web routes, rejects traversal and symlink escapes, uses UTF-8-safe inline names and known media MIME types, disables caching, and applies a sandbox CSP to HTML/SVG/XML. The preview was updated to image `sha256:7974574bd45360f92a243a834c373e146d52b8a69483474711ec1a20152d0dba`; its read-only non-root container restored the existing Wiki and source history. The previous preview remains stopped for rollback.

The original source page and uploaded raw text render in the updated UI. The asset response route compiled into this image; a direct browser download/asset fetch was not separately exercised in this turn. HTTP byte ranges and directory-browser route support remain pending.

### Direct asset-route verification — 2026-11-11

The packaged download image passed direct HTTP checks in an offline, non-root, read-only container: exact UTF-8 file bytes and encoded filename headers, login enforcement, traversal rejection, symlink escape rejection, and sandbox CSP for HTML. These checks supersede the untested direct-route statement above. Browser project export and byte ranges remain separate pending work. Reproduce by mounting `verify_asset_delivery.py` as `/check.py` and running it with the image's Python entrypoint.

### Browser project export — 2026-11-11

Original Settings → Maintenance → Export project completed and Chrome downloaded `/home/hunt/Downloads/Web Wiki.llmwiki.zip` (14,025 bytes, 41 ZIP entries). ZIP integrity passed; the generated Chinese source summary, original uploaded text and ingest-cache.json match their project files byte for byte. SHA-256: `7a34963243f632893d69233331d20e39f0de14678d735fd300b07cf91976b6b9`. This supersedes the pending browser-export statement above. Local screenshot: `.runtime/download-20261111/export-success.jpg`.
