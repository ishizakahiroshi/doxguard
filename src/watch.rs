//! `doxguard watch add`: append terms to a `lines` watchlist that lives outside the repository.
//!
//! This is the only command that writes a watchlist. It appends only, never removes or
//! rewrites, and never reports a term value or a path (see `AddReport`).

use std::{
    collections::{BTreeSet, HashMap, HashSet},
    fs,
    io::Write,
    path::Path,
};

use anyhow::{Context, Result, bail};

use crate::{
    config::{Config, WatchlistSource},
    watchlist,
};

const WATCHLIST_MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Default)]
pub struct AddReport {
    pub added: usize,
    pub exists: usize,
    /// Reason kinds only (static labels), never the rejected value.
    pub rejected: Vec<&'static str>,
    /// Effective terms in the target source after the append (or what it would hold for a dry run).
    pub total: usize,
}

impl AddReport {
    pub fn rejected_kinds(&self) -> Vec<&'static str> {
        self.rejected
            .iter()
            .copied()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    }

    pub fn succeeded(&self) -> bool {
        self.rejected.is_empty()
    }
}

enum Verdict {
    Valid(String),
    Rejected(&'static str),
}

fn judge(
    raw: &str,
    source: &WatchlistSource,
    config: &Config,
    allowed: &HashSet<String>,
) -> Verdict {
    let term = raw.trim();
    if term.is_empty() {
        return Verdict::Rejected("empty");
    }
    if term.chars().any(char::is_control) {
        return Verdict::Rejected("control-character");
    }
    if term.starts_with('#') {
        return Verdict::Rejected("comment-prefix");
    }
    if term.chars().count() < config.noise.min_needle_length {
        return Verdict::Rejected("too-short");
    }
    let normalized = watchlist::normalize_needle(term, config.noise.ascii_case_insensitive);
    if allowed.contains(&normalized) {
        return Verdict::Rejected("allow-listed");
    }
    if watchlist::should_skip(&normalized, source, config) {
        return Verdict::Rejected("ignored-by-noise-rules");
    }
    Verdict::Valid(term.to_owned())
}

/// Number of effective terms `text` would contribute (same rules as `watchlist::load`).
fn effective_count(
    text: &str,
    source: &WatchlistSource,
    config: &Config,
    allowed: &HashSet<String>,
) -> usize {
    let mut seen = HashSet::new();
    watchlist::parse_lines(text)
        .iter()
        .map(|value| watchlist::normalize_needle(value.trim(), config.noise.ascii_case_insensitive))
        .filter(|needle| {
            !watchlist::should_skip(needle, source, config)
                && !allowed.contains(needle)
                && seen.insert(needle.clone())
        })
        .count()
}

/// The target location for an error message, only when `--show-paths` /
/// `DOXGUARD_SHOW_PATHS` is on. Success output never uses this.
fn shown_location(path: &Path) -> String {
    if crate::config::show_paths() {
        format!(" ({})", path.display())
    } else {
        String::new()
    }
}

/// Append `terms` to the chosen `lines` source.
///
/// `base` resolves a relative watchlist path (same base `scan` uses); `repo_root` is the
/// worktree the target must stay outside of. Error messages never contain a term; they
/// contain a location only when `--show-paths` / `DOXGUARD_SHOW_PATHS` is on.
pub fn add(
    config: &Config,
    base: &Path,
    repo_root: &Path,
    env: &HashMap<String, String>,
    terms: &[String],
    source_number: Option<usize>,
    dry_run: bool,
) -> Result<AddReport> {
    let number = match source_number {
        Some(0) => bail!(
            "--source is 1-based. Pass a number from 1 to the number of watchlist sources in the config, or omit --source to use the first `lines` source"
        ),
        Some(number) => number,
        None => config
            .watchlists
            .iter()
            .position(|source| matches!(source, WatchlistSource::Lines { .. }))
            .map(|index| index + 1)
            .context(
                "the config has no `lines` watchlist source to append to. Add a `lines` entry to `watchlists` in the config (its path should point outside the repository, via an environment variable such as `${WATCHLIST_ROOT}`), then run again",
            )?,
    };
    let Some(source) = config.watchlists.get(number - 1) else {
        bail!(
            "watchlist source #{number} does not exist (the config has {} source(s)). Pass --source with a number from 1 to {}, or omit --source",
            config.watchlists.len(),
            config.watchlists.len()
        );
    };
    if !matches!(source, WatchlistSource::Lines { .. }) {
        bail!(
            "watchlist source #{number} is not a `lines` source; only `lines` sources can be appended to. Pass --source with the number of a `lines` source, add a `lines` source to the config, or edit that file by hand"
        );
    }
    let target = match watchlist::expand_path_names(source.path(), base, env) {
        Ok(path) => path,
        Err(missing) => bail!(
            "environment variable(s) {} used by watchlist source #{number} not set; nothing was written. Set them in the shell that runs doxguard, then run again",
            missing.join(", ")
        ),
    };

    let Some(file_name) = target.file_name() else {
        bail!(
            "watchlist source #{number} does not name a file. Set its path in the config to a file (for example `${{WATCHLIST_ROOT}}/terms.txt`)"
        );
    };
    let parent = target.parent().unwrap_or_else(|| Path::new("."));
    let parent_real = parent.canonicalize().with_context(|| {
        format!(
            "the parent directory of watchlist source #{number} does not exist{}. Create that directory, or change the source path in the config, then run again",
            shown_location(parent)
        )
    })?;
    if !parent_real.is_dir() {
        bail!(
            "the parent of watchlist source #{number} is not a directory. Change the source path in the config to a file inside an existing directory"
        );
    }
    let target_real = parent_real.join(file_name);
    let repo_real = repo_root.canonicalize().context(
        "failed to resolve the repository root. Run doxguard from inside the repository",
    )?;
    if target_real.starts_with(&repo_real) {
        bail!(
            "watchlist source #{number} is inside the repository worktree{}; refusing to write terms into a tracked location. Point the source at a file outside the repository (via an environment variable such as `${{WATCHLIST_ROOT}}`), then run again",
            shown_location(&target_real)
        );
    }

    let existing = match fs::symlink_metadata(&target_real) {
        Ok(meta) => {
            if meta.file_type().is_symlink() {
                bail!(
                    "watchlist source #{number} is a symbolic link{}; refusing to write through it. Point the source at the real file instead of the link",
                    shown_location(&target_real)
                );
            }
            if !meta.file_type().is_file() {
                bail!(
                    "watchlist source #{number} is not a regular file{}. Point the source at a regular text file",
                    shown_location(&target_real)
                );
            }
            let limit = config.max_file_size.min(WATCHLIST_MAX_BYTES);
            if meta.len() > limit {
                bail!(
                    "watchlist source #{number} already exceeds the size limit ({limit} bytes). Remove terms from the file by hand or raise `maxFileSize` in the config, then run again"
                );
            }
            let bytes = fs::read(&target_real).with_context(|| {
                format!(
                    "failed to read watchlist source #{number}. Check that the file is readable"
                )
            })?;
            Some(String::from_utf8(bytes).map_err(|_| {
                anyhow::anyhow!(
                    "watchlist source #{number} is not valid UTF-8. Re-save the file as UTF-8, then run again"
                )
            })?)
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(_) => bail!(
            "failed to inspect watchlist source #{number}. Check the permissions of the file and its directory"
        ),
    };
    let existing_text = existing.as_deref().unwrap_or_default();

    let allowed = watchlist::allowed_names(config);
    let mut known: HashSet<String> = watchlist::parse_lines(existing_text)
        .iter()
        .map(|value| watchlist::normalize_needle(value.trim(), config.noise.ascii_case_insensitive))
        .collect();

    let mut report = AddReport::default();
    let mut additions: Vec<String> = Vec::new();
    for raw in terms {
        match judge(raw, source, config, &allowed) {
            Verdict::Rejected(reason) => report.rejected.push(reason),
            Verdict::Valid(term) => {
                let normalized =
                    watchlist::normalize_needle(&term, config.noise.ascii_case_insensitive);
                if known.insert(normalized) {
                    additions.push(term);
                    report.added += 1;
                } else {
                    report.exists += 1;
                }
            }
        }
    }

    // Match the existing line ending; a new file uses LF. No BOM is ever written.
    let eol = match existing_text.find('\n') {
        Some(index) if existing_text[..index].ends_with('\r') => "\r\n",
        _ => "\n",
    };
    let mut payload = String::new();
    if !additions.is_empty() && !existing_text.is_empty() && !existing_text.ends_with('\n') {
        payload.push_str(eol);
    }
    for term in &additions {
        payload.push_str(term);
        payload.push_str(eol);
    }

    let limit = config.max_file_size.min(WATCHLIST_MAX_BYTES);
    if existing_text.len() as u64 + payload.len() as u64 > limit {
        bail!(
            "appending would exceed the watchlist size limit ({limit} bytes); nothing was written. Remove terms from the file by hand or raise `maxFileSize` in the config, then run again"
        );
    }
    let after = format!("{existing_text}{payload}");
    report.total = effective_count(&after, source, config, &allowed);

    // Valid terms are still appended when others are rejected (the exit code reports it).
    if dry_run || payload.is_empty() {
        return Ok(report);
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&target_real)
        .with_context(|| {
            format!(
                "failed to open watchlist source #{number} for appending. Check that the file and its directory are writable"
            )
        })?;
    file.write_all(payload.as_bytes()).with_context(|| {
        format!(
            "failed to append to watchlist source #{number}. Check free disk space and that the file is writable; the file may be partly written, so review it by hand"
        )
    })?;
    file.flush()?;
    Ok(report)
}
