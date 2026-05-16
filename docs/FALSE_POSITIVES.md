# False Positives — Near-Zero by Construction

A CI-gating tool earns trust by being **right enough, often enough** that
maintainers never start treating it as noise. Spectra's defining property is
that its false-positive rate is **near-zero by construction, not by heuristic
tuning**. This document explains why, where the residual risk is, and how the
M3 suppression mechanism closes it.

---

## 1. Why FP is near-zero by construction

An absolute scanner asks *"is there a missing owner check anywhere in this
code?"* Any large program has intentionally-unchecked accounts (escape
hatches, `AccountInfo` forwarded to a CPI), so the scanner must tune a
heuristic threshold and lives with a false-positive budget.

Spectra only ever asks *"did this upgrade remove a guard the deployed version
enforced?"* That reframing removes the heuristic entirely:

### Layer 1 — Differential definition, not heuristic

A finding is a **set-difference fact**: a guard is present in the baseline
slot and absent from the candidate slot. There is no threshold, no
confidence score, no pattern-likelihood. `signer_check_removed` is true iff
the baseline slot had a `Signer` guard and the candidate slot does not.

### Layer 2 — Identical-input invariant

`spectra check --baseline X --candidate X` produces **zero findings, exit
0**. This is a tested integration invariant and a CI step. Any future change
that would emit a spurious finding on byte-identical input breaks the build.

### Layer 3 — Strictly-differential no-FP property

The integration suite asserts that an **unchanged context inside an
otherwise-changed program produces zero findings**. A finding can never name
a context that did not lose a guard, even when the program as a whole
changed. This is the property that makes Spectra silent on everything except
real regressions.

### Layer 4 — Downgrade-vs-equivalent-pin logic

The single realistic FP class for a differential differ is a guard that was
**re-expressed but still enforced**. Spectra handles this in the engine:

| Change | Finding? |
|---|---|
| `Account<'info, Mint>` → `#[account(owner = token::ID)] UncheckedAccount` | **No** — owner still pinned |
| `Account<'info, Mint>` → `UncheckedAccount` (no pin) | `type_cosplay_protection_removed` |
| `Program<'info, Token>` → `#[account(address = token::ID)] AccountInfo` | **No** — program id still pinned |
| `Program<'info, Token>` → `UncheckedAccount` | `cpi_target_unpinned` |

The `type_cosplay_protection_removed`, `owner_check_removed`, and
`cpi_target_unpinned` arms fire only if **no equivalent pin remains**. See
[SEVERITY.md](SEVERITY.md) §3 and the `diff_slot` logic in
`spectra-core/src/regression.rs`.

### Layer 5 — Silent on already-missing checks (by construction)

A check that was *already missing in the baseline* is **not** a regression.
Spectra is deliberately silent — that is the absolute scanners' job. This is
the single biggest source of would-be noise in an absolute tool, and
Spectra's differential design eliminates it categorically rather than tuning
it down.

---

## 2. Residual FP risk and the M3 suppression file

The construction above leaves one residual class: an **intentional, reviewed
guard relaxation** where a compensating check exists *outside*
`#[derive(Accounts)]` (a manual `require!()` in the instruction body that the
M0 engine does not model), or a pre-launch program where no on-chain account
yet depends on the guard.

M3 introduces `spectra-allow.toml` for exactly this — and only this:

```toml
schema_version = 1

[[suppress]]
rule_id      = "owner_check_removed"
target       = "Withdraw::destination"
rationale    = "Owner is now enforced by an explicit require!(destination.owner == authority.key()) in the instruction body (PR #813); the #[account] pin was redundant."
expires      = "2026-12-31"
upgrade_pr   = "https://github.com/example/protocol/pull/813"
```

Required fields, all enforced at parse time:

- `rule_id` — exact rule ID from [SEVERITY.md](SEVERITY.md).
- `target` — one `Context::account` symbol. **No `*` wildcards.**
- `rationale` — non-empty. An empty rationale is **refused**.
- `expires` — ISO date. An **expired** suppression fails CI (no silent rot).
- `upgrade_pr` — URL of the PR that introduced the suppression.

Forbidden by design: global rule disable (severity is not configurable),
wildcard targets, empty-rationale entries, and silent suppression — every
suppressed finding still appears in the report, annotated with its
rationale. Suppression is "annotate the finding with a reviewable
justification with an expiry," never "make it disappear."

---

## 3. What counts as a false positive

| Case | Classification |
|---|---|
| Identical trees diffed | Tested invariant: zero findings, exit 0. Any deviation is a Spectra bug. |
| Guard re-expressed but still enforced (`Account<T>` → owner-pinned Unchecked) | **Not a finding** — Layer 4 suppresses it in the engine. |
| Check already missing in the baseline | **Not a finding** — out of scope by construction (absolute-scanner territory). |
| Guard genuinely removed, but compensated by a manual body check the engine cannot see | True regression *relative to the declarative surface*; surfaced for review; suppressible via M3 with rationale. **Engine FP only in the strict sense that a non-`#[derive(Accounts)]` check exists** — M1 native-path analysis narrows this. |
| Guard genuinely removed, intentional | True regression, correctly flagged. The maintainer suppresses with rationale + expiry. Not an FP. |

---

## 4. Measurement plan (M4)

The proposals **measure** the false-positive rate; they do not assert one
today (`[NO PUBLIC DATA AVAILABLE]` until M4):

- Per pilot, record every finding produced during the upgrade cycle.
- Classify each: true regression caught / true regression already known /
  intentional relaxation (suppressed) / false positive.
- Publish the per-pilot table in the M4 walkthrough.

---

## 5. Cross-references

- Severity + downgrade-vs-pin rule: [SEVERITY.md](SEVERITY.md)
- Threat-model soundness section: [THREAT_MODEL.md](THREAT_MODEL.md) §3.1
- Authoritative engine description: [`../TECHNICAL_SPEC.md`](../TECHNICAL_SPEC.md)
- Why already-missing checks are out of scope: [NON_GOALS.md](NON_GOALS.md)
