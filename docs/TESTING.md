# Testing Guide

Step-by-step guide for reproducing every Spectra M0 acceptance test from a
clean clone. This file is the canonical testing reference for grant
reviewers and is referenced by deliverable `0c` in the Spectra roadmap.

## Prerequisites

- Rust stable (1.78+). Spectra pins the toolchain in [`rust-toolchain.toml`](../rust-toolchain.toml) so a `rustup` install is sufficient — no nightly required.
- Python 3.9+ (only required for the SARIF JSON-parse smoke step and for the optional Python wrapper).
- Optional: Docker 24+ for the containerised reproduction path.
- ~5 minutes of wall-clock time on a commodity laptop.

## 1. Clean-clone reproduction (host build)

```bash
git clone https://github.com/ayodyadsr/spectra
cd spectra

# Static checks
cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings

# Build
cargo build --release --workspace

# Run the full test suite (8 tests)
cargo test --release --workspace
```

Expected outcome:

- `cargo fmt --check` exits 0 (no diff).
- `cargo clippy -D warnings` exits 0 (no lints).
- `cargo build --release` succeeds in under 60 seconds with cargo cache cold.
- `cargo test --release` reports **8 passed, 0 failed**.

## 2. End-to-end CLI gates

The CI workflow at [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)
runs these gates on every push. They can be replayed locally:

| Gate | Command | Expected exit | What it verifies |
|---|---|---|---|
| Synthetic regression — markdown | `./target/release/spectra check --old examples/lending_v1.json --new examples/lending_v2.json --format markdown` | `1` | 4 BREAKING + 2 warning findings render. |
| Synthetic regression — JSON | same with `--format json` | `1` | JSON shape parses, `breaking_count >= 1`. |
| Synthetic regression — SARIF | same with `--format sarif --report out.sarif` | `1` | SARIF 2.1.0 parses, `runs[0].results.length >= 6`. |
| Identical-IDL invariant | `./target/release/spectra check --old examples/lending_v1.json --new examples/lending_v1.json --format markdown` | `0` | Zero false positives. |
| Invocation error | `./target/release/spectra check --old examples/lending_v1.json --new examples/lending_v2.json --format definitely-not-a-format` | `2` | Exit-code contract: bad invocation ≠ regression. |
| Quiet no-output-on-clean | `./target/release/spectra check --old examples/lending_v1.json --new examples/lending_v1.json --format markdown --quiet` | `0` (and zero stdout) | Clean CI runs produce no noise. |

A one-line reproduction of all six is also available via `make demo` (see [`Makefile`](../Makefile)).

## 3. Docker reproduction (containerised path)

For reviewers who want a hermetic build with no Rust toolchain on the host:

```bash
docker build -t spectra:test .

# Synthetic regression — must exit 1
docker run --rm spectra:test check \
  --old /examples/lending_v1.json \
  --new /examples/lending_v2.json \
  --format markdown

# Identical IDL — must exit 0
docker run --rm spectra:test check \
  --old /examples/lending_v1.json \
  --new /examples/lending_v1.json \
  --format markdown
```

The Docker image is also built and smoke-tested on every CI push in the
`docker` job — reviewers can verify by browsing [Spectra CI runs](https://github.com/ayodyadsr/spectra/actions).

## 4. Real-world IDL reproduction

To reproduce the Drift Protocol v2.155 → v2.162 benchmark (428 KB
production IDL, 6 ms wall-clock, 6 findings including one real
silent-corruption case), follow the step-by-step commands in
[`docs/BENCHMARK_DRIFT.md`](BENCHMARK_DRIFT.md).

## 5. Competitive benchmark reproduction

Head-to-head against `diff -u`, `jd`, `dyff`, `json-diff` on the same
Drift IDL pair — commands and exact tool versions in
[`docs/COMPETITIVE_BENCHMARK.md`](COMPETITIVE_BENCHMARK.md).

## 6. Per-test mapping

The 8 tests gated by `cargo test --release`:

| Test name | Type | Location | Verifies |
|---|---|---|---|
| `anchor_instruction_discriminator_matches_known_vector` | unit | `spectra-core/src/discriminator.rs` | `sha256("global:initialize")[..8] = afaf6d1f0d989bed` |
| `account_discriminator_changes_with_name` | unit | `spectra-core/src/discriminator.rs` | Different names yield different discriminators. |
| `synthetic_regression_demo_detects_breaking_changes` | integration | `spectra-core/tests/` | 4 BREAKING + 2 warning on the bundled fixture. |
| `identical_idls_produce_clean_report` | integration | `spectra-core/tests/` | Zero findings on `--old X --new X`. |
| `no_false_collision_on_synthetic_fixture` | integration | `spectra-core/tests/` | Non-colliding names are not flagged. |
| `sarif_output_is_valid_for_synthetic_fixture` | integration | `spectra-core/tests/` | SARIF JSON parses, required keys present. |
| `sarif_clean_report_has_zero_results` | integration | `spectra-core/tests/` | Clean run emits valid SARIF with empty `results`. |
| `markdown_renderer_still_produces_silent_corruption_row` | integration | `spectra-core/tests/` | Silent-corruption finding renders correctly. |

## 7. How to add a new test (M1 onwards)

The M1 deliverable ships a `docs/TESTING_M1.md` extension that documents
golden-file fixture additions. The minimum bar for any new rule:

1. Add a fixture pair under `examples/regressions/<rule>/{v1,v2}.json`.
2. Add a golden-file integration test that calls `diff_idls` and asserts
   the expected `Finding` variant + count.
3. Add a new row in `docs/SOLANA_EDGE_CASES.md` describing the edge case
   and its detection layer.
4. Add the rule + severity to `docs/SEVERITY.md` (canonical rule ID +
   exit-code contract).
5. Run `cargo test --release` — the new test must pass and no existing
   test may regress.

## 8. Reporting test failures

If any of the above commands fails on a clean clone of `main`, please
open a GitHub issue using the **Bug report** template at
[github.com/ayodyadsr/spectra/issues/new](https://github.com/ayodyadsr/spectra/issues/new)
and include:

- Your OS + `rustc --version` + `cargo --version`.
- The commit hash of `main` you cloned (`git rev-parse HEAD`).
- The exact command run and its full stderr.

Security-related test failures (e.g. Spectra fails to flag a known
silent-corruption case in a public fixture) should be reported privately
per [`SECURITY.md`](../SECURITY.md).
