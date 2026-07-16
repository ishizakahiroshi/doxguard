use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use serde::Deserialize;

pub const CONFIG_FILENAME: &str = "doxguard.config.json";

const DEFAULT_EXEMPT_PATHS: &[&str] = &[
    CONFIG_FILENAME,
    "src/patterns.rs",
    "tests/",
    ".githooks/pre-commit",
    ".husky/pre-commit",
    ".github/workflows/doxguard",
    ".github/workflows/validate",
    ".github/workflows/release",
    "docs/local/",
];

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
            Self::Lines { path, label } => label.clone().unwrap_or_else(|| format!("lines:{path}")),
            Self::Csv { path, label, .. } => label.clone().unwrap_or_else(|| format!("csv:{path}")),
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
}

#[derive(Debug, Clone, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct NoiseConfig {
    #[serde(rename = "minNeedleLength")]
    pub min_needle_length: usize,
    #[serde(rename = "skipShortKanaGivenNames")]
    pub skip_short_kana_given_names: bool,
}

impl Default for NoiseConfig {
    fn default() -> Self {
        Self {
            min_needle_length: 2,
            skip_short_kana_given_names: true,
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
            },
            noise: NoiseConfig::default(),
            exempt_paths: Vec::new(),
            max_file_size: default_max_file_size(),
        }
    }
}

impl Config {
    pub fn all_exempt_paths(&self) -> impl Iterator<Item = &str> {
        DEFAULT_EXEMPT_PATHS
            .iter()
            .copied()
            .chain(self.exempt_paths.iter().map(String::as_str))
    }

    pub fn validate(&self) -> Result<()> {
        if self.noise.min_needle_length == 0 {
            bail!("noise.minNeedleLength must be at least 1");
        }
        if self.max_file_size == 0 {
            bail!("maxFileSize must be at least 1");
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
            regex::Regex::new(&custom.regex)
                .with_context(|| format!("invalid custom regex `{}`", custom.name))?;
        }
        Ok(())
    }
}

#[derive(Debug)]
pub struct LoadedConfig {
    pub config: Config,
    pub path: PathBuf,
    pub found: bool,
    pub warnings: Vec<String>,
}

pub fn load(cwd: &Path, requested_path: Option<&Path>) -> Result<LoadedConfig> {
    let env_path = std::env::var_os("DOXGUARD_CONFIG").map(PathBuf::from);
    let requested = requested_path
        .map(PathBuf::from)
        .or(env_path)
        .unwrap_or_else(|| PathBuf::from(CONFIG_FILENAME));
    let path = if requested.is_absolute() {
        requested
    } else {
        cwd.join(requested)
    };
    if !path.exists() {
        return Ok(LoadedConfig {
            config: Config::default(),
            path,
            found: false,
            warnings: Vec::new(),
        });
    }
    let text = fs::read_to_string(&path)
        .with_context(|| format!("failed to read config {}", path.display()))?;
    let config: Config = serde_json::from_str(&text)
        .with_context(|| format!("failed to parse config {}", path.display()))?;
    config.validate()?;

    let warnings = config
        .watchlists
        .iter()
        .filter(|source| !source.path().contains("${"))
        .map(|source| {
            format!(
                "WARN: watchlist path is literal instead of env-based: {}. Prefer `${{WATCHLIST_ROOT}}/...` so private paths never enter the repository.",
                source.path()
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
    std::env::vars().collect()
}
