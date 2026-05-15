# Roadmap with Acceptance Tests

Every milestone is **gated by acceptance tests**, not by activity. A milestone is complete only when the listed acceptance tests pass in CI on a tagged commit and the linked deliverables are publicly inspectable.

The roadmap is intentionally conservative. Milestones are sized for a single engineer working ~3-4 days a week against a 16-week clock, with Loader v4 and Anchor-2026 contingency buffer.

---

## M0 — Anchor legacy IDL diff (shipped 2026-05-14)

**Deliverables (shipped):**

- `spectra-core` Rust crate + `spectra` CLI binary.
- Anchor legacy IDL parser.
- 11 finding kinds: 9 baseline + R-ACC-SILENT-CORRUPT + R-DISC-COLL.
- JSON + markdown report formats.
- 5/5 tests green (Anchor-vector discriminator, synthetic-regression integration, identical-IDL clean-report, no-false-positive collision, account-discriminator name distinction).
- CI workflow: fmt + clippy `-D warnings` + build + test + demo-exit-1 + identical-IDL exit-0 + JSON demo + artifact upload.
- Public repo, Apache-2.0-licensed, asciinema cast committed.

**Acceptance tests (all PASS as of commit `e5aad06`, CI run `25851086148`):**

1. ✅ `cargo test --release` -> 5/5.
2. ✅ `spectra check --old examples/lending_v1.json --new examples/lending_v2.json` -> exit 1, 4 BREAKING + 2 warning findings including R-ACC-SILENT-CORRUPT.
3. ✅ `spectra check --old examples/lending_v1.json --new examples/lending_v1.json` -> exit 0, zero findings.
4. ✅ Anchor-known-vector unit test: `instruction_discriminator("initialize") == 0xafaf6d1f0d989bed`.

---

## M1 — Schema breadth + richer rule coverage

**Deliverables:**

- M1.a — Anchor 2026 (Codama) IDL parser.
- M1.b — Shank-generated native IDL parser.
- M1.c — `defined_type` / events / errors comparators (R-EVENT-*, R-ERROR-CODE-CHANGED, R-DEFINED-TYPE-CHANGED).
- M1.d — Enum variant insertion / removal comparator (R-ENUM-VAR-INSERT / R-ENUM-VAR-REM).
- M1.e — Anchor 0.30+ explicit discriminator override conflict detector (R-DISC-OVERRIDE-CONFLICT).
- M1.f — Loader v4 program-data-account adapter (contingency; budget line included).

**Acceptance tests:**

1. Anchor 2026 fixture pair produces an expected finding set documented in `tests/fixtures/anchor_2026/`.
2. Shank native fixture pair produces an expected finding set.
3. Reordering an enum variant in a fixture produces R-ENUM-VAR-INSERT for every downstream serialized site.
4. An IDL with `#[instruction(discriminator = "non_algorithmic")]` that does not match the algorithmic discriminator produces R-DISC-OVERRIDE-CONFLICT.
5. A v3-format program-data-account fixture and a v4-format fixture both produce identical findings on a structurally-identical upgrade.

---

## M2 — Bounded pre-deployment execution via `litesvm`

**Deliverables:**

- M2.a — `litesvm` runner that loads `old.so` and `new.so`, seeds a deterministic account set, and replays a ≤ 50-tx hand-curated corpus per pilot.
- M2.b — Replay differential comparator (R-REPLAY-DESERIALIZE-FAIL, R-REPLAY-LOG-DIVERGENCE, R-REPLAY-CPI-FAIL).

**Bounds (enforced):**

- ≤ 50 transactions per pilot.
- ≤ 60 s end-to-end on a free-tier `ubuntu-latest` runner.
- No mainnet snapshot. No archive-RPC fetch during CI runtime. Archive-RPC budget covers one-time corpus authoring at pilot onboarding only.

**Acceptance tests:**

1. Synthetic pilot corpus (provided in `tests/fixtures/litesvm/`) replays cleanly against `old.so` and produces R-REPLAY-DESERIALIZE-FAIL against `new.so` for the upgrade variants under test.
2. Wall-clock measurement asserts < 60 s on the GitHub-hosted `ubuntu-latest` runner.

---

## M3 — Suppression file + composite Action

**Deliverables:**

- M3.a — `spectra-allow.toml` parser + suppression-application logic, with the schema documented in [FALSE_POSITIVES.md](FALSE_POSITIVES.md) and [MIGRATION.md](MIGRATION.md).
- M3.b — `spectra-action` composite GitHub Action: build IDL for base + head, run `spectra check`, post markdown report as PR comment, upload JSON artifact.

**Acceptance tests:**

1. A `spectra-allow.toml` entry with empty `rationale` is **rejected** at parse time.
2. A suppression entry whose `expires` date is past produces R-SUPPRESS-EXPIRED in the report.
3. A wildcard `target = "*"` entry is rejected at parse time.
4. The Action posts an idempotent PR comment (re-running on the same SHA does not duplicate).

---

## M4 — Pilots + walkthroughs + measurement

**Deliverables:**

- M4.a — ≥ 1 confirmed pilot deploying Spectra against a real upgradable Anchor program in CI.
- M4.b — 2 public walkthroughs against real upgradable programs (write-up + commits + CI run links).
- M4.c — Per-pilot FP-rate table classifying every finding as: true regression caught, true regression already known, intentional change suppressed, or false positive.
- M4.d — mdBook docs published; Solana Discord AMA.

**Acceptance tests:**

1. Pilot integration linked from this repo (CI workflow visible).
2. Two walkthrough write-ups published, each citing specific Spectra findings against the target programs' upgrade history.
3. FP-rate table published; each row carries a `commit` + `pr` link for verification.

---

## What the roadmap deliberately does **not** promise

- No mainnet snapshot replay.
- No formal verification.
- No protocol-specific invariant DSL.
- No Token-2022 TLV layout detection.
- No PDA seed-derivation drift detection (Future Expansion only).
- No runtime monitoring.

Each is also listed in [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md) so the omissions are surface-visible.

---

## Cross-references

- Severity rules: [SEVERITY.md](SEVERITY.md).
- Threat model: [THREAT_MODEL.md](THREAT_MODEL.md).
- Corpus design: [CORPUS.md](CORPUS.md).
- Replay design: [REPLAY.md](REPLAY.md).
