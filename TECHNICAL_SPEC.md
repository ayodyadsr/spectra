# Spectra — Technical Specification

**Status:** M0 PoC shipped (`v0.1.0-m0`, 2026-05-15). M1–M4 pending grant.
**License:** Apache-2.0
**Spec version:** 1.0 (matches release `v0.1.0-m0`)

This document is the **single-file engineering reference** for Spectra. It is
the canonical entry point for grant reviewers, audit firms, and downstream
tool authors. Deeper material is linked into the dedicated docs under
[`docs/`](docs/).

If you read **one** doc to understand Spectra end-to-end, read this one.

---

## Table of contents

1. [Scope](#1-scope)
2. [System overview](#2-system-overview)
3. [Public API surface](#3-public-api-surface)
4. [Data model](#4-data-model)
5. [Data flow](#5-data-flow)
6. [State model — how Spectra handles Solana state](#6-state-model--how-spectra-handles-solana-state)
7. [Determinism guarantees](#7-determinism-guarantees)
8. [Error model & exit-code contract](#8-error-model--exit-code-contract)
9. [Extension points](#9-extension-points)
10. [Versioning & compatibility policy](#10-versioning--compatibility-policy)
11. [Non-goals](#11-non-goals)
12. [Cross-reference index](#12-cross-reference-index)

---

## 1. Scope

Spectra is a **static, deterministic, behavioural-regression diff engine for
Solana program upgrades**. It compares two versions of a Solana program's
Anchor IDL JSON and emits structured findings classifying every change as
`BREAKING`, `warning`, or `info`, with a severity-tiered process exit code
suitable for CI gating.

### In scope (M0 — shipped)

- Parse Anchor legacy-schema IDL JSON (single-file, no network I/O).
- Compute Anchor discriminators (`sha256("global:<name>")[..8]`,
  `sha256("account:<name>")[..8]`).
- Detect 11 rule classes spanning instructions, accounts, fields,
  layout-vs-discriminator mismatch, and discriminator collision (full table
  in [`docs/SEVERITY.md`](docs/SEVERITY.md)).
- Render findings as JSON, Markdown, or SARIF 2.1.0.
- Severity-tiered process exit code (`0` / `1` / `2` / `3`).

### Pending in subsequent milestones (NOT shipped in M0)

| Milestone | Surface added |
|---|---|
| M1 | Anchor 2026 (Codama) parser, Shank native-IDL parser, defined-type / events / errors reference graph, on-chain IDL fetch via Solana JSON-RPC adapter |
| M2 | Bounded `litesvm` replay harness (≤50 hand-curated transactions per pilot, <60 s wall-clock) |
| M3 | `spectra-allow.toml` suppression schema, GitHub Marketplace Action, PR-comment integration |
| M4 | ≥1 pilot integration, 2 publicly documented integration walkthroughs, mdBook documentation, Solana Discord AMA |

### Out of scope (permanent — not deferred)

See [`docs/NON_GOALS.md`](docs/NON_GOALS.md). Highlights:

- **Not a formal verifier.** Audit-firm formal-verification territory
  (~$15k–$100k, 2–6 weeks).
- **Not a build-provenance tool.** `solana-verifiable-build` /
  `anchor verify` answer that question — different layer.
- **Not a runtime monitor.** Pre-merge only; post-deploy monitoring is
  Hypernative / Range territory.
- **Not a mainnet-replay harness.** M2 uses `litesvm` against a bounded
  per-pilot corpus, not a historical mainnet snapshot.
- **Not a Token-2022 TLV-extension detector.** TLV is not described in
  Anchor IDL.

---

## 2. System overview

```
┌──────────────────────────────────────────────────────────────────────┐
│                              Spectra                                 │
│                                                                      │
│  ┌────────────┐    ┌────────────┐    ┌──────────┐    ┌────────────┐ │
│  │            │    │            │    │          │    │            │ │
│  │  IDL  v_n  │──▶ │   PARSE    │──▶ │   DIFF   │──▶ │   RENDER   │ │
│  │  IDL v_n+1 │    │  (idl/*)   │    │ (diff/*) │    │ (report/*) │ │
│  │            │    │            │    │          │    │            │ │
│  └────────────┘    └────────────┘    └──────────┘    └────────────┘ │
│                                            │                         │
│                                            ▼                         │
│                                     ┌──────────────┐                 │
│                                     │  EXIT CODE   │                 │
│                                     │  0 / 1 / 2 / 3│                │
│                                     └──────────────┘                 │
└──────────────────────────────────────────────────────────────────────┘
```

Five Rust modules in [`spectra-core/src/`](spectra-core/src/), one binary
entry point, one Python wrapper, one GitHub Action scaffold.

| Module | Lines (M0) | Responsibility |
|---|---:|---|
| `idl.rs` | 131 | Parse Anchor legacy-schema IDL JSON → strongly-typed `Idl` |
| `discriminator.rs` | 65 | `sha256("global:<n>")[..8]` / `sha256("account:<n>")[..8]` |
| `diff.rs` | 404 | Pairwise IDL comparison → `Vec<Finding>` (11 rule classes) |
| `report.rs` | 340 | Render `DiffReport` as JSON / Markdown / SARIF 2.1.0 |
| `main.rs` | 108 | `clap`-based CLI, exit-code mapping |
| `lib.rs` | 19 | Public re-exports |

Total: **~1,070 lines of Rust** for the entire M0 surface. Bounded by
design — the rule corpus is intentionally narrow.

---

## 3. Public API surface

### 3.1 CLI surface (`spectra` binary)

```text
spectra check
  --old <PATH>          # baseline (v_n) Anchor IDL JSON
  --new <PATH>          # candidate (v_{n+1}) Anchor IDL JSON
  [--report <PATH>]     # optional file to write the rendered report to
  [--format <FMT>]      # json | markdown | sarif         (default: json)
  [--quiet]             # suppress stdout on clean runs   (default: off)
```

**Stability:** Stable for M0. New flags are additive; existing flag semantics
will not change within a major version. Removal of a flag requires a major
version bump.

### 3.2 Library surface (`spectra_core` crate)

Three public entry points, three public types:

```rust
// Public entry points (all infallible except IDL load):
pub fn Idl::from_path(path: &Path) -> Result<Idl>;
pub fn diff_idls(old: &Idl, new: &Idl) -> DiffReport;
pub fn report::render_markdown(report: &DiffReport) -> String;
pub fn report::render_sarif(report: &DiffReport, target: &str) -> String;

// Public types (non-exhaustive — new variants may be added in minor releases):
pub struct Idl { /* … */ }
pub struct DiffReport { /* … */ }
pub enum Finding { /* 11 variants in M0 */ }
pub enum Severity { Breaking, Warning, Info }
```

**Stability contract:** All variants of `Finding` are
`#[non_exhaustive]`-compatible — new rules may be added in minor releases.
The `DiffReport` struct guarantees the `breaking_count` and `warning_count`
fields are stable across minor versions.

### 3.3 GitHub Action surface

See [`spectra-action/action.yml`](spectra-action/action.yml). Composite
action stub at M0; full Marketplace publication is an M3 deliverable.

---

## 4. Data model

### 4.1 `Idl` — parsed Anchor IDL

```rust
pub struct Idl {
    pub name: String,             // program name, e.g. "drift"
    pub instructions: Vec<Instruction>,
    pub accounts: Vec<Account>,
}

pub struct Instruction {
    pub name: String,             // discriminator input: "global:<name>"
    pub args: Vec<Field>,
    pub accounts: Vec<InstructionAccount>,
}

pub struct Account {
    pub name: String,             // discriminator input: "account:<name>"
    pub kind: TypeKind,           // Struct { fields } | Enum { variants }
}

pub struct Field {
    pub name: String,
    pub ty: String,               // serialised Borsh type, e.g. "u64", "Pubkey", "Vec<u8>"
}
```

Full type definitions: [`spectra-core/src/idl.rs`](spectra-core/src/idl.rs).

### 4.2 `Finding` — a single detected change

11 variants in M0, each carrying the structured fields needed for SARIF
rendering and downstream programmatic remediation. The two security-critical
ones:

```rust
pub enum Finding {
    // ... 9 other variants ...

    /// Silent on-chain corruption: account layout changed but the 8-byte
    /// account discriminator did NOT. The runtime accepts old data and reads
    /// it into the new layout, with no error.
    AccountLayoutChangedSameDiscriminator {
        account: String,
        discriminator: String,    // hex, 16 chars
        detail: String,
    },

    /// Two human-chosen names produce the same 8-byte SHA-256 truncated tag.
    /// The runtime cannot distinguish them — calls get silently misrouted.
    DiscriminatorCollision {
        kind: DiscriminatorKind,  // Instruction | Account
        name_a: String,
        name_b: String,
        discriminator: String,    // hex, 16 chars
    },
}
```

Full enum: [`spectra-core/src/diff.rs`](spectra-core/src/diff.rs). Severity
mapping: [`docs/SEVERITY.md`](docs/SEVERITY.md).

### 4.3 `DiffReport` — the engine's return value

```rust
pub struct DiffReport {
    pub findings: Vec<Finding>,
    pub breaking_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

impl DiffReport {
    pub fn is_clean(&self) -> bool { self.breaking_count == 0 }
}
```

---

## 5. Data flow

A single `spectra check` invocation runs the following pipeline:

```
  ┌─────────────────────────────────────────────────────────────────┐
  │ STEP 1 — INPUT                                                  │
  │  Two filesystem paths: --old, --new. No network access. No env  │
  │  vars consulted. Deterministic over the file contents alone.    │
  └────────────────────────────────────┬────────────────────────────┘
                                       │
                                       ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ STEP 2 — PARSE                                                  │
  │  Idl::from_path(path) reads UTF-8 JSON, deserialises via serde_ │
  │  json into the strongly-typed Idl struct. Unknown fields are    │
  │  ignored (forward-compatible with Anchor IDL evolution).        │
  │  Failure → exit 2 (invocation error, NOT a regression).         │
  └────────────────────────────────────┬────────────────────────────┘
                                       │
                                       ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ STEP 3 — DIFF                                                   │
  │  diff_idls(old, new) runs 11 rule checks in a fixed, documented │
  │  order. Each rule appends typed Findings to a Vec. No rule has  │
  │  side effects; every rule is pure.                              │
  └────────────────────────────────────┬────────────────────────────┘
                                       │
                                       ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ STEP 4 — RENDER                                                 │
  │  report::render_<format>(&report) serialises DiffReport.        │
  │  JSON and SARIF are byte-stable for the same input. Markdown    │
  │  is human-readable; column order is stable but column widths    │
  │  may vary with content.                                         │
  └────────────────────────────────────┬────────────────────────────┘
                                       │
                                       ▼
  ┌─────────────────────────────────────────────────────────────────┐
  │ STEP 5 — EXIT                                                   │
  │  Process exit code derived from DiffReport:                     │
  │    0 if breaking_count == 0                                     │
  │    1 if breaking_count >  0                                     │
  │    2 on invocation error (bad path, bad JSON, bad --format)     │
  │    3 on refuse-to-analyse (unrecognised IDL shape — M1+)        │
  └─────────────────────────────────────────────────────────────────┘
```

**Wall-clock budget:** ≤ 20 ms on a 1 MB IDL on commodity hardware. Measured
6 ms on a 428 KB production Drift IDL (M0 acceptance benchmark — see
[`docs/BENCHMARK_DRIFT.md`](docs/BENCHMARK_DRIFT.md)).

---

## 6. State model — how Spectra handles Solana state

Spectra has **no persistent state** at M0. It is a stateless pure-function
CLI:

```
spectra_check(idl_old_bytes, idl_new_bytes) → (report_bytes, exit_code)
```

Same inputs → bit-identical outputs. No filesystem writes outside
`--report`. No network. No environment-variable reads. This property is
load-bearing for CI reproducibility and is asserted in two tests
(`identical_idls_produce_clean_report`, `sarif_clean_report_has_zero_results`).

### State surfaces added per milestone

| Milestone | State surface | Persistence | Failure mode |
|---|---|---|---|
| **M0** (shipped) | None | n/a | n/a |
| **M1** | On-chain IDL fetch via Solana JSON-RPC (`spectra-rpc` adapter) | Ephemeral — fetched IDL is held in memory for one diff, then dropped | Network failure → exit 3 (refuse-to-analyse); never silently exit 0. Failure-mode contract: [`docs/TESTING.md`](docs/TESTING.md) M1 extension |
| **M2** | In-memory `litesvm` VM with ≤ 50-tx corpus | Ephemeral — VM is constructed per invocation, dropped at end | Corpus exceeds budget (60 s) → exit 3; deserialisation panic in a replayed tx → exit 1 (regression) |
| **M3** | `spectra-allow.toml` suppression file | Filesystem (committed to user repo) | Schema violation → exit 2; expired suppression → exit 1 (no silent waivers) |

### Solana-specific state hazards Spectra detects

Spectra exists because the **upgradeable BPF Loader does not validate that a
new program's account layout is compatible with on-chain accounts already
written by the old program.** Two state hazards are unique to Solana's
upgrade model and Spectra detects both statically from the IDL pair:

1. **`AccountLayoutChangedSameDiscriminator`** — the 8-byte Anchor account
   discriminator stays the same but the field layout has changed. The
   runtime accepts old account data and deserialises it into the new shape
   without error. *Money on paper becomes wrong money on chain.* Detected
   by comparing per-account `(discriminator, layout_hash)` pairs across the
   IDL pair.

2. **`DiscriminatorCollision`** — two instructions (or two accounts) have
   different names but produce the same `sha256("global:<name>")[..8]`. The
   runtime cannot tell them apart. *Calls silently misroute.* Detected by
   computing discriminators for every name in both IDLs and checking the
   resulting 8-byte tags are pairwise unique.

These are the **only two findings whose absence from M0 would have been a
correctness bug**. The other 9 rules cover the conventional
shape-of-the-interface changes — important but not Solana-unique.

---

## 7. Determinism guarantees

| Property | Guaranteed at M0? | How |
|---|---|---|
| Same input → identical JSON output, byte-for-byte | Yes | `serde_json::to_string_pretty` over an ordered struct; no `HashMap` in the serialised path |
| Same input → identical SARIF output, byte-for-byte | Yes | Same — SARIF is serialised through `serde_json` over an ordered struct |
| Same input → identical Markdown output, byte-for-byte | Yes (modulo content-dependent column widths) | Tables are rendered with a fixed column order; row order is the natural order of `findings` |
| Order of `findings` in `DiffReport` is stable across runs | Yes | Rules append in a fixed, documented order; within a rule, items are processed in the input IDL's natural order |
| Discriminator computation is bit-exact across platforms | Yes | `sha2` crate, no platform-dependent code paths |
| Wall-clock latency is bounded | Yes (best-effort) | M0 measured at 6 ms on 428 KB; budget is ≤ 20 ms / 1 MB |

The identical-IDL-input-emits-zero-findings invariant is asserted by both a
unit test and a CI step against a 428 KB production IDL — i.e. the
false-positive floor is established on real data, not a synthetic fixture.

---

## 8. Error model & exit-code contract

### 8.1 Canonical exit codes (stable across major versions)

| Code | Meaning | Examples |
|---|---|---|
| `0` | Clean — no `BREAKING` findings | All-warning runs; clean runs |
| `1` | Regression — at least one `BREAKING` finding | Synthetic-regression fixture |
| `2` | Invocation error — input is structurally invalid | Missing file; non-UTF-8 JSON; unknown `--format` |
| `3` | Refuse-to-analyse — input is shape Spectra cannot soundly diff (M1+) | Unrecognised IDL schema; RPC failure when fetching on-chain IDL |

`0` and `1` are exhaustively distinguished by whether
`DiffReport::breaking_count > 0`. `2` is invocation; `3` is "structurally OK
but we cannot draw a sound conclusion." The two are kept distinct so CI can
treat them differently (e.g. `2` = engineer typed wrong flag, retry; `3` =
toolchain limitation, escalate).

### 8.2 Error type (M0 — `anyhow`; M1 migrates to `thiserror`)

M0 uses `anyhow::Error` throughout for ergonomics. **M1 migrates the
library crates (`spectra-core`, `spectra-rpc`, etc.) to a `thiserror`-based
`SpectraError` enum** so downstream embedders can `match` on variants
programmatically. `anyhow` will be retained only inside `spectra-cli`.

Migration schema documented in `docs/MIGRATION.md` for the
`spectra-allow.toml` companion file; the library error refactor is tracked
in [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) §2.

---

## 9. Extension points

### 9.1 New detection rules

Today (M0): rules live as `match` arms inside `diff::diff_idls`. **M1 adds
a `Rule` trait** so audit firms can register house rules without forking.
The contract is sketched in [`docs/RULE_ENGINE.md`](docs/RULE_ENGINE.md) §3:

```rust
pub trait Rule {
    const RULE_ID: &'static str;
    const SEVERITY: Severity;
    fn check(&self, old: &Idl, new: &Idl) -> Vec<Finding>;
}
```

A downstream crate (e.g. `audit-firm-house-rules`) depends on
`spectra-rules`, implements `Rule` for its custom checks, and registers
them via `Registry::register::<MyRule>()`. The CLI accepts
`--rules-from <crate>` to load additional rule crates.

### 9.2 New IDL schemas

`Idl::from_path` auto-detects schema by first-line heuristic +
distinguishing top-level keys. **M1 adds Anchor 2026 (Codama) and Shank
native-IDL parsers** behind the same `from_path` entry. Internal type
mapping is documented in [`docs/ANCHOR.md`](docs/ANCHOR.md).

### 9.3 New output formats

`report::render_<format>` functions are independent and additive. A
downstream user wanting JUnit XML or CycloneDX SBOM output can add a new
renderer without touching the diff engine.

### 9.4 New input sources (M1)

The CLI gains `--old-rpc <URL> --program <Pubkey>` flags to fetch the old
IDL directly from an upgradeable program's on-chain IDL account. Failure
modes (timeout, 429, 503, malformed response, truncated payload, missing
on-chain IDL) are codified in the M1 RPC adapter contract.

---

## 10. Versioning & compatibility policy

Spectra follows **Semantic Versioning 2.0.0** from `v0.1.0-m0` onward.

| Change class | Version bump | Examples |
|---|---|---|
| Adding a new `Finding` variant | minor (`0.x.0`) | New M1 rule lands |
| Adding a new CLI flag | minor | `--rules-from`, `--old-rpc` |
| Adding a new output format | minor | JUnit XML |
| Adding a new exit code (within reserved range) | minor | None planned |
| Changing the meaning of an existing exit code | **major** (`1.0.0`+) | Would never happen without major bump |
| Changing the SARIF rule ID for an existing finding | **major** | Stable contract for downstream filtering |
| Renaming a public Rust type or field | **major** | |
| Removing a CLI flag | **major** | |

The first stable release (`v1.0.0`) is targeted at the end of M4.
Pre-`v1.0.0` minor releases may include breaking changes if a critical
correctness bug requires it; such changes will be called out explicitly in
[`CHANGELOG.md`](CHANGELOG.md) and the release notes.

### Solana-specific compatibility

| Solana surface | Spectra's compatibility commitment |
|---|---|
| Anchor IDL legacy schema | Full support (M0) — matches `anchor-lang` 0.29 through current |
| Anchor IDL Codama schema | Full support (M1) — tracks `anchor-lang` 2026.x |
| Shank native IDL | Full support (M1) — tracks `solana-program/shank` releases |
| BPF Loader v3 | Implicit (no version-specific code paths) |
| BPF Loader v4 / SBPF activation | Adapter published within 4 weeks of mainnet activation (sustainability commitment in README §Future Plans) |
| Token-2022 TLV extensions | **Not supported** — not described in Anchor IDL; permanent non-goal |

---

## 11. Non-goals

Spectra deliberately does NOT attempt:

- **Formal verification.** Provably preserving invariants across an upgrade
  is audit-firm territory and a 2–6 week, $15k–$100k engagement. Spectra
  identifies *which* changes need scrutiny — not whether their effects are
  safe.
- **Build provenance.** "Does this `.so` match this source tree?" is
  `solana-verifiable-build` / `anchor verify`.
- **Runtime monitoring.** "Did something go wrong after the upgrade went
  live?" is Hypernative / Range.
- **Mainnet snapshot replay.** M2 uses bounded `litesvm` corpora, not
  historical mainnet state. A heavy-corpus replay system would require
  archive RPC + IO budgets Spectra explicitly does not take on.
- **`.so` bytecode parsing.** PDA-drift detection from BPF disassembly was
  considered and explicitly descoped — see [`docs/NON_GOALS.md`](docs/NON_GOALS.md)
  for the falsification rationale.
- **Token-2022 TLV extensions.** Out of scope; TLV is not described in
  Anchor IDL.
- **`.rodata` constant diff.** Out of scope.

---

## 12. Cross-reference index

For any topic listed here, this spec is the **entry point**; the linked doc
is the **canonical source**.

| Topic | Canonical doc |
|---|---|
| Per-milestone architecture detail | [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) |
| Threat model & adversary classes | [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) |
| Severity tiers + rule IDs + exit-code contract | [`docs/SEVERITY.md`](docs/SEVERITY.md) |
| 25-row Solana edge-case coverage matrix | [`docs/SOLANA_EDGE_CASES.md`](docs/SOLANA_EDGE_CASES.md) |
| False-positive prevention strategy (5-layer) | [`docs/FALSE_POSITIVES.md`](docs/FALSE_POSITIVES.md) |
| Rule engine internals + M1 `Rule` trait | [`docs/RULE_ENGINE.md`](docs/RULE_ENGINE.md) |
| Anchor-specific hazards (Borsh, discriminators, zero-copy, events) | [`docs/ANCHOR.md`](docs/ANCHOR.md) |
| `spectra-allow.toml` suppression schema | [`docs/MIGRATION.md`](docs/MIGRATION.md) |
| Drop-in CI integration templates (GHA, pre-commit, cargo-make) | [`docs/CI_INTEGRATION.md`](docs/CI_INTEGRATION.md) |
| Step-by-step testing reproduction guide | [`docs/TESTING.md`](docs/TESTING.md) |
| Real-world Drift Protocol benchmark | [`docs/BENCHMARK_DRIFT.md`](docs/BENCHMARK_DRIFT.md) |
| Competitive head-to-head benchmark | [`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md) |
| Q1-style technical paper (long-form) | [`docs/PAPER.md`](docs/PAPER.md) |
| Permanent non-goals | [`docs/NON_GOALS.md`](docs/NON_GOALS.md) |
| Roadmap with acceptance-test gates | [`docs/ROADMAP.md`](docs/ROADMAP.md) |
| Vulnerability reporting policy + SLAs | [`SECURITY.md`](SECURITY.md) |
| Release history | [`CHANGELOG.md`](CHANGELOG.md) |

---

## Appendix A — One-page summary for reviewers

If you have 60 seconds:

1. **What:** Static, deterministic CLI that diffs two Anchor IDL JSON files
   and flags upgrade hazards a Solana program upgrade would silently inflict
   on existing on-chain state.
2. **Why:** Solana's upgradeable BPF loader does not validate
   account-layout compatibility against pre-upgrade on-chain data, and does
   not check that new instruction discriminators do not collide with
   existing ones. Spectra is the only public tool that catches both
   statically.
3. **How:** Parse → diff → render → exit. 5 modules, ~1,070 lines of Rust,
   6 ms wall-clock on a 428 KB production IDL, zero false positives on
   identical input.
4. **Where:** [github.com/ayodyadsr/spectra](https://github.com/ayodyadsr/spectra),
   Apache-2.0, M0 PoC tag `v0.1.0-m0` on 2026-05-15.
5. **Status:** M0 shipped; M1–M4 pending grant. Engineering claims in this
   spec are reproducible from the current `main` per
   [`docs/TESTING.md`](docs/TESTING.md).
