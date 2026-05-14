# spectra-action

Composite GitHub Action that runs `spectra check` on every pull request and posts a structured report.

This is a **scaffold** shipped with the M0 PoC. Full Marketplace publication (with PR-comment integration, separate distribution repo, and cached binary distribution) is the M3 deliverable per the grant proposal.

## Example usage

```yaml
name: spectra
on:
  pull_request:
    paths: ['target/idl/lending.json']

jobs:
  diff:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with: { fetch-depth: 0 }
      - name: Fetch baseline IDL
        run: git show origin/main:target/idl/lending.json > /tmp/old.json
      - uses: ayodyadsr/spectra/spectra-action@main
        with:
          old-idl: /tmp/old.json
          new-idl: target/idl/lending.json
          report-path: spectra-report.json
          format: markdown
```
