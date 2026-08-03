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
