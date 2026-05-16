# syntax=docker/dockerfile:1.6
#
# Spectra CLI container image.
#
# Build:    docker build -t spectra:dev .
# Demo:     docker run --rm spectra:dev check \
#               --baseline /examples/vault_baseline \
#               --candidate /examples/vault_candidate \
#               --format markdown
#           # exit 1 — the candidate removes account-validation guards the
#           # baseline enforced.
# Identity: docker run --rm spectra:dev check \
#               --baseline /examples/vault_baseline \
#               --candidate /examples/vault_baseline
#           # exit 0 — zero false positives: identical input is, by
#           # construction, not a regression.
#
# The image embeds the synthetic baseline/candidate Anchor fixtures under
# /examples so the demo can be reproduced with no host files.

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
