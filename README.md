# Spectra

[![CI](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml/badge.svg)](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Project Overview

**Tagline:** A CI-time, strictly-differential account-validation security-regression gate for Solana program upgrades — it fires only when an upgrade *removes or weakens* an account-validation guard the already-deployed version enforced.

**Project Description:**
Spectra is an open-source CLI and GitHub Action that parses two Anchor program source trees — a **baseline** (the last released / on-chain-deployed version) and a **candidate** (the upgrade PR under review) — extracts the per-account-slot security-guard set Anchor enforces for each `#[derive(Accounts)]` context, and reports a finding **only** when the candidate drops, downgrades, or bypasses a guard the baseline enforced. It emits machine-readable JSON, human-readable Markdown, or SARIF 2.1.0 (uploaded straight to GitHub's Security tab), and gates a CI build with a severity-tiered exit code. Every claim in this README is reproducible from this repository.

**Problem Statement:**

Access-control / account-validation failures are the single largest quantified loss class in the Solana ecosystem. The canonical bugs — a missing signer check, a missing owner / discriminator check (type cosplay), an unpinned CPI target, a dropped `has_one` / PDA-seeds / custom constraint — are well understood, and absolute scanners (Sec3 X-Ray, Auditware Radar, l3x, Octane) already hunt for them in a single snapshot. But there is a structural gap none of those tools fills:

1. **A guard that was *present* in the deployed version and *removed* on upgrade is invisible to a snapshot scanner.** An absolute scanner asks "is there a missing owner check *anywhere* in this code?" — it has no notion of "the previous version", so a guard silently deleted in an upgrade PR does not register as a *regression*; at best it competes with every other absolute finding and its heuristic false-positive budget.
2. **Absolute scanners must tune heuristics to keep false positives down.** A regression gate that only ever compares against the deployed baseline has a false-positive rate that is near-zero *by construction*, not by tuning: identical input yields zero findings, and a finding can only mean "this PR took away a guarantee the deployed program already gave its users."
3. **No public Solana tool gates an upgrade PR on account-validation *regression* specifically.** Build-provenance tools (`solana-verifiable-build` / `anchor verify`) answer a different question; the Foundation's STRIDE / SIRN stack is post-deployment; absolute scanners are stateless. The differential-regression layer is open.

**Solana Integration:**
Solana programs are almost always deployed via an *upgradeable* loader. Anchor's `#[derive(Accounts)]` context structs are the declarative place where account-validation guards live: `Signer<'info>`, typed wrappers (`Account<'info, T>` / `AccountLoader` / `InterfaceAccount`) that enforce the owner + 8-byte discriminator check, `Program<'info, T>` CPI-target pins, and `#[account(...)]` attributes (`signer`, `has_one`, `owner`, `address`, `seeds`/`bump`, `constraint`). Downgrading a typed slot to `UncheckedAccount` / `AccountInfo`, or deleting a constraint attribute, silently removes a runtime security check with no compiler error. Spectra models exactly this guard set per slot and diffs it across versions.

**Founder Interest:**
The applicant has 20+ years in offensive security — most recently Red Team lead at Indonesia's largest commercial bank by assets (a top-tier Southeast Asian financial institution serving tens of millions of users) — and has built CI-time security gates and detection signatures for production engineering teams. The strictly-differential framing (compare against the deployed baseline; near-zero false positives by construction) is a detection-engineering pattern, not a generic-static-analysis one, and it is the position no existing free Solana scanner occupies.

**Expected Impact:**

Concrete, verifiable outcomes (each maps to evidence in this repo):

1. **Pre-deploy detection of account-validation *regressions*.** The bundled synthetic baseline → candidate Anchor fixture pair drops five distinct guards on one instruction and adds a brand-new unvalidated account slot; Spectra reports 6 BREAKING + 1 warning and exits `1`. An unchanged context in the same changed program yields zero findings. Evidence: [`examples/`](examples/), [`spectra-core/tests/integration_test.rs`](spectra-core/tests/integration_test.rs).
2. **Near-zero false positives by construction.** Identical input (same tree as baseline and candidate) yields zero findings and exit `0` — asserted by a test and a dedicated CI step. This is the structural property, not a tuned heuristic.
3. **CI-gateable severity contract that integrates with existing GitHub Security tooling.** SARIF 2.1.0 output uploads to GitHub Code Scanning via the standard `github/codeql-action/upload-sarif@v3` action — same surface as CodeQL — with a 3-level exit-code contract (`0`/`1`/`2`) verified by dedicated CI steps. Evidence: [`docs/SEVERITY.md`](docs/SEVERITY.md), [`docs/CI_INTEGRATION.md`](docs/CI_INTEGRATION.md).
4. **Complementary to — not a re-implementation of — absolute scanners.** Spectra stays silent on a missing check that was *already missing in the baseline* (that is the absolute scanners' job, by construction not a regression). It fires only on a removed guarantee. Evidence: [`docs/STRIDE_GAP_ANALYSIS.md`](docs/STRIDE_GAP_ANALYSIS.md).

### Project Details

**Technology Stack:**
- **Rust 2021 edition** — core engine ([`spectra-core`](spectra-core)): `syn` 2 (full + extra-traits) and `proc-macro2` for parsing Anchor source, `walkdir` for tree traversal, `serde` / `serde_json` for report serialisation, `clap` 4 for the CLI.
- **Python 3.9+ wrapper** — [`spectra-cli`](spectra-cli), subprocess-invokes the Rust binary so the same engine is reachable from Python-first CI environments.
- **GitHub Action scaffold** — [`spectra-action`](spectra-action), composite action; full Marketplace publish lands in M3.
- **SARIF 2.1.0** — output format for GitHub Code Scanning, consumed by `github/codeql-action/upload-sarif@v3`.

**Core Architecture:**
The engine is intentionally narrow and deterministic. Three modules:

- `accounts` — walks a program source tree, finds every `#[derive(Accounts)]` struct (recursing into `mod` blocks), and reduces each account slot to the set of security `Guard`s Anchor enforces for it: `Signer`, `Typed(T)` (owner + discriminator / type-cosplay), `Owner`, `Address`, `HasOne`, `Seeds`, `Constraint`, `ProgramId`. Files that fail to parse as Rust are skipped, not fatal. Guard sets are `BTreeSet`s so the model is deterministic.
- `regression` — strictly-differential differ. For each context present in *both* versions, for each guard present in the baseline slot and absent from the candidate slot, it emits a typed `Finding`. Downgrade rules require that *no* equivalent pin remains (e.g. `Typed → Owner` is not a regression; `Typed → UncheckedAccount` is).
- `report` — renders findings as JSON, Markdown, or SARIF 2.1.0. SARIF maps `Breaking → level: error`, `Warning → level: warning`, with a per-rule rule catalogue.

**CLI Specification:**
```text
spectra check --baseline <DIR> --candidate <DIR> [--report <PATH>] [--format json|markdown|sarif] [--quiet]
```

```bash
# Detect regressions and gate the merge:
spectra check --baseline examples/vault_baseline --candidate examples/vault_candidate --format markdown
# → exit 1, prints 6 BREAKING + 1 warning findings

# Strictly-differential invariant: identical input → zero false positives:
spectra check --baseline examples/vault_baseline --candidate examples/vault_baseline --format json --quiet
# → exit 0, zero stdout

# GitHub Code Scanning integration:
spectra check --baseline base/ --candidate pr/ --format sarif --report out.sarif
# Then: github/codeql-action/upload-sarif@v3 with sarif_file: out.sarif
```

Exit-code contract (verified in CI):

| Code | Meaning |
|---|---|
| `0` | Clean — no breaking regressions. |
| `1` | At least one BREAKING regression — block the merge. |
| `2` | Invocation error — bad path, unreadable source, unknown `--format` value. |

**What Spectra is NOT:**
- **Not an absolute scanner.** It does **not** find missing checks that were already missing in the baseline — that is Sec3 X-Ray / Auditware Radar / l3x / Octane territory, and Spectra is complementary to them, not a replacement.
- **Not a formal verifier.** It does not prove invariants are preserved; that is audit-firm territory.
- **Not a build-provenance tool.** It does not check that the deployed `.so` matches public source; that is `solana-verify` / `anchor verify`.
- **Not a runtime monitor.** It is pre-merge only; for post-deploy alerting see Hypernative / Range.
- **Not a native-program analyser at M0.** Non-Anchor manual `is_signer` / `owner ==` checks are a documented M1 roadmap item, not silently mis-handled — see [`docs/NON_GOALS.md`](docs/NON_GOALS.md).

### Ecosystem Fit

**Ecosystem Position:**

| Question | Existing tool | Spectra? |
|---|---|---|
| Does the deployed bytecode match public source? | `solana-verify`, `anchor verify` | No — different layer (build provenance) |
| Is there a missing account-validation check *anywhere* in this snapshot? | Sec3 X-Ray, Auditware Radar, l3x, Octane | No — that is the absolute scanners' job |
| **Did this upgrade PR *remove or weaken* a guard the deployed version enforced?** | **(no public tool before Spectra)** | **Yes — this is the gap** |
| Did something go wrong after the upgrade went live? | Hypernative, Range, STRIDE | No — too late by then |

**Target Audience:**
- **Primary:** Solana program teams shipping upgradeable Anchor programs into production (DeFi protocols, NFT marketplaces, oracles).
- **Secondary:** Audit firms running a pre-engagement regression diff between a client's deployed version and their proposed upgrade.
- **Tertiary:** Solana Foundation security partners who want a standard upgrade-PR regression gate alongside the absolute scanners they already recommend.

**Need Identification:**
After the April 2026 Drift incident the Foundation consolidated its security investment around STRIDE (post-deployment operational evaluation) and SIRN (incident response), plus a recommended set of free absolute scanners (Sec3 X-Ray, Auditware Radar, and others). Every one of those is either post-deployment or single-snapshot. None gates an *upgrade PR* on whether it regresses an account-validation guarantee relative to the deployed baseline. Full lifecycle mapping: [`docs/STRIDE_GAP_ANALYSIS.md`](docs/STRIDE_GAP_ANALYSIS.md).

**Similar Projects in the Solana Ecosystem:**
- **Sec3 X-Ray / Auditware Radar / l3x / Octane** — absolute (single-snapshot) static scanners. They have no notion of "the previous version", so a guard silently deleted in an upgrade is not surfaced *as a regression*; their false-positive rate is heuristic-tuned. Spectra is strictly-differential and complementary: it stays silent on absolute findings and fires only on removed guarantees.
- **`solana-verifiable-build` / `anchor verify`** — build provenance; a different layer.
- **STRIDE (Asymmetric Research)** — evaluates a *deployed* protocol's operational posture; one lifecycle stage later than Spectra.
- **Audit-firm formal verification** — proves invariants on a specific revision; different cost class and scope.
- **Hypernative / Range** — runtime monitors that fire *after* a bad upgrade ships; a different layer.

Spectra is therefore complementary, not substitutive, to every tool in this list.

---

## Team

- **Team Name:** Ayodya (independent contributor)
- **Contact Name:** Ayodya
- **Contact Email:** ayodyadsr@gmail.com
- **Website / Repository:** https://github.com/ayodyadsr/spectra

### Team members

- **Ayodya** (lead engineer, sole maintainer for M0–M4 scope)

#### LinkedIn Profiles (if available)

- Available privately upon request from the grant committee; not publicly listed.

### Team Code Repos

- Spectra (this repo): https://github.com/ayodyadsr/spectra

### Team GitHub Accounts

- https://github.com/ayodyadsr

### Team's Experience

20+ years of offensive-security and detection-engineering work, most recently Red Team lead at Indonesia's largest commercial bank by assets (a top-tier Southeast Asian financial institution serving tens of millions of users). Three skill surfaces map one-to-one onto Spectra's design:

1. **Authoring detection signatures with a near-zero false-positive bar** ↔ Spectra's strictly-differential design (a finding can only mean "a guarantee was removed").
2. **Differential / regression analysis for vulnerability discovery** ↔ the baseline-vs-candidate guard-set diff engine.
3. **Building CI-time security gates for production engineering teams** ↔ Spectra's severity-tiered exit code, SARIF output, and GitHub Action surface.

The applicant has no prior Solana-specific OSS contributions; the M0 PoC shipped before grant submission — with green CI from the first commit — is the direct mitigation for that gap.

---

## Development Status

**Current Status:** M0 PoC is shipped, public, and Apache-2.0-licensed. The repository is referenced in the grant proposal at [`02_proposals/drafts/solana-program-verification-tooling/final_proposal.md`](../02_proposals/drafts/solana-program-verification-tooling/final_proposal.md). Submission to https://solana.org/grants-funding is pending final pilot LOI outreach.

**Proof of Concept:**
The bundled synthetic fixture pair models a realistic upgrade regression. [`examples/vault_baseline`](examples/vault_baseline) is the "deployed" Anchor vault program; [`examples/vault_candidate`](examples/vault_candidate) is the "upgrade under review" that silently:

- drops `has_one = authority` on the `vault` slot of `Withdraw`;
- downgrades `authority` from `Signer<'info>` to `UncheckedAccount<'info>` (signer check removed);
- downgrades `destination` from `Account<'info, TokenAccount>` to `UncheckedAccount<'info>` and drops its `constraint` (type-cosplay + custom constraint removed);
- drops the PDA `seeds`/`bump` derivation on `config`;
- downgrades `token_program` from `Program<'info, Token>` to `UncheckedAccount` (CPI target no longer pinned);
- adds a brand-new `EmergencyDrain` context with an unvalidated `anyone` slot (new attack surface — *warning*, not a regression of an existing guarantee).

The `Initialize` context is byte-identical between the two versions and **must** produce zero findings — the strictly-differential property. Verbatim Spectra output (exit `1`):

```markdown
# Spectra Account-Validation Regression Report

**Findings:** 6 breaking, 1 warning

| Severity | Rule | Detail |
|---|---|---|
| BREAKING | has_one_constraint_removed | `Withdraw::vault` dropped `has_one = authority` |
| BREAKING | signer_check_removed | `Withdraw::authority` no longer requires a signer (baseline did) |
| BREAKING | type_cosplay_protection_removed | `Withdraw::destination` downgraded `TokenAccount` -> `UncheckedAccount<'info>` (owner+discriminator check lost) |
| BREAKING | custom_constraint_removed | `Withdraw::destination` dropped `constraint = destination.owner==authority.key()` |
| BREAKING | pda_derivation_removed | `Withdraw::config` dropped PDA `seeds`/`bump` derivation |
| BREAKING | cpi_target_unpinned | `Withdraw::token_program` CPI target program id no longer pinned |
| warning | unvalidated_account_introduced | `EmergencyDrain::anyone` new UncheckedAccount/AccountInfo slot (new attack surface) |

> Spectra exits non-zero when any BREAKING finding is present: this upgrade takes away a security guarantee the deployed version already gave its users. Review each row before deploy.
```

**Real-world validation:** `[NO PUBLIC DATA AVAILABLE]` at M0. A reproducible benchmark against a real public Anchor program's deployed-vs-upgrade source pair is an explicit M1 deliverable (see Roadmap M1.5). The M0 evidence base is the synthetic fixture above plus the test suite — no real-world numbers are claimed that have not been measured.

**Development Progress (M0 — shipped):**
- ✅ Rust core engine ([`spectra-core`](spectra-core)) + `spectra` CLI binary.
- ✅ Python wrapper ([`spectra-cli`](spectra-cli)) with `spectra-py` entry point.
- ✅ GitHub Action scaffold ([`spectra-action/action.yml`](spectra-action/action.yml)).
- ✅ 9 finding kinds covering Anchor `#[derive(Accounts)]` guard regressions (full table below).
- ✅ JSON, Markdown, and SARIF 2.1.0 output formats.
- ✅ Severity-tiered exit code (0 / 1 / 2) verified by dedicated CI steps.
- ✅ 6 integration tests green, including the strictly-differential no-false-positive property (an unchanged context in a changed program yields zero findings).
- ✅ Synthetic baseline → candidate Anchor fixture pair ([`examples/vault_baseline`](examples/vault_baseline) → [`examples/vault_candidate`](examples/vault_candidate)) → 6 BREAKING + 1 warning.
- ✅ `cargo fmt --all -- --check` + `cargo clippy --all-targets -- -D warnings` clean.
- ✅ Apache 2.0 license.

**Testing and Validation:**
- **Static checks gated in CI:** `cargo fmt -- --check`, `cargo clippy --all-targets -- -D warnings`, `cargo build --release --workspace`.
- **Integration tests:** `cargo test --release --workspace` runs 6 tests covering: synthetic-upgrade detection (asserts exactly 6 breaking + 1 warning and each specific finding), identical-program clean report, the strictly-differential no-false-positive property (zero findings name the unchanged `Initialize` context), SARIF schema validity, SARIF clean-empty output, and the Markdown signer-regression row.
- **End-to-end demo gates:** CI runs `spectra check` on the bundled baseline → candidate fixture and asserts exit `1`; runs the same tree against itself and asserts exit `0`; validates SARIF parses as JSON with exactly 7 results; runs an unknown `--format` and asserts exit `2`; runs `--quiet` on a clean input and asserts zero stdout.

**Known Limitations:**
- **Anchor `#[derive(Accounts)]` only at M0.** Native (non-Anchor) manual `is_signer` / `owner ==` checks are an M1 roadmap item, explicitly documented in [`docs/NON_GOALS.md`](docs/NON_GOALS.md), not silently mis-handled.
- **Source-tree input, not on-chain bytecode.** Spectra diffs two source trees; obtaining the baseline tree (e.g. from the verified-build source of the deployed version) is the operator's responsibility at M0.
- **A context removed wholesale** is treated as an interface change, not a silent weakening of a still-callable instruction — out of M0 scope by design.
- **`spectra-allow.toml` suppression file** for intentional guard changes lands in M3.

---

## Quick Start

```bash
# 1. Install Rust (one-time, skip if you already have it):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Clone and build:
git clone https://github.com/ayodyadsr/spectra
cd spectra
cargo build --release --workspace

# 3. Run the demo (the synthetic baseline → candidate fixture pair):
./target/release/spectra check \
  --baseline examples/vault_baseline \
  --candidate examples/vault_candidate \
  --format markdown
# → exit 1, prints the 6 BREAKING + 1 warning findings shown above

# 4. Verify the strictly-differential property (identical input → clean):
./target/release/spectra check \
  --baseline examples/vault_baseline \
  --candidate examples/vault_baseline
# → exit 0, zero findings

# 5. Run the test suite (6 integration tests, all green):
cargo test --release --workspace
```

For a hermetic, container-only reproduction with no Rust toolchain on the host, see the multi-stage [`Dockerfile`](Dockerfile) — also exercised on every push by the `docker` CI job. Full step-by-step testing guide: [`docs/TESTING.md`](docs/TESTING.md).

---

## The 9 things Spectra checks today (M0)

| Finding | Severity | What it means in plain words |
|---|---|---|
| `signer_check_removed` | BREAKING | Baseline required this account to sign; the candidate no longer does. The canonical missing-signer-check bug, introduced on upgrade. |
| `type_cosplay_protection_removed` | BREAKING | Baseline used a typed wrapper (owner + discriminator check); the candidate downgraded the slot to `UncheckedAccount` / `AccountInfo`. |
| `owner_check_removed` | BREAKING | Baseline pinned the account owner (`owner =` / `address =`); the candidate dropped that pin. |
| `has_one_constraint_removed` | BREAKING | Baseline enforced a `has_one` relational-integrity check the candidate dropped. |
| `custom_constraint_removed` | BREAKING | Baseline enforced a custom `constraint =` predicate the candidate dropped. |
| `pda_derivation_removed` | BREAKING | Baseline derived the account as a PDA (`seeds`/`bump`); the candidate dropped the derivation, allowing an arbitrary account. |
| `cpi_target_unpinned` | BREAKING | Baseline pinned a CPI target program id (`Program<'info, T>` / `address`); the candidate downgraded it to an unvalidated account. |
| `validated_account_slot_removed` | BREAKING | A validated account slot present in the baseline context was removed while the context still exists. |
| `unvalidated_account_introduced` | warning | The candidate introduces a brand-new `UncheckedAccount` / `AccountInfo` slot that did not exist in the baseline — new attack surface to review (not a regression of an existing guarantee). |

Canonical rule IDs, severity tiers, and the exit-code contract: [`docs/SEVERITY.md`](docs/SEVERITY.md).

---

## Development Roadmap

This roadmap is the version submitted to the Solana Foundation. Every milestone is gated by **acceptance tests** that must pass in CI on a tagged commit — not by activity. The current state of each acceptance gate is tracked in [`docs/ROADMAP.md`](docs/ROADMAP.md).

### Overview

- **Estimated Duration:** 16 weeks (post-grant) + pre-grant M0 work already shipped.
- **Full-Time Equivalent (FTE):** ~0.875 FTE (35 hr/week × 16 weeks).
- **Total Costs:** **$28,115 USD**.

### Milestone 0 — PoC (pre-grant, shipped)

| Number | Deliverable | Specification | Status |
| ---: | --- | --- | --- |
| 0a. | License | Apache License 2.0 applied to the project. | ✅ ([`LICENSE`](LICENSE)) |
| 0b. | Documentation | `docs/` covering threat model, severity contract, architecture, non-goals, FP policy, CI integration, and adoption plan. README explains setup, CLI, and full report format. | ✅ ([`docs/`](docs/)) |
| 0c. | Testing and Testing Guide | 6 integration tests including the strictly-differential no-false-positive property. Reproduction: `cargo test --release --workspace`. | ✅ |
| 0d. | Demo | Reproducible synthetic fixture pair + asciinema recorder ([`scripts/record-demo.sh`](scripts/record-demo.sh)). | ✅ |
| 1. | Anchor `#[derive(Accounts)]` guard-regression diff | 9 finding kinds, severity-tiered, deterministic output. | ✅ |
| 2. | Output formats | JSON + Markdown + SARIF 2.1.0; SARIF uploads via `github/codeql-action/upload-sarif@v3`. | ✅ |

### Milestone 1 — Native programs + real-world validation — 4 weeks — $6,300

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 0a. | License | Apache License 2.0 (remains in force). |
| 0b. | Documentation | Inline Rustdoc on all public items (`cargo doc` clean under `-D missing_docs`); per-new-finding entry in [`docs/SEVERITY.md`](docs/SEVERITY.md); [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) updated with the native-program parser stage. |
| 0c. | Testing and Testing Guide | Golden-file suite covering each finding kind added to `spectra-core/tests/`; `cargo test --release --workspace` remains the single entry; testing guide describes how to add a new golden case. |
| 0d. | Repository | All M1 source/tests committed; release tagged `v0.3.0-m1`; reproduces deterministically from a clean clone. |
| 0e. | Article | Tutorial walking through a native-program regression diff on a worked public example. |
| 1.1 | Native (non-Anchor) guard extraction | Detect manual `is_signer` / `owner ==` / `key ==` checks so the differ covers non-Anchor programs. |
| 1.2 | Cross-version slot-rename heuristic | Reduce false negatives when a slot is renamed but its guard set is unchanged. |
| 1.3 | Defined-constraint resolution | Resolve `constraint =` expressions that reference helper functions / consts rather than treating them as opaque strings. |
| 1.4 | Whole-context-removal policy | Optional stricter mode that flags removal of an entire validated context. |
| 1.5 | Real-world validation | Reproducible benchmark against a real public Anchor program's deployed-vs-upgrade source pair; report committed. |

### Milestone 2 — `litesvm` pre-deployment harness — 5 weeks — $7,875

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 2.1 | `spectra harness` subcommand | Loads a hand-curated per-protocol transaction corpus (≤50 tx) into a `litesvm` instance with the candidate loaded. |
| 2.2 | Guard-regression replay reporter | Reports per-tx `AccountNotInitialized` / `AccountDidNotDeserialize` / signer-missing regressions that the static differ predicted. |
| 2.3 | Bounded budget | <60 s wall-clock on a free-tier GitHub Actions runner. Explicitly **not** a mainnet snapshot replay. |
| 2.4 | Worked example | One end-to-end run against a public upgradable Anchor program; corpus + report committed. |

### Milestone 3 — Suppression file + Action + PR comment — 4 weeks — $6,300

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 3.1 | `spectra-allow.toml` schema | Per-finding suppression with mandatory `rationale`, `expires`, and `upgrade_pr` fields. No silent waivers. |
| 3.2 | Composite GitHub Action | Published on the GitHub Marketplace; Spectra's own CI uses the published action as its smoke test. |
| 3.3 | PR comment integration | Single-comment-per-PR format that updates in place on subsequent pushes (no thread spam). |
| 3.4 | mdBook getting-started page | Hosted on GitHub Pages; covers install, first run, suppression-file workflow. |

### Milestone 4 — Pilot + walkthroughs + community docs — 3 weeks — $5,400

| Number | Deliverable | Specification |
| ---: | --- | --- |
| 4.1 | ≥1 confirmed protocol pilot | LOI signed during M0–M3 (outreach template at [`02_proposals/drafts/solana-program-verification-tooling/loi_outreach.md`](../02_proposals/drafts/solana-program-verification-tooling/loi_outreach.md)); pilot integrates Spectra into their CI. Pilot CI logs published. |
| 4.2 | 2 publicly documented integration walkthroughs | Against real upgradable Anchor programs of the applicant's choosing. Each is plain markdown: commits + CI run links + diff report + analysis. |
| 4.3 | mdBook documentation complete | Full reference + per-milestone tutorial. |
| 4.4 | Solana Discord AMA | One community office-hour session. Recording published. |

### Budget Breakdown

| Category | Item | Hours/Week | Duration | Rate | Total |
| --- | --- | --- | --- | --- | --- |
| Personnel | Lead engineering (applicant) | 35 hr | 16 weeks | $45/hr | $25,200 |
| Personnel | Pilot integration support (M4) | 5 hr | 3 weeks | $45/hr | $675 |
| Personnel | Native-program parser buffer (one-time contingency) | 40 hr | one-time | $45/hr | $1,800 |
| Subscriptions | Archive RPC for `litesvm` corpus curation (Helius / Triton dev tier) | — | 4 months @ $80/mo | — | $320 |
| Subscriptions | Domain + mdBook hosting | — | 4 months | — | $120 |
| | | | | **Total** | **$28,115 USD** |

Rate alignment: $45/hr is conservative against Solana Mobile Builder Grant precedent scaled for a multi-month engineering deliverable, and well below typical audit-firm engineering rates for equivalent Solana-security work.

---

## Testing & Verification Strategy

Every milestone is **gated by acceptance tests**, not by activity. Each deliverable maps to a specific check a grant reviewer can run on the linked commit.

**Static checks (gated in CI on every push, all milestones):**
- `cargo fmt -- --check` — formatting deviation fails CI.
- `cargo clippy --all-targets -- -D warnings` — every warning is an error.
- `cargo build --release --workspace` — release build must succeed on the Linux GHA runner.

**Test suite (gated in CI):**
- `cargo test --release --workspace` — 6 integration tests pass, including:
  - Synthetic-upgrade detection: exactly 6 BREAKING + 1 warning, with each specific finding asserted by kind + account.
  - Identical-program clean report: zero findings, exit `0`.
  - **Strictly-differential no-false-positive property:** no finding may name the unchanged `Initialize` context even though the program as a whole changed.
  - SARIF schema validity: emitted SARIF parses as JSON, `version == "2.1.0"`, driver name `Spectra`, one result per finding.
  - SARIF clean-empty: zero findings produces a valid SARIF document with an empty `results` array.
  - Markdown signer-regression row: the signer regression is named by rule id and tier in Markdown.

**End-to-end demo gates (gated in CI):**
- Demo exit-1: `spectra check` on the bundled baseline → candidate fixture must exit `1`.
- Identical-tree exit-0: same tree as `--baseline` and `--candidate` must exit `0`.
- SARIF JSON parse: `--format sarif` output must parse as JSON with exactly 7 results and exit `1`.
- Exit-2 invocation error: `--format definitely-not-a-format` must exit `2` (not `1`, not `0`).
- `--quiet` no-output-on-clean: identical-tree run with `--quiet` must produce zero stdout.

**Real-world validation:** `[NO PUBLIC DATA AVAILABLE]` at M0 — an explicit M1.5 deliverable. No real-world performance or detection numbers are claimed at M0 that have not been measured.

**Reproduction from a clean clone:**
```bash
git clone https://github.com/ayodyadsr/spectra && cd spectra
cargo test --release --workspace                   # 6 tests pass
./target/release/spectra check \
  --baseline examples/vault_baseline \
  --candidate examples/vault_candidate              # exit 1
./target/release/spectra check \
  --baseline examples/vault_baseline \
  --candidate examples/vault_baseline               # exit 0
```

---

## Future Plans

**Maintenance Commitment:**
Beyond the funded milestones, the applicant commits to maintaining Spectra for a **minimum of 12 months** after M4 completion — bug fixes, security patches, and compatibility updates for new Anchor schema versions.

**Sustainability Strategy:**
- **Bounded surface area.** Spectra deliberately does not chase formal verification, runtime monitoring, or full mainnet replay. The strictly-differential guard-set diff has a small, well-defined surface.
- **No SaaS commitment.** The tool is a CLI + Action + library. No hosted service, no infrastructure surface, no subscription billing.
- **Dogfooding.** Spectra's own CI uses the M3-published Action as its smoke test starting M3.

**Long-term Vision:**
- Establish Spectra as the default CI-time upgrade-regression gate for Solana programs, run alongside the absolute scanners teams already use.
- Contribute the differ as a reusable Rust crate so audit firms and downstream tools can embed it.

---

## Success Metrics

Success is measured against **acceptance tests defined in [`docs/ROADMAP.md`](docs/ROADMAP.md)**, not adoption marketing numbers. Every metric below is binary (pass/fail) and verifiable from public CI artifacts.

| Metric | M0 (shipped) | M1 target | M2 target | M3 target |
|---|---|---|---|---|
| Finding kinds covered | 9 ✅ | + native-program path | maintained | maintained |
| Tests passing (`cargo test --release --workspace`) | 6 / 6 ✅ | golden-file suite ≥ 1 per finding | + corpus harness | + suppression-file parse |
| Strictly-differential no-FP property | asserted ✅ | asserted | asserted | asserted |
| CI gates green | ✅ | ✅ | ✅ | ✅ |
| Real-world validation | M1.5 deliverable | ≥1 public program pair | maintained | maintained |

**Non-metrics (intentionally not tracked):** GitHub stars, downloads, social followers. These do not measure whether Spectra catches account-validation regressions before deploy; the acceptance tests above measure that directly.

---

## Documentation Index

| If you want to know… | Read |
|---|---|
| The single-file top-level engineering specification | [`TECHNICAL_SPEC.md`](TECHNICAL_SPEC.md) |
| Position vs the Foundation's STRIDE/SIRN stack + absolute scanners | [`docs/STRIDE_GAP_ANALYSIS.md`](docs/STRIDE_GAP_ANALYSIS.md) |
| What Spectra explicitly is **not** | [`docs/NON_GOALS.md`](docs/NON_GOALS.md) |
| Every rule ID + severity + exit-code contract | [`docs/SEVERITY.md`](docs/SEVERITY.md) |
| The threat model and adversary classes | [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) |
| Pipeline architecture across M0–M3 | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| How false positives are kept near-zero by construction | [`docs/FALSE_POSITIVES.md`](docs/FALSE_POSITIVES.md) |
| Drop-in GitHub Actions / pre-commit templates | [`docs/CI_INTEGRATION.md`](docs/CI_INTEGRATION.md) |
| The milestone roadmap (acceptance-test-gated) | [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Step-by-step testing guide (host + Docker) | [`docs/TESTING.md`](docs/TESTING.md) |

---

## Project Layout

```
spectra/
├── spectra-core/          # Rust crate + spectra binary (the actual tool)
│   ├── src/accounts.rs    #   Anchor #[derive(Accounts)] guard extractor
│   ├── src/regression.rs  #   strictly-differential differ
│   └── src/report.rs      #   JSON / Markdown / SARIF renderers
├── spectra-cli/           # Python wrapper (subprocess-invokes the Rust bin)
├── spectra-action/        # GitHub Action scaffold (Marketplace publish = M3)
├── examples/              # Synthetic baseline → candidate Anchor fixtures
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

Please do not file public issues for exploitable security findings. Vulnerability-reporting policy is documented in [`SECURITY.md`](SECURITY.md). Reports go privately to `ayodyadsr@gmail.com`.

## Code of Conduct

This project follows the Contributor Covenant 2.1 — see [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## Changelog

Release notes are maintained in [`CHANGELOG.md`](CHANGELOG.md) in [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/) format.
