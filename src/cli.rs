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
    /// Include the detected value in output. This may expose private data in logs.
    #[arg(long, action = ArgAction::SetTrue)]
    show_matched: bool,
}

fn mode(args: &ScanArgs) -> ScanMode {
    if args.staged {
        ScanMode::Staged
    } else if args.diff {
        ScanMode::Diff
    } else if args.all_tracked {
        ScanMode::AllTracked
    } else {
        ScanMode::Packaged
    }
}

const REDACTED_MATCH: &str = "[REDACTED]";

fn is_bidi_control(character: char) -> bool {
    matches!(
        character,
        '\u{061c}'
            | '\u{200e}'
            | '\u{200f}'
            | '\u{202a}'..='\u{202e}'
            | '\u{2066}'..='\u{2069}'
    )
}

fn sanitize_terminal(value: &str) -> String {
    let mut sanitized = String::with_capacity(value.len());
    for character in value.chars() {
        if character.is_control() || is_bidi_control(character) {
            sanitized.extend(character.escape_default());
        } else {
            sanitized.push(character);
        }
    }
    sanitized
}

fn json_report(result: &ScanResult, show_matched: bool) -> Result<String> {
    let mut value = serde_json::to_value(result)?;
    if let Some(hits) = value
        .get_mut("hits")
        .and_then(serde_json::Value::as_array_mut)
    {
        for hit in hits {
            let Some(hit) = hit.as_object_mut() else {
                continue;
            };
            for field in ["file", "source", "suggestion"] {
                if let Some(serde_json::Value::String(value)) = hit.get_mut(field) {
                    *value = sanitize_terminal(value);
                }
            }
            if let Some(serde_json::Value::String(value)) = hit.get_mut("matched") {
                *value = if show_matched {
                    sanitize_terminal(value)
                } else {
                    REDACTED_MATCH.to_owned()
                };
            }
        }
    }
    if let Some(warnings) = value
        .get_mut("warnings")
        .and_then(serde_json::Value::as_array_mut)
    {
        for warning in warnings {
            if let serde_json::Value::String(value) = warning {
                *value = sanitize_terminal(value);
            }
        }
    }
    Ok(serde_json::to_string_pretty(&value)?)
}

fn text_report(result: &ScanResult, show_matched: bool) -> String {
    if result.hits.is_empty() {
        if result.coverage_skips > 0 {
            return format!(
                "INCOMPLETE: doxguard skipped {} file(s) that could not be scanned (scanned {} of {} files).\n",
                result.coverage_skips, result.scanned, result.total_files
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
        let matched = if show_matched {
            sanitize_terminal(&hit.matched)
        } else {
            REDACTED_MATCH.to_owned()
        };
        output.push_str(&format!(
            "{}:{}\n  matched: {:?}\n  source:  {}\n  suggest: {}\n\n",
            sanitize_terminal(&hit.file),
            hit.line_number,
            matched,
            sanitize_terminal(&hit.source),
            sanitize_terminal(&hit.suggestion)
        ));
    }
    output
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
        println!(
            "{status}: {}{}",
            sanitize_terminal(&action.path),
            sanitize_terminal(&detail)
        );
    }
}

fn run_scan(args: &ScanArgs) -> Result<u8> {
    let mode_count = [args.staged, args.diff, args.all_tracked, args.packaged]
        .into_iter()
        .filter(|selected| *selected)
        .count();
    if mode_count != 1 {
        anyhow::bail!("scan requires exactly one mode");
    }
    let cwd = std::env::current_dir()?;
    let scan_mode = mode(args);
    let scan_root = if scan_mode == ScanMode::Packaged {
        cwd.clone()
    } else {
        scan::repository_root(&cwd)?
    };
    let explicit_config = args.config.is_some()
        || std::env::var_os("DOXGUARD_CONFIG").is_some_and(|value| !value.is_empty());
    let mut loaded = config::load_from(&cwd, &scan_root, args.config.as_deref())?;
    if args.strict {
        loaded.config.apply_strict();
    }
    let watchlist_root = if explicit_config { &cwd } else { &scan_root };
    let watchlists = watchlist::load(&loaded.config, watchlist_root, &process_env())?;
    let patterns = patterns::build(&loaded.config)?;
    let mut warnings = loaded.warnings;
    warnings.extend(watchlists.warnings);
    let paths = scan::files_for_mode(scan_mode, &scan_root)?;
    let result = scan::scan_paths(
        scan_mode,
        paths,
        &scan_root,
        &loaded.config,
        &watchlists.matcher,
        &patterns,
        warnings,
    )?;
    for warning in &result.warnings {
        eprintln!("{}", sanitize_terminal(warning));
    }
    match args.format {
        OutputFormat::Json => println!("{}", json_report(&result, args.show_matched)?),
        OutputFormat::Text => {
            let report = text_report(&result, args.show_matched);
            if result.hits.is_empty() && result.coverage_skips == 0 {
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
        Cli::try_parse_from(["doxguard", "scan", "--staged", "--block", "--strict"])
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
            eprintln!("ERROR: {}", sanitize_terminal(&format!("{error:#}")));
            ExitCode::from(2)
        }
    }
}
