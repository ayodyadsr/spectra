# Security Policy

Spectra is a security tool. Vulnerabilities in Spectra itself can mislead
reviewers into trusting an unsafe Solana program upgrade. They are taken
seriously.

## Supported versions

| Version | Status |
|---|---|
| `main` branch HEAD | ✅ Supported. All security fixes land here first. |
| Pre-1.0 tagged releases | ✅ Supported during the grant period. |

Once a `1.0` release is cut, the supported-version table will be updated to
list the latest minor release as supported.

## Reporting a vulnerability

**Do not file a public GitHub issue for an exploitable security finding in
Spectra.**

Instead, contact the maintainer privately at **ayodyadsr@gmail.com**. Please
include:

- A description of the finding and its impact.
- Step-by-step reproduction (commands, fixtures, expected vs observed
  Spectra output).
- The commit hash you reproduced on, and your `cargo --version` /
  `rustc --version`.
- Any proposed mitigation, if you have one.

Encrypted reports are welcome — request a PGP key in your first message and
one will be returned out-of-band.

### Response SLAs (during the grant period)

| Event | Target SLA |
|---|---|
| Acknowledgement of report | 48 hours |
| Triage + severity assignment | 7 days |
| Fix or documented mitigation for a confirmed Critical or High finding | 30 days |
| Public disclosure (after fix lands) | 90 days from report, or sooner by agreement |

For non-exploitable robustness issues (incorrect findings, false negatives,
hangs on malformed input), please file a regular GitHub issue using the
**Bug report** template.

## Scope

In scope for this policy:

- The Spectra Rust crate (`spectra-core/`) and CLI binary.
- The Python wrapper (`spectra-cli/`).
- The composite GitHub Action scaffold (`spectra-action/`).
- The CI workflow (`.github/workflows/ci.yml`) and Docker image
  (`Dockerfile`).

Out of scope:

- Vulnerabilities in upstream dependencies that Spectra does not magnify
  (please report those upstream). Spectra will track and pull upgrades for
  RUSTSEC advisories on its direct dependencies during the grant period.
- Guard regressions Spectra does not model because the compensating check
  lives outside `#[derive(Accounts)]` (e.g. a manual `require!()` in the
  instruction body — the M1 native path narrows this) — those are
  documented coverage boundaries, not security bugs in Spectra. File a
  regular issue with the `enhancement` label and reference
  [`docs/NON_GOALS.md`](docs/NON_GOALS.md) /
  [`docs/FALSE_POSITIVES.md`](docs/FALSE_POSITIVES.md).

## Acknowledgements

Contributors who responsibly disclose confirmed vulnerabilities will be
credited in the project changelog at their preferred name + handle, unless
they request to remain anonymous.
