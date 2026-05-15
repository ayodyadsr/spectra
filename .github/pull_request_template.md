<!--
Thanks for sending a PR to Spectra! Please fill in the sections below.
For security-sensitive changes please coordinate privately first per
SECURITY.md — do not open a public PR that demonstrates an unfixed
detection-evasion path.
-->

## Summary

<!-- 1–3 bullets describing what this PR does and why. -->

## Type of change

- [ ] Bug fix (no behaviour change for clean inputs)
- [ ] New detection rule
- [ ] New output format / renderer
- [ ] Documentation only
- [ ] CI / build / packaging
- [ ] Refactor (no behaviour change)
- [ ] Other (please describe)

## Behavioural impact

<!--
If this PR changes Spectra's findings or exit codes on any existing
fixture, list the before/after below. If it does not, write "None".
-->

| Fixture | Old finding count / exit | New finding count / exit |
|---|---|---|
| `examples/lending_v1.json` → `examples/lending_v2.json` |  |  |
| Drift v2.155 → v2.162 (if applicable) |  |  |

## Test plan

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --release --workspace` (all 8 tests still pass)
- [ ] New rule? Added a golden-file integration test under `spectra-core/tests/`
- [ ] New rule? Added rule ID + severity row in [`docs/SEVERITY.md`](../docs/SEVERITY.md)
- [ ] New rule? Added edge-case row in [`docs/SOLANA_EDGE_CASES.md`](../docs/SOLANA_EDGE_CASES.md)
- [ ] Docs updated where relevant (README, TESTING, CHANGELOG `[Unreleased]`)

## Related issues

<!-- Use `Closes #123` to auto-close on merge, or `Refs #123` to link. -->

## Reviewer notes

<!--
Anything you want reviewers to focus on — tricky invariants, areas you
are unsure about, false-positive risk on real-world IDLs, etc.
-->
