use serde_json::{Value, json};
use std::{
    fs,
    path::Path,
    process::{Command, Output},
};
use tempfile::tempdir;

fn git(cwd: &Path, args: &[&str]) -> String {
    let output = Command::new("git")
        .args(args)
        .current_dir(cwd)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "fixture git failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8(output.stdout).unwrap().trim().to_owned()
}
fn init(cwd: &Path) {
    git(cwd, &["init", "-q"]);
    git(cwd, &["config", "user.name", "Fixture Author"]);
    git(cwd, &["config", "user.email", "fixture@example.com"]);
    fs::write(cwd.join("doxguard.config.json"), "{}").unwrap();
}
fn commit(cwd: &Path) {
    git(
        cwd,
        &["-c", "core.hooksPath=/dev/null", "commit", "-qm", "fixture"],
    );
}
fn scan(cwd: &Path, mode: &str) -> Output {
    Command::new(env!("CARGO_BIN_EXE_doxguard"))
        .args(["scan", mode, "--block", "--format", "json"])
        .current_dir(cwd)
        .env_remove("DOXGUARD_CONFIG")
        .env_remove("DOXGUARD_SHOW_PATHS")
        .env_remove("DOXGUARD_WATCHLIST_DIR")
        .env_remove("GIT_GRAFT_FILE")
        .output()
        .unwrap()
}
fn report(cwd: &Path) -> (i32, Value) {
    let output = scan(cwd, "--history");
    (
        output.status.code().unwrap(),
        serde_json::from_slice(&output.stdout).unwrap(),
    )
}

#[test]
fn deleted_blobs_refs_and_unreachable_objects_preserve_history_contract() {
    let temp = tempdir().unwrap();
    init(temp.path());
    fs::write(
        temp.path().join("日本語 file.txt"),
        "fixture_private@sample.test\n",
    )
    .unwrap();
    git(temp.path(), &["add", "日本語 file.txt"]);
    commit(temp.path());
    let oid = git(temp.path(), &["rev-parse", "HEAD:日本語 file.txt"]);
    git(temp.path(), &["tag", "fixture-old"]);
    git(temp.path(), &["rm", "日本語 file.txt"]);
    commit(temp.path());
    fs::write(
        temp.path().join("dangling.txt"),
        "fixture_dangling@sample.test\n",
    )
    .unwrap();
    git(temp.path(), &["hash-object", "-w", "dangling.txt"]);
    let (exit, result) = report(temp.path());
    assert_eq!(exit, 1);
    assert_eq!(result["mode"], "history");
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    assert_eq!(result["hits"][0]["file"], "日本語 file.txt");
    assert_eq!(result["hits"][0]["blob_oid"], oid);
    assert_eq!(result["hits"][0]["matched"], "[REDACTED]");
    assert_eq!(result["baseline_applied"], false);
    let tracked = scan(temp.path(), "--all-tracked");
    assert_eq!(tracked.status.code(), Some(0));
}

#[test]
fn empty_history_direct_blob_and_tree_refs_are_handled() {
    let temp = tempdir().unwrap();
    init(temp.path());
    assert_eq!(report(temp.path()).0, 0);
    fs::write(temp.path().join("file.txt"), "fixture_direct@sample.test\n").unwrap();
    let oid = git(temp.path(), &["hash-object", "-w", "file.txt"]);
    git(temp.path(), &["update-ref", "refs/tags/fixture-blob", &oid]);
    assert_eq!(report(temp.path()).0, 1);
    git(temp.path(), &["add", "file.txt"]);
    let tree = git(temp.path(), &["write-tree"]);
    git(
        temp.path(),
        &["update-ref", "refs/tags/fixture-tree", &tree],
    );
    let (_, result) = report(temp.path());
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    assert_eq!(result["hits"][0]["file"], "file.txt");
}

#[test]
fn history_paths_keep_exemptions_and_lockfile_structure_without_subtree_aliases() {
    let temp = tempdir().unwrap();
    init(temp.path());
    fs::create_dir(temp.path().join("tests")).unwrap();
    fs::write(
        temp.path().join("tests/fixture.txt"),
        "fixture_exempt@sample.test\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("public.txt"),
        "fixture_exempt@sample.test\n",
    )
    .unwrap();
    fs::write(temp.path().join("Cargo.lock"), "fixture_lock@sample.test\n").unwrap();
    fs::write(
        temp.path().join("doxguard.config.json"),
        json!({"exemptPaths":["tests/"]}).to_string(),
    )
    .unwrap();
    git(
        temp.path(),
        &["add", "tests/fixture.txt", "Cargo.lock", "public.txt"],
    );
    commit(temp.path());
    let (_, result) = report(temp.path());
    assert_eq!(result["hits"].as_array().unwrap().len(), 2);
    assert_eq!(result["hits"][0]["file"], "Cargo.lock");
    assert_eq!(result["hits"][1]["file"], "public.txt");
}

#[test]
fn history_coverage_blocks_oversize_invalid_utf8_links_and_gitlinks() {
    let temp = tempdir().unwrap();
    init(temp.path());
    fs::write(temp.path().join("big.txt"), "x".repeat(40)).unwrap();
    fs::write(temp.path().join("invalid.txt"), [0xff, 0xfe]).unwrap();
    git(temp.path(), &["add", "big.txt", "invalid.txt"]);
    commit(temp.path());
    let target = git(temp.path(), &["rev-parse", "HEAD"]);
    fs::write(temp.path().join("target.txt"), "fixture_target").unwrap();
    let link = git(temp.path(), &["hash-object", "-w", "target.txt"]);
    git(
        temp.path(),
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("120000,{link},link"),
        ],
    );
    git(
        temp.path(),
        &[
            "update-index",
            "--add",
            "--cacheinfo",
            &format!("160000,{target},submodule"),
        ],
    );
    commit(temp.path());
    fs::write(
        temp.path().join("doxguard.config.json"),
        json!({"maxFileSize":20}).to_string(),
    )
    .unwrap();
    let (exit, result) = report(temp.path());
    assert_eq!(exit, 1);
    assert_eq!(result["coverage_skips"], 4);
}

#[test]
fn shallow_partial_and_grafted_history_fail_without_fetching() {
    let source = tempdir().unwrap();
    init(source.path());
    fs::write(source.path().join("file.txt"), "clean\n").unwrap();
    git(source.path(), &["add", "file.txt"]);
    commit(source.path());
    let destination = tempdir().unwrap();
    let clone = destination.path().join("clone");
    git(
        destination.path(),
        &[
            "clone",
            "--depth",
            "1",
            "--no-local",
            source.path().to_str().unwrap(),
            clone.to_str().unwrap(),
        ],
    );
    assert_eq!(scan(&clone, "--history").status.code(), Some(2));
    git(
        source.path(),
        &["config", "remote.fixture.promisor", "true"],
    );
    assert_eq!(scan(source.path(), "--history").status.code(), Some(2));
    git(
        source.path(),
        &["config", "--unset", "remote.fixture.promisor"],
    );
    let head = git(source.path(), &["rev-parse", "HEAD"]);
    fs::write(source.path().join(".git/info/grafts"), head).unwrap();
    assert_eq!(scan(source.path(), "--history").status.code(), Some(2));
}

#[test]
fn raw_blob_scanning_does_not_run_textconv_or_hide_nul_content() {
    let temp = tempdir().unwrap();
    init(temp.path());
    fs::write(temp.path().join(".gitattributes"), "*.txt diff=fixture\n").unwrap();
    git(
        temp.path(),
        &[
            "config",
            "diff.fixture.textconv",
            "fixture_nonexistent_converter",
        ],
    );
    fs::write(temp.path().join("file.txt"), b"\0fixture_nul@sample.test\n").unwrap();
    git(temp.path(), &["add", "file.txt", ".gitattributes"]);
    commit(temp.path());
    let (exit, result) = report(temp.path());
    assert_eq!(exit, 1);
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    assert_eq!(result["coverage_skips"], 0);
}

#[test]
fn replace_refs_cannot_hide_original_reachable_blobs() {
    let temp = tempdir().unwrap();
    init(temp.path());
    fs::write(
        temp.path().join("file.txt"),
        "fixture_original@sample.test\n",
    )
    .unwrap();
    git(temp.path(), &["add", "file.txt"]);
    commit(temp.path());
    fs::write(temp.path().join("file.txt"), "clean\n").unwrap();
    git(temp.path(), &["add", "file.txt"]);
    commit(temp.path());
    let head = git(temp.path(), &["rev-parse", "HEAD"]);
    let tree = git(temp.path(), &["rev-parse", "HEAD^{tree}"]);
    let replacement = git(
        temp.path(),
        &["commit-tree", &tree, "-m", "fixture replacement"],
    );
    git(temp.path(), &["replace", &head, &replacement]);
    let (exit, result) = report(temp.path());
    assert_eq!(exit, 1);
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
}

#[test]
fn nul_tree_entries_preserve_newline_and_tab_in_names() {
    use std::{io::Write, process::Stdio};
    let temp = tempdir().unwrap();
    init(temp.path());
    fs::write(
        temp.path().join("fixture.txt"),
        "fixture_tree@sample.test\n",
    )
    .unwrap();
    let oid = git(temp.path(), &["hash-object", "-w", "fixture.txt"]);
    let path = "line\nwith\ttab.txt";
    let mut child = Command::new("git")
        .args(["mktree", "-z"])
        .current_dir(temp.path())
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(format!("100644 blob {oid}\t{path}\0").as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    assert!(output.status.success());
    let tree = String::from_utf8(output.stdout).unwrap();
    git(
        temp.path(),
        &["update-ref", "refs/tags/fixture-special-tree", tree.trim()],
    );
    let (exit, result) = report(temp.path());
    assert_eq!(exit, 1);
    assert_eq!(result["hits"].as_array().unwrap().len(), 1);
    // CLI escapes terminal controls; JSON parses back to their displayed escape spelling.
    assert_eq!(result["hits"][0]["file"], "line\\nwith\\ttab.txt");
}
