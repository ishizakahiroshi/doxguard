use std::{
    fs,
    path::Path,
    process::{Command, Output},
};

use tempfile::tempdir;

fn run(program: &Path, cwd: &Path, args: &[&str]) -> Output {
    Command::new(program)
        .args(args)
        .current_dir(cwd)
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

#[test]
fn init_install_hooks_and_staged_scan_work_end_to_end() {
    let binary = Path::new(env!("CARGO_BIN_EXE_doxguard"));
    let temp = tempdir().unwrap();
    git(temp.path(), &["init", "-q"]);
    git(temp.path(), &["config", "user.name", "Fixture Author"]);
    git(
        temp.path(),
        &["config", "user.email", "fixture@example.com"],
    );

    let initialized = run(binary, temp.path(), &["init"]);
    assert!(initialized.status.success());
    assert!(temp.path().join("doxguard.config.json").exists());
    assert!(temp.path().join(".githooks/pre-commit").exists());
    assert!(temp.path().join(".github/workflows/doxguard.yml").exists());
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
    assert!(temp.path().join(".git/doxguard/hooks/pre-commit").exists());

    let initialized_again = run(binary, temp.path(), &["init"]);
    assert!(initialized_again.status.success());
    assert!(String::from_utf8_lossy(&initialized_again.stdout).contains("SKIPPED"));

    fs::write(temp.path().join("fixture.txt"), "server=192.168.50.9\n").unwrap();
    git(temp.path(), &["add", "fixture.txt"]);
    let blocked = run(binary, temp.path(), &["scan", "--staged", "--block"]);
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
        binary,
        temp.path(),
        &["scan", "--staged", "--block", "--format", "json"],
    );
    assert!(passed.status.success());
    let json: serde_json::Value = serde_json::from_slice(&passed.stdout).unwrap();
    assert_eq!(json["hits"].as_array().unwrap().len(), 0);

    let hook_run = Command::new("git")
        .args(["commit", "-m", "synthetic fixture"])
        .current_dir(temp.path())
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
fn usage_errors_exit_two() {
    let binary = Path::new(env!("CARGO_BIN_EXE_doxguard"));
    let temp = tempdir().unwrap();
    let output = run(binary, temp.path(), &["scan", "--staged", "--diff"]);
    assert_eq!(output.status.code(), Some(2));
}
