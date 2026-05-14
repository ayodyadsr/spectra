use crate::diff::{DiffReport, Finding, Severity};

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
