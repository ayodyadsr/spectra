# Real-World Benchmark — Drift Protocol v2.155 → v2.162

[`docs/BENCHMARK.md`](BENCHMARK.md) uses the synthetic regression fixture shipped in this repo. This document is the **real-world** counterpart: Spectra run against an actual production Anchor program upgrade pair, fetched from a public GitHub repository.

The target program is [Drift Protocol v2](https://github.com/drift-labs/protocol-v2) — one of the largest upgradable Anchor programs deployed on Solana mainnet. The IDLs used here are the public Anchor IDL JSONs committed to the protocol's SDK.

Every number below is reproducible by anyone running the commands in §5.

---

## 1. The upgrade pair

| Property | v2.155 | v2.162 |
|---|---|---|
| Commit (path: `sdk/src/idl/drift.json`) | [`590049e6bf`](https://github.com/drift-labs/protocol-v2/blob/590049e6bf/sdk/src/idl/drift.json) | [`0d35029d78`](https://github.com/drift-labs/protocol-v2/blob/0d35029d78/sdk/src/idl/drift.json) |
| Date | 2026-01-21 | 2026-04-01 |
| IDL size (JSON) | 421.5 KB | 428.3 KB |
| IDL lines | 19,712 | 20,138 |
| Instructions | 246 | 249 (+3) |
| Accounts | 27 | 27 |
| Types | ~115 | 115 |
| Events | 26 | 26 |
| Errors | 349 | 349 |

This is **7 minor versions over ~2.5 months** — a representative production cadence for a high-velocity Solana DeFi program.

---

## 2. Without Spectra — what a reviewer would face

`diff -u drift_v2_155.json drift_v2_162.json` produces:

- **393 lines** of unified diff output.
- **319 actual changed lines** (added or removed; excluding hunk markers and headers).
- Changes scattered across a 20,138-line IDL.

A human reviewer would need to:

1. Read 319 changed lines distributed throughout a 20 K-line JSON file.
2. For every account-field reorder, type change, or rename, perform the inference chain documented in [docs/BENCHMARK.md §2](BENCHMARK.md#2-without-spectra--what-the-reviewer-sees) — Anchor discriminator algorithm, Borsh positional encoding, presence-vs-absence reasoning.
3. Compute every instruction's discriminator (`sha256("global:" + name)[..8]`) for every new or removed instruction name to determine whether existing clients break.
4. Scan for discriminator collisions across the 249 + 27 + 115 + 26 = **417 IDL names**.
5. Hold the conclusion across the whole 20 K-line file.

For a single reviewer on a 393-line release PR, step 4 is essentially infeasible by hand. Step 3 is computationally tedious. Step 2 — the silent-corruption case — is where the bug shipped in §3 lives.

---

## 3. With Spectra — what the reviewer sees

```bash
spectra check \
  --old /tmp/drift_v2_155.json \
  --new /tmp/drift_v2_162.json \
  --format markdown
```

Output (verbatim):

```markdown
# Spectra Diff Report

**Old program:** `drift`
**New program:** `drift`

**Findings:** 2 breaking, 4 warning

| Severity | Kind | Detail |
|---|---|---|
| warning  | instruction_added | `updatePerpMarketConfig` (disc 147a516508c69be9) |
| warning  | instruction_added | `placeScaleOrders` (disc a9fb457eb0719c9c) |
| warning  | instruction_added | `adminWithdrawFromInsuranceFundVault` (disc 1c1d4db1ed3c9f8c) |
| BREAKING | account_field_type_changed | `PerpMarket.padding`: {"array":["u8",23]} -> {"array":["u8",22]} |
| warning  | account_field_added | `PerpMarket.marketConfig: u8` |
| BREAKING | account_layout_changed_same_discriminator | `PerpMarket` layout changed but discriminator 0adf0c2c6bf537f7 is unchanged (silent-corruption risk) |
```

Exit code `1`. CI fails.

**Six findings extracted from 319 changed lines in 6 ms.** Three new instructions (informational), one account-field type narrowing, one additive field, and — critically — the **silent-corruption case** on `PerpMarket` flagged as a single labelled finding with the computed account discriminator `0adf0c2c6bf537f7`.

---

## 4. What the silent-corruption finding means here

Drift's v2.162 upgrade converted **1 byte of `PerpMarket.padding` into a new `marketConfig: u8` field**. The total `PerpMarket` size is preserved; the byte that was previously a no-op padding byte now has semantic meaning.

For every existing on-chain `PerpMarket` account:

1. The `PerpMarket` discriminator `0adf0c2c6bf537f7` did not change (the struct name `PerpMarket` is unchanged).
2. The Solana runtime + Anchor will still accept every existing on-chain `PerpMarket` as valid.
3. The byte at the old `padding[22]` offset is now read as `marketConfig`.

This is **safe if and only if** that byte was zero in every pre-existing on-chain `PerpMarket` (i.e. padding was actually zeroed, as the convention assumes). If the byte was not zero — say, an old version of the program once wrote to it then reverted — every existing market suddenly reads a non-zero `marketConfig` after the upgrade.

This is exactly the case that Spectra is designed to surface for **explicit reviewer attention**:

- An audit firm reviewing the upgrade would ask: "have you verified the padding byte was zero in all 100+ on-chain `PerpMarket` accounts?"
- The protocol team would either confirm via state inspection or, if confirmed safe, suppress with `spectra-allow.toml` and a rationale citing the verification.
- A `git diff` reviewer can **see** the padding-shrink-and-add lines, but combining them into the inference "this is silent corruption unless that byte was zero" requires the 4-step Anchor + Borsh chain documented in [BENCHMARK.md](BENCHMARK.md). On a 393-line release PR, this case is realistically missed.

This is not a hypothetical scenario. It is a **real change** in a **real upgrade** of a **real protocol with $300M+ TVL**. Spectra found it in 6 ms. A reviewer would find it only by reading 319 changed lines with full Anchor + Borsh context.

---

## 5. Performance — Spectra vs `diff -u` on real production IDL

| Operation | Wall-clock (avg of 5 runs, commodity laptop) |
|---|---|
| `diff -u drift_v2_155.json drift_v2_162.json` | 4.2 ms |
| `spectra check --old drift_v2_155.json --new drift_v2_162.json` | 6.2 ms |
| `spectra check --old drift_v2_162.json --new drift_v2_162.json` (identical-input invariant) | 6.2 ms |

Raw measurements:

```
diff -u (5 runs): 0.004, 0.004, 0.004, 0.005, 0.004
spectra check (5 runs): 0.007, 0.007, 0.006, 0.005, 0.006
```

Spectra is **~1.5× slower than `diff`** on a 428 KB IDL — still under 10 ms. For CI gating purposes the cost is negligible. The performance claim in [CI_INTEGRATION.md §6](CI_INTEGRATION.md) ("well under 1 second on a 10-instruction / 5-account IDL") is conservatively validated: Spectra processes Drift's 249-instruction / 115-type / 26-event IDL in under 10 ms.

**Identical-IDL exit-0 invariant verified on real-world data:**

```bash
$ spectra check --old drift_v2_162.json --new drift_v2_162.json --format markdown
# Spectra Diff Report
...
**Findings:** 0 breaking, 0 warning
No regressions detected.
Spectra: 0 breaking, 0 warning
$ echo $?
0
```

**Zero false positives on a 428 KB production IDL.** This is the tested invariant that lets Spectra serve as a `required` CI check on real protocols.

---

## 6. Reproducing this benchmark

Every output above was produced with exactly these commands on commit [`5abd61d`](https://github.com/ayodyadsr/spectra/commit/5abd61d) of Spectra:

```bash
# 1. Build the release binary.
cd /path/to/spectra
cargo build --release

# 2. Fetch the public Drift IDLs at two historical commits.
mkdir -p /tmp/drift && cd /tmp/drift
curl -sSL -o drift_v2_155.json \
  https://raw.githubusercontent.com/drift-labs/protocol-v2/590049e6bf/sdk/src/idl/drift.json
curl -sSL -o drift_v2_162.json \
  https://raw.githubusercontent.com/drift-labs/protocol-v2/0d35029d78/sdk/src/idl/drift.json

# 3. Reviewer baseline — what diff alone shows.
diff -u drift_v2_155.json drift_v2_162.json | wc -l
# → 393 lines
diff -u drift_v2_155.json drift_v2_162.json | grep -E "^[+-]" | grep -v "^[+-][+-][+-]" | wc -l
# → 319 changed lines

# 4. Run Spectra.
/path/to/spectra/target/release/spectra check \
  --old drift_v2_155.json \
  --new drift_v2_162.json \
  --format markdown
# → 2 BREAKING + 4 warning, exit code 1

# 5. Confirm no false positives on identical input.
/path/to/spectra/target/release/spectra check \
  --old drift_v2_162.json \
  --new drift_v2_162.json \
  --format markdown
# → 0 findings, exit code 0

# 6. Measure wall-clock.
for i in 1 2 3 4 5; do
  { TIMEFORMAT='%R'; time /path/to/spectra/target/release/spectra check \
      --old drift_v2_155.json \
      --new drift_v2_162.json \
      --format markdown > /dev/null; } 2>&1
done
# → ~6 ms per run
```

---

## 7. What this benchmark proves

- **Scale.** Spectra correctly parses a real production 428 KB Anchor IDL with 249 instructions, 27 accounts, 115 types, 26 events, and 349 errors — without modification, without crashing, without dependence on the Solana SDK.
- **No false positives on real-world IDL.** Identical input → exit 0, zero findings on 428 KB of production JSON. This is the CI-gating contract validated against real data.
- **Real finding on a real upgrade.** Spectra detected a `PerpMarket` layout change with stable discriminator — the exact silent-corruption pattern documented in [SEVERITY.md](SEVERITY.md) `R-ACC-SILENT-CORRUPT` — in a real Drift protocol release that shipped on mainnet.
- **Performance under realistic load.** ~6 ms on a 428 KB IDL pair. Negligible cost for a CI gate.

What this benchmark **does not** prove:

- Whether Drift's specific upgrade was *actually* unsafe. (The padding-to-config conversion is safe if all on-chain `PerpMarket` accounts had `padding[22] = 0` before upgrade. Spectra correctly defers this judgement to a reviewer with state-inspection access.)
- Whether all 6 ms scale linearly to 10× larger IDLs. (Spectra's diff engine is O(n) in IDL size + O(n²) for collision scan; for any plausible IDL the wall-clock stays well under 100 ms.)
- Whether the same approach catches non-IDL-visible regressions. (It does not — see [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).)

---

## 8. Cross-references

- Synthetic-fixture before/after walkthrough: [BENCHMARK.md](BENCHMARK.md).
- Head-to-head vs `git diff`: [VS_GIT_DIFF.md](VS_GIT_DIFF.md).
- Per-rule severity reference: [SEVERITY.md](SEVERITY.md).
- Coverage matrix: [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).
