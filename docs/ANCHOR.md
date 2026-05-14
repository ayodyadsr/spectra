# Anchor-Specific Compatibility Analysis

Anchor is Spectra's primary first-class target because it is the dominant programming model for upgradable Solana programs. This document collects the Anchor-specific compatibility hazards Spectra reasons about — and the ones it explicitly does not.

---

## 1. Discriminators

Anchor identifies instructions and accounts by an 8-byte SHA-256 prefix:

- Instruction discriminator: `sha256("global:" + ix_name)[..8]`.
- Account discriminator: `sha256("account:" + struct_name)[..8]`.

Spectra computes these in `spectra-core::discriminator` with no Solana SDK dependency. The Anchor canonical vector `sha256("global:initialize")[..8] = afaf6d1f0d989bed` is a unit-tested invariant.

**Hazards Spectra detects:**

- Renaming an instruction or account -> discriminator drift (caught indirectly via `instruction_removed` + `instruction_added` or `account_removed` + `account_added`).
- Two IDL names sharing a discriminator -> R-DISC-COLL.
- Account layout change with discriminator unchanged -> R-ACC-SILENT-CORRUPT.

**Hazards Spectra plans to detect (M1):**

- Anchor 0.30+ explicit `#[instruction(discriminator = "...")]` overrides that do not match the algorithmic discriminator -> R-DISC-OVERRIDE-CONFLICT.

---

## 2. Borsh serialization properties

Anchor uses Borsh, which is:

- **Positional** — fields are serialized in declaration order.
- **No padding** — no alignment, no implicit padding bytes.
- **Length-prefixed for variable-length types** — strings, vectors, optional.

The consequence: **any** account field reorder is a layout-breaking change. **Any** struct-variant insertion before existing variants is a tag-renumbering change. Spectra encodes these as deterministic rules, not heuristics.

**Specific hazards covered:**

- Account field reorder -> R-ACC-FIELD-REORDER.
- Account field type change -> R-ACC-FIELD-TYPE.
- Account field add -> R-ACC-FIELD-ADD (warning + storage-resize note).
- Account field remove -> R-ACC-FIELD-REM.
- Instruction arg add / remove / widen -> R-INS-ARG.

**Specific hazards M1 covers:**

- Enum variant insertion in the middle of a list -> R-ENUM-VAR-INSERT (Borsh tag is the variant's positional index).
- Enum variant removal -> R-ENUM-VAR-REM.
- Shared `defined_type` layout change -> R-DEFINED-TYPE-CHANGED.

---

## 3. Zero-copy accounts

Accounts marked `#[account(zero_copy)]` with `#[repr(C)]` use C struct layout, not Borsh. They are aligned and may have implicit padding.

**Today (M0):**

- Spectra does **not** model alignment or padding. IDL does not surface it explicitly enough.

**M1 plan:**

- Detect the `zero_copy` attribute on accounts in the Anchor 2026 schema.
- Emit a coarser `R-ZERO-COPY-LAYOUT-MAYBE-CHANGED` warning when any zero-copy account's field set or types change at all. The warning is intentionally over-broad until proper alignment-aware diff lands as Future Expansion.

**Future Expansion (not promised):**

- Padding-aware diff for zero-copy accounts requires modelling `#[repr(C)]` rules per architecture. This is research-grade and is explicitly listed in [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md) §2 as not in the M0–M3 scope.

---

## 4. Events

Anchor events are `emit!`-able structs serialized to base64 program logs. Off-chain indexers, subgraphs, and bots subscribe to them.

**M1 plan:**

- R-EVENT-REM — removing an event breaks all subscribers.
- R-EVENT-FIELD-REORDER — off-chain decoders decode the wrong fields.
- R-EVENT-FIELD-TYPE — width/encoding mismatch.

Events are not in M0 because Anchor legacy IDL emits them under a different shape than Anchor 2026; M1's schema dispatcher resolves the normalisation.

---

## 5. Errors

Anchor errors are numbered (`#[error_code]`). Clients pattern-match by code.

**M1 plan:**

- R-ERROR-CODE-CHANGED — renumbering breaks client error-handling branches.

---

## 6. Schema version drift (Anchor legacy vs Anchor 2026 / Codama)

Anchor's 2026 release introduces a Codama-aligned IDL shape that is **not** byte-compatible with the legacy schema. Spectra's reaction:

- M0 supports legacy IDL only.
- M0 refuses (exit 3) any IDL that fails legacy parsing.
- M1 introduces a schema dispatcher that supports both shapes and normalises them into a single internal IDL before the rule engine runs.

The refusal path is essential: a legacy-only Spectra silently parsing partial fields from an Anchor 2026 IDL would produce arbitrary findings. Refuse-to-analyse is the documented contract.

---

## 7. `init_if_needed`, `init`, `realloc`, and storage resize

`account_field_added` is a `warning`, not BREAKING, because Anchor's `realloc` and `init_if_needed` machinery makes additive field changes recoverable **if the maintainer handles storage resize**. The warning text reminds the maintainer to verify:

- `realloc` is called or the account is `init_if_needed` re-initialized.
- Rent for the new size is funded.
- Existing data is preserved (or intentionally reset, declared via [MIGRATION.md](MIGRATION.md)).

If the maintainer does not handle resize, the new field's bytes are simply absent and reads default to zero — a class of silent corruption Spectra cannot detect from IDL alone. The proposal does not over-claim coverage of this case.

---

## 8. Upgrade authority changes

Anchor programs are upgraded via the BPF loader's upgrade-authority signer. Changing or revoking the authority is an **off-chain operational action** — Spectra has no visibility into it. This is explicit non-coverage; see [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).

---

## 9. Cross-references

- Severity rules referenced above: [SEVERITY.md](SEVERITY.md).
- Edge-case matrix: [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).
- Migration declarations for intentional Anchor breaks: [MIGRATION.md](MIGRATION.md).
- Architecture: [ARCHITECTURE.md](ARCHITECTURE.md).
