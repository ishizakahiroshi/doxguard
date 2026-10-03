use std::{fs, path::Path, process::Command};

use serde_json::Value;
use tempfile::tempdir;

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(output.status.success(), "fixture Git operation failed");
}

#[test]
fn text_and_json_show_bilingual_suggestions_without_detected_values() {
    let temp = tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    fs::write(temp.path().join("doxguard.config.json"), "{}").unwrap();
    // Construct synthetic detection inputs; documentation replacements use TEST-NET.
    let windows = ["Q:", "Users", "FixtureProfile", "notes.txt"].join("\\");
    let home = ["", "home", "fixture_profile", "notes.txt"].join("/");
    let ip = ["192", "168", "50", "9"].join(".");
    let email = ["fixture_private", "sample.test"].join("@");
    fs::write(
        temp.path().join("fixture.txt"),
        format!("{windows}\n{home}\n{ip}\n{email}\n"),
    )
    .unwrap();
    git(temp.path(), &["add", "fixture.txt"]);

    for format in ["text", "json"] {
        let output = Command::new(env!("CARGO_BIN_EXE_doxguard"))
            .args(["scan", "--staged", "--block", "--format", format])
            .current_dir(temp.path())
            .env_remove("DOXGUARD_CONFIG")
            .env_remove("DOXGUARD_SHOW_PATHS")
            .env_remove("DOXGUARD_WATCHLIST_DIR")
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(1));
        let stdout = String::from_utf8(output.stdout).unwrap();
        let stderr = String::from_utf8(output.stderr).unwrap();
        let combined = format!("{stdout}{stderr}");
        assert!(combined.contains("[REDACTED]"));
        // The path detectors match only prefixes, which must also remain hidden.
        assert!(!combined.contains("Q:"));
        assert!(!combined.contains("/home/fixture_profile/"));
        for detected in [&windows, &home, &ip, &email] {
            assert!(!combined.contains(detected), "detected value was exposed");
        }
        for suggestion in [
            "Remove the personal absolute path or replace it with a placeholder",
            "Replace the home path with ~/ or a placeholder",
            "Generalize or remove the internal IP address",
            "Remove the non-public email or add an explicitly public address/domain to allow",
            "個人の絶対パス",
            "ホームパス",
            "内部 IP アドレス",
            "非公開メール",
            "RFC5737",
            "192.0.2.0/24",
            "198.51.100.0/24",
            "203.0.113.0/24",
        ] {
            assert!(
                combined.contains(suggestion),
                "missing suggestion: {suggestion}"
            );
        }
        if format == "json" {
            let report: Value = serde_json::from_str(&stdout).unwrap();
            let hits = report["hits"].as_array().unwrap();
            assert_eq!(hits.len(), 4);
            for hit in hits {
                assert_eq!(hit["matched"], "[REDACTED]");
                let suggestion = hit["suggestion"].as_str().unwrap();
                assert!(suggestion.contains(" / "));
                assert!(!suggestion.is_ascii());
            }
        }
    }
}
