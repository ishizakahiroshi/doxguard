//! Large text files are read to the end instead of being coverage-skipped.
//! Every repository, file and value here is synthetic.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

const MIB: usize = 1024 * 1024;
const SYNTHETIC_IP: &str = "192.168.50.9";

fn command(program: &str, cwd: &Path) -> Command {
    let mut command = Command::new(program);
    command.current_dir(cwd);
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if name.starts_with("GIT_") || name.starts_with("DOXGUARD_") {
            command.env_remove(key);
        }
    }
    command
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .env("GIT_CONFIG_GLOBAL", cwd.join("absent-global.gitconfig"))
        .env("GIT_CONFIG_SYSTEM", cwd.join("absent-system.gitconfig"));
    command
}

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = command("git", cwd).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap()
}

fn repo(cwd: &Path, config: &str) {
    git(cwd, &["init", "-q"]);
    git(cwd, &["config", "user.name", "Synthetic Fixture"]);
    git(cwd, &["config", "user.email", "fixture@example.com"]);
    git(cwd, &["config", "core.autocrlf", "false"]);
    fs::write(cwd.join("doxguard.config.json"), config).unwrap();
    git(
        cwd,
        &["commit", "--allow-empty", "-qm", "synthetic baseline"],
    );
}

/// Lines of filler text totalling at least `bytes`, so the last line lands
/// beyond the size that used to be skipped.
fn filler(bytes: usize) -> String {
    let line = "synthetic filler text for large file coverage 0123456789abcdef\n";
    line.repeat(bytes / line.len() + 1)
}

fn scan(cwd: &Path, args: &[&str]) -> (Option<i32>, serde_json::Value) {
    let output: Output = command(env!("CARGO_BIN_EXE_doxguard"), cwd)
        .args(["scan"])
        .args(args)
        .args(["--block", "--strict", "--format", "json"])
        .output()
        .unwrap();
    let report = serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    });
    (output.status.code(), report)
}

fn counts(report: &serde_json::Value) -> (u64, u64, usize) {
    (
        report["scanned"].as_u64().unwrap(),
        report["coverage_skips"].as_u64().unwrap(),
        report["hits"].as_array().unwrap().len(),
    )
}

#[test]
fn staged_text_above_the_old_default_limit_is_read_to_the_last_line() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    let body = format!("{}host = {SYNTHETIC_IP}\n", filler(MIB + 4096));
    fs::write(cwd.join("inventory.json"), &body).unwrap();
    // One long line with the value at its very end.
    let long_line = format!("{} {SYNTHETIC_IP}", "x".repeat(2 * MIB));
    fs::write(cwd.join("minified.js"), long_line).unwrap();
    git(cwd, &["add", "--", "inventory.json", "minified.js"]);

    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (2, 0, 2), "{report}");
    let last_line = body.lines().count();
    assert!(
        report["hits"]
            .as_array()
            .unwrap()
            .iter()
            .any(|hit| hit["file"] == "inventory.json" && hit["line_number"] == last_line)
    );
}

#[test]
fn staged_text_above_the_old_sixteen_mib_ceiling_is_read_to_the_end() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    fs::write(
        cwd.join("cases.json"),
        format!("{}address: {SYNTHETIC_IP}\n", filler(16 * MIB + 4096)),
    )
    .unwrap();
    git(cwd, &["add", "--", "cases.json"]);

    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (1, 0, 1), "{report}");
}

#[test]
fn clean_staged_text_above_the_old_ceiling_passes_the_strict_gate() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    fs::write(cwd.join("index.json"), filler(MIB + 4096)).unwrap();
    fs::write(cwd.join("cases.json"), filler(16 * MIB + 4096)).unwrap();
    git(cwd, &["add", "--", "index.json", "cases.json"]);

    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(0), "{report}");
    assert_eq!(counts(&report), (2, 0, 0), "{report}");
}

#[test]
fn worktree_and_path_modes_read_large_text_to_the_end() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    fs::write(
        cwd.join("big.txt"),
        format!("{}server={SYNTHETIC_IP}\r\n", filler(MIB + 4096)),
    )
    .unwrap();
    git(cwd, &["add", "--", "big.txt"]);
    git(cwd, &["commit", "-qm", "synthetic large file"]);

    for args in [
        &["--all-tracked"][..],
        &["--path", "big.txt"][..],
        &["--files-from-list", "list.txt"][..],
    ] {
        fs::write(cwd.join("list.txt"), "big.txt\n").unwrap();
        let (code, report) = scan(cwd, args);
        assert_eq!(code, Some(1), "{args:?}: {report}");
        let (scanned, skips, hits) = counts(&report);
        assert_eq!((skips, hits), (0, 1), "{args:?}: {report}");
        assert!(scanned >= 1, "{args:?}: {report}");
    }
}

#[test]
fn explicit_max_file_size_still_skips_and_has_no_staged_ceiling() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, &format!(r#"{{"maxFileSize": {}}}"#, MIB));
    fs::write(
        cwd.join("big.txt"),
        format!("{}{SYNTHETIC_IP}\n", filler(MIB + 4096)),
    )
    .unwrap();
    git(cwd, &["add", "--", "big.txt"]);
    let (code, report) = scan(cwd, &["--staged"]);
    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (0, 1, 0), "{report}");

    // A configured limit above 16 MiB is honoured instead of being capped.
    fs::write(
        cwd.join("doxguard.config.json"),
        format!(r#"{{"maxFileSize": {}}}"#, 20 * MIB),
    )
    .unwrap();
    fs::write(
        cwd.join("big.txt"),
        format!("{}{SYNTHETIC_IP}\n", filler(16 * MIB + 4096)),
    )
    .unwrap();
    git(cwd, &["add", "--", "big.txt"]);
    let (code, report) = scan(cwd, &["--staged"]);
    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (1, 0, 1), "{report}");
}

#[test]
fn large_non_utf8_staged_text_remains_a_coverage_skip() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    let mut bytes = format!("{SYNTHETIC_IP}\n{}", filler(MIB + 4096)).into_bytes();
    bytes.extend_from_slice(&[0xff, 0xfe, b'\n']);
    fs::write(cwd.join("mixed.txt"), bytes).unwrap();
    git(cwd, &["add", "--", "mixed.txt"]);

    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (0, 1, 0), "{report}");
}

#[test]
fn a_long_line_with_many_matches_is_scanned_in_linear_time() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // Before the per-line directive check this took about a minute.
    let line = "10.1.2.3 ".repeat(2 * MIB / 9 + 1024);
    fs::write(cwd.join("minified.js"), line).unwrap();
    git(cwd, &["add", "--", "minified.js"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (1, 0, 1), "{report}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn many_distinct_matches_are_redacted_in_linear_time() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // 20,000 distinct private addresses (10.0.0.0 to 10.0.78.31), one per line.
    // Before the single-pass redaction this took about 95 s in release.
    let mut body = String::new();
    for index in 0..20_000 {
        body.push_str(&format!("10.0.{}.{}\n", index / 256, index % 256));
    }
    fs::write(cwd.join("inventory.txt"), body).unwrap();
    git(cwd, &["add", "--", "inventory.txt"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--staged"]);

    // The full report is megabytes long; show only the warnings on failure.
    assert_eq!(code, Some(1), "{}", report["warnings"]);
    assert_eq!(counts(&report), (1, 0, 20_000));
    assert!(
        report["hits"]
            .as_array()
            .unwrap()
            .iter()
            .all(|hit| hit["matched"] == "[redacted]")
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn many_distinct_keyed_matches_on_one_line_are_reported_in_linear_time() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // 20,000 distinct `kN = 10.0.x.y` pairs on a single line, so every hit
    // also looks up its key. Before the per-line key search and the
    // single-pass redaction, this fixture took about 165 s in release.
    let mut line = String::new();
    for index in 0..20_000 {
        line.push_str(&format!("k{index} = 10.0.{}.{} ", index / 256, index % 256));
    }
    fs::write(cwd.join("minified.cfg"), line).unwrap();
    git(cwd, &["add", "--", "minified.cfg"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--staged"]);

    // The full report is megabytes long; show only the warnings on failure.
    assert_eq!(code, Some(1), "{}", report["warnings"]);
    assert_eq!(counts(&report), (1, 0, 20_000));
    let hits = report["hits"].as_array().unwrap();
    assert!(hits.iter().all(|hit| hit["matched"] == "[redacted]"));
    let variables: std::collections::HashSet<_> = hits
        .iter()
        .map(|hit| hit["variable"].as_str().unwrap())
        .collect();
    assert_eq!(variables.len(), 20_000);
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn one_very_long_matched_value_is_redacted_in_linear_time() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // One home path of 8 MiB. When every matched value went into the
    // redaction automaton, this fixture took about 80 s in debug; a random
    // 4 MiB home path needed about 2.2 GB of memory in release.
    let segment = "synthetic_cache-dir.";
    let line = format!(
        "cache_dir = /home/{}/x\n",
        segment.repeat(8 * MIB / segment.len())
    );
    fs::write(cwd.join("settings.cfg"), line).unwrap();
    git(cwd, &["add", "--", "settings.cfg"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (1, 0, 1), "{report}");
    let hit = &report["hits"][0];
    assert_eq!(hit["matched"], "[redacted]", "{report}");
    assert_eq!(hit["variable"], "cache_dir", "{report}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn crlf_lines_are_split_exactly_like_str_lines() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    // `$` only matches when the line ending, including `\r`, was removed.
    repo(
        cwd,
        r#"{"structural": {"custom": [{"name": "synthetic eol", "regex": "EOL-TOKEN-[0-9]+$", "suggestion": "synthetic"}]}}"#,
    );
    let body = format!(
        "{}a EOL-TOKEN-1\r\nb EOL-TOKEN-2\r\nc EOL-TOKEN-3",
        "filler\r\n".repeat(MIB / 8)
    );
    let first = body
        .lines()
        .position(|line| line.ends_with("EOL-TOKEN-1"))
        .unwrap()
        + 1;
    fs::write(cwd.join("crlf.txt"), &body).unwrap();
    git(cwd, &["add", "--", "crlf.txt"]);
    let staged = scan(cwd, &["--staged"]).1;
    git(cwd, &["commit", "-qm", "synthetic crlf"]);
    let tracked = scan(cwd, &["--all-tracked"]).1;

    for report in [staged, tracked] {
        let mut lines: Vec<u64> = report["hits"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|hit| hit["source"] == "structural: synthetic eol")
            .map(|hit| hit["line_number"].as_u64().unwrap())
            .collect();
        lines.sort_unstable();
        let first = first as u64;
        assert_eq!(lines, [first, first + 1, first + 2], "{report}");
    }
}

#[test]
fn a_leading_invalid_byte_stops_a_large_staged_read_early() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    let mut bytes = vec![0xff, b'\n'];
    bytes.extend_from_slice(format!("{}{SYNTHETIC_IP}\n", filler(17 * MIB)).as_bytes());
    fs::write(cwd.join("mixed.txt"), bytes).unwrap();
    git(cwd, &["add", "--", "mixed.txt"]);

    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (0, 1, 0), "{report}");
}

#[test]
fn added_line_baseline_reads_large_staged_changes() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, r#"{"noise": {"stagedAddedLinesOnly": true}}"#);
    // Rewriting 33 MiB makes the staged diff larger than the old 64 MiB cap.
    let body = format!("old={SYNTHETIC_IP}\n{}", filler(33 * MIB));
    fs::write(cwd.join("cases.json"), &body).unwrap();
    git(cwd, &["add", "--", "cases.json"]);
    git(cwd, &["commit", "-qm", "synthetic large file"]);
    let mut rewritten = String::with_capacity(body.len());
    for line in body.lines() {
        rewritten.push_str(&line.replace("filler", "FILLER"));
        rewritten.push('\n');
    }
    fs::write(
        cwd.join("cases.json"),
        format!("{rewritten}new={SYNTHETIC_IP}\n"),
    )
    .unwrap();
    git(cwd, &["add", "--", "cases.json"]);

    let (code, report) = scan(cwd, &["--staged"]);

    // The unchanged first line stays suppressed; the appended line is reported.
    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (1, 0, 1), "{report}");
    assert_eq!(report["baseline_suppressed_hits"], 1, "{report}");

    // Only the newest line is new once the rewrite is committed.
    git(cwd, &["commit", "-qm", "synthetic rewrite"]);
    fs::write(
        cwd.join("cases.json"),
        format!("{rewritten}new={SYNTHETIC_IP}\nnewer={SYNTHETIC_IP}\n"),
    )
    .unwrap();
    git(cwd, &["add", "--", "cases.json"]);
    let (code, report) = scan(cwd, &["--staged"]);
    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (1, 0, 1), "{report}");
    assert_eq!(report["baseline_suppressed_hits"], 2, "{report}");
}

/// A distinct private address for every index below 2^24.
fn private_address(index: usize) -> String {
    format!(
        "10.{}.{}.{}",
        index / 65_536 % 256,
        index / 256 % 256,
        index % 256
    )
}

#[test]
fn one_key_followed_by_many_distinct_values_is_reported_in_linear_time() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // `ips = 10.0.0.0 10.0.0.1 ...`: every hit belongs to the same key, and
    // the text between that key and each hit used to be searched again for
    // `;` `,` `{` `}` (100,000 values took about 36 s in release).
    const COUNT: usize = 50_000;
    let mut line = "ips = ".to_owned();
    for index in 0..COUNT {
        line.push_str(&private_address(index));
        line.push(' ');
    }
    fs::write(cwd.join("minified.cfg"), line).unwrap();
    git(cwd, &["add", "--", "minified.cfg"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--staged"]);

    // The full report is megabytes long; show only the warnings on failure.
    assert_eq!(code, Some(1), "{}", report["warnings"]);
    assert_eq!(counts(&report), (1, 0, COUNT));
    assert!(
        report["hits"]
            .as_array()
            .unwrap()
            .iter()
            .all(|hit| hit["variable"] == "ips" && hit["matched"] == "[redacted]")
    );
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn many_hits_inside_one_long_key_name_are_reported_in_linear_time() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // `k-10.0.0.0-10.0.0.1-...=v`: every hit sits inside one long key name,
    // and the key search used to read that name again from its start for
    // every hit (100,000 values took minutes in release). With `$env:` in
    // front, the cut prefix still yields the key `env` for every hit.
    const COUNT: usize = 40_000;
    for (file, prefix) in [("plain.cfg", "k"), ("env.ps1", "$env:k")] {
        let mut line = prefix.to_owned();
        for index in 0..COUNT {
            line.push('-');
            line.push_str(&private_address(index));
        }
        line.push_str("=v");
        fs::write(cwd.join(file), line).unwrap();
    }
    git(cwd, &["add", "--", "plain.cfg", "env.ps1"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{}", report["warnings"]);
    assert_eq!(counts(&report), (2, 0, 2 * COUNT));
    let hits = report["hits"].as_array().unwrap();
    for (file, variable) in [
        ("plain.cfg", serde_json::Value::Null),
        ("env.ps1", "env".into()),
    ] {
        let in_file: Vec<_> = hits.iter().filter(|hit| hit["file"] == file).collect();
        assert_eq!(in_file.len(), COUNT, "{file}");
        assert!(
            in_file.iter().all(|hit| hit["variable"] == variable),
            "{file}"
        );
    }
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn a_long_line_with_many_matches_and_an_allow_directive_is_scanned_in_linear_time() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // The repeated-address line of the linear-time test above, ending in an
    // allow directive. Every match used to parse all of the line's directives
    // again (about 62 s in release for the unrelated one).
    let line = "10.1.2.3 ".repeat(2 * MIB / 9 + 1024);
    for (file, directive) in [
        // A scoped token that is not in the address keeps the hit.
        ("unrelated.js", "# doxguard: allow fixture-unrelated"),
        // A scoped token that is the address allows every match.
        ("scoped.js", "# doxguard: allow 10.1.2.3"),
        // `--strict` refuses the bare form, so the hit stays.
        ("bare.js", "# doxguard: allow"),
    ] {
        fs::write(cwd.join(file), format!("{line}{directive}\n")).unwrap();
    }
    git(cwd, &["add", "--", "unrelated.js", "scoped.js", "bare.js"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--staged"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (3, 0, 2), "{report}");
    let mut files: Vec<_> = report["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["file"].as_str().unwrap().to_owned())
        .collect();
    files.sort();
    assert_eq!(files, ["bare.js", "unrelated.js"], "{report}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}

#[test]
fn history_scan_of_a_long_line_with_many_matches_is_linear() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd, "{}");
    // History checked the allow directives of the whole line for every match,
    // with no per-line shortcut at all. Just under the default 1 MiB blob cap.
    let line = "10.1.2.3 ".repeat(110_000);
    fs::write(cwd.join("minified.js"), format!("{line}\n")).unwrap();
    git(cwd, &["add", "--", "minified.js"]);
    git(cwd, &["commit", "-qm", "synthetic long line"]);

    let started = std::time::Instant::now();
    let (code, report) = scan(cwd, &["--history"]);

    assert_eq!(code, Some(1), "{report}");
    assert_eq!(counts(&report), (1, 0, 1), "{report}");
    assert_eq!(report["mode"], "history", "{report}");
    assert!(
        started.elapsed() < std::time::Duration::from_secs(30),
        "{:?}",
        started.elapsed()
    );
}
