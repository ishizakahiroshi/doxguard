use std::{
    borrow::Cow,
    io::{self, Write},
    path::PathBuf,
    process::ExitCode,
};

use aho_corasick::{AhoCorasick, MatchKind};
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
    /// never changes files. Pass exactly one mode (--staged, --diff, --all-tracked,
    /// --history, --packaged, --files-from-list, or --path/--path-from).
    /// Add --block to exit 1 when matches are found. Exit codes: 0 =
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
    /// Scan a literal file or directory (repeatable), without requiring git.
    #[arg(long = "path", action = ArgAction::Append)]
    paths: Vec<PathBuf>,
    /// Read literal paths from a UTF-8 file, one per line; combine with --path.
    #[arg(long)]
    path_from: Option<PathBuf>,
    /// Include matched values in text/JSON. Omit this for logs and AI sessions.
    #[arg(long)]
    show_matches: bool,
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
    } else if !args.paths.is_empty() || args.path_from.is_some() {
        ScanMode::Paths
    } else if args.history {
        ScanMode::History
    } else if args.files_from_list.is_some() {
        ScanMode::FilesFromList
    } else {
        ScanMode::Packaged
    }
}

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

fn json_report(result: &ScanResult) -> Result<String> {
    let mut value = serde_json::to_value(result)?;
    if let Some(hits) = value
        .get_mut("hits")
        .and_then(serde_json::Value::as_array_mut)
    {
        for hit in hits {
            let Some(hit) = hit.as_object_mut() else {
                continue;
            };
            for field in ["file", "variable", "matched", "source", "suggestion"] {
                if let Some(serde_json::Value::String(value)) = hit.get_mut(field) {
                    *value = sanitize_terminal(value);
                }
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

fn text_report(result: &ScanResult) -> String {
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
        output.push_str(&format!(
            "{}:{}\n  variable: {}\n  matched: {:?}\n  source:  {}\n  suggest: {}\n\n",
            sanitize_terminal(&hit.file),
            hit.line_number,
            sanitize_terminal(hit.variable.as_deref().unwrap_or("(unavailable)")),
            sanitize_terminal(&hit.matched),
            sanitize_terminal(&hit.source),
            sanitize_terminal(&hit.suggestion)
        ));
    }
    output
}

/// Hides every reported value in metadata text. All distinct values share one
/// automaton, so each text is scanned once however many values there are.
struct Redactor {
    matcher: Option<AhoCorasick>,
    max_text_len: usize,
}

impl Redactor {
    /// `max_text_len` bounds every text later passed to `redact`. A longer
    /// value cannot occur in any of them, so it stays out of the automaton,
    /// whose size grows with the value bytes (a DFA has no size limit).
    fn new<'a>(values: impl IntoIterator<Item = &'a str>, max_text_len: usize) -> Self {
        // An empty pattern would match at every offset. Matching ignores ASCII
        // case and reads each run of `/` and `\` as one `/`, so values
        // differing only in ASCII case or in path separators are one pattern.
        // A value is folded first: only its folded form occurs in the folded
        // texts, and that form is no longer than the value.
        // A path-like value can be detected with a leading separator or a
        // trailing `.`, `-`, `_` or `.git` (a sentence end, a hyphenated word,
        // a clone URL) that its name in another text lacks, so shorter forms
        // are patterns too: the form without leading separators and trailing
        // dots, and the form without any of those suffixes. Both are kept, so
        // a name that keeps part of the suffix still matches the longer one.
        // A form is added only while it still names a path (a `/` before its
        // end): `/opt` would hide every `opt`. A form whose last name was all
        // suffix (`github/private/-` to `github/private/`) would hide every
        // name in that folder, so it is not added either.
        fn without_suffixes(name: &str) -> &str {
            let mut name = name;
            loop {
                if let Some(rest) = name.strip_suffix(['.', '-', '_']) {
                    name = rest;
                } else if let Some(rest) = name.strip_suffix(".git") {
                    name = rest;
                } else {
                    return name;
                }
            }
        }
        let names_path = |form: &str| form.trim_end_matches('/').contains('/');
        let mut values: Vec<_> = values
            .into_iter()
            .flat_map(|value| {
                let folded = unify_separators(&value.to_ascii_lowercase()).0.into_owned();
                let dotless = folded.trim_start_matches('/').trim_end_matches('.');
                let bare = without_suffixes(dotless);
                let lost_last_name = bare.ends_with('/') && !dotless.ends_with('/');
                let bare = (bare != dotless && names_path(bare) && !lost_last_name)
                    .then(|| bare.to_owned());
                let dotless = names_path(dotless).then(|| dotless.to_owned());
                [Some(folded), dotless, bare]
            })
            .flatten()
            .filter(|value| !value.is_empty() && value.len() <= max_text_len)
            .collect();
        values.sort_unstable();
        values.dedup();
        let matcher = (!values.is_empty()).then(|| {
            AhoCorasick::builder()
                .ascii_case_insensitive(true)
                .match_kind(MatchKind::Standard)
                .build(values)
                // Only the noncontiguous NFA can fail to build (a DFA or
                // contiguous NFA that does not fit falls back to it), once its
                // tables pass 2^31 entries: about a GiB of distinct values.
                // Each value is at most the longest redacted text, so a single
                // long match no longer gets near that.
                .expect("redaction automaton exceeds aho-corasick limits")
        });
        Self {
            matcher,
            max_text_len,
        }
    }

    fn redact(&self, text: &str) -> String {
        debug_assert!(
            text.len() <= self.max_text_len,
            "text is longer than the bound given to Redactor::new"
        );
        let Some(matcher) = &self.matcher else {
            return text.to_owned();
        };
        // ASCII-only case folding matches watchlist semantics, and non-ASCII
        // bytes compare exactly. Matching runs on a copy where each run of `/`
        // and `\` is one `/`; `starts` maps its offsets back to the original
        // text, so a match that touches a run covers the whole run. Redact all
        // overlapping spans once; replacement markers are never fed back into
        // the matcher.
        let (unified, starts) = unify_separators(text);
        let mut spans: Vec<_> = matcher
            .find_overlapping_iter(unified.as_ref())
            .map(|found| match &starts {
                Some(starts) => (starts[found.start()], starts[found.end()]),
                None => (found.start(), found.end()),
            })
            .collect();
        spans.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in spans {
            if let Some(last) = merged.last_mut().filter(|last| start <= last.1) {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        let mut redacted = String::new();
        let mut cursor = 0;
        for (start, end) in merged {
            redacted.push_str(&text[cursor..start]);
            redacted.push_str("[redacted]");
            cursor = end;
        }
        redacted.push_str(&text[cursor..]);
        redacted
    }
}

/// Reads each run of `/` and `\` in `text` as one `/`, for matching only.
///
/// Both are one ASCII byte and never occur inside a multi-byte character. With
/// no run of two or more, every byte keeps its offset, so no map is needed (a
/// lone `\` is a plain one-for-one replacement). Otherwise the map holds, for
/// each byte offset of the returned text plus its end, the offset of the same
/// position in `text`: a merged `/` maps to the start of its run and the byte
/// after it to the end of the run.
fn unify_separators(text: &str) -> (Cow<'_, str>, Option<Vec<usize>>) {
    let is_separator = |byte: u8| byte == b'/' || byte == b'\\';
    let has_run = text
        .as_bytes()
        .windows(2)
        .any(|pair| is_separator(pair[0]) && is_separator(pair[1]));
    if !has_run {
        return if text.contains('\\') {
            (Cow::Owned(text.replace('\\', "/")), None)
        } else {
            (Cow::Borrowed(text), None)
        };
    }
    let mut unified = String::with_capacity(text.len());
    let mut starts = Vec::with_capacity(text.len() + 1);
    let mut in_run = false;
    for (index, character) in text.char_indices() {
        if character == '/' || character == '\\' {
            if !in_run {
                unified.push('/');
                starts.push(index);
            }
            in_run = true;
        } else {
            in_run = false;
            unified.push(character);
            starts.extend(index..index + character.len_utf8());
        }
    }
    starts.push(text.len());
    (Cow::Owned(unified), Some(starts))
}

fn report_result(result: &ScanResult, show_matches: bool) -> ScanResult {
    let mut report = result.clone();
    if show_matches {
        return report;
    }
    // The longest text redacted below. A longer value cannot occur in any of
    // them, so the redactor leaves it out.
    let max_text_len = result
        .hits
        .iter()
        .flat_map(|hit| {
            [&hit.file, &hit.source, &hit.suggestion]
                .into_iter()
                .chain(&hit.variable)
        })
        .chain(&result.warnings)
        .map(String::len)
        .max()
        .unwrap_or(0);
    let redactor = Redactor::new(
        result.hits.iter().map(|hit| hit.matched.as_str()),
        max_text_len,
    );
    for hit in &mut report.hits {
        hit.matched = "[redacted]".to_owned();
        hit.file = redactor.redact(&hit.file);
        hit.variable = hit
            .variable
            .as_deref()
            .map(|variable| redactor.redact(variable));
        hit.source = redactor.redact(&hit.source);
        hit.suggestion = redactor.redact(&hit.suggestion);
    }
    report.warnings = report
        .warnings
        .iter()
        .map(|warning| redactor.redact(warning))
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
        !args.paths.is_empty() || args.path_from.is_some(),
    ]
    .into_iter()
    .filter(|selected| *selected)
    .count();
    if mode_count != 1 {
        anyhow::bail!(
            "scan requires exactly one mode. Pass exactly one of --staged, --diff, --all-tracked, --history, --packaged, --files-from-list, or --path/--path-from"
        );
    }
    let cwd = std::env::current_dir()?;
    let scan_mode = mode(args);
    let scan_root = if matches!(scan_mode, ScanMode::Packaged | ScanMode::Paths) {
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
        let targets = if scan_mode == ScanMode::Paths {
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
            let paths = if let Some(list) = &args.files_from_list {
                scan::files_from_list(list, &cwd, &scan_root)?
            } else {
                scan::files_for_mode(scan_mode, &scan_root)?
            };
            paths.into_iter().map(scan::ScanTarget::from).collect()
        };
        scan::scan_targets(
            scan_mode,
            targets,
            &scan_root,
            &loaded.config,
            &watchlists.matcher,
            &patterns,
            warnings,
        )?
    };
    let report = report_result(&result, args.show_matches || args.show_matched);
    for warning in &report.warnings {
        eprintln!("{}", sanitize_terminal(warning));
    }
    match args.format {
        OutputFormat::Json => println!("{}", json_report(&report)?),
        OutputFormat::Text => {
            let report = text_report(&report);
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

#[cfg(test)]
mod redaction_tests {
    use super::{Redactor, report_result};
    use crate::scan::{HitKind, ScanHit, ScanMode, ScanResult};
    use aho_corasick::AhoCorasickKind;

    /// ASCII lowercase, then each run of `/` and `\` becomes one `/`.
    fn fold(text: &str) -> String {
        let mut folded = String::new();
        let mut in_run = false;
        for character in text.to_ascii_lowercase().chars() {
            let separator = character == '/' || character == '\\';
            if !separator {
                folded.push(character);
            } else if !in_run {
                folded.push('/');
            }
            in_run = separator;
        }
        folded
    }

    /// The oracle, independent of `Redactor`: a brute force that hides every
    /// range of `text` (on character boundaries) whose fold equals the fold of
    /// some value, so values match across case and separator spelling.
    fn redact_reference(values: &[&str], text: &str) -> String {
        let mut needles = std::collections::HashSet::new();
        for value in values {
            let folded = fold(value);
            // A path-like value also hides its form without leading `/` and
            // trailing `.`, and its form without trailing `.`, `-`, `_` or
            // `.git` as well, while a form keeps a `/` before its end; the
            // shorter one only if it still ends in a name.
            let keeps_inner_slash = |form: &str| {
                let mut inner = form;
                while let Some(rest) = inner.strip_suffix('/') {
                    inner = rest;
                }
                inner.contains('/')
            };
            let mut dotless = folded.as_str();
            while let Some(rest) = dotless.strip_prefix('/') {
                dotless = rest;
            }
            while let Some(rest) = dotless.strip_suffix('.') {
                dotless = rest;
            }
            let mut bare = dotless;
            loop {
                let last = bare.chars().next_back();
                if matches!(last, Some('.' | '-' | '_')) {
                    bare = &bare[..bare.len() - 1];
                } else if bare.ends_with(".git") {
                    bare = &bare[..bare.len() - 4];
                } else {
                    break;
                }
            }
            if keeps_inner_slash(dotless) {
                needles.insert(dotless.to_owned());
            }
            let ends_in_name = !bare.ends_with('/') || dotless.ends_with('/');
            if keeps_inner_slash(bare) && ends_in_name {
                needles.insert(bare.to_owned());
            }
            needles.insert(folded);
        }
        needles.remove("");
        let bounds: Vec<usize> = text
            .char_indices()
            .map(|(index, _)| index)
            .chain([text.len()])
            .collect();
        let mut spans = Vec::new();
        for (position, &begin) in bounds.iter().enumerate() {
            for &end in &bounds[position + 1..] {
                if needles.contains(&fold(&text[begin..end])) {
                    spans.push((begin, end));
                }
            }
        }
        spans.sort_unstable();
        let mut merged: Vec<(usize, usize)> = Vec::new();
        for (start, end) in spans {
            if let Some(last) = merged.last_mut().filter(|last| start <= last.1) {
                last.1 = last.1.max(end);
            } else {
                merged.push((start, end));
            }
        }
        let mut redacted = String::new();
        let mut cursor = 0;
        for (start, end) in merged {
            redacted.push_str(&text[cursor..start]);
            redacted.push_str("[redacted]");
            cursor = end;
        }
        redacted.push_str(&text[cursor..]);
        redacted
    }

    /// Uses the tightest bound, which drops every value longer than the text.
    fn assert_matches_reference(values: &[&str], text: &str) {
        assert_eq!(
            Redactor::new(values.iter().copied(), text.len()).redact(text),
            redact_reference(values, text),
            "values={values:?} text={text:?}"
        );
    }

    #[test]
    fn single_pass_redaction_matches_the_per_value_reference() {
        let cases: &[(&[&str], &str)] = &[
            // Overlapping values, and one value overlapping itself.
            (&["abcab", "cabx"], "zabcabxz"),
            (&["aba"], "x abababa y"),
            // Touching spans merge into one marker (RFC 5737 addresses).
            (&["192.0.2.1", "zz"], "192.0.2.1zz 192.0.2.1 zz"),
            // One value inside another.
            (&["198.51.100.7", "100.7"], "at 198.51.100.7 and 100.79"),
            // ASCII case variants of text and values.
            (&["host-a", "HOST-B"], "HOST-A host-b Host-A hOsT-b"),
            // Non-ASCII before, inside and after; non-ASCII compares exactly.
            (&["東京", "é"], "前東京後 café É 東京x"),
            (&["xé", "éy"], "東xéy東"),
            (&["aé"], "AÉ aé Aé"),
            (&["É"], "é É"),
            // Duplicates, empty values and values that never appear.
            (&["dup", "dup", "DUP", ""], "dup Dup x"),
            (&[""], "unchanged"),
            (&[], "unchanged"),
            (&["absent"], "nothing here"),
            // The whole text, and empty text.
            (&["203.0.113.9"], "203.0.113.9"),
            (&["abc"], ""),
        ];
        for (values, text) in cases {
            assert_matches_reference(values, text);
        }
        assert_eq!(
            Redactor::new(["abcab", "cabx"], usize::MAX).redact("zabcabxz"),
            "z[redacted]z"
        );
        assert_eq!(
            Redactor::new([""], usize::MAX).redact("unchanged"),
            "unchanged"
        );
    }

    #[test]
    fn values_longer_than_the_bound_stay_out_of_the_automaton() {
        let text = "at 192.0.2.1 and 192.0.2.10";
        let long = format!("{text} and more");
        let values = [long.as_str(), "192.0.2.1", "192.0.2.10"];
        let redactor = Redactor::new(values, text.len());
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 2);
        assert_eq!(redactor.redact(text), redact_reference(&values, text));
        // A value exactly as long as the bound is kept.
        assert_eq!(Redactor::new([text], text.len()).redact(text), "[redacted]");
        // With only over-long values there is no automaton at all.
        let redactor = Redactor::new([long.as_str()], text.len());
        assert!(redactor.matcher.is_none());
        assert_eq!(redactor.redact(text), text);
    }

    #[test]
    fn ascii_case_variants_share_one_pattern() {
        let text = "HOST-A host-a Host-A x";
        let values = ["host-a", "HOST-A", "Host-A", "hOsT-a"];
        let redactor = Redactor::new(values, text.len());
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
        assert_eq!(redactor.redact(text), redact_reference(&values, text));
        // Non-ASCII letters are compared exactly, so they stay apart.
        let redactor = Redactor::new(["é", "É"], 2);
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 2);
    }

    #[test]
    fn path_separators_match_each_other() {
        // A value written with `/` hides the same path written with `\`.
        assert_eq!(
            Redactor::new(["github/private/x"], usize::MAX).redact(r"github\private\x\f.md"),
            r"[redacted]\f.md"
        );
        // A value written with `\` hides both spellings.
        let redactor = Redactor::new([r"a\b"], usize::MAX);
        assert_eq!(redactor.redact(r"a/b a\b"), "[redacted] [redacted]");
        // Mixed separators in the value and in the text.
        let values = [r"a/b\c"];
        let text = r"A\B/C and a/b/c and a\b\c";
        assert_eq!(
            Redactor::new(values, text.len()).redact(text),
            "[redacted] and [redacted] and [redacted]"
        );
        assert_matches_reference(&values, text);
        assert_matches_reference(&[r"A\B/C"], r"a/b\c");
        // The two spellings are one pattern.
        let values = ["a/b", r"a\b"];
        let redactor = Redactor::new(values, 3);
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
    }

    #[test]
    fn runs_of_separators_are_one_separator() {
        // A value whose separators come in pairs (as in escaped JSON text)
        // hides names that use a single separator, whichever one.
        for value in [r"github\\private\\x", "github//private//x"] {
            let redactor = Redactor::new([value], usize::MAX);
            assert_eq!(
                redactor.redact(r"github\private\x\f.md"),
                r"[redacted]\f.md"
            );
            assert_eq!(redactor.redact("github/private/x/f.md"), "[redacted]/f.md");
        }
        // A value with single separators hides a whole run in the text.
        let redactor = Redactor::new(["a/b"], usize::MAX);
        assert_eq!(redactor.redact("a//b"), "[redacted]");
        assert_eq!(redactor.redact(r"a\\b"), "[redacted]");
        assert_eq!(redactor.redact(r"x a/\/\b y"), "x [redacted] y");
        // A value starting or ending with a separator reaches the whole run,
        // even though the match starts or ends inside it.
        let redactor = Redactor::new(["/b"], usize::MAX);
        assert_eq!(redactor.redact("a//b"), "a[redacted]");
        assert_eq!(redactor.redact(r"a\\\b"), "a[redacted]");
        let redactor = Redactor::new(["a/"], usize::MAX);
        assert_eq!(redactor.redact("a//b"), "[redacted]b");
        assert_eq!(redactor.redact(r"a\\\b"), "[redacted]b");
        // Multi-byte characters next to a run stay intact.
        let redactor = Redactor::new(["東/é"], usize::MAX);
        assert_eq!(redactor.redact(r"x東\\é 東//é"), "x[redacted] [redacted]");
        let redactor = Redactor::new(["/"], usize::MAX);
        assert_eq!(
            redactor.redact(r"東//é\\É/"),
            "東[redacted]é[redacted]É[redacted]"
        );
        for (values, text) in [
            (&["a/b"][..], "a//b a\\\\b a/\\/b"),
            (&["/b"], "東//b é\\\\b"),
            (&["a/"], "a//é a\\\\東"),
            (&["//"], "x///y"),
        ] {
            assert_matches_reference(values, text);
        }
        // Values equal after folding runs are one pattern.
        let redactor = Redactor::new(["a/b", r"a\\b", "a//b"], 4);
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
    }

    #[test]
    fn path_like_values_also_hide_their_trimmed_form() {
        // A trailing `.` that ended a sentence is not part of the name.
        let redactor = Redactor::new(["github/private/x."], usize::MAX);
        assert_eq!(
            redactor.redact(r"github\private\x\f.md"),
            r"[redacted]\f.md"
        );
        // Leading separators are not part of the relative name.
        let home = format!("/{}/{}/", "home", "synthetic-user");
        let redactor = Redactor::new([home.as_str()], usize::MAX);
        assert_eq!(
            redactor.redact("home/synthetic-user/notes.md"),
            "[redacted]notes.md"
        );
        assert_eq!(
            redactor.redact(&format!("{home}notes.md")),
            "[redacted]notes.md"
        );
        // Without a `/` the value is kept whole, so `Inc` stays visible.
        let redactor = Redactor::new(["inc."], usize::MAX);
        assert_eq!(redactor.redact("Inc and Inc."), "Inc and [redacted]");
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
        // A trimmed form with no `/` before its end no longer names a path,
        // so it is not a pattern: `/e` must not hide every `e`.
        for value in ["/e", "/opt.", "/a/", r"\\x."] {
            let redactor = Redactor::new([value], usize::MAX);
            assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
            assert_eq!(redactor.redact("release data/x box"), "release data/x box");
        }
        // A trimmed form that would be empty is not a pattern.
        let redactor = Redactor::new(["/", "//."], usize::MAX);
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 2);
        // A path-like value adds exactly one trimmed form.
        let redactor = Redactor::new(["/a/b."], usize::MAX);
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 2);
        // The trimmed form is subject to the same length bound.
        let redactor = Redactor::new(["/a/b."], 3);
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
        for (values, text) in [
            (&["/a/b."][..], "a/b. /a/b./ a\\b x/a/b"),
            (&["inc."], "Inc. Inc"),
            (&["//."], "x// ./"),
        ] {
            assert_matches_reference(values, text);
        }
    }

    #[test]
    fn path_like_values_also_hide_the_form_without_name_suffixes() {
        // A clone-URL `.git`, a hyphen or underscore, or a mix of them after
        // the name is not part of the name in a file path.
        for value in [
            "github/private/x.git",
            "github/private/x-",
            "github/private/x_",
            "github/private/x.git-",
            "github/private/x..",
            "GitHub/Private/X.GIT",
        ] {
            let redactor = Redactor::new([value], usize::MAX);
            assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 2);
            assert_eq!(
                redactor.redact(r"github\private\x\f.md"),
                r"[redacted]\f.md",
                "{value}"
            );
            assert_matches_reference(&[value], r"github\private\x\f.md Github/Private/X.git-");
        }
        // The form without only the trailing dots is kept as well, so a name
        // that keeps part of the suffix is still hidden whole.
        for (value, name) in [
            ("github/private/x.git.", r"github\private\x.git\f.md"),
            ("github/private/x-.", r"github\private\x-\f.md"),
            ("github/private/x_.", r"github\private\x_\f.md"),
        ] {
            let redactor = Redactor::new([value], usize::MAX);
            assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 3);
            assert_eq!(redactor.redact(name), r"[redacted]\f.md", "{value}");
            assert_eq!(
                redactor.redact(r"github\private\x\f.md"),
                r"[redacted]\f.md",
                "{value}"
            );
            assert_matches_reference(&[value], name);
        }
        // A last name that is all suffix is not cut back to its folder, which
        // would hide the start of every name in that folder.
        for value in ["github/private/-", "github/private/.git"] {
            let redactor = Redactor::new([value], usize::MAX);
            assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
            let name = r"github\private\y\f.md";
            assert_eq!(redactor.redact(name), name, "{value}");
            assert_matches_reference(&[value], name);
        }
        // Without a `/` before the end the shorter form is not added.
        for value in ["inc-", "/a-", "/e.git", "a/.git", "x_"] {
            let redactor = Redactor::new([value], usize::MAX);
            assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 1);
            assert_matches_reference(&[value], "inc a e a/ x");
        }
        // A trailing separator ends the name already: nothing more is dropped.
        let home = format!("/{}/{}/", "home", "synthetic-user");
        let redactor = Redactor::new([home.as_str()], usize::MAX);
        assert_eq!(redactor.matcher.as_ref().unwrap().patterns_len(), 2);
        assert_eq!(
            redactor.redact("home/synthetic-user/notes.md"),
            "[redacted]notes.md"
        );
    }

    #[test]
    fn the_length_bound_covers_every_redacted_field() {
        // Each field in turn is the only text long enough to hold the value, so
        // leaving that field out of the bound would leave the value in it.
        let value = "synthetic-long-value";
        for field in 0..5 {
            let text = |index: usize| {
                if index == field {
                    format!("<{value}>")
                } else {
                    "x".to_owned()
                }
            };
            let input = ScanResult {
                watchlist_statistics: Default::default(),
                baseline_applied: false,
                baseline_suppressed_hits: 0,
                mode: ScanMode::Paths,
                scanned: 1,
                total_files: 1,
                exempt_or_skipped: 0,
                coverage_skips: 0,
                watchlist_needles: 1,
                structural_patterns: 0,
                hits: vec![ScanHit {
                    blob_oid: None,
                    file: text(0),
                    line_number: 1,
                    matched: value.to_owned(),
                    variable: Some(text(1)),
                    source: text(2),
                    kind: HitKind::Watchlist,
                    suggestion: text(3),
                }],
                warnings: vec![text(4)],
            };
            let output = report_result(&input, false);
            let hit = &output.hits[0];
            let redacted = [
                hit.file.as_str(),
                hit.variable.as_deref().unwrap(),
                hit.source.as_str(),
                hit.suggestion.as_str(),
                output.warnings[0].as_str(),
            ];
            for (index, got) in redacted.into_iter().enumerate() {
                let expected = if index == field { "<[redacted]>" } else { "x" };
                assert_eq!(got, expected, "field {field}");
            }
        }
    }

    /// xorshift64: deterministic and dependency-free.
    fn next(state: &mut u64, bound: u64) -> u64 {
        *state ^= *state << 13;
        *state ^= *state >> 7;
        *state ^= *state << 17;
        *state % bound
    }

    /// Up to `max_chars` characters mixing ASCII case variants, both path
    /// separators and multi-byte characters, including a non-ASCII pair that
    /// differs only in case.
    fn random_word(state: &mut u64, max_chars: u64) -> String {
        const ALPHABET: [&str; 10] = ["a", "b", "A", "B", ".", "é", "É", "東", "/", "\\"];
        let length = next(state, max_chars + 1);
        (0..length)
            .map(|_| ALPHABET[next(state, ALPHABET.len() as u64) as usize])
            .collect()
    }

    #[test]
    fn single_pass_redaction_matches_the_reference_on_random_inputs() {
        let mut state = 0x2545_f491_4f6c_dd1d_u64;
        for _ in 0..20_000 {
            let text = random_word(&mut state, 24);
            let count = 1 + next(&mut state, 4);
            let values: Vec<String> = (0..count).map(|_| random_word(&mut state, 4)).collect();
            let values: Vec<&str> = values.iter().map(String::as_str).collect();
            assert_matches_reference(&values, &text);
        }
    }

    #[test]
    fn single_pass_redaction_matches_the_reference_with_over_a_hundred_values() {
        // Over 100 patterns make aho-corasick build an NFA instead of a DFA.
        let mut state = 0x9e37_79b9_7f4a_7c15_u64;
        for case in 0..200 {
            // At least 32 characters, so every short value fits the bound.
            let text = loop {
                let text = random_word(&mut state, 64);
                if text.chars().count() >= 32 {
                    break text;
                }
            };
            let chars: Vec<char> = text.chars().collect();
            let mut values = Vec::new();
            // Words of 5 to 8 characters, which mostly do not occur.
            while values.len() < 150 {
                let word = random_word(&mut state, 8);
                if word.chars().count() >= 5 {
                    values.push(word);
                }
            }
            // Pieces of the text, so spans overlap and touch.
            for _ in 0..12 {
                let start = next(&mut state, chars.len() as u64) as usize;
                let end = (start + 1 + next(&mut state, 6) as usize).min(chars.len());
                values.push(chars[start..end].iter().collect());
            }
            // ASCII case variants, which fold into the same pattern.
            let variants: Vec<String> = values
                .iter()
                .step_by(5)
                .map(|value| value.to_ascii_uppercase())
                .collect();
            values.extend(variants);
            // The whole text is as long as the bound; the others are longer.
            if case % 10 == 0 {
                values.push(text.to_ascii_uppercase());
            }
            values.push(format!("{text}a"));
            values.push(format!("B{}", text.to_ascii_uppercase()));
            let values: Vec<&str> = values.iter().map(String::as_str).collect();
            let redactor = Redactor::new(values.iter().copied(), text.len());
            let matcher = redactor.matcher.as_ref().unwrap();
            assert!(matcher.patterns_len() > 100, "{}", matcher.patterns_len());
            assert_ne!(matcher.kind(), AhoCorasickKind::DFA);
            assert_eq!(
                redactor.redact(&text),
                redact_reference(&values, &text),
                "values={values:?} text={text:?}"
            );
        }
    }

    #[test]
    fn overlapping_ascii_variants_are_redacted_in_every_metadata_field() {
        let metadata = "prefix ABABA 東京 SeCoNd suffix";
        let hits = ["aba", "bab", "second"]
            .into_iter()
            .map(|matched| ScanHit {
                blob_oid: None,
                file: metadata.to_owned(),
                line_number: 1,
                matched: matched.to_owned(),
                variable: Some(metadata.to_owned()),
                source: metadata.to_owned(),
                kind: HitKind::Watchlist,
                suggestion: metadata.to_owned(),
            })
            .collect();
        let input = ScanResult {
            watchlist_statistics: Default::default(),
            baseline_applied: false,
            baseline_suppressed_hits: 0,
            mode: ScanMode::Paths,
            scanned: 1,
            total_files: 1,
            exempt_or_skipped: 0,
            coverage_skips: 0,
            watchlist_needles: 3,
            structural_patterns: 0,
            hits,
            warnings: vec![metadata.to_owned()],
        };
        let output = report_result(&input, false);
        let expected = "prefix [redacted] 東京 [redacted] suffix";
        for hit in output.hits {
            assert_eq!(hit.file, expected);
            assert_eq!(hit.variable.as_deref(), Some(expected));
            assert_eq!(hit.source, expected);
            assert_eq!(hit.suggestion, expected);
            assert_eq!(hit.matched, "[redacted]");
        }
        assert_eq!(output.warnings, vec![expected]);
        let shown = report_result(&input, true);
        assert_eq!(shown.hits, input.hits);
        assert_eq!(shown.warnings, input.warnings);
    }
}
