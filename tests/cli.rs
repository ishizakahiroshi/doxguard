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
fn hook_script_quotes_paths_with_shell_metacharacters() {
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
    let assignment = script
        .lines()
        .find(|line| line.starts_with("native_abs="))
        .expect("hook must assign native_abs");
    assert!(
        assignment.starts_with("native_abs='") && assignment.ends_with('\''),
        "native_abs must be single-quoted: {assignment}"
    );

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

#[test]
fn usage_errors_exit_two() {
    let temp = tempdir().unwrap();
    let output = run(binary(), temp.path(), &["scan", "--staged", "--diff"]);
    assert_eq!(output.status.code(), Some(2));
}
