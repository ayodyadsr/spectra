# CI/CD Integration

Spectra is **CI-native**: its purpose is to fail a pre-merge build when an
upgrade PR silently removes an account-validation guard the deployed version
enforced. This document gives drop-in templates for the integration paths
that cover the majority of Anchor repositories.

All examples use the M0 CLI shape:

```
spectra check --baseline <DIR> --candidate <DIR>
              [--format json|markdown|md|sarif] [--report <PATH>] [--quiet]
```

`--baseline` and `--candidate` are **Rust source trees**, not IDL JSON. The
baseline is the last released / on-chain-deployed version; the candidate is
the PR head. The M3 composite Action will package the same logic into a
single `uses:` line.

---

## 1. GitHub Actions (M0-compatible)

Place at `.github/workflows/spectra.yml` in the target repository:

```yaml
name: Spectra account-validation regression gate

on:
  pull_request:
    paths:
      - "programs/**"

jobs:
  spectra:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0

      - name: Install Spectra
        run: |
          curl -L -o spectra https://github.com/ayodyadsr/spectra/releases/latest/download/spectra-linux-x86_64
          chmod +x spectra
          sudo mv spectra /usr/local/bin/

      - name: Materialise the baseline (last released) source tree
        run: git worktree add /tmp/baseline "${{ github.base_ref }}"

      - name: Run Spectra (SARIF)
        run: |
          spectra check \
            --baseline /tmp/baseline/programs \
            --candidate programs \
            --format sarif \
            --report spectra.sarif || true   # never short-circuit the upload

      - name: Upload SARIF to GitHub Code Scanning
        if: always()
        uses: github/codeql-action/upload-sarif@v3
        with:
          sarif_file: spectra.sarif
          category: spectra

      - name: Enforce the gate
        run: |
          spectra check \
            --baseline /tmp/baseline/programs \
            --candidate programs \
            --format markdown \
            --report spectra-report.md

      - name: Upload report
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: spectra-report
          path: spectra-report.md
```

Notes:

- `fetch-depth: 0` is required so `git worktree add` can resolve the base
  ref. The base branch is the right baseline only if it tracks the last
  on-chain release — pin to a release tag if your default branch runs ahead
  of mainnet.
- The first `spectra check` runs with `|| true` so the SARIF artifact uploads
  even on a BREAKING exit; the second invocation (no `|| true`) is the actual
  merge gate — a BREAKING finding (exit `1`) fails the PR.
- Spectra reads only source; no `anchor build` step is required for the gate.

---

## 2. Pre-commit hook (M0-compatible)

A local guard before push, comparing the working tree against `HEAD`:

```bash
# .git/hooks/pre-commit
#!/usr/bin/env bash
set -euo pipefail

command -v spectra >/dev/null || { echo "spectra not installed — skipping"; exit 0; }

BASE=$(mktemp -d)
git worktree add --detach "$BASE" HEAD >/dev/null 2>&1 || exit 0
trap 'git worktree remove --force "$BASE" >/dev/null 2>&1 || true' EXIT

spectra check --baseline "$BASE/programs" --candidate programs --format markdown
```

The hook does not block when Spectra is absent; adoption is opt-in per
machine.

---

## 3. `cargo make` task (M0-compatible)

```toml
# Makefile.toml
[tasks.spectra-check]
description = "Spectra regression check vs origin/main source tree"
script = '''
set -euo pipefail
git fetch origin main
BASE=$(mktemp -d)
git worktree add --detach "$BASE" origin/main
spectra check --baseline "$BASE/programs" --candidate programs --format markdown
git worktree remove --force "$BASE"
'''
```

Invoke with `cargo make spectra-check`.

---

## 4. GitHub Code Scanning (SARIF)

Spectra emits SARIF 2.1.0 with `--format sarif`, uploadable via
`github/codeql-action/upload-sarif@v3` — the same first-party surface GitHub
Advanced Security uses for CodeQL. Each finding maps to a SARIF `result`
whose `ruleId` is the Spectra finding kind (e.g.
`type_cosplay_protection_removed`) and whose `level` is derived from severity
(`error` for BREAKING, `warning` for warning). The result's
`logicalLocations` carry the `Context::account` pair as a stable navigation
handle. The driver name is `Spectra`; the driver `rules` array carries all 9
rule descriptors with `helpUri`s into [SEVERITY.md](SEVERITY.md). On the
bundled fixture pair the SARIF document contains exactly 7 `results`
(6 `error` + 1 `warning`) — verified by a CI smoke step.

---

## 5. M3 composite Action (pending grant)

M3 collapses the consumer YAML to:

```yaml
- uses: ayodyadsr/spectra-action@v1
  with:
    baseline-ref: <release-tag-or-base-ref>
    programs-dir: programs
    suppression-file: spectra-allow.toml
    post-pr-comment: true
```

The Action will: materialise the baseline tree from `baseline-ref`, run
`spectra check`, upload SARIF, maintain a single updated-in-place PR comment,
and honour `spectra-allow.toml`. Spectra's own CI dogfoods the published
Action as its smoke gate.

---

## 6. Failure-handling guidance for maintainers

When Spectra fails CI on your PR, **do not** suppress blindly:

1. Read the finding — each cites a `rule_id` linking to
   [SEVERITY.md](SEVERITY.md).
2. Decide: did this upgrade really remove a guard the deployed version
   enforced?
   - **Yes** → fix the upgrade; the gate did its job.
   - **No — the guard is now enforced another way the engine cannot see** →
     suppress in `spectra-allow.toml` with a rationale citing the
     compensating check + an `expires` date (M3).
   - **No — pre-launch, no on-chain accounts yet** → suppress with rationale
     and a near-term expiry.
3. Never suppress with an empty rationale — Spectra refuses it.

---

## 7. Performance budget

The M0 engine is a pure `syn`-parse + set-diff pipeline with no network and
no build step. It is fast enough that the Spectra step is negligible against
`anchor build` / test time in any realistic PR. No specific wall-clock number
is asserted here: a reproducible timing on a real public Anchor program's
deployed-vs-upgrade source pair is an explicit **M1.5** deliverable
(`[NO PUBLIC DATA AVAILABLE]` until then). The M2 replay milestone is bounded
to ≤60 s end-to-end so the full pipeline stays within a free-tier CI budget.

---

## 8. Cross-references

- Exit-code semantics: [SEVERITY.md](SEVERITY.md) §4
- Suppression schema: [FALSE_POSITIVES.md](FALSE_POSITIVES.md) §2
- Architecture context: [ARCHITECTURE.md](ARCHITECTURE.md)
- Authoritative engine spec: [`../TECHNICAL_SPEC.md`](../TECHNICAL_SPEC.md)
