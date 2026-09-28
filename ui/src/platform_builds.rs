//! Local platform packaging controls, rendered by Cranpose.
use crate::ide::{Palette, rememberPalette};
use cranpose::{
    BasicTextField, Button, ButtonSpec, Column, ColumnSpec, LinearArrangement, Modifier, Row,
    RowSpec, SpanStyle, Text, TextFieldState, TextStyle, composable, remember,
    rememberHostMessages, rememberMutableStateOf, send_to_host, text::TextUnit,
};
use cranpose_core::CollectEvents;
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

#[derive(Clone, Default, PartialEq)]
pub struct BuildState {
    pub busy: bool,
    pub status: String,
    pub lines: Vec<String>,
    pub artifact: String,
}
impl BuildState {
    fn begin(&mut self) {
        self.busy = true;
        self.status = "Working…".into();
        self.lines.clear();
        self.artifact.clear();
    }
    pub fn apply(&mut self, event: &Value) {
        match event["type"].as_str().unwrap_or_default() {
            "stage" => self.status = event["message"].as_str().unwrap_or_default().into(),
            "log" => self.append(event["text"].as_str().unwrap_or_default()),
            "artifact" => {
                self.artifact = event["artifact"]["path"]
                    .as_str()
                    .unwrap_or_default()
                    .into();
                self.status = "Package ready".into();
            }
            "doctor" => {
                self.lines.clear();
                self.status = if event["report"]["ready"] == true {
                    "Base tools ready"
                } else {
                    "Setup needed"
                }
                .into();
                if let Some(checks) = event["report"]["checks"].as_array() {
                    for check in checks {
                        self.append(&format!(
                            "{} {} · {}",
                            if check["ready"] == true { "+" } else { "!" },
                            check["name"].as_str().unwrap_or_default(),
                            check["detail"].as_str().unwrap_or_default()
                        ));
                    }
                }
                self.append(event["report"]["launch"].as_str().unwrap_or_default());
            }
            "devices" => {
                self.lines.clear();
                self.status = "Choose a device ID below".into();
                self.append(event["text"].as_str().unwrap_or_default());
            }
            "plan" => {
                self.lines.clear();
                self.status = "Local build plan".into();
                self.append(&serde_json::to_string_pretty(&event["value"]).unwrap_or_default());
            }
            "job_finished" => {
                self.busy = false;
                if event["cancelled"] == true {
                    self.status = "Stopped".into();
                } else if let Some(error) = event["error"].as_str() {
                    self.status = error.into();
                } else if self.status == "Working…" {
                    self.status = "Done".into();
                }
            }
            _ => {}
        }
    }
    fn append(&mut self, text: &str) {
        for line in text.lines() {
            self.lines.push(line.chars().take(600).collect());
        }
        if self.lines.len() > 80 {
            self.lines.drain(..self.lines.len() - 80);
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
    let expanded = rememberMutableStateOf(|| false);
    let choosing = rememberMutableStateOf(|| false);
    let tools_open = rememberMutableStateOf(|| false);
    let device = remember(|| TextFieldState::new("")).with(|s| *s);
    let state = rememberMutableStateOf(|| BuildState {
        status: "Build here. Run on your OS or device.".into(),
        ..Default::default()
    });
    CollectEvents(
        rememberHostMessages("cranpose.build"),
        (),
        move |payload: String| {
            if let Ok(event) = serde_json::from_str(&payload) {
                let mut value = state.get();
                value.apply(&event);
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
            if state.get().busy {
                Control(palette, "Stop build / app", true, false, || {
                    let _ = send_to_host("cranpose.build", "{\"action\":\"cancel\"}");
                });
            }
            Text(
                state.get().status,
                Modifier::empty().fill_max_width(),
                style(palette, false),
            );
            if !state.get().artifact.is_empty() {
                Text(
                    state.get().artifact,
                    Modifier::empty().fill_max_width(),
                    style(palette, true),
                );
            }
            if !state.get().lines.is_empty() {
                Control(
                    palette,
                    if expanded.get() {
                        "Hide details"
                    } else {
                        "Show details"
                    },
                    true,
                    false,
                    move || expanded.set(!expanded.get()),
                );
                if expanded.get() {
                    for line in state.get().lines {
                        Text(
                            line,
                            Modifier::empty().fill_max_width(),
                            style(palette, true),
                        );
                    }
                }
            }
        },
    );
}
#[expect(non_snake_case)]
fn Control(
    palette: Palette,
    label: &str,
    enabled: bool,
    selected: bool,
    action: impl FnMut() + 'static,
) {
    let label = label.to_owned();
    let mut action = action;
    Button(
        Modifier::empty()
            .rounded_corners(6.0)
            .background(if selected {
                palette.selection()
            } else {
                palette.background
            })
            .padding(7.0),
        ButtonSpec::default(),
        move || {
            if enabled {
                action();
            }
        },
        move || {
            Text(label.clone(), Modifier::empty(), style(palette, !enabled));
        },
    );
}
fn style(palette: Palette, muted: bool) -> TextStyle {
    TextStyle {
        span_style: SpanStyle {
            color: Some(if muted { palette.muted } else { palette.text }),
            font_size: TextUnit::Sp(12.0),
            ..Default::default()
        },
        ..Default::default()
    }
}

#[cfg(test)]
#[path = "tests/platform_builds_tests.rs"]
mod tests;
