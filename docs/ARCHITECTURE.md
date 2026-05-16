# Architecture

Spectra is a deterministic, strictly-differential account-validation
regression pipeline. This document describes the architecture as it exists
today (M0) and as it is planned through M3.

Every component is **deterministic**. Spectra has no network access, no clock,
and no randomness; reproducibility is a property of the design, not a tested
side-effect.

---

## 1. M0 architecture (shipped)

```
+-------------------+     +----------------------+     +-------------------------+
| baseline source   | --> | accounts.rs          | --> | regression.rs           |
| tree (--baseline) |     | walk tree (walkdir)  |     | strictly-differential   |
+-------------------+     | parse Rust (syn)     |     | guard-set diff:         |
                          | per-slot Guard set   |     |  baseline − candidate   |
+-------------------+     | from #[derive(       |     |  (downgrade-vs-pin      |
| candidate source  | --> |   Accounts)]         |     |   logic)                |
| tree (--candidate)|     +----------------------+     +-----------+-------------+
+-------------------+                                              |
                                                                   v
                                                       +-----------+-------------+
                                                       | report.rs               |
                                                       | JSON / Markdown /       |
                                                       | SARIF 2.1.0  + exit code|
                                                       +-------------------------+
```

**Components** (`spectra-core/src/`):

- `accounts.rs` — walks each source tree with `walkdir`, parses every `.rs`
  file with `syn` (recursing into `mod` blocks), and reduces each account slot
  in every `#[derive(Accounts)]` struct to a typed `Guard` set. The
  `#[account(...)]` parser is a tolerant `proc_macro2::TokenTree` walk that
  handles bare keywords (`mut`, `signer`) mixed with `key = expr` pairs.
  Files that do not parse as Rust are skipped, not fatal. **Deterministic.**
- `regression.rs` — the strictly-differential differ. For every context in
  *both* trees, emits a typed `Finding` only when a baseline guard is absent
  from the candidate slot, applying the downgrade-vs-equivalent-pin rule.
  **Deterministic.**
- `report.rs` — renders Markdown and SARIF 2.1.0 (JSON is direct `serde_json`
  in the CLI). Per-rule SARIF catalogue. **Deterministic.**
- `main.rs` — `clap` CLI (`spectra check ...`); maps the report to exit
  `0`/`1`/`2`. **Deterministic.**

**Properties:** no network; no clock; no randomness; no filesystem writes
outside the optional `--report` path. For a fixed `(baseline, candidate)`
pair the output bytes and the ordered finding list are identical across runs
and hosts.

---

## 2. M1 architecture (pending grant)

M1 adds the **native-program guard path** without changing the pipeline
shape: a second extractor that recognises manual `is_signer` / `owner ==` /
`key ==` checks in instruction bodies, plus defined-constraint resolution
(`constraint =` referencing helper fns/consts resolved instead of opaque-
stringified). The differ and renderers are unchanged.

```
        +-------------------+
        | source tree       |
        +---------+---------+
                  |
        +---------v---------+
        | extractor dispatch |
        +----+---------+-----+
             |         |
   #[derive(Accounts)] |  manual native checks (syn-AST flow)
             |         |
             v         v
        +----+---------+----+
        | normalised Guard sets |
        +----------+------------+
                   |
                   v
        +----------+------------+
        | regression.rs (unchanged differ) |
        +----------+------------+
                   v
                Report
```

Also in M1: `thiserror`-based `SpectraError` (library crates move off
`anyhow`) so audit firms can `match` on error variants; `cargo-semver-checks`
gates public-API stability. **M1.5** delivers the real-world validation
benchmark against a public Anchor program's deployed-vs-upgrade source pair.

---

## 3. M2 architecture (pending grant)

M2 introduces **bounded execution** via `litesvm` — the only component that
runs code; fenced, deterministic-per-input, and bounded.

```
+-----------------+      +-----------------+
| baseline build  |      | candidate build |
+--------+--------+      +--------+--------+
         |                        |
         v                        v
   litesvm runner           litesvm runner
   - load program           - load program
   - seed accounts          - seed accounts
   - replay corpus (<=50 tx)- replay corpus (<=50 tx)
         |                        |
         +-----------+------------+
                     v
            guard-regression replay diff
            - AccountNotInitialized?
            - signer-missing?
            - per-tx delta
                     v
                  Findings
```

**Bounds (enforced):** ≤50 transactions per pilot; ≤60 s end-to-end in a
free-tier GitHub Actions runner; no mainnet snapshot fetch and no archive RPC
during CI runtime — the corpus is hand-curated. The Archive-RPC budget line
funds **one-time** corpus authoring at pilot onboarding, not CI runtime.
These bounds are why M2 is feasible on a free-tier runner and why it is
**not** "mainnet replay."

---

## 4. M3 architecture (pending grant)

- `spectra-allow.toml` — declarative per-finding suppression with mandatory
  `rationale`, `expires`, `upgrade_pr`. An expired suppression fails CI — no
  silent waivers. See [FALSE_POSITIVES.md](FALSE_POSITIVES.md).
- `spectra-action` — composite GitHub Action published to the Marketplace;
  runs `spectra check`, uploads SARIF via
  `github/codeql-action/upload-sarif@v3`, and maintains a single
  updated-in-place PR comment. Spectra's own CI dogfoods the published Action
  as its smoke gate.

---

## 5. Determinism guarantees

| Component | Property | How enforced |
|---|---|---|
| `accounts.rs` parser | Same source bytes → same Guard sets | `syn` deterministic; source-order slot iteration |
| `regression.rs` differ | Same pair → same ordered finding list | Pure functions, context-then-slot order |
| `report.rs` | Same findings → same bytes | Fixed serialisation, stable order |
| CLI | Identical trees → exit 0 | CI step "identical-tree exit-0" + integration test |

No component reads the network, the system clock, or `/dev/urandom`.

---

## 6. Explicitly NOT in the architecture

No LLM / ML / statistical heuristic; no IDL parsing (Spectra reads Rust
source, not IDL JSON); no symbolic execution; no SMT solver; no
formal-verification claim; no runtime monitoring; no absolute-scan pass. If a
future component ever requires a heuristic, it ships behind an explicit
subcommand and is labelled as such here.
