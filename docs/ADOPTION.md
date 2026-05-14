# Adoption Strategy

Tooling that lives only as a repo is not tooling. This document is the explicit plan for how Spectra reaches enough Solana programs that the FP-rate measurement is real-world calibrated and the protocol-level value is demonstrable.

The plan is concrete, not aspirational. Every step lists a measurable outcome and the deliverable that demonstrates it.

---

## 1. Zero-friction integration

The first adoption friction is "how many YAML lines to deploy this." M3 collapses Spectra integration to a single composite Action step:

```yaml
- uses: ayodyadsr/spectra-action@v1
  with:
    program: <program-name>
    suppression-file: spectra-allow.toml
    post-pr-comment: true
```

**Outcome:** A protocol can integrate Spectra in one PR that touches one file.

**Deliverable:** `spectra-action` published in M3, referenced from [CI_INTEGRATION.md](CI_INTEGRATION.md).

---

## 2. PR visibility

Spectra outputs are intentionally **PR-comment-friendly**: the markdown report is the same shape a reviewer would write by hand. Each finding cites:

- The rule ID.
- The specific symbol affected.
- A one-line explanation of the consequence ("Borsh layout reorder corrupts existing accounts").
- The link back to [SEVERITY.md](SEVERITY.md).

**Outcome:** A reviewer who has never seen Spectra before can read a finding and act on it without leaving the PR.

**Deliverable:** Markdown report shape is fixed and tested; M3 Action posts it idempotently.

---

## 3. Audit-firm anchoring

The fastest credibility path for a security tool on Solana is firm endorsement. The pilot recruitment plan targets:

| Channel | Why |
|---------|-----|
| Sec3 | Active Solana audit + tooling shop. |
| OtterSec | Audits the largest Anchor programs; ships `solana-verifiable-build` / `solana-verify` (build provenance, complementary to Spectra). |
| Neodyme | Audits validator / runtime + program code. |
| Halborn | Multi-chain firm with active Solana practice. |
| Range / Hypernative | Runtime-monitoring vendors who consume the same upgrade-event surface. |

LOI outreach copy is drafted at [`02_proposals/drafts/solana-program-verification-tooling/loi_outreach.md`](../../02_proposals/drafts/solana-program-verification-tooling/loi_outreach.md). The proposal commits to **≥ 1 confirmed pilot + 2 public walkthroughs**, not "3 firms committed."

**Outcome:** At least one audit firm or upgradable-protocol team is publicly running Spectra against a real program by M4.

**Deliverable:** The pilot's CI workflow file is linked from this repo.

---

## 4. Public walkthroughs

Two public walkthroughs are explicit M4 deliverables:

1. Walkthrough A: an Anchor program's real upgrade where Spectra catches a layout-breaking change (true positive).
2. Walkthrough B: an Anchor program's coordinated migration where Spectra surfaces findings that the maintainer suppresses via `spectra-allow.toml` (true positive + correctly waived).

Each walkthrough is a write-up + commits + CI run links. Format: plain markdown, no marketing.

**Outcome:** A new protocol team can read a walkthrough and decide adoption without contacting the maintainer.

**Deliverable:** Two markdown write-ups linked from the README under "Walkthroughs."

---

## 5. Trust signals

Adoption is also gated by the **looks-trustworthy** signals reviewers check:

| Signal | Status |
|--------|--------|
| MIT license | ✅ (LICENSE in repo) |
| CI green on every push | ✅ (badge in README) |
| Asciinema cast committed | ✅ (`demo.cast`) |
| Per-rule documentation | ✅ ([SEVERITY.md](SEVERITY.md)) |
| Threat model documented | ✅ ([THREAT_MODEL.md](THREAT_MODEL.md)) |
| Non-goals documented | ✅ ([NON_GOALS.md](NON_GOALS.md)) |
| Failure modes documented | ✅ ([THREAT_MODEL.md](THREAT_MODEL.md) §3) |
| Determinism guarantees documented | ✅ ([ARCHITECTURE.md](ARCHITECTURE.md) §5) |
| Issue triage SLA stated | ✅ (CONTRIBUTING.md: 7-day during grant period) |
| Security disclosure policy | ⏳ Published after grant decision |

These are the items a senior reviewer scans before reading code. The list is short on purpose.

---

## 6. Discoverability

- Solana Discord — AMA scheduled in M4.
- Solana Forum — proposal references the open RFP at `forum.solana.com/t/program-verification-tooling/1032` so the work is discoverable from the RFP thread.
- `mdBook` docs — published in M4 under `docs.spectra.tools` (domain to be registered with grant funds; budget line absent today, will be added pre-publish if claimed).

---

## 7. What adoption is **not**

- **Not download counts.** Spectra is a CI tool, not an end-user binary. Adoption is measured by pilot integrations and walkthroughs, not stars or downloads.
- **Not marketing partnerships.** No co-marketing announcements are in scope.
- **Not aggressive promotion.** The proposal commits to a single Discord AMA in M4. No multi-channel push.

---

## 8. Cross-references

- LOI outreach copy: `02_proposals/drafts/solana-program-verification-tooling/loi_outreach.md`.
- Pilot acceptance tests: [ROADMAP.md](ROADMAP.md) §M4.
- CI integration shape pilots will deploy: [CI_INTEGRATION.md](CI_INTEGRATION.md).
