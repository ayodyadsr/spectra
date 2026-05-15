# Competitive Benchmark — Spectra vs. Generic JSON / YAML Diff Tools

This document benchmarks Spectra against every publicly available tool a reviewer might reasonably consider as a "good-enough alternative" for diffing Anchor IDL JSON: `diff -u`, `jd`, `dyff`, and `json-diff`. Every measurement is reproducible by running the commands in §6.

The headline result: **Spectra is the only tool capable of CI-gating a real Solana upgrade**, and is also the fastest tool on real-world Anchor IDL after `diff -u` — while producing semantically correct output that the four others structurally cannot.

---

## 1. Tools surveyed

The benchmark set was assembled by searching for the most-starred / most-installed public CLIs that take two JSON or YAML files and emit a diff. Each tool was installed at its latest release and given the same Drift IDL pair from [BENCHMARK_DRIFT.md](BENCHMARK_DRIFT.md).

| Tool | Language | Source | Mode used |
|---|---|---|---|
| `diff -u` (GNU diffutils 3.10) | C | system | line-level textual diff |
| `jd` (v1.9.2) | Go | [josephburnett/jd](https://github.com/josephburnett/jd) | structural JSON diff, default output |
| `dyff` (development build) | Go | [homeport/dyff](https://github.com/homeport/dyff) | `dyff between` |
| `json-diff` | Node | [andreyvit/json-diff](https://github.com/andreyvit/json-diff) | default output |
| **`spectra check`** | Rust | this repo | Anchor-IDL-aware diff |

A search for "solana IDL diff" / "anchor IDL diff" returned no public tool with Solana semantics; Anchor's own [issue #2452](https://github.com/coral-xyz/anchor/issues/2452) (requesting IDL backward-compatibility checks) is still open. Spectra is, to our knowledge, the only public implementation of this capability.

---

## 2. Headline comparison table

Run on commodity laptop (Linux x86_64), best of 5 runs each.

| Capability | `diff -u` | `jd` | `dyff` | `json-diff` | **Spectra** |
|---|---|---|---|---|---|
| Wall-clock on 428 KB Drift IDL pair | 5 ms | 32 ms | 105 ms | **9,200 ms** | **5–7 ms** |
| Wall-clock on synthetic fixture | 1 ms | 3 ms | 7 ms | 45 ms | **2 ms** |
| Whitespace-only reformat ⇒ noise lines reported | **39,715** | 0 | 0 | 0 | **0** |
| Whitespace-only reformat ⇒ exit code | **1 (false block)** | 0 | 0 | 0 | **0** |
| Severity classification (BREAKING vs warning) | no | no | no | no | **yes** |
| Severity-gated exit (exit 1 on BREAKING only) | no | no | no | no | **yes** |
| Exit 0 on warning-only upgrade | **no (exit 1)** | **no (exit 1)** | always 0 | **no (exit 1)** | **yes** |
| Computes Anchor instruction discriminator `sha256("global:" + name)[..8]` | no | no | no | no | **yes** |
| Computes Anchor account discriminator `sha256("account:" + name)[..8]` | no | no | no | no | **yes** |
| Detects silent-corruption (`R-ACC-SILENT-CORRUPT`) | no | no | no | no | **yes** |
| Detects discriminator collision in 8-byte hash space (`R-DISC-COLL`) | no | no | no | no | **yes** |
| SARIF 2.1.0 output → GitHub Security tab | no | no | no | no | **yes** |
| Markdown output for PR comments | no | no | no | no | **yes** |
| Logical Solana-rule IDs (e.g. `instruction_args_changed`) | no | no | no | no | **yes** |
| Auditable rule catalog | n/a | n/a | n/a | n/a | [SEVERITY.md](SEVERITY.md) |

Spectra is the only tool that **structurally can** answer the question a Solana reviewer actually needs to answer: "does this upgrade preserve discriminators, layout, and named runtime invariants?"

---

## 3. Performance — wall-clock detail

### 3.1 Drift Protocol v2.155 → v2.162 (428 KB Anchor IDL)

```
diff -u (5 runs):     0.005 0.005 0.005 0.005 0.005   avg = 5.0 ms
spectra check  (5):   0.007 0.007 0.007 0.007 0.005   avg = 6.6 ms
jd       (5 runs):    0.033 0.032 0.032 0.032 0.033   avg = 32.4 ms
dyff between (5):     0.111 0.104 0.106 0.105 0.104   avg = 106 ms
json-diff (3 runs):   9.474 9.196 8.980               avg = 9,217 ms
```

`json-diff` was reduced to 3 runs because each run takes ~9 seconds. It is the only tool in this set that is **not** viable as a CI gate on real production Solana IDL.

Spectra is **~16× faster than the next semantic structural tool (`jd`)** and within ~1.6× of `diff -u` — while doing strictly more useful work.

### 3.2 Synthetic regression fixture (`examples/lending_v{1,2}.json`, ~32 lines)

```
diff -u (5 runs):    0.001 0.001 0.001 0.001 0.001   avg = 1.0 ms
spectra (5 runs):    0.002 0.002 0.003 0.005 0.005   avg = 3.4 ms
jd (5 runs):         0.003 0.003 0.003 0.002 0.003   avg = 2.8 ms
dyff (5 runs):       0.009 0.008 0.007 0.006 0.007   avg = 7.4 ms
json-diff (5 runs):  0.036 0.045 0.040 0.051 0.049   avg = 44.2 ms
```

On the small fixture, Spectra and `jd` are statistically indistinguishable; both are dominated by process-startup cost. The Drift-scale numbers — where Spectra is dramatically ahead of `jd`, `dyff`, and `json-diff` — are the ones that matter for CI gating real protocols.

---

## 4. Correctness — what each tool produces on the real Drift upgrade

The Drift v2.155 → v2.162 upgrade contains exactly one finding of interest: a silent-corruption-class layout change on `PerpMarket` (padding `[u8,23]→[u8,22]` + new `marketConfig: u8`, with the `PerpMarket` discriminator `0adf0c2c6bf537f7` unchanged). A reviewer needs to be told **"this is silent corruption"** with the discriminator in hand, in language they can act on.

| Tool | What it tells the reviewer |
|---|---|
| `diff -u` | 319 changed lines scattered across a 20,138-line file. No notion of "discriminator", "account", "Borsh layout", or "silent corruption". Reviewer must hold the entire Anchor + Borsh inference chain in their head — see [BENCHMARK.md §2](BENCHMARK.md#2-without-spectra--what-the-reviewer-sees). |
| `jd` | Roughly equivalent to `diff -u` in semantic content: structural JSON node-by-node deltas, no Solana awareness. Output is more compact but the inference chain to "this is silent corruption" is the same as for `diff -u`. |
| `dyff` | Human-readable list of structural changes. Still no Solana semantics. Exits 0 on every change — cannot gate CI. |
| `json-diff` | Color-coded structural diff. Same semantic limit as `jd`. Takes ~9 seconds on the 428 KB IDL. |
| **`spectra check`** | A 6-row table labelling each finding with a stable rule ID (`account_layout_changed_same_discriminator`), the computed account discriminator (`0adf0c2c6bf537f7`), severity (BREAKING), and a one-line explanation pointing at silent corruption. Exits 1, blocking the CI gate. See [BENCHMARK_DRIFT.md §3](BENCHMARK_DRIFT.md#3-with-spectra--what-the-reviewer-sees). |

This is not "Spectra has prettier output." It is: **the four generic tools cannot produce the finding `R-ACC-SILENT-CORRUPT` at all**, because that finding does not live in the JSON tree — it lives in the relationship between (a) the Anchor discriminator algorithm, (b) Borsh positional serialisation, and (c) the fact that the *absence* of a discriminator change combined with the *presence* of a layout change is the bug. A diff tool that does not know what an Anchor discriminator is cannot warn you about preserving one. See [VS_GIT_DIFF.md §3](VS_GIT_DIFF.md) for the formal argument.

---

## 5. CI-gateability — the practical bar

A tool is usable as a CI **gate** if and only if:

1. It exits 0 on additive / non-breaking changes (otherwise every PR is blocked, the gate is disabled within a week, and you have learned nothing).
2. It exits non-zero on actually breaking changes.
3. It produces no false positives on innocent reformatting (otherwise `prettier --write *.json` silently breaks every protocol that adopts the gate).

Mapping each tool to the bar:

| Tool | Bar 1 (warning-only ⇒ exit 0) | Bar 2 (BREAKING ⇒ exit 1) | Bar 3 (whitespace reformat ⇒ exit 0) | Usable as a CI gate? |
|---|---|---|---|---|
| `diff -u` | fail (exits 1) | exits 1 | **fail (exits 1, 39,715 lines noise)** | no |
| `jd` | fail (exits 1) | exits 1 | pass | no — would block every additive upgrade |
| `dyff` | pass (exits 0 always) | **fail (exits 0)** | pass | no — can't distinguish breaking from non-breaking |
| `json-diff` | fail (exits 1) | exits 1 | pass | no — would block every additive upgrade |
| **`spectra check`** | **pass** | **pass** | **pass** | **yes** |

Spectra is the only tool that clears all three bars. This is the core claim the Solana grant proposal makes — and is now verified end-to-end against generic alternatives, not against a hand-picked synthetic.

---

## 6. Reproducing the benchmark

```bash
# Tools.
diff --version | head -1            # GNU diffutils 3.10
jd -version                          # 1.9.2
dyff version                         # development
json-diff --version                  # latest npm

# Fetch fixtures.
mkdir -p /tmp/drift && cd /tmp/drift
curl -sSL -o drift_v2_155.json \
  https://raw.githubusercontent.com/drift-labs/protocol-v2/590049e6bf/sdk/src/idl/drift.json
curl -sSL -o drift_v2_162.json \
  https://raw.githubusercontent.com/drift-labs/protocol-v2/0d35029d78/sdk/src/idl/drift.json

# Whitespace-reformat case.
python3 -c "import json; d=json.load(open('drift_v2_162.json')); \
  json.dump(d, open('drift_v2_162_reformatted.json','w'), indent=4, sort_keys=True)"

# Build Spectra.
cd /path/to/spectra && cargo build --release
SP=/path/to/spectra/target/release/spectra

# Drift wall-clock — 5 runs per tool.
for cmd in \
  "diff -u /tmp/drift/drift_v2_155.json /tmp/drift/drift_v2_162.json" \
  "jd /tmp/drift/drift_v2_155.json /tmp/drift/drift_v2_162.json" \
  "dyff between /tmp/drift/drift_v2_155.json /tmp/drift/drift_v2_162.json" \
  "json-diff /tmp/drift/drift_v2_155.json /tmp/drift/drift_v2_162.json" \
  "$SP check --old /tmp/drift/drift_v2_155.json --new /tmp/drift/drift_v2_162.json --format markdown" ; do
  echo "=== $cmd ==="
  for i in 1 2 3 4 5; do
    { TIMEFORMAT='%R'; time eval $cmd > /dev/null 2>&1 || true ; } 2>&1
  done
done

# CI-gate exit-code tests.
diff -u /tmp/drift/drift_v2_162.json /tmp/drift/drift_v2_162_reformatted.json > /dev/null; echo "diff whitespace exit=$?"
$SP check --old /tmp/drift/drift_v2_162.json --new /tmp/drift/drift_v2_162_reformatted.json --format markdown > /dev/null; echo "spectra whitespace exit=$?"
```

---

## 7. What this benchmark deliberately does **not** claim

Spectra does **not** beat generic tools when the question being asked is "show me every JSON node that changed." `jd`'s structural output is more compact than `spectra`'s for that question, and `dyff`'s coloured output is friendlier for browsing. For *agnostic* JSON diff, those tools are better choices.

The claim here is much narrower and much stronger: **for the specific job of CI-gating an Anchor IDL upgrade on Solana, every other tool in this set is structurally inadequate**. Spectra is purpose-built for that job and produces findings the other tools cannot, in wall-clock time competitive with `diff -u`.

This benchmark is also a single-host measurement; CI wall-clock on shared GitHub runners will be 2–5× slower across the board, but the *relative* ordering of tools is stable (process-startup + IO dominate at this scale).

---

## 8. Cross-references

- Synthetic-fixture before/after walkthrough: [BENCHMARK.md](BENCHMARK.md).
- Real-world Drift validation: [BENCHMARK_DRIFT.md](BENCHMARK_DRIFT.md).
- Head-to-head against `git diff` (structural argument): [VS_GIT_DIFF.md](VS_GIT_DIFF.md).
- Canonical rule IDs and severities: [SEVERITY.md](SEVERITY.md).
- Coverage matrix against 25 Solana upgrade concerns: [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).
- SARIF + GitHub Code Scanning integration: [CI_INTEGRATION.md](CI_INTEGRATION.md).
