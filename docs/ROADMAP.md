# Roadmap with Acceptance Tests

Every milestone is **gated by acceptance tests**, not by activity. A milestone
is complete only when the listed tests pass in CI on a tagged commit and the
linked deliverables are publicly inspectable. Sized for a single engineer
against a 16-week clock with a native-parser contingency buffer.

Budget: $28,115 total — M1 $6,300 / M2 $7,875 / M3 $6,300 / M4 $5,400 +
contingencies ($1,800 native-parser buffer, $320 archive RPC, $120 hosting).

---

## M0 — Strictly-differential Anchor account-validation gate (shipped, not billable)

**Deliverables (shipped):**

- `spectra-core` Rust crate + `spectra` CLI (`spectra check --baseline DIR
  --candidate DIR`).
- `accounts.rs` guard extractor + `regression.rs` strictly-differential
  differ + `report.rs` JSON / Markdown / SARIF 2.1.0 renderers.
- 9 finding kinds (8 BREAKING + `unvalidated_account_introduced` warning).
- Exit-code contract `0` clean / `1` BREAKING / `2` invocation error (no
  exit 3).
- Synthetic baseline → candidate fixture (`examples/vault_baseline` →
  `examples/vault_candidate`).
- Apache-2.0; green CI on every push.

**Acceptance tests (all PASS at the tagged release):**

1. `cargo test --release` → 6 integration tests pass.
2. `spectra check --baseline examples/vault_baseline --candidate
   examples/vault_candidate` → exit `1`, exactly 6 BREAKING + 1 warning.
3. `spectra check --baseline examples/vault_baseline --candidate
   examples/vault_baseline` → exit `0`, zero findings.
4. SARIF on the fixture pair → exactly 7 `results` (6 `error` + 1 `warning`),
   9 driver `rules`, driver name `Spectra`.
5. Bad `--format` → exit `2`.
6. `cargo fmt -- --check` + `cargo clippy --all-targets -- -D warnings`
   clean.

---

## M1 — Native programs + real-world validation — 4 weeks — $6,300

**Deliverables:**

- M1.a — Native (non-Anchor) guard extractor: detects manual `is_signer` /
  `owner ==` / `key ==` checks in instruction bodies via `syn`-AST analysis.
- M1.b — Defined-constraint resolution: `constraint =` referencing helper
  fns/consts resolved instead of opaque-stringified.
- M1.c — Cross-version slot-rename heuristic (renamed-but-unchanged-guard
  slot must not produce a false negative).
- M1.d — Opt-in whole-context-removal strict mode (default behaviour
  unchanged).
- M1.e — `thiserror`-based `SpectraError` enum (library crates off `anyhow`).
- **M1.5 — Real-world validation:** reproducible benchmark against a real
  public Anchor program's deployed-vs-upgrade source pair; report committed.

**Acceptance tests:**

1. Native-program golden fixtures: manual `is_signer` / `owner ==` removal
   produces the corresponding finding kind.
2. A `constraint = helper_fn(...)` whose body changes is resolved, not
   opaque-stringified, on a golden fixture.
3. A renamed-but-unchanged-guard slot produces **no** false negative on a
   golden fixture.
4. Whole-context-removal flag emits a finding **only** under the opt-in flag.
5. `cargo-semver-checks` confirms no breaking change to the v0.2 public API.
6. M1.5 benchmark report committed against a real public Anchor program pair
   (until then real-world numbers are `[NO PUBLIC DATA AVAILABLE]`).
7. Tagged release `v0.3.0-m1` with the CI run link.

---

## M2 — `litesvm` pre-deployment harness — 5 weeks — $7,875

**Deliverables:**

- M2.a — `spectra harness` subcommand loading a hand-curated ≤50-tx corpus
  into `litesvm` and running it against the candidate build.
- M2.b — Guard-regression replay reporter: `AccountNotInitialized` /
  `AccountDidNotDeserialize` / signer-missing surfaced per tx.

**Bounds (enforced):** ≤50 tx per pilot; ≤60 s end-to-end on a free-tier
`ubuntu-latest` runner; no mainnet snapshot, no archive-RPC fetch during CI
runtime (archive-RPC budget covers one-time corpus authoring only).

**Acceptance tests:**

1. Synthetic pilot corpus replays cleanly against the baseline build and
   surfaces a guard-regression on the candidate.
2. Wall-clock asserts <60 s on GitHub-hosted `ubuntu-latest`.
3. Worked-example public walkthrough committed (corpus + report).
4. Tagged release `v0.4.0-m2`.

---

## M3 — Suppression file + Action + PR comment — 4 weeks — $6,300

**Deliverables:**

- M3.a — `spectra-allow.toml` parser + suppression-application logic; schema
  in [FALSE_POSITIVES.md](FALSE_POSITIVES.md).
- M3.b — `spectra-action` composite Action published to Marketplace;
  single-PR-comment integration; mdBook getting-started page on GitHub Pages.

**Acceptance tests:**

1. A `spectra-allow.toml` entry with empty `rationale` is **rejected** at
   parse time.
2. A suppression whose `expires` date is past **fails CI**.
3. A wildcard `target = "*"` entry is rejected at parse time.
4. The Action posts an idempotent PR comment (re-run on the same SHA does not
   duplicate).
5. Spectra's own CI uses the published Marketplace Action as its smoke gate.
6. Tagged release `v0.5.0-m3`.

---

## M4 — Pilots + walkthroughs + measurement — 3 weeks — $5,400

**Deliverables:**

- M4.a — ≥1 confirmed pilot deploying Spectra against a real upgradable
  Anchor program in CI (prioritising a SIRN-roster firm or a protocol team
  they evaluate).
- M4.b — 2 public walkthroughs against real upgradable programs.
- M4.c — Per-pilot FP-rate table classifying every finding: true regression
  caught / true regression already known / intentional relaxation suppressed
  / false positive.
- M4.d — mdBook docs complete; Solana Discord AMA.

**Acceptance tests:**

1. Pilot integration linked from this repo (CI workflow visible).
2. Two walkthrough write-ups published, each citing specific Spectra findings.
3. FP-rate table published; each row carries a `commit` + `pr` link.
4. Tagged release `v1.0.0` — semver stability commitment begins.

---

## What the roadmap deliberately does **not** promise

- No mainnet snapshot replay.
- No formal verification.
- No protocol-specific invariant DSL.
- No absolute scanning (already-missing checks stay out of scope by
  construction — absolute-scanner territory).
- No IDL diffing / discriminator / Borsh-layout analysis (different problem,
  out of scope entirely).
- No runtime monitoring.

Each is also in [NON_GOALS.md](NON_GOALS.md) so the omissions are
surface-visible.

---

## Cross-references

- Severity + exit-code contract: [SEVERITY.md](SEVERITY.md)
- Threat model: [THREAT_MODEL.md](THREAT_MODEL.md)
- Architecture across M0–M3: [ARCHITECTURE.md](ARCHITECTURE.md)
- Authoritative engine spec: [`../TECHNICAL_SPEC.md`](../TECHNICAL_SPEC.md)
