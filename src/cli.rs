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
        // case and treats `\` as `/`, so values differing only in ASCII case or
        // in path separator are one pattern.
        let mut values: Vec<_> = values
            .into_iter()
            .filter(|value| !value.is_empty() && value.len() <= max_text_len)
            .map(|value| value.to_ascii_lowercase().replace('\\', "/"))
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
        // bytes compare exactly. `\` and `/` are one byte each, so matching on
        // a copy with `\` turned into `/` keeps every span an offset into the
        // original text. Redact all overlapping spans once; replacement
        // markers are never fed back into the matcher.
        let unified = if text.contains('\\') {
            Cow::Owned(text.replace('\\', "/"))
        } else {
            Cow::Borrowed(text)
        };
        let mut spans: Vec<_> = matcher
            .find_overlapping_iter(unified.as_ref())
            .map(|found| (found.start(), found.end()))
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

    /// The per-value redaction that `Redactor` replaced, kept as the oracle.
    /// Matching folds ASCII case and treats `\` and `/` as the same separator.
    fn redact_reference(values: &[&str], text: &str) -> String {
        let mut values = values.to_vec();
        values.sort_by_key(|value| std::cmp::Reverse(value.len()));
        values.dedup();
        let folded = text.to_ascii_lowercase().replace('\\', "/");
        let mut spans = Vec::new();
        for value in &values {
            if value.is_empty() {
                continue;
            }
            let needle = value.to_ascii_lowercase().replace('\\', "/");
            let mut start = 0;
            while start < folded.len() {
                let Some(offset) = folded[start..].find(&needle) else {
                    break;
                };
                let begin = start + offset;
                spans.push((begin, begin + needle.len()));
                start = begin + folded[begin..].chars().next().unwrap().len_utf8();
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
