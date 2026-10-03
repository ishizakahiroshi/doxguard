use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
    sync::atomic::{AtomicBool, Ordering},
};

use anyhow::{Context, Result, bail};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;

pub const CONFIG_FILENAME: &str = "doxguard.config.json";

static SHOW_PATHS: AtomicBool = AtomicBool::new(false);

/// Turn on file-system locations in error and warning messages (`--show-paths` or
/// `DOXGUARD_SHOW_PATHS`). Off by default so locations do not reach AI transcripts or
/// public CI logs. Findings (`file:line`, JSON `file`) are not affected.
pub fn set_show_paths(enabled: bool) {
    SHOW_PATHS.store(enabled, Ordering::Relaxed);
}

pub fn show_paths() -> bool {
    SHOW_PATHS.load(Ordering::Relaxed)
}

/// `DOXGUARD_SHOW_PATHS` is on for `1` or `true` (case-insensitive).
pub fn show_paths_from_env() -> bool {
    std::env::var("DOXGUARD_SHOW_PATHS")
        .map(|value| {
            let value = value.trim();
            value == "1" || value.eq_ignore_ascii_case("true")
        })
        .unwrap_or(false)
}

/// Name a place for a message: `<name> <path>` when locations are enabled, otherwise
/// `<name>` followed by a pointer to `--show-paths`.
pub fn place(name: &str, path: &Path) -> String {
    if show_paths() {
        format!("{name} {}", path.display())
    } else {
        format!("{name} (location hidden; pass --show-paths to show it)")
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum WatchlistSource {
    Lines {
        path: String,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        label: Option<String>,
    },
    Csv {
        path: String,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        column: Option<ColumnSpec>,
        #[serde(default)]
        columns: Option<Vec<ColumnSpec>>,
        #[serde(default)]
        label: Option<String>,
        #[serde(default, rename = "parenVariants")]
        paren_variants: bool,
    },
    Directory {
        path: String,
        #[serde(default)]
        optional: bool,
        #[serde(default)]
        label: Option<String>,
        #[serde(default = "default_directory_min_length", rename = "minNameLength")]
        min_name_length: usize,
        #[serde(default = "default_directory_depth", rename = "maxDepth")]
        max_depth: usize,
        #[serde(default = "default_directory_entries", rename = "maxEntries")]
        max_entries: usize,
        /// Include link basenames without inspecting or traversing their targets.
        /// The directory root must still be a real directory.
        #[serde(default, rename = "includeLinkNames")]
        include_link_names: bool,
    },
}

fn default_directory_min_length() -> usize {
    10
}
fn default_directory_depth() -> usize {
    2
}
fn default_directory_entries() -> usize {
    5000
}

impl WatchlistSource {
    pub fn path(&self) -> &str {
        match self {
            Self::Lines { path, .. } | Self::Csv { path, .. } | Self::Directory { path, .. } => {
                path
            }
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Lines { label, .. } | Self::Csv { label, .. } | Self::Directory { label, .. } => {
                label.clone().unwrap_or_else(|| "watchlist".to_owned())
            }
        }
    }

    pub fn display_label(&self, index: usize) -> String {
        match self {
            Self::Lines { label, .. } | Self::Csv { label, .. } | Self::Directory { label, .. } => {
                label
                    .clone()
                    .unwrap_or_else(|| format!("watchlist source #{}", index + 1))
            }
        }
    }

    pub fn optional(&self) -> bool {
        match self {
            Self::Lines { optional, .. }
            | Self::Csv { optional, .. }
            | Self::Directory { optional, .. } => *optional,
        }
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum ColumnSpec {
    Name(String),
    Index(usize),
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CustomPatternConfig {
    pub name: String,
    pub regex: String,
    #[serde(default)]
    pub suggestion: Option<String>,
}

const CUSTOM_REGEX_SIZE_LIMIT: usize = 1 << 20;

pub fn compile_custom_regex(pattern: &CustomPatternConfig) -> Result<Regex> {
    RegexBuilder::new(&pattern.regex)
        .size_limit(CUSTOM_REGEX_SIZE_LIMIT)
        .dfa_size_limit(CUSTOM_REGEX_SIZE_LIMIT)
        .build()
        .with_context(|| format!("invalid custom regex `{}`", pattern.name))
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct StructuralConfig {
    #[serde(rename = "windowsPath")]
    pub windows_path: bool,
    #[serde(rename = "posixHome")]
    pub posix_home: bool,
    #[serde(rename = "privateIp")]
    pub private_ip: bool,
    pub email: bool,
    pub custom: Vec<CustomPatternConfig>,
}

impl Default for StructuralConfig {
    fn default() -> Self {
        Self {
            windows_path: true,
            posix_home: true,
            private_ip: true,
            email: true,
            custom: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct AllowConfig {
    pub names: Vec<String>,
    pub emails: Vec<String>,
    #[serde(rename = "emailDomains")]
    pub email_domains: Vec<String>,
    /// When true, bare `doxguard: allow` (no token) is ignored. Scoped allows still work.
    #[serde(rename = "disallowBareAllow")]
    pub disallow_bare_allow: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NoiseConfig {
    #[serde(rename = "shortNeedleMaxLength")]
    pub short_needle_max_length: usize,
    #[serde(rename = "stagedAddedLinesOnly")]
    pub staged_added_lines_only: bool,
    #[serde(rename = "minNeedleLength")]
    pub min_needle_length: usize,
    #[serde(rename = "skipShortKanaGivenNames")]
    pub skip_short_kana_given_names: bool,
    /// Match ASCII letters case-insensitively (needles and haystack lowercased).
    #[serde(rename = "asciiCaseInsensitive")]
    pub ascii_case_insensitive: bool,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            short_needle_max_length: 0,
            staged_added_lines_only: false,
            min_needle_length: 2,
            skip_short_kana_given_names: true,
            ascii_case_insensitive: false,
        }
    }
}

fn default_max_file_size() -> u64 {
    1024 * 1024
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Config {
    pub watchlists: Vec<WatchlistSource>,
    pub structural: StructuralConfig,
    pub allow: AllowConfig,
    pub noise: NoiseConfig,
    #[serde(rename = "exemptPaths")]
    pub exempt_paths: Vec<String>,
    #[serde(rename = "maxFileSize")]
    pub max_file_size: u64,
    /// When true with `--block`, coverage skips (oversize / non-UTF-8 / symlink / unreadable)
    /// cause exit 1 so silent holes cannot pass a gate.
    #[serde(rename = "failOnSkip")]
    pub fail_on_skip: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watchlists: Vec::new(),
            structural: StructuralConfig::default(),
            allow: AllowConfig {
                names: Vec::new(),
                emails: Vec::new(),
                email_domains: vec![
                    "example.com".to_owned(),
                    "users.noreply.github.com".to_owned(),
                ],
                disallow_bare_allow: false,
            },
            noise: NoiseConfig::default(),
            exempt_paths: Vec::new(),
            max_file_size: default_max_file_size(),
            fail_on_skip: true,
        }
    }
}

impl Config {
    /// Apply CLI `--strict`: bare allow off + fail on coverage skips.
    pub fn apply_strict(&mut self) {
        self.allow.disallow_bare_allow = true;
        self.fail_on_skip = true;
    }

    pub fn all_exempt_paths(&self) -> impl Iterator<Item = &str> {
        self.exempt_paths.iter().map(String::as_str)
    }

    pub fn validate(&self) -> Result<()> {
        if self.noise.min_needle_length == 0 {
            bail!("noise.minNeedleLength must be at least 1");
        }
        if self.max_file_size == 0 {
            bail!("maxFileSize must be at least 1");
        }
        for exempt in &self.exempt_paths {
            let trimmed = exempt.trim();
            let normalized = trimmed.replace('\\', "/");
            let normalized = normalized.trim_end_matches('/');
            let drive_absolute = normalized
                .as_bytes()
                .get(1)
                .is_some_and(|separator| *separator == b':');
            if normalized.is_empty()
                || normalized.starts_with('/')
                || drive_absolute
                || normalized
                    .split('/')
                    .any(|component| component.is_empty() || component == "." || component == "..")
            {
                bail!(
                    "exemptPaths entries must be repository-relative files or directories without `.` or `..` components (got {exempt:?})"
                );
            }
        }
        for domain in &self.allow.email_domains {
            if !is_multi_label_domain(domain) {
                bail!(
                    "allow.emailDomains entry must be a multi-label domain like example.com (got {domain:?})"
                );
            }
        }
        for source in &self.watchlists {
            if source.path().is_empty() {
                bail!(
                    "watchlist path must not be empty. Set `path` for every watchlist source (for example `${{WATCHLIST_ROOT}}/terms.txt`)"
                );
            }
            if let WatchlistSource::Csv {
                column, columns, ..
            } = source
            {
                if column.is_some() == columns.is_some()
                    || columns.as_ref().is_some_and(Vec::is_empty)
                {
                    bail!("CSV sources require exactly one of `column` or non-empty `columns`");
                }
                for selected in column.iter().chain(columns.iter().flatten()) {
                    match selected {
                        ColumnSpec::Index(0) => {
                            bail!("numeric CSV columns are 1-based and must be at least 1")
                        }
                        ColumnSpec::Name(name) if name.trim().is_empty() => {
                            bail!("CSV column names must not be empty")
                        }
                        _ => {}
                    }
                }
            }
            if let WatchlistSource::Directory {
                min_name_length,
                max_depth,
                max_entries,
                ..
            } = source
            {
                if *min_name_length == 0
                    || *max_depth == 0
                    || *max_depth > 64
                    || *max_entries == 0
                    || *max_entries > 1_000_000
                {
                    bail!(
                        "directory sources require minNameLength >= 1, maxDepth 1..64, and maxEntries 1..1000000"
                    );
                }
            }
        }
        for custom in &self.structural.custom {
            compile_custom_regex(custom)?;
        }
        Ok(())
    }
}

fn is_multi_label_domain(domain: &str) -> bool {
    let domain = domain.trim().trim_matches('.');
    if domain.is_empty() || domain.contains('/') || domain.contains(' ') {
        return false;
    }
    // Require at least one dot so bare TLDs like "com" cannot mute all emails.
    domain.contains('.') && !domain.starts_with('.') && !domain.ends_with('.')
}

#[derive(Debug)]
pub struct LoadedConfig {
    pub config: Config,
    pub path: PathBuf,
    pub found: bool,
    pub warnings: Vec<String>,
}

pub fn load(cwd: &Path, requested_path: Option<&Path>) -> Result<LoadedConfig> {
    load_from(cwd, cwd, requested_path)
}

/// Load an explicitly requested config relative to the invocation directory,
/// while resolving the implicit repository config from `auto_root`.
pub fn load_from(
    invocation_cwd: &Path,
    auto_root: &Path,
    requested_path: Option<&Path>,
) -> Result<LoadedConfig> {
    if requested_path.is_some_and(|path| path.as_os_str().is_empty()) {
        bail!("--config path must not be empty. Pass a config file path, or omit --config");
    }
    let env_path = std::env::var_os("DOXGUARD_CONFIG")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from);
    let explicit = requested_path.is_some() || env_path.is_some();
    let requested = requested_path
        .map(PathBuf::from)
        .or(env_path)
        .unwrap_or_else(|| PathBuf::from(CONFIG_FILENAME));
    let path = if requested.is_absolute() {
        requested
    } else {
        let base = if explicit { invocation_cwd } else { auto_root };
        base.join(requested)
    };
    if !path.exists() {
        if explicit {
            bail!(
                "{} not found. Check the --config value or DOXGUARD_CONFIG, or create one with `doxguard init`",
                place("the config", &path)
            );
        }
        // No config was found at the repository root. Falling back to defaults is
        // intentional for structural-only CI, but it is also what happens when
        // `doxguard init` created a config in a subdirectory: that config is never
        // read from the worktree root, and the scan silently runs with 0 watchlist
        // needles. Surface it (without echoing the absolute path) so the drop is not
        // invisible. The message is a warning only; it does not change the exit code.
        return Ok(LoadedConfig {
            config: Config::default(),
            path,
            found: false,
            warnings: vec![
                "WARN: no doxguard config found at the repository root; scanning with built-in structural patterns only (0 watchlist needles). If you ran `doxguard init` in a subdirectory, move doxguard.config.json to the repository root.".to_owned(),
            ],
        });
    }
    // Hard ceiling checked before the config is read, so `maxFileSize` cannot
    // be the thing that protects loading the file that defines it.
    const CONFIG_MAX_BYTES: u64 = 1 << 20;
    let meta = fs::metadata(&path)
        .with_context(|| format!("failed to stat {}", place("the config", &path)))?;
    if meta.len() > CONFIG_MAX_BYTES {
        bail!(
            "{} is {} bytes (limit is {CONFIG_MAX_BYTES}); refuse to load unbounded config. Trim the config; large term lists belong in a watchlist file",
            place("the config", &path),
            meta.len()
        );
    }
    // A character device (e.g. /dev/zero) reports len 0 and would make
    // read_to_string loop forever; only read regular files.
    if !meta.is_file() {
        bail!(
            "{} is not a regular file; refuse to read a non-regular path. Point --config / DOXGUARD_CONFIG at a JSON file",
            place("the config", &path)
        );
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read {}", place("the config", &path)))?;
    let config: Config = serde_json::from_str(&text).with_context(|| {
        format!(
            "failed to parse {}; fix the JSON at the position reported below",
            place("the config", &path)
        )
    })?;
    config.validate()?;

    let warnings = config
        .watchlists
        .iter()
        .enumerate()
        .filter(|(_, source)| !source.path().contains("${"))
        .map(|(index, _)| {
            format!(
                "WARN: watchlist source #{} uses a literal path instead of an environment reference. Prefer `${{WATCHLIST_ROOT}}/...` so private paths never enter the repository.",
                index + 1
            )
        })
        .collect();

    Ok(LoadedConfig {
        config,
        path,
        found: true,
        warnings,
    })
}

pub fn process_env() -> HashMap<String, String> {
    std::env::vars_os()
        .filter_map(|(name, value)| Some((name.into_string().ok()?, value.into_string().ok()?)))
        .collect()
}
