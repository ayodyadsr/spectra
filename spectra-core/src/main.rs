use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use spectra_core::{
    diff_programs,
    report::{render_markdown, render_sarif},
    ProgramModel,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "spectra",
    version,
    about = "Strictly-differential account-validation security-regression gate for Solana program upgrades"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Diff a baseline program source tree against a candidate (the upgrade
    /// PR) and report account-validation regressions.
    Check {
        /// Path to the baseline (last released / on-chain) program source tree
        #[arg(long)]
        baseline: PathBuf,
        /// Path to the candidate (upgrade under review) program source tree
        #[arg(long)]
        candidate: PathBuf,
        /// Optional path to write the report file
        #[arg(long)]
        report: Option<PathBuf>,
        /// Output format: json | markdown | sarif
        #[arg(long, default_value = "json")]
        format: String,
        /// Suppress stdout on clean runs; exit code still signals status.
        #[arg(long, default_value_t = false)]
        quiet: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Check {
            baseline,
            candidate,
            report,
            format,
            quiet,
        } => run_check(&baseline, &candidate, report.as_deref(), &format, quiet),
    };
    match result {
        Ok(code) => code,
        Err(err) => {
            eprintln!("Error: {:#}", err);
            ExitCode::from(2)
        }
    }
}

fn run_check(
    baseline: &Path,
    candidate: &Path,
    report_path: Option<&Path>,
    format: &str,
    quiet: bool,
) -> Result<ExitCode> {
    let base_model = ProgramModel::from_source_dir(baseline)?;
    let cand_model = ProgramModel::from_source_dir(candidate)?;
    let report = diff_programs(&base_model, &cand_model);

    let output = match format {
        "markdown" | "md" => render_markdown(&report),
        "sarif" => render_sarif(&report, &candidate.display().to_string()),
        "json" => serde_json::to_string_pretty(&report)?,
        other => bail!(
            "unknown --format `{}`; expected one of: json, markdown, sarif",
            other
        ),
    };

    if let Some(path) = report_path {
        std::fs::write(path, &output)?;
        eprintln!("Wrote report to {}", path.display());
    }

    let suppress_stdout = quiet && report.is_clean();
    if !suppress_stdout {
        println!("{}", output);
    }

    if !quiet {
        eprintln!(
            "Spectra: {} breaking, {} warning",
            report.breaking_count, report.warning_count
        );
    }

    Ok(if report.breaking_count > 0 {
        ExitCode::from(1)
    } else {
        ExitCode::from(0)
    })
}
