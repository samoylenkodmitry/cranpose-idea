use crate::ide::{Palette, rememberPalette};
use cranpose::{
    Box as UiBox, BoxSpec, Button, ButtonSpec, Color, Column, ColumnSpec, GraphicsLayer,
    LinearArrangement, Modifier, Row, RowSpec, ScrollState, SpanStyle, Text, TextStyle,
    VerticalAlignment, composable, remember, rememberHostMessages, rememberMutableStateOf,
    send_to_host,
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
    diagnostics: Vec<Diagnostic>,
}

#[derive(Clone, Default, PartialEq, Deserialize)]
#[serde(default)]
struct Diagnostic {
    message: String,
    level: String,
    file: String,
    line: usize,
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

/// Project targets, source outline and build diagnostics, rendered by Cranpose.
#[composable]
pub fn Dashboard() {
    let palette = rememberPalette();
    let scroll = remember(|| ScrollState::new(0.0)).with(|value| *value);
    let workspace = rememberMutableStateOf(Workspace::default);
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
            Column(
                Modifier::empty().fill_max_width(),
                ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(6.0)),
                move || {
                    Text(
                        "Cranpose Studio",
                        Modifier::empty(),
                        style(palette.text, 20.0, false),
                    );
                    Text(
                        "Preview · Inspect · Tune",
                        Modifier::empty(),
                        style(palette.muted, 12.0, false),
                    );
                    UiBox(
                        Modifier::empty()
                            .fill_max_width()
                            .height(2.0)
                            .graphics_layer(move || GraphicsLayer {
                                render_effect: Some(cranpose_plugin_authoring_ui::accent_effect(
                                    palette.accent,
                                )),
                                ..Default::default()
                            }),
                        BoxSpec::default(),
                        || {},
                    );
                },
            );
            Row(
                Modifier::empty(),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Action(palette, "Refresh", "refresh", "", !workspace.get().busy);
                    Action(palette, "Docs", "docs", "", true);
                },
            );
            let state = workspace.get();
            Text(
                state.status.clone(),
                Modifier::empty(),
                style(palette.muted, 12.0, false),
            );
            Section(palette, "Application", move || {
                let state = workspace.get();
                if state.targets.is_empty() {
                    Text(
                        "Open a Cargo workspace with a Cranpose application.",
                        Modifier::empty(),
                        style(palette.muted, 12.0, false),
                    );
                    Action(palette, "Create project", "create", "", !state.busy);
                }
                for target in state.targets {
                    let chosen = target.id == state.selected;
                    let id = target.id.clone();
                    Row(
                        Modifier::empty()
                            .fill_max_width()
                            .background(if chosen {
                                palette.selection()
                            } else {
                                Color::TRANSPARENT
                            })
                            .rounded_corners(8.0)
                            .padding(8.0)
                            .clickable(move |_| request("select", &id)),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(8.0))
                            .vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            Text(
                                if chosen { "●" } else { "○" },
                                Modifier::empty(),
                                style(palette.accent, 12.0, true),
                            );
                            let target = target.clone();
                            Column(Modifier::empty(), ColumnSpec::default(), move || {
                                Text(
                                    target.name.clone(),
                                    Modifier::empty(),
                                    style(palette.text, 13.0, chosen),
                                );
                                Text(
                                    format!("{} · {}", target.package, target.kind),
                                    Modifier::empty(),
                                    style(palette.muted, 11.0, false),
                                );
                            });
                        },
                    );
                }
                if !workspace.get().targets.is_empty() {
                    Row(
                        Modifier::empty(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                            .vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            let enabled = !workspace.get().busy;
                            Action(palette, "Preview", "preview", "", enabled);
                            Action(palette, "Run", "run", "", enabled);
                            Action(palette, "Check", "check", "", enabled);
                        },
                    );
                    Row(
                        Modifier::empty(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                            .vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            Action(palette, "Test", "test", "", !workspace.get().busy);
                            Action(palette, "Run config", "configure", "", true);
                            if workspace.get().busy {
                                Action(palette, "Stop", "stop", "", true);
                            }
                        },
                    );
                }
            });
            Section(palette, "Components", move || {
                let current = editor.get();
                Text(
                    if current.path.is_empty() {
                        "Open a Rust source file".to_owned()
                    } else {
                        current.name
                    },
                    Modifier::empty(),
                    style(palette.muted, 11.0, false),
                );
                for symbol in current.composables {
                    Row(
                        Modifier::empty().fill_max_width(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::SpaceBetween)
                            .vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            let offset = symbol.offset.to_string();
                            Text(
                                format!("{}  :{}", symbol.name, symbol.line),
                                Modifier::empty()
                                    .padding(6.0)
                                    .clickable(move |_| request("navigate", &offset)),
                                style(palette.text, 13.0, false),
                            );
                            Action(palette, "›", "component", &symbol.name, true);
                        },
                    );
                }
            });
            if !state.diagnostics.is_empty() {
                Section(palette, "Build problems", move || {
                    for (index, diagnostic) in workspace.get().diagnostics.into_iter().enumerate() {
                        Column(
                            Modifier::empty()
                                .fill_max_width()
                                .padding(6.0)
                                .clickable(move |_| request("diagnostic", &index.to_string())),
                            ColumnSpec::default(),
                            move || {
                                Text(
                                    diagnostic.message.clone(),
                                    Modifier::empty(),
                                    style(palette.text, 12.0, false),
                                );
                                Text(
                                    format!(
                                        "{} · {}:{}",
                                        diagnostic.level, diagnostic.file, diagnostic.line
                                    ),
                                    Modifier::empty(),
                                    style(palette.muted, 11.0, false),
                                );
                            },
                        );
                    }
                });
            }
        },
    );
}

#[composable]
fn Section(palette: Palette, title: &'static str, content: impl FnMut() + 'static) {
    Column(
        Modifier::empty()
            .fill_max_width()
            .rounded_corners(10.0)
            .background(palette.surface)
            .padding(12.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(8.0)),
        move || {
            Text(
                title,
                Modifier::empty().padding(4.0),
                style(palette.text, 13.0, true),
            );
            content();
        },
    );
}

#[expect(non_snake_case)]
fn Action(palette: Palette, label: &str, action: &str, value: &str, enabled: bool) {
    let primary = action == "preview" || action == "create";
    let label = label.to_owned();
    let action = action.to_owned();
    let value = value.to_owned();
    Button(
        Modifier::empty()
            .background(if enabled && primary {
                palette.selection()
            } else {
                Color::TRANSPARENT
            })
            .rounded_corners(6.0)
            .padding(7.0),
        ButtonSpec::default(),
        move || {
            if enabled {
                request(&action, &value);
            }
        },
        move || {
            Text(
                label.clone(),
                Modifier::empty(),
                style(
                    if !enabled {
                        palette.muted
                    } else if primary {
                        palette.accent
                    } else {
                        palette.text
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
