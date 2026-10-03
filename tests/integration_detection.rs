//! Synthetic-only coverage for the integration contract, independent of author lists.
use doxguard::{config::Config, scan, watchlist};
use std::{
    collections::HashMap,
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

fn config(text: &str) -> Config {
    let config: Config = serde_json::from_str(text).unwrap();
    config.validate().unwrap();
    config
}

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .args(args)
            .current_dir(root)
            .output()
            .unwrap()
            .status
            .success()
    );
}

fn run(root: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .args(args)
        .current_dir(root)
        .env_remove("DOXGUARD_CONFIG")
        .env_remove("DOXGUARD_SHOW_PATHS")
        .env_remove("SYNTHETIC_INTEGRATION_ROOT")
        .output()
        .unwrap()
}

#[test]
fn required_sources_optional_skips_and_explicit_structural_ci() {
    let temp = tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    for value in [
        r#"{"watchlists":[{"type":"lines","path":"${SYNTHETIC_INTEGRATION_ROOT}/words.txt"}]}"#,
        r#"{"watchlists":[{"type":"csv","path":"${SYNTHETIC_INTEGRATION_ROOT}/table.csv","column":1}]}"#,
        r#"{"watchlists":[{"type":"directory","path":"${SYNTHETIC_INTEGRATION_ROOT}"}]}"#,
    ] {
        fs::write(temp.path().join("doxguard.config.json"), value).unwrap();
        let output = run(temp.path(), &["scan", "--all-tracked", "--block"]);
        assert_eq!(output.status.code(), Some(2));
    }
    fs::write(temp.path().join("doxguard.config.json"), r#"{"watchlists":[{"type":"lines","path":"${SYNTHETIC_INTEGRATION_ROOT}/words.txt","optional":true}]}"#).unwrap();
    let output = run(temp.path(), &["scan", "--all-tracked", "--block"]);
    assert!(output.status.success());
    assert!(String::from_utf8_lossy(&output.stderr).contains("not set"));
    let ci = temp.path().join("ci.json");
    fs::write(&ci, r#"{"watchlists":[]}"#).unwrap();
    assert!(
        run(
            temp.path(),
            &["scan", "--all-tracked", "--block", "--config", "ci.json"]
        )
        .status
        .success()
    );
    assert!(Config::default().fail_on_skip);
}

#[test]
fn joined_csv_preserves_full_names_even_when_short_given_names_are_noise() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("table.csv"),
        "\u{feff}surname,given_name\nSynthetic,かな\nEmpty,\n,Missing\n",
    )
    .unwrap();
    let cfg = config(
        r#"{"watchlists":[{"type":"csv","path":"table.csv","column":"given_name"},{"type":"csv","path":"table.csv","columns":["surname","given_name"],"label":"fixture given_name"},{"type":"csv","path":"table.csv","columns":[1,2]}]}"#,
    );
    let loaded = watchlist::load(&cfg, temp.path(), &HashMap::new()).unwrap();
    assert!(loaded.matcher.matches("かな").next().is_none());
    assert_eq!(loaded.matcher.matches("Syntheticかな").count(), 1);
    assert!(loaded.matcher.matches("Empty").next().is_none());
    let cfg = config(
        r#"{"watchlists":[{"type":"csv","path":"table.csv","columns":[1,2]}],"noise":{"asciiCaseInsensitive":true},"allow":{"names":["SYNTHETICかな"]}}"#,
    );
    assert!(
        watchlist::load(&cfg, temp.path(), &HashMap::new())
            .unwrap()
            .matcher
            .is_empty()
    );
}

#[test]
fn csv_selection_rejects_invalid_contracts_and_masks_reader_errors() {
    for source in [
        r#"{"column":1,"columns":[]}"#,
        r#"{"column":1,"columns":[2]}"#,
        r#"{"columns":[]}"#,
        r#"{}"#,
        r#"{"columns":[0]}"#,
        r#"{"columns":[""]}"#,
    ] {
        let mut source: serde_json::Value = serde_json::from_str(source).unwrap();
        source["type"] = "csv".into();
        source["path"] = "table.csv".into();
        let cfg: Config =
            serde_json::from_value(serde_json::json!({"watchlists":[source]})).unwrap();
        assert!(cfg.validate().is_err());
    }
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("table.csv"),
        "first,second\nSyntheticSecret\n",
    )
    .unwrap();
    for selection in [r#""missing""#, "3"] {
        let cfg = config(&format!(
            r#"{{"watchlists":[{{"type":"csv","path":"table.csv","columns":[{selection}]}}]}}"#
        ));
        let error = watchlist::load(&cfg, temp.path(), &HashMap::new()).unwrap_err();
        assert!(!format!("{error:#}").contains("SyntheticSecret"));
    }
}

#[test]
fn parentheses_expand_all_csv_sources_and_short_bare_names_stay_quiet() {
    let temp = tempdir().unwrap();
    for (index, value) in [
        "SyntheticOne(Fixture)",
        "SyntheticTwo（Fixture）",
        "SyntheticThree(Fixture)",
        "SyntheticFour（Fixture）",
    ]
    .iter()
    .enumerate()
    {
        fs::write(
            temp.path().join(format!("table{index}.csv")),
            format!("name\n{value}\n"),
        )
        .unwrap();
    }
    let sources: Vec<_> = (0..4).map(|index| serde_json::json!({"type":"csv","path":format!("table{index}.csv"),"column":"name","parenVariants":true})).collect();
    let cfg = config(&serde_json::json!({"watchlists":sources}).to_string());
    let loaded = watchlist::load(&cfg, temp.path(), &HashMap::new()).unwrap();
    for value in [
        "SyntheticOne",
        "SyntheticTwo",
        "SyntheticThree",
        "SyntheticFour",
    ] {
        assert_eq!(loaded.matcher.matches(value).count(), 1);
    }
}

#[test]
fn directory_watchlists_are_bounded_by_depth_length_and_entries() {
    let temp = tempdir().unwrap();
    let root = temp.path().join("personal");
    fs::create_dir(&root).unwrap();
    fs::write(
        root.join("synthetic-long-file"),
        "private-content-is-not-a-needle",
    )
    .unwrap();
    fs::write(root.join("short"), "").unwrap();
    fs::create_dir(root.join("child")).unwrap();
    fs::write(root.join("child/synthetic-child-file"), "").unwrap();
    fs::create_dir(root.join("child/grandchild")).unwrap();
    fs::write(root.join("child/grandchild/synthetic-too-deep"), "").unwrap();
    let cfg = config(
        r#"{"watchlists":[{"type":"directory","path":"personal"}],"allow":{"names":["synthetic-child-file"]}}"#,
    );
    let loaded = watchlist::load(&cfg, temp.path(), &HashMap::new()).unwrap();
    assert_eq!(loaded.matcher.len(), 1);
    assert!(
        loaded
            .matcher
            .matches("synthetic-long-file")
            .next()
            .is_some()
    );
    assert!(
        loaded
            .matcher
            .matches("private-content-is-not-a-needle")
            .next()
            .is_none()
    );
    let cfg = config(r#"{"watchlists":[{"type":"directory","path":"personal","maxEntries":1}]}"#);
    assert!(watchlist::load(&cfg, temp.path(), &HashMap::new()).is_err());
    for invalid in [
        r#"{"maxDepth":0}"#,
        r#"{"maxDepth":65}"#,
        r#"{"maxEntries":0}"#,
        r#"{"minNameLength":0}"#,
    ] {
        let mut source: serde_json::Value = serde_json::from_str(invalid).unwrap();
        source["type"] = "directory".into();
        source["path"] = "personal".into();
        let cfg: Config =
            serde_json::from_value(serde_json::json!({"watchlists":[source]})).unwrap();
        assert!(cfg.validate().is_err());
    }
}

#[test]
fn files_list_scans_gitignored_submissions_and_preserves_cwd_and_names() {
    let temp = tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    fs::create_dir(temp.path().join("sub")).unwrap();
    fs::write(temp.path().join(".gitignore"), "sub/\n").unwrap();
    fs::write(
        temp.path().join("doxguard.config.json"),
        r#"{"watchlists":[{"type":"lines","path":"words.txt"}]}"#,
    )
    .unwrap();
    fs::write(temp.path().join("words.txt"), "SyntheticIdentity\n").unwrap();
    fs::write(
        temp.path().join("sub/日本語 submission.txt"),
        "SyntheticIdentity\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("sub/list.txt"),
        "\u{feff}日本語 submission.txt\r\n\r\n日本語 submission.txt\r\n",
    )
    .unwrap();
    let output = run(
        &temp.path().join("sub"),
        &[
            "scan",
            "--files-from-list",
            "list.txt",
            "--block",
            "--format",
            "json",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["total_files"], 1);
    assert_eq!(report["hits"][0]["file"], "sub/日本語 submission.txt");
    assert_eq!(report["hits"][0]["matched"], "[REDACTED]");
    assert_eq!(
        fs::read_to_string(temp.path().join("sub/日本語 submission.txt")).unwrap(),
        "SyntheticIdentity\n"
    );
    assert_eq!(
        run(
            temp.path(),
            &["scan", "--files-from-list", "sub/list.txt", "--staged"]
        )
        .status
        .code(),
        Some(2)
    );
}

#[test]
fn list_errors_and_coverage_skips_fail_closed() {
    let temp = tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    fs::write(
        temp.path().join("doxguard.config.json"),
        r#"{"maxFileSize":16}"#,
    )
    .unwrap();
    fs::write(temp.path().join("big.txt"), "x".repeat(64)).unwrap();
    fs::write(temp.path().join("bad.txt"), [0x80]).unwrap();
    for (entry, code) in [
        ("big.txt", 1),
        ("bad.txt", 1),
        ("missing.txt", 2),
        ("../outside.txt", 2),
        ("C:/fixture.txt", 2),
        ("/fixture.txt", 2),
        (".", 2),
    ] {
        fs::write(temp.path().join("list.txt"), entry).unwrap();
        let output = run(
            temp.path(),
            &["scan", "--files-from-list", "list.txt", "--block"],
        );
        assert_eq!(
            output.status.code(),
            Some(code),
            "entry={entry}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
    fs::write(temp.path().join("list.txt"), [0x80]).unwrap();
    assert_eq!(
        run(temp.path(), &["scan", "--files-from-list", "list.txt"])
            .status
            .code(),
        Some(2)
    );
    assert!(scan::files_from_list(Path::new("absent.txt"), temp.path(), temp.path()).is_err());
}

#[cfg(windows)]
#[test]
fn junctions_are_not_followed_for_watchlists_or_list_targets() {
    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    fs::write(outside.path().join("synthetic-long-name"), "fixture").unwrap();
    let link = temp.path().join("linked");
    let output = Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(output.status.success());
    let cfg = config(r#"{"watchlists":[{"type":"directory","path":"."}]}"#);
    assert!(watchlist::load(&cfg, temp.path(), &HashMap::new()).is_err());
    fs::write(temp.path().join("list.txt"), "linked/synthetic-long-name").unwrap();
    assert!(scan::files_from_list(Path::new("list.txt"), temp.path(), temp.path()).is_err());
    // Remove the link only; TempDir then removes its own contents safely.
    fs::remove_dir(link).unwrap();
}

#[cfg(windows)]
#[test]
fn repository_internal_junctions_remain_coverage_skips_and_parent_traversal_is_rejected() {
    let temp = tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    fs::create_dir_all(temp.path().join("actual/deeper")).unwrap();
    fs::write(temp.path().join("actual/deeper/fixture.txt"), "fixture").unwrap();
    fs::write(temp.path().join("actual/fixture.txt"), "actual fixture").unwrap();
    fs::write(temp.path().join("fixture.txt"), "different fixture").unwrap();
    let link = temp.path().join("linked");
    assert!(
        Command::new("cmd.exe")
            .args(["/c", "mklink", "/J"])
            .arg(&link)
            .arg(temp.path().join("actual").join("deeper"))
            .output()
            .unwrap()
            .status
            .success()
    );
    fs::write(temp.path().join("list.txt"), "linked/fixture.txt").unwrap();
    let output = run(
        temp.path(),
        &[
            "scan",
            "--files-from-list",
            "list.txt",
            "--block",
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["coverage_skips"], 1);
    assert_eq!(report["scanned"], 0);
    fs::write(temp.path().join("list.txt"), "linked/../fixture.txt").unwrap();
    let output = run(
        temp.path(),
        &["scan", "--files-from-list", "list.txt", "--block"],
    );
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("parent components"));
    fs::remove_dir(link).unwrap();
}
