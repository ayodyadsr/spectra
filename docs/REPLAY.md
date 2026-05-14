# Replay / Simulation Architecture

M2 introduces the only Spectra component that **executes** code: a bounded `litesvm` replay of a hand-curated transaction corpus against the old and new program binaries.

This document defines the corpus format, the replay loop, the bounds enforced on it, and the failure modes it surfaces. The mechanism is deliberately small. It is not "we replay mainnet."

---

## 1. Why `litesvm`, not mainnet replay

Free-tier GitHub-hosted runners give ~14 GB disk and a 6-hour wall-clock budget. A non-trivial mainnet snapshot does not fit, and downloading one per CI run is bandwidth-prohibitive. The Spectra design therefore commits to:

- **`litesvm`** — a lightweight in-process VM with a current API surface (the deprecated `solana-program-test` is not used).
- A **bounded corpus** per pilot, hand-curated to represent the real upgrade hazards for that program.
- No archive-RPC during CI. Archive-RPC budget exists only for one-time corpus authoring at pilot onboarding.

The cost of this choice is that M2 cannot answer "is the upgrade safe against the entire mainnet history." The benefit is that M2 is **actually runnable** in the CI environment Spectra targets.

---

## 2. Corpus TOML format

Each pilot defines a `spectra-corpus.toml` file:

```toml
schema_version = 1

[program]
id = "<base58 program id>"
idl = "idl/program.json"

[[seed_account]]
name = "alice_pool"
owner = "<base58 owner>"
data_b64 = "..."   # initial account state, captured at pilot onboarding via archive RPC
lamports = 2_039_280

[[tx]]
name = "alice_deposits_100"
signer = "alice"
ix = "deposit"
args_borsh_b64 = "..."
accounts = ["alice_pool", "vault"]
expect_success = true

[[tx]]
name = "bob_withdraws_max"
signer = "bob"
ix = "withdraw"
args_borsh_b64 = "..."
accounts = ["bob_pool", "vault"]
expect_success = true
```

Required fields per `[[tx]]`:

- `name` — human-readable identifier referenced in findings.
- `signer` — deterministic test key. Spectra ships a fixed keypair set so re-runs are reproducible.
- `ix` — instruction name from the IDL.
- `args_borsh_b64` — Borsh-encoded args, base64.
- `accounts` — ordered list of account names from the `[[seed_account]]` section.
- `expect_success` — boolean asserted against the old-program replay.

---

## 3. Replay loop

For each pair of `(old.so, new.so)`:

1. Spin up two `litesvm` contexts.
2. Load the program into each.
3. Apply every `[[seed_account]]` to each context.
4. Replay every `[[tx]]` against each context, in order.
5. For each `(tx, old_context, new_context)` triple, compare:
   - **Deserialize result.** Old: must match `expect_success`. New: differing result -> R-REPLAY-DESERIALIZE-FAIL.
   - **Log shape.** Differing log lines (instruction-level, not bytewise) -> R-REPLAY-LOG-DIVERGENCE (warning).
   - **CPI signatures.** Differing CPI signature set -> R-REPLAY-CPI-FAIL.

The output is structured findings, just like the static layer.

---

## 4. Bounds enforced

| Bound | Value | Enforcement |
|-------|-------|-------------|
| Corpus size | ≤ 50 transactions | Parser rejects > 50. |
| Wall-clock | ≤ 60 s end-to-end | CI step asserts. |
| Network | None | No HTTP client linked into the replay binary. |
| Filesystem writes | Only the `--report` path | Same as static layer. |
| Clock | Fixed slot 0 | `litesvm` deterministic slot. |
| Randomness | None | No `getrandom` in the corpus path. |

A corpus that exceeds the size or wall-clock bound is a corpus authoring bug, not a Spectra bug.

---

## 5. What replay does **not** prove

Replay against a bounded corpus is **not**:

- A proof of upgrade safety across all possible inputs.
- A proof of upgrade safety on mainnet state Spectra has not seen.
- A formal-verification result.

It is calibration evidence: "the upgrade preserves behaviour for the hazards this pilot's team chose to encode."

The pilot team is responsible for choosing the corpus. Spectra is responsible for executing it deterministically and reporting differentials.

---

## 6. Corpus authoring at pilot onboarding

The Archive-RPC budget line covers a **one-time** corpus-authoring engagement per pilot:

1. The pilot lists their top-N upgrade hazards (e.g. "deposit/withdraw on a live pool with non-zero balance," "CPI into the SPL token program").
2. Spectra's author fetches representative account states from archive RPC and encodes them as `[[seed_account]]` entries.
3. The pilot reviews and approves the corpus.
4. The corpus then lives in the pilot's repo and runs in their CI on every PR — no archive RPC during normal CI runs.

This split (one-time authoring vs per-PR runtime) is what keeps M2's recurring cost free-tier-friendly.

---

## 7. Cross-references

- Bounds context: [ARCHITECTURE.md](ARCHITECTURE.md) §3.
- Corpus authoring rules: [CORPUS.md](CORPUS.md).
- Rules surfaced by replay: [SEVERITY.md](SEVERITY.md) §4.
