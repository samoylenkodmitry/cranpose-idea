use crate::ide::{Palette, rememberPalette};
use cranpose::{
    Button, ButtonSpec, Color, Column, ColumnSpec, LinearArrangement, Modifier, Row, RowSpec,
    ScrollState, SpanStyle, Text, TextStyle, composable, remember, rememberHostMessages,
    rememberMutableStateOf, send_to_host,
    text::{FontWeight, TextUnit},
};
use cranpose_core::CollectEvents;
use serde::Deserialize;

#[derive(Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
struct Target {
    id: String,
    name: String,
    #[serde(rename = "packageName")]
    package: String,
    kind: String,
}

#[derive(Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
struct Workspace {
    status: String,
    busy: bool,
    targets: Vec<Target>,
    selected: String,
    root: String,
}

#[derive(Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
struct Editor {
    path: String,
    name: String,
    composables: Vec<Symbol>,
}

#[derive(Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
struct Symbol {
    name: String,
    offset: usize,
    line: usize,
}

fn command(action: &str, value: &str) -> bool {
    send_to_host(
        "cranpose.action",
        &serde_json::json!({"action": action, "value": value}).to_string(),
    )
}

fn request(action: &str, value: &str) {
    let _ = command(action, value);
}

/// The Cranpose development workspace, rendered by Cranpose itself.
#[composable]
pub fn Dashboard() {
    let palette = rememberPalette();
    let scroll = remember(|| ScrollState::new(0.0)).with(|value| *value);
    let workspace = rememberMutableStateOf(|| Workspace {
        status: "Connect to IntelliJ IDEA to explore a Cranpose workspace.".into(),
        ..Workspace::default()
    });
    let editor = rememberMutableStateOf(Editor::default);
    CollectEvents(
        rememberHostMessages("cranpose.project"),
        (),
        move |payload: String| {
            if let Ok(value) = serde_json::from_str(&payload) {
                workspace.set(value);
            }
        },
    );
    CollectEvents(
        rememberHostMessages("cranpose.editor"),
        (),
        move |payload: String| {
            if let Ok(value) = serde_json::from_str(&payload) {
                editor.set(value);
            }
        },
    );
    Column(
        Modifier::empty()
            .fill_max_size()
            .background(palette.background)
            .vertical_scroll(scroll, false)
            .padding(16.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(16.0)),
        move || {
            Text(
                "CRANPOSE",
                Modifier::empty(),
                style(palette.accent, 11.0, true),
            );
            Text(
                "Build. See. Compose.",
                Modifier::empty(),
                style(palette.text, 23.0, true),
            );
            Text(
                "Your Rust UI workspace",
                Modifier::empty(),
                style(palette.muted, 13.0, false),
            );
            let state = workspace.get();
            Card(palette, "WORKSPACE", move || {
                let state = workspace.get();
                Text(
                    state.status,
                    Modifier::empty(),
                    style(palette.text, 13.0, false),
                );
                if !state.root.is_empty() {
                    Text(
                        state.root,
                        Modifier::empty(),
                        style(palette.muted, 11.0, false),
                    );
                }
                Row(
                    Modifier::empty(),
                    RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                    move || {
                        Action(palette, "Refresh", "refresh", "", !workspace.get().busy);
                        Action(palette, "Docs", "docs", "", true);
                    },
                );
            });
            if state.targets.is_empty() {
                Card(palette, "GET STARTED", move || {
                    Text(
                        "Open a Cargo workspace that depends on cranpose, then refresh. Or create a starter in an empty project.",
                        Modifier::empty(),
                        style(palette.text, 13.0, false),
                    );
                    Action(
                        palette,
                        "Create starter",
                        "create",
                        "",
                        !workspace.get().busy,
                    );
                    Text(
                        "Rust editing works with the JetBrains Rust plugin. Type cp to insert Cranpose components and state.",
                        Modifier::empty(),
                        style(palette.muted, 12.0, false),
                    );
                });
            } else {
                Card(palette, "TARGETS", move || {
                    let state = workspace.get();
                    for target in state.targets {
                        let selected = target.id == state.selected;
                        let id = target.id.clone();
                        let fill = if selected {
                            palette.accent
                        } else {
                            palette.background
                        };
                        let ink = if selected {
                            palette.on_accent
                        } else {
                            palette.text
                        };
                        Column(
                            Modifier::empty()
                                .fill_max_width()
                                .background(fill)
                                .rounded_corners(7.0)
                                .padding(10.0)
                                .clickable(move |_| request("select", &id)),
                            ColumnSpec::default(),
                            move || {
                                Text(
                                    target.name.clone(),
                                    Modifier::empty(),
                                    style(ink, 14.0, true),
                                );
                                Text(
                                    format!("{} · {}", target.package, target.kind),
                                    Modifier::empty(),
                                    style(ink, 11.0, false),
                                );
                            },
                        );
                    }
                });
                Card(palette, "DEVELOP", move || {
                    Row(
                        Modifier::empty(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                        move || {
                            let enabled = !workspace.get().busy;
                            Action(palette, "Check", "check", "", enabled);
                            Action(palette, "Run", "run", "", enabled);
                            Action(palette, "Test", "test", "", enabled);
                        },
                    );
                    Row(
                        Modifier::empty(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                        move || {
                            Action(
                                palette,
                                "Live preview",
                                "preview",
                                "",
                                !workspace.get().busy,
                            );
                            Action(palette, "Stop", "stop", "", workspace.get().busy);
                        },
                    );
                    Text(
                        "Preview runs your target in the IDE. Rebuild after edits; inspect its layout and switch themes.",
                        Modifier::empty(),
                        style(palette.muted, 12.0, false),
                    );
                });
            }
            Card(palette, "COMPOSABLES IN EDITOR", move || {
                let current = editor.get();
                if current.path.is_empty() {
                    Text(
                        "Open a Rust file to navigate its composables.",
                        Modifier::empty(),
                        style(palette.muted, 13.0, false),
                    );
                } else {
                    Text(
                        current.name,
                        Modifier::empty(),
                        style(palette.text, 13.0, true),
                    );
                    if current.composables.is_empty() {
                        Text(
                            "No #[composable] functions in this file.",
                            Modifier::empty(),
                            style(palette.muted, 12.0, false),
                        );
                    }
                    for symbol in current.composables {
                        Action(
                            palette,
                            &format!("{}  :{}", symbol.name, symbol.line),
                            "navigate",
                            &symbol.offset.to_string(),
                            true,
                        );
                    }
                }
            });
            Text(
                "Rendered with Cranpose",
                Modifier::empty(),
                style(palette.muted, 11.0, false),
            );
        },
    );
}

#[composable]
fn Card(palette: Palette, title: &'static str, content: impl FnMut() + 'static) {
    Column(
        Modifier::empty()
            .fill_max_width()
            .background(palette.surface)
            .rounded_corners(10.0)
            .padding(12.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(10.0)),
        move || {
            Text(title, Modifier::empty(), style(palette.muted, 10.0, true));
            content();
        },
    );
}

#[expect(non_snake_case)]
fn Action(palette: Palette, label: &str, action: &str, value: &str, enabled: bool) {
    let label = label.to_string();
    let action = action.to_string();
    let value = value.to_string();
    Button(
        Modifier::empty()
            .background(if enabled {
                palette.accent
            } else {
                palette.background
            })
            .rounded_corners(6.0)
            .padding(8.0),
        ButtonSpec::default(),
        move || {
            if enabled {
                command(&action, &value);
            }
        },
        move || {
            Text(
                label.clone(),
                Modifier::empty(),
                style(
                    if enabled {
                        palette.on_accent
                    } else {
                        palette.muted
                    },
                    12.0,
                    false,
                ),
            );
        },
    );
}

fn style(color: Color, size: f32, bold: bool) -> TextStyle {
    TextStyle {
        span_style: SpanStyle {
            color: Some(color),
            font_size: TextUnit::Sp(size),
            font_weight: bold.then_some(FontWeight::BOLD),
            ..SpanStyle::default()
        },
        ..TextStyle::default()
    }
}

#[cfg(test)]
#[path = "tests/dashboard_tests.rs"]
mod tests;
