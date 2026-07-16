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
    assert!(allowed_by_directive(
        "Northwind Harbor // doxguard: allow Northwind",
        "Northwind Harbor"
    ));
    assert!(allowed_by_directive(
        "192.168.50.9 # doxguard: allow",
        "192.168.50.9"
    ));
    assert!(allowed_by_directive(
        "Contoso Works <!-- secrets-scan: allow Contoso -->",
        "Contoso Works"
    ));
    assert!(!allowed_by_directive(
        "Northwind Harbor // doxguard: allow Fabrikam",
        "Northwind Harbor"
    ));
}
