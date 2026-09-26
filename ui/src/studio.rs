use crate::{
    ide::{Palette, rememberPalette},
    studio_model::Studio,
};
use cranpose::{
    BasicTextField, BoxWithConstraints, BoxWithConstraintsScope, Button, ButtonSpec, Color, Column,
    ColumnSpec, LinearArrangement, Modifier, MutableState, Row, RowSpec, ScrollState, SpanStyle,
    Text, TextFieldState, TextStyle, composable, remember, rememberHostMessages,
    rememberMutableStateOf, send_to_host,
    text::{FontWeight, TextUnit},
};
use cranpose_core::{CollectEvents, SideEffect};
use serde_json::{Value, json};

fn send(value: Value) {
    let _ = send_to_host("studio.host", &value.to_string());
}
fn edit(state: MutableState<Studio>, action: impl FnOnce(&mut Studio)) {
    let mut next = state.get();
    action(&mut next);
    state.set(next);
}
fn start(state: MutableState<Studio>) {
    edit(state, |studio| {
        if let Some(request) = studio.start() {
            send(request);
        }
    });
}
fn toggle_menu(state: MutableState<Studio>, menu: &str) {
    edit(state, |studio| {
        studio.menu = if studio.menu == menu {
            String::new()
        } else {
            menu.into()
        };
    });
}

/// Preview controls, session state and layout inspection, rendered entirely by Cranpose.
#[composable]
pub fn PreviewStudio() {
    let palette = rememberPalette();
    let state = rememberMutableStateOf(Studio::default);
    for channel in [
        "studio.init",
        "cranpose.project",
        "studio.command",
        "studio.child",
    ] {
        cranpose::key(channel, move || {
            CollectEvents(rememberHostMessages(channel), (), move |payload: String| {
                edit(state, |studio| {
                    for request in studio.handle(channel, &payload) {
                        send(request);
                    }
                });
            });
        });
    }
    cranpose::LaunchedEffectAsync((), move |_| {
        Box::pin(async move {
            loop {
                cranpose::delay(std::time::Duration::from_millis(500)).await;
                let studio = state.get();
                if studio.connected && studio.live && (studio.settings.inspect || studio.pick) {
                    request_snapshot(&studio);
                }
            }
        })
    });
    BoxWithConstraints(
        Modifier::empty()
            .fill_max_size()
            .background(palette.background),
        move |scope| {
            let size = scope.constraints();
            let width = size.max_width.max(300.0);
            let height = size.max_height.max(240.0);
            let studio = state.get();
            let menu_height = menu_height(&studio);
            let inspector_height = if studio.settings.inspect {
                (height * 0.32).clamp(130.0, 300.0)
            } else {
                0.0
            };
            let problems_height = if studio.diagnostics.is_empty() {
                0.0
            } else {
                66.0
            };
            let top = 126.0 + menu_height;
            let stage_height = (height - top - inspector_height - problems_height - 28.0).max(40.0);
            let scale = if studio.settings.fit {
                ((width - 32.0) / studio.settings.width as f32)
                    .min((stage_height - 28.0) / studio.settings.height as f32)
                    .clamp(0.05, 1.0)
            } else {
                studio.settings.zoom
            };
            let frame_width = studio.settings.width as f32 * scale;
            let frame_height = studio.settings.height as f32 * scale;
            let x = if frame_width <= width {
                (width - frame_width) * 0.5
            } else {
                studio.pan_x.clamp(width - frame_width, 0.0)
            };
            let y = top
                + if frame_height <= stage_height {
                    (stage_height - frame_height) * 0.5
                } else {
                    studio.pan_y.clamp(stage_height - frame_height, 0.0)
                };
            let selected = studio.snapshot.nodes.iter().find(|node| node.id == studio.selected).map(|node| json!({"x": node.x, "y": node.y, "width": node.width, "height": node.height}));
            let layout = json!({"action": "layout", "session": studio.session, "x": x, "y": y, "width": frame_width, "height": frame_height, "viewport": {"y": top, "height": stage_height}, "logicalWidth": studio.settings.width, "logicalHeight": studio.settings.height, "scale": scale, "dark": studio.settings.dark, "pick": studio.pick, "selected": selected});
            let settings = studio.settings.clone();
            SideEffect(move || {
                send(layout.clone());
                send(json!({"action": "settings", "value": settings}));
            });
            Column(
                Modifier::empty().fill_max_size(),
                ColumnSpec::default(),
                move || {
                    Toolbar(state, palette, width);
                    Menu(state, palette, width, menu_height);
                    Column(
                        Modifier::empty()
                            .fill_max_width()
                            .height(stage_height)
                            .background(stage_color(palette))
                            .padding(16.0),
                        ColumnSpec::default(),
                        move || {
                            if !state.get().connected {
                                Text(
                                    if state.get().busy {
                                        "Building your application…"
                                    } else {
                                        "Your application, running here"
                                    },
                                    Modifier::empty().padding(8.0),
                                    style(palette.text, 15.0, true),
                                );
                                Text(
                                    "Rust rendering · native input · live state",
                                    Modifier::empty().padding(8.0),
                                    style(palette.muted, 12.0, false),
                                );
                                if !state.get().busy {
                                    Chip("Start preview".into(), palette, true, move || {
                                        start(state)
                                    });
                                }
                            }
                        },
                    );
                    if inspector_height > 0.0 {
                        Inspector(state, palette, width, inspector_height);
                    }
                    if problems_height > 0.0 {
                        Column(
                            Modifier::empty()
                                .fill_max_width()
                                .height(problems_height)
                                .padding(8.0),
                            ColumnSpec::default(),
                            move || {
                                for (index, problem) in
                                    state.get().diagnostics.iter().take(2).enumerate()
                                {
                                    let problem = problem.clone();
                                    cranpose::key(index, move || {
                                        Chip(problem.message, palette, false, move || {
                                            if let Some(file) = &problem.file {
                                                send(
                                                    json!({"action": "navigate", "file": file, "line": problem.line}),
                                                );
                                            }
                                        });
                                    });
                                }
                            },
                        );
                    }
                    Row(
                        Modifier::empty().fill_max_width().height(28.0).padding(6.0),
                        RowSpec::default().horizontal_arrangement(LinearArrangement::SpaceBetween),
                        move || {
                            let studio = state.get();
                            Text(
                                studio.status,
                                Modifier::empty(),
                                style(
                                    if studio.restart_required {
                                        Color(0.87, 0.62, 0.25, 1.0)
                                    } else {
                                        palette.muted
                                    },
                                    11.0,
                                    false,
                                ),
                            );
                            if studio.pid > 0 {
                                Text(
                                    format!("PID {}", studio.pid),
                                    Modifier::empty(),
                                    style(palette.muted, 10.0, false),
                                );
                            }
                        },
                    );
                },
            );
        },
    );
}

#[composable]
fn Toolbar(state: MutableState<Studio>, palette: Palette, width: f32) {
    Column(
        Modifier::empty()
            .fill_max_width()
            .height(126.0)
            .padding(6.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(4.0)),
        move || {
            Row(
                Modifier::empty().fill_max_width().height(34.0),
                RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(5.0)),
                move || {
                    let studio = state.get();
                    let target = studio
                        .target()
                        .map(|target| target.name.clone())
                        .unwrap_or_else(|| "Select application".into());
                    Chip(format!("{target} ▾"), palette, false, move || {
                        toggle_menu(state, "target")
                    });
                    Chip(
                        if studio.session == 0 {
                            "Run"
                        } else {
                            "Restart"
                        }
                        .into(),
                        palette,
                        true,
                        move || start(state),
                    );
                    Chip("Stop".into(), palette, false, move || {
                        let session = state.get().session;
                        send(json!({"action": "stop", "session": session}));
                        edit(state, |studio| {
                            studio.connected = false;
                            studio.busy = false;
                            studio.session = 0;
                            studio.pid = 0;
                            studio.status = "Preview stopped".into();
                        });
                    });
                    if width > 510.0 {
                        Chip("Run config".into(), palette, false, move || {
                            send(
                                json!({"action": "configure", "target": state.get().settings.target}),
                            )
                        });
                    }
                },
            );
            Row(
                Modifier::empty().fill_max_width().height(34.0),
                RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(5.0)),
                move || {
                    let studio = state.get();
                    let variant = studio
                        .previews
                        .iter()
                        .find(|preview| preview.id == studio.settings.preview)
                        .map(|preview| preview.name.clone())
                        .unwrap_or_else(|| "Application".into());
                    Chip(format!("{variant} ▾"), palette, false, move || {
                        toggle_menu(state, "preview")
                    });
                    Chip(
                        format!("{} × {} ▾", studio.settings.width, studio.settings.height),
                        palette,
                        false,
                        move || toggle_menu(state, "size"),
                    );
                    Chip(
                        if studio.settings.dark {
                            "Dark"
                        } else {
                            "Light"
                        }
                        .into(),
                        palette,
                        studio.settings.dark,
                        move || edit(state, |studio| studio.settings.dark = !studio.settings.dark),
                    );
                    Chip(
                        if studio.settings.fit {
                            "Fit ▾".into()
                        } else {
                            format!("{}% ▾", (studio.settings.zoom * 100.0) as u32)
                        },
                        palette,
                        false,
                        move || toggle_menu(state, "zoom"),
                    );
                },
            );
            Row(
                Modifier::empty().fill_max_width().height(34.0),
                RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(4.0)),
                move || {
                    let studio = state.get();
                    Chip(
                        "Reload ▾".into(),
                        palette,
                        studio.settings.hot_reload,
                        move || toggle_menu(state, "reload"),
                    );
                    Chip("Pick".into(), palette, studio.pick, move || {
                        edit(state, |studio| {
                            studio.pick = !studio.pick;
                            if studio.pick {
                                studio.settings.inspect = true;
                            }
                        });
                        request_snapshot(&state.get());
                    });
                    Chip(
                        "Inspect".into(),
                        palette,
                        studio.settings.inspect,
                        move || {
                            edit(state, |studio| {
                                studio.settings.inspect = !studio.settings.inspect
                            });
                            request_snapshot(&state.get());
                        },
                    );
                    Chip("PNG".into(), palette, false, move || {
                        send(json!({"action": "export", "session": state.get().session}))
                    });
                    if width > 420.0 {
                        Chip("Source".into(), palette, false, move || {
                            navigate_selected(&state.get())
                        });
                    }
                },
            );
        },
    );
}

fn menu_height(studio: &Studio) -> f32 {
    let count = match studio.menu.as_str() {
        "target" => studio.targets.len(),
        "preview" => studio.previews.len() + 1,
        "size" => 6,
        "reload" => 3,
        "zoom" => 5,
        _ => 0,
    };
    if count == 0 {
        0.0
    } else {
        (count.min(6) as f32 * 32.0) + 12.0
    }
}

#[composable]
fn Menu(state: MutableState<Studio>, palette: Palette, width: f32, height: f32) {
    if height == 0.0 {
        return;
    }
    let scroll = remember(|| ScrollState::new(0.0)).with(|value| *value);
    Column(
        Modifier::empty()
            .width(width)
            .height(height)
            .background(palette.surface)
            .vertical_scroll(scroll, false)
            .padding(6.0),
        ColumnSpec::default(),
        move || {
            let studio = state.get();
            match studio.menu.as_str() {
                "target" => {
                    for target in studio.targets {
                        let id = target.id();
                        let selected = id == studio.settings.target;
                        cranpose::key(id.clone(), move || {
                            Chip(
                                format!("{} / {}", target.package_name, target.name),
                                palette,
                                selected,
                                move || {
                                    edit(state, |studio| {
                                        studio.settings.target = id.clone();
                                        studio.settings.preview.clear();
                                        studio.menu.clear();
                                    })
                                },
                            );
                        });
                    }
                }
                "preview" => {
                    Chip(
                        "Application".into(),
                        palette,
                        studio.settings.preview.is_empty(),
                        move || {
                            edit(state, |studio| {
                                if let Some(request) = studio.select_preview(String::new()) {
                                    send(request);
                                }
                            })
                        },
                    );
                    for preview in studio.previews {
                        let id = preview.id;
                        let selected = id == studio.settings.preview;
                        cranpose::key(id.clone(), move || {
                            Chip(
                                format!("{}  {}", preview.group, preview.name),
                                palette,
                                selected,
                                move || {
                                    edit(state, |studio| {
                                        if let Some(request) = studio.select_preview(id.clone()) {
                                            send(request);
                                        }
                                    })
                                },
                            );
                        });
                    }
                }
                "size" => {
                    CustomSize(state, palette);
                    for (label, width, height) in [
                        ("Compact", 360, 640),
                        ("Phone", 480, 760),
                        ("Tablet", 800, 600),
                        ("Desktop", 1280, 800),
                    ] {
                        Chip(
                            format!("{label}  {width} × {height}"),
                            palette,
                            studio.settings.width == width && studio.settings.height == height,
                            move || {
                                edit(state, |studio| {
                                    studio.settings.width = width;
                                    studio.settings.height = height;
                                    studio.menu.clear();
                                })
                            },
                        );
                    }
                }
                "reload" => {
                    Chip(
                        format!(
                            "Hot code reload: {}",
                            if studio.settings.hot_reload {
                                "on"
                            } else {
                                "off"
                            }
                        ),
                        palette,
                        studio.settings.hot_reload,
                        move || {
                            edit(state, |studio| {
                                studio.settings.hot_reload = !studio.settings.hot_reload;
                                studio.menu.clear();
                                if studio.connected {
                                    studio.restart_required = true;
                                    studio.status = "Restart to change reload mode".into();
                                }
                            })
                        },
                    );
                    Chip(
                        format!(
                            "Reload on save: {}",
                            if studio.settings.auto_build {
                                "on"
                            } else {
                                "off"
                            }
                        ),
                        palette,
                        studio.settings.auto_build,
                        move || {
                            edit(state, |studio| {
                                studio.settings.auto_build = !studio.settings.auto_build;
                                studio.menu.clear();
                                if studio.connected {
                                    studio.restart_required = true;
                                    studio.status = "Restart to change file watching".into();
                                }
                            })
                        },
                    );
                    Chip(
                        format!(
                            "Live inspection: {}",
                            if studio.live { "on" } else { "paused" }
                        ),
                        palette,
                        studio.live,
                        move || {
                            edit(state, |studio| {
                                studio.live = !studio.live;
                                studio.menu.clear();
                            })
                        },
                    );
                }
                "zoom" => {
                    for (label, zoom) in [
                        ("Fit", 0.0),
                        ("50%", 0.5),
                        ("75%", 0.75),
                        ("100%", 1.0),
                        ("150%", 1.5),
                    ] {
                        Chip(label.into(), palette, false, move || {
                            edit(state, |studio| {
                                studio.settings.fit = zoom == 0.0;
                                if zoom > 0.0 {
                                    studio.settings.zoom = zoom;
                                }
                                studio.menu.clear();
                            })
                        });
                    }
                }
                _ => {}
            }
        },
    );
}

#[composable]
fn Inspector(state: MutableState<Studio>, palette: Palette, width: f32, height: f32) {
    let tree_scroll = remember(|| ScrollState::new(0.0)).with(|value| *value);
    let properties_scroll = remember(|| ScrollState::new(0.0)).with(|value| *value);
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(height)
            .background(palette.surface),
        RowSpec::default(),
        move || {
            Column(
                Modifier::empty()
                    .width(width * 0.48)
                    .height(height)
                    .vertical_scroll(tree_scroll, false)
                    .padding(8.0),
                ColumnSpec::default(),
                move || {
                    Text(
                        "LAYOUT",
                        Modifier::empty().padding(4.0),
                        style(palette.muted, 10.0, true),
                    );
                    let studio = state.get();
                    let mut depths = std::collections::HashMap::new();
                    for node in studio.snapshot.nodes {
                        let depth = node
                            .parent
                            .as_ref()
                            .and_then(|parent| depths.get(parent))
                            .copied()
                            .unwrap_or(0);
                        depths.insert(node.id.clone(), depth + 1);
                        let label = format!("{}{}", "  ".repeat(depth), node.label());
                        let id = node.id;
                        Text(
                            label,
                            Modifier::empty()
                                .fill_max_width()
                                .padding(5.0)
                                .background(if id == studio.selected {
                                    palette.background
                                } else {
                                    palette.surface
                                })
                                .clickable(move |_| {
                                    edit(state, |studio| studio.selected = id.clone())
                                }),
                            style(palette.text, 11.0, false),
                        );
                    }
                },
            );
            Column(
                Modifier::empty()
                    .width(width * 0.52)
                    .height(height)
                    .vertical_scroll(properties_scroll, false)
                    .padding(8.0),
                ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(4.0)),
                move || {
                    let studio = state.get();
                    Text(
                        "PROPERTIES",
                        Modifier::empty().padding(4.0),
                        style(palette.muted, 10.0, true),
                    );
                    if let Some(node) = studio
                        .snapshot
                        .nodes
                        .iter()
                        .find(|node| node.id == studio.selected)
                    {
                        Text(
                            node.kind.clone(),
                            Modifier::empty(),
                            style(palette.text, 12.0, true),
                        );
                        Text(
                            format!(
                                "Position  {:.1}, {:.1}\nSize  {:.1} × {:.1}",
                                node.x, node.y, node.width, node.height
                            ),
                            Modifier::empty(),
                            style(palette.muted, 11.0, false),
                        );
                        for (index, source) in node.sources.iter().enumerate() {
                            let source = source.clone();
                            let path = studio.resolve_source(&source);
                            cranpose::key(index, move || {
                                Chip(
                                    format!("{} :{} ↗", source.name, source.line),
                                    palette,
                                    false,
                                    move || {
                                        send(
                                            json!({"action": "navigate", "file": path, "line": source.line}),
                                        )
                                    },
                                );
                            });
                        }
                        for modifier in &node.modifiers {
                            Text(
                                modifier.name.clone(),
                                Modifier::empty().padding(2.0),
                                style(palette.text, 11.0, true),
                            );
                            for property in &modifier.properties {
                                Text(
                                    format!("{}  {}", property.name, property.value),
                                    Modifier::empty(),
                                    style(palette.muted, 11.0, false),
                                );
                            }
                        }
                    } else {
                        Text(
                            "Pick an element in the preview or select a layout node.",
                            Modifier::empty(),
                            style(palette.muted, 12.0, false),
                        );
                    }
                },
            );
        },
    );
}

#[composable]
fn CustomSize(state: MutableState<Studio>, palette: Palette) {
    let settings = state.get().settings;
    let width =
        remember(move || TextFieldState::new(settings.width.to_string())).with(|value| *value);
    let height =
        remember(move || TextFieldState::new(settings.height.to_string())).with(|value| *value);
    Row(
        Modifier::empty().fill_max_width().height(42.0),
        RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(6.0)),
        move || {
            BasicTextField(
                width,
                Modifier::empty()
                    .width(72.0)
                    .height(32.0)
                    .background(palette.background)
                    .padding(6.0),
                style(palette.text, 12.0, false),
            );
            Text(
                "×",
                Modifier::empty().padding(6.0),
                style(palette.muted, 12.0, false),
            );
            BasicTextField(
                height,
                Modifier::empty()
                    .width(72.0)
                    .height(32.0)
                    .background(palette.background)
                    .padding(6.0),
                style(palette.text, 12.0, false),
            );
            Chip("Apply".into(), palette, true, move || {
                if let (Ok(width), Ok(height)) =
                    (width.text().parse::<u32>(), height.text().parse::<u32>())
                {
                    edit(state, |studio| {
                        studio.settings.width = width;
                        studio.settings.height = height;
                        studio.settings.normalize();
                        studio.menu.clear();
                    });
                }
            });
        },
    );
}

fn request_snapshot(studio: &Studio) {
    if studio.connected {
        send(
            json!({"action": "message", "session": studio.session, "channel": "cranpose.inspector.v2.request", "payload": json!({"requestId": studio.snapshot.request_id + 1}).to_string()}),
        );
    }
}
fn navigate_selected(studio: &Studio) {
    if let Some(source) = studio
        .snapshot
        .nodes
        .iter()
        .find(|node| node.id == studio.selected)
        .and_then(|node| node.sources.last())
    {
        send(
            json!({"action": "navigate", "file": studio.resolve_source(source), "line": source.line}),
        );
    } else if let Some(preview) = studio
        .previews
        .iter()
        .find(|preview| preview.id == studio.settings.preview)
    {
        send(
            json!({"action": "navigate", "file": std::path::Path::new(&studio.root).join(&preview.file), "line": preview.line}),
        );
    }
}

#[expect(non_snake_case)]
fn Chip(label: String, palette: Palette, selected: bool, action: impl Fn() + 'static) {
    let color = if selected {
        palette.accent
    } else {
        palette.surface
    };
    let foreground = if selected {
        palette.on_accent
    } else {
        palette.text
    };
    let text = label.clone();
    cranpose::key(&label, move || {
        Button(
            Modifier::empty()
                .height(30.0)
                .background(color)
                .rounded_corners(5.0),
            ButtonSpec::default(),
            action,
            move || {
                Text(
                    text.clone(),
                    Modifier::empty().padding(6.0),
                    style(foreground, 11.0, selected),
                );
            },
        );
    });
}
fn style(color: Color, size: f32, bold: bool) -> TextStyle {
    TextStyle {
        span_style: SpanStyle {
            color: Some(color),
            font_size: TextUnit::Sp(size),
            font_weight: bold.then_some(FontWeight::BOLD),
            ..Default::default()
        },
        ..Default::default()
    }
}
fn stage_color(palette: Palette) -> Color {
    Color(
        palette.background.0 * 0.92,
        palette.background.1 * 0.92,
        palette.background.2 * 0.92,
        1.0,
    )
}
