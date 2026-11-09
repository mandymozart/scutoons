# The hosted Live relay (LIVE.md B.4, #199): the deck/relay binary as a container for bunny Magic Containers, app
# `scutoons-live` at wss://live.scutoons.com (#441: wss and QUIC end at the relay, on 7700 over TCP and UDP). Build with the
# deck/ WORKSPACE as the context (the relay needs deck/protocol and the workspace's Cargo.lock; deck/.dockerignore keeps
# the target folders, deck/lab and the ISOs out):
#
#   docker build -f deck/relay/Dockerfile -t scutoons-live deck
#   docker run --rm -p 7700:7700 -e LIVE_TOKEN_SECRET=… scutoons-live
#   curl http://127.0.0.1:7700/status            → {"sessions":0,"live":0}
#
# A static musl binary on an empty image: nothing else runs in it (no shell, no CA store – its one outbound call, the
# certificate from the API, has the roots compiled in). Env: LIVE_TOKEN_SECRET (unset = dev mode, any non-empty conductor
# token passes), RELAY_LEAD_MS (150 hosted), RELAY_BLOCKLIST, RUST_LOG, RELAY_CERT_URL + RELAY_CERT_KEY (src/cert.rs: the
# pair goes to /certs). Built by .github/workflows/live-image.yml.

FROM rust:1-alpine AS build
RUN apk add --no-cache musl-dev
WORKDIR /src
COPY . .
# only the relay (the workspace's other members are read for the lock file, not built); the release profile is the
# workspace's (LTO, one codegen unit, stripped)
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/src/target \
    cargo build --release --locked -p relay && cp target/release/relay /relay && mkdir /certs

FROM scratch
COPY --from=build /relay /relay
COPY --from=build --chown=65534:65534 /certs /certs
ENV RELAY_ADDR=0.0.0.0:7700 \
    RELAY_LEAD_MS=150
# nobody: the port is above 1024, only /certs is written
USER 65534:65534
EXPOSE 7700
ENTRYPOINT ["/relay"]
