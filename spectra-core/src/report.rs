//! Report renderers for [`RegressionReport`]: Markdown for humans, SARIF
//! 2.1.0 for GitHub Code Scanning. JSON is produced directly via `serde_json`
//! in the CLI entry point and needs no dedicated renderer.

use crate::regression::{Finding, RegressionReport, Severity};
use serde_json::{json, Value};

/// Render the regression report as a human-readable Markdown document.
pub fn render_markdown(report: &RegressionReport) -> String {
    let mut out = String::new();
    out.push_str("# Spectra Account-Validation Regression Report\n\n");
    out.push_str(&format!(
        "**Findings:** {} breaking, {} warning\n\n",
        report.breaking_count, report.warning_count
    ));

    if report.findings.is_empty() {
        out.push_str(
            "No account-validation regressions detected. This upgrade does not remove or weaken any owner / signer / type / `has_one` / PDA / CPI guard that the baseline enforced.\n",
        );
        return out;
    }

    out.push_str("| Severity | Rule | Detail |\n");
    out.push_str("|---|---|---|\n");
    for f in &report.findings {
        let sev = match f.severity() {
            Severity::Breaking => "BREAKING",
            Severity::Warning => "warning",
        };
        out.push_str(&format!("| {} | {} | {} |\n", sev, rule_id(f), detail(f)));
    }

    if report.breaking_count > 0 {
        out.push_str(
            "\n> Spectra exits non-zero when any BREAKING finding is present: this upgrade takes away a security guarantee the deployed version already gave its users. Review each row before deploy.\n",
        );
    }

    out
}

/// SARIF 2.1.0 for GitHub Advanced Security ingestion. Each finding maps to a
/// SARIF `result`; the candidate source tree is recorded as the artifact
/// location and the `Context::account` pair as the logical location.
pub fn render_sarif(report: &RegressionReport, candidate_path: &str) -> String {
    let rules: Vec<Value> = rule_catalog()
        .iter()
        .map(|(id, name, help, level)| {
            json!({
                "id": id,
                "name": name,
                "shortDescription": { "text": *help },
                "fullDescription": { "text": *help },
                "defaultConfiguration": { "level": *level },
                "helpUri": format!(
                    "https://github.com/ayodyadsr/spectra/blob/main/docs/SEVERITY.md#{}",
                    id
                ),
            })
        })
        .collect();

    let results: Vec<Value> = report
        .findings
        .iter()
        .map(|f| finding_to_sarif_result(f, candidate_path))
        .collect();

    let doc = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "Spectra",
                    "version": env!("CARGO_PKG_VERSION"),
                    "informationUri": "https://github.com/ayodyadsr/spectra",
                    "rules": rules,
                }
            },
            "results": results,
            "properties": {
                "breaking_count": report.breaking_count,
                "warning_count": report.warning_count,
            }
        }]
    });

    serde_json::to_string_pretty(&doc).expect("SARIF serialisation cannot fail")
}

fn rule_catalog() -> &'static [(&'static str, &'static str, &'static str, &'static str)] {
    &[
        (
            "signer_check_removed",
            "SignerCheckRemoved",
            "Baseline required this account to sign; the candidate no longer does. Canonical missing-signer-check bug introduced on upgrade.",
            "error",
        ),
        (
            "type_cosplay_protection_removed",
            "TypeCosplayProtectionRemoved",
            "A typed Anchor wrapper enforcing owner + discriminator was downgraded to UncheckedAccount/AccountInfo; type-cosplay protection lost.",
            "error",
        ),
        (
            "owner_check_removed",
            "OwnerCheckRemoved",
            "Baseline pinned the account owner / address; the candidate dropped the pin.",
            "error",
        ),
        (
            "has_one_constraint_removed",
            "HasOneConstraintRemoved",
            "Baseline enforced a has_one relational-integrity check the candidate dropped.",
            "error",
        ),
        (
            "custom_constraint_removed",
            "CustomConstraintRemoved",
            "Baseline enforced a custom #[account(constraint = ...)] predicate the candidate dropped.",
            "error",
        ),
        (
            "pda_derivation_removed",
            "PdaDerivationRemoved",
            "Baseline derived this account as a PDA (seeds/bump); the candidate dropped the derivation, allowing an arbitrary account.",
            "error",
        ),
        (
            "cpi_target_unpinned",
            "CpiTargetUnpinned",
            "Baseline pinned a CPI target program id; the candidate downgraded it to an unvalidated account (arbitrary-program-invocation hazard).",
            "error",
        ),
        (
            "validated_account_slot_removed",
            "ValidatedAccountSlotRemoved",
            "A validated account slot present in the baseline context was removed while the context still exists.",
            "error",
        ),
        (
            "unvalidated_account_introduced",
            "UnvalidatedAccountIntroduced",
            "The candidate introduces a new UncheckedAccount/AccountInfo slot absent from the baseline. New attack surface to review.",
            "warning",
        ),
    ]
}

fn finding_to_sarif_result(f: &Finding, candidate_path: &str) -> Value {
    let level = match f.severity() {
        Severity::Breaking => "error",
        Severity::Warning => "warning",
    };
    let rule = rule_id(f);
    json!({
        "ruleId": rule,
        "level": level,
        "message": { "text": format!("{}: {}", rule, detail(f)) },
        "locations": [{
            "physicalLocation": { "artifactLocation": { "uri": candidate_path } },
            "logicalLocations": [{
                "name": logical_name(f),
                "kind": "member",
            }],
        }],
    })
}

fn logical_name(f: &Finding) -> String {
    let (c, a) = ctx_acct(f);
    format!("{}::{}", c, a)
}

fn ctx_acct(f: &Finding) -> (&str, &str) {
    match f {
        Finding::SignerCheckRemoved { context, account }
        | Finding::TypeCosplayProtectionRemoved {
            context, account, ..
        }
        | Finding::OwnerCheckRemoved {
            context, account, ..
        }
        | Finding::HasOneConstraintRemoved {
            context, account, ..
        }
        | Finding::CustomConstraintRemoved {
            context, account, ..
        }
        | Finding::PdaDerivationRemoved { context, account }
        | Finding::CpiTargetUnpinned { context, account }
        | Finding::ValidatedAccountSlotRemoved { context, account }
        | Finding::UnvalidatedAccountIntroduced { context, account } => (context, account),
    }
}

fn rule_id(f: &Finding) -> &'static str {
    match f {
        Finding::SignerCheckRemoved { .. } => "signer_check_removed",
        Finding::TypeCosplayProtectionRemoved { .. } => "type_cosplay_protection_removed",
        Finding::OwnerCheckRemoved { .. } => "owner_check_removed",
        Finding::HasOneConstraintRemoved { .. } => "has_one_constraint_removed",
        Finding::CustomConstraintRemoved { .. } => "custom_constraint_removed",
        Finding::PdaDerivationRemoved { .. } => "pda_derivation_removed",
        Finding::CpiTargetUnpinned { .. } => "cpi_target_unpinned",
        Finding::ValidatedAccountSlotRemoved { .. } => "validated_account_slot_removed",
        Finding::UnvalidatedAccountIntroduced { .. } => "unvalidated_account_introduced",
    }
}

fn detail(f: &Finding) -> String {
    match f {
        Finding::SignerCheckRemoved { context, account } => format!(
            "`{}::{}` no longer requires a signer (baseline did)",
            context, account
        ),
        Finding::TypeCosplayProtectionRemoved {
            context,
            account,
            baseline_type,
            candidate_type,
        } => format!(
            "`{}::{}` downgraded `{}` -> `{}` (owner+discriminator check lost)",
            context, account, baseline_type, candidate_type
        ),
        Finding::OwnerCheckRemoved {
            context,
            account,
            baseline_pin,
        } => format!(
            "`{}::{}` dropped owner/address pin `{}`",
            context, account, baseline_pin
        ),
        Finding::HasOneConstraintRemoved {
            context,
            account,
            target,
        } => format!("`{}::{}` dropped `has_one = {}`", context, account, target),
        Finding::CustomConstraintRemoved {
            context,
            account,
            expr,
        } => format!("`{}::{}` dropped `constraint = {}`", context, account, expr),
        Finding::PdaDerivationRemoved { context, account } => format!(
            "`{}::{}` dropped PDA `seeds`/`bump` derivation",
            context, account
        ),
        Finding::CpiTargetUnpinned { context, account } => format!(
            "`{}::{}` CPI target program id no longer pinned",
            context, account
        ),
        Finding::ValidatedAccountSlotRemoved { context, account } => format!(
            "`{}::{}` validated account slot removed from context",
            context, account
        ),
        Finding::UnvalidatedAccountIntroduced { context, account } => format!(
            "`{}::{}` new UncheckedAccount/AccountInfo slot (new attack surface)",
            context, account
        ),
    }
}
