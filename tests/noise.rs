use doxguard::{
    config::{ColumnSpec, Config, WatchlistSource},
    watchlist,
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(output.status.success(), "fixture Git failed");
}
fn init(cwd: &Path) {
    git(cwd, &["init", "-q"]);
    git(cwd, &["config", "user.name", "Fixture Author"]);
    git(cwd, &["config", "user.email", "fixture@example.com"]);
}
fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .args(args)
        .current_dir(cwd)
        .env_remove("DOXGUARD_CONFIG")
        .env_remove("DOXGUARD_SHOW_PATHS")
        .env_remove("DOXGUARD_WATCHLIST_DIR")
        .env("GIT_DIFF_OPTS", "-U100")
        .output()
        .unwrap()
}
fn report(cwd: &Path, mode: &str) -> (i32, Value) {
    let output = run(cwd, &["scan", mode, "--block", "--format", "json"]);
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}
fn baseline_config(cwd: &Path) {
    fs::write(
        cwd.join("doxguard.config.json"),
        json!({"noise":{"stagedAddedLinesOnly":true}}).to_string(),
    )
    .unwrap();
}

#[test]
fn boundary_checks_precede_dedup_and_use_unicode_lengths() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("terms.txt"), "QZ\n山森\n").unwrap();
    let mut config = Config::default();
    config.watchlists.push(WatchlistSource::Lines {
        path: "terms.txt".into(),
        optional: false,
        label: None,
    });
    let loaded = watchlist::load(&config, temp.path(), &HashMap::new()).unwrap();
    let matcher = loaded.matcher;
    assert_eq!(
        matcher.matches_spanned_with_boundary("xQZ9 QZ", 2).count(),
        1
    );
    let span = matcher
        .matches_spanned_with_boundary("xQZ9 QZ", 2)
        .next()
        .unwrap()
        .1;
    assert_eq!(span, 5..7);
    assert_eq!(matcher.matches_spanned_with_boundary("xQZ9", 0).count(), 1);
    assert_eq!(matcher.matches_spanned_with_boundary("xQZ9", 1).count(), 1);
    assert_eq!(
        matcher.matches_spanned_with_boundary("x山森9", 2).count(),
        0
    );
    assert_eq!(
        matcher.matches_spanned_with_boundary("（山森）", 2).count(),
        1
    );
    assert_eq!(matcher.matches_spanned_with_boundary("QZ", 2).count(), 1);
    assert_eq!(
        matcher.matches_spanned_with_boundary("あQZい", 2).count(),
        1
    );
}

#[test]
fn variant_candidates_have_exclusive_reason_counts() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("terms.csv"),
        "given_name\nA\nかな\nPublic\nAcme（Fixture）\nAcme\n",
    )
    .unwrap();
    let mut config = Config::default();
    config.allow.names.push("Public".into());
    config.watchlists.push(WatchlistSource::Csv {
        path: "terms.csv".into(),
        optional: false,
        label: None,
        column: Some(ColumnSpec::Name("given_name".into())),
        columns: None,
        paren_variants: true,
    });
    let matcher = watchlist::load(&config, temp.path(), &HashMap::new())
        .unwrap()
        .matcher;
    let stats = matcher.statistics();
    assert_eq!(
        (
            stats.candidates,
            stats.too_short,
            stats.short_kana,
            stats.allow_listed,
            stats.duplicate,
            stats.loaded
        ),
        (6, 1, 1, 1, 1, 2)
    );
    assert_eq!(
        stats.candidates,
        stats.too_short + stats.short_kana + stats.allow_listed + stats.duplicate + stats.loaded
    );
}

#[test]
fn baseline_uses_index_additions_and_other_modes_keep_all_hits() {
    let temp = tempdir().unwrap();
    init(temp.path());
    baseline_config(temp.path());
    let email = ["fixture_private", "sample.test"].join("@");
    fs::write(temp.path().join("file.txt"), format!("{email}\nclean\n")).unwrap();
    git(temp.path(), &["add", "file.txt"]);
    git(
        temp.path(),
        &[
            "-c",
            "core.hooksPath=/dev/null",
            "commit",
            "-qm",
            "fixture baseline",
        ],
    );
    fs::write(temp.path().join("file.txt"), format!("{email}\nchanged\n")).unwrap();
    git(temp.path(), &["add", "file.txt"]);
    // Unstaged extra leak must not enter the staged baseline calculation.
    fs::write(
        temp.path().join("file.txt"),
        format!("{email}\nchanged\n{email}\n"),
    )
    .unwrap();
    let (exit, result) = report(temp.path(), "--staged");
    assert_eq!(exit, 0);
    assert_eq!(result["baseline_applied"], true);
    assert_eq!(result["baseline_suppressed_hits"], 1);
    assert_eq!(result["hits"].as_array().unwrap().len(), 0);
    for mode in ["--all-tracked", "--diff"] {
        let (exit, result) = report(temp.path(), mode);
        assert_eq!(exit, 1);
        assert_eq!(result["baseline_applied"], false);
        assert_eq!(result["hits"].as_array().unwrap().len(), 2);
    }
    // HEAD containing this same value does not excuse a newly added file.
    fs::write(temp.path().join("日本語 file.txt"), format!("{email}\n")).unwrap();
    git(temp.path(), &["add", "日本語 file.txt"]);
    let (exit, result) = report(temp.path(), "--staged");
    assert_eq!(exit, 1);
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    assert_eq!(result["hits"][0]["file"], "日本語 file.txt");
}

#[test]
fn initial_commit_and_binary_git_attributes_do_not_suppress_new_hits() {
    let temp = tempdir().unwrap();
    init(temp.path());
    baseline_config(temp.path());
    git(temp.path(), &["config", "color.ui", "always"]);
    fs::write(temp.path().join(".gitattributes"), "*.txt binary\n").unwrap();
    fs::write(
        temp.path().join("file.txt"),
        "fixture_private@sample.test\n",
    )
    .unwrap();
    git(temp.path(), &["add", "file.txt", ".gitattributes"]);
    let (exit, result) = report(temp.path(), "--staged");
    assert_eq!(exit, 1);
    assert_eq!(result["baseline_suppressed_hits"], 0);
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
}

#[test]
fn cli_boundary_statistics_and_legacy_allows_follow_one_rule() {
    let temp = tempdir().unwrap();
    let lists = tempdir().unwrap();
    init(temp.path());
    fs::write(lists.path().join("terms.txt"), "QZ\nAcme Labs\n").unwrap();
    fs::write(
        temp.path().join("doxguard.config.json"),
        json!({
            "watchlists":[{"type":"lines","path":lists.path().join("terms.txt")}],
            "noise":{"shortNeedleMaxLength":2,"asciiCaseInsensitive":true}
        })
        .to_string(),
    )
    .unwrap();
    fs::write(temp.path().join("file.txt"), "xqz9 QZ\nAcme Labs # secrets-scan: allow Acme\nAcme Labs # doxguard: allow Acme\nAcme Labs # doxguard: allow Acme-Labs-Long\n").unwrap();
    git(temp.path(), &["add", "file.txt"]);
    let (exit, result) = report(temp.path(), "--staged");
    assert_eq!(exit, 1);
    assert_eq!(result["hits"].as_array().unwrap().len(), 2);
    assert_eq!(result["watchlist_statistics"]["loaded"], 2);
    assert_eq!(result["hits"][0]["matched"], "[REDACTED]");
    let text = run(temp.path(), &["scan", "--staged", "--block"]);
    let stderr = String::from_utf8(text.stderr).unwrap();
    assert!(stderr.contains("Watchlist candidates: 2; loaded: 2"));
    assert!(stderr.contains("Baseline applied: false"));
}

#[test]
fn baseline_git_failure_stops_and_external_diff_is_disabled() {
    let temp = tempdir().unwrap();
    init(temp.path());
    baseline_config(temp.path());
    fs::write(
        temp.path().join("file.txt"),
        "fixture_private@sample.test\n",
    )
    .unwrap();
    git(temp.path(), &["add", "file.txt"]);
    git(
        temp.path(),
        &[
            "config",
            "diff.external",
            "fixture_missing_external_command",
        ],
    );
    let (exit, result) = report(temp.path(), "--staged");
    assert_eq!(exit, 1);
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    git(
        temp.path(),
        &["config", "diff.algorithm", "fixture_invalid_algorithm"],
    );
    let output = run(
        temp.path(),
        &["scan", "--staged", "--block", "--format", "json"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(output.stdout.is_empty());
    let error = String::from_utf8(output.stderr).unwrap();
    assert!(
        error.contains("diff.algorithm") || error.contains("cannot determine staged baseline"),
        "{error}"
    );
    assert!(!error.contains("fixture_private"));
}
