use doxguard::{config::Config, patterns};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

fn run(cwd: &Path, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .current_dir(cwd)
        .args(args)
        .env_remove("DOXGUARD_CONFIG")
        .output()
        .unwrap()
}

fn config(cwd: &Path, extra: serde_json::Value) {
    let mut value = serde_json::json!({
        "structural": {"windowsPath": false, "posixHome": false, "privateIp": false,
            "email": false, "domain": true, "ipv4": true, "privateRepoPath": true},
        "failOnSkip": true
    });
    for (key, value_extra) in extra.as_object().unwrap() {
        value[key] = value_extra.clone();
    }
    fs::write(cwd.join("doxguard.config.json"), value.to_string()).unwrap();
}

fn json(output: &Output) -> serde_json::Value {
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn outside_git_redacts_text_and_json_and_extracts_keys() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    fs::write(
        temp.path().join("fixture.txt"),
        concat!(
            "host = build.example.test\n",
            "{\"endpoint\":\"https://service.example.test\",\"ip\":\"192.0.2.9\"}\n",
            "$env:TARGET = '203.0.113.9'\n",
            "A sentence mentioning github/private/synthetic-internal.\n",
        ),
    )
    .unwrap();
    let args = ["scan", "--path", "fixture.txt", "--block"];
    let text = run(temp.path(), &args);
    assert_eq!(text.status.code(), Some(1));
    let report = String::from_utf8(text.stderr).unwrap();
    for value in [
        "build.example.test",
        "service.example.test",
        "192.0.2.9",
        "203.0.113.9",
        "synthetic-internal",
    ] {
        assert!(!report.contains(value));
    }
    assert!(report.contains("fixture.txt:1"));
    assert!(report.contains("variable: host"));
    assert!(report.contains("variable: (unavailable)"));
    let output = run(
        temp.path(),
        &[
            "scan",
            "--path",
            "fixture.txt",
            "--block",
            "--format",
            "json",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let report = json(&output);
    assert_eq!(report["hits"].as_array().unwrap().len(), 5);
    let vars: Vec<_> = report["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["variable"].as_str())
        .collect();
    assert_eq!(
        vars,
        [
            Some("host"),
            Some("ip"),
            Some("endpoint"),
            Some("TARGET"),
            None
        ]
    );
    for hit in report["hits"].as_array().unwrap() {
        assert_eq!(hit["matched"], "[redacted]");
    }
    let shown = run(
        temp.path(),
        &[
            "scan",
            "--path",
            "fixture.txt",
            "--format",
            "json",
            "--show-matches",
        ],
    );
    assert!(
        String::from_utf8(shown.stdout)
            .unwrap()
            .contains("build.example.test")
    );
}

#[test]
fn allowlists_cover_subdomains_cidr_and_private_repos_without_suffix_bypasses() {
    let temp = tempdir().unwrap();
    config(
        temp.path(),
        serde_json::json!({"allow": {
            "domains": ["example.test"], "ipv4": ["192.0.2.0/24", "203.0.113.9"],
            "privateRepos": ["synthetic-internal"]
        }}),
    );
    fs::write(
        temp.path().join("fixture.txt"),
        concat!(
            "host = BUILD.EXAMPLE.TEST\n192.0.2.009\n203.0.113.9\n",
            "github/private/SYNTHETIC-INTERNAL\n",
            "host = notexample.test\nhost = example.test.attacker.test\n",
            "198.51.100.9\ngithub\\private\\synthetic-other\n",
            "host = example.com\n",
        ),
    )
    .unwrap();
    let output = run(
        temp.path(),
        &[
            "scan",
            "--path",
            "fixture.txt",
            "--format",
            "json",
            "--block",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(json(&output)["hits"].as_array().unwrap().len(), 5);
}

#[test]
fn staged_domain_gate_uses_index_and_is_opt_in() {
    let temp = tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(temp.path())
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "-q"]);
    fs::write(
        temp.path().join("fixture.txt"),
        "host = build.example.test\n",
    )
    .unwrap();
    git(&["add", "fixture.txt"]);
    assert!(
        run(temp.path(), &["scan", "--staged", "--block"])
            .status
            .success()
    );
    config(temp.path(), serde_json::json!({}));
    fs::write(temp.path().join("fixture.txt"), "safe\n").unwrap();
    assert_eq!(
        run(temp.path(), &["scan", "--staged", "--block"])
            .status
            .code(),
        Some(1)
    );
    config(
        temp.path(),
        serde_json::json!({"allow": {"domains": ["example.test"]}}),
    );
    assert!(
        run(temp.path(), &["scan", "--staged", "--block"])
            .status
            .success()
    );
}

#[test]
fn explicit_paths_fail_closed_and_do_not_inherit_repository_exemptions() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({"maxFileSize": 80}));
    fs::create_dir_all(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("tests/fixture.txt"),
        "build.example.test\n",
    )
    .unwrap();
    fs::write(temp.path().join("oversize.txt"), "x".repeat(90)).unwrap();
    fs::write(temp.path().join("invalid.txt"), [0xff, 0xfe]).unwrap();
    fs::write(
        temp.path().join("paths.txt"),
        "\u{feff}tests\noversize.txt\ninvalid.txt\n\n",
    )
    .unwrap();
    let output = run(
        temp.path(),
        &[
            "scan",
            "--path-from",
            "paths.txt",
            "--path",
            "tests/fixture.txt",
            "--format",
            "json",
            "--block",
        ],
    );
    let report = json(&output);
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(report["total_files"], 3);
    assert_eq!(report["coverage_skips"], 2);
    assert_eq!(report["hits"].as_array().unwrap().len(), 1);
    for args in [
        vec!["scan", "--path", "missing.txt"],
        vec!["scan", "--path", "tests", "--staged"],
        vec!["scan", "--path", ""],
    ] {
        assert_eq!(run(temp.path(), &args).status.code(), Some(2));
    }
    fs::write(temp.path().join("empty-paths.txt"), "\n").unwrap();
    assert_eq!(
        run(temp.path(), &["scan", "--path-from", "empty-paths.txt"])
            .status
            .code(),
        Some(2)
    );
}

#[test]
fn ip_allow_is_shared_with_rfc1918_and_invalid_config_is_rejected() {
    let mut config = Config::default();
    // Build the synthetic private address from octets, never a live host value.
    let address = [10, 0, 0, 9].map(|n| n.to_string()).join(".");
    config.allow.ipv4.push(format!("{address}/32"));
    let pattern = patterns::build(&config)
        .unwrap()
        .into_iter()
        .find(|p| p.name.contains("RFC1918"))
        .unwrap();
    assert!(pattern.is_allowed(&address, &config));
    assert!(!pattern.is_allowed(&address.replace('9', "8"), &config));
    for invalid in [
        "192.0.2.1/33",
        "192.0.2.1/",
        "192.0.2.999",
        "example.test",
        "192.0.2.1/-1",
    ] {
        config.allow.ipv4 = vec![invalid.to_owned()];
        assert!(config.validate().is_err());
    }
}

#[test]
fn public_ip_is_detected_without_host_context_and_invalid_octets_are_quiet() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    fs::write(
        temp.path().join("fixture.txt"),
        "192.0.2.9\n999.192.0.2.9\n192.0.2.999\n198.51.100.9,203.0.113.9\n",
    )
    .unwrap();
    let output = run(
        temp.path(),
        &["scan", "--path", "fixture.txt", "--format", "json"],
    );
    assert_eq!(json(&output)["hits"].as_array().unwrap().len(), 3);
}

#[test]
fn redaction_also_covers_values_in_source_labels_and_suggestions() {
    let temp = tempdir().unwrap();
    config(
        temp.path(),
        serde_json::json!({"structural": {
            "domain": false, "custom": [{"name": "build.example.test", "regex": "build\\.example\\.test", "suggestion": "remove build.example.test"}]
        }}),
    );
    fs::write(
        temp.path().join("fixture.txt"),
        "host = build.example.test\n",
    )
    .unwrap();
    let output = run(
        temp.path(),
        &["scan", "--path", "fixture.txt", "--format", "json"],
    );
    assert!(
        !String::from_utf8(output.stdout)
            .unwrap()
            .contains("build.example.test")
    );
}

#[test]
fn recursive_paths_never_follow_links_and_count_them_as_coverage_skips() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    fs::create_dir(temp.path().join("surface")).unwrap();
    fs::create_dir(temp.path().join("target")).unwrap();
    fs::write(
        temp.path().join("target/fixture.txt"),
        "build.example.test\n",
    )
    .unwrap();
    let link = temp.path().join("surface/link");
    #[cfg(unix)]
    std::os::unix::fs::symlink(temp.path().join("target"), &link).unwrap();
    #[cfg(windows)]
    {
        let output = Command::new("pwsh.exe").args(["-NoProfile", "-NonInteractive", "-Command",
            "New-Item -ItemType Junction -Path $env:DOXGUARD_TEST_LINK -Target $env:DOXGUARD_TEST_TARGET -ErrorAction Stop | Out-Null"])
            .env("DOXGUARD_TEST_LINK", &link).env("DOXGUARD_TEST_TARGET", temp.path().join("target"))
            .output().unwrap();
        assert!(output.status.success(), "cannot create synthetic junction");
    }
    let output = run(
        temp.path(),
        &["scan", "--path", "surface", "--format", "json", "--block"],
    );
    assert_eq!(output.status.code(), Some(1));
    let report = json(&output);
    assert_eq!(report["coverage_skips"], 1);
    assert_eq!(report["hits"].as_array().unwrap().len(), 0);
    #[cfg(windows)]
    fs::remove_dir(link).unwrap();
}

#[test]
fn path_reports_name_files_as_passed_without_the_working_directory() {
    let temp = tempdir().unwrap();
    let work = temp.path().join("work");
    fs::create_dir_all(work.join("surface/sub")).unwrap();
    fs::create_dir(temp.path().join("outside")).unwrap();
    config(&work, serde_json::json!({"maxFileSize": 80}));
    for file in [
        "fixture.txt",
        "surface/sub/nested.txt",
        "listed.txt",
        "../outside/parent.txt",
    ] {
        fs::write(work.join(file), "build.example.test\n").unwrap();
    }
    fs::write(work.join("surface/sub/oversize.txt"), "x".repeat(90)).unwrap();
    fs::write(work.join("paths.txt"), "listed.txt\n").unwrap();
    let nested = Path::new("surface").join("sub").join("nested.txt");
    let oversize = Path::new("surface").join("sub").join("oversize.txt");
    let (nested, oversize) = (nested.to_str().unwrap(), oversize.to_str().unwrap());
    // The temporary directory's unique name stands in for the cwd and home
    // components, and survives JSON's backslash escaping.
    let marker = temp.path().file_name().unwrap().to_str().unwrap();
    let args = |format: &'static str| {
        [
            "scan",
            "--path",
            "fixture.txt",
            "--path",
            "./fixture.txt",
            "--path",
            "surface",
            "--path",
            "../outside/parent.txt",
            "--path-from",
            "paths.txt",
            "--block",
            "--format",
            format,
        ]
    };
    let output = run(&work, &args("json"));
    assert_eq!(output.status.code(), Some(1));
    let report = json(&output);
    assert_eq!(report["total_files"], 5);
    assert_eq!(report["coverage_skips"], 1);
    let mut files: Vec<_> = report["hits"]
        .as_array()
        .unwrap()
        .iter()
        .map(|hit| hit["file"].as_str().unwrap().to_owned())
        .collect();
    files.sort();
    let mut expected = vec!["../outside/parent.txt", "fixture.txt", "listed.txt", nested];
    expected.sort();
    assert_eq!(files, expected);
    let skipped = format!("WARN: skipped {oversize} (oversize)");
    assert!(
        report["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|warning| warning == skipped.as_str()),
        "{report}"
    );
    for format in ["json", "text"] {
        let output = run(&work, &args(format));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(!text.contains(marker), "{format}: {text}");
        if format == "text" {
            assert!(text.contains(&format!("{nested}:1")), "{text}");
            assert!(text.contains(&skipped), "{text}");
        }
    }
    // An absolute path is reported exactly as it was passed.
    let absolute = work.join("fixture.txt");
    let absolute = absolute.to_str().unwrap();
    let report = json(&run(
        &work,
        &["scan", "--path", absolute, "--format", "json"],
    ));
    assert_eq!(report["hits"][0]["file"], absolute);
}

#[test]
fn redaction_treats_both_path_separators_as_one() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    let directory = temp.path().join("github/private/synthetic-internal");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("f.md"),
        "see github/private/synthetic-internal\n",
    )
    .unwrap();
    let json_output = run(
        temp.path(),
        &["scan", "--path", "github", "--format", "json", "--block"],
    );
    assert_eq!(json_output.status.code(), Some(1));
    let report = json(&json_output);
    assert_eq!(
        report["hits"][0]["file"],
        format!("[redacted]{}f.md", std::path::MAIN_SEPARATOR)
    );
    // Doubled separators in the text give a value whose separators are doubled,
    // yet the single-separator file name is still hidden.
    fs::write(
        directory.join("f.md"),
        "see github//private//synthetic-internal\n",
    )
    .unwrap();
    let doubled = run(
        temp.path(),
        &["scan", "--path", "github", "--format", "json", "--block"],
    );
    assert_eq!(doubled.status.code(), Some(1));
    assert_eq!(
        json(&doubled)["hits"][0]["file"],
        format!("[redacted]{}f.md", std::path::MAIN_SEPARATOR)
    );
    let text_output = run(temp.path(), &["scan", "--path", "github", "--block"]);
    assert_eq!(text_output.status.code(), Some(1));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&text_output.stdout),
        String::from_utf8_lossy(&text_output.stderr)
    );
    assert!(!text.contains("synthetic-internal"), "{text}");
}

#[test]
fn redaction_hides_a_path_value_that_ends_a_sentence_in_the_file_name() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    let directory = temp.path().join("github/private/synthetic-internal");
    fs::create_dir_all(&directory).unwrap();
    fs::write(
        directory.join("f.md"),
        "A sentence mentioning github/private/synthetic-internal.\n",
    )
    .unwrap();
    let json_output = run(
        temp.path(),
        &["scan", "--path", "github", "--format", "json", "--block"],
    );
    assert_eq!(json_output.status.code(), Some(1));
    assert_eq!(
        json(&json_output)["hits"][0]["file"],
        format!("[redacted]{}f.md", std::path::MAIN_SEPARATOR)
    );
    let text_output = run(temp.path(), &["scan", "--path", "github", "--block"]);
    assert_eq!(text_output.status.code(), Some(1));
    let text = format!(
        "{}{}",
        String::from_utf8_lossy(&text_output.stdout),
        String::from_utf8_lossy(&text_output.stderr)
    );
    assert!(!text.contains("synthetic-internal"), "{text}");
}

#[test]
fn redaction_hides_a_path_value_followed_by_git_or_a_hyphen_in_the_file_name() {
    for body in [
        "clone github/private/synthetic-internal.git\n",
        "see github/private/synthetic-internal- now\n",
    ] {
        let temp = tempdir().unwrap();
        config(temp.path(), serde_json::json!({}));
        let directory = temp.path().join("github/private/synthetic-internal");
        fs::create_dir_all(&directory).unwrap();
        fs::write(directory.join("f.md"), body).unwrap();
        let json_output = run(
            temp.path(),
            &["scan", "--path", "github", "--format", "json", "--block"],
        );
        assert_eq!(json_output.status.code(), Some(1), "{body}");
        assert_eq!(
            json(&json_output)["hits"][0]["file"],
            format!("[redacted]{}f.md", std::path::MAIN_SEPARATOR),
            "{body}"
        );
        let text_output = run(temp.path(), &["scan", "--path", "github", "--block"]);
        assert_eq!(text_output.status.code(), Some(1), "{body}");
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&text_output.stdout),
            String::from_utf8_lossy(&text_output.stderr)
        );
        assert!(!text.contains("synthetic-internal"), "{body}: {text}");
    }
}

#[test]
fn redaction_hides_a_home_value_in_a_relative_file_name() {
    let temp = tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(temp.path())
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "-q"]);
    config(
        temp.path(),
        serde_json::json!({"structural": {
            "windowsPath": false, "posixHome": true, "privateIp": false, "email": false,
            "domain": false, "ipv4": false, "privateRepoPath": false
        }}),
    );
    // Built at run time so the source holds no POSIX home path.
    let home = format!("/{}/{}/", "home", "synthetic-user");
    fs::create_dir_all(temp.path().join("home/synthetic-user")).unwrap();
    fs::write(
        temp.path().join("home/synthetic-user/notes.md"),
        format!("see {home}project\n"),
    )
    .unwrap();
    git(&["add", "home/synthetic-user/notes.md"]);
    let output = run(
        temp.path(),
        &["scan", "--staged", "--format", "json", "--block"],
    );
    assert_eq!(output.status.code(), Some(1));
    let report = json(&output);
    assert_eq!(report["hits"].as_array().unwrap().len(), 1);
    assert_eq!(report["hits"][0]["file"], "[redacted]notes.md");
}

#[test]
fn non_utf8_data_retains_ascii_findings_and_remains_a_coverage_skip() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    let mut bytes = vec![0xff, 0xfe, b' '];
    bytes.extend_from_slice(b"host = build.example.test\n");
    fs::write(temp.path().join("fixture.txt"), bytes).unwrap();
    let output = run(
        temp.path(),
        &[
            "scan",
            "--path",
            "fixture.txt",
            "--format",
            "json",
            "--block",
        ],
    );
    assert_eq!(output.status.code(), Some(1));
    let report = json(&output);
    assert_eq!(report["coverage_skips"], 1);
    assert_eq!(report["hits"].as_array().unwrap().len(), 1);
    assert_eq!(report["hits"][0]["matched"], "[redacted]");
}

#[test]
fn invalid_domain_occurrence_does_not_suppress_valid_repeat_on_same_line() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    fs::write(
        temp.path().join("fixture.txt"),
        "_build.example.test build.example.test\n",
    )
    .unwrap();
    let output = run(
        temp.path(),
        &["scan", "--path", "fixture.txt", "--format", "json"],
    );
    assert_eq!(json(&output)["hits"].as_array().unwrap().len(), 1);
}

#[test]
fn explicit_directory_exemptions_remain_relative_to_the_working_directory() {
    let temp = tempdir().unwrap();
    config(
        temp.path(),
        serde_json::json!({"exemptPaths": ["generated/"]}),
    );
    fs::create_dir(temp.path().join("generated")).unwrap();
    fs::write(
        temp.path().join("generated/fixture.txt"),
        "build.example.test\n",
    )
    .unwrap();
    let output = run(
        temp.path(),
        &["scan", "--path", "generated", "--block", "--format", "json"],
    );
    assert!(output.status.success());
    assert_eq!(json(&output)["hits"].as_array().unwrap().len(), 0);
}

#[test]
fn unicode_prose_does_not_hide_ascii_domains_or_ip_addresses() {
    let temp = tempdir().unwrap();
    config(temp.path(), serde_json::json!({}));
    fs::write(
        temp.path().join("fixture.txt"),
        "接続先はbuild.example.testです\n接続先192.0.2.9です\n",
    )
    .unwrap();
    let output = run(
        temp.path(),
        &["scan", "--path", "fixture.txt", "--format", "json"],
    );
    assert_eq!(json(&output)["hits"].as_array().unwrap().len(), 2);
}

#[test]
fn instruction_staged_profile_scans_default_exemptions_and_lockfiles() {
    let temp = tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(temp.path())
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "-q"]);
    fs::create_dir(temp.path().join("tests")).unwrap();
    for path in ["tests/fixture.md", "package-lock.json"] {
        fs::write(temp.path().join(path), "host = build.example.test\n").unwrap();
    }
    git(&["add", "tests/fixture.md", "package-lock.json"]);
    let args = [
        "scan", "--staged", "--block", "--strict", "--format", "json",
    ];
    config(temp.path(), serde_json::json!({}));
    let default = run(temp.path(), &args);
    assert_eq!(default.status.code(), Some(1));
    assert_eq!(json(&default)["scanned"], 2);
    assert_eq!(json(&default)["coverage_skips"], 0);

    config(
        temp.path(),
        serde_json::json!({"includeDefaultExemptPaths": true}),
    );
    let legacy_exemptions = run(temp.path(), &args);
    assert_eq!(legacy_exemptions.status.code(), Some(0));
    assert_eq!(json(&legacy_exemptions)["scanned"], 0);

    config(
        temp.path(),
        serde_json::json!({"includeDefaultExemptPaths": false}),
    );
    // The gate must still read the index even after the worktree is cleaned.
    fs::write(temp.path().join("tests/fixture.md"), "safe\n").unwrap();
    fs::write(temp.path().join("package-lock.json"), "{}\n").unwrap();
    let checked = run(temp.path(), &args);
    assert_eq!(checked.status.code(), Some(1));
    let report = json(&checked);
    assert_eq!(report["scanned"], 2);
    assert_eq!(report["hits"].as_array().unwrap().len(), 2);
    assert_eq!(report["coverage_skips"], 0);

    config(
        temp.path(),
        serde_json::json!({"includeDefaultExemptPaths": false,
            "exemptPaths": ["tests/"]}),
    );
    let excluded = run(temp.path(), &args);
    assert_eq!(excluded.status.code(), Some(1));
    let report = json(&excluded);
    assert_eq!(report["scanned"], 2);
    assert_eq!(report["hits"].as_array().unwrap().len(), 1);
    assert_eq!(report["hits"][0]["file"], "package-lock.json");
}

#[test]
fn instruction_profile_rejects_scoped_bare_and_legacy_inline_allows() {
    let temp = tempdir().unwrap();
    let git = |args: &[&str]| {
        assert!(
            Command::new("git")
                .current_dir(temp.path())
                .args(args)
                .status()
                .unwrap()
                .success()
        );
    };
    git(&["init", "-q"]);
    fs::write(
        temp.path().join("fixture.md"),
        concat!(
            "host = build.example.test # doxguard: allow build.example.test\n",
            "host = build.example.test # doxguard: allow\n",
            "host = build.example.test # secrets-scan: allow example.test\n",
            "host = build.example.test <!-- doxguard: allow build.example.test-->\n",
        ),
    )
    .unwrap();
    git(&["add", "fixture.md"]);
    config(temp.path(), serde_json::json!({}));
    let args = ["scan", "--staged", "--block", "--format", "json"];
    let compatible = run(temp.path(), &args);
    assert_eq!(compatible.status.code(), Some(0));
    assert_eq!(json(&compatible)["hits"].as_array().unwrap().len(), 0);
    let strict_args = [
        "scan", "--staged", "--block", "--strict", "--format", "json",
    ];
    let compatible_strict = run(temp.path(), &strict_args);
    assert_eq!(compatible_strict.status.code(), Some(1));
    assert_eq!(
        json(&compatible_strict)["hits"].as_array().unwrap().len(),
        1
    );

    config(
        temp.path(),
        serde_json::json!({"allow": {"disallowInlineAllow": true}}),
    );
    for mode_args in [
        strict_args.to_vec(),
        vec![
            "scan",
            "--path",
            "fixture.md",
            "--block",
            "--strict",
            "--format",
            "json",
        ],
    ] {
        let blocked = run(temp.path(), &mode_args);
        assert_eq!(blocked.status.code(), Some(1));
        let report = json(&blocked);
        // The legacy scoped directive also contains a distinct domain token.
        let lines: Vec<_> = report["hits"]
            .as_array()
            .unwrap()
            .iter()
            .map(|hit| hit["line_number"].as_u64().unwrap())
            .collect();
        assert_eq!(lines, [1, 2, 3, 3, 4]);
        assert_eq!(report["coverage_skips"], 0);
    }
    config(
        temp.path(),
        serde_json::json!({"allow": {"disallowInlineAllow": true,
            "domains": ["example.test"]}}),
    );
    let allowed = run(temp.path(), &strict_args);
    assert_eq!(allowed.status.code(), Some(0));
    assert_eq!(json(&allowed)["hits"].as_array().unwrap().len(), 0);
}

#[test]
fn required_watchlists_fail_closed_without_printing_unavailable_sources() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("fixture.md"), "SyntheticWatchword\n").unwrap();
    let variable = "DOXGUARD_SYNTHETIC_WATCHLIST_ROOT";
    let watchlists = serde_json::json!([{
        "type": "lines", "path": "${DOXGUARD_SYNTHETIC_WATCHLIST_ROOT}/words.txt",
        "label": "synthetic-watchlist"
    }]);
    let scan = |root: Option<&str>| {
        let mut command = Command::new(env!("CARGO_BIN_EXE_doxguard"));
        command
            .current_dir(temp.path())
            .args([
                "scan",
                "--path",
                "fixture.md",
                "--block",
                "--strict",
                "--format",
                "json",
            ])
            .env_remove("DOXGUARD_CONFIG")
            .env_remove(variable);
        if let Some(root) = root {
            command.env(variable, root);
        }
        command.output().unwrap()
    };
    // Required by default: an unavailable input must never silently drop coverage.
    config(temp.path(), serde_json::json!({"watchlists": watchlists}));
    for root in [None, Some(""), temp.path().to_str()] {
        let compatible = scan(root);
        assert_eq!(compatible.status.code(), Some(2));
        assert!(compatible.stdout.is_empty());
    }
    config(
        temp.path(),
        serde_json::json!({"watchlists": watchlists, "requireWatchlists": true}),
    );
    for root in [None, Some(""), temp.path().to_str()] {
        let blocked = scan(root);
        assert_eq!(blocked.status.code(), Some(2));
        assert!(blocked.stdout.is_empty());
        assert_eq!(
            String::from_utf8(blocked.stderr).unwrap().trim(),
            "ERROR: required watchlist source is unavailable"
        );
    }
    fs::write(temp.path().join("words.txt"), "SyntheticWatchword\n").unwrap();
    for required in [false, true] {
        config(
            temp.path(),
            serde_json::json!({"watchlists": watchlists, "requireWatchlists": required}),
        );
        let checked = scan(temp.path().to_str());
        assert_eq!(checked.status.code(), Some(1));
        let report = json(&checked);
        assert_eq!(report["watchlist_needles"], 1);
        assert_eq!(report["hits"].as_array().unwrap().len(), 1);
        assert_eq!(report["hits"][0]["kind"], "watchlist");
        assert_eq!(report["hits"][0]["matched"], "[redacted]");
        assert_eq!(report["warnings"].as_array().unwrap().len(), 0);
    }
}
