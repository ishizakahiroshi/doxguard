use std::{fs, path::Path, process::Command};

use tempfile::tempdir;

#[test]
fn generated_ci_config_scans_without_a_private_watchlist_and_is_not_overwritten() {
    let repo = tempdir().unwrap();
    let git = Command::new("git")
        .args(["init", "-q"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert!(git.status.success());
    let binary = Path::new(env!("CARGO_BIN_EXE_doxguard"));
    let run = |args: &[&str]| {
        Command::new(binary)
            .args(args)
            .current_dir(repo.path())
            .env_remove("DOXGUARD_CONFIG")
            .env_remove("DOXGUARD_WATCHLIST_DIR")
            .env_remove("DOXGUARD_SHOW_PATHS")
            .output()
            .unwrap()
    };
    assert!(run(&["init"]).status.success());
    let config_path = repo.path().join("doxguard.ci.json");
    let config = fs::read_to_string(&config_path).unwrap();
    let workflow = fs::read_to_string(repo.path().join(".github/workflows/doxguard.yml")).unwrap();
    assert!(workflow.contains("--config doxguard.ci.json"));
    assert!(
        run(&[
            "scan",
            "--all-tracked",
            "--block",
            "--strict",
            "--config",
            "doxguard.ci.json"
        ])
        .status
        .success()
    );
    fs::write(&config_path, format!("{config}\n")).unwrap();
    assert!(run(&["init"]).status.success());
    assert_eq!(
        fs::read_to_string(config_path).unwrap(),
        format!("{config}\n")
    );
}

#[test]
fn husky_hook_is_preserved_while_native_cache_is_prepared() {
    let repo = tempdir().unwrap();
    assert!(
        Command::new("git")
            .args(["init", "-q"])
            .current_dir(repo.path())
            .status()
            .unwrap()
            .success()
    );
    fs::create_dir(repo.path().join(".husky")).unwrap();
    let hook = repo.path().join(".husky/pre-commit");
    let original = "#!/bin/sh\nprintf 'existing check\\n'\n";
    fs::write(&hook, original).unwrap();
    assert!(
        Command::new("git")
            .args(["config", "core.hooksPath", ".husky"])
            .current_dir(repo.path())
            .status()
            .unwrap()
            .success()
    );
    let output = Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .arg("install-hooks")
        .current_dir(repo.path())
        .env_remove("DOXGUARD_CONFIG")
        .env_remove("DOXGUARD_WATCHLIST_DIR")
        .env_remove("DOXGUARD_SHOW_PATHS")
        .output()
        .unwrap();
    assert!(output.status.success());
    assert_eq!(fs::read_to_string(hook).unwrap(), original);
    assert!(repo.path().join(".git/doxguard/hooks/pre-commit").is_file());
    assert!(!repo.path().join(".githooks/pre-commit").exists());
    let hooks_path = Command::new("git")
        .args(["config", "--get", "core.hooksPath"])
        .current_dir(repo.path())
        .output()
        .unwrap();
    assert_eq!(
        String::from_utf8(hooks_path.stdout).unwrap().trim(),
        ".husky"
    );
}
