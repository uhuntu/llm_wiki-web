# Container build definition

This image runs the single-user Web host and defaults to live HTTP model configuration through the original Settings UI. Complete desktop parity and public deployment remain pending. Current definition targets Linux/amd64.

Assemble a new context first; Docker does not read the working source tree:

```sh
python3 experiments/unchanged-web-host/prepare.py --source . --output /tmp/llmwiki-container-context
docker build --platform linux/amd64 -t llmwiki-unchanged-host:experiment /tmp/llmwiki-container-context
```

The builder installs Node dependencies with npm ci, builds the original frontend, then compiles the external Rust host from the original Cargo lockfile. The runtime uses a non-root UID, keeps data separate under /data, and verifies upstream source hashes on startup. It still needs native Tauri/GTK/WebKit libraries; it does not run a desktop or remote desktop.

For this Linux loopback-only experiment, use host networking; publishing a bridge port does not redirect the application's internal loopback listener:

```sh
docker volume create llmwiki-host-fixtures
docker run --rm --init --network host \
  --mount type=volume,source=llmwiki-host-fixtures,target=/data \
  llmwiki-unchanged-host:experiment
```

Open the one-use login URL printed by the process. Start with a separate workspace while validating. For local Ollama, host networking lets the configured loopback URL reach the host service. For an isolated fixture run, override the image command with `--mode fixture --port 18902`. A new live workspace has no model credentials; configure one in Settings. Restarting the process prints a fresh one-use login URL.

## Context isolation

prepare.py writes a deny-by-default .dockerignore containing exact host source paths. Expanded upstream/, fixtures/, cookies, node_modules, target/, .env and unknown files are not copied. The original source enters the image through a Git-produced archive of the pinned commit, so untracked files added under the expanded upstream directory cannot enter the image.

An offline inspection target needs no base-image download:

```sh
docker build --target context-check -t llmwiki-unchanged-context-check /tmp/llmwiki-container-context
```

That target only packages the allowed build inputs; it does not validate dependency installation or the runtime image. See VALIDATION.md for the checks actually executed.

## Base and dependency references

Defaults match the locally tested Node and Rust versions. Version tags are specified, but image digests and Debian package snapshots are not pinned; this is not a bit-for-bit reproducible build claim.

- [Official Node image](https://hub.docker.com/_/node): 24.22.1-bookworm.
- [Official Rust image](https://hub.docker.com/_/rust): 1.99.1-bookworm.
- [Tauri Linux prerequisites](https://v2.tauri.app/start/prerequisites/#linux) inform the native package list; protobuf-compiler and libprotobuf-dev are additionally required by this project's Rust dependencies.

The full Linux/amd64 image build and offline non-root runtime checks passed in a fresh isolated Docker data directory on a separate disk. Initial disk/network limits were resolved without deleting existing user images or files. See [VALIDATION.md](VALIDATION.md) for the exact scope, protobuf dependency fix and remaining limitations.

## Build networks with a private CA

If an HTTPS proxy uses a local certificate authority, assemble with `--build-ca /path/to/public-ca-bundle.pem` and pass both uppercase and lowercase proxy Docker build arguments. Debian apt uses the lowercase names. For example, after exporting the uppercase variables in the shell:

```sh
export http_proxy="$HTTP_PROXY" https_proxy="$HTTPS_PROXY" no_proxy="$NO_PROXY"
docker build --network host \
  --build-arg HTTP_PROXY --build-arg HTTPS_PROXY --build-arg NO_PROXY \
  --build-arg http_proxy --build-arg https_proxy --build-arg no_proxy \
  -t llmwiki-unchanged-host:experiment /tmp/llmwiki-container-context
```

The argument must contain public PEM certificates; private-key inputs are rejected. The default is an empty CA file. Frontend and backend build stages trust that file, and Node reads it through `NODE_EXTRA_CA_CERTS`. It is not copied into the final runtime stage. TLS verification stays enabled. Proxy values should be supplied through environment variables, not written into source or configuration files.
