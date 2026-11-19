# Publishing a preview image

No public image is claimed until the workflow succeeds and anonymous pulling is verified.

1. Validate the changed frontend, Rust host, and event multiplexing under concurrent subscriptions. Run an isolated packaged smoke check and the real import/Agent workflow before selecting a release commit.
2. Push the reviewed commit, then manually run **Publish preview image** with a unique version such as `v0.1.0-preview.1`. The workflow builds Linux amd64 from the pinned upstream source and publishes `ghcr.io/OWNER/REPOSITORY:VERSION` using the repository GITHUB_TOKEN. No model credentials belong in Actions secrets or the image.
3. In GitHub package settings, ensure the intended package is publicly visible. Verify an anonymous pull and a fresh workspace startup; publishing a package does not by itself prove public visibility or runtime health.
4. Record the image digest, host commit, upstream commit, validated scope and limitations in a prerelease. Provide the matching source and retained GPL license/attribution. Do not tag an unvalidated image as latest.
5. Update the README with the actual version that passed acceptance. For offline delivery, docker save that exact image and provide SHA256SUMS alongside it.

The workflow requires Docker Hub, GitHub, Debian, npm and crates.io network access. It does not configure a model, migrate user data, or update deployed containers. Full desktop feature parity remains pending.
