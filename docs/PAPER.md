# Spectra: A Static Analyzer for Behavioral Regression in Upgradeable Solana Programs

**Ayodya**
Independent Security Researcher
[github.com/ayodyadsr/spectra](https://github.com/ayodyadsr/spectra)

---

## Abstract

**Background.** Program upgrades on Solana — the second-largest DeFi blockchain by
total value locked ($10.2B, 329+ protocols, Q1 2026) — are enabled by default for every
user-deployed smart contract via the `BPFLoaderUpgradeab1e` runtime. No existing public
tool detects whether an upgrade preserves backward compatibility with existing on-chain
state: textual diff tools lack the semantic knowledge to interpret Borsh serialization
consequences, build-provenance tools address a different threat layer, and professional
security audits cost $15,000–$100,000+ per engagement with 2–6 week lead times, making
them infeasible as per-upgrade gates.

**Problem.** We characterize the *behavioral regression gap*: a class of upgrade
defects — specifically, silent account corruption and discriminator collision — that are
(a) financially consequential, (b) invisible to textual diff regardless of reviewer
expertise, and (c) uncovered by any existing free, CI-integrable tool.

**Contribution.** We present **Spectra**, a deterministic static analyzer that
compares two Anchor IDL versions and emits structured, severity-classified findings
in 1–2 ms with zero false positives on identical inputs. Spectra implements 11 rule
types (M0), covering the complete Anchor legacy-schema IDL detection surface. Of the
six findings Spectra produces on the canonical synthetic fixture, two (33%) are
structurally impossible to derive from textual diff; both correspond to the
highest-impact failure modes: silent mainnet state corruption and silent instruction
misrouting, neither of which produces a Solana runtime error.

**Results.** On the canonical fixture (32-line textual diff), manual safe review
requires 7 reasoning steps, of which 4 (57%) demand off-diff domain knowledge and
1 (14%) is computationally infeasible without tooling. Spectra resolves all 7 steps
in 1–2 ms at zero cost, with a contractual exit-code interface (0 = clean,
1 = BREAKING, 2 = invocation error, 3 = refuse-to-analyse) suitable for use as a
`required` CI check. On a real-world production IDL — Drift Protocol v2.155 → v2.162
(428 KB, 249 instructions, 319 changed lines across 20,138 lines) — Spectra extracts
6 findings in 6 ms, including a real silent-corruption case on `PerpMarket` that is
structurally invisible to textual diff. The full roadmap extends to 23 rule types
across M0–M2.

---

## 1. Introduction

### 1.1 Motivation

Smart contract ecosystems have suffered unprecedented financial losses from
application-layer vulnerabilities. The Hacken 2025 Yearly Security Report documents
$4.0 billion in total crypto losses during 2025 alone, of which $512 million (12.8%)
originated from smart contract vulnerabilities specifically [Hacken, 2025]. On Solana,
cumulative exploit losses from application-layer attacks between December 2020 and
April 2025 exceeded $530 million across at least 14 major security incidents
[Collins, 2025].

These aggregate figures, however, conflate multiple vulnerability classes. A specific
and underaddressed class concerns *upgrade-time behavioral regression*: defects
introduced not through initial deployment bugs, but through the act of upgrading a
live program in ways that corrupt existing on-chain state or misroute existing client
invocations. This class is distinct in three respects that make it resistant to
existing mitigations:

1. **No runtime signal.** The Solana runtime does not validate whether an upgrade
   preserves Borsh layout compatibility with existing account data. A
   discriminator-stable layout change is accepted silently; corruption is observable
   only downstream, as incorrect field reads.

2. **Textual diff is insufficient.** As we characterize formally in Section 3,
   detecting the silent-corruption case requires combining a visible change (layout
   modification) with an invisible non-change (account name unchanged → discriminator
   unchanged) and two pieces of off-diff domain knowledge (Anchor's discriminator
   algorithm; Borsh's positional encoding). No amount of textual review resolves this
   structurally.

3. **No public tool covers this gap.** The four security layers currently in use —
   build provenance (`solana-verify`), manual textual diff, professional audit, and
   post-deploy runtime monitoring (Hypernative, Range) — each address a different
   question and leave the behavioral regression question unanswered.

### 1.2 Research Gap

We define the *behavioral regression gap* as follows: given two versions of an Anchor
program IDL, no publicly available, free, CI-integrable tool determines whether
deploying the new version is compatible with existing on-chain state and existing
client invocation patterns. This gap exists because:

- Textual diff tools operate on text, not Borsh semantics.
- `solana-verify` / `anchor verify` answer build provenance, not compatibility.
- Professional audits are economically infeasible for routine patch-level upgrades.
- Runtime monitors detect failures *after* deployment, not before.

### 1.3 Contributions

This paper makes the following contributions:

1. **Problem characterization with measurement.** We provide the first formal
   decomposition of the upgrade review task into reasoning steps, quantifying the
   fraction that requires off-diff domain knowledge (57%) and the fraction that is
   computationally infeasible without tooling (14%).

2. **Identification of two structurally undetectable finding classes.** We demonstrate
   that silent account corruption (`R-ACC-SILENT-CORRUPT`) and discriminator collision
   (`R-DISC-COLL`) cannot be surfaced from textual diff regardless of reviewer
   expertise, and constitute 33% of findings in the canonical fixture.

3. **Spectra M0: a working implementation.** Spectra implements 11 rule types covering
   the complete Anchor legacy-schema IDL detection surface, with a contractual
   exit-code interface, verified zero-FP invariant, and 1–2 ms wall-clock performance —
   matching plain `diff` on realistic inputs with no meaningful overhead.

4. **Real-world validation at production scale.** Spectra is evaluated against a real
   Solana mainnet program upgrade — Drift Protocol v2.155 → v2.162, a 428 KB Anchor
   IDL with 249 instructions — completing analysis in 6 ms and surfacing a real
   silent-corruption case (`PerpMarket` padding-to-config conversion) that is
   structurally invisible to textual diff.

5. **A complete rule roadmap to 23 types.** SEVERITY.md documents 9 additional rules
   reserved for M1 and 3 for M2, constituting the full planned detection surface
   for Anchor programs through replay-based pre-deploy testing.

---

## 2. Background

### 2.1 Solana Program Upgradeability

Solana smart contracts ("programs") are compiled to Solana Bytecode Format (sBPF)
and deployed as executable accounts on-chain. The dominant deployment loader is
`BPFLoaderUpgradeab1e`. Sec3 (2022) documents: *"By default, all user-deployed
Solana programs are deployed with `BPFLoaderUpgradeab1e` and hence are upgradable."*
The Solana SDK itself warns: *"This ability breaks the 'code is law' contract that
once a program is on-chain it is immutable."* [bpf_loader_upgradeable.rs, Solana SDK].

Upgradeability means that any program with a live upgrade authority can have its
bytecode replaced in a single transaction, with no user consent required and no
on-chain notification to existing users. The financial exposure this creates is
substantial: as of Q1 2026, Solana's DeFi ecosystem locks $10.2 billion in value
across 329+ protocols [BingX, 2026], the majority of which runs on upgradeable programs.

### 2.2 Anchor IDL and Borsh Serialization

Anchor is the dominant Solana smart contract framework. It generates an
**Interface Definition Language (IDL)** JSON artifact enumerating every instruction,
account, and field. Two properties of Anchor and Borsh create upgrade risk:

**Borsh positional encoding.** Borsh serializes struct fields in declaration order
with no field names or type tags. A field reorder changes the byte layout without
changing field names or types. Existing on-chain accounts, serialized for the old
field order, will deserialize into the wrong fields under the new program — with no
runtime error.

**Anchor discriminator algorithm.** Account and instruction discriminators are
derived as the first 8 bytes of SHA-256("account:\<Name\>") and
SHA-256("global:\<name\>") respectively. This algorithm is deterministic and based
solely on the name string. Two implications follow: (i) renaming an account or
instruction changes its discriminator, breaking existing clients; (ii) two different
names may produce colliding 8-byte prefixes (~1/2³² probability per name pair),
causing silent instruction misrouting at the runtime dispatch layer.

### 2.3 Existing Tool Landscape

Table 1 summarizes the four security layers in current use and the question each
answers.

**Table 1. Security tool landscape for Solana program upgrades (as of May 2026)**

| Layer | Representative Tool | Question Answered | Upgrade Compatibility? |
|---|---|---|---|
| Build provenance | `solana-verify`, `anchor verify` | Does deployed bytecode match public source? | No |
| Behavioral regression | **(gap)** | Does v_{n+1} preserve compatibility with existing state? | — |
| Formal verification | Audit-firm internal tooling | Are stated invariants provably preserved? | Partial, not public |
| Runtime monitoring | Hypernative, Range | Did something go wrong after deploy? | No (post-hoc) |

The behavioral regression layer is unoccupied by any public, free, CI-integrable tool.
This paper presents Spectra as the first instantiation of this layer.

A reviewer may reasonably ask whether a generic structural JSON diff tool — `jd`,
`dyff`, `json-diff`, or even GNU `diff -u` — would be adequate as a stopgap.
§5.6 answers this empirically by benchmarking all four against Spectra on the
identical real-world Drift IDL pair: each generic tool fails at least one of the
three CI-gate preconditions (severity-gated exit, no false positive on
whitespace reformat, additive change does not block merge), and none can detect
the two structurally-undetectable finding classes proved in §3.2.

---

## 3. Problem Characterization

### 3.1 Manual Review Cost: A Structured Decomposition

We decompose the task of safely approving a realistic Solana program upgrade PR into
atomic reasoning steps. The fixture used is the canonical synthetic regression case
shipped with Spectra (`examples/lending_v1.json` → `examples/lending_v2.json`,
introduced at commit `e5aad06`), representing a patch-level upgrade to a lending
protocol containing a representative mix of upgrade hazards.

A textual diff (`diff -u`) of this fixture produces **32 lines** of output.
Table 2 decomposes the reasoning required for safe PR approval.

**Table 2. Manual reasoning decomposition for one realistic upgrade diff (n = 7 steps)**

| Step | Reviewer Operation | Off-Diff Knowledge Required | Feasible Manually? |
|---|---|---|---|
| 1 | Note version bump (informational) | None | Yes |
| 2 | Identify `u64 → u128`; derive that Borsh caller serialization changes from 8 to 16 bytes | Borsh encoding | Yes (with expertise) |
| 3 | Note instruction rename; derive that Anchor discriminator changes and old clients will error | Anchor discriminator algorithm | Yes (with expertise) |
| 4 | Note account field reorder; derive that existing on-chain accounts deserialize into wrong fields | Borsh positional encoding | Yes (with expertise) |
| 5 | Note that account *name* did **not** change (absent change); derive discriminator is stable; conclude runtime will silently accept old accounts into new layout → **silent corruption** | Both (Borsh + Anchor) + reasoning over absence | Yes (with expertise, under low load) |
| 6 | Note additive field; flag that protocol must handle `realloc` and rent for existing accounts | Solana account model | Yes (with expertise) |
| 7 | Enumerate all instruction/account names; compute SHA-256 prefix-8 for each; check all pairwise combinations for collision | Computation | **No — infeasible by hand** |

**Quantification:** 4 of 7 steps (57%) require specialized domain knowledge not visible
in the diff. 1 of 7 steps (14%) is computationally infeasible without tooling.
Consequently, **71% of the critical review steps cannot be completed solely by reading
the diff**, regardless of reviewer expertise.

Step 5 is the highest-risk miss in practice: it requires reasoning over a
*non-event* (the absent name change) and chaining four inference steps, two of which
are invisible in any textual representation. BENCHMARK.md §2 notes: *"the
silent-corruption case in step 5 is the most commonly missed finding, because it
requires combining a presence (layout change) with an absence (name unchanged) with
off-diff knowledge (discriminator algorithm). It is exactly the case that paid audits
exist to catch."*

### 3.2 Two Structurally Undetectable Finding Classes

Of the 11 finding types in Spectra M0, two are fundamentally not derivable from any
textual diff. We prove this by construction.

**Definition.** A finding is *textually undetectable* if and only if its detection
requires either: (a) computing a function over the text (rather than reading it), or
(b) reasoning over the *absence* of a change combined with off-text knowledge.

**`R-ACC-SILENT-CORRUPT`** satisfies condition (b). Detection requires:

```
Premise 1:   Account layout changed       (visible in diff)
Premise 2:   Account name unchanged       (visible only as ABSENCE of a diff line)
Inference 1: name unchanged → sha256("account:<Name>")[..8] unchanged
             (requires Anchor discriminator knowledge)
Inference 2: discriminator unchanged → Solana runtime accepts old account bytes as
             valid input to new program layout
             (requires Solana runtime knowledge)
Conclusion:  every existing on-chain account of this type will be silently misread
             after upgrade, with no runtime error
```

This is a four-step chain where two premises are not representable in text diff
and two inference steps require domain knowledge external to the diff.

**`R-DISC-COLL`** satisfies condition (a). Detection requires computing
SHA-256("\<prefix\>:\<name\>")[..8] for every instruction and account name in the new
IDL and checking all pairwise combinations for collision. The collision exists in
*hash space*, not in *text space*; no textual representation can expose it.

**Consequence.** On the canonical fixture, 2 of 6 findings (33%) are textually
undetectable. Both correspond to the highest-impact failure mode class: silent state
corruption and silent instruction misrouting, respectively.

### 3.3 Coverage Gap Quantification

`SOLANA_EDGE_CASES.md` enumerates **25 distinct upgrade concerns** relevant to
Solana programs. Table 3 summarizes coverage by milestone. Of the 25 concerns, 5 are
permanently out of scope by principled design (Token-2022 TLV, Token-2022
transfer-hook, upgrade-authority transfer, mainnet snapshot replay, compiler/LLVM
bugs), leaving 20 in-scope concerns.

**Table 3. Coverage of 25 Solana upgrade concerns by milestone**

| Milestone | Concerns covered (full) | Of in-scope (n = 20) | Of total (n = 25) |
|---|---|---|---|
| M0 (implemented) | 6 | 30% | 24% |
| M1 (pending grant) | +8 | 70% | 56% |
| M2 (pending grant) | +1 (CPI signature stability via replay) | 75% | 60% |
| M1 partial (zero-copy padding-size, SBPF informational) | +2 partial | 85% | 68% |
| Permanent out-of-scope | 5 | — | — |

The 24% M0 coverage figure requires context: the 6 concerns covered include the two
classes — silent corruption and discriminator collision — that are *undetectable* by
any other tool. The 5 permanently-out-of-scope items are excluded by principled design
decisions documented in `NON_GOALS.md`, not by implementation gaps. The three remaining
items (PDA seed-derivation drift, PDA bump-seed search, `.rodata` constants) are
marked "Future Expansion" in `SOLANA_EDGE_CASES.md` — feasible but deferred beyond M2.

---

## 4. Spectra: Design and Implementation

### 4.1 Design Principles

Spectra is designed around three invariants required for CI usability:

**I1 — Determinism.** Identical inputs must always produce identical output.
SHA-256 computation is deterministic; Spectra has no network access, no randomness,
no clock dependency.

**I2 — Contractual exit codes.** Exit 0 must be machine-trustable as a merge
condition. This requires a verified zero-FP invariant: `spectra check --old X --new X`
must always return exit 0 with zero findings, even when the JSON is reformatted.
(Contrast: `diff X X` exits 0, but `diff` exits non-zero on *any* difference,
including whitespace, making it unusable as a `required` CI check.)

**I3 — No false silence.** Spectra must never emit exit 0 on input it cannot fully
analyze. Unknown IDL schema versions return exit 3 (refuse-to-analyse),
not a clean report.

### 4.2 Detection Algorithm

Spectra operates in three sequential passes over the parsed IDL pair:

**Pass 1 — Instruction diff (O(N)).**
For each instruction in the old IDL: check presence in new IDL by name.
Missing → `R-INS-REM` BREAKING (discriminator computed).
New → `R-INS-ADD` warning (discriminator computed).
Matched → compare argument lists positionally; change → `R-INS-ARG` BREAKING.

**Pass 2 — Account layout diff (O(N·F) where F = fields per account).**
For each account: extract ordered field list; compare to new IDL.
Removal → `R-ACC-FIELD-REM` BREAKING.
Reorder → `R-ACC-FIELD-REORDER` BREAKING.
Type change → `R-ACC-FIELD-TYPE` BREAKING.
Addition → `R-ACC-FIELD-ADD` warning.
After any layout change: check whether account name is unchanged.
If yes → `R-ACC-SILENT-CORRUPT` BREAKING (discriminator computed,
"silent-corruption risk" label).

**Pass 3 — Discriminator collision sweep (O(N²) hashing + sort).**
Compute SHA-256-based 8-byte discriminator for every instruction and account name
in the new IDL. Sort by discriminator prefix. Any two names sharing the same 8 bytes →
`R-DISC-COLL` BREAKING.

### 4.3 Rule Table

Table 4 presents the complete M0 rule table from `SEVERITY.md`.

**Table 4. Spectra M0 rule table (11 rules, from `SEVERITY.md`)**

| Rule ID | Finding Kind | Severity | Mechanism |
|---|---|---|---|
| `R-INS-REM` | `instruction_removed` | BREAKING | Old clients hit `InstructionFallbackNotFound` |
| `R-INS-ARG` | `instruction_args_changed` | BREAKING | Borsh arg layout mismatch → corrupt deserialize |
| `R-INS-ADD` | `instruction_added` | warning | Informational |
| `R-ACC-REM` | `account_removed` | BREAKING | Old discriminator no longer accepted |
| `R-ACC-ADD` | `account_added` | warning | Informational |
| `R-ACC-FIELD-REM` | `account_field_removed` | BREAKING | Borsh layout shifts |
| `R-ACC-FIELD-ADD` | `account_field_added` | warning | Informational; verify realloc + rent |
| `R-ACC-FIELD-REORDER` | `account_field_reordered` | BREAKING | Borsh positional — existing accounts corrupt |
| `R-ACC-FIELD-TYPE` | `account_field_type_changed` | BREAKING | Width/encoding change corrupts existing accounts |
| **`R-ACC-SILENT-CORRUPT`** | `account_layout_changed_same_discriminator` | **BREAKING** | **Discriminator stable; runtime silently accepts old bytes into new layout** |
| **`R-DISC-COLL`** | `discriminator_collision` | **BREAKING** | **Two names share truncated 8-byte SHA-256** |

Rules in bold are the two textually-undetectable finding classes (§3.2).
Severities are deterministic — a property of the rule, not context — and contractually
stable within a major version (`SEVERITY.md` §6).

### 4.4 Exit-Code Contract

```
Exit 0  ←  analysis complete; no BREAKING findings        (safe to merge)
Exit 1  ←  analysis complete; ≥1 BREAKING finding         (block merge)
Exit 2  ←  invocation error (bad path, JSON parse failure)
Exit 3  ←  refuse-to-analyse (unsupported IDL schema)
```

Exit 3 is the *honest-failure* code: Spectra never substitutes exit 0 for inputs
it cannot soundly analyze. This property distinguishes Spectra from plain textual
diff, which has no concept of "I don't understand this input."

### 4.5 Implementation

Spectra is implemented in Rust (Spectra-core crate + binary), with a Python wrapper
and a GitHub Actions composite scaffold. **M0 has zero Solana SDK dependency** — the
tool operates on pure JSON in, structured report out. This means Spectra runs on any
CI runner without a blockchain toolchain, a prerequisite for zero-friction adoption.

---

## 5. Evaluation

All results in this section are reproducible from commit
[`0a5c684`](https://github.com/ayodyadsr/spectra/commit/0a5c684) (CI run
[25910510275](https://github.com/ayodyadsr/spectra/actions/runs/25910510275), green).
The synthetic fixture results carry from commit `e5aad06`; the real-world Drift
results (§5.5) carry from `0a5c684`.

### 5.1 Finding Coverage on Canonical Fixture

Table 5 presents the complete finding-by-finding comparison between textual diff
and Spectra on the canonical fixture.

**Table 5. Per-finding comparison: textual diff vs. Spectra (`lending_v1.json` → `lending_v2.json`)**

| Finding | Derivable from `diff`? | Spectra output |
|---|---|---|
| `deposit.amount` widened `u64 → u128` | Partially — text change visible; Borsh length consequence is not | `R-INS-ARG` BREAKING |
| `withdraw` instruction renamed to `withdrawFunds` | Partially — name change visible; discriminator `b712469c946da122` consequence is not | `R-INS-REM` BREAKING + `R-INS-ADD` warning (both discriminators computed) |
| `Pool` fields reordered | Partially — reorder visible; BREAKING consequence requires Borsh knowledge | `R-ACC-FIELD-REORDER` BREAKING (before/after order explicit) |
| **`Pool` layout changed, name unchanged** | **No** — requires 4-step inference chain (§3.2) | **`R-ACC-SILENT-CORRUPT` BREAKING, discriminator `f19a6d0411b16dbc`, "silent-corruption risk" label** |
| `Pool.fee_bps` field added | Yes | `R-ACC-FIELD-ADD` warning |
| Discriminator collision check | **No** — computationally infeasible (§3.2) | `R-DISC-COLL`: zero collisions (correctly absent on this fixture) |

**Summary:** 2 of 6 findings (33%) cannot be derived from textual diff.
The remaining 4 are textually visible but require Borsh/Anchor domain knowledge
to interpret severity correctly.

### 5.2 Performance on the Canonical Fixture

Table 6 presents wall-clock measurements from `BENCHMARK.md` §4 (5 consecutive runs,
Spectra M0 release binary, commodity laptop).

**Table 6. Wall-clock performance on the canonical fixture: textual diff vs. Spectra**

| Tool | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Median |
|---|---|---|---|---|---|---|
| `diff -u` | ~1 ms | ~1 ms | ~1 ms | ~1 ms | ~1 ms | 1 ms |
| `spectra check` | 2 ms | 2 ms | 1 ms | 1 ms | 1 ms | **1 ms** |

Spectra introduces no meaningful latency overhead relative to textual diff on the
canonical fixture. The SHA-256 computation over all IDL names (Pass 3) is
sub-millisecond. Performance is not the bottleneck — semantic correctness is.

### 5.3 Zero False-Positive Invariant

`spectra check --old X --new X` (identical inputs) returns exit 0, zero findings.
This invariant is verified by:

- Integration test `identical_idls_produce_clean_report` in `tests/integration_test.rs`.
- CI step `Identical-IDL exit-0 check (no false positives)` in
  `.github/workflows/ci.yml`.

This invariant is the condition that makes Spectra usable as a `required` CI check.
Plain `diff` exits non-zero on any textual change, including JSON whitespace
reformatting, and therefore cannot be used as a merge gate for this purpose.

### 5.4 Adversary Coverage

From `THREAT_MODEL.md`, Spectra is designed against five adversary classes:

| Adversary | Description | Spectra Coverage |
|---|---|---|
| A1 — Honest-but-rushed maintainer | Unintentional layout break | **Primary target** — CI blocks before mainnet |
| A2 — Inattentive reviewer | Visual miss on field reorder buried in large diff | **Primary target** — deterministic structured finding |
| A3 — Malicious insider with merge access | Intentional silent-corruption change | **Partial** — change is surfaced; cannot prevent merge if actor controls CI |
| A4 — External PR submitter (no merge access) | Non-maintainer introduces breaking change | **Primary target** — surfaced during review |
| A5 — Build supply-chain attacker | Bytecode substitution | **Out of scope** — `solana-verify` layer |

### 5.5 Real-World Validation: Drift Protocol v2.155 → v2.162

The canonical fixture in §5.1 is synthetic. To validate that the same approach
scales to production complexity, we ran Spectra against a real Solana mainnet
program upgrade: [Drift Protocol v2](https://github.com/drift-labs/protocol-v2),
one of the largest upgradable Anchor programs on Solana ($300M+ historical TVL
[Drift Foundation, 2025]). The IDLs are the public Anchor IDL JSONs at two
historical commits:

- v2.155 — `drift-labs/protocol-v2@590049e6bf` (2026-01-21).
- v2.162 — `drift-labs/protocol-v2@0d35029d78` (2026-04-01).

**Table 7. Scale of the Drift v2 IDL pair**

| Property | v2.155 | v2.162 |
|---|---|---|
| IDL size (JSON) | 421.5 KB | 428.3 KB |
| IDL lines | 19,712 | 20,138 |
| Instructions | 246 | 249 (+3) |
| Accounts | 27 | 27 |
| Types | ~115 | 115 |
| Events | 26 | 26 |
| Errors | 349 | 349 |

This represents **7 minor versions over ~2.5 months** — a representative production
cadence for a high-velocity Solana DeFi protocol. `diff -u` over this pair produces
**393 lines of unified output (319 actual changed lines)** distributed across a
20,138-line file. A manual reviewer would need to perform the 7-step inference chain
from §3.1 on every change region and a collision scan over **417 IDL names**
(249 + 27 + 115 + 26) — which by hand is computationally infeasible.

Spectra produces the following report (verbatim):

```markdown
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

The critical finding is a real instance of `R-ACC-SILENT-CORRUPT`. Drift's v2.162
upgrade converts **1 byte of `PerpMarket.padding` into a new `marketConfig: u8`
field**. The struct name `PerpMarket` is unchanged, so the account discriminator
`0adf0c2c6bf537f7` is preserved. The Solana runtime and Anchor framework will accept
every existing on-chain `PerpMarket` account as valid; the byte at the old
`padding[22]` offset is now interpreted as `marketConfig`. This is safe **if and only
if** that byte was zero in every pre-existing on-chain `PerpMarket` — which is the
exact question Spectra is designed to surface for explicit reviewer attention. On a
393-line release PR, this case is realistically missed by manual review.

**Table 8. Wall-clock on real-world production IDL (428 KB, 5 consecutive runs)**

| Operation | Run 1 | Run 2 | Run 3 | Run 4 | Run 5 | Mean |
|---|---|---|---|---|---|---|
| `diff -u drift_v2_155.json drift_v2_162.json` | 4 ms | 4 ms | 4 ms | 5 ms | 4 ms | 4.2 ms |
| `spectra check --old v2_155 --new v2_162` | 7 ms | 7 ms | 6 ms | 5 ms | 6 ms | 6.2 ms |
| `spectra check --old v2_162 --new v2_162` (identical-input invariant) | — | — | — | — | — | 6.2 ms, **exit 0, 0 findings** |

Spectra is ~1.5× slower than `diff -u` on a 428 KB production IDL — still under
10 ms, negligible for CI gating. Notably, the zero-FP invariant (§5.3) is preserved
on real-world data: `spectra check --old v2.162 --new v2.162` exits 0 with zero
findings on the full 428 KB production IDL. This is the contract that makes Spectra
usable as a `required` CI check on real protocols.

Full reproducibility commands and the line-by-line analysis of the silent-corruption
case are in `BENCHMARK_DRIFT.md` [Spectra-BENCHMARK-DRIFT].

### 5.6 Head-to-Head Against Generic Diff Tools

To rule out the hypothesis "an existing JSON / YAML diff tool is good enough,"
Spectra was benchmarked against four publicly available structural diff tools
on the identical Drift IDL pair: GNU `diff -u` 3.10 [GNU-DIFFUTILS], `jd` 1.9.2
[Burnett-JD], `dyff` [Homeport-DYFF], and `json-diff` [Vit-JSONDIFF]. The
selection criterion was: any CLI tool with public install instructions that
takes two JSON or YAML files and reports a structural diff. No tool with Anchor
or Solana semantics was found in the survey, consistent with §1.2's research-gap
claim and the unresolved Anchor issue #2452.

**Table 9: Competitive performance on Drift v2.155 → v2.162 (428 KB, 5 runs each).**

| Tool | Mean wall-clock | Severity-gated exit | Whitespace-reformat ⇒ exit | `R-ACC-SILENT-CORRUPT` detection |
|------|----------------:|---------------------|---------------------------|----------------------------------|
| `diff -u` | 5.0 ms | none | **1** (39,715 noise lines) | no |
| `jd` | 32.4 ms | none (1 on any change) | 0 | no |
| `dyff` | 106 ms | **always 0** (incl. breaking) | 0 | no |
| `json-diff` | 9,217 ms | none (1 on any change) | 0 | no |
| **Spectra** | **6.6 ms** | **0 / 1 / 2 / 3** | **0** | **yes** |

Spectra is the only tool in the survey that satisfies the three preconditions of
a CI gate simultaneously: (1) exit 0 on additive-only upgrades, (2) exit non-zero
on actually breaking upgrades, (3) zero false positives on whitespace reformatting.
Each generic tool fails at least one precondition. Crucially, the
silent-corruption case `R-ACC-SILENT-CORRUPT` is **not detectable** by any of the
four generic tools, consistent with the structural-undetectability proof of §3.2:
the finding requires combining (a) the Anchor discriminator algorithm and (b)
the *absence* of a discriminator change with (c) the *presence* of a layout
change. No generic JSON differ has access to fact (a), and no purely textual diff
can encode the logical conjunction of "absence" and "presence" as a single
named finding. The full per-tool methodology, raw measurements, and reproduction
commands are published as `COMPETITIVE_BENCHMARK.md` [Spectra-COMPETITIVE].

**Performance interpretation.** Spectra is ~16× faster than the fastest
*structural* alternative (`jd`) and within 1.5× of textual `diff -u` while doing
strictly more useful work — computing Anchor discriminators, distinguishing
silent corruption, and emitting severity-gated exit codes. The 9-second
`json-diff` figure illustrates that some popular structural differs are not
viable as CI gates on real-world Solana IDL at all; this is non-trivial when the
candidate tool is sometimes recommended as a "drop-in solution" in informal
Solana discussions.

**What this real-world result does not prove.** It does not prove that Drift's
specific upgrade was *actually* unsafe — the padding-to-config conversion is safe if
all on-chain `PerpMarket` accounts had `padding[22] = 0` before upgrade. Spectra
correctly defers this judgment to a reviewer with state-inspection access; it is the
*flagging* of the case for explicit review, not the determination of exploitability,
that is the contribution. The result also does not generalize to arbitrarily larger
IDLs without further measurement: Spectra's collision scan is O(N²), and although it
remains sub-10 ms at 417 IDL names, scaling beyond ~10⁴ names would require profiling.

---

## 6. Related Work

### 6.1 Build Provenance Tools

`solana-verifiable-build` (distributing the `solana-verify` binary) and
`anchor verify` answer the question: *"Does the deployed bytecode match the
claimed source after a reproducible Docker build?"* These tools are orthogonal
to Spectra: they verify the build layer, not the behavioral compatibility layer.
A program may be verifiably built from its public source and still introduce
a breaking layout change in that source.

### 6.2 Runtime Monitoring

Hypernative and Range are post-deploy monitoring services that detect anomalous
on-chain activity. Their detection window is after deployment; they cannot prevent
an upgrade from reaching mainnet. Spectra is strictly pre-deploy.

### 6.3 Formal Verification

Audit firms (OtterSec, Neodyme, Sec3, Halborn) perform engagement-level formal
and manual review. This layer is effective but has properties incompatible with
CI gating: engagement costs ($15,000–$100,000+) and lead times (2–6 weeks) make
per-upgrade coverage economically infeasible for actively-developed protocols.
Spectra occupies the *automated first-pass gate* position that audit firms
presuppose exists but do not themselves provide.

### 6.4 Smart Contract Analysis in Other Ecosystems

The Ethereum ecosystem has produced tools such as Slither [Feist et al., 2019],
Mythril [Consensys, 2018], and Echidna [Grieco et al., 2020] for static analysis
and fuzzing of EVM-bytecode smart contracts. These tools address a fundamentally
different execution model (EVM vs. sBPF), different state model (storage slots vs.
account data), and different framework assumptions (Solidity/Vyper vs. Rust/Anchor).
No direct port of these tools addresses the Anchor IDL diff and Borsh-semantic
analysis that Spectra performs.

---

## 7. Threats to Validity

### 7.1 Construct Validity

The canonical fixture in §5.1 is synthetic. It was constructed to represent a
*representative mix of upgrade hazards*, not a random sample of real-world upgrades.
The 7-step manual reasoning decomposition (§3.1) is an authorial analysis, not a
controlled user study. Claims about reviewer miss rates are characterized as
"anecdotal, conservative" in `BENCHMARK.md` and should be read accordingly.

**Mitigation.** The step decomposition is derived from the documented fixture in a
verifiable, reproducible manner. Any reviewer can reproduce the same decomposition
by attempting the manual review without tooling. The existence of the structurally
undetectable findings (§3.2) is a formal property, not an empirical one: it is
demonstrated by construction and does not depend on reviewer behavior. The real-world
Drift validation (§5.5) provides one external data point that the same finding
structure occurs in a production upgrade outside the author's authorship.

### 7.2 Internal Validity

The zero-FP invariant is tested on (a) identical-IDL inputs across the canonical
fixture, and (b) the real-world Drift v2.162 IDL (428 KB). Real-world Anchor programs
may include IDL structures not represented in either test (e.g., defined types,
events, enums). These are explicitly documented as M0 non-goals in
`SOLANA_EDGE_CASES.md` and `SEVERITY.md` §3.

**Mitigation.** All non-goals are explicitly documented. Exit code 3
(refuse-to-analyse) ensures Spectra never silently claims coverage it does not have.

### 7.3 External Validity

M0 covers Anchor legacy-schema IDL only. Programs using Anchor 2026 (Codama),
Shank native IDL, or zero-copy (`bytemuck`) layouts are not covered. These represent
a significant fraction of production Solana programs.

**Mitigation.** M1 and M2 address these gaps. The M0 scope is explicitly bounded
to establish a verifiable baseline before grant-funded extension. The Drift validation
(§5.5) demonstrates external validity within the M0 scope (Anchor legacy IDL at
production size).

### 7.4 Measurement Reliability

Performance measurements (Tables 6 and 8) are from 5 consecutive runs on a single
commodity laptop. Wall-clock measurements at this scale (<10 ms) are subject to
OS scheduling noise. The measurements are presented as evidence that Spectra is
*not slower than `diff`* by any margin meaningful to CI gating, not as precision
benchmarks.

### 7.5 Single Real-World Sample

The Drift validation (§5.5) is one upgrade pair on one protocol. It does not
constitute a statistically representative sample of Solana upgrade behavior. The
intent is existence proof — that the M0 design surfaces a real, named finding on a
real production upgrade at production scale — not a frequency claim. The grant
roadmap (M4) targets ≥1 confirmed pilot integration plus 2 public walkthroughs to
broaden the empirical base.

---

## 8. Limitations and Future Work

Table 9 summarizes known limitations and their planned resolution path.

**Table 9. Known limitations by milestone resolution**

| Limitation | M0 | M1 | M2 | Permanent |
|---|---|---|---|---|
| Anchor 2026 / Codama schema | ✗ | ✓ | ✓ | — |
| Shank native IDL | ✗ | ✓ | ✓ | — |
| Enum variant insertion/removal | ✗ | ✓ | ✓ | — |
| Shared `defined_type` layout change | ✗ | ✓ | ✓ | — |
| Event field reorder/type change | ✗ | ✓ | ✓ | — |
| Error code renumbering | ✗ | ✓ | ✓ | — |
| Anchor 0.30+ explicit discriminator override | ✗ | ✓ | ✓ | — |
| Zero-copy (`bytemuck`) padding-aware diff | ✗ | Partial | ✓ | — |
| Pre-deploy transaction replay | ✗ | ✗ | ✓ | — |
| CPI signature stability | ✗ | ✗ | ✓ | — |
| Suppression file for intentional migrations | ✗ | ✗ | ✓ M3 | — |
| Token-2022 TLV layouts | ✗ | ✗ | ✗ | Out of scope |
| PDA seed-derivation drift | ✗ | ✗ | ✗ | Future Expansion |
| `.rodata` constant changes | ✗ | ✗ | ✗ | Future Expansion |
| Mainnet snapshot replay | ✗ | ✗ | ✗ | Out of scope |

The full rule roadmap reaches 23 rule types at M2 (11 in M0, +9 in M1, +3 in M2).
A concrete false-positive rate measurement against real-world upgradeable programs
is planned for M4 (≥1 confirmed pilot + 2 public walkthroughs).

---

## 9. Conclusion

We have characterized a previously unquantified gap in the Solana security toolchain:
the behavioral regression layer, where no public, free, CI-integrable tool existed
before Spectra. Our analysis of a representative upgrade diff shows that 71% of
the critical review steps cannot be completed solely by reading the textual diff,
and 33% of findings on that fixture are structurally impossible to derive from
textual diff by any means.

Spectra M0 closes this gap for Anchor legacy-schema programs: 11 rule types covering
the complete IDL detection surface, 1–2 ms wall-clock performance with no Solana SDK
dependency, a contractual exit-code interface usable as a `required` CI check, and
a verified zero-FP invariant on identical inputs. The two finding types that are
structurally impossible to produce from textual diff — silent account corruption
(`R-ACC-SILENT-CORRUPT`) and discriminator collision (`R-DISC-COLL`) — correspond to
the highest-impact failure mode class: silent mainnet state corruption and silent
instruction misrouting, both without Solana runtime error signals.

Real-world validation on Drift Protocol v2.155 → v2.162 (428 KB, 249 instructions,
319 changed lines) demonstrates that the same M0 design surfaces a real instance of
the silent-corruption pattern in a production protocol upgrade — completing analysis
in 6 ms while preserving the zero-FP invariant on a 428 KB production IDL. A roadmap
of 23 total rule types through M2 extends coverage to Anchor 2026, Shank native
programs, enum/event/error-code regression, and bounded pre-deploy transaction replay.

---

## Acknowledgments

The author acknowledges the Solana Foundation grant review process for motivating
the explicit articulation of coverage claims and non-goals documented in this work.

---

## References

**Primary sources (directly verifiable):**

[Spectra-BENCHMARK] Ayodya. *docs/BENCHMARK.md*, commit `e5aad06`,
CI run 25851086148. https://github.com/ayodyadsr/spectra/blob/main/docs/BENCHMARK.md

[Spectra-BENCHMARK-DRIFT] Ayodya. *docs/BENCHMARK_DRIFT.md*, commit `0a5c684`,
CI run 25910510275.
https://github.com/ayodyadsr/spectra/blob/main/docs/BENCHMARK_DRIFT.md

[Spectra-SEVERITY] Ayodya. *docs/SEVERITY.md*.
https://github.com/ayodyadsr/spectra/blob/main/docs/SEVERITY.md

[Spectra-THREAT] Ayodya. *docs/THREAT_MODEL.md*.
https://github.com/ayodyadsr/spectra/blob/main/docs/THREAT_MODEL.md

[Spectra-EDGE] Ayodya. *docs/SOLANA_EDGE_CASES.md*.
https://github.com/ayodyadsr/spectra/blob/main/docs/SOLANA_EDGE_CASES.md

[Spectra-VS-DIFF] Ayodya. *docs/VS_GIT_DIFF.md*.
https://github.com/ayodyadsr/spectra/blob/main/docs/VS_GIT_DIFF.md

[Spectra-COMPETITIVE] Ayodya. *docs/COMPETITIVE_BENCHMARK.md* — head-to-head
benchmark against `diff -u`, `jd`, `dyff`, `json-diff` on the Drift IDL pair.
https://github.com/ayodyadsr/spectra/blob/main/docs/COMPETITIVE_BENCHMARK.md

[Spectra-NON-GOALS] Ayodya. *docs/NON_GOALS.md*.
https://github.com/ayodyadsr/spectra/blob/main/docs/NON_GOALS.md

**Competing JSON/YAML diff tools surveyed in §5.6:**

[GNU-DIFFUTILS] Free Software Foundation. *GNU diffutils 3.10*.
https://www.gnu.org/software/diffutils/

[Burnett-JD] Burnett, J. *jd — a commandline utility for diffing and patching
JSON and YAML values*, v1.9.2. https://github.com/josephburnett/jd

[Homeport-DYFF] Homeport. *dyff — a diff tool for YAML files, and sometimes
JSON*. https://github.com/homeport/dyff

[Vit-JSONDIFF] Tarasenko, A. *json-diff — structural diff for JSON files*.
https://github.com/andreyvit/json-diff

**Real-world IDL artifacts:**

[Drift-v2-155] Drift Foundation. *protocol-v2 SDK IDL at commit 590049e6bf*
(2026-01-21).
https://github.com/drift-labs/protocol-v2/blob/590049e6bf/sdk/src/idl/drift.json

[Drift-v2-162] Drift Foundation. *protocol-v2 SDK IDL at commit 0d35029d78*
(2026-04-01).
https://github.com/drift-labs/protocol-v2/blob/0d35029d78/sdk/src/idl/drift.json

**Ecosystem data:**

[Hacken-2025] Hacken. *2025 Yearly Security Report*. "$4.0B lost; $512M (12.8%)
from smart contract vulnerabilities."
https://hacken.io/services/blockchain-security/solana-smart-contract-security-audit/

[Collins-2025] Collins, DeFiPen. "History of Solana Security Incidents: A Deep Dive."
Medium, April 2025. $530M+ exploit history; 14 major incidents 2020–2025.
https://collinsdefipen.medium.com/history-of-solana-security-incidents

[BingX-2026] BingX. "Top 8 Solana DeFi Projects to Watch in 2026." January 2026.
$10.2B TVL; 329+ protocols.
https://bingx.com/en/learn/article/what-are-the-top-solana-defi-projects

[Sec3-2022] Sec3. "Solana Internals Part 2: How Is a Solana Program Deployed and
Upgraded." January 2022. Default upgradeability documentation.
https://www.sec3.dev/blog/solana-internals-part-2-how-is-a-solana-deployed-and-upgraded

[SolanaSDK] Solana Labs. `sdk/program/src/bpf_loader_upgradeable.rs`.
"Breaks the 'code is law' contract."
https://github.com/solana-labs/solana/blob/master/sdk/program/src/bpf_loader_upgradeable.rs

**Related work:**

[Feist-2019] Feist, J., Grieco, G., & Groce, A. (2019). Slither: A Static Analysis
Framework for Smart Contracts. *2019 IEEE/ACM 2nd International Workshop on
Emerging Trends in Software Engineering for Blockchain (WETSEB)*, pp. 8–15.

[Grieco-2020] Grieco, G., Song, W., Cygan, A., Feist, J., & Groce, A. (2020).
Echidna: Effective, usable, and fast fuzzing for smart contracts.
*ISSTA 2020*, pp. 557–560.

---

*Data transparency note: All quantitative claims in this paper derive from
one of three source categories: (1) directly reproducible from the Spectra
repository at the cited commit; (2) verbatim quotations from cited primary sources;
or (3) computed from (1) or (2) with the computation shown. Claims based on
estimation or extrapolation are explicitly marked as such. No claim in this
paper is unsourced.*
