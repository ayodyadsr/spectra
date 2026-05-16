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
| `examples/vault_baseline` → `examples/vault_candidate` |  |  |
| Identical-tree invariant (`--baseline X --candidate X`) |  |  |

## Test plan

- [ ] `cargo fmt --all -- --check`
- [ ] `cargo clippy --all-targets -- -D warnings`
- [ ] `cargo test --release --workspace` (all 6 integration tests still pass)
- [ ] New rule? Added a baseline/candidate fixture pair + golden integration test under `spectra-core/tests/`
- [ ] New rule? Added rule ID + severity row in [`docs/SEVERITY.md`](../docs/SEVERITY.md)
- [ ] New rule? Updated the boundary statement in [`docs/NON_GOALS.md`](../docs/NON_GOALS.md) if it narrows a documented non-goal
- [ ] Docs updated where relevant (README, TESTING, CHANGELOG `[Unreleased]`)

## Related issues

<!-- Use `Closes #123` to auto-close on merge, or `Refs #123` to link. -->

## Reviewer notes

<!--
Anything you want reviewers to focus on — tricky invariants, areas you
are unsure about, false-positive risk on real-world source trees, etc.
-->
