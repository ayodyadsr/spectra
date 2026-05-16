# Non-Goals

Spectra is **not** the following things. Every item here is an active design choice — not a missing feature, not "future work unless someone asks."

This document is referenced from the README so reviewers can verify Spectra is not silently claiming a scope it does not deliver.

---

## 1. Non-goal table

| Non-goal | Why this is not Spectra's job | Where the work belongs |
|----------|------------------------------|------------------------|
| **Prove upgrade correctness.** | Spectra detects **compatibility regressions**, not semantic equivalence. A no-op upgrade and an upgrade that secretly drains every account both produce zero structural findings if the IDL is identical. | Formal verification (audit-firm internal). |
| **Replace audit firms.** | The audit happens in code review. Spectra's only role is to make compatibility regressions visible during review. | Audit firms (OtterSec, Neodyme, Sec3, Halborn). |
| **Verify build reproducibility.** | "Does the deployed bytecode match the source?" is `solana-verifiable-build` / `solana-verify`. Spectra trusts the IDL inputs it is given. | `solana-verifiable-build`. |
| **Detect runtime exploits.** | Spectra runs pre-merge, in CI. It does not watch mainnet. | Hypernative, Range. |
| **Detect the Drift-exploit class (social engineering, multisig compromise, durable-nonce abuse, fictitious collateral).** | None of these touch the IDL. The April 2026 Drift exploit produced zero IDL/discriminator/layout changes; no IDL-diff tool or pre-deploy static analyser could detect it. The Drift IDL pair is a *fixture* here, not a prevention claim. | STRIDE pillars (operational security, access controls, multisig configurations, key management) + SIRN incident response. |
| **Enforce protocol invariants.** | "Total supply must never decrease" is a protocol invariant. Spectra has no DSL for them. | Protocol-specific assertions in code + audit-firm work. |
| **Detect Token-2022 TLV layout changes.** | IDL does not represent TLV. | A separate Token-2022 detector pack (listed as Future Expansion in the grant proposal). |
| **Detect Token-2022 transfer-hook reentrancy.** | Out of structural-diff scope. | Separate detector pack. |
| **Detect PDA seed-derivation drift.** | Requires BPF disassembly; research-grade. | Future Expansion only; not promised in this grant. |
| **Detect constant / `.rodata` changes.** | Not visible in IDL. A fee constant change from 30 bps to 300 bps does not appear. | Source-level review. |
| **Detect upgrade-authority transfers.** | Off-chain operational action. | Governance / multisig review. |
| **Detect compiler / LLVM bugs.** | Out of threat model. | Toolchain maintainers. |
| **Replay mainnet history.** | Infeasible on free-tier CI. M2 uses a bounded ≤ 50-tx hand-curated corpus per pilot. | Out-of-CI batch analysis if a pilot wants it. |
| **Score programs ("Spectra rating").** | No scoring system. Findings are structured rule IDs; reviewers decide. | Audit-firm reports. |
| **Suppress findings without rationale.** | Suppression schema requires non-empty `rationale`, explicit `expires`, and a PR URL. Empty rationale is refused at parse time. | The maintainer writes down why. |
| **Globally disable rules.** | Severity is not configurable. Only per-finding suppression is. | The maintainer suppresses individual findings. |
| **Operate without determinism.** | No network, no clock, no randomness. Same input bytes always produce the same output bytes. | If a future feature needs nondeterminism, it ships behind a separate subcommand and is labelled bounded heuristic. |

---

## 2. Compatibility ≠ correctness

The single most important non-claim:

> **Compatibility regression analysis is not correctness verification.**

A program upgrade can pass Spectra cleanly (no compatibility regressions) and still be a catastrophic correctness change. Example: same IDL, same layouts, same discriminators — but the `withdraw` instruction now sends to a different recipient. Spectra says zero findings, because the **schema** is preserved. Code review and audit catch this layer.

This sentence belongs in every Spectra adoption conversation. The grant proposal and the README both surface it.

---

## 3. The contract Spectra **does** make

For completeness, here is what Spectra **does** promise:

- Deterministic structural compatibility analysis of supported IDL schemas.
- Per-rule severity classification per [SEVERITY.md](SEVERITY.md).
- Refuse-to-analyse (exit 3) on inputs Spectra cannot soundly process.
- Public M0 PoC with green CI on every push.
- One pilot + two walkthroughs + FP-rate measurement in M4.

That is the entire promise. Everything in this document is outside it.

---

## 4. Cross-references

- Full edge-case matrix: [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).
- Threat model that frames these non-goals: [THREAT_MODEL.md](THREAT_MODEL.md).
- Architecture that enforces the determinism non-goal-of-randomness: [ARCHITECTURE.md](ARCHITECTURE.md).
- Position vs the Foundation's STRIDE/SIRN stack + the Drift detection-boundary analysis: [STRIDE_GAP_ANALYSIS.md](STRIDE_GAP_ANALYSIS.md).
