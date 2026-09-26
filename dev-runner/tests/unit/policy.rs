use super::{ReloadDecision, classify};

#[test]
fn patches_ui_literals_and_callback_amounts() {
    let old = "fn Card() { Text(\"Before\"); count.set(count.get() + 1); }";
    assert_eq!(
        classify(old, &old.replace("Before", "After!").replace("+ 1", "+ 2")),
        ReloadDecision::Patch
    );
}

#[test]
fn rejects_changes_that_can_invalidate_live_values() {
    for (old, next) in [
        ("struct State { count: i32 }", "struct State { count: i64 }"),
        (
            "fn App() { let x = [0; 4]; }",
            "fn App() { let x = [0; 5]; }",
        ),
        ("fn App() { let x = 1_i32; }", "fn App() { let x = 1_i64; }"),
        (
            "fn App() { move || a.get(); }",
            "fn App() { move || b.get(); }",
        ),
        (
            "fn App() { remember(|| 1); }",
            "fn App() {\n remember(|| 1); }",
        ),
        ("fn App() { f::<4>(); }", "fn App() { f::<5>(); }"),
        ("const N: usize = 4;", "const N: usize = 5;"),
        ("fn main() { launch(1); }", "fn main() { launch(2); }"),
        (
            "fn App() { format!(\"{a}\"); }",
            "fn App() { format!(\"{b}\"); }",
        ),
        (
            "fn App() { let x = b\"a\"; }",
            "fn App() { let x = b\"ab\"; }",
        ),
    ] {
        assert!(
            matches!(classify(old, next), ReloadDecision::Restart(_)),
            "{old} -> {next}"
        );
    }
}

#[test]
fn keeps_invalid_edits_out_of_the_running_workspace() {
    assert!(matches!(
        classify("fn App() {}", "fn App() {"),
        ReloadDecision::Invalid(_)
    ));
    assert_eq!(
        classify("fn App() {}", "fn App() {}"),
        ReloadDecision::Unchanged
    );
}
