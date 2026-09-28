//! Local platform packaging controls, rendered by Cranpose.
use crate::ide::rememberPalette;
use cranpose::{
    BasicTextField, Column, ColumnSpec, LinearArrangement, Modifier, Row, RowSpec, Text,
    TextFieldState, composable, remember, rememberHostMessages, rememberMutableStateOf,
    send_to_host,
};
use cranpose_core::CollectEvents;
use cranpose_plugin_ui::{
    controls::{ActionButton as Control, label_style as style},
    tasks::{TaskOutput, TaskState},
};
use serde_json::{Value, json};

const PLATFORMS: &[(&str, &str)] = &[
    ("mac", "Mac ARM"),
    ("mac-intel", "Mac Intel"),
    ("linux", "Linux x64"),
    ("linux-arm", "Linux ARM"),
    ("windows", "Windows x64"),
    ("windows-arm", "Windows ARM"),
    ("android", "Android"),
    ("ios-sim", "iOS Sim ARM"),
    ("ios-sim-intel", "iOS Sim Intel"),
    ("ios", "iOS Device"),
];

/// Platform-specific presentation; generic progress and output live in the SDK.
fn apply_build_event(state: &mut TaskState, event: &Value) {
    if !state.apply(event) {
        match event["type"].as_str().unwrap_or_default() {
            "artifact" => {
                state.result = event["artifact"]["path"]
                    .as_str()
                    .unwrap_or_default()
                    .into();
                state.status = "Package ready".into();
            }
            "doctor" => {
                state.lines.clear();
                state.status = if event["report"]["ready"] == true {
                    "Base tools ready"
                } else {
                    "Setup needed"
                }
                .into();
                if let Some(checks) = event["report"]["checks"].as_array() {
                    for check in checks {
                        state.append(&format!(
                            "{} {} · {}",
                            if check["ready"] == true { "+" } else { "!" },
                            check["name"].as_str().unwrap_or_default(),
                            check["detail"].as_str().unwrap_or_default()
                        ));
                    }
                }
                state.append(event["report"]["launch"].as_str().unwrap_or_default());
            }
            "devices" => {
                state.lines.clear();
                state.status = "Choose a device ID below".into();
                state.append(event["text"].as_str().unwrap_or_default());
            }
            "plan" => {
                state.lines.clear();
                state.status = "Local build plan".into();
                state.append(&serde_json::to_string_pretty(&event["value"]).unwrap_or_default());
            }
            _ => {}
        }
    }
}

#[composable]
pub fn PlatformBuilds(manifest: String, package: String, binary: String) {
    let palette = rememberPalette();
    let platform =
        rememberMutableStateOf(|| match (std::env::consts::OS, std::env::consts::ARCH) {
            ("macos", "aarch64") => "mac",
            ("macos", _) => "mac-intel",
            ("windows", "aarch64") => "windows-arm",
            ("windows", _) => "windows",
            ("linux", "aarch64") => "linux-arm",
            _ => "linux",
        });
    let release = rememberMutableStateOf(|| false);
    let choosing = rememberMutableStateOf(|| false);
    let tools_open = rememberMutableStateOf(|| false);
    let device = remember(|| TextFieldState::new("")).with(|s| *s);
    let state = rememberMutableStateOf(|| TaskState {
        status: "Build here. Run on your OS or device.".into(),
        ..Default::default()
    });
    CollectEvents(
        rememberHostMessages("cranpose.build"),
        (),
        move |payload: String| {
            if let Ok(event) = serde_json::from_str(&payload) {
                let mut value = state.get();
                apply_build_event(&mut value, &event);
                state.set(value);
            }
        },
    );
    Column(
        Modifier::empty().fill_max_width(),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(8.0)),
        move || {
            let label = PLATFORMS
                .iter()
                .find(|(key, _)| *key == platform.get())
                .map(|(_, label)| *label)
                .unwrap_or("Platform");
            Control(
                palette,
                &format!("{label} · Change"),
                !state.get().busy,
                true,
                move || choosing.set(!choosing.get()),
            );
            if choosing.get() {
                for choices in PLATFORMS.chunks(2) {
                    Row(
                        Modifier::empty().fill_max_width(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(6.0)),
                        move || {
                            for &(key, label) in choices {
                                Control(
                                    palette,
                                    label,
                                    !state.get().busy,
                                    platform.get() == key,
                                    move || {
                                        platform.set(key);
                                        choosing.set(false);
                                    },
                                );
                            }
                        },
                    );
                }
            }
            let mobile = platform.get().starts_with("ios") || platform.get() == "android";
            if mobile {
                Text(
                    "Device ID · from Devices",
                    Modifier::empty(),
                    style(palette, true),
                );
                BasicTextField(
                    device,
                    Modifier::empty()
                        .fill_max_width()
                        .height(34.0)
                        .background(palette.background)
                        .rounded_corners(6.0)
                        .padding(8.0),
                    style(palette, false),
                );
            }
            Control(
                palette,
                if release.get() {
                    "Release build"
                } else {
                    "Development build"
                },
                !state.get().busy,
                release.get(),
                move || release.set(!release.get()),
            );
            Control(
                palette,
                if tools_open.get() {
                    "Hide tools"
                } else {
                    "Tools & devices"
                },
                true,
                false,
                move || tools_open.set(!tools_open.get()),
            );
            for actions in [
                [("doctor", "Check tools"), ("setup", "Set up tools")],
                [("plan", "Build plan"), ("devices", "Devices")],
                [("build", "Build package"), ("run", "Build & run")],
            ] {
                if actions[0].0 != "build" && !tools_open.get() {
                    continue;
                }
                let manifest = manifest.clone();
                let package = package.clone();
                let binary = binary.clone();
                Row(
                    Modifier::empty().fill_max_width(),
                    RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(6.0)),
                    move || {
                        for (action, label) in actions {
                            let payload = json!({"action":action,"platform":platform.get(),"manifest":manifest,"package":package,"binary":binary,"release":release.get(),"device":device.text()}).to_string();
                            Control(
                                palette,
                                label,
                                !state.get().busy && (action != "devices" || mobile),
                                action == "build",
                                move || {
                                    let mut next = state.get();
                                    next.begin();
                                    state.set(next);
                                    if !send_to_host("cranpose.build", &payload) {
                                        let mut next = state.get();
                                        next.busy = false;
                                        next.status =
                                            "Open this panel inside the IDE to build".into();
                                        state.set(next);
                                    }
                                },
                            );
                        }
                    },
                );
            }
            TaskOutput(palette, state.get(), "Stop build / app", || {
                let _ = send_to_host("cranpose.build", "{\"action\":\"cancel\"}");
            });
        },
    );
}

#[cfg(test)]
#[path = "tests/platform_builds_tests.rs"]
mod tests;
