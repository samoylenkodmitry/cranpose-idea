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
