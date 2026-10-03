use std::{
    collections::{HashMap, HashSet},
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::OnceLock,
};

use anyhow::{Context, Result, anyhow, bail};
use rayon::prelude::*;
use regex::Regex;
use serde::Serialize;

#[path = "history.rs"]
mod history;
pub use history::scan_history;

use crate::{
    config::Config,
    patterns::StructuralPattern,
    watchlist::{WatchlistMatcher, WatchlistStatistics},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ScanMode {
    Staged,
    Diff,
    AllTracked,
    Packaged,
    FilesFromList,
    History,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum HitKind {
    Watchlist,
    Structural,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ScanHit {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blob_oid: Option<String>,
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
    pub watchlist_statistics: WatchlistStatistics,
    pub baseline_applied: bool,
    pub baseline_suppressed_hits: usize,
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
enum ScanKind {
    /// Watchlist needles and structural patterns.
    Full,
    /// Watchlist needles only; structural patterns skipped. Used for `exemptPaths`,
    /// which are meant to silence synthetic structural fixtures but must still catch
    /// a real private identity that lands in an "exempt" file (F-B12).
    WatchlistOnly,
    /// Structural patterns only; watchlist matching skipped. Used for dependency
    /// lockfiles, which can carry private registry hosts / `file:` paths / private
    /// IPs but are noisy for watchlist matching (F-A01).
    StructuralOnly,
}

impl ScanKind {
    fn wants_watchlist(self) -> bool {
        matches!(self, ScanKind::Full | ScanKind::WatchlistOnly)
    }

    fn wants_structural(self) -> bool {
        matches!(self, ScanKind::Full | ScanKind::StructuralOnly)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Eligibility {
    Scan(ScanKind),
    QuietSkip,
    CoverageSkip(&'static str),
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
        .ok_or_else(|| anyhow!("could not find `{name}` on PATH. Install it or add it to PATH"))
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
    // The tool is named by its file name; the full location (which can reveal a home
    // directory) is shown only with `--show-paths`.
    let tool = if crate::config::show_paths() {
        program.display().to_string()
    } else {
        program.file_stem().map_or_else(
            || "command".to_owned(),
            |stem| stem.to_string_lossy().into_owned(),
        )
    };
    let output = command.output().with_context(|| {
        format!(
            "failed to start {tool}{}",
            if crate::config::show_paths() {
                ""
            } else {
                " (location hidden; pass --show-paths to show it)"
            }
        )
    })?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        bail!(
            "{}",
            if message.is_empty() {
                format!("{tool} {} failed", args.join(" "))
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

/// Resolve the worktree root once so Git enumeration and file reads use the
/// same repository-relative coordinate system even when invoked below it.
pub fn repository_root(cwd: &Path) -> Result<PathBuf> {
    let output = run_git(&["rev-parse", "--show-toplevel"], cwd)?;
    let root = output.trim_end_matches(['\r', '\n']);
    if root.is_empty() {
        bail!("git rev-parse returned an empty repository root");
    }
    Ok(PathBuf::from(root))
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
        bail!(
            "npm pack did not return a file list. Run --packaged from the package directory (the one with package.json)"
        );
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
        ScanMode::FilesFromList => bail!("--files-from-list requires an explicit list path"),
        ScanMode::History => bail!("history uses reachable Git objects, not worktree paths"),
    }
}

/// Read a bounded, local list. Every entry stays inside the repository and uses
/// the invocation cwd as its base; reported paths always use the repository root.
pub fn files_from_list(list: &Path, cwd: &Path, repo_root: &Path) -> Result<Vec<String>> {
    const MAX_LIST_BYTES: u64 = 64 * 1024 * 1024;
    const MAX_LIST_PATHS: usize = 100_000;
    let list_path = cwd.join(list);
    let metadata = fs::symlink_metadata(&list_path)
        .map_err(|_| anyhow!("cannot read file list; check the list path and permissions"))?;
    if is_link(&metadata) || !metadata.is_file() || metadata.len() > MAX_LIST_BYTES {
        bail!("file list must be a regular UTF-8 file no larger than 64 MiB");
    }
    // Bounded read also protects against growth after the metadata check.
    use std::io::Read;
    let mut bytes = Vec::new();
    fs::File::open(&list_path)
        .map_err(|_| anyhow!("cannot open file list"))?
        .take(MAX_LIST_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| anyhow!("cannot read file list"))?;
    if bytes.len() as u64 > MAX_LIST_BYTES {
        bail!("file list exceeds 64 MiB");
    }
    let text = String::from_utf8(bytes).map_err(|_| anyhow!("file list must be valid UTF-8"))?;
    let root = repo_root
        .canonicalize()
        .context("cannot resolve repository root")?;
    // Git and Windows cwd can spell the same directory differently (short names,
    // casing, or a verbatim prefix). Match its filesystem identity, then retain
    // the lexical suffix so in-repository links and parent traversal remain visible.
    // Use the outermost matching ancestor: a nested link back to root must not
    // disappear from the suffix used for coverage checks.
    let invocation_root = cwd
        .ancestors()
        .filter(|ancestor| ancestor.canonicalize().is_ok_and(|path| path == root))
        .last()
        .ok_or_else(|| anyhow!("file list target must resolve from a repository directory"))?;
    let mut seen = HashSet::new();
    let mut paths = Vec::new();
    for line in text.strip_prefix('\u{feff}').unwrap_or(&text).lines() {
        let entry = line.trim();
        if entry.is_empty() {
            continue;
        }
        let entry = entry.replace('\\', "/");
        if entry.starts_with('/') || entry.contains(':') || entry.contains('\0') {
            bail!("file list entries must be repository-relative paths");
        }
        let candidate = cwd.join(&entry);
        let real = candidate
            .canonicalize()
            .map_err(|_| anyhow!("file list contains a missing or unresolvable target"))?;
        if !real.starts_with(&root) {
            bail!("file list target is outside the repository");
        }
        if !fs::metadata(&real)
            .map_err(|_| anyhow!("file list target is unreadable"))?
            .is_file()
        {
            bail!("file list targets must be regular files");
        }
        // Preserve link coverage reporting, but reject any link that escapes root.
        let relative = candidate
            .strip_prefix(invocation_root)
            .map_err(|_| anyhow!("file list target must resolve from a repository directory"))?;
        let mut component_path = repo_root.to_owned();
        let mut found_link = false;
        for component in relative.components() {
            component_path.push(component);
            if fs::symlink_metadata(&component_path).is_ok_and(|metadata| is_link(&metadata)) {
                found_link = true;
            }
        }
        if found_link
            && relative
                .components()
                .any(|part| part == std::path::Component::ParentDir)
        {
            bail!("file list paths cannot combine link traversal with parent components");
        }
        let mut normalized = PathBuf::new();
        for component in relative.components() {
            match component {
                std::path::Component::Normal(name) => normalized.push(name),
                std::path::Component::CurDir => {}
                std::path::Component::ParentDir => {
                    if !normalized.pop() {
                        bail!("file list path escapes the repository");
                    }
                }
                _ => bail!("invalid file list path"),
            }
        }
        let path = normalized
            .to_str()
            .ok_or_else(|| anyhow!("file list target must have a UTF-8 name"))?
            .replace('\\', "/");
        if seen.insert(path.clone()) {
            if paths.len() >= MAX_LIST_PATHS {
                bail!("file list exceeds 100000 distinct paths");
            }
            paths.push(path);
        }
    }
    Ok(paths)
}

fn is_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn directive_regex() -> &'static Regex {
    static DIRECTIVE: OnceLock<Regex> = OnceLock::new();
    DIRECTIVE.get_or_init(|| {
        // Group 1 captures any non-space characters glued directly onto `allow`
        // (e.g. `-list`, `=x`, `list` in `allowlist`) so a word like `allowlist`
        // is not mistaken for a bare allow. Group 2 captures a space-separated
        // scoped token (which may contain `-`, as in Anne-Marie); the `-->`
        // comment closer is cut off below because the regex crate has no lookahead.
        Regex::new(r"(?i)(?:doxguard|secrets-scan):\s*allow(\S*)(?:\s+([^\s>]+))?")
            .expect("static directive regex")
    })
}

pub fn allowed_by_directive(line: &str, matched: &str, config: &Config) -> bool {
    directive_regex().captures_iter(line).any(|capture| {
        // Characters glued directly to `allow` with no separating space.
        let glued = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
        if !glued.is_empty() {
            // `allow-->` (comment closer, no space) is still the bare form; every
            // other glued suffix (`allow-list`, `allow=x`, `allowlist`, ...) is not
            // an allow directive at all and must never suppress a hit.
            if glued == "-->" {
                return !config.allow.disallow_bare_allow;
            }
            return false;
        }
        let Some(target) = capture.get(2) else {
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
        token = token.trim_end_matches([
            ',', '.', ';', ':', '!', '?', '、', '。', '；', '：', '！', '？',
        ]);
        if token.is_empty() {
            // An explicit punctuation-only token must never become a bare allow.
            return false;
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
    let is_directory = exempt.ends_with('/');
    let drive_absolute = exempt
        .as_bytes()
        .get(1)
        .is_some_and(|separator| *separator == b':');
    if exempt.is_empty()
        || exempt.starts_with('/')
        || drive_absolute
        || exempt
            .trim_end_matches('/')
            .split('/')
            .any(|component| component.is_empty() || component == "." || component == "..")
    {
        return false;
    }
    let exempt = exempt.trim_end_matches('/');
    if exempt.is_empty() {
        return false;
    }
    if path == exempt {
        return true;
    }
    if is_directory && path.starts_with(&format!("{exempt}/")) {
        return true;
    }
    false
}

fn classify_path(path: &str, cwd: &Path, config: &Config, mode: ScanMode) -> Eligibility {
    if mode == ScanMode::FilesFromList {
        let mut target = cwd.to_owned();
        for component in Path::new(path).components() {
            target.push(component);
            match fs::symlink_metadata(&target) {
                Ok(metadata) if is_link(&metadata) => {
                    return Eligibility::CoverageSkip("symlink or reparse point");
                }
                Ok(_) => {}
                Err(_) => return Eligibility::CoverageSkip("missing or unreadable listed target"),
            }
        }
    }
    let normalized = normalize_path(path);
    let lower = normalized.to_ascii_lowercase();
    // Binaries are never scanned for either pattern class.
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
    // Which pattern classes apply. exemptPaths keep watchlist matching (a real
    // identity must not hide in an "exempt" file); lockfiles keep structural
    // matching (private hosts / paths / IPs) but drop noisy watchlist matching.
    let kind = if config
        .all_exempt_paths()
        .any(|exempt| path_is_exempt(&normalized, exempt))
    {
        ScanKind::WatchlistOnly
    } else if SKIP_FILENAMES.contains(&filename) {
        ScanKind::StructuralOnly
    } else {
        ScanKind::Full
    };
    // Staged mode reads index blobs; worktree size/symlink checks would false-skip.
    if mode == ScanMode::Staged {
        return Eligibility::Scan(kind);
    }
    let joined = cwd.join(path);
    let metadata = match fs::symlink_metadata(&joined) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Eligibility::QuietSkip;
        }
        Err(_) => return Eligibility::CoverageSkip("unscannable metadata"),
    };
    if is_link(&metadata) {
        return Eligibility::CoverageSkip("symlink");
    }
    if !metadata.is_file() {
        return Eligibility::QuietSkip;
    }
    if metadata.len() > config.max_file_size {
        return Eligibility::CoverageSkip("oversize");
    }
    Eligibility::Scan(kind)
}

const STAGED_BLOB_MAX_BYTES: u64 = 16 * 1024 * 1024;

/// Size-probe every staged blob with a single `git cat-file --batch-check` instead
/// of one process per file, halving the child processes a pre-commit hook spawns
/// (F-A05). Returns `None` when the batch probe is unavailable (`-Z` needs git
/// 2.42) so the caller falls back to the per-file probe. Only the *size probe* is
/// batched: blob content is still read per file, leaving the security-critical
/// read path unchanged.
fn staged_blob_sizes(paths: &[String], cwd: &Path) -> Option<HashMap<String, u64>> {
    let git = git_program().ok()?;
    let mut child = Command::new(git)
        .args([
            "-c",
            "core.quotepath=false",
            "cat-file",
            "--batch-check",
            "-Z",
        ])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let mut stdin = child.stdin.take()?;
    let specs: Vec<String> = paths.iter().map(|path| format!(":0:{path}")).collect();
    // Write from a separate thread: git streams records back while we are still
    // writing, so writing everything first would deadlock on a full pipe buffer
    // exactly in the large-staged-set case this optimizes.
    let writer = std::thread::spawn(move || {
        for spec in specs {
            if stdin.write_all(spec.as_bytes()).is_err() || stdin.write_all(b"\0").is_err() {
                return;
            }
        }
    });
    let output = child.wait_with_output().ok()?;
    let _ = writer.join();
    if !output.status.success() {
        return None;
    }
    // Records come back in input order, so zip them with the requested paths.
    let records = output
        .stdout
        .split(|byte| *byte == 0)
        .filter(|record| !record.is_empty());
    let mut sizes = HashMap::new();
    for (path, record) in paths.iter().zip(records) {
        let Ok(record) = std::str::from_utf8(record) else {
            continue;
        };
        // "<oid> <type> <size>" for a resolved blob; "<spec> missing" otherwise.
        let fields: Vec<&str> = record.split_whitespace().collect();
        if fields.len() == 3 {
            if let Ok(size) = fields[2].parse::<u64>() {
                sizes.insert(path.clone(), size);
            }
        }
    }
    Some(sizes)
}

fn read_scan_content(
    mode: ScanMode,
    path: &str,
    cwd: &Path,
    max_file_size: u64,
    staged_sizes: Option<&HashMap<String, u64>>,
) -> Result<ContentLoad> {
    if mode == ScanMode::Staged {
        // Index blob, not worktree — pre-commit must gate what will be committed.
        let spec = format!(":0:{path}");
        let limit = max_file_size.min(STAGED_BLOB_MAX_BYTES);
        // Ask for the blob size first so an oversize blob is never loaded. Prefer
        // the batched probe; fall back to a per-file one when it was unavailable.
        let probed = match staged_sizes.and_then(|sizes| sizes.get(path).copied()) {
            Some(size) => Some(size),
            None => run_git(&["cat-file", "-s", &spec], cwd)
                .ok()
                .and_then(|size| size.trim().parse::<u64>().ok()),
        };
        if probed.is_some_and(|size| size > limit) {
            return Ok(ContentLoad::CoverageSkip("oversize staged blob"));
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
        .matches_spanned_with_boundary(&haystack, config.noise.short_needle_max_length)
        .filter(|(item, _)| !allowed_by_directive(line, &item.needle, config))
        .map(|(item, span)| ScanHit {
            blob_oid: None,
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

/// Everything a single file scan needs beyond its own path and scan kind.
struct ScanContext<'a> {
    mode: ScanMode,
    cwd: &'a Path,
    config: &'a Config,
    matcher: &'a WatchlistMatcher,
    patterns: &'a [StructuralPattern],
    staged_sizes: Option<&'a HashMap<String, u64>>,
}

fn scan_file(context: &ScanContext<'_>, path: &str, kind: ScanKind) -> Result<FileScanOutcome> {
    let config = context.config;
    match read_scan_content(
        context.mode,
        path,
        context.cwd,
        config.max_file_size,
        context.staged_sizes,
    )? {
        ContentLoad::CoverageSkip(reason) => Ok(FileScanOutcome {
            hits: Vec::new(),
            coverage_skip: Some(reason),
        }),
        ContentLoad::Text(content) => {
            let mut hits = Vec::new();
            for (index, line) in content.lines().enumerate() {
                let line_number = index + 1;
                if kind.wants_watchlist() {
                    hits.extend(watchlist_hits(
                        line,
                        path,
                        line_number,
                        context.matcher,
                        config,
                    ));
                }
                if !kind.wants_structural() {
                    continue;
                }
                for pattern in context.patterns {
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
                            blob_oid: None,
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
    let mut files: Vec<(String, ScanKind)> = Vec::new();
    for path in paths {
        match classify_path(&path, cwd, config, mode) {
            Eligibility::Scan(kind) => files.push((path, kind)),
            Eligibility::QuietSkip => {}
            Eligibility::CoverageSkip(reason) => {
                coverage_skips += 1;
                warnings.push(format!("WARN: skipped {path} ({reason})"));
            }
        }
    }
    // One batched size probe for the whole staged set instead of one per file.
    let staged_sizes = if mode == ScanMode::Staged && !files.is_empty() {
        let staged: Vec<String> = files.iter().map(|(path, _)| path.clone()).collect();
        staged_blob_sizes(&staged, cwd)
    } else {
        None
    };
    let context = ScanContext {
        mode,
        cwd,
        config,
        matcher,
        patterns,
        staged_sizes: staged_sizes.as_ref(),
    };
    let outcomes: Result<Vec<FileScanOutcome>> = if files.len() <= 4 {
        files
            .iter()
            .map(|(path, kind)| scan_file(&context, path, *kind))
            .collect()
    } else {
        files
            .par_iter()
            .map(|(path, kind)| scan_file(&context, path, *kind))
            .collect()
    };
    let mut hits = Vec::new();
    let mut scanned = 0usize;
    for ((path, _kind), outcome) in files.iter().zip(outcomes?) {
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
    let baseline_applied = mode == ScanMode::Staged && config.noise.staged_added_lines_only;
    let mut baseline_suppressed_hits = 0;
    if baseline_applied {
        let mut added = HashMap::new();
        // Validate every scanned file, even when no hit happened to be found.
        for (path, _) in &files {
            added.insert(path.clone(), staged_added_lines(path, cwd)?);
        }
        hits.retain(|hit| {
            let keep = added
                .get(&hit.file)
                .is_some_and(|lines| lines.contains(&hit.line_number));
            if !keep {
                baseline_suppressed_hits += 1;
            }
            keep
        });
    }
    Ok(ScanResult {
        watchlist_statistics: matcher.statistics().clone(),
        baseline_applied,
        baseline_suppressed_hits,
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

fn staged_added_lines(path: &str, cwd: &Path) -> Result<HashSet<usize>> {
    // With no HEAD, Git treats the cached diff as a diff against the empty tree.
    let mut command = Command::new(git_program()?);
    // GIT_DIFF_OPTS can override -U0 and turn unchanged context into added ranges.
    command.env_remove("GIT_DIFF_OPTS");
    command
        .current_dir(cwd)
        .args([
            "-c",
            "core.quotepath=false",
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--no-renames",
            "--no-color",
            "--output-indicator-new=+",
            "--output-indicator-old=-",
            "--output-indicator-context= ",
            "--text",
            "--unified=0",
            "--inter-hunk-context=0",
            "--",
            path,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|_| anyhow!("cannot determine staged baseline; Git diff failed"))?;
    use std::io::Read;
    const DIFF_LIMIT: u64 = 64 * 1024 * 1024;
    let mut bytes = Vec::new();
    let read = child
        .stdout
        .take()
        .ok_or_else(|| anyhow!("missing baseline diff output"))?
        .take(DIFF_LIMIT + 1)
        .read_to_end(&mut bytes);
    if read.is_err() || bytes.len() as u64 > DIFF_LIMIT {
        let _ = child.kill();
        let _ = child.wait();
        bail!("cannot determine staged baseline; diff unreadable or exceeds 64 MiB");
    }
    if !child
        .wait()
        .map_err(|_| anyhow!("cannot wait for baseline diff"))?
        .success()
    {
        bail!("cannot determine staged baseline; Git diff failed");
    }
    let diff =
        String::from_utf8(bytes).map_err(|_| anyhow!("staged baseline diff must be UTF-8"))?;
    let hunk = Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@").expect("static hunk regex");
    let mut added = HashSet::new();
    for line in diff.lines().filter(|line| line.starts_with("@@")) {
        let captures = hunk
            .captures(line)
            .ok_or_else(|| anyhow!("cannot parse staged baseline hunk"))?;
        let start: usize = captures[1]
            .parse()
            .map_err(|_| anyhow!("invalid staged baseline position"))?;
        let count: usize = captures
            .get(2)
            .map_or(Ok(1), |value| value.as_str().parse())
            .map_err(|_| anyhow!("invalid staged baseline count"))?;
        let end = start
            .checked_add(count)
            .ok_or_else(|| anyhow!("staged baseline range overflow"))?;
        added.extend(start..end);
    }
    Ok(added)
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
