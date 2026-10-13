//! Rollout regressions use isolated synthetic repositories and hooks only.
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

fn repo(cwd: &Path) {
    git(cwd, &["init", "-q"]);
    git(cwd, &["config", "user.name", "Synthetic Fixture"]);
    git(cwd, &["config", "user.email", "fixture@example.com"]);
    git(
        cwd,
        &["commit", "--allow-empty", "-qm", "synthetic baseline"],
    );
}

fn run(cwd: &Path, args: &[&str]) -> Output {
    command(env!("CARGO_BIN_EXE_doxguard"), cwd)
        .args(args)
        .output()
        .unwrap()
}

#[test]
fn staged_blob_reads_ignore_git_replace_refs() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    fs::write(cwd.join("needles.txt"), "SyntheticNeedle\n").unwrap();
    fs::write(
        cwd.join("doxguard.config.json"),
        r#"{
          "watchlists":[{"type":"lines","path":"needles.txt","label":"synthetic"}],
          "requireWatchlists":true,"includeDefaultExemptPaths":false,
          "structural":{"windowsPath":false,"posixHome":false,"privateIp":false,"email":false}
        }"#,
    )
    .unwrap();
    fs::write(cwd.join("policy.md"), "SyntheticNeedle\n").unwrap();
    git(cwd, &["add", "--", "policy.md"]);
    let indexed = git(cwd, &["rev-parse", ":policy.md"]);
    fs::write(cwd.join("safe.txt"), "safe replacement\n").unwrap();
    let replacement = git(cwd, &["hash-object", "-w", "safe.txt"]);
    git(cwd, &["replace", indexed.trim(), replacement.trim()]);

    // Prove the fixture actually replaces ordinary Git reads, while the
    // immutable object recorded in the index still contains the watchlist term.
    assert_eq!(
        git(cwd, &["cat-file", "blob", indexed.trim()]),
        "safe replacement\n"
    );
    assert_eq!(
        git(
            cwd,
            &["--no-replace-objects", "cat-file", "blob", indexed.trim()]
        ),
        "SyntheticNeedle\n"
    );
    assert!(git(cwd, &["ls-files", "--stage"]).contains(indexed.trim()));
    fs::write(cwd.join("policy.md"), "safe worktree\n").unwrap();

    let output = run(
        cwd,
        &[
            "scan",
            "--staged",
            "--block",
            "--strict",
            "--format",
            "json",
            "--show-matches",
        ],
    );
    assert_eq!(
        output.status.code(),
        Some(1),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["total_files"], 1);
    assert_eq!(report["scanned"], 1);
    assert_eq!(report["coverage_skips"], 0);
    assert_eq!(report["hits"].as_array().unwrap().len(), 1);
    assert_eq!(report["hits"][0]["file"], "policy.md");
    assert_eq!(report["hits"][0]["matched"], "SyntheticNeedle");
}

#[cfg(unix)]
#[test]
fn custom_hooks_path_preserves_symlink_parent_resolution() {
    use std::{
        os::unix::fs::{PermissionsExt, symlink},
        path::{Component, PathBuf},
    };

    let temp = tempdir().unwrap();
    let cwd = temp.path().join("repo");
    fs::create_dir(&cwd).unwrap();
    let cwd = cwd.canonicalize().unwrap();
    repo(&cwd);
    let external = temp.path().join("external");
    fs::create_dir_all(external.join("inner")).unwrap();
    fs::create_dir(external.join("hooks")).unwrap();
    let external = external.canonicalize().unwrap();
    fs::create_dir_all(cwd.join(".git/doxguard")).unwrap();
    symlink(external.join("inner"), cwd.join(".git/doxguard/link")).unwrap();

    let raw_hooks = cwd.join(".git/doxguard/link/../hooks");
    let mut collapsed = PathBuf::new();
    for component in raw_hooks.components() {
        match component {
            Component::ParentDir => {
                collapsed.pop();
            }
            Component::CurDir => {}
            other => collapsed.push(other.as_os_str()),
        }
    }
    assert_eq!(collapsed, cwd.join(".git/doxguard/hooks"));
    assert_eq!(raw_hooks.canonicalize().unwrap(), external.join("hooks"));

    let hook = external.join("hooks/pre-push");
    let hook_bytes = b"#!/bin/sh\nprintf x >> custom-hook-ran.txt\n";
    fs::write(&hook, hook_bytes).unwrap();
    fs::set_permissions(&hook, fs::Permissions::from_mode(0o755)).unwrap();
    git(
        &cwd,
        &["config", "core.hooksPath", raw_hooks.to_str().unwrap()],
    );
    git(&cwd, &["hook", "run", "pre-push"]);

    let output = run(&cwd, &["install-hooks"]);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        git(&cwd, &["config", "--get", "core.hooksPath"]),
        format!("{}\n", raw_hooks.display())
    );
    git(&cwd, &["hook", "run", "pre-push"]);
    assert_eq!(fs::read(cwd.join("custom-hook-ran.txt")).unwrap(), b"xx");
    assert_eq!(fs::read(hook).unwrap(), hook_bytes);
}

#[cfg(windows)]
#[test]
fn husky_init_rejects_linked_cache_before_any_output_write() {
    // Remove only the junction itself, even if an assertion panics. Its target
    // is another synthetic directory within this test's temporary root.
    struct Junction(std::path::PathBuf);
    impl Drop for Junction {
        fn drop(&mut self) {
            let _ = fs::remove_dir(&self.0);
        }
    }

    let temp = tempdir().unwrap();
    let cwd = temp.path().join("repo");
    let external = temp.path().join("external");
    fs::create_dir(&cwd).unwrap();
    fs::create_dir(&external).unwrap();
    repo(&cwd);
    fs::create_dir(cwd.join(".husky")).unwrap();
    let husky_bytes = b"#!/bin/sh\nprintf synthetic-husky\n";
    fs::write(cwd.join(".husky/pre-commit"), husky_bytes).unwrap();
    git(&cwd, &["config", "core.hooksPath", ".husky"]);
    let git_config_before = fs::read(cwd.join(".git/config")).unwrap();
    fs::write(external.join("sentinel"), "unchanged").unwrap();
    let link = cwd.join(".git/doxguard");
    let output = command("pwsh", &cwd)
        .args([
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$ErrorActionPreference = 'Stop'; New-Item -ItemType Junction -Path $env:DOXGUARD_FIXTURE_LINK -Target $env:DOXGUARD_FIXTURE_TARGET | Out-Null",
        ])
        .env("DOXGUARD_FIXTURE_LINK", &link)
        .env("DOXGUARD_FIXTURE_TARGET", &external)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "junction fixture unavailable: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let junction = Junction(link);

    let output = run(&cwd, &["init"]);
    assert_eq!(
        output.status.code(),
        Some(2),
        "stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        fs::read(cwd.join(".git/config")).unwrap(),
        git_config_before
    );
    assert_eq!(
        git(&cwd, &["config", "--get", "core.hooksPath"]),
        ".husky\n"
    );
    assert_eq!(
        fs::read(cwd.join(".husky/pre-commit")).unwrap(),
        husky_bytes
    );
    for path in [
        "doxguard.config.json",
        "doxguard.ci.json",
        ".github",
        ".githooks",
    ] {
        assert!(!cwd.join(path).exists(), "partial init output: {path}");
    }
    assert_eq!(fs::read_dir(&external).unwrap().count(), 1);
    assert_eq!(fs::read(external.join("sentinel")).unwrap(), b"unchanged");
    drop(junction);
}
