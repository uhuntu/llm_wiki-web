# swc-build Web Wiki deployment

Deployed 2026-11-11 to hunt@10.11.108.241 using the local Git bundle and prebuilt lifecycle-20261111 image. No server-side build or dependency download was performed.

- Source: `/home/hunt/llm_wiki-web`, main at cdb48f9.
- Workspace: `/home/hunt/llmwiki-web-data`, mode 0700, mounted at /data.
- Container: `llmwiki-web-server`, UID/GID 1000, read-only root, capabilities dropped, no-new-privileges, restart unless-stopped, bounded Docker log rotation.
- Listener: `127.0.0.1:18932`; no LAN listener is enabled.
- Launcher: `~/.runtime` is not used. Actual launcher is `~/llm_wiki-web/.runtime/deployment/run-server.sh`.
- Image and source bundle: `~/llm_wiki-web/.runtime/deployment/`.
- Check report and probe script are in the same private deployment directory.

On the workstation, establish the same-port tunnel:

```sh
ssh -N -L 127.0.0.1:18932:127.0.0.1:18932 hunt@10.11.108.241
```

Open `http://127.0.0.1:18932/`. Login initially requires the one-use URL printed by `docker logs llmwiki-web-server`; use 127.0.0.1 instead of localhost in the browser URL to keep its cookie separate from the workstation preview. Same-port forwarding preserves the host/origin checks. A tunnel is currently running on the workstation. It is not an automatic boot service.

The initial login URL has been consumed by browser verification. For another user session, an already authenticated browser can request a new `/login-ticket`; alternatively, after ensuring no work is running, restart the container and use its new startup ticket. Re-running verify_server.py also restarts the container and generates a new login-url.txt; this is an administrative check, not a routine health endpoint.

Validation passed: anonymous request returns 401, original Rust write/read commands, file persistence after container restart, and browser access through the SSH tunnel. The transferred image tar SHA-256 is bf49f94d56971b05f1847ee2a136e7cbd986c82be806732dff0792b24ae91fae. The server Docker image ID differs from the workstation ID after load; executable, frontend index and runner SHA-256 all match, and 475 upstream blobs verify unchanged.

No model has been configured and no existing DG3 documents were imported. Existing _Wiki and SQLite data are not mounted in this container. Next: select an approved model and define an import copy/adapter before accessing existing DG3 content. A workspace backup schedule and unattended long-running operation remain future work. Existing 8090/8091 knowledge-base services and 8081/8082 Dependency-Track containers were not changed.
