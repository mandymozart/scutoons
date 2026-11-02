# The rig module compile service (#281, agents/research/RIG-EDITING.md §4; LIVE.md B.4): the second container of the bunny
# Magic Containers app `scutoons-live`, beside the Live relay (one pod: shared localhost, 8 CPU / 32 GiB, no per-container
# limits - so one compile at a time, 2 cargo jobs, none while the relay reports a live show). Build with the deck/ WORKSPACE
# as the context, like the relay's (deck/.dockerignore keeps target folders, deck/lab and the ISOs out):
#
#   docker build -f deck/build/Dockerfile -t scutoons-build deck
#   docker run --rm -p 7710:7710 -e MODULE_BUILD_TOKEN=… -e RELAY_STATUS_URL=off scutoons-build
#   curl http://127.0.0.1:7710/health            → {"ok":true,"waiting":0,"busy":false}
#
# What is inside: rustc + cargo pinned below, the wasm32-unknown-unknown target, the SDK (deck/module + deck/module-macros
# sources with their standalone manifests from deck/build/sdk), the vendored closure of the SDK's dependencies (so every
# compile runs `--offline`: no network needed at runtime), and a build folder primed with the SDK built once (so a compile is
# seconds). The service writes every member's Cargo.toml itself (src/compile.rs); a member's files are only src/*.rs.
# No secrets in the image or in what the compiled code can reach: MODULE_BUILD_TOKEN is the service's own env (bunny) and is
# removed from the compiler's environment. User nobody. Env: MODULE_BUILD_TOKEN (required), BUILD_LISTEN, RELAY_STATUS_URL
# (default http://127.0.0.1:7700/status, `off` = no live-show check), BUILD_JOBS, BUILD_WALL_SECS, BUILD_MEMORY_MB,
# BUILD_QUEUE, BUILD_QUEUE_WAIT_SECS, BUILD_MAX_WASM_KB (src/config.rs). Built by .github/workflows/build-image.yml.
#
# The toolchain pin is the one the deck builds with (rustc 1.99.1): a new version is a deliberate change here - the same
# source and toolchain give the same .wasm, which the CDN names by its hash.
ARG RUST_VERSION=1.99.1

FROM rust:${RUST_VERSION}-slim-bookworm AS service
WORKDIR /src
COPY . .
# only the service (the workspace's other members are read for the lock file, not built)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p scutoons-build && cp target/release/scutoons-build /scutoons-build

FROM rust:${RUST_VERSION}-slim-bookworm
RUN rustup target add wasm32-unknown-unknown
COPY --from=service /scutoons-build /usr/local/bin/scutoons-build

# the SDK: the workspace's sources, the standalone manifests (a copy cannot inherit from the workspace)
COPY module/src /opt/sdk/module/src
COPY build/sdk/module.Cargo.toml /opt/sdk/module/Cargo.toml
COPY module-macros/src /opt/sdk/module-macros/src
COPY build/sdk/macros.Cargo.toml /opt/sdk/module-macros/Cargo.toml
# the workspace's lock pins the versions the SDK's dependencies are vendored at
COPY Cargo.lock /opt/lock-seed/Cargo.lock

ENV BUILD_SDK_DIR=/opt/sdk \
    BUILD_WORK_DIR=/var/build/work \
    BUILD_TARGET_DIR=/var/build/target \
    BUILD_CARGO_HOME=/opt/cargo-home \
    BUILD_LOCK=/opt/lock/Cargo.lock \
    BUILD_LISTEN=0.0.0.0:7710

# 1. the stub member crate (written by the service itself: the manifest every member gets) · 2. `cargo vendor` its dependency
# closure (the only step that needs the network) and point cargo at it · 3. the lock that results is the one every compile
# starts from · 4. one compile of the stub builds the SDK and its dependencies into the build folder (this fails the image
# build if the SDK, the vendoring or the offline setup is broken) · 5. nobody owns what a compile writes to
RUN set -eux; \
    mkdir -p /opt/cargo-home /opt/lock /var/build/work /var/build/target /opt/stub; \
    scutoons-build stub /opt/stub; \
    cp /opt/lock-seed/Cargo.lock /opt/stub/Cargo.lock; \
    cd /opt/stub; \
    cargo vendor --versioned-dirs /opt/vendor > /opt/cargo-home/config.toml; \
    cp Cargo.lock /opt/lock/Cargo.lock; \
    cd /; \
    scutoons-build prime; \
    rm -rf /opt/stub /opt/lock-seed; \
    chown -R 65534:65534 /var/build /opt/cargo-home

USER 65534:65534
EXPOSE 7710
ENTRYPOINT ["scutoons-build"]
