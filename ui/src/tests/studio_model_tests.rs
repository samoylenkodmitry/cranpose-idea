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

#[test]
fn inspector_search_includes_full_text_source_and_modifier_values() {
    let snapshot = Snapshot::parse(r#"{"schema":2,"nodes":[{"id":"root","kind":"Column"},{"id":"label","parent":"root","kind":"Text","text":"Café quiet morning","sources":[{"name":"Card","file":"src/card.rs"}],"modifiers":[{"name":"padding","properties":[{"name":"all","value":"12"}]}]}]}"#).expect("snapshot");
    for query in ["café morning", "card.rs", "padding 12"] {
        let rows = snapshot.rows(query, &Default::default());
        assert_eq!(rows.len(), 2);
        assert!(!rows[0].matches);
        assert!(rows[1].matches);
    }
}

#[test]
fn picking_reveals_ancestors_and_snapshot_removal_clears_stale_selection() {
    let mut studio = studio();
    studio.start();
    studio.snapshot =
        Snapshot::parse(r#"{"schema":2,"nodes":[{"id":"root"},{"id":"child","parent":"root"}]}"#)
            .expect("snapshot");
    studio.collapsed.insert("root".into());
    studio.select_node("child".into());
    assert!(studio.collapsed.is_empty());
    assert!(studio.inspector_details);
    assert_eq!(
        studio
            .selection_path()
            .iter()
            .map(|n| n.id.as_str())
            .collect::<Vec<_>>(),
        vec!["root", "child"]
    );
    studio.handle("studio.child", r#"{"session":1,"event":"message","channel":"cranpose.inspector.v2.snapshot","payload":"{\"schema\":2,\"requestId\":2,\"nodes\":[]}"}"#);
    assert!(studio.selected.is_empty());
}

#[test]
fn rebuilt_ui_restores_running_session_without_restarting_application() {
    let mut previous = studio();
    previous.start();
    previous.start();
    previous.connected = true;
    previous.pid = 42;
    previous.settings.inspect = true;
    previous.selected = "child".into();
    previous
        .source_maps
        .push(("/cache/launcher".into(), "/project".into()));
    let checkpoint = serde_json::to_value(&previous).expect("checkpoint");
    assert!(
        checkpoint.get("snapshot").is_none(),
        "layout frames must not be copied into every checkpoint"
    );
    let mut restored = Studio::default();
    let requests = restored.handle("studio.init", &json!({"root":"/project","cache":"/cache","checkpoint":checkpoint,"activeSession":2,"candidateSession":0}).to_string());
    assert!(
        requests.is_empty(),
        "reconnection must not start another application"
    );
    assert!(restored.connected && restored.initialized && !restored.busy);
    assert_eq!(restored.pid, 42);
    assert_eq!(restored.selected, "child");
    assert_eq!(restored.source_maps, previous.source_maps);
    restored.targets = previous.targets;
    assert_eq!(restored.start().expect("target")["session"], 3);
}

#[test]
fn native_session_state_wins_over_a_stale_ui_checkpoint() {
    let mut previous = studio();
    previous.start();
    previous.connected = true;
    previous.pid = 42;
    let mut restored = Studio::default();
    restored.handle(
        "studio.init",
        &json!({"checkpoint":previous,"activeSession":0,"candidateSession":0}).to_string(),
    );
    assert_eq!(restored.session, 0);
    assert_eq!(restored.pid, 0);
    assert!(!restored.connected && !restored.busy);
}

#[test]
fn wide_inspection_preserves_preview_height_and_clips_before_the_sidebar() {
    let wide = StudioLayout::new(1000.0, 600.0, true, 0.0, 0.0);
    assert_eq!(wide.inspector_width, 400.0);
    assert_eq!(wide.stage_width, 600.0);
    assert_eq!(wide.inspector_height, 0.0);
    assert_eq!(wide.stage_height + wide.top + 28.0, 600.0);
    let small = StudioLayout::new(500.0, 240.0, true, 0.0, 0.0);
    assert_eq!(small.inspector_width, 0.0);
    assert!(small.stage_height > 0.0);
    assert_eq!(
        small.stage_height + small.inspector_height + small.top + 28.0,
        240.0
    );
}
