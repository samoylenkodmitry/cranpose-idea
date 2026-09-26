use super::*;

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
