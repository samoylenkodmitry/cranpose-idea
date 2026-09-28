use super::*;

#[test]
fn output_is_bounded_and_failures_release_the_controls() {
    let mut state = TaskState {
        busy: true,
        ..Default::default()
    };
    for n in 0..1000 {
        apply_build_event(
            &mut state,
            &json!({"type":"log","text":format!("{n} {}", "🦀".repeat(1000))}),
        );
    }
    assert_eq!(state.lines.len(), 80);
    assert!(state.lines.iter().all(|line| line.chars().count() <= 600));
    apply_build_event(
        &mut state,
        &json!({"type":"job_finished","error":"Missing SDK","cancelled":false}),
    );
    assert!(!state.busy);
    assert_eq!(state.status, "Missing SDK");
}

#[test]
fn package_success_and_cancellation_have_distinct_status() {
    let mut state = TaskState {
        busy: true,
        ..Default::default()
    };
    apply_build_event(
        &mut state,
        &json!({"type":"artifact","artifact":{"path":"/build/Application.app"}}),
    );
    assert!(state.busy);
    apply_build_event(
        &mut state,
        &json!({"type":"job_finished","cancelled":false,"error":null}),
    );
    assert_eq!(state.status, "Package ready");
    assert!(!state.busy);
    apply_build_event(
        &mut state,
        &json!({"type":"job_finished","cancelled":true,"error":"Cancelled: /long/package/path/application"}),
    );
    assert_eq!(state.status, "Stopped");
    state.begin();
    assert!(state.busy);
    assert!(state.result.is_empty());
}

#[test]
fn doctor_retains_actionable_missing_tool_details() {
    let mut state = TaskState::default();
    apply_build_event(
        &mut state,
        &json!({"type":"doctor","report":{"ready":false,"launch":"Requires an iOS simulator","checks":[{"name":"xcrun","ready":false,"detail":"Install Xcode"}]}}),
    );
    assert_eq!(state.status, "Setup needed");
    assert_eq!(
        state.lines,
        ["! xcrun · Install Xcode", "Requires an iOS simulator"]
    );
}
