//! Cargo run configuration editor rendered entirely by Cranpose.
use crate::ide::{Palette, rememberPalette};
use cranpose::{
    BasicTextField, Button, ButtonSpec, Column, ColumnSpec, LinearArrangement, Modifier, Row,
    RowSpec, ScrollState, SpanStyle, Text, TextFieldState, TextStyle, composable, remember,
    rememberHostMessages, rememberMutableStateOf, send_to_host, text::TextUnit,
};
use cranpose_core::{CollectEvents, SideEffect};
use serde_json::{Value, json};
const FIELDS: &[(&str, &str)] = &[
    ("manifest", "Cargo manifest"),
    ("package", "Package"),
    ("target", "Target"),
    ("arguments", "Application arguments"),
    ("features", "Cargo features"),
    ("directory", "Working directory"),
    ("environment", "Environment · one NAME=value per line"),
];
fn style(palette: Palette, size: f32, muted: bool) -> TextStyle {
    TextStyle {
        span_style: SpanStyle {
            color: Some(if muted { palette.muted } else { palette.text }),
            font_size: TextUnit::Sp(size),
            ..Default::default()
        },
        ..Default::default()
    }
}
#[composable]
pub fn RunSettings() {
    let palette = rememberPalette();
    let config = rememberMutableStateOf(|| json!({"command":"run","kind":"bin"}));
    let version = rememberMutableStateOf(|| 0u64);
    let targets = rememberMutableStateOf(Vec::<Value>::new);
    CollectEvents(
        rememberHostMessages("runSettings.init"),
        (),
        move |payload: String| {
            if let Ok(value) = serde_json::from_str::<Value>(&payload) {
                config.set(value["value"].clone());
                targets.set(value["targets"].as_array().cloned().unwrap_or_default());
                version.set(version.get() + 1);
            }
        },
    );
    cranpose::key(version.get(), move || {
        let initial = config.get();
        let fields = remember(move || {
            FIELDS
                .iter()
                .map(|(key, _)| TextFieldState::new(initial[*key].as_str().unwrap_or_default()))
                .collect::<Vec<_>>()
        })
        .with(Clone::clone);
        let command =
            rememberMutableStateOf(|| config.get()["command"].as_str().unwrap_or("run").to_owned());
        let kind =
            rememberMutableStateOf(|| config.get()["kind"].as_str().unwrap_or("bin").to_owned());
        let no_defaults =
            rememberMutableStateOf(|| config.get()["noDefaultFeatures"].as_bool().unwrap_or(false));
        let scroll = remember(|| ScrollState::new(0.0)).with(|s| *s);
        let capture = {
            let fields = fields.clone();
            move || {
                let mut value = json!({"command":command.get(),"kind":kind.get(),"noDefaultFeatures":no_defaults.get()});
                for ((key, _), field) in FIELDS.iter().zip(&fields) {
                    value[*key] = json!(field.text());
                }
                value
            }
        };
        let value = capture();
        let initialized = version.get() > 0;
        SideEffect(move || {
            if initialized {
                let _ = send_to_host("runSettings.changed", &value.to_string());
            }
        });
        CollectEvents(
            rememberHostMessages("runSettings.sync"),
            (),
            move |token: String| {
                let _ = send_to_host(
                    "runSettings.synced",
                    &json!({"token":token,"value":capture()}).to_string(),
                );
            },
        );
        Column(
            Modifier::empty()
                .fill_max_size()
                .background(palette.background)
                .vertical_scroll(scroll, false)
                .padding(20.0),
            ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(14.0)),
            move || {
                Text(
                    "Cranpose · Cargo configuration",
                    Modifier::empty(),
                    style(palette, 18.0, false),
                );
                Text(
                    "Choose a workspace target or edit its Cargo settings.",
                    Modifier::empty(),
                    style(palette, 12.0, true),
                );
                for target in targets.get() {
                    let label = target["label"].as_str().unwrap_or("Target").to_owned();
                    let target = target.clone();
                    let label_copy = label.clone();
                    Button(
                        Modifier::empty().fill_max_width().height(32.0),
                        ButtonSpec::default(),
                        move || {
                            let mut value = config.get();
                            for (key, target_key) in [
                                ("manifest", "manifest"),
                                ("package", "packageName"),
                                ("target", "name"),
                                ("kind", "kind"),
                            ] {
                                value[key] = target[target_key].clone();
                            }
                            value["features"] = json!(
                                target["features"]
                                    .as_array()
                                    .map(|a| a
                                        .iter()
                                        .filter_map(Value::as_str)
                                        .collect::<Vec<_>>()
                                        .join(","))
                                    .unwrap_or_default()
                            );
                            value["directory"] = json!(
                                std::path::Path::new(
                                    target["manifest"].as_str().unwrap_or_default()
                                )
                                .parent()
                                .unwrap_or(std::path::Path::new(""))
                                .to_string_lossy()
                            );
                            config.set(value);
                            version.set(version.get() + 1);
                        },
                        move || {
                            let _ = Text(
                                label_copy.clone(),
                                Modifier::empty().padding(6.0),
                                style(palette, 12.0, false),
                            );
                        },
                    );
                }
                Row(
                    Modifier::empty().height(36.0),
                    RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                    move || {
                        for option in ["run", "check", "test"] {
                            let selected = command.get() == option;
                            Button(
                                Modifier::empty().width(90.0).height(32.0).background(
                                    if selected {
                                        palette.surface
                                    } else {
                                        palette.background
                                    },
                                ),
                                ButtonSpec::default(),
                                move || command.set(option.into()),
                                move || {
                                    let _ = Text(
                                        option,
                                        Modifier::empty().padding(6.0),
                                        style(palette, 12.0, false),
                                    );
                                },
                            );
                        }
                    },
                );
                Row(
                    Modifier::empty().height(36.0),
                    RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                    move || {
                        for option in ["bin", "example"] {
                            let selected = kind.get() == option;
                            Button(
                                Modifier::empty().width(110.0).height(32.0).background(
                                    if selected {
                                        palette.surface
                                    } else {
                                        palette.background
                                    },
                                ),
                                ButtonSpec::default(),
                                move || kind.set(option.into()),
                                move || {
                                    let _ = Text(
                                        option,
                                        Modifier::empty().padding(6.0),
                                        style(palette, 12.0, false),
                                    );
                                },
                            );
                        }
                    },
                );
                for ((key, label), field) in FIELDS.iter().zip(&fields) {
                    let field = *field;
                    Text(*label, Modifier::empty(), style(palette, 12.0, true));
                    BasicTextField(
                        field,
                        Modifier::empty()
                            .fill_max_width()
                            .height(if *key == "environment" { 92.0 } else { 36.0 })
                            .background(palette.surface)
                            .padding(8.0),
                        style(palette, 13.0, false),
                    );
                }
                let label = if no_defaults.get() {
                    "✓ Disable default features"
                } else {
                    "Disable default features"
                };
                Button(
                    Modifier::empty().height(34.0),
                    ButtonSpec::default(),
                    move || no_defaults.set(!no_defaults.get()),
                    move || {
                        let _ = Text(
                            label,
                            Modifier::empty().padding(6.0),
                            style(palette, 12.0, false),
                        );
                    },
                );
            },
        );
    });
}
