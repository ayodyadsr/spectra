# Rule Engine

The rule engine is the part of Spectra that owns the mapping from `(old_idl, new_idl)` to the set of findings. The M0 implementation is a flat module of comparator functions; M1 refactors it into a `Rule` trait + `RuleRegistry` so new rules can be added without touching the diff driver.

This document describes both the current shape and the target shape.

---

## 1. M0 shape (shipped)

The M0 engine lives in `spectra-core::diff::diff_idls`. It is a sequence of named comparators that each push findings into a shared `Vec<Finding>`:

```rust
pub fn diff_idls(old: &Idl, new: &Idl) -> Vec<Finding> {
    let mut findings = Vec::new();
    diff_instructions(old, new, &mut findings);
    diff_accounts(old, new, &mut findings);
    detect_silent_corruption(&mut findings);
    detect_discriminator_collisions(new, &mut findings);
    findings.sort_by(|a, b| /* stable */);
    findings
}
```

Each comparator is a pure function over `(old, new)` and produces specific `Finding::*` variants. Severity is assigned by the `severity()` function on `Finding`, which is the canonical lookup defined in [SEVERITY.md](SEVERITY.md).

This shape is fine for M0's 11 rules. It is not scalable to M1's 20+ rules.

---

## 2. M1 target shape

M1 introduces a `Rule` trait:

```rust
pub trait Rule: Send + Sync {
    /// Stable rule ID, e.g. "R-ACC-FIELD-REORDER".
    fn id(&self) -> &'static str;

    /// Severity classification — must match SEVERITY.md.
    fn severity(&self) -> Severity;

    /// Run the rule against an IDL pair and emit findings.
    fn check(&self, old: &Idl, new: &Idl, sink: &mut dyn FindingSink);
}

pub trait FindingSink {
    fn emit(&mut self, finding: Finding);
}
```

A `RuleRegistry` holds the ordered list of rules:

```rust
pub struct RuleRegistry {
    rules: Vec<Box<dyn Rule>>,
}

impl RuleRegistry {
    pub fn default() -> Self {
        let mut r = Self { rules: Vec::new() };
        r.register(Box::new(InstructionRemovedRule));
        r.register(Box::new(InstructionArgsChangedRule));
        // ... one entry per rule in SEVERITY.md
        r
    }

    pub fn run(&self, old: &Idl, new: &Idl) -> Vec<Finding> {
        let mut findings = Vec::new();
        let mut sink = VecSink(&mut findings);
        for rule in &self.rules {
            rule.check(old, new, &mut sink);
        }
        findings.sort_by(stable_order);
        findings
    }
}
```

Properties:

- **Rule isolation.** A bug in one rule cannot corrupt another rule's findings.
- **Stable ordering.** Findings are sorted by a fixed key (rule id, then target name) so report output is reproducible.
- **Discoverability.** `RuleRegistry::ids()` returns the full list of registered rule IDs — useful for documenting "what is currently in the engine" and for future per-rule suppression UI.

---

## 3. Determinism requirements on rules

Every rule must satisfy:

| Requirement | Reason |
|-------------|--------|
| Pure function of `(old, new)` | Same input -> same output. |
| No `HashMap` iteration with non-deterministic seed for finding emission | Output order matters. Use `BTreeMap` or sort before emitting. |
| No network, no clock, no `/dev/urandom` | Reproducibility across hosts. |
| No panic on malformed input it can recognise as malformed | Refuse-to-analyse (exit 3) is the documented path. |

Rules that need to share precomputed state (e.g. the discriminator map for collision detection) do so via an explicit context object, not via global mutable state.

---

## 4. How a new rule is added

1. Add the `Finding::*` variant to `spectra-core::diff::Finding`.
2. Add the rule ID + severity to [SEVERITY.md](SEVERITY.md).
3. Implement the `Rule` impl in `spectra-core::diff::rules::<rule_name>`.
4. Register it in `RuleRegistry::default()`.
5. Add one synthetic fixture pair to `tests/fixtures/<rule_id>/`.
6. Add the `cargo test` assertion against the expected report.
7. Update [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md) if the rule changes the coverage matrix.

The contract for the change is documented; the CI loop enforces it.

---

## 5. Why not a config-driven rule DSL

A pluggable rule DSL ("write your detector in YAML") has been considered and explicitly rejected for M0–M3:

- Rules are sparse — under 30 in total through M3. A DSL pays its complexity cost only past ~100 rules.
- IDL semantics (positional Borsh layout, discriminator algorithm) are non-trivial; expressing them in a DSL pushes the difficulty into the DSL definition rather than removing it.
- Audit firms reviewing Spectra's claims want to read **Rust**, not a custom DSL.

A DSL may be revisited as Future Expansion if the rule count grows substantially.

---

## 6. Cross-references

- Rule list: [SEVERITY.md](SEVERITY.md).
- Architecture context: [ARCHITECTURE.md](ARCHITECTURE.md).
- Edge-case coverage that rule additions track against: [SOLANA_EDGE_CASES.md](SOLANA_EDGE_CASES.md).
