use std::{collections::HashMap, fs};

use doxguard::{
    config::{ColumnSpec, Config, WatchlistSource},
    patterns,
    scan::{HitKind, ScanMode, allowed_by_directive, scan_paths},
    watchlist,
};
use tempfile::tempdir;

#[test]
fn loads_lines_and_csv_watchlists_with_allow_and_noise_rules() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("names.txt"),
        "# synthetic fixture\nNorthwind Harbor\nAllowed Product\n",
    )
    .unwrap();
    fs::write(
        temp.path().join("people.csv"),
        "id,name,given_name\n1,Contoso Works（Fixture）,かな\n2,Fabrikam Studio,みどり\n",
    )
    .unwrap();

    let mut config = Config::default();
    config.allow.names.push("Allowed Product".to_owned());
    config.watchlists = vec![
        WatchlistSource::Lines {
            path: "${FIXTURE_ROOT}/names.txt".to_owned(),
            label: Some("fixture lines".to_owned()),
        },
        WatchlistSource::Csv {
            path: "${FIXTURE_ROOT}/people.csv".to_owned(),
            column: ColumnSpec::Name("name".to_owned()),
            label: Some("fixture names".to_owned()),
            paren_variants: true,
        },
        WatchlistSource::Csv {
            path: "${FIXTURE_ROOT}/people.csv".to_owned(),
            column: ColumnSpec::Name("given_name".to_owned()),
            label: Some("fixture given_name".to_owned()),
            paren_variants: false,
        },
    ];
    let env = HashMap::from([(
        "FIXTURE_ROOT".to_owned(),
        temp.path().to_string_lossy().into_owned(),
    )]);
    let loaded = watchlist::load(&config, temp.path(), &env).unwrap();

    let matches: Vec<_> = loaded
        .matcher
        .matches("Northwind Harbor, Contoso Works, and みどり")
        .map(|item| item.needle.as_str())
        .collect();
    assert_eq!(matches, ["Northwind Harbor", "Contoso Works", "みどり"]);
    assert!(loaded.matcher.matches("Allowed Product").next().is_none());
    assert!(loaded.matcher.matches("かな").next().is_none());
}

#[test]
fn detects_all_structural_kinds_and_honors_three_allow_mechanisms() {
    let temp = tempdir().unwrap();
    fs::write(
        temp.path().join("fixture.txt"),
        concat!(
            "win=X:\\Users\\fixture\\work\n",
            "posix=/home/fixture/work\n",
            "ip=192.168.50.9\n",
            "private=private@sample.test\n",
            "exact=public@sample.test\n",
            "domain=team@docs.public.test\n",
            "name=Allowed Product\n",
        ),
    )
    .unwrap();

    let mut config = Config::default();
    config.allow.names.push("Allowed Product".to_owned());
    config.allow.emails.push("public@sample.test".to_owned());
    config.allow.email_domains.push("public.test".to_owned());
    config.watchlists.push(WatchlistSource::Lines {
        path: "${FIXTURE_ROOT}/names.txt".to_owned(),
        label: Some("fixture".to_owned()),
    });
    fs::write(temp.path().join("names.txt"), "Allowed Product\n").unwrap();
    let env = HashMap::from([(
        "FIXTURE_ROOT".to_owned(),
        temp.path().to_string_lossy().into_owned(),
    )]);
    let matcher = watchlist::load(&config, temp.path(), &env).unwrap().matcher;
    let patterns = patterns::build(&config).unwrap();
    let result = scan_paths(
        ScanMode::AllTracked,
        vec!["fixture.txt".to_owned()],
        temp.path(),
        &config,
        &matcher,
        &patterns,
        Vec::new(),
    )
    .unwrap();

    assert_eq!(result.hits.len(), 4);
    assert!(
        result
            .hits
            .iter()
            .all(|hit| hit.kind == HitKind::Structural)
    );
    assert!(result.hits.iter().any(|hit| hit.source.contains("Windows")));
    assert!(result.hits.iter().any(|hit| hit.source.contains("POSIX")));
    assert!(result.hits.iter().any(|hit| hit.source.contains("RFC1918")));
    assert!(result.hits.iter().any(|hit| hit.source.contains("Email")));
}

#[test]
fn inline_directive_supports_scoped_bare_and_legacy_forms() {
    let config = Config::default();
    assert!(allowed_by_directive(
        "Northwind Harbor // doxguard: allow Northwind",
        "Northwind Harbor",
        &config
    ));
    assert!(allowed_by_directive(
        "192.168.50.9 # doxguard: allow",
        "192.168.50.9",
        &config
    ));
    assert!(allowed_by_directive(
        "Contoso Works <!-- secrets-scan: allow Contoso -->",
        "Contoso Works",
        &config
    ));
    assert!(!allowed_by_directive(
        "Northwind Harbor // doxguard: allow Fabrikam",
        "Northwind Harbor",
        &config
    ));
    // allow* prefixes must not act as bare allow
    assert!(!allowed_by_directive(
        "private@sample.test // doxguard: allowlist note",
        "private@sample.test",
        &config
    ));
    // hyphenated tokens are valid allow scopes
    assert!(allowed_by_directive(
        "Anne-Marie // doxguard: allow Anne-Marie",
        "Anne-Marie",
        &config
    ));
    assert!(allowed_by_directive(
        "Northwind Harbor // doxguard: allow Northwind,",
        "Northwind Harbor",
        &config
    ));
    assert!(allowed_by_directive(
        "Northwind Harbor // doxguard: allow Northwind。",
        "Northwind Harbor",
        &config
    ));
    // the HTML comment closer must not leak into the token
    assert!(allowed_by_directive(
        "Contoso-Labs <!-- doxguard: allow Contoso-Labs -->",
        "Contoso-Labs",
        &config
    ));
    assert!(allowed_by_directive(
        "Contoso-Labs <!-- doxguard: allow Contoso-Labs-->",
        "Contoso-Labs",
        &config
    ));
    // a leading-hyphen token never matches an unrelated hit
    assert!(!allowed_by_directive(
        "some.watched.name # doxguard: allow -legacy",
        "some.watched.name",
        &config
    ));
    // bare allow written inside an HTML comment stays the bare form
    assert!(allowed_by_directive(
        "192.168.50.9 <!-- doxguard: allow -->",
        "192.168.50.9",
        &config
    ));
    // short tokens must not suppress (min length 4)
    assert!(!allowed_by_directive(
        "192.168.50.9 # doxguard: allow 168",
        "192.168.50.9",
        &config
    ));
    assert!(!allowed_by_directive(
        "192.168.50.9 # doxguard: allow .",
        "192.168.50.9",
        &config
    ));
    let mut strict = Config::default();
    strict.apply_strict();
    assert!(
        !allowed_by_directive("192.168.50.9 # doxguard: allow", "192.168.50.9", &strict),
        "strict mode must reject bare allow"
    );
    assert!(allowed_by_directive(
        "192.168.50.9 # doxguard: allow 192.168.50.9",
        "192.168.50.9",
        &strict
    ));
}

#[test]
fn path_exempt_uses_boundaries_not_raw_substring() {
    use doxguard::scan::path_is_exempt;
    assert_eq!(
        Config::default().all_exempt_paths().count(),
        0,
        "consumer repositories must not inherit quiet-skip paths"
    );
    assert!(path_is_exempt("tests/fixture.txt", "tests/"));
    assert!(!path_is_exempt("tests/fixture.txt", "tests"));
    assert!(path_is_exempt("tests", "tests"));
    assert!(!path_is_exempt("mytests/fixture.txt", "tests/"));
    assert!(!path_is_exempt(
        ".github/workflows/doxguard.yml",
        ".github/workflows/doxguard"
    ));
    assert!(path_is_exempt(
        ".github/workflows/doxguard.yml",
        ".github/workflows/"
    ));
    assert!(!path_is_exempt("src/tests/fixture.txt", "tests/"));
    assert!(!path_is_exempt("src/main.rs", ""));
    assert!(!path_is_exempt("any/path", ""));
}

#[test]
fn metadata_errors_are_coverage_skips_but_missing_files_are_quiet() {
    let config = Config::default();
    let matcher = watchlist::WatchlistMatcher::new(Vec::new()).unwrap();
    let patterns = patterns::build(&config).unwrap();
    let temp = tempdir().unwrap();
    let result = scan_paths(
        ScanMode::AllTracked,
        vec!["missing.txt".to_owned(), "invalid\0path.txt".to_owned()],
        temp.path(),
        &config,
        &matcher,
        &patterns,
        Vec::new(),
    )
    .unwrap();

    assert_eq!(result.coverage_skips, 1);
    assert_eq!(result.scanned, 0);
    assert!(
        result
            .warnings
            .iter()
            .any(|warning| warning.contains("unscannable metadata"))
    );
}

#[test]
fn staged_scan_reads_index_not_worktree() {
    use std::process::Command;

    let temp = tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "fixture@example.com"]);
    git(&["config", "user.name", "fixture"]);
    fs::write(temp.path().join("leak.txt"), "192.168.50.9\n").unwrap();
    git(&["add", "leak.txt"]);
    // Clean worktree must not hide the staged blob.
    fs::write(temp.path().join("leak.txt"), "safe\n").unwrap();

    let config = Config::default();
    let matcher = watchlist::WatchlistMatcher::new(Vec::new()).unwrap();
    let patterns = patterns::build(&config).unwrap();
    let result = scan_paths(
        ScanMode::Staged,
        vec!["leak.txt".to_owned()],
        temp.path(),
        &config,
        &matcher,
        &patterns,
        Vec::new(),
    )
    .unwrap();
    assert!(
        result
            .hits
            .iter()
            .any(|hit| hit.matched.contains("192.168")),
        "expected staged private IP hit, got {:?}",
        result.hits
    );
}

#[test]
fn ascii_case_insensitive_watchlist_matches_lower_haystack() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("names.txt"), "Contoso Works\n").unwrap();
    let mut config = Config {
        noise: doxguard::config::NoiseConfig {
            ascii_case_insensitive: true,
            ..Default::default()
        },
        ..Default::default()
    };
    config.watchlists.push(WatchlistSource::Lines {
        path: "${FIXTURE_ROOT}/names.txt".to_owned(),
        label: Some("fixture".to_owned()),
    });
    let env = HashMap::from([(
        "FIXTURE_ROOT".to_owned(),
        temp.path().to_string_lossy().into_owned(),
    )]);
    let matcher = watchlist::load(&config, temp.path(), &env).unwrap().matcher;
    assert!(matcher.matches("contoso works").next().is_some());
}

#[test]
fn missing_env_is_soft_but_resolved_missing_watchlist_fails_closed() {
    let temp = tempdir().unwrap();
    let mut config = Config::default();
    config.watchlists.push(WatchlistSource::Lines {
        path: "${FIXTURE_ROOT}/missing.txt".to_owned(),
        label: Some("fixture".to_owned()),
    });

    let missing_env = watchlist::load(&config, temp.path(), &HashMap::new()).unwrap();
    assert!(missing_env.matcher.is_empty());
    assert!(
        missing_env
            .warnings
            .iter()
            .any(|warning| warning.contains("FIXTURE_ROOT not set"))
    );

    let env = HashMap::from([(
        "FIXTURE_ROOT".to_owned(),
        temp.path().to_string_lossy().into_owned(),
    )]);
    let error = watchlist::load(&config, temp.path(), &env).unwrap_err();
    assert!(!error.to_string().contains("missing.txt"));
    assert!(
        error
            .to_string()
            .contains("resolved successfully but was not found"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn oversized_config_is_refused_before_parse() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("doxguard.config.json");
    fs::write(&path, "x".repeat((1 << 20) + 1)).unwrap();
    let error = doxguard::config::load(temp.path(), Some(&path)).unwrap_err();
    assert!(
        error
            .to_string()
            .contains("refuse to load unbounded config"),
        "unexpected error: {error:#}"
    );
}

#[test]
fn oversize_staged_blob_is_coverage_skipped() {
    use std::process::Command;

    let temp = tempdir().unwrap();
    let git = |args: &[&str]| {
        let output = Command::new("git")
            .args(args)
            .current_dir(temp.path())
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "fixture@example.com"]);
    git(&["config", "user.name", "fixture"]);
    fs::write(
        temp.path().join("big.txt"),
        format!("192.168.50.9\n{}", "x".repeat(64)),
    )
    .unwrap();
    git(&["add", "big.txt"]);

    let config = Config {
        max_file_size: 16,
        ..Default::default()
    };
    let matcher = watchlist::WatchlistMatcher::new(Vec::new()).unwrap();
    let patterns = patterns::build(&config).unwrap();
    let result = scan_paths(
        ScanMode::Staged,
        vec!["big.txt".to_owned()],
        temp.path(),
        &config,
        &matcher,
        &patterns,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(result.coverage_skips, 1);
    assert_eq!(result.scanned, 0);
    assert_eq!(result.exempt_or_skipped, 1);
    assert_eq!(result.total_files, 1);
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("skipped big.txt") && w.contains("oversize staged blob"))
    );
    assert!(result.hits.is_empty());
}

#[test]
fn case_insensitive_hits_report_original_line_casing() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("names.txt"), "Contoso Works\n").unwrap();
    fs::write(temp.path().join("fixture.txt"), "brand=CONTOSO WORKS\n").unwrap();
    let mut config = Config {
        noise: doxguard::config::NoiseConfig {
            ascii_case_insensitive: true,
            ..Default::default()
        },
        ..Default::default()
    };
    config.watchlists.push(WatchlistSource::Lines {
        path: "${FIXTURE_ROOT}/names.txt".to_owned(),
        label: Some("fixture".to_owned()),
    });
    let env = HashMap::from([(
        "FIXTURE_ROOT".to_owned(),
        temp.path().to_string_lossy().into_owned(),
    )]);
    let matcher = watchlist::load(&config, temp.path(), &env).unwrap().matcher;
    let patterns = patterns::build(&config).unwrap();
    let result = scan_paths(
        ScanMode::AllTracked,
        vec!["fixture.txt".to_owned()],
        temp.path(),
        &config,
        &matcher,
        &patterns,
        Vec::new(),
    )
    .unwrap();
    let hit = result
        .hits
        .iter()
        .find(|hit| hit.kind == HitKind::Watchlist)
        .expect("expected a watchlist hit");
    assert_eq!(hit.matched, "CONTOSO WORKS");
}

#[test]
fn oversize_file_emits_coverage_skip_warning() {
    let temp = tempdir().unwrap();
    let big = "x".repeat(64);
    fs::write(temp.path().join("big.txt"), format!("192.168.50.9\n{big}")).unwrap();
    let config = Config {
        max_file_size: 16,
        ..Default::default()
    };
    let matcher = watchlist::WatchlistMatcher::new(Vec::new()).unwrap();
    let patterns = patterns::build(&config).unwrap();
    let result = scan_paths(
        ScanMode::AllTracked,
        vec!["big.txt".to_owned()],
        temp.path(),
        &config,
        &matcher,
        &patterns,
        Vec::new(),
    )
    .unwrap();
    assert_eq!(result.coverage_skips, 1);
    assert_eq!(result.scanned, 0);
    assert_eq!(result.exempt_or_skipped, 1);
    assert_eq!(result.total_files, 1);
    assert!(
        result
            .warnings
            .iter()
            .any(|w| w.contains("skipped big.txt") && w.contains("oversize"))
    );
    assert!(result.hits.is_empty());
}

#[test]
fn bom_is_removed_from_lines_values_and_csv_headers() {
    let temp = tempdir().unwrap();
    fs::write(temp.path().join("lines.txt"), "\u{feff}SyntheticBomLine\n").unwrap();
    fs::write(
        temp.path().join("values.csv"),
        "\u{feff}name,kind\nSyntheticBomCsv,fixture\n",
    )
    .unwrap();
    let config = Config {
        watchlists: vec![
            WatchlistSource::Lines {
                path: "${FIXTURE_ROOT}/lines.txt".to_owned(),
                label: Some("synthetic lines".to_owned()),
            },
            WatchlistSource::Csv {
                path: "${FIXTURE_ROOT}/values.csv".to_owned(),
                column: ColumnSpec::Name("name".to_owned()),
                label: Some("synthetic csv".to_owned()),
                paren_variants: false,
            },
        ],
        ..Default::default()
    };
    let env = HashMap::from([(
        "FIXTURE_ROOT".to_owned(),
        temp.path().to_string_lossy().into_owned(),
    )]);

    let matcher = watchlist::load(&config, temp.path(), &env).unwrap().matcher;

    assert!(matcher.matches("SyntheticBomLine").next().is_some());
    assert!(matcher.matches("SyntheticBomCsv").next().is_some());
}

#[test]
fn watchlist_has_a_hard_size_ceiling_independent_of_config() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("oversized.txt");
    let file = fs::File::create(&path).unwrap();
    file.set_len((64 * 1024 * 1024) + 1).unwrap();
    let mut config = Config {
        max_file_size: 128 * 1024 * 1024,
        ..Default::default()
    };
    config.watchlists.push(WatchlistSource::Lines {
        path: "${FIXTURE_ROOT}/oversized.txt".to_owned(),
        label: Some("synthetic oversized".to_owned()),
    });
    let env = HashMap::from([(
        "FIXTURE_ROOT".to_owned(),
        temp.path().to_string_lossy().into_owned(),
    )]);

    let error = watchlist::load(&config, temp.path(), &env).unwrap_err();

    assert!(error.to_string().contains("effective limit is 67108864"));
}

#[test]
fn config_rejects_unknown_watchlist_fields_and_unsafe_exempt_paths() {
    let typo =
        r#"{"watchlists":[{"type":"lines","path":"${FIXTURE_ROOT}/names.txt","lable":"typo"}]}"#;
    assert!(serde_json::from_str::<Config>(typo).is_err());

    for invalid in [
        "../private",
        "/private",
        "C:/private",
        "./generated",
        "generated//nested",
    ] {
        let config = Config {
            exempt_paths: vec![invalid.to_owned()],
            ..Default::default()
        };
        assert!(config.validate().is_err(), "must reject {invalid:?}");
    }

    for valid in ["generated", "generated/", "docs/generated.txt"] {
        let config = Config {
            exempt_paths: vec![valid.to_owned()],
            ..Default::default()
        };
        config.validate().unwrap();
    }
}
