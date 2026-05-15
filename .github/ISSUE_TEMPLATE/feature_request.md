---
name: Feature request
about: Propose a new rule, output format, or workflow integration for Spectra
title: "[feature] "
labels: enhancement
assignees: ayodyadsr
---

> Spectra's scope is intentionally narrow: **security-critical
> behavioural-regression diffing for Solana program upgrades**. Feature
> requests outside that scope (general-purpose IDL prettifiers, runtime
> tracing, fuzzing harnesses, etc.) will be politely declined — see
> [`docs/NON_GOALS.md`](../../docs/NON_GOALS.md).

## What problem does this solve?

<!--
One paragraph. Who is affected, what goes wrong today, and how often.
Concrete examples (program names, commit ranges, IDL snippets) help a
lot.
-->

## Proposed rule / behaviour

<!--
If this is a new detection rule, please describe:
- The breaking-change pattern (with a minimal IDL diff if possible)
- Proposed severity (BREAKING / WARN / INFO — see docs/SEVERITY.md)
- Proposed rule ID (e.g. `account-field-tag-changed`)
- Expected exit code under the current contract
-->

## Alternatives considered

<!--
Existing tools, manual workflows, or other rules that partially cover
this. If `diff -u`, `jd`, or another diff tool already catches it,
explain what they miss.
-->

## Additional context

<!--
Links to upstream Anchor / SPL changes, related GitHub issues, prior
incidents, or relevant rows in docs/SOLANA_EDGE_CASES.md.
-->
