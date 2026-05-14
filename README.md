# Spectra

> CI-time behavioural-regression CLI for Solana program upgrades.
> **Status:** M0 PoC — Anchor IDL diff prototype.

[![CI](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml/badge.svg)](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE)

## What it does

Spectra takes two versions of a Solana program (Anchor IDL for the M0 PoC; compiled `.so` ELF in M1) and reports, in seconds, whether the upgrade preserves discriminators, storage layout, and named runtime invariants.

It sits between two existing layers of the Solana security toolbox:

| Layer | Tool | What it answers |
|---|---|---|
| Build provenance | `solana-verifiable-build`, `anchor verify` | "Does the deployed binary match public source?" |
| **Behavioural regression** | **Spectra** | **"Does v_{n+1} preserve invariants that v_n holders depend on?"** |
| Formal verification | OtterSec `solana-verify` | "Are stated invariants provably preserved?" (research-grade, slow) |
| Runtime monitor | Hypernative, Range | "Did something go wrong after deploy?" |

Spectra is the lightweight CI-fast-path layer the ecosystem currently lacks. The M0 PoC focuses on the IDL diff surface — the rest of the pipeline (ELF parsing, state replay, invariant DSL) is in the milestone roadmap below.

## Demo

```bash
cargo build --release

./target/release/spectra check \
  --old examples/lending_v1.json \
  --new examples/lending_v2.json \
  --format markdown
```

Expected: **3 BREAKING + 2 warning**, including:

- `withdraw` instruction removed — old clients invoking the v1 discriminator silently fail.
- `deposit.amount` widened `u64 -> u128` — caller serialisation length mismatches; the call decodes as a corrupt argument.
- `Pool` account fields reordered — existing on-chain accounts deserialise into wrong field positions (silent data corruption).
- `Pool.fee_bps` field added — informational, but applicant must verify storage-resize is handled.
- `withdrawFunds` instruction added — informational, replaces removed `withdraw`.

The CLI exits non-zero on any BREAKING finding so it fails CI cleanly.

### Captured demo output

This is the verbatim markdown report produced by the command above (exit code `1`):

```markdown
# Spectra Diff Report

**Old program:** `lending`
**New program:** `lending`

**Findings:** 3 breaking, 2 warning

| Severity | Kind | Detail |
|---|---|---|
| BREAKING | instruction_args_changed | `deposit`: [amount: u64] -> [amount: u128] |
| BREAKING | instruction_removed | `withdraw` (disc b712469c946da122) |
| warning  | instruction_added | `withdrawFunds` (disc 52b7b3ffcd4ed2be) |
| warning  | account_field_added | `Pool.fee_bps: u16` |
| BREAKING | account_field_reordered | `Pool`: [total_supply, rate, authority] -> [total_supply, authority, rate, fee_bps] |
```

An asciinema cast of the same run is recorded at [`demo.cast`](demo.cast). Replay locally:

```bash
asciinema play demo.cast
```

## Quick start

```bash
# 1. Install Rust (one-time):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Clone + build:
git clone https://github.com/ayodyadsr/spectra
cd spectra
cargo build --release

# 3. Run the demo:
make demo

# 4. Run the test suite:
cargo test --release
```

## CLI reference

```
spectra check --old <PATH> --new <PATH> [--report <PATH>] [--format json|markdown]
```

- `--old` — baseline IDL (the program version currently deployed)
- `--new` — candidate IDL (the version you are about to upgrade to)
- `--report` — optional path to also write the report to disk
- `--format` — `json` (default, machine-parseable) or `markdown` (PR-comment friendly)

Exit codes:

- `0` — no breaking findings
- `1` — at least one breaking finding
- `2` — invocation error (bad path, parse failure)

## Project layout

```
spectra/
├── spectra-core/          # Rust crate + spectra binary
├── spectra-cli/           # Python wrapper (subprocess-invokes the Rust bin)
├── spectra-action/        # GitHub Action scaffold (full Marketplace publish = M3)
├── examples/              # Synthetic-regression Anchor IDLs for demo + tests
├── scripts/record-demo.sh # asciinema recorder for the demo cast
└── .github/workflows/     # CI: fmt + clippy + test + green-demo verification
```

## Roadmap

The full development plan submitted to the Solana Foundation:

| Milestone | Deliverable | Status |
|---|---|---|
| **M0** | IDL diff prototype on one Anchor program, public repo, green CI | **This PoC** |
| M1 | `spectra-core` Rust crate with ELF parsing, full Anchor + native IDL diff, discriminator drift detection, golden-file test suite | Pending grant |
| M2 | `solana-program-test` integration, mainnet snapshot loader, replay corpus runner | Pending grant |
| M3 | TOML invariant DSL, runner, structured JSON report, composite GitHub Action on Marketplace | Pending grant |
| M4 | 3 protocol pilots integrating Spectra; mdBook docs; Solana Discord office hours AMA | Pending grant |

## Demo recording (asciinema)

The cast is committed at [`demo.cast`](demo.cast) in this repo. To regenerate:

```bash
./scripts/record-demo.sh   # rewrites demo.cast headlessly
asciinema play demo.cast   # replay locally
```

Uploading the cast to asciinema.org is optional and intentionally not done by the script — keep the artifact local so the repo remains the single source of truth.

## Why this PoC exists

This PoC ships **before** the grant submission to address the explicit reviewer risk acknowledged in the proposal: the applicant has no prior Solana-specific OSS contributions. Public M0 code with green CI is the most direct mitigation.

## Background

Spectra is built and maintained by **Ayodya** — 20+ years of penetration testing, formerly Red Team lead at Bank Mandiri. Direct daily work on (a) binary diffing for vulnerability discovery, (b) authoring static analysers and detection signatures, and (c) building CI-time security gates for production engineering teams. The skill set maps one-to-one onto Spectra's three core surfaces: ELF / discriminator diffing (binary analysis), invariant authoring (detection engineering), CI-pipeline integration (DevSecOps).

## License

MIT. See [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issue triage SLA during the grant period: 7 days.

## Security

Please do not file public issues for exploitable security findings. Contact the maintainer privately (a `SECURITY.md` policy will be published after the grant decision).
