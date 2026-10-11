use std::{
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use anyhow::Result;
use clap::{ArgAction, Args, Parser, Subcommand, ValueEnum};

use crate::{
    config::{self, process_env},
    patterns,
    scaffold::{self, ActionStatus, ScaffoldAction},
    scan::{self, ScanMode, ScanResult},
    watchlist,
};

#[derive(Debug, Parser)]
#[command(
    name = "doxguard",
    version,
    about = "Keep personal identity data out of public repositories",
    disable_help_subcommand = true
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scan repository content.
    Scan(ScanArgs),
    /// Create a config, pre-commit hook, and structural-only CI workflow.
    Init,
    /// Install a core.hooksPath pre-commit hook (Husky is detected, not overwritten).
    InstallHooks,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Text,
    Json,
}

#[derive(Debug, Args)]
struct ScanArgs {
    /// Scan files staged for commit.
    #[arg(long, action = ArgAction::SetTrue)]
    staged: bool,
    /// Scan files changed compared with HEAD.
    #[arg(long, alias = "files-from-diff", action = ArgAction::SetTrue)]
    diff: bool,
    /// Scan every git-tracked file.
    #[arg(long, action = ArgAction::SetTrue)]
    all_tracked: bool,
    /// Scan the file list produced by npm pack.
    #[arg(long, action = ArgAction::SetTrue)]
    packaged: bool,
    /// Scan a literal file or directory (repeatable), without requiring git.
    #[arg(long = "path", action = ArgAction::Append)]
    paths: Vec<PathBuf>,
    /// Read literal paths from a UTF-8 file, one per line; combine with --path.
    #[arg(long)]
    path_from: Option<PathBuf>,
    /// Include matched values in text/JSON. Omit this for logs and AI sessions.
    #[arg(long)]
    show_matches: bool,
    /// Exit 1 when matches are found.
    #[arg(long)]
    block: bool,
    /// Report matches but exit 0.
    #[arg(long)]
    dry_run: bool,
    /// Select human-readable or JSON output.
    #[arg(long, value_enum, default_value_t = OutputFormat::Text)]
    format: OutputFormat,
    /// Config path. DOXGUARD_CONFIG is used when this option is omitted.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Stricter gate: disallow bare `doxguard: allow`, and fail on coverage skips with `--block`.
    #[arg(long, action = ArgAction::SetTrue)]
    strict: bool,
}

fn mode(args: &ScanArgs) -> ScanMode {
    if args.staged {
        ScanMode::Staged
    } else if args.diff {
        ScanMode::Diff
    } else if args.all_tracked {
        ScanMode::AllTracked
    } else if !args.paths.is_empty() || args.path_from.is_some() {
        ScanMode::Paths
    } else {
        ScanMode::Packaged
    }
}

fn text_report(result: &ScanResult) -> String {
    if result.hits.is_empty() {
        if result.coverage_skips > 0 {
            return format!(
                "INCOMPLETE: doxguard has {} coverage skip(s) (scanned {} files)\n",
                result.coverage_skips, result.scanned
            );
        }
        return format!(
            "OK: doxguard passed (scanned {} files; {} needles + {} structural patterns)\n",
            result.scanned, result.watchlist_needles, result.structural_patterns
        );
    }
    let mut output = format!(
        "BLOCKED: doxguard detected {} match(es).\n\n",
        result.hits.len()
    );
    for hit in &result.hits {
        output.push_str(&format!(
            "{}:{}\n  variable: {}\n  matched: {:?}\n  source:  {}\n  suggest: {}\n\n",
            hit.file,
            hit.line_number,
            hit.variable.as_deref().unwrap_or("(unavailable)"),
            hit.matched,
            hit.source,
            hit.suggestion
        ));
    }
    output
}

fn report_result(result: &ScanResult, show_matches: bool) -> ScanResult {
    let mut report = result.clone();
    if show_matches {
        return report;
    }
    let mut values: Vec<_> = result.hits.iter().map(|hit| hit.matched.as_str()).collect();
    values.sort_by_key(|value| std::cmp::Reverse(value.len()));
    values.dedup();
    let redact = |text: &str| {
        values
            .iter()
            .filter(|value| !value.is_empty())
            .fold(text.to_owned(), |text, value| {
                text.replace(value, "[redacted]")
            })
    };
    for hit in &mut report.hits {
        hit.matched = "[redacted]".to_owned();
        hit.file = redact(&hit.file);
        hit.variable = hit.variable.as_deref().map(redact);
        hit.source = redact(&hit.source);
        hit.suggestion = redact(&hit.suggestion);
    }
    report.warnings = report
        .warnings
        .iter()
        .map(|warning| redact(warning))
        .collect();
    report
}

fn print_actions(actions: &[ScaffoldAction]) {
    for action in actions {
        let status = match action.status {
            ActionStatus::Created => "CREATED",
            ActionStatus::Skipped => "SKIPPED",
            ActionStatus::Configured => "CONFIGURED",
        };
        let detail = action
            .detail
            .as_deref()
            .map(|detail| format!(" ({detail})"))
            .unwrap_or_default();
        println!("{status}: {}{detail}", action.path);
    }
}

fn run_scan(args: &ScanArgs) -> Result<u8> {
    let mode_count = [
        args.staged,
        args.diff,
        args.all_tracked,
        args.packaged,
        !args.paths.is_empty() || args.path_from.is_some(),
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count();
    if mode_count != 1 {
        anyhow::bail!("scan requires exactly one mode");
    }
    let cwd = std::env::current_dir()?;
    let mut loaded = config::load(&cwd, args.config.as_deref())?;
    if args.strict {
        loaded.config.apply_strict();
    }
    let watchlists = watchlist::load(&loaded.config, &cwd, &process_env())?;
    let patterns = patterns::build(&loaded.config)?;
    let mut warnings = loaded.warnings;
    warnings.extend(watchlists.warnings);
    let paths = if mode(args) == ScanMode::Paths {
        let mut roots = args.paths.clone();
        if let Some(path_from) = &args.path_from {
            let text = std::fs::read_to_string(path_from)?;
            roots.extend(
                text.trim_start_matches('\u{feff}')
                    .lines()
                    .filter(|line| !line.trim().is_empty())
                    .map(PathBuf::from),
            );
        }
        scan::files_from_paths(&roots, &cwd, &loaded.config)?
    } else {
        scan::files_for_mode(mode(args), &cwd)?
    };
    let result = scan::scan_paths(
        mode(args),
        paths,
        &cwd,
        &loaded.config,
        &watchlists.matcher,
        &patterns,
        warnings,
    )?;
    let report = report_result(&result, args.show_matches);
    for warning in &report.warnings {
        eprintln!("{warning}");
    }
    match args.format {
        OutputFormat::Json => println!("{}", serde_json::to_string_pretty(&report)?),
        OutputFormat::Text => {
            let report = text_report(&report);
            if result.hits.is_empty() {
                print!("{report}");
                io::stdout().flush()?;
            } else {
                eprint!("{report}");
                io::stderr().flush()?;
            }
        }
    }
    let blocked = args.block
        && !args.dry_run
        && (!result.hits.is_empty() || (loaded.config.fail_on_skip && result.coverage_skips > 0));
    Ok(u8::from(blocked))
}

fn try_run(cli: Cli) -> Result<u8> {
    match cli.command {
        Command::Scan(args) => run_scan(&args),
        Command::Init => {
            print_actions(&scaffold::initialize(&std::env::current_dir()?)?);
            Ok(0)
        }
        Command::InstallHooks => {
            print_actions(&scaffold::install_hooks(&std::env::current_dir()?)?);
            Ok(0)
        }
    }
}

pub fn run() -> ExitCode {
    let invoked_as_hook = std::env::args_os().count() == 1
        && std::env::current_exe()
            .ok()
            .and_then(|path| path.file_stem().map(|stem| stem == "pre-commit"))
            .unwrap_or(false);
    let parsed = if invoked_as_hook {
        Cli::try_parse_from(["doxguard", "scan", "--staged", "--block"])
    } else {
        Cli::try_parse()
    };
    let cli = match parsed {
        Ok(cli) => cli,
        Err(error) => {
            let code = error.exit_code();
            let _ = error.print();
            return ExitCode::from(if code == 0 { 0 } else { 2 });
        }
    };
    match try_run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("ERROR: {error:#}");
            ExitCode::from(2)
        }
    }
}
