# Severity Classification Rules

Spectra assigns a fixed severity to every finding kind. Severities are deterministic — they are a property of the finding kind, not of context or heuristics. The CLI's exit code is derived from the maximum severity present in the report.

This document is the canonical rule table. Any change to a rule's severity is a breaking change to Spectra's contract and must be released in a major version bump.

---

## 1. Severity levels

| Level | Meaning | Exit code contribution |
|-------|---------|------------------------|
| BREAKING | The upgrade is incompatible with at least one existing on-chain state or client invocation pattern. Default action: block merge. | `1` |
| warning | The upgrade is forward-compatible at the schema level but introduces a downstream consideration (storage resize, indexer update, etc.). Default action: surface in PR. | `0` |
| informational | A change exists but cannot break compatibility (e.g. doc-only IDL field). | `0` |

Severities are not configurable in M0. M3 introduces `spectra-allow.toml` for **per-finding suppression with rationale**, not for global severity downgrade.

---

## 2. Rule table (M0)

Every rule has a stable ID. Rule IDs are referenced from finding output and from suppression entries.

| ID | Finding kind | Severity | Why |
|----|--------------|----------|-----|
| R-INS-REM | `instruction_removed` | BREAKING | Old clients hit `InstructionFallbackNotFound`. |
| R-INS-ARG | `instruction_args_changed` | BREAKING | Borsh arg layout mismatch -> corrupt deserialize. |
| R-INS-ADD | `instruction_added` | warning | Informational. |
| R-ACC-REM | `account_removed` | BREAKING | Old account discriminator no longer accepted. |
| R-ACC-ADD | `account_added` | warning | Informational. |
| R-ACC-FIELD-REM | `account_field_removed` | BREAKING | Borsh layout shifts. |
| R-ACC-FIELD-ADD | `account_field_added` | warning | Informational; protocol must verify `realloc` + rent. |
| R-ACC-FIELD-REORDER | `account_field_reordered` | BREAKING | Borsh layout reorder corrupts existing accounts. |
| R-ACC-FIELD-TYPE | `account_field_type_changed` | BREAKING | Width/encoding change corrupts existing accounts. |
| R-ACC-SILENT-CORRUPT | `account_layout_changed_same_discriminator` | BREAKING | The discriminator-stable / layout-changed case — runtime accepts old bytes into new layout. |
| R-DISC-COLL | `discriminator_collision` | BREAKING | Two IDL names share the truncated 8-byte SHA-256. |

---

## 3. Rule table (M1 — pending grant)

These rules are designed and reserved; they activate when the M1 schema parsers ship.

| ID | Finding kind | Severity | Trigger |
|----|--------------|----------|---------|
| R-EVENT-REM | `event_removed` | BREAKING | Indexers and subscribers break. |
| R-EVENT-FIELD-REORDER | `event_field_reordered` | BREAKING | Off-chain event consumers decode wrong fields. |
| R-EVENT-FIELD-TYPE | `event_field_type_changed` | BREAKING | Off-chain decode mismatch. |
| R-ERROR-CODE-CHANGED | `error_code_changed` | BREAKING | Client error-handling branches break. |
| R-ENUM-VAR-INSERT | `enum_variant_inserted` | BREAKING | Borsh enum tag is positional; insertion mid-list renumbers all subsequent variants. |
| R-ENUM-VAR-REM | `enum_variant_removed` | BREAKING | As above + the removed tag becomes an unknown variant. |
| R-DEFINED-TYPE-CHANGED | `defined_type_layout_changed` | BREAKING | Shared struct used across instructions / accounts. |
| R-DISC-OVERRIDE-CONFLICT | `discriminator_override_conflicts_algorithm` | BREAKING | Anchor 0.30+ explicit discriminator override does not match the algorithmic one — silent foot-gun. |
| R-IDL-SCHEMA-UNKNOWN | `idl_schema_unsupported` | refuse (exit 3) | IDL schema version is not legacy / Anchor 2026 / Shank. |

---

## 4. Rule table (M2 — pending grant)

M2 introduces `litesvm`-based bounded execution. New rule classes:

| ID | Finding kind | Severity | Trigger |
|----|--------------|----------|---------|
| R-REPLAY-DESERIALIZE-FAIL | `corpus_tx_deserialize_failure_after_upgrade` | BREAKING | A canned pre-upgrade-valid transaction fails to deserialize against the post-upgrade program. |
| R-REPLAY-LOG-DIVERGENCE | `corpus_tx_log_diverged` | warning | The transaction succeeds but emits a different log shape. |
| R-REPLAY-CPI-FAIL | `corpus_tx_cpi_signature_mismatch` | BREAKING | A CPI signature the program previously emitted is no longer valid. |

M2 rules are **bounded** — they apply only to the ≤50-tx per-pilot transaction corpus described in [CORPUS.md](CORPUS.md). They are explicitly **not** "we replay mainnet."

---

## 5. Exit-code contract

| Exit code | Meaning |
|-----------|---------|
| `0` | Analysis completed; no BREAKING findings. |
| `1` | Analysis completed; at least one BREAKING finding. |
| `2` | Invocation error (bad path, JSON parse failure, unknown CLI flag). |
| `3` | Refuse to analyse — the input is recognised as something Spectra cannot soundly diff (e.g. unsupported IDL schema version). |

Exit `0` on identical IDLs is a tested invariant — see the `no_false_collision_on_synthetic_fixture` integration test and the CI step "Identical-IDL exit-0 check."

A clean report at exit `0` is **never** silent failure: anything Spectra cannot understand goes to exit `2` or `3`.

---

## 6. Stability promise

Rule IDs are stable. The mapping `rule_id -> severity` is stable within a major version. New rule IDs may be added in minor versions. Severity downgrades require a major version bump and a documented migration note.
