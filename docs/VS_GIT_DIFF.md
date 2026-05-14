# Spectra vs `git diff`

The most common question when reviewing Spectra: **"Couldn't a careful reviewer get the same result from `git diff`?"**

The honest answer is: **partially.** Some findings are visible in a textual diff if the reviewer knows Anchor's discriminator algorithm and Borsh's positional encoding by heart and never gets tired. Other findings are **fundamentally not derivable** from a textual diff at all.

This document makes the distinction concrete using the synthetic regression fixture shipped in this repo: [`examples/lending_v1.json`](../examples/lending_v1.json) → [`examples/lending_v2.json`](../examples/lending_v2.json).

---

## 1. The raw `git diff` of the same fixture

Running `diff -u examples/lending_v1.json examples/lending_v2.json` produces 32 lines of textual diff:

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

That is what every reviewer sees today.

---

## 2. The Spectra report on the same input

Running `spectra check --old examples/lending_v1.json --new examples/lending_v2.json --format markdown` produces:

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

Exit code `1`, structured JSON also available, CI-gateable.

---

## 3. Side-by-side: what the reviewer must do

| Real upgrade hazard | What `git diff` shows | What the reviewer must mentally compute | What Spectra surfaces |
|---|---|---|---|
| `deposit.amount` widened `u64 → u128` | `- "type": "u64"` / `+ "type": "u128"` | "Anchor uses Borsh. Borsh is positional. Caller serializes 8 bytes today; callee expects 16 bytes after upgrade. Either the call rejects or — if there are more args — silent corruption of the next arg." | **`instruction_args_changed` BREAKING** with the type transition spelled out |
| `withdraw` instruction renamed `withdrawFunds` | `- "name": "withdraw"` / `+ "name": "withdrawFunds"` | "Anchor discriminator is `sha256(\"global:\" + name)[..8]`. Old name and new name produce different 8 bytes. Existing clients invoking the v1 discriminator will hit `InstructionFallbackNotFound`." | **`instruction_removed` BREAKING** (disc `b712469c946da122`) + **`instruction_added` warning** (disc `52b7b3ffcd4ed2be`) — both discriminators computed |
| `Pool` fields reordered | `total_supply, authority, rate, fee_bps` instead of `total_supply, rate, authority` | "Borsh is positional. Existing on-chain accounts have bytes laid out for the old order. After upgrade the deserializer reads `authority` from where `rate` lives. Silent corruption." | **`account_field_reordered` BREAKING** with the full before/after order |
| `Pool` discriminator stable while layout changed | **TWO SEPARATE FACTS** the reviewer must combine: (a) "name `Pool` not renamed" — i.e. the **absence** of a change; (b) "fields reordered" | "Because the struct name is unchanged, the account discriminator is unchanged. The runtime will still accept existing on-chain accounts as valid `Pool`s. But the layout has changed, so reads return wrong fields. This is **silent corruption** of every existing `Pool` account on mainnet." | **`account_layout_changed_same_discriminator` BREAKING** as a **single** finding with the computed discriminator `f19a6d0411b16dbc` and the explicit `silent-corruption risk` label |
| Discriminator collision between two IDL names | **Invisible.** Diff cannot show this. Two different names look different to a text comparator. | Reviewer would have to: enumerate every instruction + account name in the new IDL, compute `sha256("global:" + name)[..8]` for each (and `"account:" + name`), sort and look for collisions. Not feasible by hand. | **`discriminator_collision` BREAKING** when any collision exists. M0 fixture has none → no finding (correctly absent). |
| `withdrawFunds` added (new entrypoint) | `+ "name": "withdrawFunds"` | "New instruction — informational." | **`instruction_added` warning** |
| `Pool.fee_bps` added | `+ "name": "fee_bps"` | "Additive field — protocol must handle `realloc` and rent for existing accounts, else new field reads default to zero." | **`account_field_added` warning** with a reminder to verify resize |

---

## 4. Things `git diff` fundamentally cannot do

Three categories of result are **not** derivable from a textual diff, however careful the reviewer:

### 4.1 Discriminator collision (`R-DISC-COLL`)

Anchor truncates SHA-256 to 8 bytes (~2^32 effort to brute-force a collision). Two unrelated names can produce the same discriminator. The runtime dispatches on those 8 bytes; a collision means instruction A is silently delivered to handler B.

`git diff` shows names. It does not hash them. The collision exists in hash space, not in text space, so no amount of careful reading uncovers it. Spectra computes every discriminator on the new IDL and reports any prefix-8 collision.

### 4.2 Combined facts ("silent-corruption case", `R-ACC-SILENT-CORRUPT`)

Diff shows changes. The silent-corruption case depends on **a change being present in one place AND the absence of a change being maintained in another place**:

- Layout changed (visible in diff)
- Account name unchanged (visible only as the *absence* of a change)
- Therefore discriminator unchanged (not visible at all — derived knowledge)
- Therefore existing accounts on mainnet remain dispatchable into the new layout (consequence of Solana runtime + Anchor + Borsh)

This is a four-step inference chain spanning a presence + an absence + two pieces of off-diff knowledge. Reviewers miss it under load. Spectra produces a single finding that fuses the four steps with the computed discriminator attached.

### 4.3 CI-gateable exit code

`git diff` exits non-zero on **any** diff, including whitespace. It cannot be used as a PR gate because every PR would fail.

Spectra exits:

- `0` on no BREAKING findings (including identical inputs — tested invariant).
- `1` on at least one BREAKING finding.
- `2` on invocation error.
- `3` on refuse-to-analyse (unsupported IDL schema).

See [SEVERITY.md](SEVERITY.md) §5.

This is the contract that lets Spectra live in `required` CI checks.

---

## 5. Performance comparison

| Operation | Wall-clock (lending fixture, 24-line / 6-account IDL) |
|---|---|
| `diff -u v1.json v2.json` | ~1 ms (linear in file size) |
| `spectra check --old v1.json --new v2.json --format markdown` | **1–2 ms** (measured on a commodity laptop, 5 runs: 1, 1, 1, 2, 2 ms) |

Spectra is **not slower than diff** on realistic IDL sizes. The cost story is dominated by SHA-256 of N names (sub-millisecond for any plausible program). Performance is not the bottleneck for either tool — accuracy and interpretation are.

---

## 6. Where `git diff` is enough

In fairness, for small programs (≤ 3 accounts, ≤ 5 instructions) reviewed by attentive engineers who know Anchor + Borsh internals, `git diff` is often sufficient. Spectra is **over-engineering** for this case. The proposal does not pretend otherwise.

Spectra's return-to-scale curve:

| Program size / cadence | Spectra value-add vs `git diff` |
|---|---|
| 1–3 accounts, rarely upgraded, attentive review | **Low** — diff suffices |
| 5–10 accounts, monthly upgrades | **Medium** — silent-corruption + collision findings start mattering |
| 10+ accounts, weekly upgrades, reviewer rotation | **High** — diff alone produces unacceptable miss rate |
| Audit firm running ≥ 10 program engagements | **High** — uniform machine-checkable contract per program |

This is also documented in [docs/ADOPTION.md](ADOPTION.md) §1 (zero-friction integration is what matters; the people who want a CI gate are running upgradable programs).

---

## 7. Bottom line

`git diff` is the **input** Spectra builds on. Spectra is not a fancier diff — it is the **semantic interpretation layer** between text changes and their consequence on existing on-chain state.

- For three findings (`R-DISC-COLL`, `R-ACC-SILENT-CORRUPT`, `R-INS-ARG`) Spectra produces results that `git diff` is structurally incapable of producing.
- For four findings (`R-INS-REM`, `R-INS-ADD`, `R-ACC-REM`, `R-ACC-ADD`) Spectra duplicates what an attentive reviewer would catch from diff, with the value-add being **consistency**, **machine-checkability**, and **CI-gating**.
- For programs where the protocol never upgrades, Spectra is not the right tool.

See [BENCHMARK.md](BENCHMARK.md) for the end-to-end before/after walkthrough on the synthetic fixture.
