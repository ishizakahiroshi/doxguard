//! `.icns` is a binary extension, and `@2x` image names are not email addresses.
//! Every repository, file and value here is synthetic.
use std::{fs, path::Path, process::Command};
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

fn git(cwd: &Path, args: &[&str]) {
    let output = command("git", cwd).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "git: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn repo(cwd: &Path) {
    git(cwd, &["init", "-q"]);
    git(cwd, &["config", "user.name", "Synthetic Fixture"]);
    git(cwd, &["config", "user.email", "fixture@example.com"]);
    git(cwd, &["config", "core.autocrlf", "false"]);
    fs::write(
        cwd.join("doxguard.config.json"),
        r#"{"allow":{"emailDomains":["allowed.test"]},"structural":{"ipv4":true}}"#,
    )
    .unwrap();
}

fn scan_all(cwd: &Path) -> serde_json::Value {
    let output = command(env!("CARGO_BIN_EXE_doxguard"), cwd)
        .args([
            "scan",
            "--all-tracked",
            "--block",
            "--strict",
            "--format",
            "json",
        ])
        .output()
        .unwrap();
    serde_json::from_slice(&output.stdout).unwrap_or_else(|_| {
        panic!(
            "stdout={} stderr={}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )
    })
}

/// Line numbers (1-based) of email hits in `file`.
fn email_lines(report: &serde_json::Value, file: &str) -> Vec<u64> {
    report["hits"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|hit| hit["file"] == file && hit["source"].as_str().unwrap().contains("Email"))
        .map(|hit| hit["line_number"].as_u64().unwrap())
        .collect()
}

#[test]
fn density_image_names_are_not_emails() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    let names = [
        "icons/128x128@2x.png",
        "AppIcon-20x20@2x-1.png",
        "AppIcon-83.5x83.5@2x.png",
        "logo@3x.PNG",
        "photo@2x.jpg",
        "mark@2x.svg",
        "see icons/icon@2x.png for details",
    ];
    fs::write(cwd.join("names.txt"), names.join("\n") + "\n").unwrap();
    git(cwd, &["add", "--", "."]);

    let report = scan_all(cwd);

    assert_eq!(
        email_lines(&report, "names.txt"),
        Vec::<u64>::new(),
        "{report}"
    );
}

#[test]
fn real_emails_next_to_the_image_name_shape_are_still_detected() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    let lines = [
        "user@example.com",
        "someone@2x.com",
        "a@3x.png.example.com",
        "name@2xcorp.org",
        "x@1.2.3.4",
        "a@2x.png.c",
        "b@2x.pngs",
        "c@2x-a.png",
        "d@x.png",
        "e@2x.txt",
    ];
    fs::write(cwd.join("mail.txt"), lines.join("\n") + "\n").unwrap();
    git(cwd, &["add", "--", "."]);

    let report = scan_all(cwd);
    let found = email_lines(&report, "mail.txt");

    // Line 5 (x@1.2.3.4) is an IP, not an email; every other line must hit.
    // Line 6 has a dotted tail after the image name, so it is not exempt.
    for line in [1, 2, 3, 4, 6, 7, 8, 9, 10] {
        assert!(
            found.contains(&line),
            "line {line} missing: {found:?}\n{report}"
        );
    }
    let ip_hit = report["hits"]
        .as_array()
        .unwrap()
        .iter()
        .any(|hit| hit["file"] == "mail.txt" && hit["line_number"] == 5);
    assert!(ip_hit, "x@1.2.3.4 must still be reported: {report}");
}

#[test]
fn icns_is_a_quiet_binary_skip() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    let mut bytes = b"icns".to_vec();
    bytes.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0xff, 0xfe, 0x80, 0x81]);
    fs::write(cwd.join("app.icns"), &bytes).unwrap();
    fs::write(cwd.join("app.ICNS"), &bytes).unwrap();
    git(cwd, &["add", "--", "."]);

    let report = scan_all(cwd);

    assert_eq!(report["coverage_skips"], 0, "{report}");
}

#[test]
fn other_non_utf8_extensions_remain_coverage_skips() {
    let temp = tempdir().unwrap();
    let cwd = temp.path();
    repo(cwd);
    let mut bytes = b"icns".to_vec();
    bytes.extend_from_slice(&[0x00, 0x00, 0x01, 0x00, 0xff, 0xfe, 0x80, 0x81]);
    fs::write(cwd.join("data.dat"), &bytes).unwrap();
    fs::write(cwd.join("fixture.txt"), &bytes).unwrap();
    git(cwd, &["add", "--", "."]);

    let report = scan_all(cwd);

    assert_eq!(report["coverage_skips"], 2, "{report}");
}
