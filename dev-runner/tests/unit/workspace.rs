use super::*;

fn metadata(root: &Path) -> Metadata {
    Metadata {
        workspace_root: root.to_owned(),
        target_directory: root.join("target"),
        packages: vec![Package {
            id: "app".into(),
            name: "app".into(),
            features: BTreeMap::new(),
            manifest_path: root.join("Cargo.toml"),
            dependencies: vec![Dependency {
                name: "cranpose".into(),
                rename: None,
                path: None,
            }],
            targets: vec![Target {
                name: "app".into(),
                kind: vec!["bin".into()],
                src_path: root.join("src/main.rs"),
                required_features: vec![],
            }],
        }],
    }
}

#[test]
fn development_copy_preserves_every_original_byte() {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("app");
    fs::create_dir_all(root.join("src")).expect("source directory");
    let manifest = "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\ncranpose='0.1'\n[profile.release]\nlto=true\n";
    let source = "#![forbid(unsafe_code)]\n#[cranpose::composable]\nfn App() {}\nfn main() {}\n";
    fs::write(root.join("Cargo.toml"), manifest).expect("manifest");
    fs::write(root.join("Cargo.lock"), "untouched lockfile").expect("lock");
    fs::write(root.join("src/main.rs"), source).expect("source");
    let workspace = DevWorkspace::prepare(
        &metadata(&root),
        &temp.path().join("cache/project"),
        &temp.path().join("cache/macros"),
    )
    .expect("prepare");
    assert_eq!(
        fs::read_to_string(root.join("Cargo.toml")).expect("manifest"),
        manifest
    );
    assert_eq!(
        fs::read_to_string(root.join("Cargo.lock")).expect("lock"),
        "untouched lockfile"
    );
    assert_eq!(
        fs::read_to_string(root.join("src/main.rs")).expect("source"),
        source
    );
    let copied =
        fs::read_to_string(workspace.directory.join("src/main.rs")).expect("private source");
    assert_eq!(copied.lines().count(), source.lines().count());
    assert!(copied.contains("#[cranpose_dev_macros::hot] #[cranpose::composable]"));
    assert!(copied.starts_with("#![forbid(unsafe_code)]\n"));
    assert!(syn::parse_file(&copied).is_ok());
    assert!(
        workspace
            .write_source(Path::new("../escaped.rs"), "")
            .is_err()
    );
    assert!(
        DevWorkspace::prepare(
            &metadata(&root),
            &root.join("inside"),
            &temp.path().join("support")
        )
        .is_err()
    );
}

#[test]
fn plain_preview_has_no_reload_runtime_and_external_dependencies_still_resolve() {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("app");
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::create_dir_all(temp.path().join("shared")).expect("external dependency");
    let manifest = "[package]\nname='app'\nversion='0.1.0'\n[dependencies]\ncranpose='0.1'\nshared={path='../shared'}\n";
    fs::write(root.join("Cargo.toml"), manifest).expect("manifest");
    let source = "#[cranpose::composable] fn App() {}\nfn main() {}";
    fs::write(root.join("src/main.rs"), source).expect("source");
    let workspace = DevWorkspace::prepare_with_mode(
        &metadata(&root),
        &temp.path().join("cache"),
        &temp.path().join("support"),
        false,
    )
    .expect("plain preview");
    assert_eq!(
        fs::read_to_string(workspace.directory.join("src/main.rs")).expect("copy"),
        source
    );
    let private = fs::read_to_string(workspace.directory.join("Cargo.toml"))
        .expect("manifest")
        .parse::<DocumentMut>()
        .expect("toml");
    assert!(private["dependencies"].get("subsecond").is_none());
    assert!(private["dependencies"].get("cranpose-dev-macros").is_none());
    assert_eq!(
        private["dependencies"]["shared"]["path"].as_str(),
        temp.path()
            .join("shared")
            .canonicalize()
            .expect("canonical")
            .to_str()
    );
    assert_eq!(
        fs::read_to_string(root.join("Cargo.toml")).expect("original"),
        manifest
    );
}

#[test]
fn private_example_launcher_preserves_dev_dependencies_and_library_features() {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("app");
    fs::create_dir_all(root.join("src")).expect("src");
    fs::create_dir_all(root.join("examples")).expect("examples");
    let manifest = "[package]\nname='app'\nversion='0.1.0'\n[workspace]\n[dependencies]\ncranpose='0.1'\n[dev-dependencies]\nserde='1'\n[features]\nextra=[]\n";
    fs::write(root.join("Cargo.toml"), manifest).expect("manifest");
    fs::write(
        root.join("src/lib.rs"),
        "#[cranpose::composable] pub fn App() {}",
    )
    .expect("library");
    fs::write(root.join("examples/demo.rs"), "fn main() {}").expect("example");
    let mut metadata = metadata(&root);
    metadata.packages[0].features.insert("extra".into(), vec![]);
    metadata.packages[0].targets = vec![
        Target {
            name: "app".into(),
            kind: vec!["lib".into()],
            src_path: root.join("src/lib.rs"),
            required_features: vec![],
        },
        Target {
            name: "demo".into(),
            kind: vec!["example".into()],
            src_path: root.join("examples/demo.rs"),
            required_features: vec!["extra".into()],
        },
    ];
    let mut workspace = DevWorkspace::prepare(
        &metadata,
        &temp.path().join("cache"),
        &temp.path().join("support"),
    )
    .expect("private workspace");
    let package = &metadata.packages[0];
    crate::launcher::prepare(&mut workspace, package, &package.targets[1]).expect("launcher");
    let launcher = fs::read_to_string(
        workspace
            .directory
            .join(".cranpose-dev/launcher/Cargo.toml"),
    )
    .expect("launcher manifest")
    .parse::<DocumentMut>()
    .expect("toml");
    assert_eq!(launcher["dependencies"]["serde"].as_str(), Some("1"));
    assert_eq!(
        launcher["features"]["extra"]
            .as_array()
            .expect("feature")
            .get(0)
            .and_then(Value::as_str),
        Some("app/extra")
    );
    assert!(launcher.get("lib").is_none());
    assert_eq!(
        fs::read_to_string(root.join("Cargo.toml")).expect("original"),
        manifest
    );
}
