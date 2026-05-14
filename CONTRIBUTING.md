# Contributing to Spectra

Spectra is the M0 PoC for a Solana Foundation grant submission. Contributions are welcome but the scope is intentionally tight until the grant decision lands. Please open an issue before sending a PR.

## Dev environment

Requirements:
- Rust stable (managed via `rust-toolchain.toml`; install via [rustup](https://rustup.rs/))
- Python 3.9+ (only if working on `spectra-cli`)

Build, lint, test:

```bash
cargo build --release
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo test --release
```

Run the synthetic-regression demo:

```bash
make demo
```

## Project layout

| Path | Purpose |
|---|---|
| `spectra-core/` | Rust crate + `spectra` binary |
| `spectra-cli/` | Python wrapper around the binary |
| `spectra-action/` | GitHub Action scaffold (full publication is M3 deliverable) |
| `examples/` | Synthetic-regression Anchor IDLs used by the demo and tests |
| `scripts/` | Helper scripts (asciinema recording, etc.) |

## Issue triage SLA

During the grant period: all issues triaged within 7 days. Tag bugs `bug`, feature requests `enhancement`, security findings `security` (please do not file public issues for exploitable findings; email instead — see SECURITY.md once published).

## Out of scope (for now)

The roadmap in the grant proposal lists what's intentionally deferred to M1–M4. Please don't send PRs that try to land formal verification, multi-program CPI replay, or runtime monitoring — those have their own milestones.
