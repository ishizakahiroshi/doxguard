use std::{fs, path::Path};

#[test]
fn npm_platform_packages_match_root_optional_dependencies() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let root_package: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(root.join("package.json")).unwrap()).unwrap();
    let version = root_package["version"].as_str().unwrap();
    let optional = root_package["optionalDependencies"].as_object().unwrap();

    for slug in [
        "win32-x64",
        "win32-arm64",
        "linux-x64",
        "linux-arm64",
        "darwin-x64",
        "darwin-arm64",
    ] {
        let package: serde_json::Value = serde_json::from_str(
            &fs::read_to_string(root.join("npm/platforms").join(slug).join("package.json"))
                .unwrap(),
        )
        .unwrap();
        let name = package["name"].as_str().unwrap();
        assert_eq!(package["version"], version);
        assert_eq!(
            optional.get(name).and_then(|value| value.as_str()),
            Some(version)
        );
        assert!(
            package["os"]
                .as_array()
                .is_some_and(|values| values.len() == 1)
        );
        assert!(
            package["cpu"]
                .as_array()
                .is_some_and(|values| values.len() == 1)
        );
        let expected_bin = if slug.starts_with("win32") {
            "bin/doxguard.exe"
        } else {
            "bin/doxguard"
        };
        assert_eq!(package["bin"]["doxguard"], expected_bin, "{slug}");
        assert!(
            package["files"]
                .as_array()
                .is_some_and(|files| files.iter().any(|file| file == expected_bin)),
            "{slug} files must ship {expected_bin}"
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
        let binary = Path::new(package["bin"]["doxguard"].as_str().unwrap())
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

fn release_workflow() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join(".github/workflows/release.yml"))
        .unwrap()
        .replace("\r\n", "\n")
}

fn npm_publish_job_from(workflow: &str) -> String {
    workflow
        .replace("\r\n", "\n")
        .split_once("  npm-publish:\n")
        .expect("npm publish job must exist")
        .1
        .to_owned()
}

fn npm_publish_job() -> String {
    npm_publish_job_from(&release_workflow())
}

fn npm_oidc_preflight_script(job: &str) -> String {
    job.replace("\r\n", "\n")
        .split_once("          node <<'NODE'\n")
        .unwrap()
        .1
        .split_once("          NODE\n")
        .unwrap()
        .0
        .to_owned()
}

#[test]
fn release_job_and_oidc_heredoc_extraction_accept_lf_and_crlf() {
    let lf = release_workflow();
    let crlf = lf.replace('\n', "\r\n");
    assert!(crlf.contains("\r\n"));
    let lf_job = npm_publish_job_from(&lf);
    let crlf_job = npm_publish_job_from(&crlf);
    assert_eq!(lf_job, crlf_job);
    assert_eq!(
        npm_oidc_preflight_script(&lf_job),
        npm_oidc_preflight_script(&crlf_job.replace('\n', "\r\n"))
    );
}

#[test]
fn npm_publish_uses_oidc_without_token_gates_or_auth_config() {
    let workflow = release_workflow();
    let job = npm_publish_job();
    for forbidden in ["NPM_TOKEN", "NODE_AUTH_TOKEN", "npm-token", "registry-url:"] {
        assert!(!workflow.contains(forbidden), "unexpected {forbidden}");
    }
    for required in [
        "id-token: write",
        "node-version: \"24\"",
        "export NPM_CONFIG_USERCONFIG=\"$RUNNER_TEMP/doxguard-npm-oidc.npmrc\"",
        "export NPM_CONFIG_GLOBALCONFIG=\"$RUNNER_TEMP/doxguard-npm-global.npmrc\"",
        "printf 'NPM_CONFIG_USERCONFIG=%s\\n' \"$NPM_CONFIG_USERCONFIG\" >> \"$GITHUB_ENV\"",
        "printf 'NPM_CONFIG_GLOBALCONFIG=%s\\n' \"$NPM_CONFIG_GLOBALCONFIG\" >> \"$GITHUB_ENV\"",
        "printf 'registry=https://registry.npmjs.org/\\n' > \"$NPM_CONFIG_USERCONFIG\"",
        ": > \"$NPM_CONFIG_GLOBALCONFIG\"",
        "atLeast(nodeVersion, [22, 14, 0])",
        "atLeast(npmVersion, [11, 5, 1])",
        "process.env.ACTIONS_ID_TOKEN_REQUEST_URL",
        "process.env.ACTIONS_ID_TOKEN_REQUEST_TOKEN",
        "npm publish \"$dir\" --access public --provenance --tag \"$dist_tag\"",
    ] {
        assert!(
            job.contains(required),
            "missing OIDC requirement {required}"
        );
    }
    assert!(!job.contains("npm install"));
    assert!(!job.contains("channel skipped"));
}

fn job_env_does_not_use_runner(job: &str) -> bool {
    // runner is unavailable in jobs.<job_id>.env according to GitHub's context table.
    let header = job.split_once("    steps:\n").unwrap().0;
    let Some((_, env)) = header.split_once("    env:\n") else {
        return true;
    };
    let env = env
        .lines()
        .take_while(|line| line.starts_with("      ") || line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    let expressions = regex::Regex::new(r"\$\{\{(.*?)\}\}").unwrap();
    let runner = regex::Regex::new(r"\brunner\s*(?:\.|\[)").unwrap();
    !expressions
        .captures_iter(&env)
        .any(|expression| runner.is_match(&expression[1]))
}

#[test]
fn npm_job_env_rejects_runner_context_before_a_runner_exists() {
    let job = npm_publish_job();
    assert!(job_env_does_not_use_runner(&job));
    for expression in ["runner.temp", "runner['temp']"] {
        let broken = job.replace(
            "    steps:\n",
            &format!("    env:\n      CONFIG: ${{{{ {expression} }}}}\n    steps:\n"),
        );
        assert!(!job_env_does_not_use_runner(&broken));
    }
    let supported = job.replace(
        "    steps:\n",
        "    env:\n      SOURCE: ${{ github.sha }}\n    steps:\n",
    );
    assert!(job_env_does_not_use_runner(&supported));
}

#[test]
fn npm_config_exports_reach_the_current_step_and_github_env_reaches_the_next_step() {
    let job = npm_publish_job();
    let preflight = job
        .split_once("      - name: Require supported npm OIDC tooling and isolated configuration\n")
        .unwrap()
        .1
        .split_once("          node <<'NODE'\n")
        .unwrap()
        .0
        .split_once("        run: |\n")
        .unwrap()
        .1
        .lines()
        .map(|line| line.strip_prefix("          ").unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    let temporary = tempfile::tempdir().unwrap();
    let runner_temp = temporary.path().join("runner temp");
    fs::create_dir(&runner_temp).unwrap();
    let github_env = temporary.path().join("github-env");
    let bash = if cfg!(windows) {
        "C:\\Program Files\\Git\\bin\\bash.exe"
    } else {
        "bash"
    };
    let checks = r#"
test "$NPM_CONFIG_USERCONFIG" = "$RUNNER_TEMP/doxguard-npm-oidc.npmrc"
test "$NPM_CONFIG_GLOBALCONFIG" = "$RUNNER_TEMP/doxguard-npm-global.npmrc"
test "$(cat "$NPM_CONFIG_USERCONFIG")" = 'registry=https://registry.npmjs.org/'
test ! -s "$NPM_CONFIG_GLOBALCONFIG"
"#;
    let first = std::process::Command::new(bash)
        .args(["--noprofile", "--norc", "-c"])
        .arg(format!("{preflight}\n{checks}"))
        .env("RUNNER_TEMP", &runner_temp)
        .env("GITHUB_ENV", &github_env)
        .env_remove("NPM_CONFIG_USERCONFIG")
        .env_remove("NPM_CONFIG_GLOBALCONFIG")
        .output()
        .expect("Bash is required to verify release environment propagation");
    assert!(first.status.success(), "current-step config export failed");
    let environment = fs::read_to_string(github_env).unwrap();
    let entries = environment
        .lines()
        .map(|line| line.split_once('=').unwrap())
        .collect::<Vec<_>>();
    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].0, "NPM_CONFIG_USERCONFIG");
    assert_eq!(entries[1].0, "NPM_CONFIG_GLOBALCONFIG");
    let second = std::process::Command::new(bash)
        .args(["--noprofile", "--norc", "-c", checks])
        .env("RUNNER_TEMP", runner_temp)
        .env_remove("NPM_CONFIG_USERCONFIG")
        .env_remove("NPM_CONFIG_GLOBALCONFIG")
        .envs(entries)
        .output()
        .unwrap();
    assert!(
        second.status.success(),
        "next-step config propagation failed"
    );
}

#[test]
fn npm_oidc_tooling_minimum_versions_reject_older_or_malformed_versions() {
    let job = npm_publish_job();
    let function = job
        .split_once("          function atLeast(version, minimum) {")
        .unwrap()
        .1
        .split_once("          const nodeVersion")
        .unwrap()
        .0;
    let script = format!(
        r#"function atLeast(version, minimum) {{{function}
const cases = [
  ['22.14.0', [22, 14, 0], true], ['24.0.0', [22, 14, 0], true],
  ['22.13.9', [22, 14, 0], false], ['20.99.99', [22, 14, 0], false],
  ['11.5.1', [11, 5, 1], true], ['11.12.0', [11, 5, 1], true],
  ['11.5.0', [11, 5, 1], false], ['10.99.99', [11, 5, 1], false],
  ['11.5.1-rc.1', [11, 5, 1], false], ['not-a-version', [11, 5, 1], false]
];
for (const [version, minimum, expected] of cases) {{
  if (atLeast(version, minimum) !== expected) throw new Error('version gate mismatch');
}}
"#
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(script)
        .output()
        .expect("Node is required to verify release tooling");
    assert!(output.status.success(), "version gate failed");
}

#[test]
fn npm_oidc_preflight_rejects_missing_permissions_and_unsupported_tools() {
    let job = npm_publish_job();
    let script = npm_oidc_preflight_script(&job);
    let encoded = serde_json::to_string(&script).unwrap();
    let harness = format!(
        r#"const vm = require('node:vm');
const script = {encoded};
const cases = [
  ['24.0.0', '11.5.1', true, true, true],
  ['22.14.0', '11.5.1', true, true, true],
  ['22.13.9', '11.5.1', true, true, false],
  ['24.0.0', '11.5.0', true, true, false],
  ['24.0.0', '11.5.1', false, true, false],
  ['24.0.0', '11.5.1', true, false, false]
];
for (const [node, npm, url, token, expected] of cases) {{
  let passed = true;
  try {{
    vm.runInNewContext(script, {{
      require: () => ({{ execFileSync: () => npm }}),
      process: {{ versions: {{ node }}, env: {{
        ACTIONS_ID_TOKEN_REQUEST_URL: url ? 'synthetic-oidc-endpoint' : '',
        ACTIONS_ID_TOKEN_REQUEST_TOKEN: token ? 'synthetic-oidc-token' : ''
      }} }}, console: {{ log: () => {{}} }}
    }});
  }} catch {{ passed = false; }}
  if (passed !== expected) throw new Error('OIDC preflight mismatch');
}}
"#
    );
    let output = std::process::Command::new("node")
        .arg("-e")
        .arg(harness)
        .output()
        .expect("Node is required to verify OIDC preflight");
    assert!(output.status.success(), "OIDC preflight controls failed");
}

#[test]
fn npm_recovery_requires_canonical_checksums_and_exact_resolved_source() {
    let workflow = release_workflow();
    let job = npm_publish_job();
    assert!(workflow.contains("Require a green Validate run for the resolved source"));
    assert!(workflow.contains("run: bash .github/scripts/resolve-release-source.sh"));
    assert!(job.contains("ref: ${{ needs.resolve-source.outputs.commit }}"));
    assert!(job.contains("SOURCE_SHA: ${{ needs.resolve-source.outputs.commit }}"));
    for required in [
        "inputs.npm_only",
        "gh release download \"$RELEASE_TAG\" --dir canonical",
        "(cd canonical && sha256sum --check SHA256SUMS.txt)",
        "test -f canonical/SOURCE_COMMIT.txt",
        "test \"$(cat canonical/SOURCE_COMMIT.txt)\" = \"$SOURCE_SHA\"",
        "test \"$version\" = \"$package_version\"",
    ] {
        assert!(job.contains(required), "missing recovery check {required}");
    }
    assert!(!job.contains("Legacy canonical release"));
    assert!(!job.contains("cargo build"));
    assert!(!job.contains("gh release upload"));
    assert!(!job.contains("gh release edit"));
    assert!(!job.contains("git tag"));
}

#[test]
fn npm_recovery_preserves_platform_order_retry_and_existing_version_skip() {
    let job = npm_publish_job();
    for required in [
        "if npm view \"$name@$version\" version >/dev/null 2>&1; then",
        "for attempt in 1 2 3 4 5; do",
        "delay=$((attempt * attempt * 15))",
        "grep -Eqi '429|rate.?limit' publish.log",
        "for slug in win32-x64 win32-arm64 linux-x64 linux-arm64 darwin-x64 darwin-arm64; do\n            publish_once \"npm/platforms/$slug\"\n          done\n          publish_once \".\"",
    ] {
        assert!(
            job.contains(required),
            "missing publish behavior {required}"
        );
    }
}
