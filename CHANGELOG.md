# Changelog

All notable changes to Spectra are documented in this file.

The format is based on [Keep a Changelog 1.1.0](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html)
from the first tagged release onward.

The pre-1.0 line documents the M0 PoC milestones shipped before the Solana
Foundation grant submission. Future tagged releases (`v0.3.0-m1`,
`v0.4.0-m2`, …) each get their own section here.

## [Unreleased]

### Changed
- **Total pivot to a strictly-differential account-validation
  security-regression gate.** Spectra now parses two Anchor **source trees**
  (a *baseline* = last released / on-chain-deployed version, and a
  *candidate* = the upgrade PR) and emits a finding **only** when the
  candidate removes, downgrades, or bypasses an account-validation guard the
  baseline enforced. The prior IDL-diff scope (discriminator drift, Borsh
  layout, account-field reorder) has been removed in full — it was not a
  top-quantified Solana loss class and is a different problem; Spectra reads
  Rust source, not IDL JSON.
- Engine rewritten: `accounts.rs` (`syn`-based guard extractor with a
  tolerant `#[account(...)]` token-walk parser), `regression.rs`
  (strictly-differential differ with downgrade-vs-equivalent-pin logic),
  `report.rs` (JSON / Markdown / SARIF 2.1.0).
- Exit-code contract simplified to `0` clean / `1` BREAKING / `2` invocation
  error. **Exit code 3 (refuse-to-analyse) removed** — un-processable input
  is exit `2`; a file that does not parse as Rust is skipped, not fatal.
- Finding set replaced with 9 account-validation kinds:
  `signer_check_removed`, `type_cosplay_protection_removed`,
  `owner_check_removed`, `has_one_constraint_removed`,
  `custom_constraint_removed`, `pda_derivation_removed`,
  `cpi_target_unpinned`, `validated_account_slot_removed` (BREAKING) +
  `unvalidated_account_introduced` (warning).
- Synthetic fixture replaced: `examples/vault_baseline` →
  `examples/vault_candidate` (6 BREAKING + 1 warning, exit 1).
- Test suite replaced with 6 integration tests including the
  strictly-differential no-false-positive property (an unchanged context
  inside an otherwise-changed program yields zero findings).
- CI, `Dockerfile`, `spectra-action`, Python wrapper, `Makefile`, and
  `scripts/record-demo.sh` migrated to `--baseline` / `--candidate` source
  trees.
- All engineering docs (`TECHNICAL_SPEC.md`, `docs/STRIDE_GAP_ANALYSIS.md`,
  `docs/SEVERITY.md`, `docs/THREAT_MODEL.md`, `docs/ARCHITECTURE.md`,
  `docs/NON_GOALS.md`, `docs/FALSE_POSITIVES.md`, `docs/CI_INTEGRATION.md`,
  `docs/ROADMAP.md`, `docs/TESTING.md`) rewritten to the differential scope.
- Positioning sharpened: Spectra is **complementary to** the
  Foundation-recommended absolute scanners (Sec3 X-Ray, Auditware Radar,
  l3x, Octane), silent by construction on already-missing checks, with
  near-zero false positives by construction rather than by heuristic tuning.

### Removed
- IDL-diff documentation that is no longer in scope (the prior real-world
  Drift IDL benchmark and the generic-JSON-diff competitive benchmark — both
  belonged to the dead IDL-diff scope).

### Notes
- Real-world detection / performance numbers are **not** asserted at M0:
  `[NO PUBLIC DATA AVAILABLE]` until the explicit M1.5 benchmark against a
  real public Anchor program's deployed-vs-upgrade source pair.

## [0.2.0-m0] — pre-grant M0 PoC

Public at https://github.com/ayodyadsr/spectra. Apache-2.0-licensed Rust
workspace (`spectra-core` + `spectra` CLI), Python wrapper (`spectra-cli`),
composite Action scaffold (`spectra-action`). Strictly-differential
account-validation regression engine over Anchor `#[derive(Accounts)]`
source; 9 finding kinds; JSON / Markdown / SARIF 2.1.0; exit `0`/`1`/`2`;
6 integration tests; synthetic baseline → candidate fixture; green CI on
every push (`fmt` + `clippy -D warnings` + build + test + end-to-end CLI
gates + Docker smoke).

[Unreleased]: https://github.com/ayodyadsr/spectra/compare/v0.2.0-m0...HEAD
[0.2.0-m0]: https://github.com/ayodyadsr/spectra/releases/tag/v0.2.0-m0
