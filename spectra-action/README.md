# spectra-action

Composite GitHub Action that runs `spectra check` on every pull request and
fails the build when the upgrade removes, downgrades, or bypasses an
account-validation guard the deployed (baseline) version enforced.

This is a **scaffold** shipped with the M0 PoC. Full Marketplace publication
(with single updated-in-place PR-comment integration, a separate distribution
repo, and cached binary distribution) is the M3 deliverable per the grant
proposal.

## Inputs

| Input | Required | Default | Description |
|---|---|---|---|
| `baseline` | yes | — | Path to the baseline (last released / on-chain) program **source tree** |
| `candidate` | yes | — | Path to the candidate (upgrade under review) program **source tree** |
| `report-path` | no | `spectra-report.json` | Where to write the report |
| `format` | no | `json` | `json`, `markdown`, or `sarif` |
| `fail-on-breaking` | no | `true` | Fail the step when any BREAKING regression is reported |

`baseline` and `candidate` are Anchor **Rust source trees**, not IDL JSON.

## Example usage

```yaml
name: spectra
on:
  pull_request:
    paths: ['programs/**']

jobs:
  regression-gate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }

      - name: Materialise the baseline (last released) source tree
        run: git worktree add /tmp/baseline "${{ github.base_ref }}"

      - uses: ayodyadsr/spectra/spectra-action@main
        with:
          baseline: /tmp/baseline/programs
          candidate: programs
          report-path: spectra-report.md
          format: markdown
          fail-on-breaking: true
```

`fetch-depth: 0` is required so `git worktree add` can resolve the base ref.
The base branch is the right baseline only if it tracks the last on-chain
release — pin to a release tag if your default branch runs ahead of mainnet.
Spectra reads only source; no `anchor build` step is required for the gate.
