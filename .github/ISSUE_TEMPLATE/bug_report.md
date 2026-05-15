---
name: Bug report
about: Spectra produced an incorrect finding, a wrong exit code, a crash, or a confusing message
title: "[bug] "
labels: bug
assignees: ayodyadsr
---

> If this is a **security finding** in Spectra itself (Spectra mislabels a
> dangerous upgrade as safe, mis-routes a discriminator, etc.) — do **not**
> file it here. Please report privately per [SECURITY.md](../../SECURITY.md).

## Summary

<!-- One sentence describing the wrong behaviour. -->

## Environment

- `spectra --version`:
- OS + architecture:
- `rustc --version` (if you built from source):
- Commit hash (`git rev-parse HEAD`):

## Reproduction

Minimum commands and IDL snippets to reproduce:

```bash
spectra check --old <path> --new <path> --format <fmt>
```

If the IDLs can be shared publicly, please attach minimised versions
(strip program-specific fields that are not load-bearing for the bug).

## Expected behaviour

<!-- What Spectra should have done, with the rule ID from docs/SEVERITY.md if applicable. -->

## Actual behaviour

```text
<paste the actual Spectra output here>
```

## Additional context

<!-- Links to commits, prior issues, related rules in docs/SOLANA_EDGE_CASES.md, etc. -->
