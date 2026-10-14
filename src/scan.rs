use std::{
    cell::OnceCell,
    collections::{HashMap, HashSet},
    fs,
    io::{BufRead, BufReader, Read, Write},
    ops::Range,
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
    Paths,
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
    pub variable: Option<String>,
    pub source: String,
    pub kind: HitKind,
    pub suggestion: String,
}

#[derive(Debug, Clone, Serialize)]
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

/// How a scan target's lines were delivered to the matcher.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LineRead {
    Complete,
    /// Preserve ASCII findings in non-UTF-8 data without claiming full coverage.
    Partial(&'static str),
    /// Not fully scanned; findings from the partial read are discarded.
    Skipped(&'static str),
}

enum LinesEnd {
    Complete { bytes: u64, replaced: bool },
    NonUtf8,
    OverLimit,
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
    full.push("--no-replace-objects");
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

fn nul_paths(output: String) -> Result<Vec<String>> {
    if !output.is_empty() && !output.ends_with('\0') {
        bail!("Git file list was not NUL terminated");
    }
    Ok(output
        .split('\0')
        .filter(|path| !path.is_empty())
        .map(str::to_owned)
        .collect())
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
    let root_manifest: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(cwd.join("package.json")).context("cannot read package root")?,
    )?;
    if packages.len() != 1
        || packages[0].get("name") != root_manifest.get("name")
        || packages[0].get("version") != root_manifest.get("version")
        || root_manifest.get("workspaces").is_some()
    {
        bail!(
            "workspace or multi-package scanning is unsupported; scan each package from its own root"
        );
    }
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
        ScanMode::Staged => nul_paths(run_git(
            &[
                "diff",
                "--cached",
                "--name-only",
                "-z",
                "--diff-filter=ACMRT",
            ],
            cwd,
        )?),
        ScanMode::Diff => nul_paths(run_git(
            &["diff", "--name-only", "-z", "--diff-filter=ACMRT", "HEAD"],
            cwd,
        )?),
        ScanMode::AllTracked => nul_paths(run_git(&["ls-files", "-z"], cwd)?),
        ScanMode::Packaged => packaged_files(cwd),
        ScanMode::FilesFromList => bail!("--files-from-list requires an explicit list path"),
        ScanMode::History => bail!("history uses reachable Git objects"),
        ScanMode::Paths => bail!("path mode requires --path or --path-from"),
    }
}

fn is_link(metadata: &fs::Metadata) -> bool {
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        // Junctions are reparse points too; never recursively follow them.
        metadata.file_attributes() & 0x400 != 0
    }
    #[cfg(not(windows))]
    {
        metadata.file_type().is_symlink()
    }
}

/// One file to scan. `path` is what every file-system check and exemption uses;
/// `display` is the name reports show. They differ only for `--path` targets.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanTarget {
    pub path: String,
    pub display: String,
}

impl From<String> for ScanTarget {
    fn from(path: String) -> Self {
        Self {
            display: path.clone(),
            path,
        }
    }
}

/// Expand literal paths outside git. Explicit missing paths and enumeration
/// failures are errors. Links are retained as coverage skips, never followed.
/// Each target is checked at the cwd-joined path, but reported as the user spelled
/// it plus the part found by walking a directory, so a report never adds the cwd
/// (often the home directory) that the command did not name.
pub fn files_from_paths(roots: &[PathBuf], cwd: &Path, config: &Config) -> Result<Vec<ScanTarget>> {
    fn visit(
        path: &Path,
        display: &Path,
        cwd: &Path,
        config: &Config,
        files: &mut HashMap<PathBuf, PathBuf>,
    ) -> Result<()> {
        let native = path.to_str().context("scan path was not UTF-8")?;
        let meta = fs::symlink_metadata(path).context("cannot inspect requested scan path")?;
        if configured_path_exempt(native, cwd, config) {
            return Ok(());
        }
        if meta.is_dir() && !is_link(&meta) {
            for entry in fs::read_dir(path).context("cannot enumerate scan directory")? {
                let entry = entry.context("cannot read scan directory entry")?;
                visit(
                    &entry.path(),
                    &display.join(entry.file_name()),
                    cwd,
                    config,
                    files,
                )?;
            }
        } else {
            // The same file named twice keeps the first spelling.
            files
                .entry(path.to_path_buf())
                .or_insert_with(|| display.to_path_buf());
        }
        Ok(())
    }
    if roots.is_empty() {
        bail!("path scan requires at least one non-empty path");
    }
    let mut files = HashMap::new();
    for root in roots {
        if root.as_os_str().is_empty() {
            bail!("scan paths must not be empty");
        }
        // Keep native path semantics: collapsing a symlink/.. changes its target.
        let path = cwd.join(root);
        visit(&path, root, cwd, config, &mut files)?;
    }
    let utf8 = |path: PathBuf| {
        path.into_os_string()
            .into_string()
            .map_err(|_| anyhow!("scan path was not UTF-8"))
    };
    let mut files: Vec<_> = files
        .into_iter()
        .map(|(path, display)| {
            Ok(ScanTarget {
                path: utf8(path)?,
                display: utf8(display)?,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    files.sort_by(|left, right| left.path.cmp(&right.path));
    Ok(files)
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
        let entry = line;
        if entry.trim().is_empty() {
            continue;
        }
        let entry = native_exemption_path(entry);
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
            .to_owned();
        let path = native_exemption_path(&path);
        if seen.insert(path.clone()) {
            if paths.len() >= MAX_LIST_PATHS {
                bail!("file list exceeds 100000 distinct paths");
            }
            paths.push(path);
        }
    }
    Ok(paths)
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
    LineDirectives::new(line, config).allows(matched)
}

/// What the inline allow directives on one line permit. They are read once,
/// when the first match on the line asks, and every other match is answered
/// from that, so a long line with many matches stays linear.
struct LineDirectives<'a> {
    line: &'a str,
    config: &'a Config,
    parsed: OnceCell<ParsedDirectives>,
}

#[derive(Default)]
struct ParsedDirectives {
    /// A bare `doxguard: allow` the config honours: every match is allowed.
    bare: bool,
    /// Lowercased scoped tokens: a match that contains one is allowed.
    tokens: Vec<String>,
}

impl<'a> LineDirectives<'a> {
    fn new(line: &'a str, config: &'a Config) -> Self {
        Self {
            line,
            config,
            parsed: OnceCell::new(),
        }
    }

    /// Whether a directive on the line allows `matched`.
    fn allows(&self, matched: &str) -> bool {
        let parsed = self.parsed.get_or_init(|| self.parse());
        if parsed.bare {
            return true;
        }
        if parsed.tokens.is_empty() {
            return false;
        }
        let matched = matched.to_ascii_lowercase();
        // Scoped allow: token must appear in the match (not reverse substring).
        parsed
            .tokens
            .iter()
            .any(|token| matched.contains(token.as_str()))
    }

    /// Everything about a directive except whether its token is in the match
    /// depends only on the line and the config, so it is worked out once.
    fn parse(&self) -> ParsedDirectives {
        let mut parsed = ParsedDirectives::default();
        if self.config.allow.disallow_inline_allow {
            return parsed;
        }
        for capture in directive_regex().captures_iter(self.line) {
            match directive(self.line, &capture) {
                // Bare `doxguard: allow` — optional hard-off for CI / --strict.
                Directive::Bare => parsed.bare |= !self.config.allow.disallow_bare_allow,
                Directive::Scoped(token) => parsed.tokens.push(token),
                Directive::Inert => {}
            }
        }
        parsed
    }
}

/// What one directive permits, before any match is looked at.
enum Directive {
    /// The bare form: every match on the line, unless the config refuses it.
    Bare,
    /// A scoped token, lowercased: the matches that contain it.
    Scoped(String),
    /// Not an allow directive, or a token that must not allow anything.
    Inert,
}

fn directive(line: &str, capture: &regex::Captures<'_>) -> Directive {
    // Characters glued directly to `allow` with no separating space.
    let glued = capture.get(1).map(|m| m.as_str()).unwrap_or_default();
    if !glued.is_empty() {
        // `allow-->` (comment closer, no space) is still the bare form; every
        // other glued suffix (`allow-list`, `allow=x`, `allowlist`, ...) is not
        // an allow directive at all and must never suppress a hit.
        if glued == "-->" {
            return Directive::Bare;
        }
        return Directive::Inert;
    }
    let Some(target) = capture.get(2) else {
        return Directive::Bare;
    };
    let mut token = target.as_str();
    // `allow token-->` captures `token--`; drop the HTML comment closer.
    if line[target.end()..].starts_with('>') {
        token = token.strip_suffix("--").unwrap_or(token);
    }
    if token.is_empty() {
        // `<!-- doxguard: allow -->` is the bare form, not a token.
        return Directive::Bare;
    }
    token = token.trim_end_matches([
        ',', '.', ';', ':', '!', '?', '、', '。', '；', '：', '！', '？',
    ]);
    if token.is_empty() {
        // An explicit punctuation-only token must never become a bare allow.
        return Directive::Inert;
    }
    let target = token.to_ascii_lowercase();
    // Short tokens (e.g. "168", ".") over-suppress structural hits.
    if target.chars().count() < 4 {
        return Directive::Inert;
    }
    Directive::Scoped(target)
}

fn native_exemption_path(path: &str) -> String {
    // Only Windows treats both slashes as separators. On Unix a backslash is
    // part of a filename, including during exemption and lockfile matching.
    #[cfg(windows)]
    {
        path.replace('\\', "/")
    }
    #[cfg(not(windows))]
    {
        path.to_owned()
    }
}

/// Path exemption uses path-boundary matching, not raw substring contains.
/// Empty / blank exempt entries never match (and are rejected at config load).
pub fn path_is_exempt(path: &str, exempt: &str) -> bool {
    let path = native_exemption_path(path);
    let exempt = native_exemption_path(exempt.trim());
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
    if mode == ScanMode::Paths && configured_path_exempt(path, cwd, config) {
        return Eligibility::QuietSkip;
    }
    if mode != ScanMode::Paths
        && config.include_default_exempt_paths
        && config
            .all_exempt_paths()
            .any(|exempt| path_is_exempt(path, exempt))
    {
        return Eligibility::QuietSkip;
    }
    if mode == ScanMode::Paths {
        match fs::symlink_metadata(cwd.join(path)) {
            Ok(meta) if !is_link(&meta) && meta.is_file() => {}
            _ => {
                return Eligibility::CoverageSkip("missing, unreadable or non-regular scan target");
            }
        }
    }
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
    let normalized = native_exemption_path(path);
    let lower = normalized.to_ascii_lowercase();
    // Binaries are never scanned for either pattern class.

    if BINARY_EXTENSIONS
        .iter()
        .any(|extension| lower.ends_with(extension))
    {
        return Eligibility::QuietSkip;
    }
    let filename = Path::new(path)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if mode != ScanMode::Paths
        && config.include_default_exempt_paths
        && SKIP_FILENAMES.contains(&filename)
    {
        return Eligibility::QuietSkip;
    }
    // Which pattern classes apply. exemptPaths keep watchlist matching (a real
    // identity must not hide in an "exempt" file); lockfiles keep structural
    // matching (private hosts / paths / IPs) but drop noisy watchlist matching.
    let kind = if mode == ScanMode::Paths {
        ScanKind::Full
    } else if config
        .all_exempt_paths()
        .any(|exempt| path_is_exempt(&normalized, exempt))
    {
        ScanKind::WatchlistOnly
    } else if SKIP_FILENAMES.contains(&filename) && !config.require_watchlists {
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
        Err(_) => return Eligibility::CoverageSkip("missing or unreadable metadata"),
    };
    if is_link(&metadata) {
        return Eligibility::CoverageSkip("symlink or reparse point");
    }
    if !metadata.is_file() {
        return Eligibility::CoverageSkip("non-regular target");
    }
    if config
        .max_file_size
        .is_some_and(|limit| metadata.len() > limit)
    {
        return Eligibility::CoverageSkip("oversize");
    }
    Eligibility::Scan(kind)
}

fn configured_path_exempt(path: &str, cwd: &Path, config: &Config) -> bool {
    let relative = Path::new(path)
        .strip_prefix(cwd)
        .ok()
        .and_then(Path::to_str);
    config.exempt_paths.iter().any(|exempt| {
        path_is_exempt(path, exempt)
            || relative.is_some_and(|relative| path_is_exempt(relative, exempt))
    })
}

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
            "--no-replace-objects",
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

/// Feed `reader` to `on_line` one line at a time, split exactly like
/// `str::lines`, so memory is bounded by the longest line instead of the file.
fn for_each_line(
    mut reader: impl BufRead,
    lossy: bool,
    limit: Option<u64>,
    on_line: &mut dyn FnMut(usize, &str),
) -> std::io::Result<LinesEnd> {
    let mut buffer = Vec::new();
    let mut bytes = 0u64;
    let mut replaced = false;
    let mut line_number = 0usize;
    loop {
        buffer.clear();
        let read = reader.read_until(b'\n', &mut buffer)?;
        if read == 0 {
            return Ok(LinesEnd::Complete { bytes, replaced });
        }
        bytes += read as u64;
        if limit.is_some_and(|limit| bytes > limit) {
            return Ok(LinesEnd::OverLimit);
        }
        line_number += 1;
        let mut line = buffer.as_slice();
        if let Some(rest) = line.strip_suffix(b"\n") {
            line = rest.strip_suffix(b"\r").unwrap_or(rest);
        }
        match std::str::from_utf8(line) {
            Ok(text) => on_line(line_number, text),
            Err(_) if lossy => {
                replaced = true;
                on_line(line_number, &String::from_utf8_lossy(line));
            }
            Err(_) => return Ok(LinesEnd::NonUtf8),
        }
    }
}

/// `Read::take` bound that stops a line without newlines from growing past
/// an explicit `maxFileSize`; unlimited when none is configured.
fn read_cap(limit: Option<u64>) -> u64 {
    limit.map_or(u64::MAX, |limit| limit.saturating_add(1))
}

fn read_scan_lines(
    mode: ScanMode,
    path: &str,
    cwd: &Path,
    max_file_size: Option<u64>,
    _staged_sizes: Option<&HashMap<String, u64>>,
    on_line: &mut dyn FnMut(usize, &str),
) -> LineRead {
    if mode == ScanMode::Staged {
        // Index blob, not worktree — pre-commit must gate what will be committed.
        return read_staged_lines(path, cwd, max_file_size, on_line);
    }
    let file = match fs::File::open(cwd.join(path)) {
        Ok(file) => file,
        Err(_) => return LineRead::Skipped("unreadable content"),
    };
    let reader = BufReader::new(file.take(read_cap(max_file_size)));
    match for_each_line(reader, mode == ScanMode::Paths, max_file_size, on_line) {
        Ok(LinesEnd::Complete {
            replaced: false, ..
        }) => LineRead::Complete,
        Ok(LinesEnd::Complete { replaced: true, .. }) => LineRead::Partial("non-UTF-8 content"),
        Ok(LinesEnd::OverLimit) => LineRead::Skipped("oversize content"),
        Ok(LinesEnd::NonUtf8) => LineRead::Skipped("non-UTF-8 content"),
        Err(_) => LineRead::Skipped("unreadable content"),
    }
}

fn read_staged_lines(
    path: &str,
    cwd: &Path,
    max_file_size: Option<u64>,
    on_line: &mut dyn FnMut(usize, &str),
) -> LineRead {
    const UNREADABLE: &str = "unreadable or non-UTF-8 staged blob";
    let literal = format!(":(literal){path}");
    let listing = match run_git(&["ls-files", "--stage", "-z", "--", &literal], cwd) {
        Ok(listing) => listing,
        Err(_) => return LineRead::Skipped("unreadable staged entry"),
    };
    let mut entries = listing.split('\0').filter(|entry| !entry.is_empty());
    let entry = entries.next().unwrap_or_default();
    let Some((header, entry_path)) = entry.split_once('\t') else {
        return LineRead::Skipped("missing staged entry");
    };
    let fields: Vec<_> = header.split_whitespace().collect();
    if entries.next().is_some()
        || entry_path != path
        || fields.len() != 3
        || !matches!(fields[0], "100644" | "100755")
        || fields[2] != "0"
        || !matches!(fields[1].len(), 40 | 64)
        || !fields[1].bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return LineRead::Skipped("non-regular or unmerged staged entry");
    }
    // Bind type, size and contents to the same immutable index object.
    let oid = fields[1];
    let Some(size) = run_git(&["cat-file", "-s", oid], cwd)
        .ok()
        .and_then(|size| size.trim().parse::<u64>().ok())
    else {
        return LineRead::Skipped("unreadable staged blob");
    };
    if max_file_size.is_some_and(|limit| size > limit) {
        return LineRead::Skipped("oversize staged blob");
    }
    let Ok(git) = git_program() else {
        return LineRead::Skipped(UNREADABLE);
    };
    let spawned = Command::new(git)
        .args([
            "--no-replace-objects",
            "-c",
            "core.quotepath=false",
            "cat-file",
            "blob",
            oid,
        ])
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut child) = spawned else {
        return LineRead::Skipped(UNREADABLE);
    };
    let ended = child.stdout.take().map(|stdout| {
        // Reading one byte past the probed size detects any mismatch.
        let reader = BufReader::new(stdout.take(size.saturating_add(1)));
        for_each_line(reader, false, None, on_line)
    });
    if !ended
        .as_ref()
        .is_some_and(|ended| staged_read_complete(ended, size, true))
    {
        // Stop a blob that is no longer being read instead of draining it.
        let _ = child.kill();
    }
    let exited = child.wait().is_ok_and(|status| status.success());
    if ended
        .as_ref()
        .is_some_and(|ended| staged_read_complete(ended, size, exited))
    {
        LineRead::Complete
    } else {
        LineRead::Skipped(UNREADABLE)
    }
}

/// A staged blob counts as scanned only when exactly the probed number of
/// bytes arrived as valid lines and `git cat-file` itself succeeded.
fn staged_read_complete(ended: &std::io::Result<LinesEnd>, size: u64, exited: bool) -> bool {
    exited && matches!(ended, Ok(LinesEnd::Complete { bytes, .. }) if *bytes == size)
}

/// The key-name pattern. `LineKeys` relies on it having no assertion that looks
/// past the end of a match (`$`, `\b`), so a match inside a prefix of a line is
/// also a match of the whole line, and on every match ending in `[:=]\s*`.
/// The test oracle repeats this pattern.
fn key_regex() -> &'static Regex {
    static KEY: OnceLock<Regex> = OnceLock::new();
    KEY.get_or_init(|| Regex::new(
        r#"(?:^|[\s{,;])(?:\$?(?:env:|script:|global:)?)?["']?([A-Za-z_][A-Za-z0-9_.-]*)["']?\s*[:=]\s*"#
    ).expect("static key regex"))
}

/// Key-regex matches as (whole match, name) byte ranges.
type KeySpans = (Range<usize>, Range<usize>);

fn key_spans(found: &regex::Captures<'_>) -> KeySpans {
    let name = found.get(1).expect("the key name group is not optional");
    (found.get_match().range(), name.range())
}

/// The whitespace run at the start of a haystack, with the key regex's `\s`.
fn whitespace_run() -> &'static Regex {
    static RUN: OnceLock<Regex> = OnceLock::new();
    RUN.get_or_init(|| Regex::new(r"\A\s*").expect("static whitespace regex"))
}

/// Variable names for the hits on one line. The key regex runs over the whole
/// line once, on the first hit, instead of over a prefix for every hit, and
/// the other per-line facts a hit needs are also recorded once, on first use.
struct LineKeys<'a> {
    line: &'a str,
    matches: OnceCell<Vec<KeySpans>>,
    /// Byte offsets of `;` `,` `{` `}`, which part a value from its key.
    delimiters: OnceCell<Vec<usize>>,
    /// Byte offsets of `:` and `=`, each with the end of the whitespace run
    /// right after it.
    assigns: OnceCell<Vec<(usize, usize)>>,
}

impl<'a> LineKeys<'a> {
    fn new(line: &'a str) -> Self {
        Self {
            line,
            matches: OnceCell::new(),
            delimiters: OnceCell::new(),
            assigns: OnceCell::new(),
        }
    }

    /// Same result as taking the last key-regex match in `&line[..start]`.
    fn variable_at(&self, start: usize) -> Option<String> {
        let matches = self.matches.get_or_init(|| {
            key_regex()
                .captures_iter(self.line)
                .map(|found| key_spans(&found))
                .collect()
        });
        // Whole-line matches that end by `start` are exactly the first ones a
        // scan of the prefix finds.
        let ended = matches.partition_point(|(whole, _)| whole.end <= start);
        let mut last = ended.checked_sub(1).map(|index| matches[index].clone());
        // A match running across `start` can hide shorter ones that only the
        // cut prefix allows; those start inside it, so rescan just that stretch.
        let crossing = matches.get(ended).filter(|(whole, _)| whole.start < start);
        if let Some(found) = crossing.and_then(|(whole, _)| self.rescan(whole.start, start)) {
            last = Some(found);
        }
        let (whole, name) = last?;
        // A key is useful only when the match is in its value; reject prose/URLs.
        if self.has_delimiter(whole.end..start) || self.line[whole.end..start].starts_with("//") {
            return None;
        }
        Some(self.line[name].to_owned())
    }

    /// The last match that a scan of `&line[..start]` finds when it resumes at
    /// `from`, where a whole-line match that runs across `start` begins.
    fn rescan(&self, from: usize, start: usize) -> Option<KeySpans> {
        let assigns = self.assigns.get_or_init(|| {
            self.line
                .bytes()
                .enumerate()
                .filter(|&(_, byte)| matches!(byte, b':' | b'='))
                .map(|(offset, _)| {
                    // Runs after distinct `:` and `=` never overlap, so this
                    // reads the line about once.
                    let run = whitespace_run().find(&self.line[offset + 1..]);
                    (offset, offset + 1 + run.map_or(0, |run| run.end()))
                })
                .collect()
        });
        // Every key match ends in `[:=]\s*`, so one that starts at `from` or
        // later needs a `:` or `=` between `from` and `start`.
        let before = assigns.partition_point(|&(offset, _)| offset < start);
        let (last_assign, run_end) = before
            .checked_sub(1)
            .map(|index| assigns[index])
            .filter(|&(offset, _)| offset >= from)?;
        // No such match runs past the whitespace after the last of them, and
        // the regex has no assertion past a match, so cutting the prefix there
        // keeps exactly the same matches. The search then never has to read
        // the rest of a long key name only to rule it out.
        let prefix = &self.line[..run_end.min(start)];
        let mut found = None;
        let mut at = from;
        // A match found from `at` on has its `:` or `=` after `at`; once past
        // the last one, the search would find nothing more.
        while at <= last_assign {
            let Some(next) = key_regex().captures_at(prefix, at) else {
                break;
            };
            let spans = key_spans(&next);
            at = spans.0.end;
            found = Some(spans);
        }
        found
    }

    /// Whether `;` `,` `{` or `}` lies in `range`. They are ASCII, so finding
    /// their bytes is the same as finding the characters.
    fn has_delimiter(&self, range: Range<usize>) -> bool {
        let delimiters = self.delimiters.get_or_init(|| {
            self.line
                .bytes()
                .enumerate()
                .filter(|&(_, byte)| matches!(byte, b';' | b',' | b'{' | b'}'))
                .map(|(offset, _)| offset)
                .collect()
        });
        let first = delimiters.partition_point(|&offset| offset < range.start);
        delimiters
            .get(first)
            .is_some_and(|&offset| offset < range.end)
    }
}

fn watchlist_hits(
    line: &str,
    keys: &LineKeys<'_>,
    directives: &LineDirectives<'_>,
    file: &str,
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
        .filter(|(item, _)| !directives.allows(&item.needle))
        .map(|(item, span)| ScanHit {
            blob_oid: None,
            file: file.to_owned(),
            line_number,
            // Slice the original line so the report keeps its casing even when
            // the haystack was lowercased (ASCII lowering preserves offsets).
            variable: keys.variable_at(span.start),
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

fn scan_file(
    context: &ScanContext<'_>,
    target: &ScanTarget,
    kind: ScanKind,
) -> Result<FileScanOutcome> {
    let (path, display) = (target.path.as_str(), target.display.as_str());
    let config = context.config;
    let matcher = context.matcher;
    let patterns = context.patterns;
    let mut hits = Vec::new();
    let mut on_line = |line_number: usize, line: &str| {
        let directives = LineDirectives::new(line, config);
        let keys = LineKeys::new(line);
        if kind.wants_watchlist() {
            hits.extend(watchlist_hits(
                line,
                &keys,
                &directives,
                display,
                line_number,
                matcher,
                config,
            ));
        }
        if !kind.wants_structural() {
            return;
        }
        for pattern in patterns {
            let mut seen = HashSet::new();
            for found in pattern.regex.find_iter(line) {
                let matched = found.as_str();
                if !pattern.valid_span(line, found.range())
                    || pattern.is_allowed(matched, config)
                    || directives.allows(matched)
                    || !seen.insert(matched)
                {
                    continue;
                }
                hits.push(ScanHit {
                    blob_oid: None,
                    file: display.to_owned(),
                    line_number,
                    matched: matched.to_owned(),
                    variable: keys.variable_at(found.start()),
                    source: format!("structural: {}", pattern.name),
                    kind: HitKind::Structural,
                    suggestion: pattern.suggestion.clone(),
                });
            }
        }
    };
    let read = read_scan_lines(
        context.mode,
        path,
        context.cwd,
        config.max_file_size,
        context.staged_sizes,
        &mut on_line,
    );
    Ok(match read {
        LineRead::Complete => FileScanOutcome {
            hits,
            coverage_skip: None,
        },
        LineRead::Partial(reason) => FileScanOutcome {
            hits,
            coverage_skip: Some(reason),
        },
        LineRead::Skipped(reason) => FileScanOutcome {
            hits: Vec::new(),
            coverage_skip: Some(reason),
        },
    })
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
    scan_targets(
        mode,
        paths.into_iter().map(ScanTarget::from).collect(),
        cwd,
        config,
        matcher,
        patterns,
        warnings,
    )
}

/// Like `scan_paths`, but findings and skip warnings name each target by its
/// `display` path.
pub fn scan_targets(
    mode: ScanMode,
    targets: Vec<ScanTarget>,
    cwd: &Path,
    config: &Config,
    matcher: &WatchlistMatcher,
    patterns: &[StructuralPattern],
    warnings: Vec<String>,
) -> Result<ScanResult> {
    let total_files = targets.len();
    let mut warnings = warnings;
    let mut coverage_skips = 0usize;
    let mut files: Vec<(ScanTarget, ScanKind)> = Vec::new();
    for target in targets {
        match classify_path(&target.path, cwd, config, mode) {
            Eligibility::Scan(kind) => files.push((target, kind)),
            Eligibility::QuietSkip => {}
            Eligibility::CoverageSkip(reason) => {
                coverage_skips += 1;

                warnings.push(format!("WARN: skipped {} ({reason})", target.display));
            }
        }
    }
    // One batched size probe for the whole staged set instead of one per file.
    let staged_sizes = if mode == ScanMode::Staged && !files.is_empty() {
        let staged: Vec<String> = files
            .iter()
            .map(|(target, _)| target.path.clone())
            .collect();
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
            .map(|(target, kind)| scan_file(&context, target, *kind))
            .collect()
    } else {
        files
            .par_iter()
            .map(|(target, kind)| scan_file(&context, target, *kind))
            .collect()
    };
    let mut hits = Vec::new();
    let mut scanned = 0usize;
    for ((target, _kind), outcome) in files.iter().zip(outcomes?) {
        if let Some(reason) = outcome.coverage_skip {
            coverage_skips += 1;
            warnings.push(format!("WARN: skipped {} ({reason})", target.display));
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
        for (target, _) in &files {
            added.insert(
                target.display.clone(),
                staged_added_lines(&target.path, cwd)?,
            );
        }
        hits.retain(|hit| {
            let keep = added
                .get(&hit.file)
                .is_some_and(|lines| lines.contains(hit.line_number));
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

/// New-side line ranges of a staged diff, merged and sorted, so a rewrite of a
/// huge file costs one range instead of one entry per line.
struct AddedLines(Vec<Range<usize>>);

impl AddedLines {
    fn new(mut ranges: Vec<Range<usize>>) -> Self {
        ranges.retain(|range| !range.is_empty());
        ranges.sort_by_key(|range| range.start);
        let mut merged: Vec<Range<usize>> = Vec::with_capacity(ranges.len());
        for range in ranges {
            match merged.last_mut() {
                Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
                _ => merged.push(range),
            }
        }
        Self(merged)
    }

    fn contains(&self, line: usize) -> bool {
        let index = self.0.partition_point(|range| range.end <= line);
        self.0.get(index).is_some_and(|range| range.contains(&line))
    }
}

fn staged_added_lines(path: &str, cwd: &Path) -> Result<AddedLines> {
    // With no HEAD, Git treats the cached diff as a diff against the empty tree.
    let mut command = Command::new(git_program()?);
    let literal = format!(":(literal){path}");
    // GIT_DIFF_OPTS can override -U0 and turn unchanged context into added ranges.
    command.env_remove("GIT_DIFF_OPTS");
    command
        .current_dir(cwd)
        .args([
            "--no-replace-objects",
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
            &literal,
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    let mut child = command
        .spawn()
        .map_err(|_| anyhow!("cannot determine staged baseline; Git diff failed"))?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill();
        let _ = child.wait();
        bail!("missing baseline diff output");
    };
    // Only hunk headers matter, so stream the diff line by line: its size
    // follows the staged file and must not cap what the gate can check.
    let hunk = Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@").expect("static hunk regex");
    let mut reader = BufReader::new(stdout);
    let mut buffer = Vec::new();
    let mut ranges = Vec::new();
    let parsed = loop {
        buffer.clear();
        match reader.read_until(b'\n', &mut buffer) {
            Ok(0) => break Ok(()),
            Ok(_) => {}
            Err(_) => break Err(anyhow!("cannot determine staged baseline; diff unreadable")),
        }
        let Ok(line) = std::str::from_utf8(&buffer) else {
            break Err(anyhow!("staged baseline diff must be UTF-8"));
        };
        if !line.starts_with("@@") {
            continue;
        }
        match hunk_range(&hunk, line) {
            Ok(range) => ranges.push(range),
            Err(error) => break Err(error),
        }
    };
    if parsed.is_err() {
        let _ = child.kill();
    }
    let status = child.wait();
    parsed?;
    if !status
        .map_err(|_| anyhow!("cannot wait for baseline diff"))?
        .success()
    {
        bail!("cannot determine staged baseline; Git diff failed");
    }
    Ok(AddedLines::new(ranges))
}

fn hunk_range(hunk: &Regex, line: &str) -> Result<Range<usize>> {
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
    Ok(start..end)
}

#[cfg(test)]
mod tests {
    use super::{AddedLines, LinesEnd, nul_paths, staged_read_complete};

    #[test]
    fn staged_reads_count_only_when_every_byte_arrived_and_git_succeeded() {
        let complete = |bytes| {
            Ok(LinesEnd::Complete {
                bytes,
                replaced: false,
            })
        };
        assert!(staged_read_complete(&complete(10), 10, true));
        assert!(!staged_read_complete(&complete(9), 10, true), "truncated");
        assert!(
            !staged_read_complete(&complete(11), 10, true),
            "extra bytes"
        );
        assert!(
            !staged_read_complete(&complete(10), 10, false),
            "git failed"
        );
        assert!(!staged_read_complete(&Ok(LinesEnd::NonUtf8), 10, true));
        assert!(!staged_read_complete(&Ok(LinesEnd::OverLimit), 10, true));
        assert!(!staged_read_complete(
            &Err(std::io::Error::other("synthetic")),
            10,
            true
        ));
    }

    #[test]
    fn added_lines_merge_unsorted_overlapping_and_empty_ranges() {
        let added = AddedLines::new(vec![5..7, 1..3, 2..4, 10..10]);
        assert_eq!(added.0, [1..4, 5..7]);
        for line in [1, 2, 3, 5, 6] {
            assert!(added.contains(line), "{line}");
        }
        for line in [0, 4, 7, 10] {
            assert!(!added.contains(line), "{line}");
        }
    }

    #[test]
    fn nul_paths_preserves_whitespace_and_newlines() {
        assert_eq!(
            nul_paths(" leading.txt\0trailing.txt \0line\nbreak.txt\0".to_owned()).unwrap(),
            [" leading.txt", "trailing.txt ", "line\nbreak.txt"]
        );
    }
}

#[cfg(test)]
mod line_key_tests {
    use std::{fs, sync::OnceLock};

    use regex::Regex;

    use super::{LineKeys, ScanMode, key_regex, scan_paths};
    use crate::{
        config::Config,
        patterns,
        watchlist::{WatchlistItem, WatchlistMatcher},
    };

    /// The per-hit lookup that `LineKeys` replaced, kept verbatim as the oracle.
    fn variable_at_reference(line: &str, start: usize) -> Option<String> {
        static KEY: OnceLock<Regex> = OnceLock::new();
        let regex = KEY.get_or_init(|| Regex::new(
            r#"(?:^|[\s{,;])(?:\$?(?:env:|script:|global:)?)?["']?([A-Za-z_][A-Za-z0-9_.-]*)["']?\s*[:=]\s*"#
        ).expect("static key regex"));
        let capture = regex.captures_iter(&line[..start]).last()?;
        let after = &line[capture.get(0)?.end()..start];
        // A key is useful only when the match is in its value; reject prose/URLs.
        if after.contains([';', ',', '{', '}']) || after.starts_with("//") {
            return None;
        }
        Some(capture.get(1)?.as_str().to_owned())
    }

    /// Compares every char boundary of `line` with the oracle and counts the
    /// boundaries that sit inside a whole-line match whose cut prefix still
    /// matches, i.e. where the rescan decides the answer.
    fn assert_same_as_reference(line: &str) -> usize {
        let keys = LineKeys::new(line);
        let mut rescued = 0;
        for start in (0..=line.len()).filter(|&start| line.is_char_boundary(start)) {
            assert_eq!(
                keys.variable_at(start),
                variable_at_reference(line, start),
                "{line:?} at byte {start}"
            );
            let crossing = keys
                .matches
                .get()
                .and_then(|matches| matches.iter().find(|(whole, _)| whole.contains(&start)))
                .filter(|(whole, _)| whole.start < start);
            if crossing.is_some_and(|(whole, _)| {
                key_regex()
                    .captures_at(&line[..start], whole.start)
                    .is_some()
            }) {
                rescued += 1;
            }
        }
        rescued
    }

    #[test]
    fn line_keys_match_the_prefix_scan_on_synthetic_lines() {
        let lines = [
            "",
            " \t ",
            "ip = 192.0.2.3",
            "ip=192.0.2.3",
            r#""ip": "192.0.2.3""#,
            r#"{"ip": "192.0.2.3", "next": "192.0.2.4"}"#,
            "'host_a': '192.0.2.3' \"host_b\" = \"192.0.2.4\"",
            "first = 192.0.2.3 second: 192.0.2.4 third=192.0.2.5",
            "a = 1; b = 192.0.2.3, c: 192.0.2.4 {d=192.0.2.5} e = 192.0.2.6",
            "url = https://example.test/path; next = 192.0.2.3",
            "link: //example.test/x and 192.0.2.3",
            "path = a//b/c",
            "$env:FIXTURE_HOST = 192.0.2.3; $script:Other = x; $global:third=y",
            "$env:=192.0.2.3 $Plain=192.0.2.4 env:k=v script:=w",
            " env:k=v",
            "x=1 env:y=2 global:z = 3",
            "192.0.2.3 starts the line",
            "key=192.0.2.3 starts too",
            " foo192.0.2.3=x",
            " a-192.0.2.3=x b-192.0.2.4 = y",
            "a=b=c",
            "a: b: c: d",
            "k: v k2 = w",
            "{a:{b:c}}",
            "key   =    value     ",
            "  name-with.dots_and-dash :  value",
            "word word word key = value",
            "名前 = 値; キー: 192.0.2.3 name＝全角 k\u{3000}=\u{3000}v",
            "émoji = 🙂 then k=192.0.2.3",
        ];
        for line in lines {
            assert_same_as_reference(line);
        }
    }

    #[test]
    fn line_keys_match_the_prefix_scan_on_generated_lines() {
        const TOKENS: [&str; 26] = [
            "a", "b", "_", ".", "-", "1", "0", ":", "=", ";", ",", "{", "}", "\"", "'", "$", "/",
            " ", " ", "\t", "env:", "script:", "global:", "é", "\u{3000}", "k=",
        ];
        let mut state: u64 = 0x2545_F491_4F6C_DD1D;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut rescued = 0;
        for _ in 0..4000 {
            let length = (next() % 32) as usize;
            let line: String = (0..length)
                .map(|_| TOKENS[(next() % TOKENS.len() as u64) as usize])
                .collect();
            rescued += assert_same_as_reference(&line);
        }
        // Make sure the sweep reached the rescan of a crossing match.
        assert!(rescued > 50, "only {rescued} rescued boundaries");
    }

    #[test]
    fn many_distinct_hits_on_one_line_keep_the_reference_variables() {
        let mut line = String::new();
        let mut planted = Vec::new();
        let mut needles = Vec::new();
        for index in 0..600 {
            let ip = format!(
                "10.{}.{}.{}",
                100 + index / 100,
                100 + index / 10 % 10,
                100 + index % 10
            );
            let needle = format!("fixture-needle-{index:04}");
            let entry = match index % 8 {
                0 => format!("k{index} = {ip}; "),
                1 => format!("\"q{index}\": \"{ip}\", "),
                2 => format!("{ip} "),
                3 => format!("a-{ip}=v "),
                4 => format!("$env:E{index}={needle} "),
                5 => format!("{needle}: {ip} "),
                // The key match crosses the hit; only the prefix yields `env`.
                6 => format!("$env:{needle}=v "),
                _ => format!("値 k{index}\u{3000}=\u{3000}{needle} {{{ip}}} "),
            };
            if entry.contains(&ip) {
                planted.push(ip);
            }
            if entry.contains(&needle) {
                planted.push(needle.clone());
                needles.push(WatchlistItem {
                    needle,
                    source: "fixture".to_owned(),
                });
            }
            line.push_str(&entry);
        }
        let temp = tempfile::tempdir().unwrap();
        fs::write(temp.path().join("many.txt"), format!("{line}\n")).unwrap();
        let config = Config::default();
        let result = scan_paths(
            ScanMode::Paths,
            vec!["many.txt".to_owned()],
            temp.path(),
            &config,
            &WatchlistMatcher::new(needles).unwrap(),
            &patterns::build(&config).unwrap(),
            Vec::new(),
        )
        .unwrap();

        for value in &planted {
            assert!(
                result.hits.iter().any(|hit| &hit.matched == value),
                "{value} was not reported"
            );
        }
        let mut named = 0;
        for hit in &result.hits {
            // Every planted value occurs once and none contains another.
            assert!(planted.contains(&hit.matched), "{}", hit.matched);
            let start = line.find(&hit.matched).unwrap();
            assert_eq!(
                hit.variable,
                variable_at_reference(&line, start),
                "{}",
                hit.matched
            );
            named += usize::from(hit.variable.is_some());
        }
        assert!(named > 100 && named < result.hits.len(), "{named}");
        assert!(
            result
                .hits
                .iter()
                .any(|hit| hit.variable.as_deref() == Some("env"))
        );
    }

    /// Every code point the regex `\s` accepts.
    const WHITESPACE: [&str; 25] = [
        "\t", "\n", "\u{b}", "\u{c}", "\r", " ", "\u{85}", "\u{a0}", "\u{1680}", "\u{2000}",
        "\u{2001}", "\u{2002}", "\u{2003}", "\u{2004}", "\u{2005}", "\u{2006}", "\u{2007}",
        "\u{2008}", "\u{2009}", "\u{200a}", "\u{2028}", "\u{2029}", "\u{202f}", "\u{205f}",
        "\u{3000}",
    ];

    /// Like `assert_same_as_reference`, but asks in the given order, so the
    /// per-line records are first built by whichever start comes first.
    fn assert_same_in_order(line: &str, starts: &[usize]) {
        let keys = LineKeys::new(line);
        for &start in starts {
            assert_eq!(
                keys.variable_at(start),
                variable_at_reference(line, start),
                "{line:?} at byte {start}"
            );
        }
    }

    #[test]
    fn line_keys_match_the_prefix_scan_on_long_keys_and_value_runs() {
        let values = |count: usize, separator: &str| {
            (0..count)
                .map(|index| format!("10.0.{}.{}", index / 256, index % 256))
                .collect::<Vec<_>>()
                .join(separator)
        };
        let lines = [
            // One short key followed by many values.
            format!("ips = {}", values(120, " ")),
            format!("ips:\u{a0}{}", values(120, "\u{2028}")),
            format!("ips =\u{85}{}", values(120, "\u{3000}")),
            // A delimiter part way through cuts the later values off.
            format!("ips = {}; {}", values(40, " "), values(40, " ")),
            format!(
                "ips = {} , {} {{{}}}",
                values(30, " "),
                values(30, " "),
                values(30, " ")
            ),
            format!("k = //{}", values(40, " ")),
            format!("k =\u{a0}//{}", values(40, " ")),
            format!("k = /{}", values(40, "//")),
            // Hits inside one long key name, with and without a namespace.
            format!("k-{}=v", values(120, "-")),
            format!("$env:k-{}=v", values(120, "-")),
            format!("$script:k.{} = v", values(120, ".")),
            format!("global:k_{}\u{2028}:\u{85}v", values(120, "_")),
            format!("\"$env:k-{}\" = v", values(60, "-")),
            format!("$env:k-{}==:=v", values(50, "-")),
            format!("x=$env:k-{}=v", values(50, "-")),
            format!("{{$env:k-{};k2-{}:v", values(40, "-"), values(40, "-")),
            // Hits in a whitespace run after a long key's `=`.
            format!(
                "k-{}=\u{a0} \u{2028}\u{85}\u{3000} {}",
                values(40, "-"),
                values(40, " ")
            ),
            // Several long keys in a row.
            format!(
                "a-{}=1 $env:b-{}=2;c-{}: 3",
                values(30, "-"),
                values(30, "-"),
                values(30, "-")
            ),
        ];
        let mut rescued = 0;
        for line in &lines {
            rescued += assert_same_as_reference(line);
        }
        assert!(rescued > 500, "only {rescued} rescued boundaries");
    }

    #[test]
    fn line_keys_match_the_prefix_scan_on_random_unicode_lines() {
        const TOKENS: [&str; 40] = [
            "a",
            "Z",
            "_",
            "k",
            "0",
            "9",
            ".",
            "-",
            ":",
            "=",
            ";",
            ",",
            "{",
            "}",
            "\"",
            "'",
            "$",
            "/",
            "//",
            "env:",
            "script:",
            "global:",
            "$env:",
            "é",
            "名",
            "🙂",
            "k=",
            "k:",
            " = ",
            "=:",
            "::",
            "==",
            ":=",
            "long-key.name_",
            "192.0.2.1",
            "\u{200b}",
            "\u{feff}",
            "\u{180e}",
            "\u{ff1d}",
            "\u{ff1a}",
        ];
        let mut state: u64 = 0x9E37_79B9_7F4A_7C15;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let mut rescued = 0;
        for round in 0..3000 {
            let length = (next() % 48) as usize;
            let line: String = (0..length)
                .map(|_| {
                    if next() % 4 == 0 {
                        WHITESPACE[(next() % WHITESPACE.len() as u64) as usize]
                    } else {
                        TOKENS[(next() % TOKENS.len() as u64) as usize]
                    }
                })
                .collect();
            if round % 2 == 0 {
                rescued += assert_same_as_reference(&line);
                continue;
            }
            let mut starts: Vec<usize> = (0..=line.len())
                .filter(|&start| line.is_char_boundary(start))
                .collect();
            for index in (1..starts.len()).rev() {
                starts.swap(index, (next() % (index as u64 + 1)) as usize);
            }
            assert_same_in_order(&line, &starts);
        }
        assert!(rescued > 200, "only {rescued} rescued boundaries");
    }
}

#[cfg(test)]
mod directive_tests {
    use super::{LineDirectives, allowed_by_directive, directive_regex};
    use crate::config::Config;

    /// The per-match check that `LineDirectives` replaced, kept verbatim as
    /// the oracle.
    fn allowed_by_directive_reference(line: &str, matched: &str, config: &Config) -> bool {
        if config.allow.disallow_inline_allow {
            return false;
        }
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

    /// Every combination of the two allow switches, plus `--strict`.
    fn configs() -> Vec<Config> {
        let mut configs = Vec::new();
        for bare_off in [false, true] {
            for inline_off in [false, true] {
                let mut config = Config::default();
                config.allow.disallow_bare_allow = bare_off;
                config.allow.disallow_inline_allow = inline_off;
                configs.push(config);
            }
        }
        let mut strict = Config::default();
        strict.apply_strict();
        configs.push(strict);
        configs
    }

    /// A synthetic private address, assembled so the source holds none.
    fn address() -> String {
        [10, 1, 2, 3].map(|octet: u8| octet.to_string()).join(".")
    }

    /// Directive fragments; lines are made by joining several of them.
    fn parts() -> Vec<String> {
        let address = address();
        let mut parts: Vec<String> = [
            "doxguard: allow",
            "DOXGUARD:allow",
            "DoxGuard:\tALLOW",
            "secrets-scan: allow",
            "SECRETS-SCAN:\u{3000}allow",
            "doxguard: allow-list",
            "doxguard: allowlist",
            "doxguard: allow=x",
            "doxguard: allow-->",
            "doxguard: allow -->",
            "<!-- doxguard: allow -->",
            "<!-- doxguard: allow fixa-->",
            "<!-- doxguard: allow fixalpha-->",
            "<!-- doxguard: allow fixalpha -->",
            "doxguard: allow fixalpha",
            "doxguard: allow FixAlpha,",
            "doxguard: allow fixalpha。",
            "doxguard: allow\u{3000}FIXALPHA",
            "doxguard: allow fixalpha>",
            "doxguard: allow ;;;",
            "doxguard: allow 名前テスト。",
            "doxguard: allow 名前",
            "doxguard: allow abc",
            "doxguard: allow abcd",
            "doxguard: allow ab--",
            "doxguard: allow --",
            "doxguard: allow -legacy",
            "doxguard: allow fixture-unrelated",
            "doxguard:",
            ": allow",
            " allow ",
            " ",
            "x",
            ">",
            "-",
            "--",
            "-->",
            "\t",
            "é",
            "fixalpha",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        parts.push(format!("doxguard: allow {address}"));
        parts.push(format!("doxguard: allow {}", &address[..6]));
        parts.push(format!("doxguard: allow {}", &address[..4]));
        parts.push(format!("<!-- secrets-scan: allow {address}-->"));
        parts.push(address);
        parts
    }

    fn matched_values() -> Vec<String> {
        let address = address();
        let mut values: Vec<String> = [
            "fixalpha",
            "FIXALPHA-x",
            "xFixAlphay",
            "名前テスト",
            "名前",
            "abc",
            "abcd",
            "zz",
            "",
            "fixture-unrelated",
            "Fixture-Unrelated-Value",
            "-legacy-x",
            ";;;",
        ]
        .into_iter()
        .map(str::to_owned)
        .collect();
        values.push(address[..6].to_owned());
        values.push(address);
        values
    }

    /// Compares the parse-once answer, through the struct and through the
    /// public function, with the oracle; returns how many were allowed.
    fn assert_same_as_reference(line: &str, values: &[String], configs: &[Config]) -> usize {
        let mut allowed = 0;
        for config in configs {
            let directives = LineDirectives::new(line, config);
            for value in values {
                let expected = allowed_by_directive_reference(line, value, config);
                let context = format!(
                    "{line:?} {value:?} bare_off={} inline_off={}",
                    config.allow.disallow_bare_allow, config.allow.disallow_inline_allow
                );
                assert_eq!(directives.allows(value), expected, "{context}");
                assert_eq!(
                    allowed_by_directive(line, value, config),
                    expected,
                    "{context}"
                );
                allowed += usize::from(expected);
            }
        }
        allowed
    }

    #[test]
    fn line_directives_match_the_per_match_check_on_every_pair_of_parts() {
        let parts = parts();
        let values = matched_values();
        let configs = configs();
        let (mut allowed, mut checked) = (0, 0);
        for first in &parts {
            for second in &parts {
                let line = format!("{first}{second}");
                allowed += assert_same_as_reference(&line, &values, &configs);
                checked += values.len() * configs.len();
            }
        }
        assert!(
            allowed > checked / 50 && allowed < checked / 2,
            "{allowed} of {checked}"
        );
    }

    #[test]
    fn line_directives_match_the_per_match_check_on_random_lines() {
        let parts = parts();
        let values = matched_values();
        let configs = configs();
        let mut state: u64 = 0xD1EC_7100_0000_0001;
        let mut next = move || {
            state ^= state << 13;
            state ^= state >> 7;
            state ^= state << 17;
            state
        };
        let (mut allowed, mut checked) = (0, 0);
        for _ in 0..2000 {
            let count = (next() % 8) as usize;
            let line: String = (0..count)
                .map(|_| parts[(next() % parts.len() as u64) as usize].as_str())
                .collect();
            allowed += assert_same_as_reference(&line, &values, &configs);
            checked += values.len() * configs.len();
        }
        assert!(
            allowed > checked / 50 && allowed < checked / 2,
            "{allowed} of {checked}"
        );
    }
}
