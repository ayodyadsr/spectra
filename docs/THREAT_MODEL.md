# Threat Model

This document is the explicit threat model for Spectra. It enumerates the adversary classes Spectra is designed to help against, the trust assumptions Spectra makes, and the failure modes (soundness / completeness / robustness) under which Spectra may produce wrong answers.

Spectra is **deterministic structural-compatibility analysis**, not formal verification and not runtime monitoring. Every claim below is bounded accordingly.

---

## 1. Adversary classes

| ID | Class | Description | Spectra's role |
|----|-------|-------------|---------------|
| A1 | Honest-but-rushed maintainer | A protocol engineer ships an upgrade that unintentionally breaks Borsh layout, removes an instruction, widens an argument, or reorders an account field. | **Primary target.** Spectra fails CI before the upgrade reaches mainnet. |
| A2 | Inattentive reviewer | The PR is reviewed but the layout-breaking change is missed visually (e.g. a one-line field reorder buried in a 600-line diff). | **Primary target.** Spectra produces a deterministic, structured finding the reviewer cannot overlook. |
| A3 | Malicious maintainer with merge access | An insider deliberately ships a layout-changing upgrade designed to corrupt existing on-chain accounts in their favour. | **Partial target.** Spectra surfaces the change, but cannot prevent merge if the same actor can also disable CI. Defence depends on branch-protection rules outside Spectra. |
| A4 | External PR submitter (no merge access) | A non-maintainer opens a PR introducing a silent-corruption change. | **Primary target.** Spectra's job is to make the change visible during review. |
| A5 | Build-supply-chain attacker | An attacker substitutes the binary or the IDL between source and deployment. | **Out of scope.** Build-provenance tools (`solana-verifiable-build` and its `solana-verify` binary) cover this layer. Spectra explicitly does not. |

Spectra is most useful against A1, A2, A4 (unintentional regressions and inattentive review). It is partially useful against A3 (visibility) and explicitly does **not** replace A5's tooling (build provenance).

---

## 2. Trust assumptions

Spectra **trusts** the following inputs as authoritative:

- The two IDL JSON files supplied via `--old` and `--new`. Spectra makes no claim that these IDLs accurately describe the deployed `.so`; that is the job of `solana-verifiable-build` / `anchor verify`.
- The Anchor legacy IDL schema's documented discriminator algorithm: `sha256("global:<name>")[..8]` for instructions, `sha256("account:<name>")[..8]` for accounts.
- The Borsh serialization rules (positional, no padding) for the layout-diff finding kinds.

Spectra makes **no** assumption about:

- Network state, slot height, clock, randomness.
- The reproducibility of the build that produced the deployed program.
- The honesty of the actor who supplied the IDL.

If the supplied IDL is forged or stripped, Spectra's findings remain internally consistent for that IDL pair but may not reflect the truly deployed program. This is an explicit limitation, not a bug.

---

## 3. Failure modes

Spectra has three orthogonal failure modes. Each is documented with the conditions under which it can occur and the user-visible signal that flags it.

### 3.1 Soundness failures (false positives)

A soundness failure is a BREAKING finding that does not in fact break upgrade compatibility.

| Cause | Example | Mitigation |
|-------|---------|------------|
| Cosmetic rename of a field that the protocol's own client code already adapts to | Renaming `total_supply` -> `totalSupply` while updating all callers in lockstep | `spectra-allow.toml` suppression with rationale + expiry (M3) |
| Pre-launch program with no on-chain accounts yet | Field reorder is meaningless because nothing depends on it | Manual review; suppression with `pre_launch = true` rationale |
| Explicit migration where the protocol code resets affected accounts | Anchor 0.30+ explicit discriminator override paired with an `init` re-set | `migration_declared = "..."` marker in `spectra-allow.toml` (M3) |

Spectra's documented design target is **a false-positive rate low enough for default-on CI gating**, with the suppression file (M3) as the mandatory escape valve. Concrete FP-rate measurement is part of M4 (≥1 pilot + 2 walkthroughs).

### 3.2 Completeness failures (false negatives)

A completeness failure is a real upgrade hazard Spectra does **not** detect. Every known false-negative class is documented in [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md). Highlights:

- Zero-copy (`bytemuck`, `#[repr(C)]`) padding-aware layout changes — IDL does not surface padding.
- Token-2022 TLV extensions — IDL does not represent TLV.
- PDA seed-derivation drift — requires BPF disassembly; explicitly Future Expansion, not M0–M3.
- Cross-program invocation contract changes — Spectra sees one program at a time.
- Constant / `.rodata` changes (e.g. switching a fee constant from 30 bps to 300 bps) — out of scope for M0–M3.

Each of these is listed in the README's "What Spectra does NOT do" section so reviewers cannot mistake silence for coverage.

### 3.3 Robustness failures (malformed input)

Spectra refuses to analyse rather than producing a misleading clean report.

- Unparseable IDL JSON -> exit code `2` (invocation error).
- Unknown IDL schema version (neither Anchor legacy nor a future supported schema) -> exit code `3` (refuse to analyse). M0 only supports legacy; M1 adds Anchor 2026 / Codama.
- Conflicting discriminator override + algorithmic discriminator -> finding `R-DISC-OVERRIDE-CONFLICT` (M1).

The exit-code contract is the user's guarantee that "clean report + exit 0" never silently hides "I did not understand your input."

---

## 4. Out-of-scope adversary capabilities

Spectra is **not** designed to defend against, and makes **no** claim about:

- Compiler bugs (LLVM / SBF backend miscompilation).
- Validator-level consensus exploits.
- Wallet UX phishing.
- Off-chain governance / multisig signer compromise.
- Frontend / client-side substitution attacks.
- Token-2022 transfer-hook reentrancy (separate detector pack, listed in proposal's Future Expansion).

These are explicitly named here so reviewers can verify Spectra is not over-claiming scope.

---

## 5. Cross-references

- Severity classification rules per finding kind: [SEVERITY.md](SEVERITY.md)
- Edge-case coverage matrix: [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md)
- False-positive mitigation strategy: [FALSE_POSITIVES.md](FALSE_POSITIVES.md)
- Migration-awareness mechanism: [MIGRATION.md](MIGRATION.md)
