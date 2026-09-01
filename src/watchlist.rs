use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use anyhow::{Context, Result, anyhow, bail};

use crate::config::{ColumnSpec, Config, WatchlistSource};

const WATCHLIST_MAX_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WatchlistItem {
    pub needle: String,
    pub source: String,
}

#[derive(Debug)]
pub struct WatchlistMatcher {
    items: Vec<WatchlistItem>,
    matcher: Option<AhoCorasick>,
}

impl WatchlistMatcher {
    pub fn new(items: Vec<WatchlistItem>) -> Result<Self> {
        let matcher = if items.is_empty() {
            None
        } else {
            Some(
                AhoCorasickBuilder::new()
                    .match_kind(MatchKind::Standard)
                    .build(items.iter().map(|item| item.needle.as_str()))?,
            )
        };
        Ok(Self { items, matcher })
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn matches<'a>(&'a self, line: &'a str) -> impl Iterator<Item = &'a WatchlistItem> {
        self.matches_spanned(line).map(|(item, _)| item)
    }

    /// First occurrence per needle, with its byte range in the haystack.
    pub fn matches_spanned<'a>(
        &'a self,
        line: &'a str,
    ) -> impl Iterator<Item = (&'a WatchlistItem, std::ops::Range<usize>)> {
        let mut seen = HashSet::new();
        self.matcher
            .as_ref()
            .into_iter()
            .flat_map(move |matcher| matcher.find_overlapping_iter(line))
            .filter_map(move |found| {
                let index = found.pattern().as_usize();
                seen.insert(index)
                    .then(|| (&self.items[index], found.start()..found.end()))
            })
    }
}

#[derive(Debug)]
pub struct LoadedWatchlists {
    pub matcher: WatchlistMatcher,
    pub warnings: Vec<String>,
}

fn expand_path(
    template: &str,
    cwd: &Path,
    env: &HashMap<String, String>,
    source_number: usize,
) -> std::result::Result<PathBuf, String> {
    static VARIABLE: std::sync::OnceLock<regex::Regex> = std::sync::OnceLock::new();
    let variable = VARIABLE.get_or_init(|| {
        regex::Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").expect("static regex")
    });
    let mut missing = Vec::new();
    let expanded = variable.replace_all(template, |captures: &regex::Captures<'_>| {
        let name = &captures[1];
        match env.get(name) {
            Some(value) if !value.is_empty() => value.clone(),
            _ => {
                missing.push(name.to_owned());
                String::new()
            }
        }
    });
    if !missing.is_empty() {
        missing.sort();
        missing.dedup();
        return Err(format!(
            "WARN: {} not set; skipped watchlist source #{source_number}",
            missing.join(", "),
        ));
    }
    let path = PathBuf::from(expanded.as_ref());
    Ok(if path.is_absolute() {
        path
    } else {
        cwd.join(path)
    })
}

fn is_short_kana(value: &str) -> bool {
    value.chars().count() <= 2
        && value
            .chars()
            .all(|character| matches!(character, '\u{3041}'..='\u{3096}' | 'ー'))
}

fn should_skip(value: &str, source: &WatchlistSource, config: &Config) -> bool {
    if value.chars().count() < config.noise.min_needle_length {
        return true;
    }
    let looks_like_given_name = match source {
        WatchlistSource::Csv { column, label, .. } => {
            let descriptor = match column {
                ColumnSpec::Name(name) => {
                    format!("{} {name}", label.as_deref().unwrap_or_default())
                }
                ColumnSpec::Index(index) => {
                    format!("{} {index}", label.as_deref().unwrap_or_default())
                }
            };
            descriptor.to_ascii_lowercase().contains("given")
        }
        WatchlistSource::Lines { label, .. } => label
            .as_deref()
            .unwrap_or_default()
            .to_ascii_lowercase()
            .contains("given"),
    };
    config.noise.skip_short_kana_given_names && looks_like_given_name && is_short_kana(value)
}

fn expand_variants(value: &str, paren_variants: bool) -> Vec<String> {
    let mut values = vec![value.to_owned()];
    if paren_variants {
        let stripped = value.split(['(', '（']).next().unwrap_or(value).trim();
        if !stripped.is_empty() && stripped != value {
            values.push(stripped.to_owned());
        }
    }
    values
}

fn read_values(source: &WatchlistSource, path: &Path) -> Result<Vec<String>> {
    match source {
        WatchlistSource::Lines { .. } => {
            let text = fs::read_to_string(path)?;
            Ok(text
                .strip_prefix('\u{feff}')
                .unwrap_or(&text)
                .lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .map(str::to_owned)
                .collect())
        }
        WatchlistSource::Csv { column, .. } => {
            let mut reader = csv::Reader::from_path(path)?;
            let headers = reader.headers()?.clone();
            let index = match column {
                ColumnSpec::Name(name) => headers
                    .iter()
                    .enumerate()
                    .position(|(index, header)| {
                        let header = if index == 0 {
                            header.strip_prefix('\u{feff}').unwrap_or(header)
                        } else {
                            header
                        };
                        header == name
                    })
                    .ok_or_else(|| anyhow!("column `{name}` not found"))?,
                ColumnSpec::Index(index) => {
                    let zero_based = index - 1;
                    if zero_based >= headers.len() {
                        bail!(
                            "CSV column index {index} is out of range (headers: {})",
                            headers.len()
                        );
                    }
                    zero_based
                }
            };
            reader
                .records()
                .map(|record| {
                    let record = record?;
                    Ok(record.get(index).unwrap_or_default().trim().to_owned())
                })
                .filter(|result: &Result<String>| {
                    result
                        .as_ref()
                        .map(|value| !value.is_empty())
                        .unwrap_or(true)
                })
                .collect()
        }
    }
}

pub fn load(
    config: &Config,
    cwd: &Path,
    env: &HashMap<String, String>,
) -> Result<LoadedWatchlists> {
    let mut warnings = Vec::new();
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let allowed_owned: HashSet<String> = if config.noise.ascii_case_insensitive {
        config
            .allow
            .names
            .iter()
            .map(|name| name.to_ascii_lowercase())
            .collect()
    } else {
        config.allow.names.iter().cloned().collect()
    };
    let allowed: HashSet<&str> = allowed_owned.iter().map(String::as_str).collect();

    for (source_index, source) in config.watchlists.iter().enumerate() {
        let source_number = source_index + 1;
        let path = match expand_path(source.path(), cwd, env, source_number) {
            Ok(path) => path,
            Err(warning) => {
                warnings.push(warning);
                continue;
            }
        };
        if !path.exists() {
            bail!(
                "watchlist source #{source_number} resolved successfully but was not found; check the configured environment variable or path"
            );
        }
        let meta = fs::metadata(&path)
            .with_context(|| format!("failed to stat watchlist source #{source_number}"))?;
        let limit = config.max_file_size.min(WATCHLIST_MAX_BYTES);
        if meta.len() > limit {
            bail!(
                "watchlist source #{source_number} is {} bytes (effective limit is {limit}); refuse to load unbounded lists",
                meta.len(),
            );
        }
        // A character device (e.g. /dev/zero) reports len 0 and would make the
        // read loop forever; only read regular files.
        if !meta.file_type().is_file() {
            bail!(
                "watchlist source #{source_number} is not a regular file; refuse to read a non-regular path"
            );
        }
        // An unset environment variable stays a soft skip for structural-only CI.
        // Once a source resolves, missing/read/parse failures fail closed so protection
        // cannot shrink silently.
        let values = read_values(source, &path)
            .with_context(|| format!("failed to read watchlist source #{source_number}"))?;
        let paren_variants = matches!(
            source,
            WatchlistSource::Csv {
                paren_variants: true,
                ..
            }
        );
        for value in values {
            for needle in expand_variants(value.trim(), paren_variants) {
                let needle = if config.noise.ascii_case_insensitive {
                    needle.to_ascii_lowercase()
                } else {
                    needle
                };
                if should_skip(&needle, source, config)
                    || allowed.contains(needle.as_str())
                    || !seen.insert(needle.clone())
                {
                    continue;
                }
                items.push(WatchlistItem {
                    needle,
                    source: source.display_label(source_index),
                });
            }
        }
    }

    Ok(LoadedWatchlists {
        matcher: WatchlistMatcher::new(items)?,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_needles_are_all_reported_once() {
        let matcher = WatchlistMatcher::new(vec![
            WatchlistItem {
                needle: "Acme".to_owned(),
                source: "fixture".to_owned(),
            },
            WatchlistItem {
                needle: "Acme Labs".to_owned(),
                source: "fixture".to_owned(),
            },
        ])
        .unwrap();
        let matches: Vec<_> = matcher
            .matches("Acme Labs and Acme")
            .map(|item| item.needle.as_str())
            .collect();
        assert_eq!(matches, ["Acme", "Acme Labs"]);
    }
}
