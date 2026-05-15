# Spectra

[![CI](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml/badge.svg)](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Project Overview

**Tagline:** A CI-time behavioural-regression diff for Solana program upgrades — catches the silent-corruption and discriminator-collision cases that general-purpose JSON diff tools cannot see.

**Project Description:**
Spectra is an open-source CLI and GitHub Action that diffs two versions of a Solana program at the Anchor IDL level and reports the upgrade hazards that would silently corrupt existing on-chain state or misroute existing client calls. It runs in roughly **6 milliseconds** on a 428 KB production IDL, emits machine-readable JSON, human-readable Markdown, or SARIF 2.1.0 (uploaded straight to GitHub's Security tab), and gates a CI build with a severity-tiered exit code. Every claim in this README is reproducible from the commit it points to.

**Solana Integration:**
Solana programs are almost always deployed via an *upgradeable* BPF loader. The loader does not check that the new program is compatible with the old program's on-chain accounts. Two classes of upgrade hazard therefore exist on Solana that do not exist on most other L1s:

1. **Account-layout silent corruption.** The 8-byte Anchor discriminator stays the same, but the field layout has changed. The runtime accepts old account data and deserialises it into the new shape, with no error. A reordered field is now read as a different field. Money on paper becomes wrong money on chain.
2. **Discriminator collision.** Anchor and Shank both derive 8-byte instruction tags from `sha256("global:<name>")[..8]`. Two different names collide in 8-byte truncated SHA-256 space at far higher probability than the 64-bit birthday bound suggests once you allow human-chosen names. A colliding pair silently misroutes calls between instructions.

Spectra targets exactly these two Solana-specific hazards, plus 9 other rules covering the conventional shape-of-the-interface changes that break old clients.

**Founder Interest:**
The applicant has 20+ years in offensive security (most recently Red Team lead at Bank Mandiri) and built numerous CI-time security gates for production engineering teams. The dominant pattern in audit findings on Solana programs is *not* the kind of arithmetic bug fuzzers find — it is shape mismatches between the deployed program and the data already on chain. Spectra is the smallest tool that closes that gap, and the gap was named directly in the open [Solana forum RFP "Program Verification Tooling"](https://forum.solana.com/t/program-verification-tooling/1032).

### Project Details

**Technology Stack:**
- **Rust 2021 edition** — core engine ([`spectra-core`](spectra-core)), `serde_json` for IDL parsing, `sha2` for discriminator computation, `clap 4` for the CLI.
- **Python 3.9+ wrapper** — [`spectra-cli`](spectra-cli), subprocess-invokes the Rust binary so the same engine is reachable from Python-first CI environments.
- **GitHub Action scaffold** — [`spectra-action`](spectra-action), composite action stub; full Marketplace publish lands in M3.
- **SARIF 2.1.0** — output format for GitHub Code Scanning, consumed by `github/codeql-action/upload-sarif@v3`.
- **litesvm** (M2 only) — bounded, in-process Solana VM for per-pilot replay of ≤50 hand-curated transactions; not a mainnet snapshot.

**Core Architecture:**
The engine is intentionally narrow and deterministic. Five modules:

- `idl` — parses Anchor legacy-schema IDL JSON; M1 adds Anchor 2026 / Codama and Shank native schemas behind the same `Idl::from_path` entry point with schema auto-detection.
- `discriminator` — computes `sha256("global:<name>")[..8]` for instructions and `sha256("account:<name>")[..8]` for accounts. The Anchor known-vector `sha256("global:initialize")[..8] = afaf6d1f0d989bed` is asserted in a unit test.
- `diff` — pairwise comparison of two parsed IDLs, emitting `Finding`s typed by 11 rule kinds (see [§All M0 rules](#all-11-things-spectra-checks-today-m0)).
- `report` — renders findings as JSON, Markdown, or SARIF 2.1.0. SARIF maps `BREAKING → level: error`, `warning → level: warning`, with per-finding `logicalLocation`.
- `main` — CLI entry point. Severity-tiered process-level exit code per [`docs/SEVERITY.md`](docs/SEVERITY.md) §5.

**CLI Specification:**
```text
spectra check --old <PATH> --new <PATH> [--report <PATH>] [--format json|markdown|sarif] [--quiet]
```

```bash
# Detect a regression and gate the merge:
spectra check --old examples/lending_v1.json --new examples/lending_v2.json --format markdown
# → exit 1, prints 4 BREAKING + 2 warning findings

# Identical-IDL invariant: zero false positives on real production IDL:
spectra check --old drift_v2.155.json --new drift_v2.155.json --format json --quiet
# → exit 0, zero stdout

# GitHub Code Scanning integration:
spectra check --old old.json --new new.json --format sarif --report out.sarif
# Then: github/codeql-action/upload-sarif@v3 with sarif_file: out.sarif
```

Exit-code contract (verified in CI):

| Code | Meaning |
|---|---|
| `0` | Clean — no breaking findings. |
| `1` | At least one BREAKING finding — block the merge. |
| `2` | Invocation error — bad path, bad JSON, unknown `--format` value. |
| `3` | Refuse-to-analyse — input is in a shape Spectra cannot soundly diff. |

**What Spectra is NOT:**
- **Not a formal verifier.** It does not prove that invariants are preserved; that is audit-firm territory (~$15k–$100k, 2–6 weeks).
- **Not a build-provenance tool.** It does not check that the deployed `.so` matches public source; that is `solana-verify` / `anchor verify`.
- **Not a runtime monitor.** It is pre-merge only; for post-deploy alerting see Hypernative / Range.
- **Not a mainnet-replay harness.** M2 uses `litesvm` with a hand-curated ≤50-tx per-pilot corpus, bounded to <60 s in CI. Heavy-corpus replay against historical mainnet upgrades is research scope, explicitly out.
- **Not a Token-2022 TLV-extension detector.** TLV is not described in Anchor IDL; out of scope.
- **Not a `.rodata` / constant differ.** Out of scope.

### Ecosystem Fit

**Ecosystem Position:**
Spectra sits between two existing layers in the Solana security stack:

| Question | Existing tool | Spectra? |
|---|---|---|
| Does the deployed bytecode match public source? | `solana-verify`, `anchor verify` | No — different layer (build provenance) |
| Are invariants provably preserved across the upgrade? | Audit-firm formal verification | No — different layer (proofs) |
| **Will the upgrade preserve old users' data and old clients' calls?** | **(no public tool before Spectra)** | **Yes — this is the gap** |
| Did something go wrong after the upgrade went live? | Hypernative, Range | No — too late by then |

The middle row is the gap the open [Solana RFP](https://forum.solana.com/t/program-verification-tooling/1032) names directly. Spectra is the smallest credible answer.

**Target Audience:**
- **Primary:** Solana program teams shipping upgradeable Anchor programs into production (DeFi protocols, NFT marketplaces, oracles).
- **Secondary:** Audit firms running pre-engagement diff scans on client programs before deeper review.
- **Tertiary:** Solana Foundation grant reviewers and security partners who need a standard pre-engagement diff utility.

**Needs Addressed:**
- **Catch silent-corruption before deploy.** The single highest-impact case in the Solana upgrade-hazard taxonomy is detected with one CLI command in ~6 ms.
- **Block breaking changes in CI.** Severity-gated exit code lets a `BREAKING` finding fail a build while a `warning` does not.
- **Surface findings in the GitHub Security tab.** SARIF 2.1.0 output uploads to GitHub Code Scanning via the standard action — same surface as CodeQL.
- **Zero false positives on identical input.** Asserted by both a test and a dedicated CI step against a 428 KB real production IDL.

**Need Identification:**
The need was identified directly in the Solana Foundation's open RFP thread `forum.solana.com/t/program-verification-tooling/1032`, which calls out upgrade-safety regression as a missing layer. The bug classes Spectra detects (silent-corruption, discriminator-collision, Borsh arg widening, account field reorder) match the failure modes named in the RFP and seen in the Drift v2.155 → v2.162 upgrade pair Spectra was validated against.

**Similar Projects in the Solana Ecosystem:**
After research across the Solana forum, the Anchor and Foundation GitHub orgs, and prior grant rounds, no public tool was found that performs **upgrade-safety regression** specifically. The closest layers are:

- **`solana-verifiable-build` / `anchor verify`** — verifies *that the deployed bytecode came from a given source tree*, not whether the new version is compatible with old on-chain state. Different layer.
- **Audit-firm formal verification (OtterSec, Halborn, etc.)** — proves invariants on a specific revision, engagement-internal, ~$15k–$100k. Different cost class and scope.
- **`diff -u` / `jd` / `dyff` / `json-diff`** — generic text/JSON diff tools. Benchmarked head-to-head in [`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md); none knows what an Anchor discriminator is, so none can detect silent-corruption.
- **Hypernative / Range** — runtime monitors that fire *after* a bad upgrade ships. Different layer (post-deploy).

Spectra is therefore complementary, not substitutive, to every tool in this list.

---

## Team

### Team members

- **Ayodya** (lead engineer, sole maintainer for M0–M4 scope)

### Team Contact

- Email: ayodyadsr@gmail.com
- GitHub: [@ayodyadsr](https://github.com/ayodyadsr)

### Team Code Repos

- Spectra PoC (this repo): https://github.com/ayodyadsr/spectra

### Team's Experience

20+ years of offensive-security and detection-engineering work, most recently Red Team lead at Bank Mandiri (Indonesia's largest bank by assets). Three skill surfaces map one-to-one onto Spectra's three core surfaces:

1. **Binary diffing for vulnerability discovery** ↔ Spectra's discriminator and layout diff engine.
2. **Authoring static analysers and detection signatures** ↔ Spectra's M0 11-rule catalogue and the M1 `Rule` trait roadmap in [`docs/RULE_ENGINE.md`](docs/RULE_ENGINE.md).
3. **Building CI-time security gates for production engineering teams** ↔ Spectra's severity-tiered exit code, SARIF output, and GitHub Action surface.

The grant proposal honestly states the applicant has no prior Solana-specific OSS contributions. The M0 PoC shipped before grant submission — with green CI from the first commit and a validated real-world benchmark on Drift Protocol — is the direct mitigation for that gap.

---

## Development Status

**Current Status:** M0 PoC is shipped, public, Apache-2.0-licensed, and validated against a real production upgrade. The repository is referenced in the grant proposal at [`02_proposals/drafts/solana-program-verification-tooling/final_proposal.md`](../02_proposals/drafts/solana-program-verification-tooling/final_proposal.md). Submission to https://solana.org/grants-funding is pending final pilot LOI outreach.

**Proof of Concept:**
Spectra was run against a real Solana mainnet program upgrade pair: [Drift Protocol v2](https://github.com/drift-labs/protocol-v2) commits `590049e6bf` (v2.155, 2026-01-21) → `0d35029d78` (v2.162, 2026-04-01). The IDLs are the public Anchor IDL JSONs committed to the protocol's SDK at those commits.

- IDL size: 428 KB, 20,138 lines, 249 instructions, 27 accounts, 115 types, 26 events, 349 errors.
- Changes between versions: 319 lines, scattered through the file.
- **Spectra completed in 6 ms** and surfaced 6 findings (2 BREAKING + 4 warning).
- **The interesting one:** Drift's `PerpMarket` account shrank `padding` from 23 bytes to 22 bytes and added a new `marketConfig: u8` in the freed byte. The `PerpMarket` discriminator (`0adf0c2c6bf537f7`) did not change. This is the silent-corruption pattern — safe **if and only if** every on-chain `PerpMarket`'s old padding byte was zero, dangerous otherwise. Exactly the case a reviewer needs to be told about explicitly.
- **Zero false positives on a 428 KB production IDL.** Running Spectra with the same file as both `--old` and `--new` exits 0 with no findings.

A human reviewer with a 393-line `diff -u` would need to do a 7-step Anchor + Borsh inference chain to reach the same conclusion. Spectra labels it `account_layout_changed_same_discriminator` directly. Reproduction commands and the line-by-line walkthrough: [`docs/BENCHMARK_DRIFT.md`](docs/BENCHMARK_DRIFT.md).

**Development Progress (M0 — shipped):**
- ✅ Rust core engine ([`spectra-core`](spectra-core)) + `spectra` CLI binary.
- ✅ Python wrapper ([`spectra-cli`](spectra-cli)) with `spectra-py` entry point.
- ✅ GitHub Action scaffold ([`spectra-action/action.yml`](spectra-action/action.yml)).
- ✅ 11 rule types covering Anchor legacy-schema IDLs (full table below).
- ✅ JSON, Markdown, and SARIF 2.1.0 output formats.
- ✅ Severity-tiered exit code (0 / 1 / 2 / 3) verified by 3 dedicated CI steps.
- ✅ 8 tests green (2 unit + 6 integration), including the Anchor `sha256("global:initialize")[..8] = afaf6d1f0d989bed` known-vector assertion.
- ✅ Synthetic regression fixture ([`examples/lending_v1.json`](examples/lending_v1.json) → [`examples/lending_v2.json`](examples/lending_v2.json)) detects 4 BREAKING + 2 warning findings.
- ✅ Real-world validation on Drift Protocol v2.155 → v2.162 (6 ms, 6 findings, one real silent-corruption case).
- ✅ Head-to-head benchmark against `diff -u`, `jd`, `dyff`, `json-diff` ([`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md)).
- ✅ Asciinema demo recording committed at [`demo.cast`](demo.cast).
- ✅ Apache 2.0 license (relicensed from MIT on 2026-05-15 for explicit patent grant + Solana SDK alignment).

**Testing and Validation:**
- **Static checks gated in CI:** `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release`.
- **Unit + integration tests:** `cargo test --release` runs 8 tests covering the Anchor discriminator known-vector, synthetic-regression detection, identical-IDL clean-report invariant, no-false-positive-collision, SARIF schema validity, SARIF clean-empty output, and the Markdown silent-corruption row.
- **End-to-end demo gates:** CI runs `spectra check` on the bundled v1→v2 regression fixture and asserts exit `1`; runs the same file against itself and asserts exit `0`; runs SARIF output and validates it parses as JSON; runs an unknown `--format` and asserts exit `2`; runs `--quiet` on a clean input and asserts zero stdout.
- **CI runs:** every push has been green. Recent runs: [`25913583638`](https://github.com/ayodyadsr/spectra/actions/runs/25913583638) (Apache 2.0 relicense), [`25912798879`](https://github.com/ayodyadsr/spectra/actions/runs/25912798879) (competitive benchmark + SARIF), [`25910510275`](https://github.com/ayodyadsr/spectra/actions/runs/25910510275) (real-world Drift validation).

**Known Limitations:**
- **Anchor legacy-schema only at M0.** Anchor 2026 (Codama schema), native programs (Shank-generated IDL), and the defined-type / events / errors reference graph are M1 deliverables.
- **No `.so` bytecode parsing at M0.** ELF + BPF disassembly is M1.
- **No PDA-drift detection.** Marked as Future Expansion in the proposal, not promised in this grant.
- **Native-program `#[repr(C)]` / `bytemuck` alignment padding** is not surfaced in Anchor IDL; will be addressed in M1 with Shank-IDL parsing.
- **`spectra-allow.toml` suppression file** for intentional schema extensions does not yet exist; lands in M3.

---

## Quick Start

```bash
# 1. Install Rust (one-time, skip if you already have it):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Clone and build:
git clone https://github.com/ayodyadsr/spectra
cd spectra
cargo build --release

# 3. Run the demo (the synthetic regression fixture):
./target/release/spectra check \
  --old examples/lending_v1.json \
  --new examples/lending_v2.json \
  --format markdown
# → exit 1, prints the 4 BREAKING + 2 warning findings shown below

# 4. Run the test suite (8 tests, all green):
cargo test --release

# 5. Replay the asciinema demo:
asciinema play demo.cast
```

### What the report looks like

Verbatim output of the demo command above (exit code `1`):

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

---

## All 11 things Spectra checks today (M0)

| Finding | Severity | What it means in plain words |
|---|---|---|
| `instruction_removed` | BREAKING | An old function was deleted. Old apps calling it fail with `InstructionFallbackNotFound`. |
| `instruction_args_changed` | BREAKING | A function's inputs changed shape. Old callers send the wrong bytes. |
| `instruction_added` | warning | A new function was added. Informational. |
| `account_removed` | BREAKING | An account type was deleted. Old account data can no longer be read. |
| `account_added` | warning | A new account type was added. Informational. |
| `account_field_removed` | BREAKING | A field was deleted from an account. Old accounts now misalign. |
| `account_field_added` | warning | A field was added. Check that storage resize is handled. |
| `account_field_reordered` | BREAKING | Field order changed. Old accounts now read wrong fields. |
| `account_field_type_changed` | BREAKING | A field changed type (e.g. `u64 → u128`). Old data is the wrong width. |
| `account_layout_changed_same_discriminator` | BREAKING | **Silent corruption.** Layout changed but the discriminator did not. The runtime accepts old data and reads it into the new layout. |
| `discriminator_collision` | BREAKING | Two different names produce the same 8-byte SHA-256 tag. Calls get misrouted. |

The full rule roadmap reaches **23 rule types** across M0–M2; see [`docs/SEVERITY.md`](docs/SEVERITY.md) for the canonical list with rule IDs, severity tiers, and the exit-code contract.

---

## Development Roadmap

This roadmap is the version submitted to the Solana Foundation. Every milestone is gated by **acceptance tests** that must pass in CI on a tagged commit — not by activity. The current state of each acceptance gate is tracked in [`docs/ROADMAP.md`](docs/ROADMAP.md).

### Overview

- **Estimated Duration:** 16 weeks (post-grant) + 2 weeks of pre-grant M0 work already shipped.
- **Full-Time Equivalent (FTE):** ~0.875 FTE (35 hr/week × 16 weeks).
- **Total Costs:** **$28,115 USD**.

### Milestone 0 — PoC (pre-grant, shipped)

| Number | Deliverable | Specification | Status |
| ---: | --- | --- | --- |
| 0a. | License | Apache License 2.0 applied to the project. | ✅ ([`LICENSE`](LICENSE)) |
| 0b. | Documentation | Comprehensive `docs/` covering threat model, severity contract, architecture, edge cases, FP policy, CI integration, rule engine, and adoption plan. README explains setup, CLI, and full report format. | ✅ ([`docs/`](docs/)) |
| 0c. | Testing and Testing Guide | 8 green tests (2 unit + 6 integration) including Anchor known-vector discriminator, synthetic-regression detection, identical-IDL clean-report invariant, no-false-positive collision, SARIF schema validity. Reproduction: `cargo test --release`. | ✅ |
| 0d. | Demo | Asciinema cast committed at [`demo.cast`](demo.cast); regenerate with [`scripts/record-demo.sh`](scripts/record-demo.sh). | ✅ |
| 0e. | Article | Q1-style technical paper at [`docs/PAPER.md`](docs/PAPER.md) (~600 lines, 9 sections including real-world Drift validation). | ✅ |
| 1. | Anchor legacy-schema IDL diff | 11 rule types, severity-tiered, deterministic output. | ✅ |
| 2. | Output formats | JSON + Markdown + SARIF 2.1.0; SARIF uploads via `github/codeql-action/upload-sarif@v3`. | ✅ |
| 3. | Real-world validation | Drift Protocol v2.155 → v2.162, 428 KB IDL, 6 ms wall-clock, 6 findings including one real silent-corruption case. | ✅ ([`docs/BENCHMARK_DRIFT.md`](docs/BENCHMARK_DRIFT.md)) |
| 4. | Competitive benchmark | Head-to-head against `diff -u`, `jd`, `dyff`, `json-diff` on the same Drift IDL pair. | ✅ ([`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md)) |

### Milestone 1 — Schema parity + collision/silent-corruption hardening — 4 weeks — $6,300

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 1.1 | Anchor 2026 (Codama) parser | Auto-detection at `Idl::from_path`; full coverage of the Codama node graph. |
| 1.2 | Shank native IDL parser | Anchor-free path for native programs; surfaces `#[repr(C)]` / `bytemuck` alignment padding the legacy IDL omits. |
| 1.3 | Defined-type resolution | Nested `types`, `events`, and `errors` diffed via reference graph (M0 ignores them). |
| 1.4 | Cross-name discriminator-collision check | Pairwise check over all instruction + account names in both old and new IDLs. |
| 1.5 | Silent-corruption layout check (hardened) | Width changes, padding changes, alignment changes, all surfaced; covered by golden-file test suite of 8 documented breakage classes. |
| 1.6 | Loader-version adapter | Thin adapter isolating BPF Loader v3 / v4 differences; 40 hr contingency budgeted separately for Loader v4 / SBPF activation. |

### Milestone 2 — `litesvm` pre-deployment harness — 5 weeks — $7,875

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 2.1 | `spectra harness` subcommand | Loads a hand-curated per-protocol transaction corpus (≤50 tx, committed in pilot's `spectra-fixtures/`) into a `litesvm` instance with `v_{n+1}` loaded. |
| 2.2 | Deserialisation panic reporter | Reports per-tx deserialisation failures (the runtime equivalent of M0's static silent-corruption finding). |
| 2.3 | Account-validation regression reporter | Reports `AccountNotInitialized` / `AccountDidNotDeserialize` / discriminator-mismatch regressions per tx. |
| 2.4 | CPI return-code regression reporter | Reports per-CPI return-code differences. |
| 2.5 | Bounded budget | <60 s wall-clock in a free-tier GitHub Actions runner. Explicitly **not** a mainnet snapshot replay. |
| 2.6 | Worked example | One end-to-end run against a public upgradable Anchor program of the applicant's choosing; corpus committed, report committed. |

### Milestone 3 — Suppression file + Action + PR comment — 4 weeks — $6,300

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 3.1 | `spectra-allow.toml` schema | Per-finding suppression with mandatory `rationale`, `expires`, and `upgrade_pr` fields. No silent waivers. Schema documented in [`docs/MIGRATION.md`](docs/MIGRATION.md). |
| 3.2 | Composite GitHub Action | Published on the GitHub Marketplace; Spectra's own CI uses the published action as its smoke test. |
| 3.3 | PR comment integration | Single-comment-per-PR format that updates in place on subsequent pushes (no thread spam). |
| 3.4 | mdBook getting-started page | Hosted on GitHub Pages; covers install, first run, suppression file workflow. |

### Milestone 4 — Pilot + walkthroughs + community docs — 3 weeks — $5,400

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 4.1 | ≥1 confirmed protocol pilot | LOI signed during M0–M3 (outreach template at [`02_proposals/drafts/solana-program-verification-tooling/loi_outreach.md`](../02_proposals/drafts/solana-program-verification-tooling/loi_outreach.md)); pilot integrates Spectra into their CI. Pilot CI logs published. |
| 4.2 | 2 publicly documented integration walkthroughs | Against real upgradable Anchor programs of the applicant's choosing (no signature required). Each walkthrough is plain markdown: commits + CI run links + diff report + analysis. |
| 4.3 | mdBook documentation complete | Full reference + per-milestone tutorial. |
| 4.4 | Solana Discord AMA | One community office-hour session. Recording published. |

### Budget Breakdown

| Category | Item | Hours/Week | Duration | Rate | Total |
| --- | --- | --- | --- | --- | --- |
| Personnel | Lead engineering (applicant) | 35 hr | 16 weeks | $45/hr | $25,200 |
| Personnel | Pilot integration support (M4) | 5 hr | 3 weeks | $45/hr | $675 |
| Personnel | Loader-version adapter buffer (one-time contingency for Loader v4 / SBPF activation mid-grant) | 40 hr | one-time | $45/hr | $1,800 |
| Subscriptions | Archive RPC for `litesvm` corpus curation (Helius / Triton dev tier) | — | 4 months @ $80/mo | — | $320 |
| Subscriptions | Domain + mdBook hosting | — | 4 months | — | $120 |
| | | | | **Total** | **$28,115 USD** |

Rate alignment: $45/hr is conservative against Solana Mobile Builder Grant precedent ($10K × 10 teams) scaled for a multi-month engineering deliverable, and well below typical audit-firm engineering rates for equivalent Solana-security work. Both contingency line items address publicly known risks (SIMD-tracked loader upgrades; absence of an in-CI mainnet snapshot path) rather than scope expansion.

---

## How Spectra compares to generic JSON / YAML diff tools

A fair early question: "isn't there already a JSON diff tool that does this?" We tested the four most-installed candidates on the same Drift IDL pair. Best of 5 runs each, commodity laptop:

| Tool | Wall-clock on 428 KB Drift IDL | Exits 1 on BREAKING only? | False positive on whitespace reformat? | Detects silent corruption? |
|---|---:|---|---|---|
| `diff -u` (GNU 3.10) | 5 ms | no | **yes — 39,715 noise lines** | no |
| `jd` (Go) | 32 ms | no (exits 1 on any change) | no | no |
| `dyff` (Go) | 106 ms | no (exits 0 *always*) | no | no |
| `json-diff` (npm) | 9,217 ms | no (exits 1 on any change) | no | no |
| **Spectra** | **6 ms** | **yes** | **no** | **yes** |

Plain-words translation:

- Every other tool either blocks every harmless additive change (bar 1), never blocks anything (bar 2), or trips on `prettier --write` (bar 3). Each one fails to be useful as a CI gate for at least one reason.
- Spectra is the only one that knows what an Anchor discriminator is, so it is the only one that can detect silent-corruption at all.
- Spectra is ~16× faster than the fastest semantic alternative (`jd`) and within ~1.5× of `diff -u` while doing dramatically more useful work.

Full methodology, raw measurements, and reproduction commands: [`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md).

---

## Future Plans

**Maintenance Commitment:**
Beyond the funded milestones, the applicant commits to maintaining Spectra for a **minimum of 12 months** after M4 completion — bug fixes, security patches, compatibility updates for new Anchor / Shank / Codama schema versions, and a Loader v4 adapter (or later) within 4 weeks of any new BPF-loader version reaching mainnet activation.

**Sustainability Strategy:**
- **Bounded surface area.** Spectra deliberately does not chase formal verification, runtime monitoring, or full mainnet replay. The thing it does — schema-and-discriminator-aware diff — has a small, well-defined surface that does not require constant catch-up with the rest of the Solana stack.
- **Loader-version isolation.** Architecture isolates BPF Loader concerns behind a thin adapter (see [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md)). A new loader version is a one-module change, not a rewrite.
- **Dogfooding.** Spectra's own CI uses the M3-published Action as its smoke test starting M3, so any regression in the Action breaks Spectra's own merges first.
- **No SaaS commitment.** The tool is a CLI + Action + library. No hosted service, no infrastructure surface, no subscription billing. Maintenance load is proportional only to code + Solana schema drift.

**Short-term Plans (6–12 months post-grant):**
- Active engagement with the Solana developer community via the [forum.solana.com RFP thread](https://forum.solana.com/t/program-verification-tooling/1032), the Solana Discord `#tooling` channel, and the Anchor Discord.
- Outreach to audit firms (OtterSec, Halborn, Trail of Bits, Neodyme, Zellic) offering Spectra as a free pre-engagement diff utility, with the explicit goal of getting it into standard audit-engagement workflows.
- Coordinate with Solana Foundation security partners so Spectra-flagged findings appear consistently in pre-deployment review checklists.

**Long-term Vision:**
- Establish Spectra as the default CI-time upgrade-safety gate for Solana programs, analogous to the role `slither` plays for Solidity.
- Expand the rule catalogue to cover Token-2022 TLV extensions (currently out of scope) and a richer set of native-program patterns as Shank-IDL adoption grows.
- Contribute the diff engine as a reusable Rust crate so audit firms and downstream tools can embed it without forking.

---

## Additional Information

**Work Already Completed (pre-grant, at the applicant's own cost):**
- Complete M0 implementation: Rust core + Python wrapper + GitHub Action scaffold + CI workflow + 8 green tests + 17+ engineering docs.
- Synthetic-regression fixture demonstrating all 11 M0 rule kinds.
- Real-world validation on Drift Protocol v2.155 → v2.162 with one detected silent-corruption case.
- Competitive benchmark against `diff -u`, `jd`, `dyff`, `json-diff` with reproducible methodology.
- Q1-style technical paper at [`docs/PAPER.md`](docs/PAPER.md) (~600 lines, 9 sections).
- Apache 2.0 licensing with explicit patent grant.

**Financial Contributions:**
No other teams or entities have contributed financially to this project. This is an independent development effort funded by the applicant's own time.

**Other Funding Applications:**
This project has not been submitted for funding to any other entity. The Solana Foundation grant proposal (referenced above) is the first and only funding application for Spectra.

**Technical Considerations:**
- **License:** Apache 2.0 (matches `solana-verifiable-build`, the Solana SDK, and Anza-published developer tooling; includes an explicit patent grant per Apache §3).
- **Type safety:** Rust 2021 edition; `cargo clippy --all-targets -- -D warnings` enforced in CI.
- **Determinism:** All output is deterministic over the same input pair. The identical-IDL-input-emits-zero-findings invariant is asserted in both a test and a CI step.
- **Semantic versioning:** Will be applied from the first tagged release.
- **No proprietary dependencies.** Every crate is on crates.io under a permissive license.

**Project Impact:**
The Solana RFP for Program Verification Tooling explicitly names upgrade-safety regression as a missing layer. Spectra is the smallest credible filling of that gap, deployable in a single CI step, with severity-tiered output that lets teams ratchet enforcement without disabling the gate. By making the silent-corruption and discriminator-collision cases visible *before* deploy rather than after, Spectra reduces the rate at which avoidable on-chain incidents become public — which is the metric the ecosystem actually cares about.

---

## Documentation Index

Every claim in this README is backed by one of these docs. Start with whichever question you're asking:

| If you want to know… | Read |
|---|---|
| The full technical story end-to-end (academic-style) | [`docs/PAPER.md`](docs/PAPER.md) |
| Whether `git diff` is good enough (no — formal argument) | [`docs/VS_GIT_DIFF.md`](docs/VS_GIT_DIFF.md) |
| The reproducible synthetic before/after walkthrough | [`docs/BENCHMARK.md`](docs/BENCHMARK.md) |
| The real-world Drift IDL benchmark | [`docs/BENCHMARK_DRIFT.md`](docs/BENCHMARK_DRIFT.md) |
| Head-to-head against `jd`, `dyff`, `json-diff`, `diff -u` | [`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md) |
| The threat model and adversary classes | [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) |
| What Spectra explicitly is **not** | [`docs/NON_GOALS.md`](docs/NON_GOALS.md) |
| Every rule ID + severity + exit-code contract | [`docs/SEVERITY.md`](docs/SEVERITY.md) |
| Pipeline architecture across M0–M3 | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| Per-edge-case coverage matrix (25 Solana concerns) | [`docs/SOLANA_EDGE_CASES.md`](docs/SOLANA_EDGE_CASES.md) |
| How false positives are kept at zero | [`docs/FALSE_POSITIVES.md`](docs/FALSE_POSITIVES.md) |
| Drop-in GitHub Actions / pre-commit / `cargo make` templates | [`docs/CI_INTEGRATION.md`](docs/CI_INTEGRATION.md) |
| The milestone roadmap (acceptance-test-gated) | [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Three-layer detection corpus design | [`docs/CORPUS.md`](docs/CORPUS.md) |
| M2 bounded-replay architecture | [`docs/REPLAY.md`](docs/REPLAY.md) |
| Rule engine internals + the M1 `Rule` trait | [`docs/RULE_ENGINE.md`](docs/RULE_ENGINE.md) |
| `spectra-allow.toml` migration-declaration schema | [`docs/MIGRATION.md`](docs/MIGRATION.md) |
| Anchor-specific hazards (Borsh, discriminators, zero-copy, events) | [`docs/ANCHOR.md`](docs/ANCHOR.md) |
| Adoption plan + pilot strategy | [`docs/ADOPTION.md`](docs/ADOPTION.md) |

---

## Project Layout

```
spectra/
├── spectra-core/          # Rust crate + spectra binary (the actual tool)
├── spectra-cli/           # Python wrapper (subprocess-invokes the Rust bin)
├── spectra-action/        # GitHub Action scaffold (full Marketplace publish = M3)
├── examples/              # Synthetic-regression Anchor IDLs for demo + tests
├── scripts/record-demo.sh # asciinema recorder for the demo cast
├── docs/                  # All engineering documentation (see index above)
└── .github/workflows/     # CI: fmt + clippy + test + green-demo verification
```

---

## License

Apache License 2.0. See [`LICENSE`](LICENSE).

## Contributing

See [`CONTRIBUTING.md`](CONTRIBUTING.md). Issue triage SLA during the grant period: 7 days.

## Security

Please do not file public issues for exploitable security findings. Contact the maintainer privately at ayodyadsr@gmail.com. A formal `SECURITY.md` policy will be published after the grant decision.
