# Migration Awareness

A non-trivial fraction of "BREAKING" findings against real protocol upgrades are **intentional** — the maintainer has a coordinated migration that resets or re-initializes affected accounts before / during the upgrade. Spectra must surface these findings (silent suppression is unsafe) and must give the maintainer a reviewable way to declare the migration so the CI gate does not become a permanent block.

This document specifies the migration-declaration mechanism added in M3.

---

## 1. The migration problem

Concrete example: a protocol team ships an upgrade that reorders fields on the `Pool` account. Without context, Spectra correctly produces:

- R-ACC-FIELD-REORDER on `Pool`.
- R-ACC-SILENT-CORRUPT on `Pool` (discriminator unchanged + layout changed).

Both findings are real. They are also exactly what the maintainer **intended**, because PR #813 ships a one-shot reset script that iterates every `Pool` account and re-initializes it via `init_if_needed` before the upgrade authority is invoked.

The maintainer needs a way to say "yes, this is intentional, here is the migration" — visible to reviewers, expirable, and auditable.

---

## 2. Suppression schema (M3)

`spectra-allow.toml` lives at the root of the consuming repository.

```toml
schema_version = 1

[[suppress]]
rule_id = "R-ACC-FIELD-REORDER"
target = "Pool"
rationale = "Coordinated migration: PR #813 re-initializes all 247 Pool accounts via init_if_needed prior to the upgrade landing."
expires = "2026-08-15"
upgrade_pr = "https://github.com/example/protocol/pull/813"
migration_declared = true
migration_script = "scripts/reset-pools.ts"
migration_pr = "https://github.com/example/protocol/pull/813"
expected_affected_accounts = 247

[[suppress]]
rule_id = "R-ACC-SILENT-CORRUPT"
target = "Pool"
rationale = "Same migration as above — Pool layout is intentionally changing and is reset in lockstep."
expires = "2026-08-15"
upgrade_pr = "https://github.com/example/protocol/pull/813"
migration_declared = true
migration_script = "scripts/reset-pools.ts"
```

| Field | Required | Purpose |
|-------|----------|---------|
| `rule_id` | yes | Exact rule ID from [SEVERITY.md](SEVERITY.md). |
| `target` | yes | One named symbol (no wildcards). |
| `rationale` | yes (non-empty) | Why the finding is being waived. |
| `expires` | yes | ISO date. After expiry, Spectra produces R-SUPPRESS-EXPIRED. |
| `upgrade_pr` | yes | URL of the PR that introduces the suppression. |
| `migration_declared` | yes | Boolean: is this waiver backed by an actual coordinated migration? |
| `migration_script` | required if `migration_declared = true` | Path to the migration code in the repo. |
| `migration_pr` | required if `migration_declared = true` | URL of the migration PR. |
| `expected_affected_accounts` | optional | If present, Spectra (M2+) cross-checks corpus replay accounts. |

---

## 3. Visibility, not silence

Suppression in Spectra is **annotation**, not deletion:

- Suppressed findings still appear in the report under a `Suppressions` section.
- Each line cites `rule_id`, `target`, `rationale`, `expires`, and the upgrade PR URL.
- The exit code is downgraded from `1` to `0` only if every BREAKING finding has a matching suppression.
- If even one BREAKING finding has no suppression, the exit code stays `1`.

This is the same pattern used by linters that treat warnings as reviewable, not erasable.

---

## 4. Refused inputs

The suppression parser **refuses**:

- Empty `rationale`.
- Wildcard `target = "*"`.
- `migration_declared = true` without `migration_script` and `migration_pr`.
- `expires` in the past at parse time (renders as a forced-fail R-SUPPRESS-EXPIRED).
- Duplicate `(rule_id, target)` pairs.
- An unknown `rule_id`.

Refusal is exit code 3 (refuse to analyse) — same path as an unsupported IDL schema. The intent is symmetric: anything the engine cannot soundly process is surfaced loudly, not absorbed.

---

## 5. Audit trail

Every `spectra check` run that consumes a `spectra-allow.toml` emits a `[suppressions]` block in the JSON report containing every entry that fired, including:

- The matched finding it suppressed.
- The full suppression row from `spectra-allow.toml`.
- A SHA-256 hash of the suppression file as it existed at run time.

A reviewer can compare two runs against the same upgrade by hashing the suppression file and confirming no quiet edit happened between them.

---

## 6. What migration declarations do **not** do

- They do **not** verify that the migration script is correct. Spectra reads the path; it does not run it. Verification is the protocol team's responsibility (and the audit firm's job).
- They do **not** waive the requirement for a corpus replay finding to pass in M2. The static finding can be suppressed; a replay R-REPLAY-DESERIALIZE-FAIL must be addressed in code.
- They do **not** persist forever. Every suppression carries `expires`. The file is intended to rot intentionally so old waivers are revisited.

---

## 7. Cross-references

- FP policy that wraps this schema: [FALSE_POSITIVES.md](FALSE_POSITIVES.md).
- Rule IDs referenced from suppressions: [SEVERITY.md](SEVERITY.md).
- M3 milestone where suppression lands: [ROADMAP.md](ROADMAP.md).
