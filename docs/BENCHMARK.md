# Benchmark — Before/After Walkthrough

Concrete, reproducible before/after comparison on the synthetic regression fixture shipped in this repo.

The goal is not to show that Spectra is faster than `git diff` — they are both well under 10 ms. The goal is to show **what each tool produces** when shown the same input, so the value-add is visible without inference.

---

## 1. The fixture

Two Anchor legacy-schema IDL JSON files representing a hypothetical lending program upgrade:

- [`examples/lending_v1.json`](../examples/lending_v1.json) — the version "currently deployed on mainnet."
- [`examples/lending_v2.json`](../examples/lending_v2.json) — the candidate upgrade.

The candidate intentionally includes a representative mix of upgrade hazards drawn from the rule table in [SEVERITY.md](SEVERITY.md): an instruction removal masked as a rename, an argument widening, an account field reorder hiding a silent-corruption case, and an additive field requiring storage resize.

---

## 2. Without Spectra — what the reviewer sees

```bash
diff -u examples/lending_v1.json examples/lending_v2.json
```

Output (verbatim, 32 lines):

```diff
--- examples/lending_v1.json
+++ examples/lending_v2.json
@@ -1,5 +1,5 @@
 {
-  "version": "0.1.0",
+  "version": "0.2.0",
   "name": "lending",
   "instructions": [
@@ -17,11 +17,11 @@
         {"name": "pool", "isMut": true, "isSigner": false}
       ],
       "args": [
-        {"name": "amount", "type": "u64"}
+        {"name": "amount", "type": "u128"}
       ]
     },
     {
-      "name": "withdraw",
+      "name": "withdrawFunds",
       "accounts": [
         {"name": "user", "isMut": true, "isSigner": true},
         {"name": "pool", "isMut": true, "isSigner": false}
@@ -38,8 +38,9 @@
         "kind": "struct",
         "fields": [
           {"name": "total_supply", "type": "u64"},
+          {"name": "authority", "type": "publicKey"},
           {"name": "rate", "type": "u64"},
-          {"name": "authority", "type": "publicKey"}
+          {"name": "fee_bps", "type": "u16"}
         ]
       }
     }
```

What the reviewer must mentally do, in order, to clear this PR safely:

1. Note the `version` bump (informational only).
2. Note that `deposit.amount` changed type. Recall that Anchor uses Borsh, that Borsh is positional, and that widening a `u64` to `u128` changes the serialized length from 8 bytes to 16. Conclude that v1 clients sending 8 bytes will either be rejected or — if `deposit` has other args — silently corrupt the next argument.
3. Note that the second instruction's `name` changed from `withdraw` to `withdrawFunds`. Recall that Anchor's instruction discriminator is `sha256("global:" + name)[..8]`. Conclude that v1 clients invoking the v1 discriminator will hit `InstructionFallbackNotFound` after upgrade.
4. Note that `Pool.authority` moved up by one field. Recall that Borsh is positional, that existing `Pool` accounts on mainnet have bytes laid out for the old order, and that those bytes will deserialize into the new field positions after the upgrade.
5. **Most importantly**, note that the **`Pool` struct name did not change** — i.e. the *absence* of a change. Recall that Anchor account discriminator is `sha256("account:" + name)[..8]`, which is stable across the upgrade. Conclude that the runtime will still accept every existing on-chain `Pool` as valid and will silently misread their fields. This is a **silent-corruption** case affecting every account already on mainnet.
6. Note that `Pool.fee_bps` is appended. Conclude that the protocol must handle `realloc` and rent for existing accounts; otherwise reads of `fee_bps` return zero.
7. Sanity-check that no two new IDL names produce a colliding 8-byte SHA-256 discriminator (in practice, no reviewer does this by hand).

Seven mental steps, four of which require recalling Anchor / Borsh internals from memory, one of which (step 5) requires noticing the *absence* of a change, and one of which (step 7) is essentially infeasible without computation.

**Empirical claim about human reviewers** (anecdotal, conservative): the silent-corruption case in step 5 is the most commonly missed finding, because it requires combining a presence (layout change) with an absence (name unchanged) with off-diff knowledge (discriminator algorithm). It is exactly the case that paid audits exist to catch.

---

## 3. With Spectra — what the reviewer sees

```bash
spectra check --old examples/lending_v1.json --new examples/lending_v2.json --format markdown
```

Output (verbatim, M0 commit `e5aad06`):

```markdown
# Spectra Diff Report

**Old program:** `lending`
**New program:** `lending`

**Findings:** 4 breaking, 2 warning

| Severity | Kind | Detail |
|---|---|---|
| BREAKING | instruction_args_changed | `deposit`: [amount: u64] -> [amount: u128] |
| BREAKING | instruction_removed | `withdraw` (disc b712469c946da122) |
| warning  | instruction_added | `withdrawFunds` (disc 52b7b3ffcd4ed2be) |
| warning  | account_field_added | `Pool.fee_bps: u16` |
| BREAKING | account_field_reordered | `Pool`: [total_supply, rate, authority] -> [total_supply, authority, rate, fee_bps] |
| BREAKING | account_layout_changed_same_discriminator | `Pool` layout changed but discriminator f19a6d0411b16dbc is unchanged (silent-corruption risk) |
```

Exit code: `1`.

Every step the reviewer had to perform mentally in §2 is now a labelled finding with a stable rule ID (per [SEVERITY.md](SEVERITY.md)):

- Step 2 (arg widening) → `R-INS-ARG` BREAKING.
- Step 3 (instruction rename) → `R-INS-REM` BREAKING + `R-INS-ADD` warning, with both discriminators computed.
- Step 4 (Pool field reorder) → `R-ACC-FIELD-REORDER` BREAKING.
- Step 5 (silent-corruption case) → `R-ACC-SILENT-CORRUPT` BREAKING with the computed account discriminator `f19a6d0411b16dbc` and the explicit `silent-corruption risk` label.
- Step 6 (additive field) → `R-ACC-FIELD-ADD` warning with the type spelled out.
- Step 7 (collision sanity-check) → integration test `no_false_collision_on_synthetic_fixture` asserts zero collisions on this fixture. If a real upgrade introduced a colliding name, `R-DISC-COLL` BREAKING would fire.

Exit code `1` means CI can block merge on this PR without the reviewer reading the diff at all. The diff is still useful — but Spectra surfaces the *interpretation*, not just the text.

---

## 4. Performance

Wall-clock (Spectra M0 release binary, commodity laptop, 5 consecutive runs):

```
$ for i in 1 2 3 4 5; do
    { TIMEFORMAT='%R'; time ./target/release/spectra check \
        --old examples/lending_v1.json \
        --new examples/lending_v2.json \
        --format markdown > /dev/null; } 2>&1
  done
0.002
0.002
0.001
0.001
0.001
```

| Tool | Wall-clock |
|---|---|
| `diff -u` | ~1 ms |
| `spectra check` | 1–2 ms |

Neither is the bottleneck. The difference is **what is produced**, not **how fast it is produced**.

---

## 5. CI behaviour (also a measured property)

A second tested invariant: `spectra check --old X --new X` (identical inputs) produces exit `0`, zero findings, no false positives. This is verified by:

- Unit / integration test `identical_idls_produce_clean_report` in `tests/integration_test.rs`.
- A standalone CI step `Identical-IDL exit-0 check (no false positives)` in `.github/workflows/ci.yml`.

`git diff X X` of course also produces zero output, but `diff` exits non-zero on any change including whitespace. Only Spectra exits `0` on a real-world identical-source upgrade with reformatted JSON, which is what makes it usable as a `required` CI check.

---

## 6. Reproducing this benchmark

```bash
# 1. Build the release binary.
cargo build --release

# 2. Run the raw diff (for comparison).
diff -u examples/lending_v1.json examples/lending_v2.json

# 3. Run Spectra.
./target/release/spectra check \
  --old examples/lending_v1.json \
  --new examples/lending_v2.json \
  --format markdown

# 4. Confirm exit code 1.
echo "EXIT=$?"

# 5. Confirm no false positives on identical input.
./target/release/spectra check \
  --old examples/lending_v1.json \
  --new examples/lending_v1.json \
  --format markdown
echo "EXIT=$?"
# → exit 0, zero findings.
```

Every line of output in §2 and §3 above was produced by exactly these commands on commit [`e5aad06`](https://github.com/ayodyadsr/spectra/commit/e5aad06) (CI run [`25851086148`](https://github.com/ayodyadsr/spectra/actions/runs/25851086148), green).

---

## 7. Cross-references

- Head-to-head rule-by-rule against `git diff`: [VS_GIT_DIFF.md](VS_GIT_DIFF.md).
- Per-rule severity reference: [SEVERITY.md](SEVERITY.md).
- Coverage matrix (what's in / out of scope): [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).
- Threat model that frames why these findings matter: [THREAT_MODEL.md](THREAT_MODEL.md).
