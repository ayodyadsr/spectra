# Solana Edge-Case Coverage

This document is the explicit edge-case matrix for Spectra. For every Solana-specific concern a reviewer might raise, the table below states whether M0 covers it today, the milestone at which it lands, or whether it is permanently out of scope.

The matrix is intentionally honest about non-coverage so reviewers can judge Spectra by what it actually does.

---

## 1. Coverage matrix

| Concern | M0 (today) | Path forward | Notes |
|---------|------------|--------------|-------|
| Anchor legacy IDL discriminator drift (rename / explicit override) | ✅ via rename detection + R-ACC-SILENT-CORRUPT | — | `sha256("global:<name>")[..8]`, `sha256("account:<name>")[..8]`. |
| Anchor 0.30+ explicit `#[instruction(discriminator = "...")]` overrides | ❌ | M1 (R-DISC-OVERRIDE-CONFLICT) | Will compare algorithmic vs declared. |
| Anchor 2026 schema (Codama-aligned) | ❌ | M1 | Schema dispatcher adds the parser. |
| Shank-generated native-program IDL | ❌ | M1 | Native programs not covered until M1. |
| Borsh layout change in Anchor account struct | ✅ via R-ACC-FIELD-REORDER / R-ACC-FIELD-TYPE / R-ACC-FIELD-REM | — | Positional encoding, no padding. |
| Borsh layout change in instruction args | ✅ via R-INS-ARG | — | |
| Silent-corruption (account name unchanged, layout changed) | ✅ via R-ACC-SILENT-CORRUPT | — | Surfaced as its own finding kind. |
| Discriminator collision across IDL names | ✅ via R-DISC-COLL | — | Truncated 8-byte SHA-256 collision check. |
| Enum variant insertion / removal (Borsh tag renumbering) | ❌ | M1 (R-ENUM-VAR-INSERT / R-ENUM-VAR-REM) | Borsh enum tag is positional. |
| Shared `defined_type` layout change | ❌ | M1 (R-DEFINED-TYPE-CHANGED) | One struct, many call sites. |
| Event field reorder / type change | ❌ | M1 (R-EVENT-*) | Breaks off-chain indexers. |
| Error code renumbering | ❌ | M1 (R-ERROR-CODE-CHANGED) | Breaks client error branches. |
| Zero-copy account (`#[repr(C)]` + `bytemuck`) padding-aware diff | ❌ | M1 partial (size-only); full padding-aware analysis is research | IDL does not surface explicit padding. |
| Token-2022 TLV extension layouts | ❌ permanent in Spectra | Separate detector pack (see Future Expansion in proposal) | IDL cannot represent TLV. |
| Token-2022 transfer-hook reentrancy / authority drift | ❌ permanent in Spectra | Separate detector pack | Out of scope by design. |
| PDA seed-derivation drift | ❌ | Future Expansion (BPF disassembly) | Not promised in this grant. |
| Program-derived address bump-seed search semantics | ❌ | Future Expansion | Same as above. |
| Constant / `.rodata` change (e.g. fee `30 -> 300` bps) | ❌ | Future Expansion | Not visible in IDL. |
| Loader v3 program-data-account format | ✅ implicit (today's default) | — | |
| Loader v4 program-data-account format | ❌ | M1 contingency (budget line included) | Activates if v4 mainnet-lands during grant. |
| SBPF version drift (v1/v2/v3) | ❌ | M1 partial | Reported as informational. |
| Cross-program invocation (CPI) signature stability | ❌ at static level | M2 via corpus replay (R-REPLAY-CPI-FAIL) | One program at a time at the static layer. |
| Upgrade-authority transfer | ❌ permanent | — | Off-chain operational action, not visible to a static diff. |
| Mainnet snapshot replay | ❌ permanent | — | Infeasible in free-tier CI; M2 uses bounded corpus instead. |
| Compiler / LLVM bug | ❌ permanent | — | Out of threat model. |

---

## 2. Why some items are permanently out of scope

These are explicitly named so reviewers can confirm Spectra is **not** silently claiming to cover them:

- **Token-2022 TLV layouts:** Anchor / Shank IDL does not describe TLV records. A correct detector requires Token-2022-specific extension knowledge and belongs in a separate detector pack.
- **PDA seed-derivation drift:** Requires reading actual BPF instructions; this is research-grade and not promised in this grant.
- **Constant / `.rodata` semantic changes:** A fee constant change is invisible to IDL-only diff. Source-level review is the right layer.
- **Upgrade-authority transfer / governance signer changes:** These are off-chain operational events, not source-level changes.
- **Mainnet snapshot replay:** Free-tier GitHub runners have ~14 GB disk and a 6 h wall-clock cap. A non-trivial mainnet snapshot does not fit; M2 uses a hand-curated bounded corpus instead. This is a deliberate design choice, not a missing feature.

---

## 3. Solana version compatibility

| Concern | Status |
|---------|--------|
| Anchor 0.29 and earlier (legacy IDL) | ✅ M0 |
| Anchor 0.30+ explicit discriminator | ❌ today; ✅ M1 |
| Anchor 2026 / Codama | ❌ today; ✅ M1 |
| Shank-generated native IDL | ❌ today; ✅ M1 |
| Loader v3 | ✅ M0 |
| Loader v4 | ❌ today; ✅ M1 contingency (budget line) |
| Solana SDK version (host of toolchain) | M0 has no SDK dependency at all — pure JSON in / report out |

---

## 4. Cross-references

- Threat model: [THREAT_MODEL.md](THREAT_MODEL.md)
- Per-finding severity: [SEVERITY.md](SEVERITY.md)
- False-positive mitigation: [FALSE_POSITIVES.md](FALSE_POSITIVES.md)
- Migration declarations: [MIGRATION.md](MIGRATION.md)
- Anchor-specific notes: [ANCHOR.md](ANCHOR.md)
