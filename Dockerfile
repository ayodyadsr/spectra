# syntax=docker/dockerfile:1.6
#
# Spectra CLI container image.
#
# Build:    docker build -t spectra:dev .
# Demo:     docker run --rm spectra:dev check \
#               --old /examples/lending_v1.json \
#               --new /examples/lending_v2.json \
#               --format markdown
# Identity: docker run --rm spectra:dev check \
#               --old /examples/lending_v1.json \
#               --new /examples/lending_v1.json
#               # exit 0 — zero false positives on identical input.
#
# The image embeds the synthetic-regression fixture under /examples so the
# Polkadot-grant Testing Guide can be reproduced with no host files.

FROM rust:1-bookworm AS builder
WORKDIR /build
COPY rust-toolchain.toml ./
COPY Cargo.toml Cargo.lock ./
COPY spectra-core ./spectra-core
COPY examples ./examples
ENV RUSTFLAGS="-D warnings"
RUN cargo build --release --workspace
RUN cargo test --release --workspace

FROM debian:bookworm-slim AS runtime
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*
COPY --from=builder /build/target/release/spectra /usr/local/bin/spectra
COPY --from=builder /build/examples /examples
WORKDIR /work
ENTRYPOINT ["/usr/local/bin/spectra"]
CMD ["--help"]
