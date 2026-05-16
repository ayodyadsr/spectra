---
name: Feature request
about: Propose a new rule, output format, or workflow integration for Spectra
title: "[feature] "
labels: enhancement
assignees: ayodyadsr
---

> Spectra's scope is intentionally narrow: **strictly-differential
> account-validation regression gating for Solana program upgrades**.
> Feature requests outside that scope (absolute scanning of already-missing
> checks, IDL diffing, runtime tracing, fuzzing harnesses, etc.) will be
> politely declined — see [`docs/NON_GOALS.md`](../../docs/NON_GOALS.md).

## What problem does this solve?

<!--
One paragraph. Who is affected, what goes wrong today, and how often.
Concrete examples (program names, commit ranges, IDL snippets) help a
lot.
-->

## Proposed rule / behaviour

<!--
If this is a new detection rule, please describe:
- The guard-regression pattern (with a minimal baseline/candidate
  `#[derive(Accounts)]` diff if possible)
- Proposed severity (BREAKING / warning — see docs/SEVERITY.md)
- Proposed rule ID (e.g. `owner_check_removed`)
- Expected exit code under the current contract (`0`/`1`/`2`)
-->

## Alternatives considered

<!--
Existing tools, manual workflows, or other rules that partially cover
this. If `git diff`, an absolute scanner (Sec3 X-Ray, Auditware Radar,
l3x, Octane), or another tool already catches it, explain what they miss.
-->

## Additional context

<!--
Links to upstream Anchor / SPL changes, related GitHub issues, prior
incidents, or relevant rows in docs/SEVERITY.md.
-->
