use spectra_core::{
    diff_idls,
    report::{render_markdown, render_sarif},
    Finding, Idl,
};
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
        report.breaking_count >= 4,
        "expected >=4 breaking findings (incl. silent-corruption), got {} (full report: {:#?})",
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

    let has_silent_corruption = report.findings.iter().any(|f| {
        matches!(f, Finding::AccountLayoutChangedSameDiscriminator { account, .. } if account == "Pool")
    });
    assert!(
        has_silent_corruption,
        "expected Pool silent-corruption finding (layout changed, discriminator unchanged)"
    );
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

/// Constructs a small IDL pair where two account names hash to colliding
/// 8-byte discriminators. We can't easily force a collision via natural names
/// at test time, so we exercise the detector via two-instructions-with-same-name
/// edge case is impossible (names are unique by construction). Instead, we
/// confirm the detector at least runs cleanly on the synthetic fixture and
/// produces zero false collision findings.
#[test]
fn no_false_collision_on_synthetic_fixture() {
    let old = Idl::from_path(&examples_dir().join("lending_v1.json")).expect("load v1");
    let new = Idl::from_path(&examples_dir().join("lending_v2.json")).expect("load v2");
    let report = diff_idls(&old, &new);
    let collisions: Vec<_> = report
        .findings
        .iter()
        .filter(|f| matches!(f, Finding::DiscriminatorCollision { .. }))
        .collect();
    assert!(
        collisions.is_empty(),
        "expected zero discriminator collisions on the synthetic fixture, got {}: {:#?}",
        collisions.len(),
        collisions
    );
}

#[test]
fn sarif_output_is_valid_for_synthetic_fixture() {
    let old = Idl::from_path(&examples_dir().join("lending_v1.json")).expect("load v1");
    let new = Idl::from_path(&examples_dir().join("lending_v2.json")).expect("load v2");
    let report = diff_idls(&old, &new);

    let sarif_text = render_sarif(&report, "examples/lending_v2.json");
    let parsed: serde_json::Value =
        serde_json::from_str(&sarif_text).expect("SARIF output must parse as JSON");

    assert_eq!(parsed["version"], "2.1.0", "SARIF version must be 2.1.0");
    assert!(parsed["$schema"].is_string(), "SARIF must declare $schema");

    let runs = parsed["runs"].as_array().expect("runs must be an array");
    assert_eq!(runs.len(), 1, "expected exactly one SARIF run");
    let run = &runs[0];

    assert_eq!(run["tool"]["driver"]["name"], "Spectra");
    assert!(
        run["tool"]["driver"]["rules"]
            .as_array()
            .map(|r| !r.is_empty())
            .unwrap_or(false),
        "rules catalog must be non-empty"
    );

    let results = run["results"].as_array().expect("results must be an array");
    assert_eq!(
        results.len(),
        report.findings.len(),
        "SARIF result count must match Finding count"
    );

    let has_silent_corruption_error = results.iter().any(|r| {
        r["ruleId"] == "account_layout_changed_same_discriminator" && r["level"] == "error"
    });
    assert!(
        has_silent_corruption_error,
        "silent-corruption finding must appear as level=error in SARIF"
    );

    for r in results {
        let loc = &r["locations"][0]["physicalLocation"]["artifactLocation"]["uri"];
        assert_eq!(
            loc, "examples/lending_v2.json",
            "every result must carry the new-IDL artifact location"
        );
    }
}

#[test]
fn sarif_clean_report_has_zero_results() {
    let old = Idl::from_path(&examples_dir().join("lending_v1.json")).expect("load v1");
    let new = old.clone();
    let report = diff_idls(&old, &new);
    let sarif_text = render_sarif(&report, "examples/lending_v1.json");
    let parsed: serde_json::Value = serde_json::from_str(&sarif_text).expect("parse SARIF");
    let results = parsed["runs"][0]["results"]
        .as_array()
        .expect("results array");
    assert!(
        results.is_empty(),
        "identical input must produce zero SARIF results, got {}",
        results.len()
    );
}

#[test]
fn markdown_renderer_still_produces_silent_corruption_row() {
    let old = Idl::from_path(&examples_dir().join("lending_v1.json")).expect("load v1");
    let new = Idl::from_path(&examples_dir().join("lending_v2.json")).expect("load v2");
    let report = diff_idls(&old, &new);
    let md = render_markdown(&report);
    assert!(
        md.contains("account_layout_changed_same_discriminator"),
        "markdown must call out silent-corruption finding by kind"
    );
}
