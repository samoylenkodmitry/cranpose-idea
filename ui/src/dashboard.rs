use crate::{
    ide::rememberPalette,
    kit::{Glyph, Label, Look, PrimaryButton, ToolButton, icons, style},
};
use cranpose::{
    Box as UiBox, BoxSpec, Color, Column, ColumnSpec, LinearArrangement, Modifier, Row, RowSpec,
    ScrollState, Text, VerticalAlignment, composable, remember, rememberHostMessages,
    rememberMutableStateOf, send_to_host, text::FontWeight,
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
    let look = Look::new(palette);
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
    UiBox(
        Modifier::empty()
            .fill_max_size()
            .background(palette.background),
        BoxSpec::default(),
        move || {
            Column(
                Modifier::empty()
                    .fill_max_size()
                    .vertical_scroll(scroll, false)
                    .padding(14.0),
                ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(12.0)),
                move || {
                    Row(
                        Modifier::empty().fill_max_width(),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(10.0))
                            .vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            UiBox(
                                Modifier::empty()
                                    .width(32.0)
                                    .height(32.0)
                                    .rounded_corners(9.0)
                                    .background(look.accent(if look.dark { 0.2 } else { 0.12 })),
                                BoxSpec::default().content_alignment(cranpose::Alignment::CENTER),
                                move || Glyph(icons::PLAY, 18.0, palette.accent),
                            );
                            Column(
                                Modifier::empty().weight(1.0),
                                ColumnSpec::default(),
                                move || {
                                    Text(
                                        "Cranpose Studio",
                                        Modifier::empty(),
                                        style(palette.text, 15.0, Some(FontWeight::SEMI_BOLD)),
                                    );
                                    Label(
                                        workspace.get().status,
                                        Modifier::empty(),
                                        style(palette.muted, 11.5, None),
                                    );
                                },
                            );
                            ToolButton(
                                look,
                                icons::REFRESH,
                                String::new(),
                                false,
                                false,
                                !workspace.get().busy,
                                ("", None),
                                |_| request("refresh", ""),
                            );
                            ToolButton(
                                look,
                                icons::DOCS,
                                String::new(),
                                false,
                                false,
                                true,
                                ("", None),
                                |_| request("docs", ""),
                            );
                        },
                    );
                    Section(look, icons::APP, "Application", move || {
                        let state = workspace.get();
                        if state.targets.is_empty() {
                            Text(
                                "Open a Cargo workspace with a Cranpose application, or start from the included Showcase.",
                                Modifier::empty().fill_max_width(),
                                style(palette.muted, 12.0, None),
                            );
                            if !state.busy {
                                PrimaryButton(look, icons::ADD, "Create project".into(), || {
                                    request("create", "")
                                });
                            }
                            return;
                        }
                        cranpose_plugin_ui::choice::CompactChoice(
                            palette,
                            "target",
                            state
                                .targets
                                .iter()
                                .map(|target| cranpose_plugin_ui::choice::ChoiceItem {
                                    id: target.id.clone(),
                                    label: target.name.clone(),
                                    detail: format!("{} · {}", target.package, target.kind),
                                })
                                .collect(),
                            state.selected,
                            |id| request("select", id),
                        );
                        Row(
                            Modifier::empty().fill_max_width(),
                            RowSpec::default()
                                .horizontal_arrangement(LinearArrangement::spaced_by(4.0))
                                .vertical_alignment(VerticalAlignment::CenterVertically),
                            move || {
                                let enabled = !workspace.get().busy;
                                if enabled {
                                    PrimaryButton(look, icons::PLAY, "Preview".into(), || {
                                        request("preview", "")
                                    });
                                }
                                for (icon, label, action) in [
                                    (icons::BUILD, "Run", "run"),
                                    (icons::CHECK, "Check", "check"),
                                    (icons::TEST, "Test", "test"),
                                ] {
                                    ToolButton(
                                        look,
                                        icon,
                                        label.into(),
                                        false,
                                        false,
                                        enabled,
                                        ("", None),
                                        move |_| request(action, ""),
                                    );
                                }
                                UiBox(Modifier::empty().weight(1.0), BoxSpec::default(), || {});
                                if workspace.get().busy {
                                    ToolButton(
                                        look,
                                        icons::STOP,
                                        "Stop".into(),
                                        false,
                                        false,
                                        true,
                                        ("", None),
                                        |_| request("stop", ""),
                                    );
                                }
                                ToolButton(
                                    look,
                                    icons::SETTINGS,
                                    String::new(),
                                    false,
                                    false,
                                    true,
                                    ("", None),
                                    |_| request("configure", ""),
                                );
                            },
                        );
                    });
                    Section(look, icons::BUILD, "Build for a platform", move || {
                        let workspace = workspace.get();
                        let target = workspace
                            .targets
                            .iter()
                            .find(|t| t.id == workspace.selected);
                        crate::platform_builds::PlatformBuilds(
                            target
                                .and_then(|t| {
                                    t.id.split_once("::").map(|(path, _)| path.to_owned())
                                })
                                .unwrap_or_default(),
                            target.map(|t| t.package.clone()).unwrap_or_default(),
                            target
                                .filter(|t| t.kind == "bin")
                                .map(|t| t.name.clone())
                                .unwrap_or_default(),
                        );
                    });
                    Section(look, icons::LAYERS, "Components", move || {
                        let current = editor.get();
                        if current.path.is_empty() {
                            Text(
                                "Open a Rust source file to list its composables.",
                                Modifier::empty(),
                                style(palette.muted, 12.0, None),
                            );
                        } else if current.composables.is_empty() {
                            Text(
                                format!("No composables in {}", current.name),
                                Modifier::empty(),
                                style(palette.muted, 12.0, None),
                            );
                        } else {
                            Text(
                                current.name.clone(),
                                Modifier::empty(),
                                style(palette.muted, 11.0, None),
                            );
                        }
                        for symbol in current.composables {
                            let offset = symbol.offset.to_string();
                            let name = symbol.name.clone();
                            Row(
                                Modifier::empty()
                                    .fill_max_width()
                                    .height(30.0)
                                    .rounded_corners(6.0)
                                    .padding_horizontal(6.0)
                                    .clickable(move |_| request("navigate", &offset)),
                                RowSpec::default()
                                    .horizontal_arrangement(LinearArrangement::spaced_by(8.0))
                                    .vertical_alignment(VerticalAlignment::CenterVertically),
                                move || {
                                    Glyph(icons::SOURCE, 14.0, palette.muted);
                                    Label(
                                        symbol.name.clone(),
                                        Modifier::empty().weight(1.0),
                                        style(palette.text, 12.5, None),
                                    );
                                    Text(
                                        format!(":{}", symbol.line),
                                        Modifier::empty(),
                                        style(palette.muted, 11.0, None),
                                    );
                                    let name = name.clone();
                                    ToolButton(
                                        look,
                                        icons::PLAY,
                                        String::new(),
                                        false,
                                        false,
                                        true,
                                        ("", None),
                                        move |_| request("component", &name),
                                    );
                                },
                            );
                        }
                    });
                    if !workspace.get().diagnostics.is_empty() {
                        Section(look, icons::WARNING, "Build problems", move || {
                            for (index, diagnostic) in
                                workspace.get().diagnostics.into_iter().enumerate()
                            {
                                Row(
                                    Modifier::empty()
                                        .fill_max_width()
                                        .rounded_corners(6.0)
                                        .padding(6.0)
                                        .clickable(move |_| {
                                            request("diagnostic", &index.to_string())
                                        }),
                                    RowSpec::default()
                                        .horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                                    move || {
                                        Glyph(
                                            icons::WARNING,
                                            14.0,
                                            if diagnostic.level == "warning" {
                                                look.warning
                                            } else {
                                                look.danger
                                            },
                                        );
                                        let diagnostic = diagnostic.clone();
                                        Column(
                                            Modifier::empty().weight(1.0),
                                            ColumnSpec::default().vertical_arrangement(
                                                LinearArrangement::spaced_by(2.0),
                                            ),
                                            move || {
                                                Text(
                                                    diagnostic.message.clone(),
                                                    Modifier::empty().fill_max_width(),
                                                    style(palette.text, 12.0, None),
                                                );
                                                Text(
                                                    format!(
                                                        "{}:{}",
                                                        diagnostic.file, diagnostic.line
                                                    ),
                                                    Modifier::empty(),
                                                    style(palette.accent, 11.0, None),
                                                );
                                            },
                                        );
                                    },
                                );
                            }
                        });
                    }
                },
            );
        },
    );
}

/// A titled card. Content lays out in a column with comfortable spacing.
#[composable]
fn Section(look: Look, icon: &'static str, title: &'static str, content: impl FnMut() + 'static) {
    Column(
        Modifier::empty()
            .fill_max_width()
            .rounded_corners(10.0)
            .background(Color(
                look.palette.surface.0,
                look.palette.surface.1,
                look.palette.surface.2,
                if look.dark { 0.7 } else { 0.9 },
            ))
            .padding(12.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(10.0)),
        move || {
            Row(
                Modifier::empty(),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(7.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Glyph(icon, 15.0, look.palette.muted);
                    Text(
                        title,
                        Modifier::empty(),
                        style(look.palette.text, 13.0, Some(FontWeight::SEMI_BOLD)),
                    );
                },
            );
            content();
        },
    );
}

#[cfg(test)]
#[path = "tests/dashboard_tests.rs"]
mod tests;
