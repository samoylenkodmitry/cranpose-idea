use crate::{
    ide::{Palette, rememberPalette},
    studio_model::Studio,
};
use cranpose::{
    BasicTextField, BoxWithConstraints, BoxWithConstraintsScope, Color, Column, ColumnSpec,
    LinearArrangement, Modifier, MutableState, Row, RowSpec, ScrollState, SpanStyle, Text,
    TextFieldState, TextOptions, TextStyle, TextWithOptions, VerticalAlignment, composable,
    remember, rememberHostMessages, rememberMutableStateOf, send_to_host,
    text::{FontWeight, TextOverflow, TextUnit},
};
use cranpose_core::{CollectEvents, SideEffect};
use serde_json::{Value, json};

#[derive(Clone, Copy, PartialEq)]
struct InspectorScrolls {
    controls: ScrollState,
    tree: ScrollState,
    details: ScrollState,
    previous_query: MutableState<String>,
}

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
    // Keep the search field when inspection moves between the side and bottom panels.
    let query = remember(|| TextFieldState::new("")).with(|value| *value);
    // Responsive branches are disposed during subcomposition. Their modifiers must
    // detach before scroll state is released; keep it with the persistent inspector.
    let previous_query = rememberMutableStateOf(String::new);
    let scrolls = remember(|| InspectorScrolls {
        controls: ScrollState::new(0.0),
        tree: ScrollState::new(0.0),
        details: ScrollState::new(0.0),
        previous_query,
    })
    .with(|value| *value);
    let last_layout = remember(|| {
        std::rc::Rc::new(std::cell::RefCell::new(
            cranpose_plugin_ux::delivery::LastValue::<(bool, Value)>::default(),
        ))
    })
    .with(Clone::clone);
    let last_checkpoint =
        remember(|| std::rc::Rc::new(std::cell::RefCell::new(String::new()))).with(Clone::clone);
    for channel in [
        "studio.init",
        "studio.viewport",
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
    // Observe the model in this composition scope so restored settings update layout immediately.
    let view = state.get();
    BoxWithConstraints(
        Modifier::empty()
            .fill_max_size()
            .background(palette.background),
        move |scope| {
            let size = scope.constraints();
            let studio = view.clone();
            // The IDE owns the embedded viewport. Observe its live dimensions so
            // split-editor changes invalidate this composition as well as measurement.
            let (width, height) = studio
                .viewport
                .map_or((size.max_width, size.max_height), |(width, height)| {
                    (width as f32, height as f32)
                });
            let width = width.max(300.0);
            let height = height.max(240.0);
            let menu_height = menu_height(&studio);
            let problems_height = if studio.diagnostics.is_empty() {
                0.0
            } else {
                66.0
            };
            let geometry = crate::studio_model::StudioLayout::new(
                width,
                height,
                studio.settings.inspect,
                menu_height,
                problems_height,
            );
            let top = geometry.top;
            let stage_height = geometry.stage_height;
            let stage_width = geometry.stage_width;
            let inspector_height = geometry.inspector_height;
            let inspector_width = geometry.inspector_width;
            let scale = if studio.settings.fit {
                ((stage_width - 32.0) / studio.settings.width as f32)
                    .min((stage_height - 28.0) / studio.settings.height as f32)
                    .clamp(0.05, 1.0)
            } else {
                studio.settings.zoom
            };
            let frame_width = studio.settings.width as f32 * scale;
            let frame_height = studio.settings.height as f32 * scale;
            let x = if frame_width <= stage_width {
                (stage_width - frame_width) * 0.5
            } else {
                studio.pan_x.clamp(stage_width - frame_width, 0.0)
            };
            let y = top
                + if frame_height <= stage_height {
                    (stage_height - frame_height) * 0.5
                } else {
                    studio.pan_y.clamp(stage_height - frame_height, 0.0)
                };
            let selected = studio.selected_bounds();
            let layout = json!({"action": "layout", "session": studio.session, "x": x, "y": y, "width": frame_width, "height": frame_height, "viewport": {"y": top, "width": stage_width, "height": stage_height}, "logicalWidth": studio.settings.width, "logicalHeight": studio.settings.height, "scale": scale, "dark": studio.settings.dark, "pick": studio.picking(), "selected": selected});
            let initialized = studio.initialized;
            let connected = studio.connected;
            let checkpoint = serde_json::to_value(&studio).unwrap_or(Value::Null);
            let checkpoint_text = checkpoint.to_string();
            let last_checkpoint = last_checkpoint.clone();
            let last_layout = last_layout.clone();
            SideEffect(move || {
                if initialized {
                    if last_layout
                        .borrow_mut()
                        .update(&(connected, layout.clone()))
                    {
                        send(layout.clone());
                    }
                    if *last_checkpoint.borrow() != checkpoint_text {
                        send(json!({"action": "checkpoint", "value": checkpoint}));
                        *last_checkpoint.borrow_mut() = checkpoint_text;
                    }
                }
            });
            Column(
                Modifier::empty().fill_max_size(),
                ColumnSpec::default(),
                move || {
                    Toolbar(state, palette, width);
                    Menu(state, palette, width, menu_height);
                    Row(
                        Modifier::empty().fill_max_width().height(stage_height),
                        RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            Column(
                                Modifier::empty()
                                    .width(stage_width)
                                    .height(stage_height)
                                    .background(stage_color(palette))
                                    .padding(16.0),
                                ColumnSpec::default(),
                                move || {
                                    if !state.get().connected {
                                        Text(
                                            if state.get().setup == "rust" {
                                                "Your project is ready"
                                            } else if state.get().busy {
                                                "Building your application…"
                                            } else {
                                                "Your application, running here"
                                            },
                                            Modifier::empty().padding(8.0),
                                            style(palette.text, 15.0, true),
                                        );
                                        Text(
                                            if state.get().setup == "rust" {
                                                "Install Rust to build and run the preview.\nOn Windows, include the C++ build tools when prompted."
                                            } else if state.get().busy {
                                                "The first build downloads and compiles dependencies.\nBuild progress appears below."
                                            } else {
                                                "Rust rendering · native input · live state"
                                            },
                                            Modifier::empty().padding(8.0),
                                            style(palette.muted, 12.0, false),
                                        );
                                        if !state.get().busy {
                                            if state.get().setup == "rust" {
                                                Chip(
                                                    "Set up Rust".into(),
                                                    palette,
                                                    true,
                                                    move || {
                                                        send(json!({"action":"setupRust"}));
                                                    },
                                                );
                                            }
                                            Chip(
                                                if state.get().setup == "rust" {
                                                    "Retry preview"
                                                } else {
                                                    "Start preview"
                                                }
                                                .into(),
                                                palette,
                                                true,
                                                move || start(state),
                                            );
                                        }
                                    }
                                },
                            );
                            if inspector_width > 0.0 {
                                Inspector(
                                    state,
                                    query,
                                    scrolls,
                                    palette,
                                    inspector_width,
                                    stage_height,
                                );
                            }
                        },
                    );
                    if inspector_height > 0.0 {
                        Inspector(state, query, scrolls, palette, width, inspector_height);
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
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::SpaceBetween)
                            .vertical_alignment(VerticalAlignment::CenterVertically),
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
                                    "● Live",
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
    let first_scroll = remember(|| ScrollState::new(0.0)).with(|s| *s);
    let second_scroll = remember(|| ScrollState::new(0.0)).with(|s| *s);
    Column(
        Modifier::empty().fill_max_width().height(84.0).padding(8.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(4.0)),
        move || {
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .height(34.0)
                    .horizontal_scroll(first_scroll, false),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(5.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || TargetControls(state, palette, width),
            );
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .height(34.0)
                    .horizontal_scroll(second_scroll, false),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(5.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    let studio = state.get();
                    let variant = studio
                        .previews
                        .iter()
                        .find(|p| p.id == studio.settings.preview)
                        .map(|p| p.name.as_str())
                        .unwrap_or("Application");
                    Chip(
                        format!("{} ▾", short_label(variant, 18)),
                        palette,
                        false,
                        move || toggle_menu(state, "preview"),
                    );
                    InspectionControls(state, palette);
                    Chip("View ▾".into(), palette, false, move || {
                        toggle_menu(state, "view")
                    });
                },
            );
        },
    );
}

#[composable]
fn TargetControls(state: MutableState<Studio>, palette: Palette, width: f32) {
    let studio = state.get();
    let target = studio
        .target()
        .map(|target| target.name.clone())
        .unwrap_or_else(|| "Select application".into());
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
    if studio.session > 0 || studio.busy {
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
    }
    let budget = ((width - 215.0) / 7.0).max(8.0) as usize;
    Chip(
        format!("{} ▾", short_label(&target, budget)),
        palette,
        false,
        move || toggle_menu(state, "target"),
    );
    Chip("More ▾".into(), palette, false, move || {
        toggle_menu(state, "more")
    });
}

#[composable]
fn ViewControls(state: MutableState<Studio>, palette: Palette) {
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
}

#[composable]
fn InspectionControls(state: MutableState<Studio>, palette: Palette) {
    let studio = state.get();
    Chip("Pick".into(), palette, studio.pick, move || {
        edit(state, |studio| {
            studio.set_picking(!studio.pick);
        });
        request_snapshot(&state.get());
    });
    Chip(
        "Inspect".into(),
        palette,
        studio.settings.inspect,
        move || {
            edit(state, |studio| {
                studio.set_inspecting(!studio.settings.inspect)
            });
            if state.get().settings.inspect {
                request_snapshot(&state.get());
            }
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
        "view" | "more" => 4,
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
                "view" => ViewControls(state, palette),
                "more" => {
                    Chip(
                        "Reload ▾".into(),
                        palette,
                        studio.settings.hot_reload,
                        move || toggle_menu(state, "reload"),
                    );
                    Chip("Run configuration".into(), palette, false, move || {
                        send(json!({"action":"configure","target":state.get().settings.target}));
                        edit(state, |s| s.menu.clear());
                    });
                    Chip("Export PNG".into(), palette, false, move || {
                        send(json!({"action":"export","session":state.get().session}));
                        edit(state, |s| s.menu.clear());
                    });
                    Chip("Reveal source".into(), palette, false, move || {
                        navigate_selected(&state.get());
                        edit(state, |s| s.menu.clear());
                    });
                }
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
fn Inspector(
    state: MutableState<Studio>,
    query: TextFieldState,
    scrolls: InspectorScrolls,
    palette: Palette,
    width: f32,
    height: f32,
) {
    let narrow = width < 560.0;
    Column(
        Modifier::empty()
            .fill_max_width()
            .height(height)
            .background(palette.surface),
        ColumnSpec::default(),
        move || {
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .height(38.0)
                    .horizontal_scroll(scrolls.controls, false)
                    .padding(4.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    let studio = state.get();
                    if narrow {
                        Chip(
                            "Layout".into(),
                            palette,
                            !studio.inspector_details,
                            move || edit(state, |s| s.inspector_details = false),
                        );
                        Chip(
                            "Details".into(),
                            palette,
                            studio.inspector_details,
                            move || edit(state, |s| s.inspector_details = true),
                        );
                    } else {
                        Text(
                            "Layout inspector",
                            Modifier::empty().padding(8.0),
                            style(palette.text, 12.0, true),
                        );
                    }
                    Chip(
                        if studio.live { "Pause" } else { "Resume" }.into(),
                        palette,
                        !studio.live,
                        move || {
                            edit(state, |s| s.set_live_inspection(!s.live));
                            if state.get().live {
                                request_snapshot(&state.get());
                            }
                        },
                    );
                    Chip("Refresh".into(), palette, false, move || {
                        request_snapshot(&state.get())
                    });
                    Chip("Expand all".into(), palette, false, move || {
                        edit(state, |s| s.collapsed.clear())
                    });
                    Chip("Collapse all".into(), palette, false, move || {
                        edit(state, |s| {
                            s.collapsed = s
                                .snapshot
                                .nodes
                                .iter()
                                .filter_map(|n| n.parent.clone())
                                .collect();
                        })
                    });
                },
            );
            Row(
                Modifier::empty().fill_max_width().height(height - 38.0),
                RowSpec::default().vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    if !narrow || !state.get().inspector_details {
                        LayoutTree(
                            state,
                            query,
                            scrolls.tree,
                            scrolls.previous_query,
                            palette,
                            if narrow { width } else { width * 0.48 },
                            height - 38.0,
                        );
                    }
                    if !narrow || state.get().inspector_details {
                        NodeDetails(
                            state,
                            scrolls.details,
                            palette,
                            if narrow { width } else { width * 0.52 },
                            height - 38.0,
                        );
                    }
                },
            );
        },
    );
}

#[composable]
fn LayoutTree(
    state: MutableState<Studio>,
    query: TextFieldState,
    scroll: ScrollState,
    previous_query: MutableState<String>,
    palette: Palette,
    width: f32,
    height: f32,
) {
    let text = query.text();
    // Return to the first match after editing a filter, preserving scroll during live snapshots.
    SideEffect(move || {
        if previous_query.get() != text {
            let studio = state.get();
            let first_match = studio
                .snapshot
                .rows(&text, &studio.collapsed)
                .iter()
                .position(|row| row.matches)
                .unwrap_or(0);
            scroll.scroll_to(first_match.saturating_sub(1) as f32 * 28.0);
            previous_query.set(text);
        }
    });
    Column(
        Modifier::empty().width(width).height(height).padding(8.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(4.0)),
        move || {
            Text(
                "Filter · name, text, source or modifier",
                Modifier::empty(),
                style(palette.muted, 10.0, false),
            );
            Row(
                Modifier::empty().fill_max_width().height(32.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(4.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    BasicTextField(
                        query,
                        Modifier::empty()
                            .width((width - 82.0).max(100.0))
                            .height(30.0)
                            .background(palette.background)
                            .rounded_corners(5.0)
                            .padding(6.0),
                        style(palette.text, 12.0, false),
                    );
                    Chip("Clear".into(), palette, false, move || {
                        query.set_text("");
                    });
                },
            );
            let studio = state.get();
            let filtering = !query.text().trim().is_empty();
            let rows = studio.snapshot.rows(&query.text(), &studio.collapsed);
            let count = if filtering {
                rows.iter().filter(|row| row.matches).count()
            } else {
                studio.snapshot.nodes.len()
            };
            Text(
                format!(
                    "{count} {}{}",
                    if filtering { "matches" } else { "nodes" },
                    if studio.snapshot.truncated {
                        " · partial snapshot"
                    } else if !studio.live {
                        " · paused"
                    } else {
                        ""
                    }
                ),
                Modifier::empty(),
                style(palette.muted, 10.0, false),
            );
            Column(
                Modifier::empty()
                    .fill_max_width()
                    .height((height - 94.0).max(25.0))
                    .vertical_scroll(scroll, false),
                ColumnSpec::default(),
                move || {
                    if rows.is_empty() {
                        Text(
                            if filtering {
                                "No matching nodes. Try text, a source file or a modifier."
                            } else if studio.connected {
                                "Waiting for a layout snapshot…"
                            } else {
                                "Start a preview to inspect its layout."
                            },
                            Modifier::empty().padding(8.0),
                            style(palette.muted, 12.0, false),
                        );
                    }
                    let window = cranpose_plugin_ux::viewport::RowWindow::new(
                        rows.len(),
                        28.0,
                        (height - 94.0).max(25.0),
                        scroll.value(),
                        2,
                    );
                    // ScrollState retains its requested offset when content shrinks.
                    // Commit the new limit so expanding again starts at the visible row.
                    let limit = (rows.len() as f32 * 28.0 - (height - 94.0).max(25.0)).max(0.0);
                    SideEffect(move || {
                        if scroll.value_non_reactive() > limit {
                            scroll.scroll_to(limit);
                        }
                    });
                    cranpose::Spacer(cranpose::Size {
                        width: 0.0,
                        height: window.before,
                    });
                    for row in rows[window.rows.clone()].iter().cloned() {
                        let node = studio.snapshot.nodes[row.index].clone();
                        let id = node.id.clone();
                        let selected = id == studio.selected;
                        let indent = (row.depth as f32 * 12.0).min((width - 130.0).max(0.0));
                        cranpose::key(node.id.clone(), move || {
                            Row(
                                Modifier::empty()
                                    .fill_max_width()
                                    .height(28.0)
                                    .background(if selected {
                                        palette.selection()
                                    } else {
                                        palette.surface
                                    })
                                    .rounded_corners(4.0),
                                RowSpec::default()
                                    .vertical_alignment(VerticalAlignment::CenterVertically),
                                move || {
                                    Text(
                                        "",
                                        Modifier::empty().width(indent).height(28.0),
                                        style(palette.muted, 11.0, false),
                                    );
                                    let toggle = id.clone();
                                    let select = id.clone();
                                    Text(
                                        if row.has_children {
                                            if row.expanded { "▾" } else { "▸" }
                                        } else {
                                            "·"
                                        },
                                        Modifier::empty().width(24.0).padding(6.0).clickable(
                                            move |_| {
                                                if row.has_children && !filtering {
                                                    edit(state, |s| {
                                                        if !s.collapsed.remove(&toggle) {
                                                            s.collapsed.insert(toggle.clone());
                                                        }
                                                    });
                                                }
                                            },
                                        ),
                                        style(
                                            if selected {
                                                palette.accent
                                            } else {
                                                palette.muted
                                            },
                                            12.0,
                                            true,
                                        ),
                                    );
                                    TextWithOptions(
                                        node.label(),
                                        Modifier::empty()
                                            .width((width - indent - 40.0).max(70.0))
                                            .height(28.0)
                                            .padding(6.0)
                                            .clickable(move |_| {
                                                edit(state, |s| s.select_node(select.clone()));
                                                navigate_selected(&state.get());
                                            }),
                                        style(
                                            if selected {
                                                palette.accent
                                            } else {
                                                palette.text
                                            },
                                            11.0,
                                            row.matches,
                                        ),
                                        TextOptions {
                                            max_lines: Some(1),
                                            soft_wrap: false,
                                            overflow: TextOverflow::Ellipsis,
                                            ..Default::default()
                                        },
                                    );
                                },
                            );
                        });
                    }
                    cranpose::Spacer(cranpose::Size {
                        width: 0.0,
                        height: window.after,
                    });
                },
            );
        },
    );
}

#[composable]
fn NodeDetails(
    state: MutableState<Studio>,
    scroll: ScrollState,
    palette: Palette,
    width: f32,
    height: f32,
) {
    Column(
        Modifier::empty()
            .width(width)
            .height(height)
            .vertical_scroll(scroll, false)
            .padding(12.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(6.0)),
        move || {
            let studio = state.get();
            if let Some(node) = studio
                .snapshot
                .nodes
                .iter()
                .find(|node| node.id == studio.selected)
            {
                Text(
                    node.kind.clone(),
                    Modifier::empty(),
                    style(palette.text, 15.0, true),
                );
                Text(
                    format!(
                        "{} × {}  ·  x {}  y {}",
                        concise(node.width),
                        concise(node.height),
                        concise(node.x),
                        concise(node.y)
                    ),
                    Modifier::empty(),
                    style(palette.muted, 11.0, false),
                );
                if let Some(text) = node.text.as_ref().filter(|text| !text.is_empty()) {
                    Text(
                        text.clone(),
                        Modifier::empty()
                            .fill_max_width()
                            .background(palette.background)
                            .rounded_corners(5.0)
                            .padding(8.0),
                        style(palette.text, 12.0, false),
                    );
                }
                if let Some(source) = studio.selected_source_request() {
                    let path = source["file"].as_str().unwrap_or_default().to_owned();
                    let line = source["line"].as_u64().unwrap_or(1);
                    let file = std::path::Path::new(&path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy();
                    Chip(format!("↗ {file}:{line}"), palette, true, move || {
                        send(json!({"action":"navigate","file":path,"line":line}));
                    });
                }
                Text(
                    "Hierarchy",
                    Modifier::empty().padding(2.0),
                    style(palette.muted, 10.0, true),
                );
                for ancestor in studio.selection_path() {
                    let id = ancestor.id.clone();
                    let label = ancestor.kind.clone();
                    cranpose::key(&ancestor.id, move || {
                        Chip(label, palette, id == state.get().selected, move || {
                            edit(state, |s| s.select_node(id.clone()));
                            navigate_selected(&state.get());
                        });
                    });
                }
                if !node.sources.is_empty() {
                    Text(
                        "Source stack",
                        Modifier::empty().padding(2.0),
                        style(palette.muted, 10.0, true),
                    );
                }
                for (index, source) in node.sources.iter().enumerate().rev() {
                    let path = studio.resolve_source(source);
                    let line = source.line;
                    let name = source
                        .name
                        .strip_prefix("__cranpose_call:")
                        .unwrap_or(&source.name);
                    let label = format!("{name} :{line} ↗");
                    cranpose::key(index, move || {
                        Chip(label, palette, false, move || {
                            send(json!({"action":"navigate","file":path,"line":line}))
                        })
                    });
                }
                if !node.modifiers.is_empty() {
                    Text(
                        "Modifiers",
                        Modifier::empty().padding(2.0),
                        style(palette.muted, 10.0, true),
                    );
                }
                for modifier in &node.modifiers {
                    Text(
                        modifier.name.clone(),
                        Modifier::empty(),
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
                    "Explore the layout",
                    Modifier::empty(),
                    style(palette.text, 14.0, true),
                );
                Text(
                    "Select a node to see its bounds, text, source and modifiers. Use Pick to select directly in the preview.",
                    Modifier::empty(),
                    style(palette.muted, 12.0, false),
                );
            }
        },
    );
}

fn concise(value: f32) -> String {
    if value.fract().abs() < 0.05 {
        format!("{value:.0}")
    } else {
        format!("{value:.1}")
    }
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
        RowSpec::default()
            .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
            .vertical_alignment(VerticalAlignment::CenterVertically),
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
    if let Some(request) = studio.inspection_request() {
        send(request);
    }
}
fn navigate_selected(studio: &Studio) {
    if let Some(request) = studio.selected_source_request() {
        send(request);
        return;
    }
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
fn Chip(label: String, _palette: Palette, selected: bool, action: impl Fn() + 'static) {
    let text = label.clone();
    cranpose::key(&label, move || {
        cranpose_plugin_authoring_ui::ActionChip(text, selected, action);
    });
}
fn short_label(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut result = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        result.push('…');
    }
    result
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
