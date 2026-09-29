use super::*;

const APP: &str = "#[composable]\nfn App() {\n    Text(\"a\");\n    Text(12);\n}\n\nstruct Palette { accent: f32 }\n\nfn main() {}\n";

fn project() -> (tempfile::TempDir, DevWorkspace) {
    let temp = tempfile::tempdir().expect("temp");
    let root = temp.path().join("app");
    fs::create_dir_all(root.join("src")).expect("source directory");
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='app'\nversion='0.1.0'\n",
    )
    .expect("manifest");
    fs::write(root.join("src/main.rs"), APP).expect("source");
    let metadata = crate::workspace::Metadata {
        workspace_root: root.clone(),
        target_directory: root.join("target"),
        packages: vec![crate::workspace::Package {
            id: "app".into(),
            name: "app".into(),
            features: Default::default(),
            manifest_path: root.join("Cargo.toml"),
            dependencies: vec![],
            targets: vec![],
        }],
    };
    let workspace = DevWorkspace::prepare(
        &metadata,
        &temp.path().join("cache/project"),
        &temp.path().join("cache/macros"),
    )
    .expect("prepare");
    (temp, workspace)
}

#[test]
fn saved_files_become_values_patches_or_rebuilds_with_reasons() {
    let (_temp, mut workspace) = project();
    let source = workspace.original.join("src/main.rs");
    let main = Path::new("src/main.rs");
    let mut compiled = workspace.sources.clone();
    let save = |text: &str| fs::write(&source, text).expect("save");
    // A wider literal moves later columns but keeps the compiled schema.
    let wider = APP.replace("\"a\"", "\"a much longer label\"");
    save(&wider);
    assert_eq!(
        change(&workspace, &compiled, main, true),
        Change::Values(wider.clone())
    );
    workspace.sources.insert(main.into(), wider.clone());
    // Returning to the compiled text restores its values; it is not unchanged.
    save(APP);
    assert_eq!(
        change(&workspace, &compiled, main, true),
        Change::Values(APP.into())
    );
    // Structural body edits compile against the compiled source, not the edited text.
    let structural = wider.replace("    Text(12);\n", "    Text(12);\n    Text(\"added\");\n");
    save(&structural);
    assert_eq!(
        change(&workspace, &compiled, main, true),
        Change::Compile(structural.clone())
    );
    assert_eq!(
        change(&workspace, &compiled, main, false),
        Change::Restart("`src/main.rs` changed".into())
    );
    compiled.insert(main.into(), structural.clone());
    workspace.sources.insert(main.into(), structural.clone());
    assert_eq!(change(&workspace, &compiled, main, true), Change::Unchanged);
    save(&structural.replace("accent: f32", "accent: f64"));
    assert_eq!(
        change(&workspace, &compiled, main, true),
        Change::Restart("Struct `Palette` fields changed".into())
    );
    save(&structural.replace("fn main() {}", "fn main() {"));
    assert!(matches!(
        change(&workspace, &compiled, main, true),
        Change::Invalid(error) if error.starts_with("src/main.rs: ")
    ));
    fs::remove_file(&source).expect("remove");
    assert_eq!(
        change(&workspace, &compiled, main, true),
        Change::Restart("`src/main.rs` removed".into())
    );
}

#[test]
fn other_files_rebuild_only_when_their_content_changes() {
    let (_temp, workspace) = project();
    let compiled = workspace.sources.clone();
    let manifest = workspace.original.join("Cargo.toml");
    let original = fs::read_to_string(&manifest).expect("manifest");
    fs::write(&manifest, &original).expect("identical rewrite");
    assert_eq!(
        change(&workspace, &compiled, Path::new("Cargo.toml"), true),
        Change::Unchanged
    );
    fs::write(&manifest, format!("{original}[features]\nextra=[]\n")).expect("edit");
    assert_eq!(
        change(&workspace, &compiled, Path::new("Cargo.toml"), true),
        Change::Restart("`Cargo.toml` changed".into())
    );
    // Editors' short-lived files never existed in the private copy.
    assert_eq!(
        change(
            &workspace,
            &compiled,
            Path::new("assets/temporary.json"),
            true
        ),
        Change::Unchanged
    );
    fs::create_dir_all(workspace.original.join("assets")).expect("assets");
    fs::write(workspace.original.join("assets/icon.svg"), "<svg/>").expect("asset");
    assert_eq!(
        change(&workspace, &compiled, Path::new("assets/icon.svg"), true),
        Change::Restart("`assets/icon.svg` changed".into())
    );
    fs::write(workspace.original.join("build.rs"), "fn main() {}").expect("build script");
    assert_eq!(
        change(&workspace, &compiled, Path::new("build.rs"), true),
        Change::Restart("`build.rs` changed".into())
    );
    // A new module is written for the compiler; its `mod` item rebuilds the preview.
    fs::write(workspace.original.join("src/widgets.rs"), "fn chip() {}").expect("module");
    assert_eq!(
        change(&workspace, &compiled, Path::new("src/widgets.rs"), true),
        Change::Compile("fn chip() {}".into())
    );
}

#[test]
fn rebuild_requests_wait_for_quiet_edits_and_can_be_withdrawn() {
    let start = Instant::now();
    let mut rebuild = Rebuild::default();
    assert_eq!(rebuild.due(start), None);
    rebuild.request("Struct `Palette` fields changed", start + REBUILD_DELAY);
    assert_eq!(rebuild.due(start + Duration::from_millis(100)), None);
    // Another restart-class save moves the deadline.
    rebuild.request(
        "Struct `Palette` fields changed",
        start + Duration::from_millis(300) + REBUILD_DELAY,
    );
    assert_eq!(rebuild.due(start + REBUILD_DELAY), None);
    assert_eq!(
        rebuild.due(start + Duration::from_millis(700)).as_deref(),
        Some("Struct `Palette` fields changed")
    );
    assert_eq!(rebuild.due(start + Duration::from_secs(5)), None);
    // A syntax error or a reverted edit withdraws the pending request.
    rebuild.request("`main` changed", start);
    rebuild.cancel();
    assert_eq!(rebuild.due(start + Duration::from_secs(5)), None);
}

#[test]
fn only_failed_patch_mechanics_request_a_rebuild() {
    let mut patches = Patches::default();
    let failed = |compile_error| Compiler::BuildFailed { compile_error };
    // The initial build failing is the user's to fix; there is no running patch base.
    assert_eq!(patches.report(failed(false)), None);
    patches.started();
    assert_eq!(patches.report(Compiler::CompileError), None);
    assert_eq!(patches.report(failed(false)), None);
    // Workspace dependency crates embed their diagnostics in the failure itself.
    patches.started();
    assert_eq!(patches.report(failed(true)), None);
    patches.started();
    assert_eq!(patches.report(Compiler::Patched), None);
    assert_eq!(patches.report(failed(false)), None);
    patches.started();
    assert_eq!(
        patches.report(failed(false)),
        Some(Outcome::Unbuilt("Hot patch could not be built".into()))
    );
    assert_eq!(
        patches.report(Compiler::PatchFailed(
            "Hot patch failed: Invalid build id".into()
        )),
        Some(Outcome::Behind("Hot patch failed: Invalid build id".into()))
    );
}

#[test]
fn recognizes_hot_patch_outcomes_in_compiler_output() {
    for (line, report) in [
        (
            r#"{"level":"INFO","message":"Hot-patching: \u001b[32msrc/main.rs\u001b[0m took \u001b[33m623ms\u001b[0m","timestamp":"  5.07s"}"#,
            Some(Compiler::Patched),
        ),
        (
            r#"{"$message_type":"diagnostic","children":[],"code":null,"level":"error","message":"cannot add `&str` to `i32`","spans":[]}"#,
            Some(Compiler::CompileError),
        ),
        (
            r#"{"reason":"compiler-message","message":{"level":"error","message":"mismatched types"}}"#,
            Some(Compiler::CompileError),
        ),
        (
            r#"{"$message_type":"diagnostic","level":"warning","message":"unused variable"}"#,
            None,
        ),
        (
            r#"{"level":"ERROR","message":"\u001b[31mBuild failed\u001b[0m: Cargo build failed - no output location.","timestamp":"  3.22s"}"#,
            Some(Compiler::BuildFailed {
                compile_error: false,
            }),
        ),
        (
            r#"{"level":"ERROR","message":"Build failed: Failed to replay workspace crate 'app'\n 1: Failed to compile workspace dep crate 'app':\n {\"$message_type\":\"diagnostic\",\"message\":\"literal out of range for `i32`\",\"level\":\"error\"}"}"#,
            Some(Compiler::BuildFailed {
                compile_error: true,
            }),
        ),
        (
            r#"{"level":"ERROR","message":"Failed to hot-patch app: Invalid build id","timestamp":"  9.1s"}"#,
            Some(Compiler::PatchFailed(
                "Hot patch failed: Invalid build id".into(),
            )),
        ),
        (
            r#"{"level":"INFO","message":"Starting full rebuild: No debug symbols in the patch output.","timestamp":"  9.1s"}"#,
            Some(Compiler::PatchFailed(
                "Hot patch failed: No debug symbols in the patch output.".into(),
            )),
        ),
        (
            r#"{"level":"WARN","message":"No clients to hotreload - try reloading the app!","timestamp":"  9.1s"}"#,
            Some(Compiler::PatchFailed(
                "Hot patch failed: the preview is not connected to the compiler".into(),
            )),
        ),
        (
            r#"{"level":"WARN","message":"Ignoring patch rebuild for BuildId(0) since there is no existing build.","timestamp":"  1.14s"}"#,
            Some(Compiler::PatchFailed("Previous build failed".into())),
        ),
        (
            r#"{"level":"INFO","message":"Build completed successfully in 1.2s, launching app! 💫"}"#,
            None,
        ),
        ("   Compiling app v0.1.0", None),
    ] {
        assert_eq!(compiler_report(line), report, "{line}");
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
