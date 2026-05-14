# False-Positive Mitigation Strategy

A CI-gating tool earns trust by being **right enough often enough** that maintainers do not start treating it as noise. This document is the explicit FP-policy: how Spectra avoids false positives by construction, where it cannot, and how the suppression mechanism (M3) closes the gap.

---

## 1. Defence in five layers

Spectra mitigates false positives at five layers, from strongest to weakest.

### Layer 1 — Structural correctness

Most findings are **definitions**, not heuristics. R-ACC-FIELD-REORDER is true whenever the field order differs; the only way to obtain a false positive is for the user to have already updated their on-chain accounts in lockstep, which is the case M3's `migration_declared` marker addresses.

### Layer 2 — Bounded comparator scope

Spectra refuses to analyse inputs it does not understand (exit code 3). This means a malformed or schema-unsupported IDL never produces a clean report — it produces an explicit refusal. See [SEVERITY.md](SEVERITY.md) §5.

### Layer 3 — Identical-IDL invariant

The CI suite includes an `Identical-IDL exit-0 check` step that runs `spectra check --old X --new X` and asserts exit 0 + zero findings. This is a permanent regression guard against any future change that would introduce spurious findings on inputs that are byte-identical.

### Layer 4 — Suppression file (M3)

When a true structural change is intentional (e.g. a coordinated migration where the protocol's own code resets accounts), the maintainer declares it in `spectra-allow.toml`:

```toml
# spectra-allow.toml
schema_version = 1

[[suppress]]
rule_id = "R-ACC-FIELD-REORDER"
target = "Pool"
rationale = "Pre-launch program — no on-chain accounts exist yet. Verified by checking program-data-account address has no descendant accounts."
expires = "2026-12-31"
upgrade_pr = "https://github.com/example/protocol/pull/812"
migration_declared = false

[[suppress]]
rule_id = "R-ACC-SILENT-CORRUPT"
target = "Pool"
rationale = "Coordinated migration: PR #813 re-initializes all 247 Pool accounts via init_if_needed prior to the upgrade landing."
expires = "2026-08-15"
upgrade_pr = "https://github.com/example/protocol/pull/813"
migration_declared = true
migration_script = "scripts/reset-pools.ts"
```

Required fields:

- `rule_id` — exact rule ID from [SEVERITY.md](SEVERITY.md).
- `target` — the specific symbol the suppression applies to (instruction name, account name, etc.).
- `rationale` — non-empty free text. Spectra **refuses** entries with empty rationale.
- `expires` — ISO date. Expired suppressions are reported as a separate `R-SUPPRESS-EXPIRED` warning so the file does not silently rot.
- `upgrade_pr` — URL pointing at the PR where the suppression was introduced.
- `migration_declared` — boolean. If true, see [MIGRATION.md](MIGRATION.md) for the additional fields.

Forbidden patterns:

- No `*` wildcard targets. Each suppression names one symbol.
- No global rule disable. Severities are not configurable; only per-finding suppression is.
- No silent suppression — every suppression entry produces a line in the report explaining what was suppressed and why.

### Layer 5 — Report transparency

Every report includes a **Suppressions** section listing each entry that fired, with its rationale visible to reviewers. Suppression is not "make the finding disappear" — it is "annotate the finding with a reviewable justification."

---

## 2. What counts as a false positive

| Case | Spectra's classification |
|------|--------------------------|
| Layout change with no existing on-chain accounts | True structural change; **not** an FP. User suppresses with `pre_launch` rationale. |
| Layout change accompanied by a migration that resets affected accounts | True structural change; **not** an FP. User declares via `migration_declared = true`. |
| Field rename with all callers updated in lockstep | Structural rename; surfaced for review. User suppresses with rationale if intentional. |
| Cosmetic JSON formatting change in IDL (whitespace, key order) | Spectra's parser is structural — JSON formatting is not visible. **Not** an FP, and not a finding. |
| Same IDL diffed against itself | **Tested invariant:** zero findings, exit 0. Any deviation is a Spectra bug. |

The first three cases are not bugs — they are designed user actions backed by `spectra-allow.toml`. The last two are correctness invariants.

---

## 3. Measurement plan (M4)

The proposal commits to **measuring** false-positive rate as part of M4, not asserting it:

- Per-pilot, record every Spectra finding produced during the pilot's upgrade cycle.
- For each finding, classify as: true regression caught, true regression already known, intentional change (suppressed), or false positive.
- Publish the per-pilot table in the M4 walkthrough write-up.

The proposal does **not** claim a numeric FP rate today. The mechanism is built; the number comes from real protocol data during M4.

---

## 4. Hard non-promises

Spectra does **not**:

- Disable rules globally.
- Allow wildcard suppression.
- Permit empty-rationale suppression.
- Hide suppressed findings from the report.

These are intentional friction. The whole point of the suppression file is that it forces the maintainer to write down **why** a finding is being waived, leaving an audit trail.

---

## 5. Cross-references

- Suppression schema lives in [MIGRATION.md](MIGRATION.md) §2.
- Rule IDs and severities: [SEVERITY.md](SEVERITY.md).
- Adoption strategy explains how the FP-policy is communicated to maintainers: [ADOPTION.md](ADOPTION.md).
