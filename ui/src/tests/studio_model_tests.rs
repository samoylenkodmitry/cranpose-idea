use super::*;

fn studio() -> Studio {
    let mut studio = Studio::default();
    studio.handle(
        "studio.init",
        r#"{"root":"/project","cache":"/cache","settings":{"width":480,"height":640}}"#,
    );
    studio.handle("cranpose.project", r#"{"targets":[{"packageName":"app","name":"desktop","kind":"bin","manifest":"/project/Cargo.toml","source":"/project/src/main.rs"}]}"#);
    studio
}

#[test]
fn native_controller_starts_only_a_debug_development_runner() {
    let mut studio = studio();
    let request = studio.start().expect("runnable target");
    assert_eq!(request["options"]["root"], "/project");
    assert_eq!(request["options"]["hotReload"], true);
    assert!(request["options"].get("release").is_none());
    assert!(studio.busy);
    assert_eq!(studio.session, 1);
}

#[test]
fn stale_session_events_cannot_replace_current_inspection() {
    let mut studio = studio();
    studio.start();
    studio.start();
    studio.handle(
        "studio.child",
        r#"{"session":1,"event":"stopped","message":"stale"}"#,
    );
    assert_eq!(studio.status, "Preparing preview…");
    studio.handle("studio.child", r#"{"session":2,"event":"connected"}"#);
    assert!(studio.connected);
}

#[test]
fn failed_candidate_keeps_previous_preview_available() {
    let mut studio = studio();
    studio.start();
    studio.start();
    studio.handle(
        "studio.child",
        r#"{"session":2,"event":"failed","message":"compile error","fallbackSession":1}"#,
    );
    assert_eq!(studio.session, 1);
    assert!(studio.connected);
    assert!(!studio.busy);
    studio.start();
    assert_eq!(studio.session, 3, "failed session IDs must never be reused");
}

#[test]
fn maps_private_compilation_paths_back_to_real_sources() {
    let mut studio = studio();
    studio.private_root = "/cache/session".into();
    let source = Source {
        manifest_dir: "/cache/session/app".into(),
        file: "src/card.rs".into(),
        line: 10,
        ..Source::default()
    };
    assert_eq!(
        studio.resolve_source(&source),
        PathBuf::from("/project/app/src/card.rs")
    );
}

#[test]
fn inspector_validates_parent_order_and_picks_deepest_node() {
    let snapshot = Snapshot::parse(r#"{"schema":2,"nodes":[{"id":"root","width":100,"height":100},{"id":"child","parent":"root","x":10,"y":10,"width":30,"height":30}]}"#).expect("valid snapshot");
    assert_eq!(
        snapshot.pick(20.0, 20.0).map(|node| node.id.as_str()),
        Some("child")
    );
    assert_eq!(
        snapshot.pick(90.0, 90.0).map(|node| node.id.as_str()),
        Some("root")
    );
    assert!(
        Snapshot::parse(r#"{"schema":2,"nodes":[{"id":"child","parent":"missing"}]}"#).is_err()
    );
}

#[test]
fn hot_patch_acknowledgement_preserves_selection_and_viewport() {
    let mut studio = studio();
    studio.start();
    studio.selected = "node-1".into();
    studio.settings.width = 720;
    studio.handle("studio.child", r#"{"session":1,"event":"message","channel":"cranpose.dev.applied","payload":"{\"generation\":3,\"pid\":42}"}"#);
    assert_eq!(studio.selected, "node-1");
    assert_eq!(studio.settings.width, 720);
    assert_eq!(studio.generation, 3);
    assert_eq!(studio.pid, 42);
    assert!(!studio.busy);
}

#[test]
fn pending_build_starts_when_metadata_arrives_and_warnings_are_not_failures() {
    let mut studio = Studio::default();
    assert!(
        studio
            .handle("studio.command", r#"{"action":"build"}"#)
            .is_empty()
    );
    let requests = studio.handle(
        "cranpose.project",
        r#"{"root":"/project","targets":[{"packageName":"app","name":"app","kind":"bin"}]}"#,
    );
    assert_eq!(requests.len(), 1);
    studio.log(r#"{"reason":"compiler-message","message":{"level":"warning","message":"unused"}}"#);
    assert!(studio.diagnostics.is_empty());
    studio.log(r#"{"$message_type":"diagnostic","level":"error","message":"overflow"}"#);
    assert_eq!(studio.diagnostics.len(), 1);
}

#[test]
fn maps_binary_launcher_paths_before_workspace_paths() {
    let mut studio = studio();
    studio.private_root = "/cache/session".into();
    studio.log(r#"{"cranposeDev":"sourceMap","private":"/cache/session/.cranpose-dev/launcher","original":"/project/app"}"#);
    let path = studio.resolve_source(&Source {
        manifest_dir: "/cache/session/.cranpose-dev/launcher".into(),
        file: "src/main.rs".into(),
        ..Source::default()
    });
    assert_eq!(path, PathBuf::from("/project/app/src/main.rs"));
}
