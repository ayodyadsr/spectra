# Non-Goals

Spectra is **not** the following things. Every item here is an active design
choice — not a missing feature, not "future work unless someone asks."

This document is referenced from the README and both proposals so reviewers
can verify Spectra is not silently claiming a scope it does not deliver.

---

## 1. Non-goal table

| Non-goal | Why this is not Spectra's job | Where the work belongs |
|---|---|---|
| **Be an absolute scanner.** A check that was *already missing in the baseline* is, by construction, not a regression. Spectra stays silent on it deliberately — re-implementing absolute scanning would import the false-positive-tuning problem the differential design exists to avoid. | The strictly-differential question ("did this upgrade *remove* a guard?") is structurally different from the absolute question ("is there a missing guard *anywhere*?"). | Sec3 X-Ray, Auditware Radar, l3x, Octane. |
| **Prove upgrade correctness.** A no-op refactor and an upgrade that secretly redirects every withdrawal both produce zero findings if no account-validation guard was removed. Spectra checks guard *regressions*, not semantic equivalence. | Functional correctness is a proof problem, not a guard-set diff. | Formal verification (audit-firm internal / STRIDE FV tier). |
| **Replace audit firms.** The audit happens in code review. Spectra's role is to make account-validation regressions impossible to miss during that review. | Auditing is human + tool judgement over the whole program. | OtterSec, Neodyme, Sec3, Halborn, etc. |
| **Verify build provenance.** "Does the deployed bytecode match this source?" is a different question. Spectra trusts the two source trees it is handed; it does **not** verify the baseline tree corresponds to the on-chain program. | Reproducible-build verification. | `solana-verifiable-build` / `anchor verify`. |
| **Detect runtime exploits.** Spectra runs pre-merge, in CI. It does not watch mainnet. | Post-deployment monitoring is a streaming problem. | Hypernative, Range, STRIDE ongoing opsec. |
| **Detect the Drift-exploit class** (social engineering, multisig compromise, durable-nonce abuse, fictitious collateral). | None of these are account-validation guard regressions; the upgrade path was not the attack surface. | STRIDE pillars (operational security, access controls, multisig, key management) + SIRN incident response. |
| **Be an IDL differ.** Spectra reads Rust **source**, not IDL JSON. It makes **no** claim about Anchor discriminator drift, Borsh layout, or account-field reorder — that is a different problem and explicitly out of scope. | Guard extraction needs the source AST (`#[derive(Accounts)]`, `#[account(...)]`), not the IDL. | Out of scope entirely — not a future-work item. |
| **Analyse native (non-Anchor) programs at M0.** Manual `is_signer` / `owner ==` / `key ==` checks are an M1 roadmap item. | M0 models Anchor's *declarative* guard surface only; native checks need `syn`-AST flow analysis. | M1 deliverable — documented, not silently mis-handled. |
| **Flag whole-context removal at M0.** A `#[derive(Accounts)]` context removed entirely is an interface change, not a silent weakening of a still-callable instruction. | Different risk class; opt-in strict mode is an M1 item. | M1 opt-in flag; default unchanged. |
| **Enforce protocol invariants.** "Total supply must never decrease" is a protocol invariant. Spectra has no DSL for them. | Invariant proof is audit/FV territory. | Protocol-specific assertions + audit-firm work. |
| **Detect constant / `.rodata` changes.** A fee constant change from 30 bps to 300 bps is not an account-validation guard. | Not in the guard model. | Source-level review. |
| **Detect upgrade-authority transfers.** Off-chain operational action, not a guard in `#[derive(Accounts)]`. | Not visible to a source guard-set diff. | Governance / multisig review. |
| **Replay mainnet history.** Infeasible on free-tier CI. M2 uses a bounded ≤50-tx hand-curated corpus per pilot. | CI runners are time-bounded. | Out-of-CI batch analysis if a pilot wants it. |
| **Score programs ("Spectra rating").** No scoring system. Findings are typed rule IDs; reviewers decide. | Scoring invites gaming and false confidence. | Audit-firm reports. |
| **Suppress findings without rationale (M3).** The `spectra-allow.toml` schema requires a non-empty `rationale`, an explicit `expires`, and an `upgrade_pr` URL; an expired suppression fails CI. | Silent waivers defeat the gate. | The maintainer writes down why, with an expiry. |
| **Operate without determinism.** No network, no clock, no randomness. Same input bytes always produce the same output bytes. | A non-deterministic gate cannot be a `required` check. | If a future feature needs nondeterminism it ships behind a separate, clearly-labelled subcommand. |

---

## 2. Regression ≠ correctness

The single most important non-claim:

> **Account-validation regression analysis is not correctness verification.**

A program upgrade can pass Spectra cleanly (no guard removed) and still be a
catastrophic correctness change — same guards, same contexts, but the
`withdraw` instruction now computes the wrong amount. Spectra reports zero
findings, because **no account-validation guarantee was removed**. Code review
and audit catch that layer. This sentence belongs in every Spectra adoption
conversation, and both proposals surface it.

---

## 3. The contract Spectra **does** make

- Deterministic, strictly-differential analysis of Anchor
  `#[derive(Accounts)]` account-validation guards (M0).
- A finding is emitted **only** when a guard in the baseline slot is absent
  from the candidate slot (no equivalent pin remaining).
- Near-zero false positives by construction: identical input → zero findings.
- Per-rule severity per [SEVERITY.md](SEVERITY.md); exit `0` / `1` / `2`.
- Public M0 PoC with green CI on every push.
- One pilot + two public walkthroughs + FP-rate observation in M4.

That is the entire promise. Everything in §1 is outside it.

---

## 4. Cross-references

- Authoritative engineering spec: [`../TECHNICAL_SPEC.md`](../TECHNICAL_SPEC.md).
- Threat model framing these non-goals: [THREAT_MODEL.md](THREAT_MODEL.md).
- Architecture enforcing determinism: [ARCHITECTURE.md](ARCHITECTURE.md).
- Why FP is near-zero by construction: [FALSE_POSITIVES.md](FALSE_POSITIVES.md).
- Position vs the Foundation's STRIDE/SIRN stack + absolute scanners and the
  Drift detection-boundary analysis: [STRIDE_GAP_ANALYSIS.md](STRIDE_GAP_ANALYSIS.md).
