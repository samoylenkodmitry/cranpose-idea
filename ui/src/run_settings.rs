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
    let initialized = rememberMutableStateOf(|| false);
    let targets = rememberMutableStateOf(Vec::<Value>::new);
    let fields = remember(|| {
        FIELDS
            .iter()
            .map(|_| TextFieldState::new(""))
            .collect::<Vec<_>>()
    })
    .with(Clone::clone);
    let command = rememberMutableStateOf(|| "run".to_owned());
    let kind = rememberMutableStateOf(|| "bin".to_owned());
    let no_defaults = rememberMutableStateOf(|| false);
    let incoming = fields.clone();
    CollectEvents(
        rememberHostMessages("runSettings.init"),
        (),
        move |payload: String| {
            if let Ok(value) = serde_json::from_str::<Value>(&payload) {
                let config = &value["value"];
                for ((key, _), field) in FIELDS.iter().zip(&incoming) {
                    field.set_text(config[*key].as_str().unwrap_or_default());
                }
                command.set(config["command"].as_str().unwrap_or("run").to_owned());
                kind.set(config["kind"].as_str().unwrap_or("bin").to_owned());
                no_defaults.set(config["noDefaultFeatures"].as_bool().unwrap_or(false));
                targets.set(value["targets"].as_array().cloned().unwrap_or_default());
                initialized.set(true);
            }
        },
    );
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
    let value = capture().to_string();
    let ready = initialized.get();
    let last_sent =
        remember(|| std::rc::Rc::new(std::cell::RefCell::new(String::new()))).with(Clone::clone);
    SideEffect(move || {
        if ready && *last_sent.borrow() != value {
            let _ = send_to_host("runSettings.changed", &value);
            *last_sent.borrow_mut() = value;
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
                let target_fields = fields.clone();
                Button(
                    Modifier::empty().fill_max_width().height(32.0),
                    ButtonSpec::default(),
                    move || {
                        for ((key, _), field) in FIELDS.iter().zip(&target_fields) {
                            let value = match *key {
                                "manifest" => {
                                    target["manifest"].as_str().unwrap_or_default().to_owned()
                                }
                                "package" => target["packageName"]
                                    .as_str()
                                    .unwrap_or_default()
                                    .to_owned(),
                                "target" => target["name"].as_str().unwrap_or_default().to_owned(),
                                "features" => target["features"]
                                    .as_array()
                                    .map(|a| {
                                        a.iter()
                                            .filter_map(Value::as_str)
                                            .collect::<Vec<_>>()
                                            .join(",")
                                    })
                                    .unwrap_or_default(),
                                "directory" => std::path::Path::new(
                                    target["manifest"].as_str().unwrap_or_default(),
                                )
                                .parent()
                                .unwrap_or(std::path::Path::new(""))
                                .to_string_lossy()
                                .into_owned(),
                                _ => continue,
                            };
                            field.set_text(value);
                        }
                        kind.set(target["kind"].as_str().unwrap_or("bin").to_owned());
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
                            Modifier::empty()
                                .width(90.0)
                                .height(32.0)
                                .background(if selected {
                                    palette.surface
                                } else {
                                    palette.background
                                }),
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
                            Modifier::empty()
                                .width(110.0)
                                .height(32.0)
                                .background(if selected {
                                    palette.surface
                                } else {
                                    palette.background
                                }),
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
}
