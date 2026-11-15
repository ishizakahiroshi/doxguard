//! Private, offline review of an immutable staged-content snapshot.
use crate::{
    config::{self, process_env},
    patterns,
    scan::{self, ContentLoad, ScanHit},
    watchlist,
};
use anyhow::{Context, Result, bail};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

/// A review keeps each staged text in memory, so it stays bounded even when the scan config has no size limit.
const REVIEW_BLOB_LIMIT: u64 = 16 * 1024 * 1024;

fn git(cwd: &Path, args: &[&str]) -> Result<Vec<u8>> {
    let out = Command::new(scan::git_program()?)
        .args(args)
        .current_dir(cwd)
        .output()
        .context("could not start Git for review")?;
    if !out.status.success() {
        bail!("Git review operation failed");
    }
    Ok(out.stdout)
}

#[derive(Serialize)]
pub struct Finding {
    pub hit: ScanHit,
    pub change: String,
    pub context: String,
}
#[derive(Serialize)]
pub struct Review {
    pub version: u8,
    pub repository: String,
    pub snapshot: String,
    pub generated_at: u64,
    pub scanned: usize,
    pub coverage_skips: usize,
    pub warning_count: usize,
    pub warnings: Vec<String>,
    pub findings: Vec<Finding>,
}

fn hash(cwd: &Path, data: &[u8]) -> Result<String> {
    let mut child = Command::new(scan::git_program()?)
        .args(["hash-object", "--stdin"])
        .current_dir(cwd)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()?;
    child
        .stdin
        .take()
        .context("Git input unavailable")?
        .write_all(data)?;
    let out = child.wait_with_output()?;
    if !out.status.success() {
        bail!("could not identify review snapshot");
    }
    Ok(String::from_utf8(out.stdout)?.trim().to_owned())
}

fn fields(bytes: &[u8]) -> Result<Vec<&str>> {
    bytes
        .split(|b| *b == 0)
        .filter(|b| !b.is_empty())
        .map(|b| std::str::from_utf8(b).context("review requires UTF-8 Git paths"))
        .collect()
}

pub fn collect(cwd: &Path, config_path: Option<&Path>, strict: bool) -> Result<Review> {
    let before = git(cwd, &["ls-files", "--stage", "-z"])?;
    let head = Command::new(scan::git_program()?)
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(cwd)
        .output()?;
    let mut entries = BTreeMap::new();
    for entry in fields(&before)? {
        let (meta, path) = entry.split_once('\t').context("invalid index entry")?;
        let parts: Vec<_> = meta.split_whitespace().collect();
        if parts.len() != 3 || parts[2] != "0" {
            bail!("resolve index conflicts before creating a review");
        }
        entries.insert(path.to_owned(), (parts[0].to_owned(), parts[1].to_owned()));
    }
    let names = git(
        cwd,
        &["diff", "--cached", "--name-status", "--find-renames", "-z"],
    )?;
    let names = fields(&names)?;
    let mut paths = Vec::new();
    let mut originals = BTreeMap::new();
    let mut i = 0;
    while i < names.len() {
        let status = names[i];
        i += 1;
        let old = *names.get(i).context("invalid changed-file list")?;
        i += 1;
        let path = if status.starts_with('R') || status.starts_with('C') {
            let new = *names.get(i).context("invalid rename entry")?;
            i += 1;
            originals.insert(new.to_owned(), old.to_owned());
            new
        } else {
            old
        };
        if !status.starts_with('D') {
            paths.push(path.to_owned());
        }
    }
    let mut loaded = config::load(cwd, config_path)?;
    if strict {
        loaded.config.apply_strict();
    }
    let words = watchlist::load(&loaded.config, cwd, &process_env())?;
    let patterns = patterns::build(&loaded.config)?;
    let mut contents = BTreeMap::new();
    for path in &paths {
        let (mode, oid) = entries
            .get(path)
            .context("changed file missing from snapshot")?;
        let size = git(cwd, &["cat-file", "-s", oid])?;
        let size: u64 = std::str::from_utf8(&size)?.trim().parse()?;
        let content = if mode != "100644" && mode != "100755" {
            ContentLoad::CoverageSkip("non-regular staged entry")
        } else if size
            > loaded
                .config
                .max_file_size
                .map_or(REVIEW_BLOB_LIMIT, |max| max.min(REVIEW_BLOB_LIMIT))
        {
            ContentLoad::CoverageSkip("oversize staged blob")
        } else {
            match String::from_utf8(git(cwd, &["cat-file", "blob", oid])?) {
                Ok(text) => ContentLoad::Text(text),
                Err(_) => ContentLoad::CoverageSkip("non-UTF-8 staged blob"),
            }
        };
        contents.insert(path.clone(), content);
    }
    let mut warnings = loaded.warnings;
    warnings.extend(words.warnings);
    let result = scan::scan_snapshot(
        paths,
        cwd,
        &loaded.config,
        &words.matcher,
        &patterns,
        warnings,
        &contents,
    )?;
    let re = regex::Regex::new(r"(?m)^@@ -\d+(?:,\d+)? \+(\d+)(?:,(\d+))? @@")?;
    let mut ranges = BTreeMap::new();
    for hit in &result.hits {
        if ranges.contains_key(&hit.file) {
            continue;
        }
        let mut args = vec![
            "diff",
            "--cached",
            "--no-ext-diff",
            "--no-textconv",
            "--find-renames",
            "--unified=0",
            "--",
        ];
        if let Some(old) = originals.get(&hit.file) {
            args.push(old);
        }
        args.push(&hit.file);
        let diff = git(cwd, &args).ok().and_then(|b| String::from_utf8(b).ok());
        let parsed = diff.map(|diff| {
            re.captures_iter(&diff)
                .filter_map(|c| {
                    let start = c[1].parse::<usize>().ok()?;
                    let count = c
                        .get(2)
                        .map_or(Some(1), |m| m.as_str().parse::<usize>().ok())?;
                    Some(start..start.saturating_add(count))
                })
                .collect::<Vec<_>>()
        });
        ranges.insert(hit.file.clone(), parsed);
    }
    let findings = result
        .hits
        .into_iter()
        .map(|hit| {
            let change = match ranges.get(&hit.file).and_then(Option::as_ref) {
                Some(ranges) if ranges.iter().any(|r| r.contains(&hit.line_number)) => "added",
                Some(_) => "existing",
                None => "unknown",
            }
            .to_owned();
            let context = match contents.get(&hit.file) {
                Some(ContentLoad::Text(text)) => text
                    .lines()
                    .enumerate()
                    .filter(|(n, _)| (n + 1).abs_diff(hit.line_number) <= 2)
                    .map(|(n, line)| {
                        format!("{}: {}", n + 1, line.chars().take(2000).collect::<String>())
                    })
                    .collect::<Vec<_>>()
                    .join("\n"),
                _ => String::new(),
            };
            Finding {
                hit,
                change,
                context,
            }
        })
        .collect();
    let root = String::from_utf8(git(cwd, &["rev-parse", "--show-toplevel"])?)?;
    let mut review = Review {
        version: 1,
        repository: root.trim().to_owned(),
        snapshot: String::new(),
        generated_at: SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs(),
        scanned: result.scanned,
        coverage_skips: result.coverage_skips,
        warning_count: result.warnings.len(),
        warnings: result.warnings,
        findings,
    };
    let mut identity = before.clone();
    identity.extend_from_slice(&head.stdout);
    // Includes repository, config-dependent findings and context, but not generation time.
    review.generated_at = 0;
    identity.extend(serde_json::to_vec(&review)?);
    review.snapshot = hash(cwd, &identity)?;
    review.generated_at = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    let after_head = Command::new(scan::git_program()?)
        .args(["rev-parse", "--verify", "HEAD"])
        .current_dir(cwd)
        .output()?;
    if before != git(cwd, &["ls-files", "--stage", "-z"])? || head.stdout != after_head.stdout {
        bail!("index or HEAD changed during review; retry");
    }
    Ok(review)
}

fn reject_link(path: &Path) -> Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_symlink() {
        bail!("review output may not follow links");
    }
    #[cfg(windows)]
    {
        use std::os::windows::fs::MetadataExt;
        if meta.file_attributes() & 0x400 != 0 {
            bail!("review output may not follow reparse points");
        }
    }
    if !meta.is_dir() {
        bail!("review output parent is not a directory");
    }
    Ok(())
}

/// Fixed ignored location; never overwrites an existing report or changes ignore rules.
pub fn destination(cwd: &Path) -> Result<PathBuf> {
    let root = String::from_utf8(git(cwd, &["rev-parse", "--show-toplevel"])?)?;
    let root = PathBuf::from(root.trim());
    // Require the directory itself to be ignored, not just a single probe filename.
    let probe = ".doxguard/reports/";
    let tracked = git(&root, &["ls-files", "-z", "--", ".doxguard/reports"])?;
    if !tracked.is_empty() {
        bail!("review output directory contains tracked files");
    }
    let ignored = Command::new(scan::git_program()?)
        .args(["check-ignore", "--quiet", "--no-index", "--", probe])
        .current_dir(&root)
        .status()?;
    if !ignored.success() {
        bail!("add /.doxguard/reports/ to .gitignore before creating a review");
    }
    reject_link(&root)?;
    let mut dir = root;
    for part in [".doxguard", "reports"] {
        dir.push(part);
        match fs::symlink_metadata(&dir) {
            Ok(_) => reject_link(&dir)?,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                let builder = fs::DirBuilder::new();
                #[cfg(unix)]
                let builder = {
                    use std::os::unix::fs::DirBuilderExt;
                    let mut builder = builder;
                    builder.mode(0o700);
                    builder
                };
                builder.create(&dir)?;
                reject_link(&dir)?;
            }
            Err(e) => return Err(e.into()),
        }
    }
    Ok(dir)
}

pub fn write(dir: &Path, review: &Review) -> Result<PathBuf> {
    if review.snapshot.len() < 12 || !review.snapshot.bytes().all(|b| b.is_ascii_hexdigit()) {
        bail!("invalid review snapshot identifier");
    }
    reject_link(dir)?;
    let root = dir
        .parent()
        .and_then(Path::parent)
        .context("invalid output location")?;
    if destination(root)? != dir {
        bail!("invalid output location");
    }
    // HTML is first serialized in memory; create_new reserves a unique name and refuses links.
    let html = render(review)?;
    let name = format!(
        "review-{}-{}-{}.html",
        review.generated_at,
        &review.snapshot[..12],
        std::process::id()
    );
    let path = dir.join(name);
    let mut opts = fs::OpenOptions::new();
    opts.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
    }
    let temporary = path.with_extension("html.partial");
    let mut file = opts
        .open(&temporary)
        .context("could not create private review file")?;
    if let Err(e) = file
        .write_all(html.as_bytes())
        .and_then(|_| file.sync_all())
    {
        drop(file);
        let _ = fs::remove_file(&temporary);
        return Err(e.into());
    }
    drop(file);
    // Publishing the complete inode via hard_link is atomic and does not replace an existing path.
    let published = fs::hard_link(&temporary, &path);
    let _ = fs::remove_file(&temporary);
    published.context("could not publish review; existing output is never overwritten")?;
    Ok(path)
}

pub fn render(review: &Review) -> Result<String> {
    let json = serde_json::to_string(review)?
        .replace('&', "\\u0026")
        .replace('<', "\\u003c")
        .replace('>', "\\u003e");
    Ok(include_str!("review.html").replace("/*REVIEW_DATA*/null", &json))
}
