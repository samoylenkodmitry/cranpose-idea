//! Local platform packaging controls, rendered by Cranpose.
use crate::{
    ide::rememberPalette,
    kit::{self, Look, PrimaryButton, ToolButton, icons},
};
use cranpose::{
    BasicTextField, Box as UiBox, BoxSpec, Column, ColumnSpec, LinearArrangement, Modifier, Row,
    RowSpec, Text, TextFieldState, VerticalAlignment, composable, remember, rememberHostMessages,
    rememberMutableStateOf, send_to_host,
};
use cranpose_core::CollectEvents;
use cranpose_plugin_ui::{
    controls::label_style as style,
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
    let look = Look::new(palette);
    Column(
        Modifier::empty().fill_max_width(),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(8.0)),
        move || {
            let label = PLATFORMS
                .iter()
                .find(|(key, _)| *key == platform.get())
                .map(|(_, label)| *label)
                .unwrap_or("Platform");
            let busy = state.get().busy;
            Row(
                Modifier::empty().fill_max_width(),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(4.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    ToolButton(
                        look,
                        icons::PHONE,
                        label.into(),
                        true,
                        choosing.get(),
                        !busy,
                        ("", None),
                        move |_| choosing.set(!choosing.get()),
                    );
                    UiBox(Modifier::empty().weight(1.0), BoxSpec::default(), || {});
                    for (mode, text) in [(false, "Development"), (true, "Release")] {
                        ToolButton(
                            look,
                            "",
                            text.into(),
                            false,
                            release.get() == mode,
                            !busy,
                            ("", None),
                            move |_| release.set(mode),
                        );
                    }
                },
            );
            if choosing.get() {
                for choices in PLATFORMS.chunks(2) {
                    Row(
                        Modifier::empty().fill_max_width(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(4.0)),
                        move || {
                            for &(key, label) in choices {
                                ToolButton(
                                    look,
                                    "",
                                    label.into(),
                                    false,
                                    platform.get() == key,
                                    !state.get().busy,
                                    ("", None),
                                    move |_| {
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
                    kit::style(palette.muted, 11.5, None),
                );
                BasicTextField(
                    device,
                    Modifier::empty()
                        .fill_max_width()
                        .height(32.0)
                        .background(palette.background)
                        .rounded_corners(7.0)
                        .padding(8.0),
                    style(palette, false),
                );
            }
            let run = |action: &'static str| {
                let payload = json!({"action":action,"platform":platform.get(),"manifest":manifest,"package":package,"binary":binary,"release":release.get(),"device":device.text()}).to_string();
                move || {
                    let mut next = state.get();
                    next.begin();
                    state.set(next);
                    if !send_to_host("cranpose.build", &payload) {
                        let mut next = state.get();
                        next.busy = false;
                        next.status = "Open this panel inside the IDE to build".into();
                        state.set(next);
                    }
                }
            };
            let (build_run, build_package) = (run("run"), run("build"));
            let tools = [
                (icons::CHECK, "Check tools", run("doctor"), true),
                (icons::SETTINGS, "Set up tools", run("setup"), true),
                (icons::DOCS, "Build plan", run("plan"), true),
                (icons::PHONE, "Devices", run("devices"), mobile),
            ];
            Row(
                Modifier::empty().fill_max_width(),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(4.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    let busy = state.get().busy;
                    if !busy {
                        let build_run = build_run.clone();
                        PrimaryButton(look, icons::PLAY, "Build & run".into(), move || build_run());
                    }
                    let build_package = build_package.clone();
                    ToolButton(
                        look,
                        icons::BUILD,
                        "Build package".into(),
                        false,
                        false,
                        !busy,
                        ("", None),
                        move |_| build_package(),
                    );
                    UiBox(Modifier::empty().weight(1.0), BoxSpec::default(), || {});
                    ToolButton(
                        look,
                        icons::SETTINGS,
                        "Tools".into(),
                        true,
                        tools_open.get(),
                        true,
                        ("", None),
                        move |_| tools_open.set(!tools_open.get()),
                    );
                },
            );
            if tools_open.get() {
                Row(
                    Modifier::empty().fill_max_width(),
                    RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(4.0)),
                    move || {
                        for (icon, label, action, available) in tools.clone() {
                            ToolButton(
                                look,
                                icon,
                                label.into(),
                                false,
                                false,
                                !state.get().busy && available,
                                ("", None),
                                move |_| action(),
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
