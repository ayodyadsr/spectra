# Changelog

All notable changes to Spectra will be documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
from the first tagged release onward.

The pre-1.0 line documents the M0 PoC milestones shipped before the
Solana Foundation grant submission. Future tagged releases (`v0.2.0-m1`,
`v0.3.0-m2`, etc.) will each get their own section here.

## [Unreleased]

### Added
- Inline Rustdoc on all public crate items in `spectra-core` (lib, idl, diff, discriminator, report). Enforced by `#![warn(missing_docs)]` + `RUSTFLAGS="-D warnings"` in CI.
- `Dockerfile` + `.dockerignore` enabling hermetic containerised reproduction of the M0 acceptance gates; new `docker` CI job builds the image and runs the synthetic-regression + identical-IDL smoke tests on every push.
- `SECURITY.md` vulnerability-reporting policy with supported-versions table and response SLAs (48 hr ack, 7 d triage, 30 d Critical/High fix).
- `docs/TESTING.md` consolidated step-by-step testing guide covering host build, end-to-end CLI gates, Docker reproduction, real-world IDL reproduction, competitive benchmark, per-test mapping, and how-to-add-a-test.
- `CODE_OF_CONDUCT.md` adopting Contributor Covenant 2.1.
- `CHANGELOG.md` (this file) and `.github/ISSUE_TEMPLATE/` + `pull_request_template.md`.

### Changed
- README cross-references updated to point at `SECURITY.md`, `Dockerfile`, `docs/TESTING.md`, and `CODE_OF_CONDUCT.md` now that they exist.

## [0.1.0-m0] — 2026-05-15

Pre-grant M0 PoC. Public at https://github.com/ayodyadsr/spectra.

### Added
- Rust workspace: `spectra-core` library + `spectra` CLI binary; Python wrapper `spectra-cli`; GitHub Action scaffold `spectra-action`.
- Anchor legacy-schema IDL diff engine covering 11 rule types: instruction added/removed/args-changed; account added/removed/field-added/field-removed/field-reordered/field-type-changed; **account-layout-changed-same-discriminator (silent corruption)**; **discriminator-collision**.
- Output formats: JSON, Markdown, SARIF 2.1.0 (uploadable to GitHub Code Scanning via `github/codeql-action/upload-sarif@v3`).
- Severity-tiered exit-code contract: `0` clean / `1` BREAKING / `2` invocation error / `3` refuse-to-analyse.
- `--quiet` flag suppressing stdout on clean runs (CI noise control).
- 8 green tests (2 unit + 6 integration) including the Anchor known-vector assertion `sha256("global:initialize")[..8] = afaf6d1f0d989bed`.
- Synthetic-regression fixture (`examples/lending_v1.json` → `examples/lending_v2.json`) demonstrating all 11 rule kinds.
- Real-world validation on Drift Protocol v2.155 → v2.162 (428 KB production IDL, 6 ms wall-clock, 6 findings including one real silent-corruption case on `PerpMarket`).
- Head-to-head competitive benchmark against `diff -u` 3.10, `jd` 1.9.2, `dyff`, `json-diff` (npm) on the same Drift IDL pair.
- 17+ engineering docs under `docs/` (architecture, severity, threat model, non-goals, edge-case matrix, FP policy, CI integration, rule engine, migration schema, Anchor specifics, adoption plan, roadmap, real-world + competitive benchmarks, Q1-style technical paper).
- Asciinema demo cast committed at `demo.cast`.
- CI workflow: `cargo fmt --check` + `cargo clippy -D warnings` + `cargo build --release` + `cargo test --release` + 5 end-to-end CLI gates verifying the exit-code contract.

### Changed
- Relicensed from MIT to **Apache License 2.0** for the explicit patent grant in Apache §3, aligning with `solana-verifiable-build`, the Solana SDK, and Anza-published developer tooling.

[Unreleased]: https://github.com/ayodyadsr/spectra/compare/v0.1.0-m0...HEAD
[0.1.0-m0]: https://github.com/ayodyadsr/spectra/releases/tag/v0.1.0-m0
