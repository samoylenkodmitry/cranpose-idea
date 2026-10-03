use crate::{
    ide::rememberPalette,
    kit::{self, Divider, Dot, Glyph, Label, Link, Look, PrimaryButton, ToolButton, icons, style},
    studio_model::{STATUS_HEIGHT, Studio, TOOLBAR_HEIGHT},
};
use cranpose::{
    BasicTextField, Box as UiBox, BoxSpec, BoxWithConstraints, BoxWithConstraintsScope, Brush,
    Canvas, Color, Column, ColumnSpec, CornerRadii, LinearArrangement, Modifier, MutableState,
    Rect, Row, RowSpec, ScrollState, Text, TextFieldState, VerticalAlignment, composable, remember,
    rememberHostMessages, rememberMutableStateOf, send_to_host, text::FontWeight,
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
fn tip(text: &str) {
    send(json!({"action":"tooltip","text":text}));
}
const TIP: kit::Tip = Some(tip);
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
/// Menus are native IDE popups so they can overlap the running application.
fn open_menu(state: MutableState<Studio>, menu: &str, anchor: Rect) {
    let items = state.get().menu_items(menu);
    if !items.is_empty() {
        send(json!({"action":"menu","menu":menu,"items":items,
            "x":anchor.x,"y":anchor.y + anchor.height + 4.0,"width":anchor.width}));
    }
}
fn stop(state: MutableState<Studio>) {
    let session = state.get().session;
    send(json!({"action": "stop", "session": session}));
    edit(state, |studio| {
        studio.connected = false;
        studio.busy = false;
        studio.session = 0;
        studio.pid = 0;
        studio.stopped = true;
        studio.status = "Preview stopped".into();
    });
}

/// Preview controls, session state and layout inspection, rendered entirely by Cranpose.
#[composable]
pub fn PreviewStudio() {
    let palette = rememberPalette();
    let look = Look::new(palette);
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
                if let Some(request) = studio.recomposition_request() {
                    send(request);
                }
                if studio.connected && studio.live && (studio.settings.inspect || studio.pick) {
                    request_snapshot(&studio);
                }
            }
        })
    });
    // Observe the model in this composition scope so restored settings update layout immediately.
    let view = state.get();
    BoxWithConstraints(
        Modifier::empty().fill_max_size().background(look.stage),
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
            let geometry = studio.layout(width, height);
            let custom_height = geometry.custom_height;
            let problems_height = geometry.problems_height;
            let top = geometry.top;
            let stage_height = geometry.stage_height;
            let stage_width = geometry.stage_width;
            let inspector_height = geometry.inspector_height;
            let inspector_width = geometry.inspector_width;
            let crate::studio_model::PreviewFrame {
                x,
                y,
                width: frame_width,
                height: frame_height,
                scale,
            } = studio.preview_frame(&geometry);
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
            let frame = Rect {
                x,
                y: y - top,
                width: frame_width,
                height: frame_height,
            };
            Column(
                Modifier::empty().fill_max_size(),
                ColumnSpec::default(),
                move || {
                    Toolbar(state, look, width);
                    if custom_height > 0.0 {
                        CustomSize(state, look);
                    }
                    Row(
                        Modifier::empty().fill_max_width().height(stage_height),
                        RowSpec::default(),
                        move || {
                            Stage(state, look, stage_width, stage_height, frame, scale);
                            if inspector_width > 0.0 {
                                Inspector(
                                    state,
                                    query,
                                    scrolls,
                                    look,
                                    inspector_width,
                                    stage_height,
                                );
                            }
                        },
                    );
                    if inspector_height > 0.0 {
                        Inspector(state, query, scrolls, look, width, inspector_height);
                    }
                    if problems_height > 0.0 {
                        Problems(state, look, problems_height);
                    }
                    StatusBar(state, look);
                },
            );
        },
    );
}

#[composable]
fn Toolbar(state: MutableState<Studio>, look: Look, width: f32) {
    let studio = state.get();
    let wide = width >= 720.0;
    let roomy = width >= 560.0;
    Column(
        Modifier::empty()
            .fill_max_width()
            .height(TOOLBAR_HEIGHT)
            .background(look.bar),
        ColumnSpec::default(),
        move || {
            let studio = studio.clone();
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .height(TOOLBAR_HEIGHT - 1.0)
                    .padding_horizontal(8.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(4.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    if studio.session == 0 && !studio.busy {
                        PrimaryButton(look, icons::PLAY, "Run".into(), move || start(state));
                    } else {
                        ToolButton(
                            look,
                            icons::STOP,
                            String::new(),
                            false,
                            false,
                            true,
                            ("Stop preview", TIP),
                            move |_| stop(state),
                        );
                        ToolButton(
                            look,
                            icons::RESTART,
                            String::new(),
                            false,
                            false,
                            true,
                            ("Rebuild and restart preview", TIP),
                            move |_| start(state),
                        );
                    }
                    Divider(look);
                    let target = studio
                        .target()
                        .map(|target| target.name.clone())
                        .unwrap_or_else(|| "Select application".into());
                    let budget = ((width - 360.0) / 8.0).clamp(8.0, 28.0) as usize;
                    ToolButton(
                        look,
                        icons::APP,
                        short_label(&target, budget),
                        true,
                        false,
                        !studio.targets.is_empty(),
                        ("Application to preview", TIP),
                        move |anchor| open_menu(state, "target", anchor),
                    );
                    if !studio.previews.is_empty() && wide {
                        let variant = studio
                            .previews
                            .iter()
                            .find(|p| p.id == studio.settings.preview)
                            .map(|p| p.name.as_str())
                            .unwrap_or("Application");
                        ToolButton(
                            look,
                            "",
                            short_label(variant, 18),
                            true,
                            false,
                            true,
                            ("Preview function", TIP),
                            move |anchor| open_menu(state, "preview", anchor),
                        );
                    }
                    UiBox(Modifier::empty().weight(1.0), BoxSpec::default(), || {});
                    let studio = state.get();
                    ToolButton(
                        look,
                        icons::PICK,
                        if roomy { "Pick".into() } else { String::new() },
                        false,
                        studio.pick,
                        true,
                        ("Pick a view; choose from a list when views overlap", TIP),
                        move |_| {
                            edit(state, |studio| studio.set_picking(!studio.pick));
                            request_snapshot(&state.get());
                        },
                    );
                    ToolButton(
                        look,
                        icons::LAYERS,
                        if roomy {
                            "Inspect".into()
                        } else {
                            String::new()
                        },
                        false,
                        studio.settings.inspect,
                        true,
                        ("Layout inspector", TIP),
                        move |_| {
                            edit(state, |studio| {
                                studio.set_inspecting(!studio.settings.inspect)
                            });
                            if state.get().settings.inspect || state.get().pick {
                                request_snapshot(&state.get());
                            }
                        },
                    );
                    Divider(look);
                    ToolButton(
                        look,
                        icons::PHONE,
                        if wide {
                            format!("{} × {}", studio.settings.width, studio.settings.height)
                        } else {
                            String::new()
                        },
                        wide,
                        false,
                        true,
                        ("Device size", TIP),
                        move |anchor| open_menu(state, "size", anchor),
                    );
                    ToolButton(
                        look,
                        if studio.settings.dark {
                            icons::MOON
                        } else {
                            icons::SUN
                        },
                        String::new(),
                        false,
                        false,
                        true,
                        ("Switch the preview between light and dark", TIP),
                        move |_| edit(state, |studio| studio.settings.dark = !studio.settings.dark),
                    );
                    ToolButton(
                        look,
                        icons::FIT,
                        if studio.settings.fit {
                            "Fit".into()
                        } else {
                            format!("{:.0}%", studio.settings.zoom * 100.0)
                        },
                        roomy,
                        false,
                        true,
                        ("Zoom", TIP),
                        move |anchor| open_menu(state, "zoom", anchor),
                    );
                    ToolButton(
                        look,
                        icons::MORE,
                        String::new(),
                        false,
                        false,
                        true,
                        ("Reload settings, run configuration and export", TIP),
                        move |anchor| open_menu(state, "more", anchor),
                    );
                },
            );
            UiBox(
                Modifier::empty()
                    .fill_max_width()
                    .height(1.0)
                    .background(look.line),
                BoxSpec::default(),
                || {},
            );
        },
    );
}

/// The canvas around the running application: a quiet dot grid, the device
/// frame's shadow and outline, its caption, and the not-running states.
#[composable]
fn Stage(
    state: MutableState<Studio>,
    look: Look,
    width: f32,
    height: f32,
    frame: Rect,
    scale: f32,
) {
    let studio = state.get();
    let connected = studio.connected;
    let busy = studio.busy;
    // Shown until the application's first frame covers the device area.
    let device = if studio.settings.dark {
        Color(0.118, 0.122, 0.133, 1.0)
    } else {
        Color(1.0, 1.0, 1.0, 1.0)
    };
    let caption = format!(
        "{} × {}  ·  {:.0}%",
        studio.settings.width,
        studio.settings.height,
        scale * 100.0
    );
    UiBox(
        Modifier::empty()
            .width(width)
            .height(height)
            .background(look.stage),
        BoxSpec::default(),
        move || {
            let dots = Color(
                look.palette.text.0,
                look.palette.text.1,
                look.palette.text.2,
                if look.dark { 0.07 } else { 0.09 },
            );
            let line = look.line;
            let shadow = if look.dark { 0.28 } else { 0.07 };
            Canvas(
                Modifier::empty().width(width).height(height),
                move |scope| {
                    let size = scope.size();
                    let step = 20.0;
                    let mut y = step * 0.5;
                    while y < size.height {
                        let mut x = step * 0.5;
                        while x < size.width {
                            scope.draw_rect_at(
                                Rect {
                                    x: x - 0.75,
                                    y: y - 0.75,
                                    width: 1.5,
                                    height: 1.5,
                                },
                                Brush::solid(dots),
                            );
                            x += step;
                        }
                        y += step;
                    }
                    if connected || busy {
                        for (spread, alpha) in [(14.0, 0.25), (8.0, 0.45), (3.0, 0.8)] {
                            scope.draw_round_rect_at(
                                Rect {
                                    x: frame.x - spread,
                                    y: frame.y - spread + spread * 0.35,
                                    width: frame.width + spread * 2.0,
                                    height: frame.height + spread * 2.0,
                                },
                                Brush::solid(Color(0.0, 0.0, 0.0, shadow * alpha)),
                                CornerRadii::uniform(8.0 + spread),
                            );
                        }
                        scope.draw_round_rect_at(
                            Rect {
                                x: frame.x - 1.0,
                                y: frame.y - 1.0,
                                width: frame.width + 2.0,
                                height: frame.height + 2.0,
                            },
                            Brush::solid(line),
                            CornerRadii::uniform(3.0),
                        );
                        scope.draw_rect_at(frame, Brush::solid(device));
                    }
                },
            );
            if busy {
                cranpose::widgets::LinearProgressIndicator(
                    Modifier::empty().fill_max_width().height(2.0),
                    look.palette.accent,
                );
            }
            if connected && frame.y >= 22.0 {
                let caption = caption.clone();
                Label(
                    caption,
                    Modifier::empty()
                        .offset(frame.x, frame.y - 20.0)
                        .width(frame.width.max(120.0)),
                    style(look.palette.muted, 11.0, None),
                );
            }
            if !connected {
                Idle(state, look, width, height);
            }
        },
    );
}

/// What the stage shows before the application is running.
#[composable]
fn Idle(state: MutableState<Studio>, look: Look, width: f32, height: f32) {
    let studio = state.get();
    let name = studio.target().map(|t| t.name.clone()).unwrap_or_default();
    let waiting = studio.pending_start && studio.targets.is_empty();
    let (icon, title, detail): (&'static str, String, String) = if studio.setup == "rust" {
        (
            icons::BUILD,
            "Rust is not installed".into(),
            "The preview builds the application with Cargo. On Windows, also install the C++ build tools.".into(),
        )
    } else if studio.busy {
        (
            icons::BUILD,
            format!("Building {name}"),
            if studio.build_step.is_empty() {
                "Starting the compiler".into()
            } else {
                format!("{}, {} crates so far", studio.build_step, studio.built)
            },
        )
    } else if waiting && studio.project_read {
        (
            icons::WARNING,
            "No application found".into(),
            if studio.project_status.is_empty() {
                "Cargo.toml has no binary target that uses Cranpose.".into()
            } else {
                studio.project_status.clone()
            },
        )
    } else if waiting {
        (
            icons::APP,
            "Reading Cargo.toml".into(),
            "The preview starts when the application is found.".into(),
        )
    } else if !studio.diagnostics.is_empty() {
        (
            icons::WARNING,
            format!("{name} did not build"),
            "The errors are listed below.".into(),
        )
    } else {
        (
            icons::PLAY,
            "Preview stopped".into(),
            if studio.status.is_empty() || studio.status == "Preview stopped" {
                String::new()
            } else {
                studio.status.clone()
            },
        )
    };
    let first_build = studio.busy && studio.built > 20;
    let panel = (width - 48.0).clamp(220.0, 380.0);
    Column(
        Modifier::empty()
            .offset((width - panel) * 0.5, (height * 0.5 - 90.0).max(24.0))
            .width(panel),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(10.0)),
        move || {
            UiBox(
                Modifier::empty()
                    .width(40.0)
                    .height(40.0)
                    .rounded_corners(11.0)
                    .background(look.accent(if look.dark { 0.18 } else { 0.12 })),
                BoxSpec::default().content_alignment(cranpose::Alignment::CENTER),
                move || Glyph(icon, 20.0, look.palette.accent),
            );
            Text(
                title.clone(),
                Modifier::empty(),
                style(look.palette.text, 16.0, Some(FontWeight::SEMI_BOLD)),
            );
            if !detail.is_empty() {
                Text(
                    detail.clone(),
                    Modifier::empty().fill_max_width(),
                    style(look.palette.muted, 12.0, None),
                );
            }
            if first_build {
                Text(
                    "The first build compiles every dependency and can take a few minutes.",
                    Modifier::empty().fill_max_width(),
                    style(look.palette.muted, 12.0, None),
                );
            }
            let studio = state.get();
            if !studio.busy && !waiting {
                Row(
                    Modifier::empty().padding_each(0.0, 6.0, 0.0, 0.0),
                    RowSpec::default()
                        .horizontal_arrangement(LinearArrangement::spaced_by(8.0))
                        .vertical_alignment(VerticalAlignment::CenterVertically),
                    move || {
                        if state.get().setup == "rust" {
                            PrimaryButton(look, icons::BUILD, "Install Rust".into(), move || {
                                send(json!({"action":"setupRust"}));
                            });
                            ToolButton(
                                look,
                                icons::RESTART,
                                "Retry".into(),
                                false,
                                false,
                                true,
                                ("", None),
                                move |_| start(state),
                            );
                        } else {
                            PrimaryButton(look, icons::PLAY, "Run".into(), move || start(state));
                        }
                    },
                );
            }
        },
    );
}

#[composable]
fn StatusBar(state: MutableState<Studio>, look: Look) {
    let studio = state.get();
    let (color, active) = if !studio.diagnostics.is_empty() {
        (look.danger, false)
    } else if studio.restart_required {
        (look.warning, false)
    } else if studio.busy {
        (look.palette.accent, true)
    } else if studio.connected {
        (look.success, studio.pid > 0)
    } else {
        (look.palette.muted, false)
    };
    Column(
        Modifier::empty()
            .fill_max_width()
            .height(STATUS_HEIGHT)
            .background(look.bar),
        ColumnSpec::default(),
        move || {
            UiBox(
                Modifier::empty()
                    .fill_max_width()
                    .height(1.0)
                    .background(look.line),
                BoxSpec::default(),
                || {},
            );
            let studio = state.get();
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .height(STATUS_HEIGHT - 1.0)
                    .padding_horizontal(10.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Dot(color, active);
                    Label(
                        studio.status.clone(),
                        Modifier::empty().weight(1.0),
                        style(
                            if studio.restart_required {
                                look.warning
                            } else {
                                look.palette.muted
                            },
                            11.0,
                            None,
                        ),
                    );
                    if !studio.diagnostics.is_empty() {
                        Glyph(icons::WARNING, 12.0, look.danger);
                        Text(
                            format!(
                                "{} problem{}",
                                studio.diagnostics.len(),
                                if studio.diagnostics.len() == 1 {
                                    ""
                                } else {
                                    "s"
                                }
                            ),
                            Modifier::empty(),
                            style(look.danger, 11.0, Some(FontWeight::MEDIUM)),
                        );
                    }
                    if studio.pid > 0 && studio.settings.hot_reload {
                        Glyph(icons::BOLT, 12.0, look.success);
                        Text(
                            "Hot reload",
                            Modifier::empty(),
                            style(look.palette.muted, 11.0, None),
                        );
                    }
                },
            );
        },
    );
}

/// Build errors, newest last, each opening its source.
#[composable]
fn Problems(state: MutableState<Studio>, look: Look, height: f32) {
    let scroll = remember(|| ScrollState::new(0.0)).with(|value| *value);
    Column(
        Modifier::empty()
            .fill_max_width()
            .height(height)
            .background(look.bar),
        ColumnSpec::default(),
        move || {
            UiBox(
                Modifier::empty()
                    .fill_max_width()
                    .height(1.0)
                    .background(look.line),
                BoxSpec::default(),
                || {},
            );
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .height(28.0)
                    .padding_horizontal(10.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Glyph(icons::WARNING, 13.0, look.danger);
                    Text(
                        "Build failed. The previous preview is still running.",
                        Modifier::empty(),
                        style(look.palette.text, 11.5, Some(FontWeight::MEDIUM)),
                    );
                },
            );
            Column(
                Modifier::empty()
                    .fill_max_width()
                    .height((height - 29.0).max(0.0))
                    .vertical_scroll(scroll, false),
                ColumnSpec::default(),
                move || {
                    for (index, problem) in state.get().diagnostics.iter().enumerate() {
                        let problem = problem.clone();
                        cranpose::key(index, move || {
                            let location = problem
                                .file
                                .as_ref()
                                .and_then(|file| file.file_name())
                                .map(|name| format!("{}:{}", name.to_string_lossy(), problem.line))
                                .unwrap_or_default();
                            let target = problem.file.clone();
                            let line = problem.line;
                            Row(
                                Modifier::empty()
                                    .fill_max_width()
                                    .height(26.0)
                                    .padding_horizontal(10.0)
                                    .clickable(move |_| {
                                        if let Some(file) = &target {
                                            send(json!({"action": "navigate", "file": file, "line": line}));
                                        }
                                    }),
                                RowSpec::default()
                                    .horizontal_arrangement(LinearArrangement::spaced_by(8.0))
                                    .vertical_alignment(VerticalAlignment::CenterVertically),
                                move || {
                                    Label(
                                        problem.message.clone(),
                                        Modifier::empty().weight(1.0),
                                        style(look.palette.text, 11.5, None),
                                    );
                                    Text(
                                        location.clone(),
                                        Modifier::empty(),
                                        style(look.palette.accent, 11.0, None),
                                    );
                                },
                            );
                        });
                    }
                },
            );
        },
    );
}

#[composable]
fn Inspector(
    state: MutableState<Studio>,
    query: TextFieldState,
    scrolls: InspectorScrolls,
    look: Look,
    width: f32,
    height: f32,
) {
    let narrow = width < 560.0;
    Row(
        Modifier::empty()
            .width(width)
            .height(height)
            .background(look.bar),
        RowSpec::default(),
        move || {
            UiBox(
                Modifier::empty()
                    .width(1.0)
                    .fill_max_height()
                    .background(look.line),
                BoxSpec::default(),
                || {},
            );
            Column(
                Modifier::empty().width(width - 1.0).height(height),
                ColumnSpec::default(),
                move || {
                    Row(
                        Modifier::empty()
                            .fill_max_width()
                            .height(38.0)
                            .horizontal_scroll(scrolls.controls, false)
                            .padding_horizontal(6.0),
                        RowSpec::default()
                            .horizontal_arrangement(LinearArrangement::spaced_by(4.0))
                            .vertical_alignment(VerticalAlignment::CenterVertically),
                        move || {
                            let studio = state.get();
                            if narrow {
                                ToolButton(
                                    look,
                                    "",
                                    "Layout".into(),
                                    false,
                                    !studio.inspector_details,
                                    true,
                                    ("", None),
                                    move |_| edit(state, |s| s.inspector_details = false),
                                );
                                ToolButton(
                                    look,
                                    "",
                                    "Details".into(),
                                    false,
                                    studio.inspector_details,
                                    true,
                                    ("", None),
                                    move |_| edit(state, |s| s.inspector_details = true),
                                );
                            } else {
                                Text(
                                    "Layout",
                                    Modifier::empty().padding_horizontal(6.0),
                                    style(look.palette.text, 12.5, Some(FontWeight::SEMI_BOLD)),
                                );
                            }
                            UiBox(Modifier::empty().weight(1.0), BoxSpec::default(), || {});
                            ToolButton(
                                look,
                                if studio.live {
                                    icons::PAUSE
                                } else {
                                    icons::PLAY
                                },
                                if studio.live { "Pause" } else { "Resume" }.into(),
                                false,
                                !studio.live,
                                true,
                                ("Pause or resume live layout updates", TIP),
                                move |_| {
                                    edit(state, |s| s.set_live_inspection(!s.live));
                                    if state.get().live {
                                        request_snapshot(&state.get());
                                    }
                                },
                            );
                            ToolButton(
                                look,
                                icons::REFRESH,
                                String::new(),
                                false,
                                false,
                                true,
                                ("Refresh layout", TIP),
                                move |_| request_snapshot(&state.get()),
                            );
                        },
                    );
                    Row(
                        Modifier::empty().fill_max_width().height(height - 38.0),
                        RowSpec::default(),
                        move || {
                            if !narrow || !state.get().inspector_details {
                                LayoutTree(
                                    state,
                                    query,
                                    scrolls.tree,
                                    scrolls.previous_query,
                                    look,
                                    if narrow {
                                        width - 1.0
                                    } else {
                                        (width - 1.0) * 0.5
                                    },
                                    height - 38.0,
                                );
                            }
                            if !narrow {
                                UiBox(
                                    Modifier::empty()
                                        .width(1.0)
                                        .fill_max_height()
                                        .background(look.line),
                                    BoxSpec::default(),
                                    || {},
                                );
                            }
                            if !narrow || state.get().inspector_details {
                                NodeDetails(
                                    state,
                                    scrolls.details,
                                    look,
                                    if narrow {
                                        width - 1.0
                                    } else {
                                        (width - 1.0) * 0.5 - 1.0
                                    },
                                    height - 38.0,
                                );
                            }
                        },
                    );
                },
            );
        },
    );
}

const ROW: f32 = 26.0;
/// A small centered circle marking a leaf view.
const LEAF: &str = "M12 10.25a1.75 1.75 0 1 0 0 3.5 1.75 1.75 0 0 0 0-3.5z";

#[composable]
fn LayoutTree(
    state: MutableState<Studio>,
    query: TextFieldState,
    scroll: ScrollState,
    previous_query: MutableState<String>,
    look: Look,
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
            scroll.scroll_to(first_match.saturating_sub(1) as f32 * ROW);
            previous_query.set(text);
        }
    });
    Column(
        Modifier::empty()
            .width(width)
            .height(height)
            .padding_each(8.0, 0.0, 8.0, 8.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(6.0)),
        move || {
            Row(
                Modifier::empty()
                    .fill_max_width()
                    .height(30.0)
                    .rounded_corners(7.0)
                    .background(look.stage)
                    .padding_horizontal(8.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Glyph(icons::SEARCH, 14.0, look.palette.muted);
                    UiBox(
                        Modifier::empty().weight(1.0).height(30.0),
                        BoxSpec::default(),
                        move || {
                            if query.text().is_empty() {
                                Label(
                                    "Filter by name, text, source or modifier".into(),
                                    Modifier::empty()
                                        .fill_max_width()
                                        .padding_each(0.0, 8.0, 0.0, 0.0),
                                    style(look.palette.muted, 12.0, None),
                                );
                            }
                            BasicTextField(
                                query,
                                Modifier::empty()
                                    .fill_max_size()
                                    .padding_each(0.0, 7.0, 0.0, 0.0),
                                style(look.palette.text, 12.0, None),
                            );
                        },
                    );
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
            Row(
                Modifier::empty().fill_max_width().height(18.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(2.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    let studio = state.get();
                    Label(
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
                        Modifier::empty().weight(1.0).padding_horizontal(2.0),
                        style(look.palette.muted, 10.5, None),
                    );
                    if filtering {
                        Link(look, "Clear", move || {
                            query.set_text("");
                        });
                    } else {
                        Link(look, "Expand all", move || {
                            edit(state, |s| s.collapsed.clear())
                        });
                        Link(look, "Collapse all", move || {
                            edit(state, |s| {
                                s.collapsed = s
                                    .snapshot
                                    .nodes
                                    .iter()
                                    .filter_map(|n| n.parent.clone())
                                    .collect();
                            })
                        });
                    }
                },
            );
            // Filter (30) and summary (18) rows, two 6 px gaps and 8 px bottom padding.
            let list_height = (height - 68.0).max(ROW);
            Column(
                Modifier::empty()
                    .fill_max_width()
                    .height(list_height)
                    .vertical_scroll(scroll, false),
                ColumnSpec::default(),
                move || {
                    if rows.is_empty() {
                        Text(
                            if filtering {
                                "No matching views. Try text, a source file or a modifier."
                            } else if studio.connected {
                                "Waiting for a layout snapshot…"
                            } else {
                                "Run the preview to see its layout."
                            },
                            Modifier::empty().padding(8.0),
                            style(look.palette.muted, 12.0, None),
                        );
                    }
                    let window = cranpose_plugin_ux::viewport::RowWindow::new(
                        rows.len(),
                        ROW,
                        list_height,
                        scroll.value(),
                        2,
                    );
                    // ScrollState retains its requested offset when content shrinks.
                    // Commit the new limit so expanding again starts at the visible row.
                    let limit = (rows.len() as f32 * ROW - list_height).max(0.0);
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
                            let toggle = id.clone();
                            let select = id.clone();
                            Row(
                                Modifier::empty()
                                    .fill_max_width()
                                    .height(ROW)
                                    .rounded_corners(6.0)
                                    .background(if selected {
                                        look.accent(if look.dark { 0.2 } else { 0.12 })
                                    } else {
                                        Color::TRANSPARENT
                                    })
                                    .clickable(move |_| {
                                        edit(state, |s| s.select_node(select.clone()));
                                        navigate_selected(&state.get());
                                    }),
                                RowSpec::default()
                                    .vertical_alignment(VerticalAlignment::CenterVertically),
                                move || {
                                    let toggle = toggle.clone();
                                    cranpose::widgets::IconWith(
                                        Modifier::empty()
                                            .padding_each(indent + 4.0, 2.0, 2.0, 2.0)
                                            .clickable(move |_| {
                                                if row.has_children && !filtering {
                                                    edit(state, |s| {
                                                        if !s.collapsed.remove(&toggle) {
                                                            s.collapsed.insert(toggle.clone());
                                                        }
                                                    });
                                                }
                                            }),
                                        if !row.has_children {
                                            LEAF
                                        } else if row.expanded {
                                            icons::CHEVRON
                                        } else {
                                            icons::CHEVRON_RIGHT
                                        },
                                        cranpose::widgets::IconSpec::sized(14.0).with_tint(Color(
                                            look.palette.muted.0,
                                            look.palette.muted.1,
                                            look.palette.muted.2,
                                            if row.has_children { 1.0 } else { 0.6 },
                                        )),
                                        None,
                                    );
                                    Label(
                                        node.label(),
                                        Modifier::empty().weight(1.0).padding_horizontal(4.0),
                                        style(
                                            if selected {
                                                look.palette.accent
                                            } else {
                                                look.palette.text
                                            },
                                            12.0,
                                            row.matches.then_some(FontWeight::SEMI_BOLD),
                                        ),
                                    );
                                    if let Some(count) = node.recompositions() {
                                        Glyph(icons::REFRESH, 12.0, look.palette.muted);
                                        Label(
                                            count.to_string(),
                                            Modifier::empty().padding_horizontal(6.0),
                                            style(look.palette.muted, 10.5, None),
                                        );
                                    }
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
fn Section(look: Look, title: &'static str) {
    Text(
        title,
        Modifier::empty().padding_each(0.0, 8.0, 0.0, 0.0),
        style(look.palette.muted, 10.5, Some(FontWeight::SEMI_BOLD)),
    );
}

#[composable]
fn NodeDetails(
    state: MutableState<Studio>,
    scroll: ScrollState,
    look: Look,
    width: f32,
    height: f32,
) {
    Column(
        Modifier::empty()
            .width(width)
            .height(height)
            .vertical_scroll(scroll, false)
            .padding_each(12.0, 2.0, 12.0, 12.0),
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
                    style(look.palette.text, 15.0, Some(FontWeight::SEMI_BOLD)),
                );
                Text(
                    format!(
                        "{} × {}   at {}, {}",
                        concise(node.width),
                        concise(node.height),
                        concise(node.x),
                        concise(node.y)
                    ),
                    Modifier::empty(),
                    style(look.palette.muted, 11.5, None),
                );
                if let Some(text) = node.text.as_ref().filter(|text| !text.is_empty()) {
                    Text(
                        text.clone(),
                        Modifier::empty()
                            .fill_max_width()
                            .background(look.stage)
                            .rounded_corners(7.0)
                            .padding(8.0),
                        style(look.palette.text, 12.0, None),
                    );
                }
                if let Some(source) = studio.selected_source_request() {
                    let path = source["file"].as_str().unwrap_or_default().to_owned();
                    let line = source["line"].as_u64().unwrap_or(1);
                    let file = std::path::Path::new(&path)
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .into_owned();
                    ToolButton(
                        look,
                        icons::SOURCE,
                        format!("{file}:{line}"),
                        false,
                        true,
                        true,
                        ("Open source", TIP),
                        move |_| send(json!({"action":"navigate","file":path,"line":line})),
                    );
                }
                Section(look, "HIERARCHY");
                for ancestor in studio.selection_path() {
                    let id = ancestor.id.clone();
                    let label = ancestor.kind.clone();
                    cranpose::key(&ancestor.id, move || {
                        let id = id.clone();
                        ToolButton(
                            look,
                            "",
                            label.clone(),
                            false,
                            id == state.get().selected,
                            true,
                            ("", None),
                            move |_| {
                                edit(state, |s| s.select_node(id.clone()));
                                navigate_selected(&state.get());
                            },
                        );
                    });
                }
                if !node.sources.is_empty() {
                    Section(look, "SOURCE STACK");
                    if node.recompositions().is_some() {
                        Text(
                            "Recompositions since each instance was created; excludes its initial composition.",
                            Modifier::empty().fill_max_width(),
                            style(look.palette.muted, 11.0, None),
                        );
                    }
                }
                for (index, source) in node.sources.iter().enumerate().rev() {
                    let path = studio.resolve_source(source);
                    let line = source.line;
                    let label = source.label();
                    cranpose::key(index, move || {
                        let path = path.clone();
                        ToolButton(
                            look,
                            icons::SOURCE,
                            label.clone(),
                            false,
                            false,
                            true,
                            ("", None),
                            move |_| send(json!({"action":"navigate","file":path,"line":line})),
                        );
                    });
                }
                if !node.modifiers.is_empty() {
                    Section(look, "MODIFIERS");
                }
                for modifier in &node.modifiers {
                    Text(
                        modifier.name.clone(),
                        Modifier::empty(),
                        style(look.palette.text, 12.0, Some(FontWeight::MEDIUM)),
                    );
                    for property in &modifier.properties {
                        Row(
                            Modifier::empty().fill_max_width(),
                            RowSpec::default()
                                .horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                            {
                                let property = property.clone();
                                move || {
                                    Text(
                                        property.name.clone(),
                                        Modifier::empty().width(96.0),
                                        style(look.palette.muted, 11.0, None),
                                    );
                                    Label(
                                        property.value.clone(),
                                        Modifier::empty().weight(1.0),
                                        style(look.palette.text, 11.0, None),
                                    );
                                }
                            },
                        );
                    }
                }
            } else {
                Text(
                    "Nothing selected",
                    Modifier::empty(),
                    style(look.palette.text, 14.0, Some(FontWeight::SEMI_BOLD)),
                );
                Text(
                    "Select a view in the tree, or turn on Pick and click it in the preview, to see its size, text, source and modifiers.",
                    Modifier::empty().fill_max_width(),
                    style(look.palette.muted, 12.0, None),
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

/// An inline editor for an exact device size; it applies once both values are valid.
#[composable]
fn CustomSize(state: MutableState<Studio>, look: Look) {
    let settings = state.get().settings;
    let width =
        remember(move || TextFieldState::new(settings.width.to_string())).with(|value| *value);
    let height =
        remember(move || TextFieldState::new(settings.height.to_string())).with(|value| *value);
    let (w, h) = (width.text(), height.text());
    SideEffect(move || {
        if let (Ok(w), Ok(h)) = (w.trim().parse::<u32>(), h.trim().parse::<u32>())
            && (120..=4096).contains(&w)
            && (120..=4096).contains(&h)
        {
            let current = state.get().settings;
            if (current.width, current.height) != (w, h) {
                edit(state, |studio| {
                    studio.settings.width = w;
                    studio.settings.height = h;
                });
            }
        }
    });
    Row(
        Modifier::empty()
            .fill_max_width()
            .height(40.0)
            .background(look.bar)
            .padding_horizontal(12.0),
        RowSpec::default()
            .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
            .vertical_alignment(VerticalAlignment::CenterVertically),
        move || {
            Glyph(icons::PHONE, 14.0, look.palette.muted);
            Text(
                "Custom size",
                Modifier::empty(),
                style(look.palette.muted, 11.5, None),
            );
            for (field, hint) in [(width, "width"), (height, "height")] {
                cranpose::key(hint, move || {
                    BasicTextField(
                        field,
                        Modifier::empty()
                            .width(64.0)
                            .height(26.0)
                            .rounded_corners(6.0)
                            .background(look.stage)
                            .padding(5.0),
                        style(look.palette.text, 12.0, None),
                    );
                });
            }
            UiBox(Modifier::empty().weight(1.0), BoxSpec::default(), || {});
            ToolButton(
                look,
                icons::CLOSE,
                String::new(),
                false,
                false,
                true,
                ("Close", TIP),
                move |_| edit(state, |studio| studio.menu.clear()),
            );
        },
    );
}

fn request_snapshot(studio: &Studio) {
    if let Some(request) = studio.inspection_request() {
        send(request);
    }
}
fn navigate_selected(studio: &Studio) {
    if let Some(request) = studio.reveal_request() {
        send(request);
    }
}
fn short_label(text: &str, limit: usize) -> String {
    let mut chars = text.chars();
    let mut result = chars.by_ref().take(limit).collect::<String>();
    if chars.next().is_some() {
        result.push('…');
    }
    result
}
