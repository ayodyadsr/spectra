# Testing Guide

Step-by-step guide for reproducing every Spectra M0 acceptance gate from a
clean clone. This is the canonical testing reference for grant reviewers.

## Prerequisites

- Rust stable. The toolchain is pinned in
  [`rust-toolchain.toml`](../rust-toolchain.toml) (`channel = "stable"`,
  components `rustfmt` + `clippy`) so a `rustup` install is sufficient — no
  nightly required.
- Python 3.9+ (only for the SARIF JSON-parse smoke step and the optional
  Python wrapper).
- Optional: Docker 24+ for the containerised path.
- ~5 minutes on a commodity laptop.

## 1. Clean-clone reproduction (host build)

```bash
git clone https://github.com/ayodyadsr/spectra
cd spectra

cargo fmt --all -- --check
cargo clippy --all-targets -- -D warnings
cargo build --release --workspace
cargo test --release --workspace
```

Expected:

- `cargo fmt --check` exits 0 (no diff).
- `cargo clippy -D warnings` exits 0 (no lints).
- `cargo build --release` succeeds.
- `cargo test --release` reports **6 passed, 0 failed** (integration suite).

## 2. End-to-end CLI gates

The CI workflow at [`.github/workflows/ci.yml`](../.github/workflows/ci.yml)
runs these on every push; replay locally:

| Gate | Command | Exit | Verifies |
|---|---|---|---|
| Regression — Markdown | `./target/release/spectra check --baseline examples/vault_baseline --candidate examples/vault_candidate --format markdown` | `1` | 6 BREAKING + 1 warning render |
| Regression — JSON | same with `--format json` | `1` | JSON parses, `breaking_count == 6` |
| Regression — SARIF | same with `--format sarif --report out.sarif` | `1` | SARIF 2.1.0 parses, `runs[0].results.length == 7` |
| Strictly-differential invariant | `./target/release/spectra check --baseline examples/vault_baseline --candidate examples/vault_baseline` | `0` | Zero findings — no FP on identical input |
| Invocation error | same with `--format definitely-not-a-format` | `2` | Bad invocation ≠ regression |
| Quiet, no output on clean | identical-tree run with `--quiet` | `0` (zero stdout) | Clean CI runs produce no noise |

One-line reproduction of all gates: `make demo` (see
[`Makefile`](../Makefile)).

## 3. Docker reproduction (containerised)

```bash
docker build -t spectra:test .

# Regression — must exit 1
docker run --rm spectra:test check \
  --baseline /examples/vault_baseline \
  --candidate /examples/vault_candidate \
  --format markdown

# Identical tree — must exit 0
docker run --rm spectra:test check \
  --baseline /examples/vault_baseline \
  --candidate /examples/vault_baseline \
  --format markdown
```

The image is also built and smoke-tested in the `docker` CI job on every
push.

## 4. Real-world reproduction

A reproducible benchmark against a real public Anchor program's
deployed-vs-upgrade **source** pair is an explicit **M1.5** deliverable. No
real-world detection or wall-clock number is asserted at M0:
`[NO PUBLIC DATA AVAILABLE]` until M1.5 ships the committed report and the
exact reproduction commands.

## 5. Per-test mapping

The 6 integration tests gated by `cargo test --release`
(`spectra-core/tests/integration_test.rs`):

| Test fn | Verifies |
|---|---|
| `synthetic_upgrade_detects_account_validation_regressions` | The `vault_baseline` → `vault_candidate` fixture yields exactly 6 BREAKING + 1 warning, exit-equivalent state. |
| `identical_program_produces_clean_report` | `--baseline X --candidate X` → zero findings, clean. |
| `unchanged_context_in_changed_program_produces_no_false_positive` | The strictly-differential property: an unchanged context inside an otherwise-changed program produces **no** finding. |
| `sarif_output_is_valid` | SARIF 2.1.0 parses; driver name `Spectra`; 9 rules; 7 results on the fixture. |
| `sarif_clean_report_has_zero_results` | A clean run emits valid SARIF with an empty `results` array. |
| `markdown_renderer_calls_out_signer_regression` | The Markdown renderer surfaces the `signer_check_removed` row correctly. |

## 6. How to add a new test (M1 onward)

Minimum bar for any new finding kind:

1. Add a baseline/candidate fixture pair under
   `examples/regressions/<rule>/{baseline,candidate}/`.
2. Add a golden integration test that calls `diff_programs` and asserts the
   expected `Finding` variant + count.
3. Add the rule + severity to [SEVERITY.md](SEVERITY.md) (canonical rule ID).
4. Add the boundary statement to [NON_GOALS.md](NON_GOALS.md) if it narrows a
   documented non-goal.
5. `cargo test --release` — the new test passes and no existing test
   regresses.

## 7. Reporting test failures

If any command above fails on a clean clone of `main`, open a GitHub issue
using the **Bug report** template and include: OS + `rustc --version` +
`cargo --version`; `git rev-parse HEAD`; the exact command and full stderr.
Security-relevant failures (Spectra fails to flag a known guard regression in
a public fixture) go privately per [`SECURITY.md`](../SECURITY.md).
