use spectra_core::{
    diff_programs,
    report::{render_markdown, render_sarif},
    Finding, ProgramModel,
};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

fn baseline() -> ProgramModel {
    ProgramModel::from_source_dir(&examples_dir().join("vault_baseline")).expect("parse baseline")
}

fn candidate() -> ProgramModel {
    ProgramModel::from_source_dir(&examples_dir().join("vault_candidate")).expect("parse candidate")
}

#[test]
fn synthetic_upgrade_detects_account_validation_regressions() {
    let report = diff_programs(&baseline(), &candidate());

    assert!(
        !report.is_clean(),
        "expected regressions, got clean: {:#?}",
        report
    );
    assert_eq!(
        report.breaking_count, 6,
        "expected exactly 6 breaking findings, got {}: {:#?}",
        report.breaking_count, report
    );
    assert_eq!(
        report.warning_count, 1,
        "expected exactly 1 warning finding, got {}: {:#?}",
        report.warning_count, report
    );

    let has = |pred: fn(&Finding) -> bool| report.findings.iter().any(pred);

    assert!(
        has(|f| matches!(f, Finding::SignerCheckRemoved { account, .. } if account == "authority")),
        "expected SignerCheckRemoved on authority"
    );
    assert!(
        has(|f| matches!(
            f,
            Finding::TypeCosplayProtectionRemoved { account, .. } if account == "destination"
        )),
        "expected TypeCosplayProtectionRemoved on destination"
    );
    assert!(
        has(|f| matches!(
            f,
            Finding::HasOneConstraintRemoved { account, .. } if account == "vault"
        )),
        "expected HasOneConstraintRemoved on vault"
    );
    assert!(
        has(|f| matches!(
            f,
            Finding::CustomConstraintRemoved { account, .. } if account == "destination"
        )),
        "expected CustomConstraintRemoved on destination"
    );
    assert!(
        has(|f| matches!(
            f,
            Finding::PdaDerivationRemoved { account, .. } if account == "config"
        )),
        "expected PdaDerivationRemoved on config"
    );
    assert!(
        has(|f| matches!(
            f,
            Finding::CpiTargetUnpinned { account, .. } if account == "token_program"
        )),
        "expected CpiTargetUnpinned on token_program"
    );
    assert!(
        has(|f| matches!(
            f,
            Finding::UnvalidatedAccountIntroduced { account, .. } if account == "anyone"
        )),
        "expected UnvalidatedAccountIntroduced on EmergencyDrain::anyone"
    );
}

#[test]
fn identical_program_produces_clean_report() {
    let b = baseline();
    let report = diff_programs(&b, &b.clone());
    assert!(report.is_clean(), "expected clean, got {:#?}", report);
    assert_eq!(report.findings.len(), 0);
}

#[test]
fn unchanged_context_in_changed_program_produces_no_false_positive() {
    // `Initialize` is byte-identical baseline↔candidate. The strictly-
    // differential property: no finding may name the `Initialize` context.
    let report = diff_programs(&baseline(), &candidate());
    let initialize_findings: Vec<_> = report
        .findings
        .iter()
        .filter(|f| {
            let s = format!("{:?}", f);
            s.contains("Initialize")
        })
        .collect();
    assert!(
        initialize_findings.is_empty(),
        "unchanged Initialize context must yield zero findings, got {:#?}",
        initialize_findings
    );
}

#[test]
fn sarif_output_is_valid() {
    let report = diff_programs(&baseline(), &candidate());
    let sarif = render_sarif(&report, "examples/vault_candidate");
    let parsed: serde_json::Value = serde_json::from_str(&sarif).expect("SARIF parses as JSON");

    assert_eq!(parsed["version"], "2.1.0");
    assert!(parsed["$schema"].is_string());
    let runs = parsed["runs"].as_array().expect("runs array");
    assert_eq!(runs.len(), 1);
    assert_eq!(runs[0]["tool"]["driver"]["name"], "Spectra");
    assert!(runs[0]["tool"]["driver"]["rules"]
        .as_array()
        .map(|r| !r.is_empty())
        .unwrap_or(false));

    let results = runs[0]["results"].as_array().expect("results array");
    assert_eq!(results.len(), report.findings.len());
    assert!(results
        .iter()
        .any(|r| r["ruleId"] == "signer_check_removed" && r["level"] == "error"));
    for r in results {
        let uri = &r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"];
        assert_eq!(uri, "examples/vault_candidate");
    }
}

#[test]
fn sarif_clean_report_has_zero_results() {
    let b = baseline();
    let report = diff_programs(&b, &b.clone());
    let sarif = render_sarif(&report, "examples/vault_baseline");
    let parsed: serde_json::Value = serde_json::from_str(&sarif).expect("parse SARIF");
    let results = parsed["runs"][0]["results"].as_array().expect("results");
    assert!(results.is_empty(), "got {} results", results.len());
}

#[test]
fn markdown_renderer_calls_out_signer_regression() {
    let report = diff_programs(&baseline(), &candidate());
    let md = render_markdown(&report);
    assert!(
        md.contains("signer_check_removed"),
        "markdown must name the signer regression by rule id"
    );
    assert!(md.contains("BREAKING"));
}
