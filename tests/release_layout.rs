use std::{fs, path::Path};

fn assert_actions_are_commit_pinned(workflow: &str, path: &Path) -> usize {
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
            .unwrap_or_else(|| panic!("{}: action has no revision: {spec}", path.display()))
            .1;
        assert!(
            revision.len() == 40 && revision.bytes().all(|byte| byte.is_ascii_hexdigit()),
            "{}: action must use a full commit SHA: {spec}",
            path.display()
        );
    }
    count
}

#[test]
fn npm_platform_packages_match_root_optional_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root_package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
    let version = root_package["version"].as_str().unwrap();
    let optional = root_package["optionalDependencies"].as_object().unwrap();

    for (slug, expected_name, expected_os, expected_cpu) in [
        (
            "win32-x64",
            "@ishizakahiroshi/doxguard-win32-x64",
            "win32",
            "x64",
        ),
        (
            "win32-arm64",
            "@ishizakahiroshi/doxguard-win32-arm64",
            "win32",
            "arm64",
        ),
        (
            "linux-x64",
            "@ishizakahiroshi/doxguard-linux-x64",
            "linux",
            "x64",
        ),
        (
            "linux-arm64",
            "@ishizakahiroshi/doxguard-linux-arm64",
            "linux",
            "arm64",
        ),
        (
            "darwin-x64",
            "@ishizakahiroshi/doxguard-darwin-x64",
            "darwin",
            "x64",
        ),
        (
            "darwin-arm64",
            "@ishizakahiroshi/doxguard-darwin-arm64",
            "darwin",
            "arm64",
        ),
    ] {
        let package: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("npm/platforms").join(slug).join("package.json"))
                .unwrap(),
        )
        .unwrap();
        let name = package["name"].as_str().unwrap();
        assert_eq!(name, expected_name, "{slug}");
        assert_eq!(package["version"], version);
        assert_eq!(
            optional.get(name).and_then(|value| value.as_str()),
            Some(version)
        );
        assert_eq!(package["os"], serde_json::json!([expected_os]), "{slug}");
        assert_eq!(package["cpu"], serde_json::json!([expected_cpu]), "{slug}");
        let expected_bin = if slug.starts_with("win32") {
            "bin/doxguard.exe"
        } else {
            "bin/doxguard"
        };
        assert!(
            package.get("bin").is_none(),
            "{slug} must not publish a competing doxguard bin"
        );
        assert_eq!(
            package["files"],
            serde_json::json!([expected_bin, "README.md", "LICENSE"]),
            "{slug} files must be strictly allowlisted"
        );
    }
}

#[test]
fn npm_launcher_map_matches_optional_dependencies_and_manifests() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let launcher = fs::read_to_string(root.join("bin/doxguard.js")).unwrap();
    let root_package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
    let optional = root_package["optionalDependencies"].as_object().unwrap();
    assert_eq!(optional.len(), 6);
    for name in optional.keys() {
        let slug = name.rsplit("doxguard-").next().unwrap();
        let package: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("npm/platforms").join(slug).join("package.json"))
                .unwrap(),
        )
        .unwrap();
        let binary_path = package["files"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(serde_json::Value::as_str)
            .find(|path| path.starts_with("bin/"))
            .unwrap();
        let binary = Path::new(binary_path)
            .file_name()
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        let entry = format!("\"{slug}\": [\"{name}\", \"{binary}\"]");
        assert!(
            launcher.contains(&entry),
            "bin/doxguard.js is missing launcher entry {entry}"
        );
    }
}

#[test]
fn npm_root_package_is_strictly_allowlisted() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
    assert_eq!(package["bin"]["doxguard"], "bin/doxguard.js");
    assert_eq!(
        package["files"],
        serde_json::json!(["bin/doxguard.js", "README.md", "LICENSE"])
    );
}

#[test]
fn tracked_github_actions_are_commit_pinned() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let workflows = root.join(".github/workflows");
    let mut action_count = 0;
    for entry in fs::read_dir(&workflows).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().and_then(|value| value.to_str()) != Some("yml") {
            continue;
        }
        let workflow = fs::read_to_string(&path).unwrap();
        action_count += assert_actions_are_commit_pinned(&workflow, &path);
    }
    assert!(
        action_count > 0,
        "expected at least one remote action reference"
    );
}

#[test]
fn manual_full_release_binds_the_input_tag_to_the_dispatch_commit() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    assert!(release.contains("fetch-depth: 0"));
    assert!(
        release.contains(r#"test "$(git rev-list -n 1 "$RELEASE_TAG")" = "$GITHUB_SHA""#),
        "manual full release must refuse a tag that does not resolve to the dispatch commit"
    );
}

#[test]
fn release_requires_same_commit_validation_and_serializes_each_tag() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    assert!(release.contains("group: release-${{"));
    assert!(release.contains("cancel-in-progress: false"));
    assert!(
        release.contains("git merge-base --is-ancestor \"$GITHUB_SHA\" refs/remotes/origin/main")
    );
    // Preflight binds the required Validate run to this exact release commit on main.
    // `--event push` was removed as the F-C06 / T06 workaround (push-triggered runs are
    // not delivered in this repo; a same-commit dispatch Validate is accepted instead).
    assert!(release.contains("--branch main"));
    assert!(release.contains("--commit \"$GITHUB_SHA\""));
    assert!(
        !release.contains("--event push"),
        "F-C06 workaround: preflight must not require a push-event Validate run"
    );
}

#[test]
fn release_pipeline_hardening_is_present() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    // F-C07: fmt/clippy run in the preflight (dispatch path too).
    assert!(release.contains("cargo fmt --all -- --check"));
    assert!(release.contains("cargo clippy --all-targets --locked -- -D warnings"));
    // F-C08: build provenance attestation for the native binaries.
    assert!(release.contains("actions/attest-build-provenance@"));
    assert!(release.contains("subject-path: staging/"));
    // F-C10: pre-release tags are marked as pre-release on GitHub.
    assert!(release.contains("--prerelease"));
    // F-B12: the npm packaged gate runs in strict mode.
    assert!(release.contains("scan --packaged --block --strict"));
    // F-C09 / T09b: trusted publishing (OIDC) — no long-lived npm token anywhere,
    // and Node 22 for the npm >= 11.5.1 requirement.
    assert!(
        !release.contains("NPM_TOKEN"),
        "trusted publishing must not reference a long-lived NPM_TOKEN"
    );
    assert!(!release.contains("NODE_AUTH_TOKEN"));
    assert!(release.contains("node-version: \"22\""));
}

#[test]
fn npm_publish_retry_is_transient_only_and_checks_lost_success() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let release = fs::read_to_string(root.join(".github/workflows/release.yml")).unwrap();
    assert!(release.contains("npm view \"$name@$version\" version"));
    for transient in ["429", "EAI_AGAIN", "ECONNRESET", "ETIMEDOUT", "E50[0234]"] {
        assert!(
            release.contains(transient),
            "missing transient token {transient}"
        );
    }
    assert!(!release.contains("E401|E403"));
}
