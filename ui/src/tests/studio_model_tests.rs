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
fn hidden_and_paused_inspection_never_expose_old_bounds_or_pick_targets() {
    let mut model = studio();
    model.start();
    model.handle("studio.child", r#"{"session":1,"event":"connected"}"#);
    model.set_picking(true);
    let request = |model: &Studio| -> u64 {
        model.inspection_request().expect("connected")["payload"]
            .as_str()
            .expect("request string")
            .parse()
            .expect("request id")
    };
    let reply = |model: &mut Studio, id, x| {
        model.handle("studio.child", &json!({"session":1,"event":"message",
            "channel":"cranpose.inspector.v2.snapshot","payload":json!({"schema":2,
            "requestId":id,"nodes":[{"id":"label","text":"Sun","x":x,"width":40,"height":20}]}).to_string()}).to_string());
    };
    let first = request(&model);
    reply(&mut model, first, 10);
    model.select_node("label".into());
    assert_eq!(model.selected_bounds().expect("fresh bounds")["x"], 10.0);
    assert!(model.picking());
    let delayed = request(&model);

    model.set_inspecting(false);
    assert!(!model.pick && !model.picking());
    assert!(model.selected_bounds().is_none());
    assert_eq!(model.selected, "label", "keep details for reopening");
    reply(&mut model, delayed, 90);
    model.set_inspecting(true);
    assert!(model.selected_bounds().is_none());
    reply(&mut model, delayed, 90);
    assert!(
        model.selected_bounds().is_none(),
        "late reply cannot restore bounds"
    );
    let fresh = request(&model);
    // Even identical nodes must make the fresh observation visible to composition.
    reply(&mut model, fresh, 10);
    assert!(model.selected_bounds().is_some());

    model.set_live_inspection(false);
    assert!(model.selected_bounds().is_none());
    let refresh = request(&model);
    reply(&mut model, refresh, 30);
    assert!(
        model.selected_bounds().is_none(),
        "paused details are not live bounds"
    );
    model.set_live_inspection(true);
    reply(&mut model, refresh, 30);
    assert!(model.selected_bounds().is_none());
    let resumed = request(&model);
    reply(&mut model, resumed, 50);
    assert_eq!(model.selected_bounds().expect("new bounds")["x"], 50.0);

    model.set_live_inspection(false);
    model.set_picking(true);
    assert!(model.live && model.settings.inspect);
    assert!(
        !model.picking(),
        "Pick waits for a new snapshot after Resume"
    );
    let pick = request(&model);
    reply(&mut model, pick, 50);
    assert!(model.picking());
    model.handle("studio.child", r#"{"session":1,"event":"stopped"}"#);
    assert!(model.selected_bounds().is_none() && !model.picking());
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
fn wizard_target_starts_once_and_missing_rust_can_retry_without_discovery() {
    let mut studio = studio();
    let target = studio.targets[0].clone();
    let request = studio.handle(
        "studio.command",
        &json!({"action":"showTarget","target":target}).to_string(),
    );
    assert_eq!(request.len(), 1);
    assert_eq!(request[0]["options"]["target"], "desktop");
    assert!(
        studio
            .handle("cranpose.project", &json!({"targets":[target]}).to_string())
            .is_empty()
    );
    studio.handle(
        "studio.child",
        r#"{"session":1,"event":"stopped","setup":"rust","message":"Install Rust"}"#,
    );
    assert_eq!(studio.setup, "rust");
    assert!(!studio.busy && !studio.connected);
    assert_eq!(studio.session, 0);
    assert!(
        studio
            .handle("cranpose.project", &json!({"targets":[target]}).to_string())
            .is_empty(),
        "No automatic retry loop"
    );
    let retry = studio.start().expect("retry target");
    assert_eq!(retry["options"]["target"], "desktop");
    assert_eq!(retry["session"], 2);
    assert!(studio.setup.is_empty());
    studio.handle(
        "studio.child",
        r#"{"session":1,"event":"stopped","setup":"rust"}"#,
    );
    assert!(
        studio.setup.is_empty() && studio.busy,
        "Ignore old setup result"
    );
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
fn picked_launcher_view_navigates_to_original_workspace_source() {
    let mut studio = studio();
    studio.log(r#"{"cranposeDev":"sourceMap","private":"/cache/workspace/.cranpose-dev/launcher","original":"/project"}"#);
    studio.log(r#"{"cranposeDev":"workspace","private":"/cache/workspace"}"#);
    studio.snapshot = Snapshot::parse(r#"{"schema":2,"nodes":[{"id":"description","kind":"Text","text":"Closest to the Sun","sources":[{"name":"__cranpose_call:Text","manifestDir":"/cache/workspace/.cranpose-dev/launcher","file":".cranpose-dev/launcher/src/screens/detail_screen.rs","line":59}]}]}"#).expect("snapshot").into();
    studio.select_node("description".into());
    let request = studio.selected_source_request().expect("source navigation");
    assert_eq!(
        request["file"],
        json!(Path::new("/project").join("src/screens/detail_screen.rs"))
    );
    assert_eq!(request["line"], 59);
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
            .expect("snapshot")
            .into();
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

#[test]
fn viewport_changes_are_live_and_not_restored_from_checkpoints() {
    let mut studio = Studio::default();
    studio.handle("studio.viewport", r#"{"width":1024,"height":620}"#);
    assert_eq!(studio.viewport, Some((1024, 620)));
    let checkpoint = serde_json::to_value(&studio).expect("serialize Studio checkpoint");
    assert!(checkpoint.get("viewport").is_none());
    studio.handle("studio.viewport", r#"{"width":480,"height":620}"#);
    studio.handle(
        "studio.init",
        &serde_json::json!({"checkpoint":checkpoint}).to_string(),
    );
    assert_eq!(studio.viewport, Some((480, 620)));
    for invalid in [
        r#"{"width":0,"height":620}"#,
        r#"{"width":100000,"height":620}"#,
        r#"{"width":-1,"height":620}"#,
    ] {
        studio.handle("studio.viewport", invalid);
        assert_eq!(studio.viewport, Some((480, 620)));
    }
}

/// Opt-in, fixed workload; report raw timings without machine-dependent CI thresholds.
#[test]
#[ignore = "run with --ignored --nocapture to measure inspector model costs"]
fn benchmark_inspector_model() {
    use std::{hint::black_box, time::Instant};
    for count in [100, 1_000, 10_000] {
        let nodes: Vec<_> = (0..count)
            .map(|id| {
                json!({
                    "id":format!("node-{id}"), "parent":if id == 0 { None } else { Some("node-0") },
                    "kind":"Text", "text":format!("Counter item {id}"), "width":100, "height":24,
                    "sources":[{"name":"CounterRow","file":"src/counter.rs","line":42}],
                    "modifiers":[{"name":"padding","properties":[{"name":"all","value":"12"}]}]
                })
            })
            .collect();
        let mut model = studio();
        model.snapshot = Snapshot::parse(&json!({"schema":2,"nodes":nodes}).to_string())
            .expect("snapshot")
            .into();
        let start = Instant::now();
        for _ in 0..200 {
            black_box(black_box(&model).clone());
        }
        let clone_micros = start.elapsed().as_secs_f64() * 1e6 / 200.0;
        let start = Instant::now();
        for _ in 0..100 {
            black_box(model.snapshot.rows(black_box(""), &Default::default()));
        }
        println!(
            "{}",
            json!({"nodes":count,"cloneMicros":clone_micros,"unfilteredRowsMicros":start.elapsed().as_secs_f64()*1e6/100.0})
        );
    }
}

#[test]
fn model_clones_share_immutable_inspection_and_old_views_survive_new_snapshots() {
    let mut model = studio();
    model.start();
    let message = |id, text| {
        json!({"session":1,"event":"message","channel":"cranpose.inspector.v2.snapshot",
        "payload":json!({"schema":2,"requestId":id,"nodes":[{"id":"label","text":text}]}).to_string()}).to_string()
    };
    model.handle("studio.child", &message(1, "before"));
    let old = model.clone();
    assert!(std::rc::Rc::ptr_eq(&model.snapshot, &old.snapshot));
    model.handle("studio.child", &message(2, "after"));
    assert!(!std::rc::Rc::ptr_eq(&model.snapshot, &old.snapshot));
    assert_eq!(old.snapshot.nodes[0].text.as_deref(), Some("before"));
    assert_eq!(model.snapshot.nodes[0].text.as_deref(), Some("after"));
    model.handle("studio.child", &message(1, "stale"));
    assert_eq!(model.snapshot.nodes[0].text.as_deref(), Some("after"));
}

#[test]
fn selection_prefers_private_application_call_over_framework_implementation() {
    let mut model = Studio {
        root: "/app".into(),
        private_root: "/cache/workspace".into(),
        ..Studio::default()
    };
    model.snapshot = Snapshot::parse(&json!({"schema":2,"nodes":[{"id":"text","sources":[
        {"name":"App","file":"src/main.rs","line":3,"manifestDir":"/cache/workspace"},
        {"name":"__cranpose_call:Text","file":"src/main.rs","line":12,"manifestDir":"/cache/workspace"},
        {"name":"Text","file":"src/text.rs","line":90,"manifestDir":"/cargo/cranpose-ui"}
    ]}]}).to_string()).expect("snapshot").into();
    model.select_node("text".into());
    let request = model.selected_source_request().expect("source");
    assert_eq!(request["file"], "/app/src/main.rs");
    assert_eq!(request["line"], 12);
    model.selected = "missing".into();
    assert!(model.selected_source_request().is_none());
}

#[test]
fn unchanged_inspection_preserves_model_and_rejects_delayed_layouts() {
    let mut model = studio();
    model.start();
    let message = |request, text, partial| {
        json!({"session":1,"event":"message",
        "channel":"cranpose.inspector.v2.snapshot","payload":json!({"schema":2,
        "requestId":request,"captureMicros":request,"truncated":partial,
        "nodes":[{"id":"root","text":text},{"id":"child","parent":"root"}]}).to_string()})
        .to_string()
    };
    model.handle("studio.child", &message(1, "same", false));
    model.selected = "child".into();
    model.collapsed.insert("root".into());
    let old = model.clone();
    model.handle("studio.child", &message(3, "same", false));
    assert_eq!(model, old);
    assert!(std::rc::Rc::ptr_eq(&old.snapshot, &model.snapshot));
    // Emulate a state container discarding the equal replacement.
    model = old;
    model.handle("studio.child", &message(2, "stale", false));
    assert_eq!(model.snapshot.nodes[0].text.as_deref(), Some("same"));
    model.handle("studio.child", &message(4, "same", true));
    assert!(model.snapshot.truncated);
    assert_eq!(model.selected, "child");
    assert!(model.collapsed.contains("root"));
    model.handle("studio.child", r#"{"session":1,"event":"connected"}"#);
    model.handle("studio.child", &message(0, "new session", false));
    assert_eq!(model.snapshot.nodes[0].text.as_deref(), Some("new session"));
}

#[test]
fn layout_changes_prune_removed_selection_and_collapsed_nodes() {
    let mut model = studio();
    model.start();
    let message = |nodes: Value| {
        json!({"session":1,"event":"message",
        "channel":"cranpose.inspector.v2.snapshot","payload":json!({"schema":2,"nodes":nodes}).to_string()}).to_string()
    };
    model.handle(
        "studio.child",
        &message(json!([{"id":"root"},{"id":"child","parent":"root"}])),
    );
    model.selected = "child".into();
    model.collapsed.insert("root".into());
    model.handle("studio.child", &message(json!([{"id":"replacement"}])));
    assert!(model.selected.is_empty());
    assert!(model.collapsed.is_empty());
}

#[test]
fn inspector_requests_use_decimal_ids_and_advance_when_the_layout_does_not() {
    let mut model = studio();
    assert!(model.inspection_request().is_none());
    model.start();
    model.handle("studio.child", r#"{"session":1,"event":"connected"}"#);
    for id in 1..=3 {
        let request = model.inspection_request().expect("connected");
        assert_eq!(
            request["payload"]
                .as_str()
                .expect("wire payload")
                .parse::<u64>()
                .expect("decimal request ID"),
            id
        );
    }
    model.handle("studio.child", r#"{"session":1,"event":"connected"}"#);
    assert_eq!(
        model.inspection_request().expect("new connection")["payload"],
        "1"
    );
}
