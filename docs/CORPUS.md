# Detection Corpus Design

The detection corpus is the set of inputs Spectra is regression-tested against. It is **the** truth source for "does Spectra still detect what it claimed to detect last release."

The corpus is intentionally bounded. Spectra is not fuzz-derived and does not chase coverage metrics; it grows the corpus only when a new finding kind is shipped or a real-world regression is filed.

---

## 1. Three-layer corpus

| Layer | Purpose | Size | Source |
|-------|---------|------|--------|
| Synthetic | Exercise every rule deterministically | 1 fixture pair per rule | Hand-authored, lives in `examples/` and `tests/fixtures/` |
| Real-world | Validate against actual Anchor program upgrades | ≤ 5 historic upgrades per supported schema | Public on-chain upgrade history, with permission of the protocol team |
| Adversarial | Cases designed to **trip** Spectra | Grows over time | Reviewer-submitted PRs + audit-firm contributions |

Every entry has:

- A `.json` IDL pair (or `.so` pair in M2).
- An `expected.json` report file under test.
- A `notes.md` describing what the corpus item exercises.

---

## 2. Synthetic layer (M0 shipped + M1 planned)

M0 has one synthetic fixture pair (`examples/lending_v1.json` vs `examples/lending_v2.json`) covering 4 BREAKING + 2 warning rule classes. M1 expands to one fixture pair per rule in [SEVERITY.md](SEVERITY.md).

Each synthetic fixture is **minimal**: it exercises exactly one rule, plus the smallest surrounding IDL needed to be schema-valid. This keeps the diff in the expected report visually inspectable.

---

## 3. Real-world layer (M4)

Real-world entries are upgrades that **actually shipped on Solana mainnet**, captured before/after, with the protocol team's permission. Each entry includes:

- Upgrade tx signature.
- The IDL before and after the upgrade.
- Real findings Spectra produced.
- The protocol team's review classifying each finding.

This layer is what gives Spectra real-world calibration. It is also the data backing the FP-rate table in [ROADMAP.md](ROADMAP.md) M4.c.

---

## 4. Adversarial layer (ongoing)

The adversarial layer is the slow-growing set of fixtures designed to break Spectra. Three sources feed it:

1. Audit-firm contributions: targeted cases the firm uses against client programs.
2. CTF-style submissions: a fixture pair where the "wrong answer" is obvious to a human and Spectra must produce the corresponding rule ID.
3. Post-mortem additions: every real-world regression Spectra fails to catch becomes a new adversarial fixture in the same patch that adds the missing detector.

Adversarial fixtures are tagged `adversarial = true` and are exempt from the "minimal" rule — they may include realistic complexity.

---

## 5. Corpus authoring rules

| Rule | Reason |
|------|--------|
| Each fixture pair lives in its own directory with `old.json`, `new.json`, `expected.json`, `notes.md`. | Reviewability. |
| `expected.json` is the **canonical** output. Any change to expected output is a deliberate Spectra contract change. | Stability. |
| No fixture references a private IDL or unpublished protocol. | Public-by-default. |
| No fixture references a real address from a protocol that has not granted permission. | Pilot consent. |
| Adversarial fixtures must be reproducible — no random seeds, no clock, no network. | Determinism. |

---

## 6. CI invariant

For every commit, CI runs:

```
spectra check --old <fixture>/old.json --new <fixture>/new.json --format json > /tmp/out.json
diff -u <fixture>/expected.json /tmp/out.json
```

A diff is a CI failure. This is what stops a refactor of the diff engine from silently changing the report shape.

M0 ships this loop for the lending fixture; M1 extends it to one fixture pair per rule.

---

## 7. What the corpus is not

- It is **not** a fuzzer. No fuzz harness is in scope for M0–M3.
- It is **not** mainnet-scale. M2 is bounded to ≤ 50 tx per pilot for the replay layer; the structural corpus is bounded by hand-authoring rate.
- It is **not** an exhaustive coverage proof. It is calibration against representative cases.

---

## 8. Cross-references

- Replay layer corpus shape: [REPLAY.md](REPLAY.md).
- Rule IDs the corpus indexes against: [SEVERITY.md](SEVERITY.md).
- Pilots that contribute real-world fixtures: [ADOPTION.md](ADOPTION.md).
