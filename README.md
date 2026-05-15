# Spectra

> A safety-check for Solana program upgrades. Run it before you ship a new version — it tells you if old users' data and old apps will still work.
>
> **Status:** M0 PoC — works on Anchor IDL files today; bigger features come with grant milestones M1–M3.

[![CI](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml/badge.svg)](https://github.com/ayodyadsr/spectra/actions/workflows/ci.yml)
[![License: Apache 2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](LICENSE)

---

## Read this first (the simple version)

A **Solana program** is a smart contract that lives on the blockchain. Unlike Ethereum, almost every Solana program ships with an "upgrade button" the developer can press to deploy a new version on top of the old one.

When the button is pressed, two scary things can happen quietly:

1. **Old data on the blockchain gets read wrong.** Imagine a savings ledger where every row used to be `[name, balance, rate]`. The new version reorders it to `[name, rate, balance]`. The blockchain doesn't notice — it hands the old bytes to the new program, and now every user's balance is being read as if it were their interest rate. Money on paper, gone in practice. **No error message. No alert.**
2. **Old apps start calling the wrong function.** Each function has an 8-byte name tag (a "discriminator"). If two functions accidentally end up with the same tag, every call to function A might silently run function B instead. Again — no error, no alert.

Spectra is the tool that catches both of these **before** you press the upgrade button. It reads the old version and the new version, and in about 6 milliseconds it tells you: "Hey — this change quietly breaks existing users."

That's it. That's the tool.

---

## A quick analogy

Think of upgrading a Solana program like remodeling an apartment building **while the tenants are still living there with their old keys and floor plans.**

- If you move the living room and the bedroom around but keep the apartment number the same, the tenants come home and walk straight into a stranger's bathroom. No alarm rings — the door still opened.
- If you renumber the mailboxes but forget that two new units now share number `7B`, mail meant for one family ends up with another. No bounce-back.

Spectra is the inspector who walks both the old blueprint and the new blueprint side-by-side and says: *"You moved the bedroom in unit 4 but didn't tell anyone — the lock still fits, so the tenant won't know until they sleep in the wrong bed."*

That "lock still fits but the room is different" case is the **silent-corruption** problem. It is the single most dangerous thing Spectra finds, and it is invisible to every general-purpose diff tool. See [`docs/PAPER.md` §3.2](docs/PAPER.md) for the formal proof.

---

## What it actually does (one sentence)

Spectra takes two Anchor IDL JSON files — the old one and the new one — and produces a structured report of every change that could break existing on-chain accounts or existing client apps, with each finding labelled `BREAKING` or `warning` and a clean exit code so it can gate a CI build.

## Where it fits in the Solana security stack

There are already tools for some questions about upgrades. Spectra fills the missing question:

| Question | Tool today | Spectra? |
|---|---|---|
| Does the deployed bytecode match the public source code? | `solana-verify`, `anchor verify` | No — different layer |
| Are the new version's invariants provably preserved? | Audit-firm formal verification (~$15k–$100k, 2–6 weeks) | No — different layer |
| **Will the new version preserve old users' data and old clients' calls?** | **(no public tool before Spectra)** | **Yes — this is the gap** |
| Did something go wrong after deploy? | Hypernative, Range (runtime monitors) | No — too late by then |

Spectra is the fast, free, CI-time layer the ecosystem was missing.

---

## See it work in one command

```bash
cargo build --release

./target/release/spectra check \
  --old examples/lending_v1.json \
  --new examples/lending_v2.json \
  --format markdown
```

You will get **4 BREAKING + 2 warning** findings, the CLI exits with code `1`, and CI fails. The findings are:

- `withdraw` instruction **removed** — old apps calling it will fail with `InstructionFallbackNotFound`.
- `deposit.amount` **widened from `u64` to `u128`** — old callers send 8 bytes; new program reads 16 bytes; arg is corrupt.
- `Pool` account fields **reordered** — old accounts deserialize into wrong fields.
- `Pool` layout **changed but name kept** — the silent-corruption case from the analogy above.
- `Pool.fee_bps` field **added** — informational, but you must verify storage resize is handled.
- `withdrawFunds` instruction **added** — informational, presumably replacing `withdraw`.

### What the report actually looks like

Verbatim output of the command above (exit code `1`):

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

An asciinema recording of the same run is at [`demo.cast`](demo.cast):

```bash
asciinema play demo.cast
```

Run the same command twice on the **same** file and Spectra reports zero findings, exit `0`. This "no false positives on identical input" property is locked in by a test and a dedicated CI step — see the workflow file under `.github/workflows/ci.yml`.

---

## Real-world test: caught a real silent-corruption case on Drift

Spectra was run on a real production upgrade pair from [Drift Protocol v2](https://github.com/drift-labs/protocol-v2) — a Solana mainnet DeFi protocol with $300M+ TVL — from version `v2.155` (Jan 2026) to `v2.162` (Apr 2026).

- IDL size: 428 KB, 20,138 lines, 249 instructions, 27 accounts, 115 types.
- Changes between versions: 319 lines, scattered through the file.
- **Spectra completed in 6 ms** and surfaced 6 findings (2 BREAKING + 4 warning).
- **The interesting one:** Drift's `PerpMarket` account shrank `padding` from 23 bytes to 22 bytes and added a new `marketConfig: u8` in the byte that opened up. The `PerpMarket` discriminator did not change. **This is the silent-corruption pattern** — safe *if and only if* every on-chain `PerpMarket`'s old padding byte was zero, dangerous otherwise. Exactly the case a reviewer needs to be told about explicitly.
- **Zero false positives on a 428 KB production IDL.** Run Spectra with the same file as both `--old` and `--new`: exit code 0, no findings.

A human reviewer with a 393-line `diff -u` would need to do a 7-step Anchor + Borsh inference chain to reach the same conclusion. Spectra labels it `account_layout_changed_same_discriminator` directly. Full reproduction commands and the line-by-line walkthrough are in [`docs/BENCHMARK_DRIFT.md`](docs/BENCHMARK_DRIFT.md).

---

## How Spectra compares to other JSON / YAML diff tools

A fair question: "isn't there already a JSON diff tool that does this?" We tested the four most-installed candidates on the same Drift IDL pair. Best of 5 runs each, commodity laptop:

| Tool | Wall-clock on 428 KB Drift IDL | Exits 1 on BREAKING only? | False positive on whitespace reformat? | Detects silent corruption? |
|---|---:|---|---|---|
| `diff -u` (GNU 3.10) | 5 ms | no | **yes — 39,715 noise lines** | no |
| `jd` (Go) | 32 ms | no (exits 1 on any change) | no | no |
| `dyff` (Go) | 106 ms | no (exits 0 *always*) | no | no |
| `json-diff` (npm) | 9,217 ms | no (exits 1 on any change) | no | no |
| **Spectra** | **6 ms** | **yes** | **no** | **yes** |

Translation in plain words:

- Every other tool either blocks every harmless additive change (bar 1) or never blocks anything (bar 2) or trips on `prettier --write` (bar 3). Each one fails to be useful as a CI gate for at least one reason.
- Spectra is the only one that knows what an Anchor discriminator is, so it is the only one that can catch the silent-corruption case at all.
- Spectra is **~16× faster than the fastest semantic alternative** (`jd`) and within 1.5× of `diff -u` while doing dramatically more useful work.

Full methodology, raw measurements, and reproduction commands are in [`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md).

---

## All 11 things Spectra checks today (M0)

This is the full M0 detection surface for Anchor legacy-schema IDLs:

| Finding | Severity | What it means in plain words |
|---|---|---|
| `instruction_removed` | BREAKING | An old function was deleted. Old apps calling it will fail. |
| `instruction_args_changed` | BREAKING | A function's inputs changed shape. Old callers send the wrong bytes. |
| `instruction_added` | warning | A new function was added. Probably fine, just letting you know. |
| `account_removed` | BREAKING | An account type was deleted. Old account data can no longer be read. |
| `account_added` | warning | A new account type was added. Informational. |
| `account_field_removed` | BREAKING | A field was deleted from an account. Old accounts now misalign. |
| `account_field_added` | warning | A field was added. Check that you handled storage resize. |
| `account_field_reordered` | BREAKING | Field order changed. Old accounts now read wrong fields. |
| `account_field_type_changed` | BREAKING | A field changed type (e.g. `u64 → u128`). Old data is the wrong width. |
| `account_layout_changed_same_discriminator` | BREAKING | **Silent corruption.** Layout changed but the name (and thus the discriminator) did not. The runtime accepts old data and reads it into the new layout. |
| `discriminator_collision` | BREAKING | Two different names accidentally produce the same 8-byte SHA-256 tag. Calls get misrouted. |

The full rule roadmap reaches **23 rule types** across M0–M2; see [`docs/SEVERITY.md`](docs/SEVERITY.md).

---

## Quick start

```bash
# 1. Install Rust (one-time, skip if you already have it):
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# 2. Clone and build:
git clone https://github.com/ayodyadsr/spectra
cd spectra
cargo build --release

# 3. Run the demo:
make demo

# 4. Run the test suite:
cargo test --release
```

---

## CLI reference

```
spectra check --old <PATH> --new <PATH> [--report <PATH>] [--format json|markdown|sarif] [--quiet]
```

| Flag | What it does |
|---|---|
| `--old` | Baseline IDL (the version currently deployed) |
| `--new` | Candidate IDL (the version you are about to upgrade to) |
| `--report` | Also write the report to this file |
| `--format json` | Default. Machine-parseable JSON output. |
| `--format markdown` | Friendly Markdown table — good for PR comments. |
| `--format sarif` | SARIF 2.1.0 — uploads directly to GitHub's Security tab via `github/codeql-action/upload-sarif`. |
| `--quiet` | Skip stdout on clean runs. The exit code still tells you what happened. Useful in CI so noise only appears on real failures. |

Exit codes (the contract is locked by [`docs/SEVERITY.md`](docs/SEVERITY.md) §5):

| Code | Meaning |
|---|---|
| `0` | All clean — no breaking findings. |
| `1` | At least one BREAKING finding — block the merge. |
| `2` | Invocation error — bad path, bad JSON, unknown `--format`. |
| `3` | Spectra refuses to analyse — input is in a shape it cannot soundly diff (e.g. unsupported IDL schema). |

---

## What Spectra does NOT do (M0)

So you can judge it fairly:

- **No `.so` bytecode parsing.** That comes in M1 with grant funding.
- **No PDA-drift detection via BPF disassembly.** Research item, not promised here.
- **No mainnet replay.** M2 uses `litesvm` with a small hand-curated transaction corpus (≤50 transactions, ≤60s in CI). Not a mainnet snapshot.
- **No invariant DSL.** Spectra is a schema-regression gate, not a verifier. If you need to prove "the total never decreases," that's audit-firm territory.
- **No Token-2022 TLV extension detection.** IDL files don't describe TLV; separate detector pack.
- **No constant / `.rodata` diffing.**
- **No upgrade-authority transfer detection.** That action is invisible to static diff.
- **No native-program `#[repr(C)]` alignment-aware diff** until Shank-IDL parsing lands in M1.

See [`docs/NON_GOALS.md`](docs/NON_GOALS.md) for the full list.

---

## Documentation index

Every claim above is backed by one of these docs. Start with whichever question you're asking:

| If you want to know… | Read |
|---|---|
| The full technical story end-to-end (academic-style) | [`docs/PAPER.md`](docs/PAPER.md) |
| Whether `git diff` is good enough (no — here's the formal argument) | [`docs/VS_GIT_DIFF.md`](docs/VS_GIT_DIFF.md) |
| The reproducible synthetic before/after walkthrough | [`docs/BENCHMARK.md`](docs/BENCHMARK.md) |
| The real-world Drift IDL benchmark | [`docs/BENCHMARK_DRIFT.md`](docs/BENCHMARK_DRIFT.md) |
| Head-to-head against `jd`, `dyff`, `json-diff`, `diff -u` | [`docs/COMPETITIVE_BENCHMARK.md`](docs/COMPETITIVE_BENCHMARK.md) |
| The threat model and adversary classes | [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md) |
| What Spectra explicitly is **not** | [`docs/NON_GOALS.md`](docs/NON_GOALS.md) |
| Every rule ID + severity + the exit-code contract | [`docs/SEVERITY.md`](docs/SEVERITY.md) |
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

## Project layout

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

## Roadmap

The full plan submitted to the Solana Foundation:

| Milestone | What ships | Status |
|---|---|---|
| **M0** | Anchor legacy-schema IDL diff (11 rule types incl. silent-corruption + discriminator-collision); 8 tests green; SARIF output; real-world Drift validation; demo cast; public repo | **This PoC** |
| M1 | Anchor 2026 (Codama) schema parser + Shank native IDL parser + defined-type / events / errors diff + Loader-version adapter (~9 more rules) | Pending grant |
| M2 | `litesvm` pre-deployment harness driven by a hand-curated per-pilot transaction corpus (≤50 tx, ≤60s in CI). Not mainnet replay. (~3 more rules) | Pending grant |
| M3 | `spectra-allow.toml` suppression file + composite GitHub Action + PR comment integration | Pending grant |
| M4 | ≥1 confirmed pilot + 2 public walkthroughs against real upgradeable Anchor programs; mdBook docs; Solana Discord AMA | Pending grant |

Full milestone gating, acceptance tests, and the budget are in [`docs/ROADMAP.md`](docs/ROADMAP.md) and the grant proposal at [`solana.org/grants-funding`](https://solana.org/grants-funding) referencing the [Solana forum RFP](https://forum.solana.com/t/program-verification-tooling/1032).

---

## Demo recording (asciinema)

The cast is committed at [`demo.cast`](demo.cast). Regenerate locally with:

```bash
./scripts/record-demo.sh   # rewrites demo.cast headlessly
asciinema play demo.cast   # replay locally
```

Uploading to asciinema.org is intentionally a manual step — the repo stays the single source of truth.

---

## Why this PoC exists

The Spectra grant proposal honestly states the applicant has no prior Solana-specific OSS contributions. This PoC, with green CI from the very first commit, is the most direct way to mitigate that reviewer risk — proof of execution, not just words.

## Background

Spectra is built and maintained by **Ayodya** — 20+ years of penetration testing, formerly Red Team lead at Bank Mandiri. Daily work on (a) binary diffing for vulnerability discovery, (b) authoring static analysers and detection signatures, and (c) building CI-time security gates for production engineering teams. The skill set maps one-to-one onto Spectra's three core surfaces: ELF / discriminator diffing (binary analysis), invariant authoring (detection engineering), CI-pipeline integration (DevSecOps).

## License

Apache License 2.0. See [LICENSE](LICENSE).

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md). Issue triage SLA during the grant period: 7 days.

## Security

Please do not file public issues for exploitable security findings. Contact the maintainer privately (a `SECURITY.md` policy will be published after the grant decision).
