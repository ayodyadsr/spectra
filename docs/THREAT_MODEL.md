# Threat Model

This is the explicit threat model for Spectra: the adversary / failure sources
it is designed to help against, the trust assumptions it makes, and the
failure modes (soundness / completeness / robustness) under which it may
produce a wrong answer.

Spectra is **deterministic, strictly-differential account-validation
regression analysis** of Anchor source — not formal verification, not an
absolute scanner, not runtime monitoring. Every claim below is bounded
accordingly.

---

## 1. Adversary / failure-source classes

| ID | Class | Description | Spectra's role |
|---|---|---|---|
| A1 | Honest-but-rushed maintainer | An engineer ships an upgrade that unintentionally downgrades `Account<T>` to `UncheckedAccount`, deletes a `has_one` / `constraint`, drops a `seeds`/`bump`, or removes a `Signer` — with **no compiler error**, because Anchor enforces guards at runtime. | **Primary target.** Spectra fails CI before the upgrade reaches mainnet. |
| A2 | Inattentive reviewer | The guard removal is a one-line type change buried in a large refactor PR and is missed visually. | **Primary target.** Spectra emits a deterministic, typed finding the reviewer cannot overlook. |
| A3 | Malicious maintainer with merge access | An insider deliberately removes a guard to enable a later drain. | **Partial target.** Spectra surfaces the regression, but cannot prevent merge if the same actor can also disable CI. Defence depends on branch-protection rules outside Spectra. |
| A4 | External PR submitter (no merge access) | A non-maintainer opens a PR that silently weakens a guard. | **Primary target.** Spectra's job is to make the regression visible during review. |
| A5 | Build-supply-chain attacker | An attacker substitutes the binary or the source between review and deployment. | **Out of scope.** Build-provenance tools (`solana-verifiable-build` / `anchor verify`) cover this. Spectra does not. |

Spectra is most useful against A1, A2, A4 (unintentional or unnoticed
regressions). It is partially useful against A3 (visibility) and explicitly
does **not** replace A5's tooling.

---

## 2. Trust assumptions

Spectra **trusts** the following as authoritative:

- The two Anchor source trees supplied via `--baseline` and `--candidate`.
  Spectra makes **no** claim that the baseline tree corresponds to the
  on-chain bytecode — that is `solana-verifiable-build` / `anchor verify`
  territory.
- That the baseline tree is the *last released / on-chain-deployed* version.
  The differential guarantee is only as good as that premise; selecting the
  baseline is an adoption concern (auto-baseline = last on-chain version),
  not an engine concern.
- Anchor's documented guard semantics: typed wrappers enforce owner + 8-byte
  discriminator; `#[account(...)]` constraints are enforced at runtime.

Spectra makes **no** assumption about network state, clock, randomness, build
reproducibility, or the honesty of the actor who supplied the trees. If the
baseline tree is forged or stale, the findings remain internally consistent
for that pair but may not reflect the truly deployed program. This is an
explicit limitation, not a bug.

---

## 3. Failure modes

### 3.1 Soundness (false positives)

A soundness failure is a BREAKING finding that does not in fact remove a
guarantee. The strictly-differential design makes this **near-zero by
construction**:

- Identical input trees → zero findings (tested invariant + CI step).
- A finding requires a guard present in the baseline slot and absent from
  the candidate slot.
- The downgrade-vs-equivalent-pin logic (see [SEVERITY.md](SEVERITY.md) §3
  and [FALSE_POSITIVES.md](FALSE_POSITIVES.md)) suppresses the obvious FP
  class: a guard re-expressed but still enforced (e.g. `Account<T>` →
  `#[account(owner = …)] UncheckedAccount`) produces **no** finding.

Residual FP sources and their mitigation (M3 `spectra-allow.toml` with
mandatory rationale + expiry): an intentional, reviewed guard relaxation
paired with a compensating off-`#[derive(Accounts)]` check the engine cannot
see; a pre-launch program where no on-chain account yet depends on the guard.
Concrete FP-rate observation is part of M4 (≥1 pilot + 2 walkthroughs).

### 3.2 Completeness (false negatives)

A completeness failure is a real account-validation regression Spectra does
**not** detect at M0. Documented, not silent:

- Native (non-Anchor) manual `is_signer` / `owner ==` / `key ==` checks —
  M1 deliverable.
- Whole-context removal — interface change, opt-in strict mode in M1.
- A guard enforced *outside* `#[derive(Accounts)]` (manual `require!()` in the
  instruction body) — M1 native-path scope.
- A regression where the guard was *already missing in the baseline* — this
  is **by construction** out of scope (absolute-scanner territory), not a
  false negative.

Each is in the README's "What Spectra does NOT do" section and in
[NON_GOALS.md](NON_GOALS.md) so reviewers cannot mistake silence for
coverage.

### 3.3 Robustness (malformed input)

Spectra refuses to produce a misleading clean report:

- A file in the tree that does not parse as Rust is **skipped**, not fatal —
  guard extraction proceeds on the parseable files (a partial tree is normal
  during refactors).
- A path that does not exist, or a tree with no readable Rust, or an unknown
  `--format` → exit `2` (invocation error).
- There is **no exit code 3**. "Clean report + exit 0" never silently hides
  "I did not understand your input": un-processable input is exit `2`.

---

## 4. Out-of-scope adversary capabilities

Spectra is **not** designed to defend against, and makes **no** claim about:
compiler / SBF backend miscompilation; validator-level consensus exploits;
wallet/UX phishing; off-chain governance / multisig signer compromise (the
Drift attack class — see [STRIDE_GAP_ANALYSIS.md](STRIDE_GAP_ANALYSIS.md) §5);
frontend / client substitution; logic / economic / oracle bugs that remove no
account-validation guard. These are named explicitly so reviewers can verify
Spectra is not over-claiming.

---

## 5. Cross-references

- Authoritative engineering spec: [`../TECHNICAL_SPEC.md`](../TECHNICAL_SPEC.md)
- Severity per finding kind + exit-code contract: [SEVERITY.md](SEVERITY.md)
- Explicit non-goals: [NON_GOALS.md](NON_GOALS.md)
- Near-zero-FP-by-construction argument: [FALSE_POSITIVES.md](FALSE_POSITIVES.md)
- Position vs the Foundation stack + Drift boundary: [STRIDE_GAP_ANALYSIS.md](STRIDE_GAP_ANALYSIS.md)
