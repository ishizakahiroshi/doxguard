use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use regex::{Regex, RegexBuilder};
use serde::Deserialize;

pub const CONFIG_FILENAME: &str = "doxguard.config.json";

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase", deny_unknown_fields)]
pub enum WatchlistSource {
    Lines {
        path: String,
        #[serde(default)]
        label: Option<String>,
    },
    Csv {
        path: String,
        column: ColumnSpec,
        #[serde(default)]
        label: Option<String>,
        #[serde(default, rename = "parenVariants")]
        paren_variants: bool,
    },
}

impl WatchlistSource {
    pub fn path(&self) -> &str {
        match self {
            Self::Lines { path, .. } | Self::Csv { path, .. } => path,
        }
    }

    pub fn label(&self) -> String {
        match self {
            Self::Lines { label, .. } | Self::Csv { label, .. } => {
                label.clone().unwrap_or_else(|| "watchlist".to_owned())
            }
        }
    }

    pub fn display_label(&self, index: usize) -> String {
        match self {
            Self::Lines { label, .. } | Self::Csv { label, .. } => label
                .clone()
                .unwrap_or_else(|| format!("watchlist source #{}", index + 1)),
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
            fail_on_skip: false,
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
                bail!("watchlist path must not be empty");
            }
            if let WatchlistSource::Csv {
                column: ColumnSpec::Index(index),
                ..
            } = source
            {
                if *index == 0 {
                    bail!("numeric CSV columns are 1-based and must be at least 1");
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
        bail!("--config path must not be empty");
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
            bail!("config not found: {}", path.display());
        }
        return Ok(LoadedConfig {
            config: Config::default(),
            path,
            found: false,
            warnings: Vec::new(),
        });
    }
    // Hard ceiling checked before the config is read, so `maxFileSize` cannot
    // be the thing that protects loading the file that defines it.
    const CONFIG_MAX_BYTES: u64 = 1 << 20;
    let meta =
        fs::metadata(&path).with_context(|| format!("failed to stat config {}", path.display()))?;
    if meta.len() > CONFIG_MAX_BYTES {
        bail!(
            "config {} is {} bytes (limit is {CONFIG_MAX_BYTES}); refuse to load unbounded config",
            path.display(),
            meta.len()
        );
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let config: Config = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse config {}", path.display()))?;
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
