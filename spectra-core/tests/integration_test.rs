use spectra_core::{diff_idls, Finding, Idl};
use std::path::PathBuf;

fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../examples")
}

#[test]
fn synthetic_regression_demo_detects_breaking_changes() {
    let old = Idl::from_path(&examples_dir().join("lending_v1.json")).expect("load v1");
    let new = Idl::from_path(&examples_dir().join("lending_v2.json")).expect("load v2");
    let report = diff_idls(&old, &new);

    assert!(
        !report.is_clean(),
        "expected regressions, got clean report: {:#?}",
        report
    );
    assert!(
        report.breaking_count >= 3,
        "expected >=3 breaking findings, got {} (full report: {:#?})",
        report.breaking_count,
        report
    );

    let has_withdraw_removed = report
        .findings
        .iter()
        .any(|f| matches!(f, Finding::InstructionRemoved { name, .. } if name == "withdraw"));
    assert!(
        has_withdraw_removed,
        "expected withdraw instruction removed"
    );

    let has_deposit_args_changed = report
        .findings
        .iter()
        .any(|f| matches!(f, Finding::InstructionArgsChanged { name, .. } if name == "deposit"));
    assert!(has_deposit_args_changed, "expected deposit args changed");

    let has_pool_reorder = report
        .findings
        .iter()
        .any(|f| matches!(f, Finding::AccountFieldReordered { account, .. } if account == "Pool"));
    assert!(has_pool_reorder, "expected Pool field reorder");
}

#[test]
fn identical_idls_produce_clean_report() {
    let old = Idl::from_path(&examples_dir().join("lending_v1.json")).expect("load v1");
    let new = old.clone();
    let report = diff_idls(&old, &new);
    assert!(
        report.is_clean(),
        "expected clean report, got {:#?}",
        report
    );
    assert_eq!(report.findings.len(), 0);
}
