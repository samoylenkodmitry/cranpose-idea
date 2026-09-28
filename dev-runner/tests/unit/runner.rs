use super::*;

#[test]
fn live_literal_width_changes_compile_when_values_cannot_be_sent() {
    let initial = "#[composable] fn App() { Text(\"a\"); Text(12); }";
    let edited = initial.replace("\"a\"", "\"a much longer label\"");
    let initial_catalog = cranpose_plugin_authoring::Catalog::parse(initial).expect("initial");
    let edited_catalog = cranpose_plugin_authoring::Catalog::parse(&edited).expect("edited");
    assert_eq!(initial_catalog.schema, edited_catalog.schema);
    // Moved call sites compile as a hot patch; only shared declarations rebuild.
    for next in [edited.clone(), edited.replace("12", "13")] {
        assert_eq!(classify(initial, &next), ReloadDecision::Patch);
    }
}

#[test]
fn compiler_diagnostics_map_windows_escaping_and_private_launcher_paths() {
    let windows =
        serde_json::json!({"message": {"spans": [{"file_name": r"C:\cache\session\src\main.rs"}]}})
            .to_string();
    let result: serde_json::Value = serde_json::from_str(&map_output(
        &windows,
        &[(
            PathBuf::from(r"C:\cache\session"),
            PathBuf::from(r"C:\project"),
        )],
    ))
    .expect("mapped JSON");
    assert_eq!(
        result["message"]["spans"][0]["file_name"],
        r"C:\project\src\main.rs"
    );
    let result = map_output(
        ".cranpose-dev/launcher/src/main.rs:12",
        &[(".cranpose-dev/launcher".into(), "/project/app".into())],
    );
    assert_eq!(result, "/project/app/src/main.rs:12");
}

#[test]
fn filters_build_and_editor_noise_before_debounce_without_losing_atomic_save_paths() {
    let directory = tempfile::tempdir().expect("directory");
    let root = directory.path().canonicalize().expect("root");
    let target = root.join("custom-output");
    for name in [
        "target/events.json",
        ".idea/workspace.xml",
        ".git/index.lock",
        "node_modules/lib.rs",
        ".gradle/settings.toml",
        ".intellijPlatform/index.json",
        ".venv/library.rs",
        "custom-output/build.rs",
        "src/.main.rs.swp",
    ] {
        assert_eq!(
            relevant_path(&root, &target, root.join(name)),
            None,
            "{name}"
        );
    }
    for name in [
        "src/main.rs",
        "Cargo.toml",
        "Cargo.lock",
        "assets/icon.png",
        "src/new.rs",
    ] {
        assert_eq!(
            relevant_path(&root, &target, root.join(name)),
            Some(name.into()),
            "{name}"
        );
    }
    assert!(
        relevant_path(
            &root,
            &target,
            root.parent().expect("parent").join("outside.rs")
        )
        .is_none()
    );
    fs::create_dir_all(root.join("src")).expect("src");
    fs::write(root.join("src/main.rs"), "fn main() {}").expect("source");
    let temporary = root.join("src/main.rs.tmp");
    fs::write(&temporary, "fn main() { println!(\"edited\"); }").expect("temporary");
    fs::rename(temporary, root.join("src/main.rs")).expect("atomic save");
    assert_eq!(
        relevant_path(&root, &target, root.join("src/main.rs")),
        Some("src/main.rs".into())
    );
    fs::remove_file(root.join("src/main.rs")).expect("remove");
    assert_eq!(
        relevant_path(&root, &target, root.join("src/main.rs")),
        Some("src/main.rs".into())
    );
}
