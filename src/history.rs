//! Bounded, read-only scanning of objects reachable from refs (not reflogs/dangling objects).
use super::*;
use std::{
    io::{Read, Write},
    time::{Duration, Instant},
};

const OUTPUT_LIMIT: u64 = 32 * 1024 * 1024;
const OBJECT_LIMIT: usize = 250_000;
const TREE_LIMIT: usize = 10_000;
const HIT_LIMIT: usize = 100_000;
const HISTORY_SECONDS: u64 = 120;
const TOTAL_IO_LIMIT: u64 = 256 * 1024 * 1024;

fn git_bytes(
    cwd: &Path,
    args: &[&str],
    input: Option<Vec<u8>>,
    limit: u64,
    started: Instant,
) -> Result<Vec<u8>> {
    let remaining = Duration::from_secs(HISTORY_SECONDS)
        .checked_sub(started.elapsed())
        .ok_or_else(|| anyhow!("history scan exceeded 120 second budget"))?;
    let command_started = Instant::now();
    let command_budget = remaining.min(Duration::from_secs(30));
    let mut child = Command::new(git_program()?)
        .current_dir(cwd)
        .args(["-c", "core.quotepath=false", "--no-replace-objects"])
        .args(args)
        .env_remove("GIT_DIFF_OPTS")
        .env_remove("GIT_SHALLOW_FILE")
        .env("GIT_NO_LAZY_FETCH", "1")
        .stdin(if input.is_some() {
            Stdio::piped()
        } else {
            Stdio::null()
        })
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|_| anyhow!("history Git command could not start"))?;
    let writer = input.map(|bytes| {
        let mut stdin = child.stdin.take().expect("piped input");
        std::thread::spawn(move || stdin.write_all(&bytes))
    });
    let stdout = child.stdout.take().expect("piped output");
    let (send, receive) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        let mut bytes = Vec::new();
        let result = stdout
            .take(limit + 1)
            .read_to_end(&mut bytes)
            .map(|_| bytes);
        let _ = send.send(result);
    });
    let result = receive.recv_timeout(command_budget);
    let bytes = match result {
        Ok(Ok(bytes)) if bytes.len() as u64 <= limit => bytes,
        _ => {
            let _ = child.kill();
            let _ = child.wait();
            let _ = reader.join();
            if let Some(writer) = writer {
                let _ = writer.join();
            }
            bail!("history Git output failed, exceeded its limit, or timed out");
        }
    };
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .map_err(|_| anyhow!("history Git command failed"))?
        {
            break status;
        }
        if command_started.elapsed() >= command_budget {
            let _ = child.kill();
            let _ = child.wait();
            bail!("history Git command timed out after output closed");
        }
        std::thread::sleep(Duration::from_millis(10));
    };
    let _ = reader.join();
    if let Some(writer) = writer {
        if !matches!(writer.join(), Ok(Ok(()))) {
            bail!("history object request failed");
        }
    }
    if !status.success() {
        bail!("history Git command failed");
    }
    Ok(bytes)
}

fn oid_valid(oid: &str) -> bool {
    matches!(oid.len(), 40 | 64) && oid.bytes().all(|byte| byte.is_ascii_hexdigit())
}

pub fn scan_history(
    cwd: &Path,
    config: &Config,
    matcher: &WatchlistMatcher,
    patterns: &[StructuralPattern],
    mut warnings: Vec<String>,
) -> Result<ScanResult> {
    let started = Instant::now();
    if std::env::var_os("GIT_GRAFT_FILE").is_some() {
        bail!("history refuses custom graft files");
    }
    let graft_path = git_bytes(
        cwd,
        &["rev-parse", "--git-path", "info/grafts"],
        None,
        OUTPUT_LIMIT,
        started,
    )?;
    let graft_path = std::str::from_utf8(&graft_path)
        .map_err(|_| anyhow!("invalid Git graft metadata"))?
        .trim();
    if fs::symlink_metadata(cwd.join(graft_path)).is_ok() {
        bail!("history refuses grafted repositories");
    }
    let config_keys = git_bytes(
        cwd,
        &["config", "--local", "--name-only", "--list"],
        None,
        OUTPUT_LIMIT,
        started,
    )?;
    let config_keys =
        std::str::from_utf8(&config_keys).map_err(|_| anyhow!("invalid repository config keys"))?;
    if config_keys.lines().any(|key| {
        let key = key.to_ascii_lowercase();
        key == "extensions.partialclone"
            || (key.starts_with("remote.")
                && (key.ends_with(".promisor") || key.ends_with(".partialclonefilter")))
    }) {
        bail!(
            "history requires a complete local object database; partial/promisor repositories are refused"
        );
    }
    let shallow = git_bytes(
        cwd,
        &["rev-parse", "--is-shallow-repository"],
        None,
        64,
        started,
    )?;
    if shallow != b"false\n" {
        bail!("history requires a complete repository; shallow clones are refused");
    }
    let objects = git_bytes(
        cwd,
        &["rev-list", "--objects", "--all", "--no-object-names"],
        None,
        OUTPUT_LIMIT,
        started,
    )?;
    let text = std::str::from_utf8(&objects)
        .map_err(|_| anyhow!("invalid reachable object identifiers"))?;
    if text.lines().count() > OBJECT_LIMIT {
        bail!("history exceeds 250000 reachable objects");
    }
    if text.lines().any(|oid| !oid_valid(oid)) {
        bail!("invalid reachable object identifier");
    }
    let checked = git_bytes(
        cwd,
        &["cat-file", "--batch-check"],
        Some(objects),
        OUTPUT_LIMIT,
        started,
    )?;
    let checked =
        std::str::from_utf8(&checked).map_err(|_| anyhow!("invalid history object metadata"))?;
    let mut blobs = HashMap::new();
    let mut trees = Vec::new();
    let mut tree_objects = HashSet::new();
    for line in checked.lines() {
        let parts: Vec<_> = line.split_whitespace().collect();
        if parts.len() != 3 || !oid_valid(parts[0]) {
            bail!("invalid history object metadata");
        }
        let size: u64 = parts[2]
            .parse()
            .map_err(|_| anyhow!("invalid history object size"))?;
        match parts[1] {
            "blob" => {
                blobs.insert(parts[0].to_owned(), size);
            }
            "commit" => trees.push(parts[0].to_owned()),
            "tree" => {
                tree_objects.insert(parts[0].to_owned());
            }
            "tag" => {}
            _ => bail!("unsupported reachable object type"),
        }
    }
    let refs = git_bytes(
        cwd,
        &["for-each-ref", "--format=%(objectname)"],
        None,
        OUTPUT_LIMIT,
        started,
    )?;
    let refs = std::str::from_utf8(&refs).map_err(|_| anyhow!("invalid history ref metadata"))?;
    if refs.lines().count() > TREE_LIMIT {
        bail!("history exceeds 10000 refs");
    }
    for reference in refs.lines() {
        if !oid_valid(reference) {
            bail!("invalid history ref identifier");
        }
        let peeled = git_bytes(
            cwd,
            &["rev-parse", &format!("{reference}^{{}}")],
            None,
            128,
            started,
        )?;
        let peeled = std::str::from_utf8(&peeled)
            .map_err(|_| anyhow!("invalid peeled history ref"))?
            .trim();
        if tree_objects.contains(peeled) {
            trees.push(peeled.to_owned());
        }
    }
    trees.sort();
    trees.dedup();
    if trees.len() > TREE_LIMIT {
        bail!("history exceeds 10000 reachable commits or root trees");
    }
    let mut targets = HashSet::new();
    let mut named = HashSet::new();
    let mut tree_bytes = 0u64;
    let mut path_bytes = 0usize;
    for tree in trees {
        let listing = git_bytes(
            cwd,
            &["ls-tree", "-r", "-z", &tree],
            None,
            OUTPUT_LIMIT,
            started,
        )?;
        tree_bytes += listing.len() as u64;
        if tree_bytes > TOTAL_IO_LIMIT {
            bail!("history tree listings exceed 256 MiB cumulative output");
        }
        for entry in listing
            .split(|byte| *byte == 0)
            .filter(|entry| !entry.is_empty())
        {
            let split = entry
                .iter()
                .position(|byte| *byte == b'\t')
                .ok_or_else(|| anyhow!("invalid history tree entry"))?;
            let fields = std::str::from_utf8(&entry[..split])
                .map_err(|_| anyhow!("invalid history tree metadata"))?;
            let fields: Vec<_> = fields.split_whitespace().collect();
            if fields.len() != 3 || !oid_valid(fields[2]) {
                bail!("invalid history tree metadata");
            }
            let path = std::str::from_utf8(&entry[split + 1..])
                .map_err(|_| anyhow!("history filename is not UTF-8"))?;
            if path.is_empty()
                || Path::new(path).is_absolute()
                || path.split('/').any(|part| part == "..")
            {
                bail!("invalid history relative path");
            }
            if fields[1] == "blob" {
                named.insert(fields[2].to_owned());
            }
            if targets.insert((path.to_owned(), fields[2].to_owned(), fields[0].to_owned())) {
                path_bytes += path.len();
            }
            if path_bytes > OUTPUT_LIMIT as usize {
                bail!("history target names exceed 32 MiB");
            }
            if targets.len() > OBJECT_LIMIT {
                bail!("history exceeds 250000 blob/path entries");
            }
        }
    }
    for oid in blobs.keys().filter(|oid| !named.contains(*oid)) {
        targets.insert((
            "__history_unnamed_blob__".to_owned(),
            oid.clone(),
            "100644".to_owned(),
        ));
        path_bytes += "__history_unnamed_blob__".len();
        if path_bytes > OUTPUT_LIMIT as usize {
            bail!("history target names exceed 32 MiB");
        }
    }
    if targets.len() > OBJECT_LIMIT {
        bail!("history exceeds 250000 blob/path entries");
    }
    let total_files = targets.len();
    let mut targets: Vec<_> = targets.into_iter().collect();
    targets.sort();
    let mut hits = Vec::new();
    let mut scanned = 0;
    let mut coverage_skips = 0;
    let mut blob_bytes = 0u64;
    for (path, oid, mode) in targets {
        let reason = if mode == "120000" {
            Some("history symlink")
        } else if mode == "160000" {
            Some("history gitlink is not inspectable")
        } else {
            None
        };
        if let Some(reason) = reason {
            coverage_skips += 1;
            warnings.push(format!("WARN: skipped {path} (blob {oid}; {reason})"));
            continue;
        }
        let kind = if !named.contains(&oid) {
            ScanKind::Full
        } else {
            match classify_path(&path, cwd, config, ScanMode::Staged) {
                Eligibility::Scan(kind) => kind,
                Eligibility::QuietSkip => continue,
                Eligibility::CoverageSkip(_) => {
                    unreachable!("staged classification has no coverage probe")
                }
            }
        };
        if blobs
            .get(&oid)
            .is_none_or(|size| *size > config.max_file_size.min(STAGED_BLOB_MAX_BYTES))
        {
            coverage_skips += 1;
            warnings.push(format!(
                "WARN: skipped {path} (blob {oid}; oversize or unavailable history blob)"
            ));
            continue;
        }
        let content = git_bytes(
            cwd,
            &["cat-file", "blob", &oid],
            None,
            config.max_file_size.min(STAGED_BLOB_MAX_BYTES),
            started,
        )?;
        blob_bytes += content.len() as u64;
        if blob_bytes > TOTAL_IO_LIMIT {
            bail!("history blob reads exceed 256 MiB");
        }
        let content = match String::from_utf8(content) {
            Ok(content) => content,
            Err(_) => {
                coverage_skips += 1;
                warnings.push(format!(
                    "WARN: skipped {path} (blob {oid}; non-UTF-8 history content)"
                ));
                continue;
            }
        };
        scanned += 1;
        for (index, line) in content.lines().enumerate() {
            if kind.wants_watchlist() {
                let haystack = if config.noise.ascii_case_insensitive {
                    line.to_ascii_lowercase()
                } else {
                    line.to_owned()
                };
                for (item, span) in matcher
                    .matches_spanned_with_boundary(&haystack, config.noise.short_needle_max_length)
                {
                    if allowed_by_directive(line, &item.needle, config) {
                        continue;
                    }
                    if hits.len() >= HIT_LIMIT {
                        bail!("history exceeds 100000 findings");
                    }
                    hits.push(ScanHit {
                        blob_oid: Some(oid.clone()),
                        file: path.clone(),
                        line_number: index + 1,
                        matched: line[span].to_owned(),
                        source: item.source.clone(),
                        kind: HitKind::Watchlist,
                        suggestion: "Generalize or remove the watchlist-derived value".to_owned(),
                    });
                }
            }
            if kind.wants_structural() {
                for pattern in patterns {
                    let mut seen = HashSet::new();
                    for matched in pattern.regex.find_iter(line).map(|found| found.as_str()) {
                        if !seen.insert(matched)
                            || pattern.is_allowed(matched, config)
                            || allowed_by_directive(line, matched, config)
                        {
                            continue;
                        }
                        if hits.len() >= HIT_LIMIT {
                            bail!("history exceeds 100000 findings");
                        }
                        hits.push(ScanHit {
                            blob_oid: Some(oid.clone()),
                            file: path.clone(),
                            line_number: index + 1,
                            matched: matched.to_owned(),
                            source: format!("structural: {}", pattern.name),
                            kind: HitKind::Structural,
                            suggestion: pattern.suggestion.clone(),
                        });
                    }
                }
            }
            if hits.len() > HIT_LIMIT {
                bail!("history exceeds 100000 findings; narrow or clean the repository");
            }
            if started.elapsed() > Duration::from_secs(HISTORY_SECONDS) {
                bail!("history scan exceeded 120 second budget");
            }
        }
    }
    Ok(ScanResult {
        watchlist_statistics: matcher.statistics().clone(),
        baseline_applied: false,
        baseline_suppressed_hits: 0,
        mode: ScanMode::History,
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
