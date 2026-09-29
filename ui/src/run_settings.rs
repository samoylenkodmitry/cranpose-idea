//! Cargo run configuration editor rendered entirely by Cranpose.
use crate::{
    ide::{Palette, rememberPalette},
    kit::{self, Look, ToolButton, icons},
};
use cranpose::{
    BasicTextField, Column, ColumnSpec, LinearArrangement, Modifier, Row, RowSpec, ScrollState,
    SpanStyle, Text, TextFieldState, TextStyle, VerticalAlignment, composable, remember,
    rememberHostMessages, rememberMutableStateOf, send_to_host,
    text::{FontWeight, TextUnit},
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
    (
        "environment",
        "Environment variables, one NAME=value per line",
    ),
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
    let look = Look::new(palette);
    Column(
        Modifier::empty()
            .fill_max_size()
            .background(palette.background)
            .vertical_scroll(scroll, false)
            .padding(20.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(12.0)),
        move || {
            Text(
                "Cargo configuration",
                Modifier::empty(),
                kit::style(palette.text, 17.0, Some(FontWeight::SEMI_BOLD)),
            );
            Text(
                "Start from a workspace target, then adjust its Cargo settings.",
                Modifier::empty(),
                style(palette, 12.0, true),
            );
            let targets = targets.get();
            if !targets.is_empty() {
                Caption(palette, "Workspace targets");
            }
            for target in targets {
                let label = target["label"].as_str().unwrap_or("Target").to_owned();
                let target_fields = fields.clone();
                ToolButton(
                    look,
                    icons::APP,
                    label,
                    false,
                    false,
                    true,
                    ("", None),
                    move |_| {
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
                );
            }
            for (caption, options, state) in [
                ("Command", &["run", "check", "test"][..], command),
                ("Target kind", &["bin", "example"][..], kind),
            ] {
                Caption(palette, caption);
                Row(
                    Modifier::empty()
                        .rounded_corners(8.0)
                        .background(palette.surface)
                        .padding(2.0),
                    RowSpec::default()
                        .horizontal_arrangement(LinearArrangement::spaced_by(2.0))
                        .vertical_alignment(VerticalAlignment::CenterVertically),
                    move || {
                        for option in options {
                            ToolButton(
                                look,
                                "",
                                option_label(option).into(),
                                false,
                                state.get() == *option,
                                true,
                                ("", None),
                                move |_| state.set((*option).into()),
                            );
                        }
                    },
                );
            }
            for ((key, label), field) in FIELDS.iter().zip(&fields) {
                let field = *field;
                Caption(palette, label);
                BasicTextField(
                    field,
                    Modifier::empty()
                        .fill_max_width()
                        .height(if *key == "environment" { 92.0 } else { 34.0 })
                        .rounded_corners(7.0)
                        .background(palette.surface)
                        .padding(8.0),
                    style(palette, 13.0, false),
                );
            }
            ToolButton(
                look,
                if no_defaults.get() {
                    icons::CHECK
                } else {
                    icons::ADD
                },
                "Disable default features".into(),
                false,
                no_defaults.get(),
                true,
                ("", None),
                move |_| no_defaults.set(!no_defaults.get()),
            );
        },
    );
}

#[composable]
fn Caption(palette: Palette, text: &'static str) {
    Text(
        text,
        Modifier::empty().padding_each(0.0, 4.0, 0.0, 0.0),
        kit::style(palette.muted, 11.5, Some(FontWeight::MEDIUM)),
    );
}

/// Cargo's command and target-kind values, as button labels.
fn option_label(option: &str) -> &str {
    match option {
        "run" => "Run",
        "check" => "Check",
        "test" => "Test",
        "bin" => "Binary",
        "example" => "Example",
        other => other,
    }
}
