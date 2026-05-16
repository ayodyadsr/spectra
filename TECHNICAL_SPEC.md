# Spectra — Technical Specification

**Status:** M0 PoC shipped (`v0.2.0-m0`). M1–M4 pending grant.
**License:** Apache-2.0
**Scope:** Strictly-differential account-validation security-regression gate
for Solana program upgrades.

> This is the single-file engineering specification. It is the authoritative
> description of what the M0 engine *actually does* — every behaviour
> documented here is reproducible from `github.com/ayodyadsr/spectra` at the
> tagged release. Where a capability is M1+ it is labelled as such and is
> **not** an M0 claim. No real-world detection or performance number is
> asserted that has not been measured; the real-world validation benchmark is
> an explicit M1.5 deliverable (see §10), marked `[NO PUBLIC DATA AVAILABLE]`
> until then.

---

## 1. One-paragraph statement of the tool

Spectra parses two Anchor program **source trees** — a *baseline* (the last
released / on-chain-deployed version) and a *candidate* (the upgrade PR under
review) — extracts, for every `#[derive(Accounts)]` context, the set of
account-validation guards Anchor enforces per account slot, and emits a finding
**only** when the candidate drops, downgrades, or bypasses a guard the baseline
enforced. It is **not** an absolute scanner: a check that was *already missing
in the baseline* is, by construction, not a regression, and Spectra stays
silent on it. The output is JSON, Markdown, or SARIF 2.1.0, with a
severity-tiered process exit code (`0` clean / `1` BREAKING / `2` invocation
error) so a CI workflow can gate a merge without log-scraping.

---

## 2. Why strictly-differential is the entire design

An absolute scanner asks *"is there a missing owner check anywhere in this
code?"* It must tune heuristics to keep false positives inside a budget,
because any sufficiently large codebase has intentionally-unchecked accounts
(escape hatches, `AccountInfo` passed to a CPI, etc.).

Spectra only ever asks *"did this upgrade take away a guarantee the deployed
version already gave its users?"* This reframing has three consequences that
are the point of the project:

1. **Near-zero false positives by construction, not by tuning.** Identical
   input trees yield zero findings (verified by an integration test and a CI
   step). A finding can only mean a guard present in the baseline is absent
   from the candidate. There is no heuristic threshold to mis-calibrate.
2. **It answers a question stateless scanners structurally cannot.** An
   absolute scanner has no notion of "the previously deployed version", so a
   silently deleted guard does not register *as a regression*; it competes
   with every other absolute finding inside a heuristic FP budget.
3. **It is complementary, not competitive.** A check that was already missing
   in the baseline is exactly the absolute scanners' job (Sec3 X-Ray,
   Auditware Radar, l3x, Octane). Spectra is deliberately silent there. The
   two layers compose; neither replaces the other. See
   [`docs/STRIDE_GAP_ANALYSIS.md`](docs/STRIDE_GAP_ANALYSIS.md).

---

## 3. Threat model (summary)

Full version: [`docs/THREAT_MODEL.md`](docs/THREAT_MODEL.md). Condensed:

| | |
|---|---|
| **Asset protected** | The set of account-validation guarantees a deployed Solana program already gives its users. |
| **Adversary / failure source** | A well-intentioned upgrade PR that silently removes a guard (refactor, type downgrade, deleted constraint) with **no compiler error** — Anchor enforces guards at runtime, not at compile time. Also: a malicious insider PR doing the same deliberately. |
| **Trust boundary** | Spectra trusts its two input source trees. It does **not** verify the baseline tree corresponds to the on-chain bytecode (that is `solana-verifiable-build` / `anchor verify` territory — out of scope, §9). |
| **Soundness goal** | Every emitted finding corresponds to a guard that is in the baseline slot and absent from the candidate slot. No finding fires on an unchanged context. |
| **Completeness boundary** | Within Anchor `#[derive(Accounts)]` declarative guards at M0. Manual native-program checks (`is_signer`, `owner ==`, `key ==`) are M1. |

---

## 4. The guard model

Spectra reduces each account slot in a `#[derive(Accounts)]` struct to a set of
typed guards. The guard enum (engine source: `spectra-core/src/accounts.rs`):

| `Guard` variant | Anchor source it is extracted from | Security property it represents |
|---|---|---|
| `Signer` | `Signer<'info>` slot type, or `#[account(signer)]` | Account must sign the transaction. |
| `Typed(T)` | `Account<'info, T>`, `AccountLoader<'info, T>`, `InterfaceAccount<'info, T>` | Anchor enforces owner + 8-byte discriminator → type-cosplay protection. |
| `Owner(expr)` | `#[account(owner = expr)]` | Account owner pinned to a specific program. |
| `Address(expr)` | `#[account(address = expr)]` | Account key pinned to a specific address. |
| `HasOne(field)` | `#[account(has_one = field)]` | Relational-integrity: `slot.field == named_account.key()`. |
| `Seeds` | `#[account(seeds = [...], bump)]` | Account is a PDA derived from the given seeds. |
| `Constraint(expr)` | `#[account(constraint = expr)]` | Arbitrary user predicate must hold. |
| `ProgramId(T)` | `Program<'info, T>` | CPI target program id pinned. |

Slots also carry `unchecked: bool` (true for `UncheckedAccount` /
`AccountInfo` / `/// CHECK:`-annotated) and the raw type string `ty`.

### 4.1 `#[account(...)]` parsing

Anchor's `#[account(...)]` attribute legitimately mixes bare keywords
(`mut`, `signer`) with `key = expr` pairs and `key = [..]` lists. A naïve
`syn::Meta`-list parse rejects the whole attribute when it hits the bare `mut`
keyword (it is not a valid `Meta::Path` in that position), which would
silently drop *every* constraint in that attribute. The M0 parser therefore
walks the attribute's `proc_macro2::TokenTree` stream directly, splitting on
top-level commas and classifying each item as a bare keyword or a `key = rest`
pair. This is the single most correctness-critical function in the engine and
is covered by an integration test (`has_one` + custom `constraint` on a `mut`
slot must both be detected).

---

## 5. The differential algorithm

Engine source: `spectra-core/src/regression.rs`. Pseudocode of the actual
implementation:

```
for (name, base_ctx) in baseline.contexts:
    if name in candidate.contexts:
        diff_context(base_ctx, candidate.contexts[name])
    # context removed wholesale = interface change, NOT a silent
    # weakening of a still-callable instruction → intentionally out of scope

diff_context(base, cand):
    for base_slot in base.slots:
        match cand.slots by slot name:
            absent  -> if base_slot had any guard:
                           emit ValidatedAccountSlotRemoved
            present -> diff_slot(base_slot, cand_slot)
    for cand_slot not in base.slots:
        if cand_slot is Unchecked and has no guards:
            emit UnvalidatedAccountIntroduced (warning)

diff_slot(base, cand):
    for g in base.guards:
        if g in cand.guards: continue          # unchanged → silent
        match g:
            Signer       -> SignerCheckRemoved
            Typed(t)     -> if cand has NO Typed/Owner/Address/ProgramId pin:
                                TypeCosplayProtectionRemoved
            Owner|Address-> if cand has NO Owner/Address/Typed pin:
                                OwnerCheckRemoved
            HasOne(f)    -> HasOneConstraintRemoved
            Constraint(e)-> CustomConstraintRemoved
            Seeds        -> PdaDerivationRemoved
            ProgramId    -> if cand has NO ProgramId/Address pin:
                                CpiTargetUnpinned
```

### 5.1 Downgrade-vs-equivalent-pin logic

The `Typed` / `Owner` / `Address` / `ProgramId` arms deliberately do **not**
fire when an *equivalent* pin remains. Examples:

- `Account<'info, Mint>` → `#[account(owner = token_program)] UncheckedAccount`
  — **not** a regression (owner still pinned a different way). No finding.
- `Account<'info, Mint>` → `UncheckedAccount` with no pin —
  `type_cosplay_protection_removed` BREAKING.
- `Program<'info, Token>` → `#[account(address = token::ID)] AccountInfo` —
  **not** a regression. `Program<'info, Token>` → `UncheckedAccount` —
  `cpi_target_unpinned` BREAKING.

This is what keeps the false-positive rate near-zero: a re-expressed-but-still-
enforced guard is not a removed guarantee.

---

## 6. Finding catalogue (M0 — exhaustive)

Nine finding kinds. JSON/SARIF `kind` is the snake_case rule ID. Canonical
table also in [`docs/SEVERITY.md`](docs/SEVERITY.md).

| Rule ID (`kind`) | Severity | Meaning |
|---|---|---|
| `signer_check_removed` | BREAKING | Baseline required this slot to sign; candidate does not. |
| `type_cosplay_protection_removed` | BREAKING | Typed Anchor wrapper (owner+discriminator) downgraded to `UncheckedAccount`/`AccountInfo` with no equivalent pin. |
| `owner_check_removed` | BREAKING | `owner =` / `address =` pin dropped with no equivalent pin. |
| `has_one_constraint_removed` | BREAKING | `has_one =` relational-integrity check dropped. |
| `custom_constraint_removed` | BREAKING | `constraint =` predicate dropped. |
| `pda_derivation_removed` | BREAKING | `seeds`/`bump` PDA derivation dropped (arbitrary account now accepted). |
| `cpi_target_unpinned` | BREAKING | CPI target program id no longer pinned. |
| `validated_account_slot_removed` | BREAKING | A slot that carried ≥1 guard in the baseline context was removed while the context still exists. |
| `unvalidated_account_introduced` | warning | Candidate adds a new `UncheckedAccount`/`AccountInfo` slot absent from the baseline. New attack surface to review (not a regression of an existing guarantee). |

---

## 7. CLI contract

```
spectra check --baseline <DIR> --candidate <DIR>
              [--report <PATH>] [--format json|markdown|md|sarif] [--quiet]
```

| Exit code | Meaning |
|---|---|
| `0` | No BREAKING finding. Warnings may be present. Clean / mergeable. |
| `1` | ≥1 BREAKING finding. The upgrade removes a guarantee — block the merge. |
| `2` | Invocation error (bad path, unparseable tree, unknown `--format`). |

There is no exit code 3. `--quiet` suppresses stdout on clean runs only; the
exit code still signals status. `--report PATH` additionally writes the
rendered output to a file. The pipeline is a pure function: no network, no
environment reads, no filesystem writes outside `--report`; identical inputs
produce byte-identical output and the same ordered finding list.

---

## 8. Verified M0 behaviour (measured, not asserted)

Reproducible from a clean clone with `make demo` / `cargo test --release`:

- **6 integration tests green** (`spectra-core/tests/integration_test.rs`),
  including the strictly-differential no-false-positive property: an unchanged
  context inside an otherwise-changed program produces **zero** findings.
- **Synthetic baseline → candidate fixture pair**
  (`examples/vault_baseline` → `examples/vault_candidate`) produces exactly
  **6 BREAKING + 1 warning**, exit `1`. Verbatim Markdown output:

  ```
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
  | warning  | unvalidated_account_introduced | `EmergencyDrain::anyone` new UncheckedAccount/AccountInfo slot (new attack surface) |
  ```

- **Identical input** (`--baseline X --candidate X`) → zero findings, exit
  `0` (test + CI step).
- **SARIF 2.1.0** on the fixture pair → exactly 7 `results` (6 `error` +
  1 `warning`), 9 `rules` in the driver catalogue, `properties.breaking_count
  = 6`, driver name `Spectra`. Uploadable via
  `github/codeql-action/upload-sarif@v3`.
- **Bad `--format`** → exit `2`.
- `cargo fmt -- --check` and `cargo clippy --all-targets -- -D warnings`
  clean; green CI on every push.

Anything not in this section is not an M0 claim.

---

## 9. Explicit non-goals (M0)

Full version: [`docs/NON_GOALS.md`](docs/NON_GOALS.md).

- **Not an absolute scanner.** Already-missing checks are out of scope by
  construction — Sec3 X-Ray / Auditware Radar / l3x / Octane territory.
- **Not a build-provenance tool.** Does not verify the baseline tree matches
  on-chain bytecode — `solana-verifiable-build` / `anchor verify` territory.
- **Not a formal verifier.** Does not prove functional invariants.
- **Not a runtime monitor.** Pre-merge static only — STRIDE / SIRN /
  Hypernative / Range territory.
- **Not a native-program analyser at M0.** Manual `is_signer` / `owner ==` /
  `key ==` checks are an M1 roadmap item, documented, not silently
  mis-handled.
- **Not an IDL differ.** Spectra reads Rust source, not IDL JSON. It makes no
  claim about discriminator drift, Borsh layout, or account-field reorder —
  those are a different problem and explicitly out of scope.

---

## 10. Roadmap acceptance gates (engineering view)

Full version: [`docs/ROADMAP.md`](docs/ROADMAP.md). Each milestone is
unlocked by CI-runnable assertions, not activity.

| Milestone | Key acceptance gate (engineering) |
|---|---|
| **M0 (shipped)** | 9 finding kinds; 6 tests; fixture → 6 BREAKING + 1 warning; identical-input → exit 0; SARIF 7 results; fmt + clippy `-D warnings` clean. |
| **M1** | Native (non-Anchor) guard extraction (`is_signer` / `owner ==` / `key ==`) with golden fixtures; defined-constraint resolution; `thiserror`-based `SpectraError` with `cargo-semver-checks` confirming no public-API break; **M1.5 real-world validation** — reproducible benchmark against a real public Anchor program's deployed-vs-upgrade source pair, report committed. Until M1.5 the real-world detection/perf numbers are `[NO PUBLIC DATA AVAILABLE]`. |
| **M2** | `spectra harness` replays a hand-curated ≤50-tx corpus through `litesvm`; guard-regression reporter surfaces `AccountNotInitialized` / signer-missing per tx; <60 s in free-tier CI. |
| **M3** | `spectra-allow.toml` suppression schema (mandatory `rationale`/`expires`/`upgrade_pr`; expired suppression fails CI); Marketplace Action; single-PR-comment integration; mdBook page. |
| **M4** | ≥1 confirmed pilot integration; 2 public integration walkthroughs; mdBook complete; Discord AMA; `v1.0.0` tag + semver commitment. |

---

## 11. Evidence index

| Claim | Reproducible from |
|---|---|
| 6 integration tests green; fmt + clippy `-D warnings` clean | `github.com/ayodyadsr/spectra/actions` — every push green |
| Fixture → 6 BREAKING + 1 warning, exit 1 | `spectra-core/tests/integration_test.rs` + `examples/` |
| Strictly-differential no-FP property | Same test file (unchanged-context-in-changed-program test) |
| SARIF 7 results / 9 rules / driver `Spectra` | CI SARIF smoke step |
| Identical input → exit 0 | CI identical-tree step |
| Position vs STRIDE / SIRN / absolute scanners | [`docs/STRIDE_GAP_ANALYSIS.md`](docs/STRIDE_GAP_ANALYSIS.md) |
| Real-world validation | `[NO PUBLIC DATA AVAILABLE]` at M0 — explicit M1.5 deliverable |
