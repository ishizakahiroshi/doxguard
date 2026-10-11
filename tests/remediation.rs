//! Synthetic regression fixtures; no real watchlists or user hooks are read.
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

fn command(program: &str, cwd: &Path) -> Command {
    let mut command = Command::new(program);
    command.current_dir(cwd);
    for (key, _) in std::env::vars_os() {
        let name = key.to_string_lossy().to_ascii_uppercase();
        if name.starts_with("GIT_") || name == "DOXGUARD_CONFIG" {
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
    let result = command("git", cwd).args(args).output().unwrap();
    assert!(
        result.status.success(),
        "git: {}",
        String::from_utf8_lossy(&result.stderr)
    );
    String::from_utf8(result.stdout).unwrap()
}
fn repo(cwd: &Path) {
    git(cwd, &["init", "-q"]);
    git(cwd, &["config", "user.name", "Synthetic Fixture"]);
    git(cwd, &["config", "user.email", "fixture@example.com"]);
    git(
        cwd,
        &["commit", "--allow-empty", "-qm", "synthetic baseline"],
    );
}
fn config(cwd: &Path, needles: &str) {
    fs::write(cwd.join("needles.txt"), needles).unwrap();
    fs::write(
        cwd.join("doxguard.config.json"),
        r#"{
      "watchlists":[{"type":"lines","path":"needles.txt","label":"synthetic"}],
      "requireWatchlists":true,"includeDefaultExemptPaths":false,
      "structural":{"windowsPath":false,"posixHome":false,"privateIp":false,"email":false},
      "noise":{"asciiCaseInsensitive":true}
    }"#,
    )
    .unwrap();
}
fn run(cwd: &Path, args: &[&str]) -> Output {
    command(env!("CARGO_BIN_EXE_doxguard"), cwd)
        .args(args)
        .env_remove("DOXGUARD_CONFIG")
        .output()
        .unwrap()
}
fn blocked(cwd: &Path, selector: &[&str]) -> serde_json::Value {
    let mut args = vec![
        "scan",
        "--block",
        "--strict",
        "--format",
        "json",
        "--show-matches",
    ];
    args.extend_from_slice(selector);
    let output = run(cwd, &args);
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

#[test]
fn git_selectors_preserve_leading_space_and_unicode() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    for name in [" secret.md", "日本語.md", "quote's.md"] {
        fs::write(cwd.join(name), "SyntheticNeedle\n").unwrap();
        git(cwd, &["add", "--", name]);
    }
    fs::write(cwd.join("secret.md"), "safe\n").unwrap();
    for mode in ["--staged", "--diff", "--all-tracked"] {
        let json = blocked(cwd, &[mode]);
        assert_eq!(json["hits"].as_array().unwrap().len(), 3);
        assert!(
            json["hits"]
                .as_array()
                .unwrap()
                .iter()
                .any(|h| h["file"] == " secret.md")
        );
    }
}

#[cfg(unix)]
#[test]
fn git_selectors_preserve_newlines_and_quotes() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    for name in ["line\nbreak.md", "double\"quote.md"] {
        fs::write(cwd.join(name), "SyntheticNeedle\n").unwrap();
        git(cwd, &["add", "--", name]);
    }
    for mode in ["--staged", "--diff", "--all-tracked"] {
        let json = blocked(cwd, &[mode]);
        assert_eq!(json["hits"].as_array().unwrap().len(), 2);
    }
}

#[cfg(unix)]
#[test]
fn literal_backslash_targets_native_file() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    fs::create_dir(cwd.join("sub")).unwrap();
    fs::write(cwd.join("sub/policy.md"), "safe").unwrap();
    fs::write(cwd.join("sub\\policy.md"), "SyntheticNeedle").unwrap();
    let json = blocked(cwd, &["--path", "sub\\policy.md"]);
    assert_eq!(json["hits"].as_array().unwrap().len(), 1);
}

#[cfg(unix)]
#[test]
fn parent_after_symlink_uses_os_resolution() {
    use std::os::unix::fs::symlink;
    let temp = tempdir().unwrap();
    let cwd = temp.path().join("repo");
    let outer = temp.path().join("outer");
    fs::create_dir_all(&cwd).unwrap();
    fs::create_dir_all(outer.join("inner")).unwrap();
    repo(&cwd);
    config(&cwd, "SyntheticNeedle\n");
    fs::write(cwd.join("policy.md"), "safe").unwrap();
    fs::write(outer.join("policy.md"), "SyntheticNeedle").unwrap();
    symlink(outer.join("inner"), cwd.join("link")).unwrap();
    let json = blocked(&cwd, &["--path", "link/../policy.md"]);
    assert_eq!(json["hits"].as_array().unwrap().len(), 1);
}

#[test]
fn bom_first_watchlist_needle_is_active() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "\u{feff}SyntheticNeedle\nSecondNeedle\n");
    fs::write(cwd.join("policy.md"), "SyntheticNeedle SecondNeedle").unwrap();
    let json = blocked(cwd, &["--path", "policy.md"]);
    assert_eq!(json["watchlist_needles"], 2);
    assert_eq!(json["hits"].as_array().unwrap().len(), 2);
}

#[test]
fn required_available_but_empty_watchlist_keeps_existing_contract() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "# empty by choice\n");
    fs::write(cwd.join("policy.md"), "safe").unwrap();
    let output = run(
        cwd,
        &["scan", "--path", "policy.md", "--block", "--format", "json"],
    );
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["watchlist_needles"], 0);
}

#[test]
fn mixed_case_metadata_is_redacted_unless_explicitly_requested() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    fs::write(cwd.join("SyntheticNeedle.md"), "config=SYNTHETICNEEDLE\n").unwrap();
    for format in ["json", "text"] {
        let output = run(
            cwd,
            &[
                "scan",
                "--path",
                "SyntheticNeedle.md",
                "--block",
                "--format",
                format,
            ],
        );
        assert_eq!(output.status.code(), Some(1));
        let text = format!(
            "{}{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
        .to_ascii_lowercase();
        assert!(!text.contains("syntheticneedle"), "{text}");
    }
    let json = blocked(cwd, &["--path", "SyntheticNeedle.md"]);
    assert!(
        json["hits"][0]["file"]
            .as_str()
            .unwrap()
            .ends_with("SyntheticNeedle.md")
    );
}

#[test]
fn staged_type_change_scans_regular_index_blob_not_worktree() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    git(cwd, &["config", "core.symlinks", "true"]);
    fs::write(cwd.join("target.txt"), "synthetic-link-target").unwrap();
    let oid = git(cwd, &["hash-object", "-w", "target.txt"]);
    git(
        cwd,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "120000",
            oid.trim(),
            "policy.md",
        ],
    );
    git(cwd, &["commit", "-qm", "synthetic link index"]);
    fs::write(cwd.join("policy.md"), "SyntheticNeedle").unwrap();
    git(cwd, &["add", "policy.md"]);
    assert!(git(cwd, &["diff", "--cached", "--name-status"]).starts_with("T\t"));
    let json = blocked(cwd, &["--diff"]);
    assert_eq!(json["hits"].as_array().unwrap().len(), 1);
    fs::write(cwd.join("policy.md"), "safe worktree alias").unwrap();
    let json = blocked(cwd, &["--staged"]);
    assert_eq!(json["hits"].as_array().unwrap().len(), 1);
}

#[test]
fn staged_links_and_gitlinks_are_explicit_coverage_skips() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    fs::write(cwd.join("policy.md"), "safe").unwrap();
    git(cwd, &["add", "policy.md"]);
    git(cwd, &["commit", "-qm", "regular"]);
    let oid = git(cwd, &["rev-parse", "HEAD:policy.md"]);
    git(
        cwd,
        &[
            "update-index",
            "--cacheinfo",
            "120000",
            oid.trim(),
            "policy.md",
        ],
    );
    let json = blocked(cwd, &["--staged"]);
    assert_eq!(json["coverage_skips"], 1);
    assert_eq!(json["hits"].as_array().unwrap().len(), 0);
    let commit = git(cwd, &["rev-parse", "HEAD"]);
    git(
        cwd,
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            "160000",
            commit.trim(),
            "module",
        ],
    );
    let json = blocked(cwd, &["--staged"]);
    assert_eq!(json["coverage_skips"], 2);
}

#[cfg(unix)]
#[test]
fn init_rejects_linked_outputs_without_touching_targets() {
    use std::os::unix::fs::symlink;
    for relative in [
        "doxguard.config.json",
        ".github",
        ".githooks",
        ".git/doxguard",
    ] {
        let temp = tempdir().unwrap();
        let cwd = temp.path().join("repo");
        fs::create_dir(&cwd).unwrap();
        repo(&cwd);
        let target = temp.path().join("untouched");
        if relative == "doxguard.config.json" {
            fs::write(&target, "sentinel").unwrap();
        } else {
            fs::create_dir(&target).unwrap();
            fs::write(target.join("sentinel"), "unchanged").unwrap();
        }
        symlink(&target, cwd.join(relative)).unwrap();
        let output = run(&cwd, &["init"]);
        assert!(!output.status.success(), "accepted {relative}");
        if target.is_file() {
            assert_eq!(fs::read_to_string(&target).unwrap(), "sentinel");
        } else {
            assert_eq!(fs::read_dir(&target).unwrap().count(), 1);
        }
    }
}

#[cfg(unix)]
#[test]
fn hook_cache_leaf_link_is_not_overwritten() {
    use std::os::unix::fs::symlink;
    let temp = tempdir().unwrap();
    let cwd = temp.path().join("repo");
    fs::create_dir(&cwd).unwrap();
    repo(&cwd);
    fs::create_dir_all(cwd.join(".git/doxguard/hooks")).unwrap();
    let target = temp.path().join("sentinel");
    fs::write(&target, "unchanged").unwrap();
    symlink(&target, cwd.join(".git/doxguard/hooks/pre-commit")).unwrap();
    assert!(!run(&cwd, &["install-hooks"]).status.success());
    assert_eq!(fs::read_to_string(target).unwrap(), "unchanged");
}

#[test]
fn init_preserves_active_default_pre_push_hook() {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    for name in ["pre-commit", "pre-push", "commit-msg", "post-commit"] {
        let hook = cwd.join(".git/hooks").join(name);
        fs::write(&hook, "#!/bin/sh\nprintf synthetic-hook >> hook-ran.txt\n").unwrap();
        #[cfg(unix)]
        fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
        git(cwd, &["hook", "run", name]);
    }
    assert!(run(cwd, &["init"]).status.success());
    assert!(run(cwd, &["init"]).status.success());
    for name in ["pre-commit", "pre-push", "commit-msg", "post-commit"] {
        git(cwd, &["hook", "run", name]);
    }
    assert_eq!(
        fs::read_to_string(cwd.join("hook-ran.txt")).unwrap(),
        "synthetic-hook".repeat(8)
    );
    let setting = command("git", cwd)
        .args(["config", "--get", "core.hooksPath"])
        .output()
        .unwrap();
    assert_eq!(setting.status.code(), Some(1));
}

#[test]
fn packaged_workspace_cannot_alias_root_files() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    fs::create_dir_all(cwd.join("packages/a")).unwrap();
    fs::write(cwd.join("package.json"), r#"{"name":"synthetic-root","version":"1.0.0","workspaces":["packages/*"],"files":["policy.md"]}"#).unwrap();
    fs::write(cwd.join("packages/a/package.json"), r#"{"name":"synthetic-child","version":"1.0.0","files":["policy.md"],"scripts":{"prepack":"node -e \"require('fs').writeFileSync('script-ran','bad')\""}}"#).unwrap();
    fs::write(cwd.join("policy.md"), "safe alias").unwrap();
    fs::write(cwd.join("packages/a/policy.md"), "SyntheticNeedle").unwrap();
    let user = cwd.join("empty-user.npmrc");
    let global = cwd.join("empty-global.npmrc");
    fs::write(&user, "").unwrap();
    fs::write(&global, "").unwrap();
    let invoke = |directory: &Path| {
        command(env!("CARGO_BIN_EXE_doxguard"), cwd)
            .args([
                "scan",
                "--packaged",
                "--block",
                "--strict",
                "--format",
                "json",
                "--config",
            ])
            .arg(cwd.join("doxguard.config.json"))
            .current_dir(directory)
            .env_remove("DOXGUARD_CONFIG")
            .env_remove("NPM_TOKEN")
            .env_remove("NODE_AUTH_TOKEN")
            .env("NPM_CONFIG_USERCONFIG", &user)
            .env("NPM_CONFIG_GLOBALCONFIG", &global)
            .env("NPM_CONFIG_CACHE", cwd.join("npm-cache"))
            .env("NPM_CONFIG_OFFLINE", "true")
            .env("NPM_CONFIG_WORKSPACES", "true")
            .output()
            .unwrap()
    };
    let output = invoke(cwd);
    assert_eq!(
        output.status.code(),
        Some(2),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stderr).contains("unsupported"));
    assert!(!cwd.join("packages/a/script-ran").exists());
}

#[test]
fn packaged_single_package_still_scans() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    fs::write(
        cwd.join("package.json"),
        r#"{"name":"synthetic-standalone","version":"1.0.0","files":["policy.md"]}"#,
    )
    .unwrap();
    fs::write(cwd.join("policy.md"), "SyntheticNeedle").unwrap();
    let user = cwd.join("empty-user.npmrc");
    let global = cwd.join("empty-global.npmrc");
    fs::write(&user, "").unwrap();
    fs::write(&global, "").unwrap();
    let output = command(env!("CARGO_BIN_EXE_doxguard"), cwd)
        .args([
            "scan",
            "--packaged",
            "--block",
            "--strict",
            "--format",
            "json",
        ])
        .current_dir(cwd)
        .env_remove("DOXGUARD_CONFIG")
        .env_remove("NPM_TOKEN")
        .env_remove("NODE_AUTH_TOKEN")
        .env("NPM_CONFIG_USERCONFIG", user)
        .env("NPM_CONFIG_GLOBALCONFIG", global)
        .env("NPM_CONFIG_CACHE", cwd.join("npm-cache"))
        .env("NPM_CONFIG_OFFLINE", "true")
        .env_remove("NPM_CONFIG_WORKSPACES")
        .env_remove("NPM_CONFIG_WORKSPACE")
        .output()
        .unwrap();
    assert_eq!(
        output.status.code(),
        Some(1),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(!json["hits"].as_array().unwrap().is_empty());
}

#[cfg(windows)]
#[test]
fn init_rejects_junction_ancestor_without_external_writes() {
    let temp = tempdir().unwrap();
    let cwd = temp.path().join("repo");
    let external = temp.path().join("external");
    fs::create_dir(&cwd).unwrap();
    fs::create_dir(&external).unwrap();
    repo(&cwd);
    fs::write(external.join("sentinel"), "unchanged").unwrap();
    let output = Command::new("cmd")
        .args(["/C", "mklink", "/J"])
        .arg(cwd.join(".github"))
        .arg(&external)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction fixture unavailable: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let output = run(&cwd, &["init"]);
    assert!(!output.status.success());
    assert_eq!(fs::read_dir(&external).unwrap().count(), 1);
    assert!(!cwd.join("doxguard.config.json").exists());
    fs::remove_dir(cwd.join(".github")).unwrap();
}

#[cfg(unix)]
#[test]
fn init_rejects_dangling_leaf_before_creating_target() {
    use std::os::unix::fs::symlink;
    let temp = tempdir().unwrap();
    let cwd = temp.path().join("repo");
    fs::create_dir(&cwd).unwrap();
    repo(&cwd);
    let target = temp.path().join("must-not-exist");
    symlink(&target, cwd.join("doxguard.config.json")).unwrap();
    assert!(!run(&cwd, &["init"]).status.success());
    assert!(!target.exists());
    assert!(!cwd.join(".github").exists());
}

#[test]
fn staged_deletion_is_not_a_missing_content_error() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    fs::write(cwd.join("policy.md"), "safe").unwrap();
    git(cwd, &["add", "policy.md"]);
    git(cwd, &["commit", "-qm", "file"]);
    git(cwd, &["rm", "policy.md"]);
    let output = run(
        cwd,
        &[
            "scan", "--staged", "--block", "--strict", "--format", "json",
        ],
    );
    assert!(output.status.success());
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["total_files"], 0);
}

#[cfg(unix)]
#[test]
fn custom_hooks_path_trailing_space_is_not_an_alias() {
    use std::os::unix::fs::PermissionsExt;
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    let hooks = cwd.join(".git/doxguard/hooks ");
    fs::create_dir_all(&hooks).unwrap();
    let hook = hooks.join("pre-push");
    fs::write(&hook, "#!/bin/sh\necho preserved >> custom-ran.txt\n").unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    git(cwd, &["config", "core.hooksPath", hooks.to_str().unwrap()]);
    git(cwd, &["hook", "run", "pre-push"]);
    assert!(run(cwd, &["install-hooks"]).status.success());
    assert_eq!(
        git(cwd, &["config", "--get", "core.hooksPath"]),
        format!("{}\n", hooks.display())
    );
    git(cwd, &["hook", "run", "pre-push"]);
    assert_eq!(
        fs::read_to_string(cwd.join("custom-ran.txt")).unwrap(),
        "preserved\npreserved\n"
    );
}

#[test]
fn explicit_empty_hooks_setting_is_preserved() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    git(cwd, &["config", "core.hooksPath", ""]);
    assert!(run(cwd, &["install-hooks"]).status.success());
    assert_eq!(git(cwd, &["config", "--get", "core.hooksPath"]), "\n");
}

#[cfg(unix)]
#[test]
fn literal_backslash_does_not_alias_default_exempt_directory() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    let path = cwd.join("doxguard.config.json");
    let mut settings: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    settings["includeDefaultExemptPaths"] = serde_json::json!(true);
    fs::write(path, serde_json::to_vec(&settings).unwrap()).unwrap();
    fs::create_dir(cwd.join("tests")).unwrap();
    fs::write(cwd.join("tests/policy.md"), "safe alias").unwrap();
    fs::write(cwd.join("tests\\policy.md"), "SyntheticNeedle").unwrap();
    git(cwd, &["add", "--", "tests/policy.md", "tests\\policy.md"]);
    for mode in ["--staged", "--diff", "--all-tracked"] {
        let report = blocked(cwd, &[mode]);
        assert_eq!(report["hits"].as_array().unwrap().len(), 1);
        assert_eq!(report["hits"][0]["file"], "tests\\policy.md");
        assert_eq!(report["scanned"], 1);
        assert_eq!(report["exempt_or_skipped"], 1);
        assert_eq!(report["coverage_skips"], 0);
    }
}

#[cfg(unix)]
#[test]
fn literal_backslash_does_not_alias_custom_explicit_exemption() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    let path = cwd.join("doxguard.config.json");
    let mut settings: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    settings["exemptPaths"] = serde_json::json!(["tests/"]);
    fs::write(path, serde_json::to_vec(&settings).unwrap()).unwrap();
    fs::create_dir(cwd.join("tests")).unwrap();
    fs::write(cwd.join("tests/policy.md"), "safe alias").unwrap();
    fs::write(cwd.join("tests\\policy.md"), "SyntheticNeedle").unwrap();
    let report = blocked(cwd, &["--path", "tests\\policy.md"]);
    assert_eq!(report["hits"].as_array().unwrap().len(), 1);
    assert_eq!(report["scanned"], 1);
    let output = run(
        cwd,
        &[
            "scan",
            "--path",
            "tests/policy.md",
            "--block",
            "--strict",
            "--format",
            "json",
        ],
    );
    assert!(output.status.success());
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["scanned"], 0);
    assert_eq!(report["hits"].as_array().unwrap().len(), 0);
}

#[cfg(windows)]
#[test]
fn windows_exemption_accepts_native_and_git_separators() {
    use doxguard::scan::path_is_exempt;
    assert!(path_is_exempt("tests\\policy.md", "tests/"));
    assert!(path_is_exempt("tests/policy.md", "tests\\"));
    assert!(path_is_exempt(
        ".github\\workflows\\doxguard.yml",
        ".github/workflows/doxguard"
    ));
}

#[cfg(unix)]
#[test]
fn literal_backslash_does_not_alias_default_lockfile_basename() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    config(cwd, "SyntheticNeedle\n");
    let path = cwd.join("doxguard.config.json");
    let mut settings: serde_json::Value =
        serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    settings["includeDefaultExemptPaths"] = serde_json::json!(true);
    fs::write(path, serde_json::to_vec(&settings).unwrap()).unwrap();
    fs::write(cwd.join("Cargo.lock"), "safe real lockfile").unwrap();
    fs::write(cwd.join("nested\\Cargo.lock"), "SyntheticNeedle").unwrap();
    git(cwd, &["add", "--", "Cargo.lock", "nested\\Cargo.lock"]);
    for mode in ["--staged", "--diff", "--all-tracked"] {
        let report = blocked(cwd, &[mode]);
        assert_eq!(report["hits"].as_array().unwrap().len(), 1);
        assert_eq!(report["hits"][0]["file"], "nested\\Cargo.lock");
        assert_eq!(report["scanned"], 1);
        assert_eq!(report["exempt_or_skipped"], 1);
    }
}
