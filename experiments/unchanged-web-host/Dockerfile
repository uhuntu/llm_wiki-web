ARG NODE_IMAGE=node:24.22.1-bookworm
ARG RUST_IMAGE=rust:1.99.1-bookworm
ARG RUNTIME_IMAGE=debian:bookworm-slim

# An offline target for inspecting exactly what enters the build context.
FROM scratch AS context-check
COPY . /context

FROM ${NODE_IMAGE} AS frontend
WORKDIR /build
COPY build-ca.crt /usr/local/share/ca-certificates/host-build.crt
RUN if [ -s /usr/local/share/ca-certificates/host-build.crt ]; then update-ca-certificates; fi
ENV NODE_EXTRA_CA_CERTS=/usr/local/share/ca-certificates/host-build.crt
RUN apt-get update \
 && apt-get install -y --no-install-recommends python3 \
 && rm -rf /var/lib/apt/lists/*
COPY upstream-source.tar upstream-manifest.json verify_source.py ./
RUN mkdir upstream && tar -xf upstream-source.tar -C upstream \
 && python3 verify_source.py
COPY package.json package-lock.json vite.config.mjs tsconfig.json ./
COPY platform ./platform
RUN npm ci --no-audit --no-fund \
 && npm run typecheck \
 && npm run build \
 && python3 verify_source.py

FROM ${RUST_IMAGE} AS backend
ARG CARGO_BUILD_JOBS=2
ENV CARGO_BUILD_JOBS=${CARGO_BUILD_JOBS}
COPY build-ca.crt /usr/local/share/ca-certificates/host-build.crt
RUN if [ -s /usr/local/share/ca-certificates/host-build.crt ]; then update-ca-certificates; fi
RUN test "$(dpkg --print-architecture)" = "amd64"
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
    build-essential pkg-config python3 protobuf-compiler libssl-dev libxdo-dev \
    libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
 && rm -rf /var/lib/apt/lists/*
# protoc's standard .proto includes are not installed with --no-install-recommends.
RUN apt-get update \
 && apt-get install -y --no-install-recommends libprotobuf-dev \
 && test -f /usr/include/google/protobuf/empty.proto \
 && rm -rf /var/lib/apt/lists/*
WORKDIR /build
COPY --from=frontend /build/upstream ./upstream
COPY --from=frontend /build/dist ./dist
COPY --from=frontend /build/upstream-manifest.json /build/verify_source.py ./
COPY backend ./backend
RUN cargo build --locked --release --manifest-path backend/Cargo.toml --bin external-host-smoke \
 && python3 verify_source.py

FROM ${RUNTIME_IMAGE} AS runtime
RUN apt-get update \
 && apt-get install -y --no-install-recommends \
    python3 ca-certificates libssl3 libxdo3 libgtk-3-0 \
    libwebkit2gtk-4.1-0 libayatana-appindicator3-1 librsvg2-2 \
 && rm -rf /var/lib/apt/lists/* \
 && groupadd --system --gid 10001 llmwiki \
 && useradd --system --uid 10001 --gid 10001 --create-home llmwiki \
 && mkdir /data && chown 10001:10001 /data && chmod 0700 /data
WORKDIR /opt/llmwiki
COPY --from=frontend /build/upstream ./upstream
COPY --from=frontend /build/dist ./dist
COPY --from=frontend /build/upstream-manifest.json ./
COPY --from=backend /build/backend/target/release/external-host-smoke ./bin/external-host
COPY run_probe.py verify_source.py ./
RUN python3 verify_source.py
LABEL org.opencontainers.image.title="LLM Wiki unchanged-source experiment" \
      org.opencontainers.image.description="Single-user Web host with live HTTP models; full desktop parity pending"
USER 10001:10001
VOLUME ["/data"]
ENTRYPOINT ["python3", "/opt/llmwiki/run_probe.py", "--binary", "/opt/llmwiki/bin/external-host", "--workspace", "/data"]
CMD ["--mode", "live", "--port", "18902"]
