# Architecture

Spectra is a deterministic structural-compatibility analysis pipeline. This document describes the architecture as it exists today (M0) and as it is planned through M3.

Every component is labelled **deterministic** or **bounded heuristic**. Spectra has no network access, no clock, and no randomness; reproducibility is a property of the design, not a tested side-effect.

---

## 1. M0 architecture (shipped)

```
+----------------+      +---------------------+      +-----------------------+
| Anchor legacy  | ---> | IDL parser          | ---> | Diff engine            |
| IDL JSON       |      | (serde, deterministic)|     | - instruction diff     |
| (--old, --new) |      +---------------------+      | - account diff         |
+----------------+                                   | - discriminator hashing|
                                                     | - silent-corruption    |
                                                     | - discriminator coll.  |
                                                     +-----------+-----------+
                                                                 |
                                                                 v
                                                     +-----------+-----------+
                                                     | Report                 |
                                                     | (JSON or markdown)     |
                                                     | + exit code            |
                                                     +-----------------------+
```

**Components:**

- `spectra-core::idl` — Anchor legacy IDL parser. Pure structural deserialization via `serde_json`. **Deterministic.**
- `spectra-core::discriminator` — `sha256("global:<name>")[..8]` and `sha256("account:<name>")[..8]`. Standalone, no Solana SDK dependency. Unit-tested against the canonical `initialize` vector. **Deterministic.**
- `spectra-core::diff` — Computes the set of findings from `(old_idl, new_idl)`. Each finding kind is produced by a dedicated comparator. **Deterministic.**
- `spectra-core::report` — Serialises findings to JSON or markdown. **Deterministic.**
- `spectra-core::main` — CLI entrypoint (`spectra check ...`). **Deterministic.**

**Properties:**

- No network calls.
- No system clock reads.
- No filesystem writes outside the optional `--report` path.
- For a fixed `(old_idl_bytes, new_idl_bytes)` pair, the output bytes are identical across runs and across hosts.

---

## 2. M1 architecture (pending grant)

M1 adds schema breadth (Anchor 2026 / Codama, Shank native IDL) and richer comparators (defined types, events, errors). The pipeline shape is unchanged; the parser and the rule set expand.

```
        +-------------------+
        | Anchor legacy     |
        +---------+---------+
                  |
        +---------v---------+
+-------+ Schema dispatcher  +-------+
| Anchor 2026 (Codama)      | Shank |
+---------+--------+--------+-------+
          |        |        |
          v        v        v
        +---+    +---+    +---+
        | normalised internal IDL |
        +-----+-------------------+
              |
              v
        +-----+-----+
        | Diff engine (rule registry)
        | - all M0 rules
        | - event / error / defined-type
        | - enum variant insert/remove
        | - explicit-disc override conflict
        +-----+-----+
              |
              v
           Report
```

**New components:**

- `spectra-core::idl::dispatcher` — picks parser by schema marker. Returns the normalised internal IDL.
- `spectra-core::idl::anchor_2026` — Codama-aligned parser.
- `spectra-core::idl::shank` — native-program IDL parser.
- `spectra-core::diff::rules` — refactored as a `Rule` trait + `RuleRegistry` (see [RULE_ENGINE.md](RULE_ENGINE.md)).
- `spectra-core::loader` — Loader v3 vs v4 program-data-account format adapter (gated by Loader v4 mainnet activation; budget includes contingency).

---

## 3. M2 architecture (pending grant)

M2 introduces **bounded execution** via `litesvm`. This is the only component in Spectra that runs code; it is fenced, deterministic-per-input, and bounded.

```
+-----------------+      +-----------------+
| --old program   |      | --new program   |
| .so + IDL       |      | .so + IDL       |
+--------+--------+      +--------+--------+
         |                        |
         v                        v
+--------+--------+      +--------+--------+
| litesvm runner  |      | litesvm runner  |
| - load program  |      | - load program  |
| - seed accounts |      | - seed accounts |
| - replay corpus |      | - replay corpus |
| (<= 50 tx)      |      | (<= 50 tx)      |
+--------+--------+      +--------+--------+
         |                        |
         +-----------+------------+
                     v
            +--------+--------+
            | Replay diff     |
            | - deserialize OK?
            | - log shape     |
            | - CPI signatures|
            +--------+--------+
                     v
                  Findings
```

**Bounds (explicitly enforced):**

- ≤ 50 transactions per pilot.
- ≤ 60 seconds end-to-end in a GitHub Actions free-tier runner.
- No mainnet snapshot fetch. No archive RPC during CI runtime. The corpus is hand-curated; see [CORPUS.md](CORPUS.md).
- An Archive-RPC budget line exists, but **only** for one-time corpus authoring at pilot onboarding — not for CI runtime.

These bounds are why M2 is feasible on a free-tier runner and why it is not "mainnet replay."

---

## 4. M3 architecture (pending grant)

M3 adds the **suppression file** and the **packaged GitHub Action**.

- `spectra-allow.toml` — declarative suppression with required `rationale`, `expires`, and `upgrade_pr` fields. See [FALSE_POSITIVES.md](FALSE_POSITIVES.md) and [MIGRATION.md](MIGRATION.md).
- `spectra-action` — composite GitHub Action that runs `spectra check`, posts the markdown report as a PR comment, and uploads the JSON report as a build artifact.

---

## 5. Determinism guarantees

| Component | Property | How enforced |
|-----------|----------|--------------|
| IDL parser | Same bytes -> same internal IDL | `serde_json` deterministic; field order preserved |
| Diff engine | Same IDL pair -> same finding set | Pure functions, sorted iteration |
| Discriminator | Same name -> same 8 bytes | SHA-256 (`sha2` crate) on `"global:" + name` / `"account:" + name` |
| Report | Same findings -> same bytes | Stable sort, fixed serialisation |
| CI | Identical IDL -> exit 0 | CI step `Identical-IDL exit-0 check` |

No component reads from the network, the system clock, or `/dev/urandom`.

---

## 6. What is explicitly NOT in the architecture

- No LLM, no ML, no statistical heuristic, no fuzz-derived signature.
- No symbolic execution.
- No SMT solver.
- No formal-verification claim.
- No runtime monitoring.

If a future Spectra component requires any of the above, it will be introduced behind an explicit subcommand and labelled **bounded heuristic** in this document.
