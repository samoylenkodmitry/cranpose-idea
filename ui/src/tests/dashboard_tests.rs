use super::*;

#[test]
fn workspace_accepts_cargo_target_payload() {
    let state: Workspace = serde_json::from_str(r#"{"status":"Ready","busy":false,"root":"/project","selected":"id","targets":[{"id":"id","name":"demo","packageName":"demo-app","kind":"bin"}]}"#).expect("workspace");
    assert_eq!(state.targets[0].package, "demo-app");
    assert_eq!(state.selected, "id");
    assert!(!state.busy);
}

#[test]
fn editor_accepts_source_offsets_and_unicode_names() {
    let state: Editor = serde_json::from_str(r#"{"path":"/project/é.rs","name":"é.rs","composables":[{"name":"Screen","offset":42,"line":3}]}"#).expect("editor");
    assert_eq!(state.composables[0].offset, 42);
    assert_eq!(state.composables[0].line, 3);
}

#[test]
fn standalone_commands_have_no_host() {
    assert!(!command("preview", ""));
}

#[test]
fn text_style_uses_theme_and_requested_weight() {
    let text = style(Color::WHITE, 12.0, true);
    assert_eq!(text.span_style.font_weight, Some(FontWeight::BOLD));
}
