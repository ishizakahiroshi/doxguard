use doxguard::review;
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::{TempDir, tempdir};

fn git(cwd: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .current_dir(cwd)
        .args(args)
        .env_remove("GIT_INDEX_FILE")
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "git failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    out
}
fn repo() -> TempDir {
    let tmp = tempdir().unwrap();
    git(tmp.path(), &["init", "-q"]);
    git(tmp.path(), &["config", "user.name", "Fixture Author"]);
    git(tmp.path(), &["config", "user.email", "fixture@example.com"]);
    // Explicit hermetic config keeps author environment and watchlists out of all fixtures.
    fs::write(tmp.path().join("review-config.json"), "{}").unwrap();
    tmp
}
fn collect(root: &Path) -> review::Review {
    review::collect(root, Some(&root.join("review-config.json")), true).unwrap()
}
fn ignore(root: &Path) {
    fs::write(root.join(".gitignore"), "/.doxguard/reports/\n").unwrap();
}

#[test]
fn staged_snapshot_and_context_never_use_unstaged_text() {
    let tmp = repo();
    let root = tmp.path();
    fs::write(
        root.join("space 日本語.txt"),
        "old=192.168.50.9\r\nkeep\r\n",
    )
    .unwrap();
    git(root, &["add", "."]);
    git(root, &["commit", "-qm", "fixture"]);
    fs::write(
        root.join("space 日本語.txt"),
        "old=192.168.50.9\nkeep\nnew=192.168.50.10\n",
    )
    .unwrap();
    git(root, &["add", "."]);
    fs::write(
        root.join("space 日本語.txt"),
        "UNSTAGED_ONLY=192.168.50.11\n",
    )
    .unwrap();
    let result = collect(root);
    assert_eq!(result.findings.len(), 2);
    assert_eq!(result.findings[0].change, "existing");
    assert_eq!(result.findings[1].change, "added");
    assert_eq!(result.findings[1].hit.line_number, 3);
    assert!(!result.findings[1].context.contains("UNSTAGED_ONLY"));
    assert_eq!(result.snapshot, collect(root).snapshot);
    fs::write(root.join("space 日本語.txt"), "changed=192.168.50.12\n").unwrap();
    git(root, &["add", "."]);
    assert_ne!(result.snapshot, collect(root).snapshot);
}

#[test]
fn initial_commit_rename_and_deletion_are_classified() {
    let tmp = repo();
    let root = tmp.path();
    fs::write(
        root.join("old file.txt"),
        "a=192.168.50.9\nb=192.168.50.10\n",
    )
    .unwrap();
    git(root, &["add", "."]);
    assert!(collect(root).findings.iter().all(|f| f.change == "added"));
    git(root, &["commit", "-qm", "fixture"]);
    git(root, &["mv", "old file.txt", "new file.txt"]);
    assert!(
        collect(root)
            .findings
            .iter()
            .all(|f| f.change == "existing")
    );
    git(root, &["commit", "-qm", "rename fixture"]);
    fs::write(root.join("new file.txt"), "a=192.168.50.9\n").unwrap();
    git(root, &["add", "."]);
    let result = collect(root);
    assert_eq!(result.findings.len(), 1);
    assert_eq!(result.findings[0].change, "existing");
}

#[test]
fn output_requires_ignore_rejects_tracked_and_never_overwrites() {
    let tmp = repo();
    let root = tmp.path();
    assert!(review::destination(root).is_err());
    assert!(!root.join(".doxguard").exists());
    fs::write(root.join(".gitignore"), "/.doxguard/reports/review.html\n").unwrap();
    assert!(review::destination(root).is_err());
    ignore(root);
    let dir = review::destination(root).unwrap();
    fs::write(root.join("fixture.txt"), "value=192.168.50.9\n").unwrap();
    git(root, &["add", "."]);
    let report = collect(root);
    let path = review::write(&dir, &report).unwrap();
    let before = fs::read(&path).unwrap();
    assert!(review::write(&dir, &report).is_err());
    assert_eq!(before, fs::read(&path).unwrap());
    let status = git(root, &["status", "--porcelain"]);
    assert!(!String::from_utf8_lossy(&status.stdout).contains(".doxguard"));
    git(root, &["add", "-f", ".doxguard/reports"]);
    assert!(review::destination(root).is_err());
}

#[test]
fn html_escapes_hostile_context_and_contains_no_external_assets() {
    let tmp = repo();
    let root = tmp.path();
    fs::write(
        root.join("fixture.txt"),
        "192.168.50.9 </script><img src=https://example.invalid/evil onerror=alert(1)>\n",
    )
    .unwrap();
    git(root, &["add", "."]);
    let html = review::render(&collect(root)).unwrap();
    assert!(!html.contains("</script><img"));
    assert!(html.contains("\\u003c/script\\u003e"));
    assert!(!html.contains("<script src="));
    assert!(html.contains("connect-src 'none'"));
    assert!(html.contains("textContent=text"));
}

#[test]
fn cli_reports_counts_without_values_and_requires_staged() {
    let tmp = repo();
    let root = tmp.path();
    ignore(root);
    fs::write(root.join("fixture.txt"), "value=192.168.50.9\n").unwrap();
    git(root, &["add", "."]);
    let out = Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .current_dir(root)
        .env_remove("DOXGUARD_CONFIG")
        .args(["review", "--staged", "--config", "review-config.json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    for bytes in [&out.stdout, &out.stderr] {
        assert!(!String::from_utf8_lossy(bytes).contains("192.168.50.9"));
    }
    assert!(String::from_utf8_lossy(&out.stdout).contains("Review saved:"));
    let out = Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .current_dir(root)
        .arg("review")
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(2));
}

#[cfg(unix)]
#[test]
fn output_refuses_symlinks() {
    let tmp = repo();
    let outside = tempdir().unwrap();
    ignore(tmp.path());
    std::os::unix::fs::symlink(outside.path(), tmp.path().join(".doxguard")).unwrap();
    assert!(review::destination(tmp.path()).is_err());
    assert!(!outside.path().join("reports").exists());
}

#[test]
fn alternate_index_is_preserved_and_used_by_cli() {
    let tmp = repo();
    let root = tmp.path();
    ignore(root);
    fs::write(root.join("fixture.txt"), "192.168.50.9\n").unwrap();
    let index = root.join("alternate-index");
    let out = Command::new("git")
        .current_dir(root)
        .env("GIT_INDEX_FILE", &index)
        .args(["add", "fixture.txt"])
        .output()
        .unwrap();
    assert!(out.status.success());
    let before = fs::read(&index).unwrap();
    let out = Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .current_dir(root)
        .env_remove("DOXGUARD_CONFIG")
        .env("GIT_INDEX_FILE", &index)
        .args(["review", "--staged", "--config", "review-config.json"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stdout).contains("1 findings"));
    assert_eq!(before, fs::read(index).unwrap());
}

#[test]
fn no_findings_and_unscannable_blob_remain_distinct() {
    let tmp = repo();
    let root = tmp.path();
    fs::write(root.join("clean.txt"), "synthetic clean content\n").unwrap();
    git(root, &["add", "."]);
    let clean = collect(root);
    assert!(clean.findings.is_empty());
    assert_eq!(clean.coverage_skips, 0);
    fs::write(root.join("unreadable.txt"), [0xff, 0xfe, 0x80]).unwrap();
    git(root, &["add", "."]);
    let incomplete = collect(root);
    assert!(incomplete.findings.is_empty());
    assert_eq!(incomplete.coverage_skips, 1);
    assert!(!incomplete.warnings.is_empty());
}

#[cfg(windows)]
#[test]
fn output_refuses_windows_junctions() {
    let tmp = repo();
    let outside = tempdir().unwrap();
    ignore(tmp.path());
    let output = Command::new("cmd.exe")
        .args(["/c", "mklink", "/J"])
        .arg(tmp.path().join(".doxguard"))
        .arg(outside.path())
        .output()
        .unwrap();
    assert!(output.status.success(), "could not create junction fixture");
    assert!(review::destination(tmp.path()).is_err());
    assert!(!outside.path().join("reports").exists());
}
