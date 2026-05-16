# Spectra vs the Solana Foundation Security Stack — Gap Analysis (2026-05-16)

**Purpose.** Position Spectra in the Solana Foundation's post-April-2026
security landscape. After the $286M Drift exploit (2026-04-01) and the
Foundation's response (STRIDE + SIRN launch, 2026-04-07), the funded security
stack consolidated. This document maps what the stack covers, what it does
not, and where Spectra fits as **complementary** infrastructure — not
competing — at the **CI-time, upgrade-time, account-validation-regression**
position.

## 1. The current Solana security stack (2026-05-16)

Per [solana.com/news/solana-ecosystem-security](https://solana.com/news/solana-ecosystem-security)
and [Asymmetric Research's STRIDE announcement](https://blog.asymmetric.re/introducing-stride-a-security-program-for-the-solana-ecosystem/):

| Layer | Tool / Program | Lifecycle stage | Funded by |
|---|---|---|---|
| Post-deployment evaluation | **STRIDE** (Asymmetric Research) | Operational | Solana Foundation grants |
| Post-deployment monitoring | STRIDE ongoing opsec | Runtime, >$10M TVL | Solana Foundation grants |
| Formal verification | STRIDE FV tier | Mathematical proof, >$100M TVL | Solana Foundation grants |
| Incident response | **SIRN** (Asymmetric + OtterSec + Neodyme + Squads + ZeroShadow) | Active incident | Membership / Foundation |
| Free ecosystem — threat detection | Hypernative | Runtime | Free tier |
| Free ecosystem — risk monitoring | Range Security | Runtime | Free tier |
| Free ecosystem — attack simulation | Riverguard by Neodyme | Pre-deployment, exploit replay | Free tier |
| Free ecosystem — absolute static analysis | Sec3 X-Ray | Pre-deployment, single snapshot | Free tier |
| Free ecosystem — absolute rule templates | Auditware Radar | Pre-deployment, single snapshot | Free tier |
| Free ecosystem — absolute scanners (additional) | l3x, Octane | Pre-deployment, single snapshot | Free tier |
| **CI-gate account-validation regression** | **(no funded tool)** | **CI / PR-merge, upgrade-time, differential** | **GAP** |

## 2. STRIDE's pillars (named)

Per [Asymmetric Research blog](https://blog.asymmetric.re/introducing-stride-a-security-program-for-the-solana-ecosystem/)
and [Coindesk 2026-04-07](https://www.coindesk.com/tech/2026/04/07/solana-foundation-unveils-security-overhaul-days-after-usd270-million-drift-exploit),
STRIDE evaluates protocols across eight pillars. Seven are publicly named:

1. Operational security
2. Access controls
3. Multisig configurations
4. Governance vulnerabilities
5. Smart contract integrity
6. Key management practices
7. Economic design
8. `[NO PUBLIC TEXT AVAILABLE — listed only in the "8-Pillars-of-STRIDE" image at blog.asymmetric.re]`

Verbatim Asymmetric framing: STRIDE targets *"misconfigured multisigs, weak
access controls, and operational gaps that traditional audits don't cover."*

## 3. The two distinct questions

| | Absolute scanners (Sec3 X-Ray, Auditware Radar, l3x, Octane) | STRIDE / SIRN | **Spectra** |
|---|---|---|---|
| Question | "Is there a missing check in *this snapshot*?" | "Is this deployed protocol's operational posture safe?" | "Did this upgrade PR *remove* a guard the deployed version enforced?" |
| Lifecycle stage | Pre-deploy, single snapshot | Post-deploy + ongoing | CI / PR-merge, differential |
| Notion of "deployed version" | None | The live protocol | The baseline *is* the deployed version |
| Fires on a PR | Yes (absolute findings) | No | Yes (regressions only) |
| Already-missing check | Reported (correctly — its job) | Out of scope | **Silent by construction** |
| Coverage gate | TVL-independent | >$10M TVL (ongoing), >$100M (FV) | TVL-independent, every PR |

**Verdict.** Absolute scanners answer *"is there a hole?"*; STRIDE answers
*"is this protocol's posture safe?"*; Spectra answers *"did this upgrade take
away a guarantee the deployed version already gave its users?"* Three
different questions on three different lifecycle stages. None of the funded
tools answers Spectra's question.

## 4. Lifecycle integration

Spectra occupies the **first** position — the upgrade-time CI gate, before a
program is deployed and long before STRIDE evaluates it:

```
Developer upgrade PR
     ↓
  cargo build
     ↓
  Absolute scanner (Sec3 X-Ray / Auditware Radar / l3x / Octane)  — "any hole in this snapshot?"
     +
  Spectra check --baseline <last on-chain> --candidate <this PR>  ←─── M0 PoC (shipped)
     ↓  (exit 0/1/2)
  CI merge gate
     ↓  (on merge → tag release)
  anchor verify / solana-verifiable-build  (build provenance)
     ↓
  Audit-firm review (e.g. a SIRN-roster firm)
     ↓
  Mainnet deploy
     ↓
  STRIDE operational evaluation  →  ongoing opsec (>$10M TVL)
     ↓
  SIRN incident response (if breached)
```

No tool in the Foundation's recommended free-ecosystem list addresses the
differential / upgrade-time position:

| Free tool | Closest function | Why it doesn't replace Spectra |
|---|---|---|
| Sec3 X-Ray | Absolute static analysis | Single snapshot; no notion of the deployed version; a silently removed guard is not flagged *as a regression* — it competes inside a heuristic FP budget |
| Auditware Radar | Absolute rule templates | Same class as X-Ray; rules apply to one snapshot, not a baseline→candidate delta |
| l3x / Octane | Absolute scanners | Same — stateless, no regression notion |
| Riverguard | Attack simulation / exploit replay | Replays known exploit transactions; does not diff guard sets across versions |
| Hypernative / Range | Runtime threat detection | Post-deployment; cannot fire on a PR |

Spectra is deliberately **silent** on an already-missing check — that is the
absolute scanners' job, and re-implementing it would import the FP-tuning
problem the differential design exists to avoid. The two layers compose.

## 5. Honest detection boundary — the Drift exploit (2026-04-01)

To be evidence-based: **the Drift exploit was not detectable by Spectra, by
any absolute scanner, or by any pre-deployment static analysis.**

Per [Coindesk technical breakdown 2026-04-02](https://www.coindesk.com/tech/2026/04/02/how-a-solana-feature-designed-for-convenience-let-an-attacker-drain-usd270-million-from-drift)
and [Elliptic 2026-04](https://www.elliptic.co/blog/drift-protocol-exploited-for-286-million-in-suspected-dprk-linked-attack),
the root causes were:

1. Six-month social-engineering operation (DPRK-linked) posing as a
   quantitative trading firm
2. Compromised signing devices via a malicious TestFlight app + an
   editor/IDE vulnerability
3. Two misleading approvals on the 5-member Security Council multisig,
   producing pre-signed transactions valid for >1 week
4. Zero-timelock Security Council migration eliminating the last defensive
   layer
5. Durable-nonce abuse to execute the pre-signed transactions weeks later
6. Fictitious collateral asset with wash-traded liquidity accepted by oracles

**None of these are account-validation guard regressions.** Spectra would not
have produced a single finding on the upgrade path, because the upgrade path
was not the attack surface. STRIDE pillars 1, 2, 3, 6 (operational security,
access controls, multisig configurations, key management) are the relevant
defensive layer.

This is precisely why Spectra is **complementary, not competitive**, to
STRIDE and to the absolute scanners. The Drift exploit is a STRIDE-pillar
problem. The class Spectra detects — an upgrade PR silently removing an
owner / signer / type / `has_one` / PDA / constraint / CPI guard the deployed
version enforced — is a different attack surface, and access-control /
account-validation failure is the single largest *quantified* loss class on
Solana.

## 6. Why the gap matters

A protocol can pass STRIDE evaluation and an absolute scan and still ship a
v_next that silently downgrades `Account<'info, Vault>` to `UncheckedAccount`
in one refactor commit — with no compiler error, because Anchor enforces
guards at runtime. STRIDE evaluates *the protocol's posture*; absolute
scanners evaluate *one snapshot*; neither gates *every upgrade PR on whether
it regresses a guarantee relative to what is deployed*. Spectra runs free, on
every PR, regardless of TVL, with zero infrastructure cost beyond GitHub
Actions minutes, and is silent unless a guarantee was removed.

## 7. Implication for grant positioning

1. Position as CI-gate infrastructure **complementary to** STRIDE and the
   absolute scanners — not a security-audit or absolute-scanner substitute.
2. Acknowledge explicitly that Spectra would not have caught the Drift
   exploit, and explain why (different attack surface).
3. State the boundary honestly: already-missing checks are out of scope by
   construction; that is the absolute scanners' job.
4. Pursue an LOI from at least one SIRN-roster firm (Asymmetric Research,
   OtterSec, Neodyme, Squads, ZeroShadow) to convert the complementary-not-
   competitive claim from self-asserted to externally-validated.
5. Submit under the standard "public good" criterion on
   [solana.org/grants-funding](https://solana.org/grants-funding) — the four
   published criteria (Public Good / Open Source / Solana-Specific / Clear
   Fund Usage) all apply.

## 8. References

- [solana.com/news/solana-ecosystem-security](https://solana.com/news/solana-ecosystem-security) — Foundation security strategy (2026-04)
- [Asymmetric STRIDE introduction blog](https://blog.asymmetric.re/introducing-stride-a-security-program-for-the-solana-ecosystem/)
- [Coindesk 2026-04-07 — Foundation security overhaul](https://www.coindesk.com/tech/2026/04/07/solana-foundation-unveils-security-overhaul-days-after-usd270-million-drift-exploit)
- [Coindesk 2026-04-02 — Drift exploit technical breakdown](https://www.coindesk.com/tech/2026/04/02/how-a-solana-feature-designed-for-convenience-let-an-attacker-drain-usd270-million-from-drift)
- [Elliptic 2026-04 — Drift Protocol $286M exploit](https://www.elliptic.co/blog/drift-protocol-exploited-for-286-million-in-suspected-dprk-linked-attack)
- [solana.org/grants-funding](https://solana.org/grants-funding) — current grant criteria
