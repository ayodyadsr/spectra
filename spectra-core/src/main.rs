use anyhow::Result;
use clap::{Parser, Subcommand};
use spectra_core::{diff_idls, report::render_markdown, Idl};
use std::path::{Path, PathBuf};

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
        /// Output format: json | markdown
        #[arg(long, default_value = "json")]
        format: String,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::Check {
            old,
            new,
            report,
            format,
        } => run_check(&old, &new, report.as_deref(), &format),
    }
}

fn run_check(old: &Path, new: &Path, report_path: Option<&Path>, format: &str) -> Result<()> {
    let old_idl = Idl::from_path(old)?;
    let new_idl = Idl::from_path(new)?;
    let report = diff_idls(&old_idl, &new_idl);

    let output = match format {
        "markdown" | "md" => render_markdown(&report),
        _ => serde_json::to_string_pretty(&report)?,
    };

    if let Some(path) = report_path {
        std::fs::write(path, &output)?;
        eprintln!("Wrote report to {}", path.display());
    }

    println!("{}", output);

    eprintln!(
        "Spectra: {} breaking, {} warning",
        report.breaking_count, report.warning_count
    );

    if report.breaking_count > 0 {
        std::process::exit(1);
    }
    Ok(())
}
