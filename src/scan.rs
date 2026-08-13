use std::{
    collections::HashSet,
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::OnceLock,
};

use anyhow::{Context, Result, anyhow, bail};
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;

use crate::{config::Config, patterns::StructuralPattern, watchlist::WatchlistMatcher};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScanMode {
    Staged,
    Diff,
    AllTracked,
    Packaged,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HitKind {
    Watchlist,
    Structural,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScanHit {
    pub file: String,
    #[serde(rename = "line_number")]
    pub line_number: usize,
    pub matched: String,
    pub source: String,
    pub kind: HitKind,
    pub suggestion: String,
}

#[derive(Debug, Serialize)]
pub struct ScanResult {
    pub mode: ScanMode,
    pub scanned: usize,
    #[serde(rename = "total_files")]
    pub total_files: usize,
    #[serde(rename = "exempt_or_skipped")]
    pub exempt_or_skipped: usize,
    /// Skips that reduce coverage (oversize, non-UTF-8, symlink, unreadable blob).
    #[serde(rename = "coverage_skips")]
    pub coverage_skips: usize,
    #[serde(rename = "watchlist_needles")]
    pub watchlist_needles: usize,
    #[serde(rename = "structural_patterns")]
    pub structural_patterns: usize,
    pub hits: Vec<ScanHit>,
    pub warnings: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eligibility {
    Scan,
    QuietSkip,
    CoverageSkip,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum ContentLoad {
    Text(String),
    CoverageSkip(&'static str),
}

const BINARY_EXTENSIONS: &[&str] = &[
    ".png", ".jpg", ".jpeg", ".gif", ".bmp", ".webp", ".ico", ".pdf", ".zip", ".tar", ".gz",
    ".bz2", ".xz", ".7z", ".rar", ".exe", ".dll", ".so", ".dylib", ".bin", ".o", ".obj", ".woff",
    ".woff2", ".ttf", ".otf", ".eot", ".mp3", ".mp4", ".wav", ".avi", ".mov", ".webm", ".m4a",
];

const SKIP_FILENAMES: &[&str] = &[
    "pnpm-lock.yaml",
    "package-lock.json",
    "yarn.lock",
    "Cargo.lock",
    "go.sum",
    "poetry.lock",
    "Pipfile.lock",
];

/// Resolve a tool strictly from PATH directories, never the cwd, so a malicious
/// repository cannot plant `.\git.exe` (Windows CreateProcess searches cwd first).
fn cached_program(
    name: &'static str,
    cache: &'static OnceLock<Option<PathBuf>>,
) -> Result<&'static Path> {
    cache
        .get_or_init(|| which::which_global(name).ok())
        .as_deref()
        .ok_or_else(|| anyhow!("could not find `{name}` on PATH"))
}

pub(crate) fn git_program() -> Result<&'static Path> {
    static GIT: OnceLock<Option<PathBuf>> = OnceLock::new();
    cached_program("git", &GIT)
}

fn npm_program() -> Result<&'static Path> {
    static NPM: OnceLock<Option<PathBuf>> = OnceLock::new();
    cached_program("npm", &NPM)
}

fn run_command(program: &Path, args: &[&str], cwd: &Path, strip_env: &[&str]) -> Result<String> {
    let mut command = Command::new(program);
    command.args(args).current_dir(cwd);
    for variable in strip_env {
        command.env_remove(variable);
    }
    let output = command
        .output()
        .with_context(|| format!("failed to start {}", program.display()))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(
            "{}",
            if message.is_empty() {
                format!("{} {} failed", program.display(), args.join(" "))
            } else {
                message
            }
        );
    }
    String::from_utf8(output.stdout).context("command output was not UTF-8")
}

fn run_git(args: &[&str], cwd: &Path) -> Result<String> {
    let mut full = Vec::with_capacity(args.len() + 2);
    full.push("-c");
    full.push("core.quotepath=false");
    full.extend_from_slice(args);
    run_command(git_program()?, &full, cwd, &[])
}

fn nul_paths(output: String) -> Vec<String> {
    output
        .split_terminator('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect()
}

fn packaged_files(cwd: &Path) -> Result<Vec<String>> {
    // `--ignore-scripts` keeps the manifest under scan from running lifecycle
    // scripts; stripping publish credentials guards the same boundary in case a
    // future npm version runs anything anyway.
    let output = run_command(
        npm_program()?,
        &["pack", "--dry-run", "--json", "--ignore-scripts"],
        cwd,
        &["NPM_TOKEN", "NODE_AUTH_TOKEN"],
    )?;
    let packages: serde_json::Value =
        serde_json::from_str(&output).context("could not parse npm pack file list")?;
    let packages = packages
        .as_array()
        .ok_or_else(|| anyhow!("npm pack did not return a package array"))?;
    let mut paths = Vec::new();
    let mut seen = HashSet::new();
    for package in packages {
        let Some(files) = package.get("files").and_then(serde_json::Value::as_array) else {
            continue;
        };
        for file in files {
            if let Some(path) = file.get("path").and_then(serde_json::Value::as_str) {
                if seen.insert(path.to_owned()) {
                    paths.push(path.to_owned());
                }
            }
        }
    }
    if paths.is_empty() {
        bail!("npm pack did not return a file list");
    }
    Ok(paths)
}

pub fn files_for_mode(mode: ScanMode, cwd: &Path) -> Result<Vec<String>> {
    match mode {
        ScanMode::Staged => Ok(nul_paths(run_git(
            &[
                "diff",
                "--cached",
                "--name-only",
                "-z",
                "--diff-filter=ACMRT",
            ],
            cwd,
        )?)),
        ScanMode::Diff => Ok(nul_paths(run_git(
            &["diff", "--name-only", "-z", "--diff-filter=ACMRT", "HEAD"],
            cwd,
        )?)),
        ScanMode::AllTracked => Ok(nul_paths(run_git(&["ls-files", "-z"], cwd)?)),
        ScanMode::Packaged => packaged_files(cwd),
    }
}

fn directive_regex() -> &'static Regex {
    static DIRECTIVE: OnceLock<Regex> = OnceLock::new();
    DIRECTIVE.get_or_init(|| {
        // Tokens may contain `-` (Anne-Marie); the `-->` comment closer is cut
        // off below because the regex crate has no lookahead to stop on it.
        Regex::new(r"(?i)(?:doxguard|secrets-scan):\s*allow\b(?:\s+([^\s>]+))?")
            .expect("static directive regex")
    })
}

pub fn allowed_by_directive(line: &str, matched: &str, config: &Config) -> bool {
    directive_regex().captures_iter(line).any(|capture| {
        let Some(target) = capture.get(1) else {
            // Bare `doxguard: allow` — optional hard-off for CI / --strict.
            return !config.allow.disallow_bare_allow;
        };
        let mut token = target.as_str();
        // `allow token-->` captures `token--`; drop the HTML comment closer.
        if line[target.end()..].starts_with('>') {
            token = token.strip_suffix("--").unwrap_or(token);
        }
        if token.is_empty() {
            // `<!-- doxguard: allow -->` is the bare form, not a token.
            return !config.allow.disallow_bare_allow;
        }
        let target = token.to_ascii_lowercase();
        // Short tokens (e.g. "168", ".") over-suppress structural hits.
        if target.chars().count() < 4 {
            return false;
        }
        let matched = matched.to_ascii_lowercase();
        // Scoped allow: token must appear in the match (not reverse substring).
        matched == target || matched.contains(&target)
    })
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

/// Path exemption uses path-boundary matching, not raw substring contains.
/// Empty / blank exempt entries never match (and are rejected at config load).
pub fn path_is_exempt(path: &str, exempt: &str) -> bool {
    let path = normalize_path(path);
    let exempt = normalize_path(exempt.trim());
    if exempt.is_empty() || exempt == "." || exempt == "/" {
        return false;
    }
    let dir_style = exempt.ends_with('/');
    let exempt = exempt.trim_end_matches('/');
    if exempt.is_empty() {
        return false;
    }
    if path == exempt {
        return true;
    }
    if path.starts_with(&format!("{exempt}/")) {
        return true;
    }
    if dir_style {
        return false;
    }
    // Prefix with boundary: next char is end, '/', or '.' (workflow stem → .yml).
    if let Some(rest) = path.strip_prefix(exempt) {
        if rest.is_empty() || rest.starts_with('/') || rest.starts_with('.') {
            return true;
        }
    }
    path.ends_with(&format!("/{exempt}")) || path.contains(&format!("/{exempt}/"))
}

fn classify_path(path: &str, cwd: &Path, config: &Config, mode: ScanMode) -> Eligibility {
    let normalized = normalize_path(path);
    if config
        .all_exempt_paths()
        .any(|exempt| path_is_exempt(&normalized, exempt))
    {
        return Eligibility::QuietSkip;
    }
    let lower = normalized.to_ascii_lowercase();
    if BINARY_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(extension))
    {
        return Eligibility::QuietSkip;
    }
    let filename = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if SKIP_FILENAMES.contains(&filename) {
        return Eligibility::QuietSkip;
    }
    // Staged mode reads index blobs; worktree size/symlink checks would false-skip.
    if mode == ScanMode::Staged {
        return Eligibility::Scan;
    }
    let joined = cwd.join(path);
    let metadata = match fs::symlink_metadata(&joined) {
        Ok(metadata) => metadata,
        Err(_) => return Eligibility::QuietSkip,
    };
    if metadata.file_type().is_symlink() {
        return Eligibility::CoverageSkip;
    }
    if !metadata.is_file() {
        return Eligibility::QuietSkip;
    }
    if metadata.len() > config.max_file_size {
        return Eligibility::CoverageSkip;
    }
    Eligibility::Scan
}

const STAGED_BLOB_MAX_BYTES: u64 = 16 * 1024 * 1024;

fn read_scan_content(
    mode: ScanMode,
    path: &str,
    cwd: &Path,
    max_file_size: u64,
) -> Result<ContentLoad> {
    if mode == ScanMode::Staged {
        // Index blob, not worktree — pre-commit must gate what will be committed.
        let spec = format!(":{path}");
        let limit = max_file_size.min(STAGED_BLOB_MAX_BYTES);
        // Ask for the blob size first so an oversize blob is never loaded.
        if let Ok(size) = run_git(&["cat-file", "-s", &spec], cwd) {
            if size.trim().parse::<u64>().is_ok_and(|size| size > limit) {
                return Ok(ContentLoad::CoverageSkip("oversize staged blob"));
            }
        }
        match run_git(&["show", &spec], cwd) {
            Ok(content) => {
                if content.len() as u64 > limit {
                    return Ok(ContentLoad::CoverageSkip("oversize staged blob"));
                }
                Ok(ContentLoad::Text(content))
            }
            Err(_) => Ok(ContentLoad::CoverageSkip(
                "unreadable or non-UTF-8 staged blob",
            )),
        }
    } else {
        match fs::read_to_string(cwd.join(path)) {
            Ok(content) => Ok(ContentLoad::Text(content)),
            Err(error) if error.kind() == std::io::ErrorKind::InvalidData => {
                Ok(ContentLoad::CoverageSkip("non-UTF-8 content"))
            }
            Err(error) => Err(error).with_context(|| format!("failed to read {path}")),
        }
    }
}

fn watchlist_hits(
    line: &str,
    path: &str,
    line_number: usize,
    matcher: &WatchlistMatcher,
    config: &Config,
) -> Vec<ScanHit> {
    let haystack = if config.noise.ascii_case_insensitive {
        line.to_ascii_lowercase()
    } else {
        line.to_owned()
    };
    matcher
        .matches_spanned(&haystack)
        .filter(|(item, _)| !allowed_by_directive(line, &item.needle, config))
        .map(|(item, span)| ScanHit {
            file: path.to_owned(),
            line_number,
            // Slice the original line so the report keeps its casing even when
            // the haystack was lowercased (ASCII lowering preserves offsets).
            matched: line[span].to_owned(),
            source: item.source.clone(),
            kind: HitKind::Watchlist,
            suggestion: "Generalize or remove the watchlist-derived value".to_owned(),
        })
        .collect()
}

struct FileScanOutcome {
    hits: Vec<ScanHit>,
    coverage_skip: Option<&'static str>,
}

fn scan_file(
    mode: ScanMode,
    path: &str,
    cwd: &Path,
    config: &Config,
    matcher: &WatchlistMatcher,
    patterns: &[StructuralPattern],
) -> Result<FileScanOutcome> {
    match read_scan_content(mode, path, cwd, config.max_file_size)? {
        ContentLoad::CoverageSkip(reason) => Ok(FileScanOutcome {
            hits: Vec::new(),
            coverage_skip: Some(reason),
        }),
        ContentLoad::Text(content) => {
            let mut hits = Vec::new();
            for (index, line) in content.lines().enumerate() {
                let line_number = index + 1;
                hits.extend(watchlist_hits(line, path, line_number, matcher, config));
                for pattern in patterns {
                    let mut seen = HashSet::new();
                    for found in pattern.regex.find_iter(line) {
                        let matched = found.as_str();
                        if !seen.insert(matched)
                            || pattern.is_allowed(matched, config)
                            || allowed_by_directive(line, matched, config)
                        {
                            continue;
                        }
                        hits.push(ScanHit {
                            file: path.to_owned(),
                            line_number,
                            matched: matched.to_owned(),
                            source: format!("structural: {}", pattern.name),
                            kind: HitKind::Structural,
                            suggestion: pattern.suggestion.clone(),
                        });
                    }
                }
            }
            Ok(FileScanOutcome {
                hits,
                coverage_skip: None,
            })
        }
    }
}

pub fn scan_paths(
    mode: ScanMode,
    paths: Vec<String>,
    cwd: &Path,
    config: &Config,
    matcher: &WatchlistMatcher,
    patterns: &[StructuralPattern],
    warnings: Vec<String>,
) -> Result<ScanResult> {
    let total_files = paths.len();
    let mut warnings = warnings;
    let mut coverage_skips = 0usize;
    let mut files = Vec::new();
    for path in paths {
        match classify_path(&path, cwd, config, mode) {
            Eligibility::Scan => files.push(path),
            Eligibility::QuietSkip => {}
            Eligibility::CoverageSkip => {
                coverage_skips += 1;
                let reason = {
                    let joined = cwd.join(&path);
                    if let Ok(meta) = fs::symlink_metadata(&joined) {
                        if meta.file_type().is_symlink() {
                            "symlink"
                        } else if meta.len() > config.max_file_size {
                            "oversize"
                        } else {
                            "unscannable"
                        }
                    } else {
                        "unscannable"
                    }
                };
                warnings.push(format!("WARN: skipped {path} ({reason})"));
            }
        }
    }
    let outcomes: Result<Vec<FileScanOutcome>> = if files.len() <= 4 {
        files
            .iter()
            .map(|path| scan_file(mode, path, cwd, config, matcher, patterns))
            .collect()
    } else {
        files
            .par_iter()
            .map(|path| scan_file(mode, path, cwd, config, matcher, patterns))
            .collect()
    };
    let mut hits = Vec::new();
    let mut scanned = 0usize;
    for (path, outcome) in files.iter().zip(outcomes?) {
        if let Some(reason) = outcome.coverage_skip {
            coverage_skips += 1;
            warnings.push(format!("WARN: skipped {path} ({reason})"));
        } else {
            scanned += 1;
        }
        hits.extend(outcome.hits);
    }
    if coverage_skips > 0 {
        warnings.push(format!(
            "WARN: {coverage_skips} coverage skip(s); content was not fully scanned"
        ));
    }
    Ok(ScanResult {
        mode,
        scanned,
        total_files,
        exempt_or_skipped: total_files - scanned,
        coverage_skips,
        watchlist_needles: matcher.len(),
        structural_patterns: patterns.len(),
        hits,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::nul_paths;

    #[test]
    fn nul_paths_preserves_whitespace_and_newlines() {
        assert_eq!(
            nul_paths(" leading.txt\0trailing.txt \0line\nbreak.txt\0".to_owned()),
            [" leading.txt", "trailing.txt ", "line\nbreak.txt"]
        );
    }
}
