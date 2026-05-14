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
| Build provenance | `solana-verifiable-build` (distributes `solana-verify` binary), `anchor verify` | "Does the deployed bytecode match the public source after a reproducible Docker build?" |
| **Behavioural regression** | **Spectra** | **"Does v_{n+1} preserve invariants that v_n holders depend on?"** |
| Formal verification (private, engagement-internal) | Audit-firm internal tooling | "Are stated invariants provably preserved?" (research-grade, slow, not publicly distributed) |
| Runtime monitor | Hypernative, Range | "Did something go wrong after deploy?" |

Spectra is the lightweight CI-fast-path layer the ecosystem currently lacks. The M0 PoC focuses on the IDL diff surface — the rest of the pipeline (ELF parsing, state replay, invariant DSL) is in the milestone roadmap below.

## Documentation

The engineering hardening package lives under [`docs/`](docs/). Start here:

| Document | Purpose |
|----------|---------|
| [docs/VS_GIT_DIFF.md](docs/VS_GIT_DIFF.md) | Head-to-head against `git diff` — what diff cannot do, and where Spectra is fundamentally different (start here if you are asking "isn't this just a fancy diff?") |
| [docs/BENCHMARK.md](docs/BENCHMARK.md) | Reproducible before/after walkthrough on the M0 fixture, with verbatim diff + Spectra outputs and wall-clock timing |
| [docs/THREAT_MODEL.md](docs/THREAT_MODEL.md) | Adversary classes, trust assumptions, soundness / completeness / robustness failure modes |
| [docs/NON_GOALS.md](docs/NON_GOALS.md) | What Spectra is explicitly **not** — compatibility ≠ correctness |
| [docs/SEVERITY.md](docs/SEVERITY.md) | Canonical rule IDs + severities + exit-code contract |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | M0–M3 pipeline + determinism guarantees |
| [docs/SOLANA_EDGE_CASES.md](docs/SOLANA_EDGE_CASES.md) | Per-edge-case coverage matrix (today / M1 / permanently out of scope) |
| [docs/FALSE_POSITIVES.md](docs/FALSE_POSITIVES.md) | Five-layer FP mitigation strategy + suppression schema |
| [docs/CI_INTEGRATION.md](docs/CI_INTEGRATION.md) | Drop-in GitHub Actions / pre-commit / `cargo make` templates |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Milestones gated by acceptance tests |
| [docs/CORPUS.md](docs/CORPUS.md) | Three-layer detection corpus design |
| [docs/REPLAY.md](docs/REPLAY.md) | M2 `litesvm` bounded-replay architecture |
| [docs/RULE_ENGINE.md](docs/RULE_ENGINE.md) | M0 comparator module + M1 `Rule` trait + `RuleRegistry` |
| [docs/MIGRATION.md](docs/MIGRATION.md) | `spectra-allow.toml` migration-declaration schema |
| [docs/ANCHOR.md](docs/ANCHOR.md) | Anchor-specific compatibility hazards (Borsh, discriminators, zero-copy, events) |
| [docs/ADOPTION.md](docs/ADOPTION.md) | Adoption plan + trust signals + pilot strategy |

## M0 scope (this PoC)

Anchor **legacy-schema** IDL JSON diff only. No ELF parsing, no Solana SDK dependency, no network access, no state replay. Detection is correct for Anchor borsh layouts; native `#[repr(C)]` / `bytemuck` alignment is **not** covered until the Shank-IDL path lands in M1.

Detection surface — exhaustive:

| Finding kind | Severity | Notes |
|---|---|---|
| `instruction_removed` | BREAKING | Old clients hit `InstructionFallbackNotFound`. |
| `instruction_args_changed` | BREAKING | Borsh arg length/type mismatch → corrupt deserialise. |
| `instruction_added` | warning | Informational. |
| `account_removed` | BREAKING | Old account discriminator no longer accepted. |
| `account_added` | warning | Informational. |
| `account_field_removed` | BREAKING | Borsh layout shifts. |
| `account_field_added` | warning | Informational; protocol must verify `realloc` + rent. |
| `account_field_reordered` | BREAKING | Borsh layout reorder. |
| `account_field_type_changed` | BREAKING | Width/encoding change. |
| `account_layout_changed_same_discriminator` | BREAKING | Silent-corruption case: discriminator stable but layout changed. |
| `discriminator_collision` | BREAKING | Two IDL names sharing the truncated 8-byte SHA-256. |

## Demo

```bash
cargo build --release

./target/release/spectra check \
  --old examples/lending_v1.json \
  --new examples/lending_v2.json \
  --format markdown
```

Expected: **4 BREAKING + 2 warning**, including:

- `withdraw` instruction removed — old clients invoking the v1 discriminator hit `InstructionFallbackNotFound`.
- `deposit.amount` widened `u64 -> u128` — caller serialisation length mismatches; the call decodes as a corrupt argument.
- `Pool` account fields reordered — existing on-chain accounts deserialise into wrong field positions.
- `Pool` layout changed but discriminator unchanged — the **silent-corruption** case: the runtime accepts the old account data and reads it into the new layout, producing wrong-field reads with no error.
- `Pool.fee_bps` field added — informational, but applicant must verify storage-resize is handled.
- `withdrawFunds` instruction added — informational, replaces removed `withdraw`.

The CLI exits non-zero on any BREAKING finding so it fails CI cleanly. A separate test asserts `spectra check --old X --new X` produces a clean report with zero findings (no false positives on identical inputs).

### Captured demo output

Verbatim markdown report from the command above (exit code `1`):

```markdown
# Spectra Diff Report

**Old program:** `lending`
**New program:** `lending`

**Findings:** 4 breaking, 2 warning

| Severity | Kind | Detail |
|---|---|---|
| BREAKING | instruction_args_changed | `deposit`: [amount: u64] -> [amount: u128] |
| BREAKING | instruction_removed | `withdraw` (disc b712469c946da122) |
| warning  | instruction_added | `withdrawFunds` (disc 52b7b3ffcd4ed2be) |
| warning  | account_field_added | `Pool.fee_bps: u16` |
| BREAKING | account_field_reordered | `Pool`: [total_supply, rate, authority] -> [total_supply, authority, rate, fee_bps] |
| BREAKING | account_layout_changed_same_discriminator | `Pool` layout changed but discriminator f19a6d0411b16dbc is unchanged (silent-corruption risk) |
```

An asciinema cast of the same run is recorded at [`demo.cast`](demo.cast). Replay locally:

```bash
asciinema play demo.cast
```

## What Spectra does NOT do (M0)

Explicit non-claims so the tool is judged honestly:

- No ELF `.so` parsing (planned for M1 follow-on work; not in this grant scope without further milestones).
- No PDA-derivation drift via BPF disassembly — research item, not promised here.
- No mainnet snapshot replay — M2 (grant-scope) uses `litesvm` with a hand-curated per-pilot transaction corpus, not mainnet replay.
- No invariant DSL — protocol-specific invariants are out of scope; Spectra is a schema-regression gate, not a verifier.
- No Token-2022 extension TLV layout detection — IDL does not describe TLV; separate detector pack.
- No constant / `.rodata` diffing.
- No upgrade-authority transfer detection (out-of-band action, not visible to a static diff).
- No native-program `#[repr(C)]` alignment-aware diff until Shank-IDL support lands in M1.

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
| **M0** | Anchor legacy-schema IDL diff (9 finding kinds + silent-corruption + discriminator-collision); 5 tests green; demo cast; public repo | **This PoC** |
| M1 | Anchor 2026 (Codama) schema parser + Shank native IDL parser + defined-type/events/errors diff + Loader-version adapter | Pending grant |
| M2 | `litesvm` pre-deployment harness driven by hand-curated per-pilot transaction corpus (≤50 tx, ≤60s in CI). Not mainnet replay. | Pending grant |
| M3 | `spectra-allow.toml` suppression file + composite GitHub Action + PR comment integration | Pending grant |
| M4 | ≥1 confirmed pilot + 2 public walkthroughs against real upgradable Anchor programs; mdBook docs; Solana Discord AMA | Pending grant |

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
