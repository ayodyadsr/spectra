# CI/CD Integration

Spectra is **CI-native**: its purpose is to fail a pre-merge build before a layout-breaking upgrade lands. This document gives drop-in templates for the three integration paths that cover the majority of Solana repositories today.

All examples use the M0 CLI shape: `spectra check --old <PATH> --new <PATH> [--format json|markdown] [--report <PATH>]`. The M3 composite Action will package the same logic into a single `uses:` line; the templates below are what teams can deploy today against the M0 binary.

---

## 1. GitHub Actions (M0-compatible)

Place at `.github/workflows/spectra.yml` in the target repository:

```yaml
name: Spectra compatibility check

on:
  pull_request:
    paths:
      - "programs/**"
      - "idl/**"

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

      - name: Build IDL for base branch
        run: |
          git worktree add /tmp/base "${{ github.base_ref }}"
          (cd /tmp/base && anchor build && cp target/idl/<program>.json /tmp/old.json)

      - name: Build IDL for PR head
        run: |
          anchor build
          cp target/idl/<program>.json /tmp/new.json

      - name: Run Spectra
        run: |
          spectra check \
            --old /tmp/old.json \
            --new /tmp/new.json \
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

- Replace `<program>` with the target program name.
- `fetch-depth: 0` is required so `git worktree add` can resolve the base ref.
- The job's exit code is the `spectra check` exit code; a BREAKING finding (exit 1) fails the PR.

---

## 2. Pre-commit hook (M0-compatible)

For maintainers who want a local guard before the push:

```bash
# .git/hooks/pre-commit
#!/usr/bin/env bash
set -euo pipefail

if ! command -v spectra >/dev/null; then
  echo "spectra not installed — skipping (run `cargo install --git https://github.com/ayodyadsr/spectra spectra-core` to enable)"
  exit 0
fi

# Compare the IDL in the staged commit against HEAD.
OLD=$(git show HEAD:idl/program.json 2>/dev/null || echo "")
NEW=$(cat idl/program.json 2>/dev/null || echo "")

if [[ -z "$OLD" || -z "$NEW" ]]; then
  exit 0
fi

OLD_TMP=$(mktemp); NEW_TMP=$(mktemp)
echo "$OLD" > "$OLD_TMP"
echo "$NEW" > "$NEW_TMP"
spectra check --old "$OLD_TMP" --new "$NEW_TMP" --format markdown
```

The hook does not block when Spectra is absent; it surfaces an informational hint instead. Adoption is opt-in per developer machine.

---

## 3. `cargo make` task (M0-compatible)

For `cargo-make` users:

```toml
# Makefile.toml
[tasks.spectra-check]
description = "Run Spectra compatibility check against origin/main IDL"
script = """
set -euo pipefail
git fetch origin main
OLD_IDL=$(git show origin/main:idl/program.json)
NEW_IDL=$(cat idl/program.json)
OLD=$(mktemp); NEW=$(mktemp)
echo "$OLD_IDL" > "$OLD"
echo "$NEW_IDL" > "$NEW"
spectra check --old "$OLD" --new "$NEW" --format markdown
"""
```

Invoke with `cargo make spectra-check`.

---

## 4. M3 composite Action (pending grant)

M3 packages all of the above as a single composite Action so the consumer YAML collapses to:

```yaml
- uses: ayodyadsr/spectra-action@v1
  with:
    program: <program-name>
    suppression-file: spectra-allow.toml
    post-pr-comment: true
```

The Action will:

1. Build IDL for base + head.
2. Run `spectra check`.
3. Post the markdown report as a PR comment (idempotent via comment marker).
4. Upload the JSON report as an artifact.
5. Honour `spectra-allow.toml` if present.

---

## 5. Failure handling guidance for maintainers

When Spectra fails CI on your PR, **do not** suppress blindly. The recommended triage:

1. Read the finding. Each finding cites a `rule_id` (e.g. `R-ACC-FIELD-REORDER`) which links back to [SEVERITY.md](SEVERITY.md).
2. Decide: is this a true regression?
   - **Yes** -> fix the upgrade. The finding has done its job.
   - **No, and we have a coordinated migration** -> declare it in `spectra-allow.toml` with `migration_declared = true` and reference the migration PR.
   - **No, and the program is pre-launch / has no on-chain accounts** -> suppress with `rationale` and a near-term `expires` date.
3. Never suppress with empty rationale. Spectra refuses entries with no rationale.

---

## 6. Performance budget

M0 cold-run on a 10-instruction / 5-account IDL pair on a free-tier `ubuntu-latest` runner completes in well under 1 second. The M2 replay milestone is bounded to ≤ 60 s end-to-end so the total Spectra step in a typical PR stays under the threshold where maintainers begin treating CI as a tax.

---

## 7. Cross-references

- Exit-code semantics: [SEVERITY.md](SEVERITY.md) §5.
- Suppression schema: [FALSE_POSITIVES.md](FALSE_POSITIVES.md) and [MIGRATION.md](MIGRATION.md).
- Architecture context: [ARCHITECTURE.md](ARCHITECTURE.md).
