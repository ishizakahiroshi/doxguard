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

use crate::{
    config::Config,
    patterns::StructuralPattern,
    watchlist::{WatchlistItem, WatchlistMatcher},
};

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
    #[serde(rename = "watchlist_needles")]
    pub watchlist_needles: usize,
    #[serde(rename = "structural_patterns")]
    pub structural_patterns: usize,
    pub hits: Vec<ScanHit>,
    pub warnings: Vec<String>,
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

fn run_command(program: &str, args: &[&str], cwd: &Path) -> Result<String> {
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .with_context(|| format!("failed to start {program}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(
            "{}",
            if message.is_empty() {
                format!("{program} {} failed", args.join(" "))
            } else {
                message
            }
        );
    }
    String::from_utf8(output.stdout).context("command output was not UTF-8")
}

fn lines(output: String) -> Vec<String> {
    output
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(str::to_owned)
        .collect()
}

fn packaged_files(cwd: &Path) -> Result<Vec<String>> {
    let npm = if cfg!(windows) { "npm.cmd" } else { "npm" };
    let output = run_command(
        npm,
        &["pack", "--dry-run", "--json", "--ignore-scripts"],
        cwd,
    )?;
    let packages: serde_json::Value =
        serde_json::from_str(&output).context("could not parse npm pack file list")?;
    let files = packages
        .as_array()
        .and_then(|packages| packages.first())
        .and_then(|package| package.get("files"))
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| anyhow!("npm pack did not return a file list"))?;
    Ok(files
        .iter()
        .filter_map(|file| file.get("path").and_then(serde_json::Value::as_str))
        .map(str::to_owned)
        .collect())
}

pub fn files_for_mode(mode: ScanMode, cwd: &Path) -> Result<Vec<String>> {
    match mode {
        ScanMode::Staged => Ok(lines(run_command(
            "git",
            &["diff", "--cached", "--name-only", "--diff-filter=ACM"],
            cwd,
        )?)),
        ScanMode::Diff => Ok(lines(run_command(
            "git",
            &["diff", "--name-only", "--diff-filter=ACM", "HEAD"],
            cwd,
        )?)),
        ScanMode::AllTracked => Ok(lines(run_command("git", &["ls-files"], cwd)?)),
        ScanMode::Packaged => packaged_files(cwd),
    }
}

fn directive_regex() -> &'static Regex {
    static DIRECTIVE: OnceLock<Regex> = OnceLock::new();
    DIRECTIVE.get_or_init(|| {
        Regex::new(r"(?i)(?:doxguard|secrets-scan):\s*allow(?:\s+([^\s\->]+))?")
            .expect("static directive regex")
    })
}

pub fn allowed_by_directive(line: &str, matched: &str) -> bool {
    directive_regex().captures_iter(line).any(|capture| {
        let Some(target) = capture.get(1) else {
            return true;
        };
        let target = target.as_str().to_ascii_lowercase();
        let matched = matched.to_ascii_lowercase();
        matched.contains(&target) || target.contains(&matched)
    })
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/")
}

fn eligible(path: &str, cwd: &Path, config: &Config) -> bool {
    let normalized = normalize_path(path);
    if config
        .all_exempt_paths()
        .any(|exempt| normalized.contains(&normalize_path(exempt)))
    {
        return false;
    }
    let lower = normalized.to_ascii_lowercase();
    if BINARY_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(extension))
    {
        return false;
    }
    let filename = Path::new(&normalized)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if SKIP_FILENAMES.contains(&filename) {
        return false;
    }
    fs::metadata(cwd.join(path))
        .map(|metadata| metadata.is_file() && metadata.len() <= config.max_file_size)
        .unwrap_or(false)
}

fn watchlist_hits(
    line: &str,
    path: &str,
    line_number: usize,
    matcher: &WatchlistMatcher,
) -> Vec<ScanHit> {
    matcher
        .matches(line)
        .filter(|item| !allowed_by_directive(line, &item.needle))
        .map(|WatchlistItem { needle, source }| ScanHit {
            file: path.to_owned(),
            line_number,
            matched: needle.clone(),
            source: source.clone(),
            kind: HitKind::Watchlist,
            suggestion: "Generalize or remove the watchlist-derived value".to_owned(),
        })
        .collect()
}

pub fn scan_file(
    path: &str,
    cwd: &Path,
    config: &Config,
    matcher: &WatchlistMatcher,
    patterns: &[StructuralPattern],
) -> Result<Vec<ScanHit>> {
    let content = match fs::read_to_string(cwd.join(path)) {
        Ok(content) => content,
        Err(error) if error.kind() == std::io::ErrorKind::InvalidData => return Ok(Vec::new()),
        Err(error) => return Err(error).with_context(|| format!("failed to read {path}")),
    };
    let mut hits = Vec::new();
    for (index, line) in content.lines().enumerate() {
        let line_number = index + 1;
        hits.extend(watchlist_hits(line, path, line_number, matcher));
        for pattern in patterns {
            let mut seen = HashSet::new();
            for found in pattern.regex.find_iter(line) {
                let matched = found.as_str();
                if !seen.insert(matched)
                    || pattern.is_allowed(matched, config)
                    || allowed_by_directive(line, matched)
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
    Ok(hits)
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
    let files: Vec<String> = paths
        .into_iter()
        .filter(|path| eligible(path, cwd, config))
        .collect();
    let scanned = files.len();
    let hit_groups: Result<Vec<Vec<ScanHit>>> = if files.len() <= 4 {
        files
            .iter()
            .map(|path| scan_file(path, cwd, config, matcher, patterns))
            .collect()
    } else {
        files
            .par_iter()
            .map(|path| scan_file(path, cwd, config, matcher, patterns))
            .collect()
    };
    let hits = hit_groups?.into_iter().flatten().collect();
    Ok(ScanResult {
        mode,
        scanned,
        total_files,
        exempt_or_skipped: total_files - scanned,
        watchlist_needles: matcher.len(),
        structural_patterns: patterns.len(),
        hits,
        warnings,
    })
}

pub fn resolve_paths(cwd: &Path, paths: &[String]) -> Vec<PathBuf> {
    paths.iter().map(|path| cwd.join(path)).collect()
}
