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
    watch, watchlist,
};

#[derive(Debug, Parser)]
#[command(
    name = "doxguard",
    version,
    about = "Keep personal identity data out of public repositories",
    disable_help_subcommand = true
)]
struct Cli {
    /// Include file-system locations (config, watchlist, scaffold paths) in error and
    /// warning messages. Hidden by default so they stay out of AI transcripts and CI
    /// logs. Same as DOXGUARD_SHOW_PATHS=1. Findings always show `file:line`.
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    show_paths: bool,
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Scan repository content.
    ///
    /// Use this to check files before they are committed or published. Read-only: it
    /// never changes files. Pass exactly one mode (--staged, --diff, --all-tracked or
    /// --packaged). Add --block to exit 1 when matches are found. Exit codes: 0 =
    /// clean (or matches reported without --block), 1 = blocked, 2 = usage or
    /// configuration error (the ERROR line says what to change).
    Scan(ScanArgs),
    /// Create a config, pre-commit hook, and structural-only CI workflow.
    ///
    /// Use this once per repository. Existing files are left unchanged (reported as
    /// SKIPPED), and no AI instruction file (CLAUDE.md, AGENTS.md) is created or edited.
    Init,
    /// Install a core.hooksPath pre-commit hook (Husky is detected, not overwritten).
    ///
    /// Use this after cloning, or when commits are not being checked. It is safe to
    /// run again; an existing hooksPath is left unchanged (reported as SKIPPED).
    InstallHooks,
    /// Manage the private watchlist (append only).
    ///
    /// Use `watch add` to add terms. Editing or removing terms, and allow-lists, are
    /// done by hand in the watchlist file or the config.
    Watch {
        #[command(subcommand)]
        command: WatchCommand,
    },
}

#[derive(Debug, Subcommand)]
enum WatchCommand {
    /// Append terms to a `lines` watchlist outside the repository. Values are never printed; paths only in errors with --show-paths.
    ///
    /// Use this when a new private term (a name, host, customer, ...) must be caught
    /// by `doxguard scan`. It appends only: existing lines are never removed or
    /// rewritten, and allow-lists are edited by hand. Output shows counts only
    /// (ADDED, EXISTS, REJECTED, TOTAL), never the term values or the file path.
    ///
    /// If it fails: exit code 2 with REJECTED: N means N terms were refused (the reason
    /// kinds are listed, e.g. too-short, comment-prefix) while the valid terms were
    /// still appended; exit code 2 with an ERROR line means nothing was written and the
    /// message says what to change (for example, set an unset environment variable or
    /// point the source outside the repository).
    Add(WatchAddArgs),
}

#[derive(Debug, Args)]
struct WatchAddArgs {
    /// Terms to watch. They stay in shell history; prefer --stdin for sensitive values.
    #[arg(value_name = "TERM", required_unless_present = "stdin")]
    terms: Vec<String>,
    /// Also read terms from standard input, one per line.
    #[arg(long, action = ArgAction::SetTrue)]
    stdin: bool,
    /// Watchlist source to append to (1-based, as in `source #N` warnings). Default: first `lines` source.
    #[arg(long, value_name = "N")]
    source: Option<usize>,
    /// Config path. DOXGUARD_CONFIG is used when this option is omitted.
    #[arg(long)]
    config: Option<PathBuf>,
    /// Judge the terms but do not write anything.
    #[arg(long)]
    dry_run: bool,
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
    /// Scan blobs reachable from all Git refs, including deleted files (full clone required).
    #[arg(long, action = ArgAction::SetTrue)]
    history: bool,
    /// Scan the file list produced by npm pack.
    #[arg(long, action = ArgAction::SetTrue)]
    packaged: bool,
    /// Scan repository-relative paths listed in a UTF-8 file, including gitignored submissions.
    #[arg(long, value_name = "PATH")]
    files_from_list: Option<PathBuf>,
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
    } else if args.history {
        ScanMode::History
    } else if args.files_from_list.is_some() {
        ScanMode::FilesFromList
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
    let statistics = &result.watchlist_statistics;
    let summary = format!(
        "Watchlist candidates: {}; loaded: {}; excluded: too_short={}, short_kana={}, allow_listed={}, duplicate={}\nBaseline applied: {}; suppressed hits: {}\n",
        statistics.candidates,
        statistics.loaded,
        statistics.too_short,
        statistics.short_kana,
        statistics.allow_listed,
        statistics.duplicate,
        result.baseline_applied,
        result.baseline_suppressed_hits
    );
    if result.hits.is_empty() {
        if result.coverage_skips > 0 {
            return format!(
                "INCOMPLETE: doxguard skipped {} file(s) that could not be scanned (scanned {} of {} files).\n{summary}",
                result.coverage_skips, result.scanned, result.total_files
            );
        }
        return format!(
            "OK: doxguard passed (scanned {} files; {} needles + {} structural patterns)\n{summary}",
            result.scanned, result.watchlist_needles, result.structural_patterns
        );
    }
    let mut output = format!(
        "BLOCKED: doxguard detected {} match(es).\n{summary}\n",
        result.hits.len()
    );
    for hit in &result.hits {
        if let Some(oid) = &hit.blob_oid {
            output.push_str(&format!("history blob: {oid}\n"));
        }
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
    let mode_count = [
        args.staged,
        args.diff,
        args.all_tracked,
        args.history,
        args.packaged,
        args.files_from_list.is_some(),
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count();
    if mode_count != 1 {
        anyhow::bail!(
            "scan requires exactly one mode. Pass exactly one of --staged, --diff, --all-tracked, --history, --packaged, --files-from-list"
        );
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
    let result = if scan_mode == ScanMode::History {
        scan::scan_history(
            &scan_root,
            &loaded.config,
            &watchlists.matcher,
            &patterns,
            warnings,
        )?
    } else {
        let paths = if let Some(list) = &args.files_from_list {
            scan::files_from_list(list, &cwd, &scan_root)?
        } else {
            scan::files_for_mode(scan_mode, &scan_root)?
        };
        scan::scan_paths(
            scan_mode,
            paths,
            &scan_root,
            &loaded.config,
            &watchlists.matcher,
            &patterns,
            warnings,
        )?
    };
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

fn run_watch_add(args: &WatchAddArgs) -> Result<u8> {
    use std::io::Read;

    const STDIN_LIMIT: u64 = 1024 * 1024;
    let mut terms = args.terms.clone();
    if args.stdin {
        let mut bytes = Vec::new();
        io::stdin().take(STDIN_LIMIT + 1).read_to_end(&mut bytes)?;
        if bytes.len() as u64 > STDIN_LIMIT {
            anyhow::bail!(
                "standard input is larger than {STDIN_LIMIT} bytes. Send fewer terms per run"
            );
        }
        let text = String::from_utf8(bytes).map_err(|_| {
            anyhow::anyhow!("standard input is not valid UTF-8. Send the terms as UTF-8 text")
        })?;
        terms.extend(text.lines().map(str::to_owned));
    }
    let cwd = std::env::current_dir()?;
    let repo_root = scan::repository_root(&cwd).unwrap_or_else(|_| cwd.clone());
    let explicit_config = args.config.is_some()
        || std::env::var_os("DOXGUARD_CONFIG").is_some_and(|value| !value.is_empty());
    // Config errors can echo the config location (when --show-paths is on); this command
    // keeps its generic message unless the caller asked for locations.
    let loaded = config::load_from(&cwd, &repo_root, args.config.as_deref()).map_err(|error| {
        if config::show_paths() {
            error
        } else {
            anyhow::anyhow!("failed to load the doxguard config (run `doxguard scan` for details)")
        }
    })?;
    if !loaded.found {
        anyhow::bail!(
            "no doxguard config found; nothing to append to. Run from the repository root that holds doxguard.config.json, or pass --config / set DOXGUARD_CONFIG"
        );
    }
    let base = if explicit_config { &cwd } else { &repo_root };
    let report = watch::add(
        &loaded.config,
        base,
        &repo_root,
        &process_env(),
        &terms,
        args.source,
        args.dry_run,
    )?;
    println!("ADDED: {}", report.added);
    println!("EXISTS: {}", report.exists);
    if report.rejected.is_empty() {
        println!("REJECTED: 0");
    } else {
        println!(
            "REJECTED: {} ({})",
            report.rejected.len(),
            sanitize_terminal(&report.rejected_kinds().join(", "))
        );
    }
    println!(
        "TOTAL: {} active term(s) in the target source",
        report.total
    );
    if args.dry_run {
        println!("DRY-RUN: nothing was written");
    }
    Ok(if report.succeeded() { 0 } else { 2 })
}

fn try_run(cli: Cli) -> Result<u8> {
    match cli.command {
        Command::Watch {
            command: WatchCommand::Add(args),
        } => run_watch_add(&args),
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
    config::set_show_paths(cli.show_paths || config::show_paths_from_env());
    match try_run(cli) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("ERROR: {}", sanitize_terminal(&format!("{error:#}")));
            ExitCode::from(2)
        }
    }
}
