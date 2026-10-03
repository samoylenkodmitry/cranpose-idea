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
    let source =
        "#![forbid(unsafe_code)]\n/* hé😀 */ #[cranpose::composable]\nfn App() {}\nfn main() {}\n";
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
    assert!(copied.contains("/* hé😀 */ extern crate"), "{copied}");
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
fn explicit_library_types_receive_preview_helpers() {
    for kinds in [
        vec!["lib"],
        vec!["rlib"],
        vec!["dylib"],
        vec!["cdylib"],
        vec!["staticlib"],
        vec!["rlib", "cdylib", "staticlib"],
    ] {
        let temp = tempfile::tempdir().expect("temp");
        let root = temp.path().join("app");
        fs::create_dir_all(root.join("src")).expect("src");
        fs::create_dir_all(root.join("framework/src")).expect("framework");
        let manifest = format!(
            "[package]\nname='app'\nversion='0.1.0'\n[workspace]\n[lib]\ncrate-type={kinds:?}\n[dependencies]\nui={{package='cranpose',path='framework'}}\n"
        );
        fs::write(root.join("Cargo.toml"), &manifest).expect("manifest");
        fs::write(
            root.join("framework/Cargo.toml"),
            "[package]\nname='cranpose'\nversion='0.1.0'\n",
        )
        .expect("framework manifest");
        fs::write(root.join("framework/src/lib.rs"), "").expect("framework source");
        let source =
            "#![forbid(unsafe_code)]\n#[ui::composable] pub fn App() { Text(\"Hello\"); }\n";
        fs::write(root.join("src/lib.rs"), source).expect("library");
        fs::write(root.join("src/main.rs"), "fn main() {}\n").expect("binary");
        let metadata = Metadata::read(&root).expect("Cargo metadata");
        let mut workspace = DevWorkspace::prepare(
            &metadata,
            &temp.path().join("cache"),
            &temp.path().join("support"),
        )
        .expect("private workspace");
        let copied =
            fs::read_to_string(workspace.directory.join("src/lib.rs")).expect("private library");
        assert!(
            copied.contains("crate::__cranpose_dev::literal"),
            "{copied}"
        );
        assert!(
            copied.contains("extern crate ui as __cranpose_api;"),
            "{kinds:?}: {copied}"
        );
        assert!(copied.contains("mod __cranpose_dev"), "{kinds:?}: {copied}");
        assert_eq!(copied.lines().count(), source.lines().count());
        assert!(copied.starts_with("#![forbid(unsafe_code)]\n"));
        assert!(syn::parse_file(&copied).is_ok());
        let package = metadata
            .packages
            .iter()
            .find(|package| package.name == "app")
            .expect("app");
        let binary = package
            .targets
            .iter()
            .find(|target| target.kind == ["bin"])
            .expect("binary");
        assert!(
            crate::launcher::prepare(&mut workspace, package, binary)
                .expect("launcher")
                .is_some()
        );
        assert_eq!(
            fs::read_to_string(root.join("src/lib.rs")).expect("original"),
            source
        );
        assert_eq!(
            fs::read_to_string(root.join("Cargo.toml")).expect("original manifest"),
            manifest
        );
    }
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

#[test]
fn repeated_launches_reuse_support_without_touching_compiler_inputs() {
    let temp = tempfile::tempdir().expect("cache");
    let root = temp.path().join("support/dev-macros");
    let first = write_support(&root).expect("support");
    let source = first.join("src/lib.rs");
    let stamp = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(123456);
    fs::File::options()
        .write(true)
        .open(&source)
        .expect("source")
        .set_modified(stamp)
        .expect("mtime");
    let actual = fs::metadata(&source)
        .expect("metadata")
        .modified()
        .expect("stamp");
    assert_eq!(write_support(&root).expect("reused"), first);
    assert_eq!(
        fs::metadata(&source)
            .expect("metadata")
            .modified()
            .expect("stamp"),
        actual
    );
    assert!(
        first
            .parent()
            .expect("bundle")
            .join("runtime/src/lib.rs")
            .is_file()
    );
}

#[test]
fn hot_reload_keys_follow_the_application_cranpose() {
    let root = tempfile::tempdir().expect("temporary workspace");
    let metadata = metadata(root.path());
    let registry = Dependency {
        name: "cranpose".into(),
        rename: None,
        path: None,
    };
    assert!(
        !metadata.cranpose_hot_reload(&registry, None),
        "no lock file"
    );
    // Locked sources are read from Cargo's caches when present.
    let home = tempfile::tempdir().expect("cargo home");
    let checkout = home
        .path()
        .join("git/checkouts/cranpose-0123456789abcdef/abcdef1/crates/cranpose");
    fs::create_dir_all(&checkout).expect("checkout");
    fs::write(
        checkout.join("Cargo.toml"),
        "[package]\nname = \"cranpose\"\nversion = \"0.1.174\"\n[features]\nhot-reload = []\n",
    )
    .expect("manifest");
    assert_eq!(
        locked_manifest(
            home.path(),
            "cranpose",
            "0.1.174",
            "git+https://github.com/samoylenkodmitry/Cranpose?rev=abcdef1#abcdef1234567890",
        ),
        Some(checkout.join("Cargo.toml"))
    );
    let registry_source = home
        .path()
        .join("registry/src/index.crates.io-1/cranpose-0.1.174");
    fs::create_dir_all(&registry_source).expect("registry source");
    fs::write(
        registry_source.join("Cargo.toml"),
        "[package]\nname = \"cranpose\"\n",
    )
    .expect("manifest");
    assert_eq!(
        locked_manifest(
            home.path(),
            "cranpose",
            "0.1.174",
            "registry+https://github.com/rust-lang/crates.io-index",
        ),
        Some(registry_source.join("Cargo.toml"))
    );
    for (version, expected) in [
        ("0.1.174", false),
        ("0.1.175", true),
        ("0.2.0", true),
        ("0.1.176-dev", true),
    ] {
        fs::write(
            root.path().join("Cargo.lock"),
            format!(
                "version = 4\n[[package]]\nname = \"cranpose\"\nversion = \"{version}\"\n\n[[package]]\nname = \"app\"\nversion = \"9.9.9\"\n"
            ),
        )
        .expect("lock");
        assert_eq!(
            metadata.cranpose_hot_reload(&registry, None),
            expected,
            "{version}"
        );
    }
    let local = root.path().join("cranpose");
    fs::create_dir_all(&local).expect("local framework");
    fs::write(
        local.join("Cargo.toml"),
        "[package]\nname = \"cranpose\"\n[features]\npreview = []\n",
    )
    .expect("manifest");
    let path = Dependency {
        name: "cranpose".into(),
        rename: None,
        path: Some(local.clone()),
    };
    assert!(!metadata.cranpose_hot_reload(&path, None));
    fs::write(
        local.join("Cargo.toml"),
        "[package]\nname = \"cranpose\"\n[features]\nhot-reload = []\n",
    )
    .expect("manifest");
    assert!(metadata.cranpose_hot_reload(&path, None));
}

#[test]
fn unresolved_graphs_ask_cargo_for_the_features() {
    let root = tempfile::tempdir().expect("temporary workspace");
    let framework = root.path().join("framework");
    fs::create_dir_all(framework.join("src")).expect("framework");
    fs::write(framework.join("src/lib.rs"), "").expect("library");
    fs::write(
        framework.join("Cargo.toml"),
        "[package]\nname = \"cranpose\"\nversion = \"0.1.174\"\nedition = \"2024\"\n[features]\nhot-reload = []\n",
    )
    .expect("framework manifest");
    let app = root.path().join("app");
    fs::create_dir_all(app.join("src")).expect("app");
    fs::write(app.join("src/main.rs"), "fn main() {}\n").expect("main");
    fs::write(
        app.join("Cargo.toml"),
        "[package]\nname = \"app\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n[dependencies]\ncranpose = { path = \"../framework\" }\n",
    )
    .expect("app manifest");
    assert_eq!(resolved_feature(&app, "cranpose", "hot-reload"), Some(true));
    assert_eq!(resolved_feature(&app, "cranpose", "missing"), Some(false));
}
