use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::tempdir;

fn binary() -> &'static Path {
    Path::new(env!("CARGO_BIN_EXE_doxguard"))
}

fn run(program: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
        // Hermetic: an author environment may export its own watchlist config.
        .env_remove("DOXGUARD_CONFIG")
        .output()
        .unwrap()
}

fn git(cwd: &Path, args: &[&str]) {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn init_repo(cwd: &Path) {
    git(cwd, &["init", "-q"]);
    git(cwd, &["config", "user.name", "Fixture Author"]);
    git(cwd, &["config", "user.email", "fixture@example.com"]);
}

fn assert_actions_are_commit_pinned(workflow: &str) {
    let mut count = 0;
    for line in workflow.lines() {
        let trimmed = line.trim();
        let Some(spec) = trimmed
            .strip_prefix("- uses: ")
            .or_else(|| trimmed.strip_prefix("uses: "))
        else {
            continue;
        };
        let spec = spec.split('#').next().unwrap().trim();
        if spec.starts_with("./") {
            continue;
        }
        count += 1;
        let revision = spec
            .rsplit_once('@')
            .unwrap_or_else(|| panic!("action reference has no revision: {spec}"))
            .1;
        assert!(
            revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "action must use a full commit SHA: {spec}"
        );
    }
    assert!(count > 0, "expected at least one remote action reference");
}

#[test]
fn init_scaffolds_and_reruns_idempotently() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());

    let initialized = run(binary(), temp.path(), &["init"]);
    assert!(initialized.status.success());
    assert!(temp.path().join("doxguard.config.json").exists());
    assert!(temp.path().join(".githooks/pre-commit").exists());
    assert!(temp.path().join(".github/workflows/doxguard.yml").exists());
    assert!(temp.path().join(".git/doxguard/hooks/pre-commit").exists());
    assert_actions_are_commit_pinned(
        &fs::read_to_string(temp.path().join(".github/workflows/doxguard.yml")).unwrap(),
    );
    assert!(
        fs::read_to_string(temp.path().join(".github/workflows/doxguard.yml"))
            .unwrap()
            .contains(&format!("doxguard@{} ", env!("CARGO_PKG_VERSION")))
    );
    let hooks_path = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(
        String::from_utf8_lossy(&hooks_path.stdout)
            .replace('\\', "/")
            .contains("/.git/doxguard/hooks")
    );

    let initialized_again = run(binary(), temp.path(), &["init"]);
    assert!(initialized_again.status.success());
    assert!(String::from_utf8_lossy(&initialized_again.stdout).contains("SKIPPED"));
}

#[test]
fn staged_scan_blocks_and_directive_allows() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());

    fs::write(temp.path().join("fixture.txt"), "server=192.168.50.9\n").unwrap();
    git(temp.path(), &["add", "fixture.txt"]);
    let blocked = run(binary(), temp.path(), &["scan", "--staged", "--block"]);
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "unexpected stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("RFC1918"));

    fs::write(
        temp.path().join("fixture.txt"),
        "server=192.168.50.9 # doxguard: allow 192.168.50.9\n",
    )
    .unwrap();
    git(temp.path(), &["add", "fixture.txt"]);
    let passed = run(
        binary(),
        temp.path(),
        &["scan", "--staged", "--block", "--format", "json"],
    );
    assert!(passed.status.success());
    let json: serde_json::Value = serde_json::from_slice(&passed.stdout).unwrap();
    assert_eq!(json["hits"].as_array().unwrap().len(), 0);
}

#[test]
fn native_hook_runs_on_commit() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());

    let initialized = run(binary(), temp.path(), &["init"]);
    assert!(initialized.status.success());
    fs::write(
        temp.path().join("fixture.txt"),
        "server=192.168.50.9 # doxguard: allow 192.168.50.9\n",
    )
    .unwrap();
    git(temp.path(), &["add", "fixture.txt"]);
    let hook_run = Command::new("git")
        .args(["commit", "-m", "synthetic fixture"])
        .current_dir(temp.path())
        // The hook child inherits this environment too.
        .env_remove("DOXGUARD_CONFIG")
        .output()
        .unwrap();
    assert!(
        hook_run.status.success(),
        "direct native hook failed: {}",
        String::from_utf8_lossy(&hook_run.stderr)
    );

    #[cfg(windows)]
    {
        let git_bash = Path::new(r"C:\Program Files\Git\bin\bash.exe");
        if git_bash.exists() {
            let syntax = Command::new(git_bash)
                .args(["-n", ".githooks/pre-commit"])
                .current_dir(temp.path())
                .output()
                .unwrap();
            assert!(
                syntax.status.success(),
                "Git Bash rejected hook: {}",
                String::from_utf8_lossy(&syntax.stderr)
            );
        }
    }
}

#[test]
fn install_hooks_preserves_existing_hooks_path() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    git(temp.path(), &["config", "core.hooksPath", ".my-hooks"]);

    let output = run(binary(), temp.path(), &["install-hooks"]);
    assert!(output.status.success());
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("SKIPPED"), "stdout: {stdout}");
    assert!(stdout.contains("left unchanged"), "stdout: {stdout}");
    // The native cache is still prepared for the user's own hook to call.
    assert!(temp.path().join(".git/doxguard/hooks/pre-commit").exists());

    let hooks_path = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8_lossy(&hooks_path.stdout).trim(),
        ".my-hooks"
    );
}

#[test]
fn cached_hook_can_reinstall_without_copying_over_itself() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let installed = run(binary(), temp.path(), &["install-hooks"]);
    assert!(installed.status.success());
    let cached = temp.path().join(".git/doxguard/hooks/pre-commit");
    let before = fs::metadata(&cached).unwrap().len();
    assert!(before > 0);

    let reinstalled = run(&cached, temp.path(), &["install-hooks"]);

    assert!(
        reinstalled.status.success(),
        "cached binary must not copy over itself; stderr: {}",
        String::from_utf8_lossy(&reinstalled.stderr)
    );
    assert_eq!(fs::metadata(&cached).unwrap().len(), before);
}

#[test]
fn strict_blocks_on_coverage_skips_end_to_end() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(
        temp.path().join("doxguard.config.json"),
        "{\"maxFileSize\": 16}\n",
    )
    .unwrap();
    fs::write(temp.path().join("big.txt"), "x".repeat(64)).unwrap();
    git(temp.path(), &["add", "big.txt"]);

    let lenient = run(binary(), temp.path(), &["scan", "--all-tracked", "--block"]);
    assert!(
        lenient.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&lenient.stderr)
    );

    let strict = run(
        binary(),
        temp.path(),
        &["scan", "--all-tracked", "--block", "--strict"],
    );
    assert_eq!(
        strict.status.code(),
        Some(1),
        "strict must fail closed on coverage skips; stderr: {}",
        String::from_utf8_lossy(&strict.stderr)
    );
    assert!(!String::from_utf8_lossy(&strict.stdout).contains("OK:"));
    assert!(String::from_utf8_lossy(&strict.stderr).contains("INCOMPLETE:"));
}

#[test]
fn nested_invocation_uses_repository_root_for_config_and_tracked_files() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::create_dir(temp.path().join("nested")).unwrap();
    fs::write(temp.path().join("names.txt"), "SyntheticIdentityNeedle\n").unwrap();
    fs::write(
        temp.path().join("doxguard.config.json"),
        r#"{"watchlists":[{"type":"lines","path":"names.txt","label":"synthetic fixture"}]}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("sibling.txt"),
        "owner=SyntheticIdentityNeedle\n",
    )
    .unwrap();
    git(temp.path(), &["add", "."]);

    let blocked = run(
        binary(),
        &temp.path().join("nested"),
        &["scan", "--all-tracked", "--block"],
    );
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "nested invocation must load the root config and scan sibling files; stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("sibling.txt"));
}

#[test]
fn nested_explicit_config_keeps_relative_watchlists_at_invocation_directory() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let nested = temp.path().join("nested");
    fs::create_dir(&nested).unwrap();
    fs::write(nested.join("names.txt"), "SyntheticExplicitNeedle\n").unwrap();
    fs::write(
        nested.join("local.json"),
        r#"{"watchlists":[{"type":"lines","path":"names.txt","label":"synthetic explicit"}]}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("sibling.txt"),
        "owner=SyntheticExplicitNeedle\n",
    )
    .unwrap();
    git(temp.path(), &["add", "sibling.txt"]);

    let blocked = run(
        binary(),
        &nested,
        &["scan", "--all-tracked", "--block", "--config", "local.json"],
    );

    assert_eq!(
        blocked.status.code(),
        Some(1),
        "explicit config and its relative watchlist must retain invocation-cwd semantics; stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("sibling.txt"));
}

#[test]
fn cli_masks_watchlist_values_and_paths_unless_explicitly_requested() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::create_dir(temp.path().join("private-fixture")).unwrap();
    fs::write(
        temp.path().join("private-fixture/watchlist.txt"),
        "SyntheticIdentityNeedle\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("doxguard.config.json"),
        r#"{"watchlists":[{"type":"lines","path":"private-fixture/watchlist.txt"}]}"#,
    )
    .unwrap();
    fs::write(
        temp.path().join("target.txt"),
        "owner=SyntheticIdentityNeedle\n",
    )
    .unwrap();
    git(temp.path(), &["add", "doxguard.config.json", "target.txt"]);

    let text = run(binary(), temp.path(), &["scan", "--all-tracked", "--block"]);
    let text_stderr = String::from_utf8_lossy(&text.stderr);
    assert_eq!(text.status.code(), Some(1), "stderr: {text_stderr}");
    assert!(text_stderr.contains("[REDACTED]"));
    assert!(text_stderr.contains("watchlist source #1"));
    assert!(!text_stderr.contains("SyntheticIdentityNeedle"));
    assert!(!text_stderr.contains("private-fixture/watchlist.txt"));

    let json = run(
        binary(),
        temp.path(),
        &["scan", "--all-tracked", "--block", "--format", "json"],
    );
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    assert_eq!(value["hits"][0]["matched"], "[REDACTED]");
    assert!(!String::from_utf8_lossy(&json.stdout).contains("SyntheticIdentityNeedle"));
    assert!(!String::from_utf8_lossy(&json.stderr).contains("private-fixture/watchlist.txt"));

    let explicit = run(
        binary(),
        temp.path(),
        &["scan", "--all-tracked", "--block", "--show-matched"],
    );
    assert!(String::from_utf8_lossy(&explicit.stderr).contains("SyntheticIdentityNeedle"));

    let explicit_json = run(
        binary(),
        temp.path(),
        &[
            "scan",
            "--all-tracked",
            "--block",
            "--show-matched",
            "--format",
            "json",
        ],
    );
    let value: serde_json::Value = serde_json::from_slice(&explicit_json.stdout).unwrap();
    assert_eq!(value["hits"][0]["matched"], "SyntheticIdentityNeedle");
}

#[test]
fn cli_escapes_terminal_and_bidi_controls_in_labels() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(
        temp.path().join("watchlist.txt"),
        "SyntheticControlNeedle\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("doxguard.config.json"),
        r#"{"watchlists":[{"type":"lines","path":"watchlist.txt","label":"fixture\u001b[31m\u202e"}]}"#,
    )
    .unwrap();
    fs::write(temp.path().join("target.txt"), "SyntheticControlNeedle\n").unwrap();
    git(temp.path(), &["add", "doxguard.config.json", "target.txt"]);

    let output = run(binary(), temp.path(), &["scan", "--all-tracked", "--block"]);
    assert_eq!(output.status.code(), Some(1));
    assert!(!output.stderr.contains(&0x1b));
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(!stderr.contains('\u{202e}'));
    assert!(stderr.contains(r"\u{1b}"), "stderr: {stderr}");
    assert!(stderr.contains(r"\u{202e}"), "stderr: {stderr}");

    let json = run(
        binary(),
        temp.path(),
        &["scan", "--all-tracked", "--block", "--format", "json"],
    );
    assert!(!json.stdout.contains(&0x1b));
    let value: serde_json::Value = serde_json::from_slice(&json.stdout).unwrap();
    let source = value["hits"][0]["source"].as_str().unwrap();
    assert!(!source.contains('\u{202e}'));
    assert!(source.contains(r"\u{1b}"));
    assert!(source.contains(r"\u{202e}"));
}

#[test]
fn native_hook_enables_strict_coverage_gate() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let initialized = run(binary(), temp.path(), &["init"]);
    assert!(initialized.status.success());
    fs::write(
        temp.path().join("doxguard.config.json"),
        "{\"maxFileSize\":16}\n",
    )
    .unwrap();
    fs::write(temp.path().join("big.txt"), "x".repeat(64)).unwrap();
    git(temp.path(), &["add", "big.txt"]);

    let commit = Command::new("git")
        .args(["commit", "-m", "synthetic oversized fixture"])
        .current_dir(temp.path())
        .env_remove("DOXGUARD_CONFIG")
        .output()
        .unwrap();
    assert_eq!(
        commit.status.code(),
        Some(1),
        "native hook must fail closed on coverage skips; stderr: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
    assert!(String::from_utf8_lossy(&commit.stderr).contains("INCOMPLETE:"));
}

#[test]
fn native_hook_blocks_non_utf8_staged_content() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let initialized = run(binary(), temp.path(), &["init"]);
    assert!(initialized.status.success());
    fs::write(temp.path().join("non-utf8.txt"), vec![b'x', 0x80]).unwrap();
    git(temp.path(), &["add", "non-utf8.txt"]);

    let commit = Command::new("git")
        .args(["commit", "-m", "synthetic non-UTF8 fixture"])
        .current_dir(temp.path())
        .env_remove("DOXGUARD_CONFIG")
        .output()
        .unwrap();
    assert_eq!(
        commit.status.code(),
        Some(1),
        "native hook must fail closed on non-UTF-8 staged content; stderr: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
    assert!(String::from_utf8_lossy(&commit.stderr).contains("INCOMPLETE:"));
}

#[test]
fn staged_scan_includes_type_changes() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(temp.path().join("fixture.txt"), "safe\n").unwrap();
    git(temp.path(), &["add", "fixture.txt"]);
    git(temp.path(), &["commit", "-q", "-m", "base"]);

    let blob_source = temp.path().join("link-blob.txt");
    fs::write(&blob_source, "192.168.50.9\n").unwrap();
    let hash = Command::new("git")
        .args(["hash-object", "-w", "link-blob.txt"])
        .current_dir(temp.path())
        .output()
        .unwrap();
    assert!(hash.status.success());
    let hash = String::from_utf8(hash.stdout).unwrap();
    let cache_info = format!("120000,{},fixture.txt", hash.trim());
    git(temp.path(), &["update-index", "--cacheinfo", &cache_info]);

    let blocked = run(binary(), temp.path(), &["scan", "--staged", "--block"]);
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "type-change blob must be scanned; stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("RFC1918"));
}

#[test]
fn staged_scan_preserves_leading_whitespace_in_filename() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let filename = " leading-space.txt";
    fs::write(temp.path().join(filename), "server=192.168.50.9\n").unwrap();
    git(temp.path(), &["add", filename]);

    let blocked = run(binary(), temp.path(), &["scan", "--staged", "--block"]);
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "leading whitespace must remain part of the Git path; stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("RFC1918"));
}

#[cfg(unix)]
#[test]
fn staged_scan_uses_explicit_stage_zero_for_colon_prefixed_paths() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let filename = "1:fixture.txt";
    fs::write(temp.path().join(filename), "server=192.168.50.9\n").unwrap();
    git(temp.path(), &["add", filename]);

    let blocked = run(binary(), temp.path(), &["scan", "--staged", "--block"]);

    assert_eq!(
        blocked.status.code(),
        Some(1),
        "stage-zero blob must be read for colon-prefixed paths; stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
}

#[test]
fn generated_hook_is_scanned_instead_of_implicitly_exempted() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let initialized = run(binary(), temp.path(), &["init"]);
    assert!(initialized.status.success());

    let hook_path = temp.path().join(".githooks/pre-commit");
    let mut hook = fs::read_to_string(&hook_path).unwrap();
    hook.push_str("# synthetic=192.168.50.9\n");
    fs::write(&hook_path, hook).unwrap();
    git(temp.path(), &["add", ".githooks/pre-commit"]);

    let blocked = run(
        binary(),
        temp.path(),
        &["scan", "--staged", "--block", "--strict"],
    );
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "generated hook must not be a built-in exemption; stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("RFC1918"));
}

#[cfg(unix)]
#[test]
fn staged_scan_handles_newline_in_git_filename() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    let filename = "line\nbreak.txt";
    fs::write(temp.path().join(filename), "server=192.168.50.9\n").unwrap();
    git(temp.path(), &["add", filename]);

    let blocked = run(binary(), temp.path(), &["scan", "--staged", "--block"]);
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "newline filename must remain one Git path; stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("RFC1918"));
}

#[test]
fn diff_scan_blocks_unstaged_changes() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(temp.path().join("fixture.txt"), "safe\n").unwrap();
    git(temp.path(), &["add", "fixture.txt"]);
    git(temp.path(), &["commit", "-q", "-m", "synthetic fixture"]);

    fs::write(temp.path().join("fixture.txt"), "server=192.168.50.9\n").unwrap();
    let blocked = run(binary(), temp.path(), &["scan", "--diff", "--block"]);
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("RFC1918"));
}

#[test]
fn packaged_scan_blocks_leaky_pack_contents() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("package.json"),
        "{\"name\":\"synthetic-fixture-pkg\",\"version\":\"0.0.0\",\"files\":[\"index.js\"]}\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("index.js"),
        "const host = \"192.168.50.9\";\n",
    )
    .unwrap();

    let blocked = run(binary(), temp.path(), &["scan", "--packaged", "--block"]);
    assert_eq!(
        blocked.status.code(),
        Some(1),
        "stderr: {}",
        String::from_utf8_lossy(&blocked.stderr)
    );
    assert!(String::from_utf8_lossy(&blocked.stderr).contains("RFC1918"));
}

#[test]
fn cwd_planted_git_is_not_executed() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    // A malicious repository plants a fake `git` where Windows CreateProcess
    // would find it first (cwd). It is not a valid executable, so any attempt
    // to launch it fails loudly — a passing scan proves it was never used.
    fs::write(temp.path().join("git.exe"), "not a real executable\n").unwrap();
    fs::write(temp.path().join("git"), "not a real executable\n").unwrap();

    let output = run(binary(), temp.path(), &["scan", "--staged", "--block"]);
    assert!(
        output.status.success(),
        "scan must resolve git from PATH, not cwd; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn empty_config_environment_variable_is_treated_as_unset() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(temp.path().join("safe.txt"), "safe\n").unwrap();
    git(temp.path(), &["add", "safe.txt"]);

    let output = Command::new(binary())
        .args(["scan", "--all-tracked", "--block"])
        .current_dir(temp.path())
        .env("DOXGUARD_CONFIG", "")
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "empty config env must not resolve to the repository directory; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[cfg(unix)]
#[test]
fn unrelated_non_unicode_environment_value_does_not_panic() {
    use std::{ffi::OsString, os::unix::ffi::OsStringExt};

    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(temp.path().join("safe.txt"), "safe\n").unwrap();
    git(temp.path(), &["add", "safe.txt"]);

    let output = Command::new(binary())
        .args(["scan", "--all-tracked", "--block"])
        .current_dir(temp.path())
        .env_remove("DOXGUARD_CONFIG")
        .env(
            "SYNTHETIC_NON_UNICODE_ENV",
            OsString::from_vec(vec![0x66, 0x80]),
        )
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "unrelated non-Unicode env must be ignored; stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn generated_hook_is_portable_and_contains_no_repository_absolute_path() {
    let temp = tempdir().unwrap();
    // `"` is not a legal NTFS filename character, so it is exercised on Unix only.
    let name = if cfg!(windows) {
        "d$(evil)`tick"
    } else {
        "d$(evil)`tick\"quote"
    };
    let evil = temp.path().join(name);
    fs::create_dir(&evil).unwrap();
    init_repo(&evil);

    let output = run(binary(), &evil, &["install-hooks"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let script = fs::read_to_string(evil.join(".githooks/pre-commit")).unwrap();
    assert!(!script.contains("native_abs="));
    assert!(!script.contains(evil.to_string_lossy().as_ref()));
    assert!(script.contains("$git_dir/doxguard/hooks/pre-commit"));

    #[cfg(unix)]
    {
        let syntax = Command::new("sh")
            .args(["-n", ".githooks/pre-commit"])
            .current_dir(&evil)
            .output()
            .unwrap();
        assert!(
            syntax.status.success(),
            "sh rejected hook: {}",
            String::from_utf8_lossy(&syntax.stderr)
        );
    }
    #[cfg(windows)]
    {
        let git_bash = Path::new(r"C:\Program Files\Git\bin\bash.exe");
        if git_bash.exists() {
            let syntax = Command::new(git_bash)
                .args(["-n", ".githooks/pre-commit"])
                .current_dir(&evil)
                .output()
                .unwrap();
            assert!(
                syntax.status.success(),
                "Git Bash rejected hook: {}",
                String::from_utf8_lossy(&syntax.stderr)
            );
        }
    }
}

#[cfg(unix)]
#[test]
fn init_refuses_to_scaffold_through_parent_symlink() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    init_repo(temp.path());
    symlink(outside.path(), temp.path().join(".github")).unwrap();

    let output = run(binary(), temp.path(), &["init"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!outside.path().join("workflows/doxguard.yml").exists());
}

#[cfg(unix)]
#[test]
fn init_skips_dangling_leaf_symlink_without_creating_its_target() {
    use std::os::unix::fs::symlink;

    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    init_repo(temp.path());
    fs::create_dir_all(temp.path().join(".github/workflows")).unwrap();
    let external_target = outside.path().join("doxguard.yml");
    symlink(
        &external_target,
        temp.path().join(".github/workflows/doxguard.yml"),
    )
    .unwrap();

    let output = run(binary(), temp.path(), &["init"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(!external_target.exists());
    assert!(String::from_utf8_lossy(&output.stdout).contains("SKIPPED"));
}

#[cfg(windows)]
#[test]
fn init_refuses_to_scaffold_through_parent_junction() {
    // A junction (mount point) is a name-surrogate reparse point and must still be
    // rejected after is_link_like stopped treating every reparse point as a link
    // (so cloud placeholders are allowed). This also fails on the pinned MSRV in CI
    // if a toolchain ever stopped reporting junctions as symlinks.
    let temp = tempdir().unwrap();
    let outside = tempdir().unwrap();
    init_repo(temp.path());
    let link = temp.path().join(".github");
    let status = Command::new("cmd")
        .args(["/c", "mklink", "/J"])
        .arg(&link)
        .arg(outside.path())
        .status()
        .unwrap();
    assert!(status.success(), "failed to create junction fixture");

    let output = run(binary(), temp.path(), &["init"]);
    assert_eq!(output.status.code(), Some(2));
    assert!(!outside.path().join("workflows/doxguard.yml").exists());
}

#[test]
fn install_hooks_from_linked_worktree_targets_common_git_dir() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(temp.path().join("seed.txt"), "seed\n").unwrap();
    git(temp.path(), &["add", "seed.txt"]);
    git(temp.path(), &["commit", "-q", "-m", "seed"]);

    let wt = temp.path().join("wt");
    git(
        temp.path(),
        &[
            "worktree",
            "add",
            "-q",
            wt.to_str().unwrap(),
            "-b",
            "feature",
        ],
    );

    // Install FROM the linked worktree: the F-B02 bug wrote a per-worktree hooksPath
    // into the shared config, which vanished when the worktree was removed.
    let output = run(binary(), &wt, &["install-hooks"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let hooks_path = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(&wt)
        .output()
        .unwrap();
    let hp = String::from_utf8_lossy(&hooks_path.stdout)
        .trim()
        .replace('\\', "/");
    assert!(
        hp.contains("/.git/doxguard/hooks"),
        "hooksPath must target the common git dir: {hp}"
    );
    assert!(
        !hp.contains("/worktrees/"),
        "hooksPath must not be per-worktree: {hp}"
    );
    assert!(temp.path().join(".git/doxguard/hooks/pre-commit").exists());

    // A commit from the MAIN worktree is gated by the shared hook.
    fs::write(temp.path().join("leak.txt"), "192.168.50.9\n").unwrap();
    git(temp.path(), &["add", "leak.txt"]);
    let commit = Command::new("git")
        .args(["commit", "-m", "synthetic leak"])
        .current_dir(temp.path())
        .env_remove("DOXGUARD_CONFIG")
        .output()
        .unwrap();
    assert_eq!(
        commit.status.code(),
        Some(1),
        "main-worktree commit must be gated by the shared hook; stderr: {}",
        String::from_utf8_lossy(&commit.stderr)
    );
}

#[test]
fn scan_without_config_warns_structural_only() {
    let temp = tempdir().unwrap();
    init_repo(temp.path());
    fs::write(temp.path().join("safe.txt"), "safe\n").unwrap();
    git(temp.path(), &["add", "safe.txt"]);

    let output = run(binary(), temp.path(), &["scan", "--all-tracked"]);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("structural patterns only"),
        "missing structural-only warning; stderr: {stderr}"
    );
}

#[test]
fn usage_errors_exit_two() {
    let temp = tempdir().unwrap();
    let output = run(binary(), temp.path(), &["scan", "--staged", "--diff"]);
    assert_eq!(output.status.code(), Some(2));
}
