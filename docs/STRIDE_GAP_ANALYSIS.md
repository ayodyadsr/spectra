# Spectra vs STRIDE — Gap Analysis (2026-05-16)

**Purpose.** Position Spectra in the Solana Foundation's post-April-2026 security landscape. After the $286M Drift exploit (2026-04-01) and the Foundation's response (STRIDE + SIRN launch, 2026-04-07), the funded security stack has consolidated. This document maps what STRIDE covers, what it does not cover, and where Spectra fits as **complementary** infrastructure — not competing.

## 1. The current Solana security stack (2026-05-16)

Per [solana.com/news/solana-ecosystem-security](https://solana.com/news/solana-ecosystem-security) and [Asymmetric Research's STRIDE announcement](https://blog.asymmetric.re/introducing-stride-a-security-program-for-the-solana-ecosystem/):

| Layer | Tool / Program | Lifecycle stage | Funded by |
|---|---|---|---|
| Post-deployment evaluation | **STRIDE** (Asymmetric Research) | Operational | Solana Foundation grants |
| Post-deployment monitoring | STRIDE ongoing opsec | Runtime, >$10M TVL | Solana Foundation grants |
| Formal verification | STRIDE FV tier | Mathematical proof, >$100M TVL | Solana Foundation grants |
| Incident response | **SIRN** (Asymmetric + OtterSec + Neodyme + Squads + ZeroShadow) | Active incident | Membership / Foundation |
| Free ecosystem — threat detection | Hypernative | Runtime | Free tier |
| Free ecosystem — risk monitoring | Range Security | Runtime | Free tier |
| Free ecosystem — attack simulation | Riverguard by Neodyme | Pre-deployment, exploit replay | Free tier |
| Free ecosystem — static analysis | Sec3 X-Ray | Pre-deployment, source-level | Free tier |
| Free ecosystem — rule templates | AuditWare Radar | Pre-deployment, custom rules | Free tier |
| **CI-gate behavioural regression** | **(no funded tool)** | **CI / PR-merge, upgrade-time** | **GAP** |

## 2. STRIDE's 8 pillars (named)

Per [Asymmetric Research blog](https://blog.asymmetric.re/introducing-stride-a-security-program-for-the-solana-ecosystem/) and [Coindesk 2026-04-07](https://www.coindesk.com/tech/2026/04/07/solana-foundation-unveils-security-overhaul-days-after-usd270-million-drift-exploit), STRIDE evaluates protocols across eight pillars. Seven are publicly named in text:

1. Operational security
2. Access controls
3. Multisig configurations
4. Governance vulnerabilities
5. Smart contract integrity
6. Key management practices
7. Economic design
8. [NO PUBLIC TEXT AVAILABLE — listed only in the "8-Pillars-of-STRIDE" image at blog.asymmetric.re]

The verbatim Asymmetric framing: STRIDE targets *"misconfigured multisigs, weak access controls, and operational gaps that traditional audits don't cover."*

## 3. What STRIDE does — and does not — cover

| Concern | STRIDE coverage | Reasoning |
|---|---|---|
| Multisig misconfiguration | ✅ Pillar 3 | Explicit |
| Access control review | ✅ Pillar 2 | Explicit |
| Operational security (key custody, signing devices) | ✅ Pillar 1 + 6 | Explicit |
| Governance attack surface | ✅ Pillar 4 | Explicit |
| Smart-contract code review (audit-firm-level) | ✅ Pillar 5 | "Smart contract integrity" — audit-firm scope |
| Economic / oracle / collateral design | ✅ Pillar 7 | Explicit |
| Formal verification (proof-based correctness) | ✅ Tier add-on | Only for >$100M TVL |
| Runtime threat monitoring (24/7) | ✅ Ongoing opsec | Only for >$10M TVL post-pass |
| **Program-upgrade behavioural-regression diffing** | **❌ Out of scope** | STRIDE is point-in-time + ongoing monitoring, not a CI gate that fires on every PR |
| **IDL / account-layout silent corruption between upgrades** | **❌ Out of scope** | Not mentioned in any STRIDE source |
| **Discriminator collision detection at upgrade time** | **❌ Out of scope** | Not mentioned in any STRIDE source |
| **Anchor SHA-256 discriminator drift on instruction rename** | **❌ Out of scope** | Not mentioned in any STRIDE source |
| **CI-gateable severity-tiered exit codes for `cargo build`/PR CI** | **❌ Out of scope** | STRIDE evaluates protocols, not commits |

**Verdict.** STRIDE answers: *is this protocol's operational posture safe to use?* It does not answer: *did this PR introduce a silent behavioural regression in the on-chain program's interface?* Those are different questions on different lifecycle stages.

## 4. Spectra's positioning (revised)

Spectra is the **CI-time / pre-deployment / upgrade-time regression-detection layer**. It runs on every PR before a program is deployed, gates merges on severity-tiered exit codes, and produces SARIF for GitHub Code Scanning. It is not an alternative to STRIDE — it is the gate that fires *before* a STRIDE-evaluated protocol ships an upgrade.

**Lifecycle integration:**

```
Developer PR
     ↓
  cargo build
     ↓
  Spectra diff old.idl new.idl  ←─── M0 PoC (shipped 2026-05-14)
     ↓ (exit 0/1/2/3)
  CI merge gate
     ↓ (on merge → tag release)
  solana-verifiable-build (build provenance)
     ↓
  Audit firm review (e.g. STRIDE member firm)
     ↓
  Mainnet deploy
     ↓
  STRIDE operational evaluation
     ↓
  STRIDE ongoing opsec (if >$10M TVL)
     ↓
  SIRN incident response (if breached)
```

Spectra occupies the **first** position — the upgrade-time CI gate. No tool in the Foundation's recommended free-ecosystem list addresses this position:

| Free tool | Closest function | Why it doesn't replace Spectra |
|---|---|---|
| Sec3 X-Ray | Static analysis on source | Source-level pattern matching, not IDL-pair diffing across versions; doesn't catch discriminator drift between deployed and proposed |
| AuditWare Radar | Custom rule templates | Same static-analysis class as X-Ray; rules apply to one snapshot, not a delta |
| Riverguard | Attack simulation / exploit replay | Replays known exploit transactions; does not diff IDLs or detect silent account-layout corruption |
| Hypernative / Range | Runtime threat detection | Post-deployment; cannot fire on a PR |

## 5. Honest detection boundary — the Drift exploit (2026-04-01)

The proposal benchmarks Spectra against Drift Protocol IDL v2.155 → v2.162 (commit `590049e6bf` → `0d35029d78`). To be evidence-based: **the Drift exploit was not detectable by Spectra, by any IDL-diff tool, or by any pre-deployment static analysis.**

Per [Coindesk technical breakdown 2026-04-02](https://www.coindesk.com/tech/2026/04/02/how-a-solana-feature-designed-for-convenience-let-an-attacker-drain-usd270-million-from-drift) and [Elliptic 2026-04](https://www.elliptic.co/blog/drift-protocol-exploited-for-286-million-in-suspected-dprk-linked-attack), the Drift exploit root causes were:

1. **Six-month social-engineering operation** (UNC4736 / AppleJeus / Citrine Sleet) posing as a quantitative trading firm
2. **Compromised signing devices** via malicious TestFlight app + VSCode/Cursor vulnerability
3. **Two misleading approvals on the 5-member Security Council multisig**, producing pre-signed transactions valid for >1 week
4. **Zero-timelock Security Council migration** eliminating the last defensive layer
5. **Durable nonce abuse** to execute pre-signed transactions weeks later
6. **Fictitious collateral asset** (CarbonVote Token) with wash-traded liquidity, accepted by Drift oracles as worth hundreds of millions

**None of these are IDL changes, discriminator drifts, or account-layout regressions.** Spectra would not have produced a single finding on the upgrade path leading up to the exploit, because the upgrade path was not the attack surface. STRIDE pillars 1, 2, 3, 6 (operational security, access controls, multisig configurations, key management practices) are the relevant defensive layer.

This is why Spectra is **complementary, not competitive**, to STRIDE. The Drift exploit is a STRIDE-pillar problem. The class of problem Spectra detects — silent account-layout corruption between upgrades, discriminator collisions, instruction-arg widening that breaks downstream integrations — is a different attack surface that has produced separate incidents in Solana history (see `docs/PAPER.md §3` for the taxonomy).

## 6. Why the gap matters

A protocol can pass STRIDE evaluation and still ship a silently-corrupted PerpMarket account layout in v2.163. STRIDE evaluates *the protocol's posture*; it does not gate *every program upgrade PR*. The Foundation funds STRIDE for protocols >$10M TVL post-pass. Spectra runs free, on every PR, regardless of TVL, with zero infrastructure cost beyond GitHub Actions minutes.

## 7. Implication for grant positioning

Spectra's grant application should:

1. **Drop** any reference to the 2024 forum.solana.com program-verification-tooling RFP (closed 2024-02-29, no longer relevant)
2. **Position** as CI-gate infrastructure complementary to STRIDE — not as a security audit substitute
3. **Acknowledge explicitly** that Spectra would not have caught the Drift exploit and explain why (different attack surface)
4. **Pursue LOI** from at least one SIRN-roster firm (Asymmetric Research, OtterSec, Neodyme, Squads, ZeroShadow) to validate the integration narrative — without an LOI from this roster, the complementary-not-competitive claim is unsupported
5. **Submit under** the standard "public good" criterion on [solana.org/grants-funding](https://solana.org/grants-funding) — security tooling is not an explicit category but the four published criteria (Public Good / Open Source / Solana-Specific / Clear Fund Usage) all apply

## 8. References

- [solana.com/news/solana-ecosystem-security](https://solana.com/news/solana-ecosystem-security) — Foundation's official security strategy page (2026-04)
- [Asymmetric STRIDE introduction blog](https://blog.asymmetric.re/introducing-stride-a-security-program-for-the-solana-ecosystem/)
- [Coindesk 2026-04-07 — Foundation security overhaul](https://www.coindesk.com/tech/2026/04/07/solana-foundation-unveils-security-overhaul-days-after-usd270-million-drift-exploit)
- [Coindesk 2026-04-02 — Drift exploit technical breakdown](https://www.coindesk.com/tech/2026/04/02/how-a-solana-feature-designed-for-convenience-let-an-attacker-drain-usd270-million-from-drift)
- [Elliptic 2026-04 — Drift Protocol $286M exploit](https://www.elliptic.co/blog/drift-protocol-exploited-for-286-million-in-suspected-dprk-linked-attack)
- [solana.org/grants-funding](https://solana.org/grants-funding) — current grant criteria
- [forum.solana.com/t/program-verification-tooling/1032](https://forum.solana.com/t/program-verification-tooling/1032) — original RFP (CLOSED 2024-02-29, listed here only as historical record)
