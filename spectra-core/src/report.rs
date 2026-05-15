//! Report renderers for [`DiffReport`]: Markdown for humans, SARIF 2.1.0 for
//! GitHub Code Scanning. JSON is produced directly via `serde_json` in the
//! CLI entry point and does not need its own renderer.

use crate::diff::{DiffReport, Finding, Severity};
use serde_json::{json, Value};

/// Render the diff report as a human-readable Markdown document.
///
/// Output includes program names, breaking/warning counts, and a table of
/// findings. When the report is clean the table is replaced with a single
/// success line.
pub fn render_markdown(report: &DiffReport) -> String {
    let mut out = String::new();
    out.push_str("# Spectra Diff Report\n\n");
    out.push_str(&format!("**Old program:** `{}`\n", report.old_program));
    out.push_str(&format!("**New program:** `{}`\n\n", report.new_program));
    out.push_str(&format!(
        "**Findings:** {} breaking, {} warning\n\n",
        report.breaking_count, report.warning_count
    ));

    if report.findings.is_empty() {
        out.push_str("No regressions detected. Upgrade is safe on the surfaces Spectra checks.\n");
        return out;
    }

    out.push_str("| Severity | Kind | Detail |\n");
    out.push_str("|---|---|---|\n");
    for f in &report.findings {
        let sev = match f.severity() {
            Severity::Breaking => "BREAKING",
            Severity::Warning => "warning",
        };
        out.push_str(&format!(
            "| {} | {} | {} |\n",
            sev,
            kind_label(f),
            detail(f)
        ));
    }

    if report.breaking_count > 0 {
        out.push_str(
            "\n> Spectra exits non-zero when any BREAKING finding is present. Review each row before deploy.\n",
        );
    }

    out
}

/// SARIF 2.1.0 output for GitHub Advanced Security / code scanning ingestion.
///
/// Each Spectra finding maps to a SARIF `result` with the rule ID equal to the
/// finding kind (e.g. `account_layout_changed_same_discriminator`). BREAKING
/// findings emit `level: "error"` so the GitHub Security tab surfaces them as
/// high severity; warnings emit `level: "warning"`.
///
/// `new_idl_path` is recorded as the artifact location so the GitHub UI links
/// the finding back to the candidate IDL file that triggered it. The logical
/// location carries the account/instruction name when one is present, giving
/// a stable navigation handle even though IDL JSON has no line numbers.
pub fn render_sarif(report: &DiffReport, new_idl_path: &str) -> String {
    let rules: Vec<Value> = rule_catalog()
        .iter()
        .map(|(id, name, help, default_level)| {
            json!({
                "id": id,
                "name": name,
                "shortDescription": { "text": *help },
                "fullDescription": { "text": *help },
                "defaultConfiguration": { "level": *default_level },
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
        .map(|f| finding_to_sarif_result(f, new_idl_path))
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
                "old_program": report.old_program,
                "new_program": report.new_program,
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
            "instruction_added",
            "InstructionAdded",
            "A new instruction was added in the candidate IDL.",
            "warning",
        ),
        (
            "instruction_removed",
            "InstructionRemoved",
            "An instruction was removed; existing clients invoking its discriminator will hit InstructionFallbackNotFound.",
            "error",
        ),
        (
            "instruction_args_changed",
            "InstructionArgsChanged",
            "Instruction argument layout changed; Borsh deserialisation will mis-decode existing client calls.",
            "error",
        ),
        (
            "account_added",
            "AccountAdded",
            "A new account type was added in the candidate IDL.",
            "warning",
        ),
        (
            "account_removed",
            "AccountRemoved",
            "An account type was removed; the old discriminator will no longer deserialise.",
            "error",
        ),
        (
            "account_field_added",
            "AccountFieldAdded",
            "An account field was added; verify storage realloc + rent handling.",
            "warning",
        ),
        (
            "account_field_removed",
            "AccountFieldRemoved",
            "An account field was removed; Borsh positional layout shifts.",
            "error",
        ),
        (
            "account_field_reordered",
            "AccountFieldReordered",
            "Account field order changed; Borsh reads will land in wrong positions.",
            "error",
        ),
        (
            "account_field_type_changed",
            "AccountFieldTypeChanged",
            "Account field type changed; encoding width/shape differs between versions.",
            "error",
        ),
        (
            "account_layout_changed_same_discriminator",
            "AccountLayoutChangedSameDiscriminator",
            "Silent-corruption case: account name (and discriminator) unchanged but layout changed. Existing on-chain accounts deserialise into wrong field positions with no runtime error.",
            "error",
        ),
        (
            "discriminator_collision",
            "DiscriminatorCollision",
            "Two IDL names produce the same truncated 8-byte SHA-256 discriminator; the dispatcher will mis-route.",
            "error",
        ),
    ]
}

fn finding_to_sarif_result(f: &Finding, new_idl_path: &str) -> Value {
    let level = match f.severity() {
        Severity::Breaking => "error",
        Severity::Warning => "warning",
    };
    let rule_id = kind_label(f);
    let message_text = format!("{}: {}", rule_id, detail(f));
    let logical_name = sarif_logical_name(f);

    let mut location = json!({
        "physicalLocation": {
            "artifactLocation": { "uri": new_idl_path }
        }
    });

    if let Some(name) = logical_name {
        location["logicalLocations"] = json!([{
            "name": name,
            "kind": sarif_logical_kind(f),
        }]);
    }

    json!({
        "ruleId": rule_id,
        "level": level,
        "message": { "text": message_text },
        "locations": [location],
    })
}

fn sarif_logical_name(f: &Finding) -> Option<String> {
    match f {
        Finding::InstructionAdded { name, .. }
        | Finding::InstructionRemoved { name, .. }
        | Finding::InstructionArgsChanged { name, .. } => Some(name.clone()),
        Finding::AccountAdded { name, .. } | Finding::AccountRemoved { name, .. } => {
            Some(name.clone())
        }
        Finding::AccountFieldAdded { account, field, .. }
        | Finding::AccountFieldRemoved { account, field, .. }
        | Finding::AccountFieldTypeChanged { account, field, .. } => {
            Some(format!("{}.{}", account, field))
        }
        Finding::AccountFieldReordered { account, .. }
        | Finding::AccountLayoutChangedSameDiscriminator { account, .. } => Some(account.clone()),
        Finding::DiscriminatorCollision { name_a, name_b, .. } => {
            Some(format!("{}|{}", name_a, name_b))
        }
    }
}

fn sarif_logical_kind(f: &Finding) -> &'static str {
    match f {
        Finding::InstructionAdded { .. }
        | Finding::InstructionRemoved { .. }
        | Finding::InstructionArgsChanged { .. } => "function",
        Finding::AccountAdded { .. }
        | Finding::AccountRemoved { .. }
        | Finding::AccountFieldReordered { .. }
        | Finding::AccountLayoutChangedSameDiscriminator { .. } => "type",
        Finding::AccountFieldAdded { .. }
        | Finding::AccountFieldRemoved { .. }
        | Finding::AccountFieldTypeChanged { .. } => "member",
        Finding::DiscriminatorCollision { .. } => "type",
    }
}

fn kind_label(f: &Finding) -> &'static str {
    match f {
        Finding::InstructionAdded { .. } => "instruction_added",
        Finding::InstructionRemoved { .. } => "instruction_removed",
        Finding::InstructionArgsChanged { .. } => "instruction_args_changed",
        Finding::AccountAdded { .. } => "account_added",
        Finding::AccountRemoved { .. } => "account_removed",
        Finding::AccountFieldReordered { .. } => "account_field_reordered",
        Finding::AccountFieldAdded { .. } => "account_field_added",
        Finding::AccountFieldRemoved { .. } => "account_field_removed",
        Finding::AccountFieldTypeChanged { .. } => "account_field_type_changed",
        Finding::AccountLayoutChangedSameDiscriminator { .. } => {
            "account_layout_changed_same_discriminator"
        }
        Finding::DiscriminatorCollision { .. } => "discriminator_collision",
    }
}

fn detail(f: &Finding) -> String {
    match f {
        Finding::InstructionAdded {
            name,
            discriminator,
        }
        | Finding::InstructionRemoved {
            name,
            discriminator,
        } => {
            format!("`{}` (disc {})", name, discriminator)
        }
        Finding::InstructionArgsChanged {
            name,
            old_args,
            new_args,
        } => format!(
            "`{}`: [{}] -> [{}]",
            name,
            old_args.join(", "),
            new_args.join(", ")
        ),
        Finding::AccountAdded {
            name,
            discriminator,
        }
        | Finding::AccountRemoved {
            name,
            discriminator,
        } => {
            format!("`{}` (disc {})", name, discriminator)
        }
        Finding::AccountFieldReordered {
            account,
            old_order,
            new_order,
        } => format!(
            "`{}`: [{}] -> [{}]",
            account,
            old_order.join(", "),
            new_order.join(", ")
        ),
        Finding::AccountFieldAdded { account, field, ty }
        | Finding::AccountFieldRemoved { account, field, ty } => {
            format!("`{}.{}: {}`", account, field, ty)
        }
        Finding::AccountFieldTypeChanged {
            account,
            field,
            old_ty,
            new_ty,
        } => {
            format!("`{}.{}`: {} -> {}", account, field, old_ty, new_ty)
        }
        Finding::AccountLayoutChangedSameDiscriminator {
            account,
            discriminator,
        } => format!(
            "`{}` layout changed but discriminator {} is unchanged (silent-corruption risk)",
            account, discriminator
        ),
        Finding::DiscriminatorCollision {
            kind_a,
            name_a,
            kind_b,
            name_b,
            discriminator,
        } => format!(
            "{} `{}` and {} `{}` share discriminator {}",
            kind_a, name_a, kind_b, name_b, discriminator
        ),
    }
}
