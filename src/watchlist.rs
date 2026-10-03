use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
};

use aho_corasick::{AhoCorasick, AhoCorasickBuilder, MatchKind};
use anyhow::{Context, Result, anyhow, bail};
use serde::Serialize;

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
    statistics: WatchlistStatistics,
}

/// Counts candidates after variant expansion, before deduplication.
#[derive(Debug, Clone, Default, Serialize)]
pub struct WatchlistStatistics {
    pub candidates: usize,
    pub too_short: usize,
    pub short_kana: usize,
    pub allow_listed: usize,
    pub duplicate: usize,
    pub loaded: usize,
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
        let statistics = WatchlistStatistics {
            candidates: items.len(),
            loaded: items.len(),
            ..Default::default()
        };
        Ok(Self {
            items,
            matcher,
            statistics,
        })
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
        self.matches_spanned_with_boundary(line, 0)
    }

    pub fn statistics(&self) -> &WatchlistStatistics {
        &self.statistics
    }

    pub fn matches_spanned_with_boundary<'a>(
        &'a self,
        line: &'a str,
        maximum: usize,
    ) -> impl Iterator<Item = (&'a WatchlistItem, std::ops::Range<usize>)> {
        let mut seen = HashSet::new();
        self.matcher
            .as_ref()
            .into_iter()
            .flat_map(move |matcher| matcher.find_overlapping_iter(line))
            .filter_map(move |found| {
                let index = found.pattern().as_usize();
                let item = &self.items[index];
                if maximum > 0 && item.needle.chars().count() <= maximum {
                    let before = line[..found.start()].chars().next_back();
                    let after = line[found.end()..].chars().next();
                    if before.is_some_and(|c| c.is_ascii_alphanumeric())
                        || after.is_some_and(|c| c.is_ascii_alphanumeric())
                    {
                        return None;
                    }
                }
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

/// Normalize a needle exactly as `load` does before de-duplication and matching.
pub(crate) fn normalize_needle(value: &str, ascii_case_insensitive: bool) -> String {
    if ascii_case_insensitive {
        value.to_ascii_lowercase()
    } else {
        value.to_owned()
    }
}

/// Parse the text of a `lines` watchlist: BOM stripped, blank and `#` lines dropped.
pub(crate) fn parse_lines(text: &str) -> Vec<String> {
    text.strip_prefix('\u{feff}')
        .unwrap_or(text)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
        .map(str::to_owned)
        .collect()
}

/// `allow.names` in the same normalized form `load` compares against.
pub(crate) fn allowed_names(config: &Config) -> HashSet<String> {
    config
        .allow
        .names
        .iter()
        .map(|name| normalize_needle(name, config.noise.ascii_case_insensitive))
        .collect()
}

/// Expand `${NAME}` references; on failure returns the sorted names of the unset
/// (or empty) variables. Variable names are not secret; their values never leave here.
pub(crate) fn expand_path_names(
    template: &str,
    cwd: &Path,
    env: &HashMap<String, String>,
) -> std::result::Result<PathBuf, Vec<String>> {
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
        return Err(missing);
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

pub(crate) fn should_skip(value: &str, source: &WatchlistSource, config: &Config) -> bool {
    if value.chars().count() < config.noise.min_needle_length {
        return true;
    }
    let looks_like_given_name = match source {
        WatchlistSource::Csv {
            column: Some(column),
            label,
            ..
        } => {
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
        _ => false,
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

/// Rust recognizes name-surrogate Windows junctions as symlinks, without treating
/// unrelated reparse points (such as cloud placeholders) as links.
fn is_link(metadata: &fs::Metadata) -> bool {
    metadata.file_type().is_symlink()
}

fn directory_values(
    path: &Path,
    min_length: usize,
    max_depth: usize,
    max_entries: usize,
    include_link_names: bool,
) -> Result<Vec<String>> {
    let root_metadata = fs::symlink_metadata(path)?;
    if is_link(&root_metadata) || !root_metadata.is_dir() {
        bail!("directory root must be a real directory");
    }
    let mut stack = vec![(path.to_owned(), 1usize)];
    let mut visited = 0usize;
    let mut values = Vec::new();
    while let Some((directory, depth)) = stack.pop() {
        for entry in fs::read_dir(directory)? {
            let entry = entry?;
            visited += 1;
            if visited > max_entries {
                bail!("directory entry limit exceeded");
            }
            let metadata = fs::symlink_metadata(entry.path())?;
            let linked = is_link(&metadata);
            if linked && !include_link_names {
                bail!("directory contains a symbolic link or reparse point");
            }
            // Link metadata describes the directory entry itself. Never inspect
            // its target: the opt-in protects only the link's own basename.
            if linked || metadata.is_file() {
                let name = entry
                    .file_name()
                    .into_string()
                    .map_err(|_| anyhow!("non-UTF-8 filename"))?;
                if name.chars().count() >= min_length {
                    values.push(name);
                }
            } else if metadata.is_dir() {
                if depth < max_depth {
                    stack.push((entry.path(), depth + 1));
                }
            } else {
                bail!("directory contains a non-regular entry");
            }
        }
    }
    Ok(values)
}

fn read_values(source: &WatchlistSource, path: &Path) -> Result<Vec<String>> {
    match source {
        WatchlistSource::Lines { .. } => {
            let text = fs::read_to_string(path)?;
            Ok(parse_lines(&text))
        }
        WatchlistSource::Csv {
            column, columns, ..
        } => {
            let mut reader = csv::Reader::from_path(path)?;
            let headers = reader.headers()?.clone();
            let indices: Vec<usize> = column
                .iter()
                .chain(columns.iter().flatten())
                .map(|column| {
                    Ok(match column {
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
                            let zero_based = index
                                .checked_sub(1)
                                .ok_or_else(|| anyhow!("CSV indices are 1-based"))?;
                            if zero_based >= headers.len() {
                                bail!(
                                    "CSV column index {index} is out of range (headers: {})",
                                    headers.len()
                                );
                            }
                            zero_based
                        }
                    })
                })
                .collect::<Result<_>>()?;
            if indices.is_empty() {
                bail!("CSV source has no selected columns");
            }
            reader
                .records()
                .map(|record| {
                    let record = record?;
                    let cells: Vec<&str> = indices
                        .iter()
                        .map(|index| record.get(*index).unwrap_or_default().trim())
                        .collect();
                    Ok(if cells.iter().any(|cell| cell.is_empty()) {
                        String::new()
                    } else {
                        cells.concat()
                    })
                })
                .filter(|result: &Result<String>| {
                    result
                        .as_ref()
                        .map(|value| !value.is_empty())
                        .unwrap_or(true)
                })
                .collect()
        }
        WatchlistSource::Directory {
            min_name_length,
            max_depth,
            max_entries,
            include_link_names,
            ..
        } => directory_values(
            path,
            *min_name_length,
            *max_depth,
            *max_entries,
            *include_link_names,
        ),
    }
}

pub fn load(
    config: &Config,
    cwd: &Path,
    env: &HashMap<String, String>,
) -> Result<LoadedWatchlists> {
    let mut warnings = Vec::new();
    let mut statistics = WatchlistStatistics::default();
    let mut items = Vec::new();
    let mut seen = HashSet::new();
    let allowed_owned: HashSet<String> = allowed_names(config);
    let allowed: HashSet<&str> = allowed_owned.iter().map(String::as_str).collect();

    for (source_index, source) in config.watchlists.iter().enumerate() {
        let source_number = source_index + 1;
        let path = match expand_path_names(source.path(), cwd, env) {
            Ok(path) => path,
            Err(missing) => {
                if source.optional() {
                    warnings.push(format!(
                        "WARN: {} not set; skipped watchlist source #{source_number}",
                        missing.join(", ")
                    ));
                    continue;
                }
                bail!(
                    "required watchlist source #{source_number} has unset or empty environment variable(s) {}; set them, or explicitly use optional: true for structural-only environments",
                    missing.join(", ")
                );
            }
        };
        if !path.exists() {
            bail!(
                "watchlist source #{source_number} resolved successfully but was not found; check the configured environment variable or path"
            );
        }
        let meta = fs::metadata(&path).with_context(|| {
            format!(
                "failed to stat watchlist source #{source_number}; check the file's permissions"
            )
        })?;
        let limit = config.max_file_size.min(WATCHLIST_MAX_BYTES);
        if meta.len() > limit && !matches!(source, WatchlistSource::Directory { .. }) {
            bail!(
                "watchlist source #{source_number} is {} bytes (effective limit is {limit}); refuse to load unbounded lists. Trim the file or raise `maxFileSize` in the config",
                meta.len(),
            );
        }
        // A character device (e.g. /dev/zero) reports len 0 and would make the
        // read loop forever; only read regular files.
        if !meta.file_type().is_file() && !matches!(source, WatchlistSource::Directory { .. }) {
            bail!(
                "watchlist source #{source_number} is not a regular file; refuse to read a non-regular path. Point the source at a regular file"
            );
        }
        // Only explicitly optional sources can skip unset environment variables.
        // Once a source resolves, missing/read/parse failures fail closed so protection
        // cannot shrink silently.
        // Reader errors (including CSV records) can contain private cell values.
        // Report only the source number and actionable reason at this boundary.
        let values = read_values(source, &path).map_err(|_| anyhow!(
            "failed to read watchlist source #{source_number}; check readability, UTF-8/CSV columns, directory limits, and symbolic links"
        ))?;
        let paren_variants = matches!(
            source,
            WatchlistSource::Csv {
                paren_variants: true,
                ..
            }
        );
        for value in values {
            for needle in expand_variants(value.trim(), paren_variants) {
                let needle = normalize_needle(&needle, config.noise.ascii_case_insensitive);
                statistics.candidates += 1;
                if needle.chars().count() < config.noise.min_needle_length {
                    statistics.too_short += 1;
                    continue;
                }
                if should_skip(&needle, source, config) {
                    statistics.short_kana += 1;
                    continue;
                }
                if allowed.contains(needle.as_str()) {
                    statistics.allow_listed += 1;
                    continue;
                }
                if !seen.insert(needle.clone()) {
                    statistics.duplicate += 1;
                    continue;
                }
                statistics.loaded += 1;
                items.push(WatchlistItem {
                    needle,
                    source: source.display_label(source_index),
                });
            }
        }
    }

    let mut matcher = WatchlistMatcher::new(items)?;
    matcher.statistics = statistics;
    Ok(LoadedWatchlists { matcher, warnings })
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
