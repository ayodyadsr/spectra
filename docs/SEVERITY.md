# Severity Classification & Exit-Code Contract

Spectra assigns a fixed severity to every finding kind. Severity is a property
of the finding kind, not of context or heuristics. The CLI's exit code is
derived from whether any BREAKING finding is present.

This document is the canonical rule table. Any change to a rule's severity is
a breaking change to Spectra's contract and ships only in a major version
bump. Rule IDs here match the `kind` field in JSON/SARIF and the SARIF rule
catalogue emitted by `spectra-core/src/report.rs`.

---

## 1. Severity levels

| Level | Meaning | Exit-code contribution |
|---|---|---|
| BREAKING | The upgrade removes, downgrades, or bypasses an account-validation guard the deployed baseline enforced. Default action: **block the merge.** | drives exit `1` |
| warning | New attack surface introduced (a brand-new unvalidated slot), but no existing guarantee was removed. Default action: surface in the PR. | exit unaffected (`0`) |

Severity is **not configurable** at M0. M3 introduces `spectra-allow.toml`
for per-finding suppression with a mandatory rationale — not for global
severity downgrade.

---

## 2. Rule table (M0 — exhaustive, 9 kinds)

Every rule maps 1:1 to a `Finding` variant in
`spectra-core/src/regression.rs`. A finding fires **only** when the guard is
present in the baseline slot and absent from the candidate slot (and, for the
pinned-kinds, no equivalent pin remains — see §3).

| Rule ID (`kind`) | Severity | Fires when |
|---|---|---|
| `signer_check_removed` | BREAKING | Baseline required this slot to sign; the candidate no longer does. Canonical missing-signer-check bug introduced on upgrade. |
| `type_cosplay_protection_removed` | BREAKING | A typed Anchor wrapper (`Account<T>` / `AccountLoader<T>` / `InterfaceAccount<T>`) enforcing owner + 8-byte discriminator was downgraded to `UncheckedAccount` / `AccountInfo` with no equivalent pin remaining. |
| `owner_check_removed` | BREAKING | `#[account(owner = …)]` / `#[account(address = …)]` pin dropped with no equivalent pin remaining. |
| `has_one_constraint_removed` | BREAKING | `#[account(has_one = …)]` relational-integrity check dropped. |
| `custom_constraint_removed` | BREAKING | `#[account(constraint = …)]` predicate dropped. |
| `pda_derivation_removed` | BREAKING | `#[account(seeds = […], bump)]` PDA derivation dropped — an arbitrary account is now accepted in this slot. |
| `cpi_target_unpinned` | BREAKING | A pinned CPI target program id (`Program<'info, T>` / `address` pin) downgraded to an unvalidated account — arbitrary-program-invocation hazard. |
| `validated_account_slot_removed` | BREAKING | A slot carrying ≥1 guard in the baseline context was removed while the context itself still exists (the instruction no longer takes — and therefore no longer checks — it). |
| `unvalidated_account_introduced` | warning | The candidate adds a brand-new `UncheckedAccount` / `AccountInfo` slot absent from the baseline. New attack surface to review — **not** a regression of an existing guarantee. |

There is no informational tier and no M1/M2 reserved-rule table — new finding
kinds, if added, ship in a minor version with a doc entry and CI evidence,
not as pre-declared placeholders.

---

## 3. Downgrade-vs-equivalent-pin rule

For `type_cosplay_protection_removed`, `owner_check_removed`, and
`cpi_target_unpinned`, the finding fires **only if no equivalent pin remains**
in the candidate slot:

- `Account<'info, Mint>` → `#[account(owner = token::ID)] UncheckedAccount`
  — **no finding** (owner still pinned, re-expressed).
- `Account<'info, Mint>` → `UncheckedAccount` (no pin) —
  `type_cosplay_protection_removed`.
- `Program<'info, Token>` → `#[account(address = token::ID)] AccountInfo` —
  **no finding**. `Program<'info, Token>` → `UncheckedAccount` —
  `cpi_target_unpinned`.

This is the core mechanism that keeps the false-positive rate near-zero: a
re-expressed-but-still-enforced guard is not a removed guarantee. See
[FALSE_POSITIVES.md](FALSE_POSITIVES.md).

---

## 4. Exit-code contract

| Exit code | Meaning |
|---|---|
| `0` | Analysis completed; **no BREAKING finding**. Warnings may be present. Clean / mergeable. |
| `1` | Analysis completed; **≥1 BREAKING finding**. The upgrade removes a guarantee — block the merge. |
| `2` | Invocation error: bad path, unparseable input tree, or unknown `--format`. |

**There is no exit code 3.** A clean report at exit `0` is never a silent
failure: anything Spectra cannot process surfaces as exit `2`. Exit `0` on
identical input trees (`--baseline X --candidate X`) is a tested invariant —
see the strictly-differential no-false-positive integration test and the CI
"identical-tree exit-0" step.

---

## 5. Stability promise

Rule IDs are stable. The mapping `rule_id → severity` is stable within a major
version. New rule IDs may be added in minor versions. Any severity downgrade
requires a major version bump and a documented migration note. The exit-code
contract (`0`/`1`/`2`, no `3`) is part of the public interface and is
covered by `cargo-semver-checks` from M1.
