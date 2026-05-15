use anyhow::{bail, Result};
use clap::{Parser, Subcommand};
use spectra_core::{
    diff_idls,
    report::{render_markdown, render_sarif},
    Idl,
};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(
    name = "spectra",
    version,
    about = "Behavioural-regression diff for Solana program upgrades"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Diff two Anchor IDL JSON files and report regressions
    Check {
        /// Path to the baseline (v_n) IDL JSON
        #[arg(long)]
        old: PathBuf,
        /// Path to the new (v_{n+1}) IDL JSON
        #[arg(long)]
        new: PathBuf,
        /// Optional path to write the report file
        #[arg(long)]
        report: Option<PathBuf>,
        /// Output format: json | markdown | sarif
        #[arg(long, default_value = "json")]
        format: String,
        /// Suppress stdout report on clean runs; exit code still signals status.
        /// Useful for CI where only failing runs should produce noise.
        #[arg(long, default_value_t = false)]
        quiet: bool,
    },
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    let result = match cli.command {
        Command::Check {
            old,
            new,
            report,
            format,
            quiet,
        } => run_check(&old, &new, report.as_deref(), &format, quiet),
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
    old: &Path,
    new: &Path,
    report_path: Option<&Path>,
    format: &str,
    quiet: bool,
) -> Result<ExitCode> {
    let old_idl = Idl::from_path(old)?;
    let new_idl = Idl::from_path(new)?;
    let report = diff_idls(&old_idl, &new_idl);

    let output = match format {
        "markdown" | "md" => render_markdown(&report),
        "sarif" => render_sarif(&report, &new.display().to_string()),
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
