//! Studio's visual language: vector icons, quiet controls and status marks.
//!
//! Colors derive from the IDE palette so light and dark themes read alike.
//! Controls are icon-first with optional labels, show hover and press
//! feedback, and report their window bounds so menus can open natively.
use crate::ide::Palette;
use cranpose::{
    Box as UiBox, BoxSpec, Button, ButtonSpec, Color, LinearArrangement, Modifier,
    PointerEventKind, PointerInputScope, Rect, Row, RowSpec, SpanStyle, Text, TextOptions,
    TextStyle, TextWithOptions, VerticalAlignment, composable, rememberMutableInteractionSource,
    rememberMutableStateOf,
    text::{FontWeight, TextOverflow, TextUnit},
};

/// 24×24 filled glyphs (Material Symbols, Apache-2.0).
pub mod icons {
    pub const PLAY: &str = "M8 5v14l11-7z";
    pub const STOP: &str = "M6 6h12v12H6z";
    pub const RESTART: &str = "M17.65 6.35A7.95 7.95 0 0 0 12 4a8 8 0 1 0 7.73 10h-2.08A6 6 0 1 1 12 6c1.66 0 3.14.69 4.22 1.78L13 11h7V4z";
    pub const PICK: &str = "M12 8a4 4 0 1 0 0 8 4 4 0 0 0 0-8zm8.94 3A8.99 8.99 0 0 0 13 3.06V1h-2v2.06A8.99 8.99 0 0 0 3.06 11H1v2h2.06A8.99 8.99 0 0 0 11 20.94V23h2v-2.06A8.99 8.99 0 0 0 20.94 13H23v-2zM12 19a7 7 0 1 1 0-14 7 7 0 0 1 0 14z";
    pub const LAYERS: &str = "M11.99 18.54l-7.37-5.73L3 14.07l9 7 9-7-1.63-1.27zM12 16l7.36-5.73L21 9l-9-7-9 7 1.63 1.27z";
    pub const SUN: &str = "M12 7a5 5 0 1 0 0 10 5 5 0 0 0 0-10zM2 13h2a1 1 0 0 0 0-2H2a1 1 0 0 0 0 2zm18 0h2a1 1 0 0 0 0-2h-2a1 1 0 0 0 0 2zM11 2v2a1 1 0 0 0 2 0V2a1 1 0 0 0-2 0zm0 18v2a1 1 0 0 0 2 0v-2a1 1 0 0 0-2 0zM5.99 4.58a1 1 0 0 0-1.41 1.41l1.06 1.06a1 1 0 0 0 1.41-1.41zm12.37 12.37a1 1 0 0 0-1.41 1.41l1.06 1.06a1 1 0 0 0 1.41-1.41zm1.06-10.96a1 1 0 0 0-1.41-1.41l-1.06 1.06a1 1 0 0 0 1.41 1.41zM7.05 18.36a1 1 0 0 0-1.41-1.41l-1.06 1.06a1 1 0 0 0 1.41 1.41z";
    pub const MOON: &str = "M12 3a9 9 0 1 0 9 9c0-.46-.04-.92-.1-1.36a5.39 5.39 0 0 1-4.4 2.26 5.4 5.4 0 0 1-3.14-9.8c-.44-.06-.9-.1-1.36-.1z";
    pub const PHONE: &str = "M17 1.01L7 1c-1.1 0-2 .9-2 2v18c0 1.1.9 2 2 2h10c1.1 0 2-.9 2-2V3c0-1.1-.9-1.99-2-1.99zM17 19H7V5h10z";
    pub const FIT: &str = "M17 4h3c1.1 0 2 .9 2 2v2h-2V6h-3zM4 8V6h3V4H4c-1.1 0-2 .9-2 2v2zm16 8v2h-3v2h3c1.1 0 2-.9 2-2v-2zM7 18H4v-2H2v2c0 1.1.9 2 2 2h3zM18 8H6v8h12z";
    pub const MORE: &str = "M6 10a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm12 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4zm-6 0a2 2 0 1 0 0 4 2 2 0 0 0 0-4z";
    pub const CHEVRON: &str = "M16.59 8.59L12 13.17 7.41 8.59 6 10l6 6 6-6z";
    pub const CHEVRON_RIGHT: &str = "M10 6L8.59 7.41 13.17 12l-4.58 4.59L10 18l6-6z";
    pub const APP: &str = "M4 8h4V4H4zm6 12h4v-4h-4zm-6 0h4v-4H4zm0-6h4v-4H4zm6 0h4v-4h-4zm6-10v4h4V4zm-6 4h4V4h-4zm6 6h4v-4h-4zm0 6h4v-4h-4z";
    pub const PAUSE: &str = "M6 19h4V5H6zm8-14v14h4V5z";
    pub const REFRESH: &str = RESTART;
    pub const EXPAND: &str = "M12 5.83L15.17 9l1.41-1.41L12 3 7.41 7.59 8.83 9zm0 12.34L8.83 15l-1.41 1.41L12 21l4.59-4.59L15.17 15z";
    pub const COLLAPSE: &str = "M7.41 18.59L8.83 20 12 16.83 15.17 20l1.41-1.41L12 14zm9.18-13.18L15.17 4 12 7.17 8.83 4 7.41 5.41 12 10z";
    pub const SEARCH: &str = "M15.5 14h-.79l-.28-.27A6.47 6.47 0 0 0 16 9.5 6.5 6.5 0 1 0 9.5 16c1.61 0 3.09-.59 4.23-1.57l.27.28v.79l5 4.99L20.49 19zm-6 0C7.01 14 5 11.99 5 9.5S7.01 5 9.5 5 14 7.01 14 9.5 11.99 14 9.5 14z";
    pub const CLOSE: &str = "M19 6.41L17.59 5 12 10.59 6.41 5 5 6.41 10.59 12 5 17.59 6.41 19 12 13.41 17.59 19 19 17.59 13.41 12z";
    pub const SOURCE: &str =
        "M9.4 16.6L4.8 12l4.6-4.6L8 6l-6 6 6 6zm5.2 0l4.6-4.6-4.6-4.6L16 6l6 6-6 6z";
    pub const WARNING: &str = "M1 21h22L12 2zm12-3h-2v-2h2zm0-4h-2v-4h2z";
    pub const BOLT: &str = "M11 21h-1l1-7H7.5c-.58 0-.57-.32-.38-.66.19-.34.05-.08.07-.12C8.48 10.94 10.42 7.54 13 3h1l-1 7h3.5c.49 0 .56.33.47.51l-.07.15C12.96 17.55 11 21 11 21z";
    pub const BUILD: &str = "M22.7 19l-9.1-9.1c.9-2.3.4-5-1.5-6.9-2-2-5-2.4-7.4-1.3L9 6 6 9 1.6 4.7C.4 7.1.9 10.1 2.9 12.1c1.9 1.9 4.6 2.4 6.9 1.5l9.1 9.1c.4.4 1 .4 1.4 0l2.3-2.3c.5-.4.5-1.1.1-1.4z";
    pub const FOLDER: &str = "M20 6h-8l-2-2H4c-1.1 0-2 .9-2 2v12c0 1.1.9 2 2 2h16c1.1 0 2-.9 2-2V8c0-1.1-.9-2-2-2zm0 12H4V8h16z";
    pub const ADD: &str = "M19 13h-6v6h-2v-6H5v-2h6V5h2v6h6z";
    pub const DOCS: &str = "M14 2H6c-1.1 0-2 .9-2 2v16c0 1.1.9 2 2 2h12c1.1 0 2-.9 2-2V8zm2 16H8v-2h8zm0-4H8v-2h8zm-3-5V3.5L18.5 9z";
    pub const CHECK: &str = "M9 16.17L4.83 12l-1.42 1.41L9 19 21 7l-1.41-1.41z";
    pub const TEST: &str = "M19.8 18.4L14 10.67V6.5l1.35-1.69c.26-.33.03-.81-.39-.81H9.04c-.42 0-.65.48-.39.81L10 6.5v4.17L4.2 18.4c-.49.66-.02 1.6.8 1.6h14c.82 0 1.29-.94.8-1.6z";
    pub const SETTINGS: &str = "M3 17v2h6v-2zM3 5v2h10V5zm10 16v-2h8v-2h-8v-2h-2v6zM7 9v2H3v2h4v2h2V9zm14 4v-2H11v2zm-6-4h2V7h4V5h-4V3h-2z";
}

/// The palette plus the derived tones Studio draws with.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Look {
    pub palette: Palette,
    pub dark: bool,
    /// The canvas behind the device frame.
    pub stage: Color,
    /// Toolbar and status bar surface.
    pub bar: Color,
    /// Hairlines between regions.
    pub line: Color,
    pub hover: Color,
    pub press: Color,
    pub success: Color,
    pub warning: Color,
    pub danger: Color,
}
impl Look {
    pub fn new(palette: Palette) -> Self {
        let b = palette.background;
        let dark = 0.2126 * b.0 + 0.7152 * b.1 + 0.0722 * b.2 < 0.5;
        let ink = if dark { 1.0 } else { 0.0 };
        let tint = |a: f32| Color(ink, ink, ink, a);
        let mix = |amount: f32| {
            Color(
                b.0 + (ink - b.0) * amount,
                b.1 + (ink - b.1) * amount,
                b.2 + (ink - b.2) * amount,
                1.0,
            )
        };
        Self {
            palette,
            dark,
            stage: if dark {
                Color(b.0 * 0.82, b.1 * 0.82, b.2 * 0.82, 1.0)
            } else {
                mix(0.035)
            },
            bar: b,
            line: tint(if dark { 0.09 } else { 0.1 }),
            hover: tint(if dark { 0.07 } else { 0.055 }),
            press: tint(if dark { 0.12 } else { 0.1 }),
            success: if dark {
                Color(0.37, 0.72, 0.54, 1.0)
            } else {
                Color(0.20, 0.55, 0.36, 1.0)
            },
            warning: if dark {
                Color(0.87, 0.66, 0.31, 1.0)
            } else {
                Color(0.69, 0.47, 0.06, 1.0)
            },
            danger: if dark {
                Color(0.90, 0.47, 0.45, 1.0)
            } else {
                Color(0.76, 0.26, 0.25, 1.0)
            },
        }
    }
    pub fn accent(&self, alpha: f32) -> Color {
        let a = self.palette.accent;
        Color(a.0, a.1, a.2, alpha)
    }
}

pub fn style(color: Color, size: f32, weight: Option<FontWeight>) -> TextStyle {
    TextStyle {
        span_style: SpanStyle {
            color: Some(color),
            font_size: TextUnit::Sp(size),
            font_weight: weight,
            ..Default::default()
        },
        ..Default::default()
    }
}

/// One line that ends in an ellipsis instead of wrapping.
#[composable]
pub fn Label(text: String, modifier: Modifier, style: TextStyle) {
    TextWithOptions(
        text,
        modifier,
        style,
        TextOptions {
            max_lines: Some(1),
            soft_wrap: false,
            overflow: TextOverflow::Ellipsis,
            ..Default::default()
        },
    );
}

#[composable]
pub fn Glyph(path: &'static str, size: f32, color: Color) {
    cranpose::widgets::Icon(path, None, size, color);
}

/// Where native tooltips go; `None` for surfaces without host support.
pub type Tip = Option<fn(&str)>;

/// A quiet toolbar control: icon, optional label and optional trailing
/// chevron. `action` receives the control's window bounds for anchoring.
#[composable]
#[expect(clippy::too_many_arguments)]
pub fn ToolButton(
    look: Look,
    icon: &'static str,
    label: String,
    chevron: bool,
    selected: bool,
    enabled: bool,
    tooltip: (&'static str, Tip),
    action: impl Fn(Rect) + 'static,
) {
    let hovered = rememberMutableStateOf(|| false);
    let anchor = rememberMutableStateOf(|| Rect {
        x: 0.0,
        y: 0.0,
        width: 0.0,
        height: 0.0,
    });
    let interaction = rememberMutableInteractionSource();
    let pressed = interaction.collectIsPressedAsState();
    let background = if !enabled {
        Color::TRANSPARENT
    } else if selected {
        look.accent(if look.dark { 0.2 } else { 0.13 })
    } else if pressed.value() {
        look.press
    } else if hovered.get() {
        look.hover
    } else {
        Color::TRANSPARENT
    };
    let tint = if !enabled {
        Color(
            look.palette.muted.0,
            look.palette.muted.1,
            look.palette.muted.2,
            0.5,
        )
    } else if selected {
        look.palette.accent
    } else {
        look.palette.text
    };
    let (tip, sink) = tooltip;
    let compact = label.is_empty() && !chevron;
    let base = Modifier::empty().height(28.0);
    let base = if compact { base.width(28.0) } else { base };
    Button(
        base.rounded_corners(6.0)
            .background(background)
            .report_window_rect_state(anchor)
            .pointer_input(tip, move |scope: PointerInputScope| async move {
                scope
                    .await_pointer_event_scope(|events| async move {
                        loop {
                            match events.await_pointer_event().await.kind {
                                PointerEventKind::Enter | PointerEventKind::Move
                                    if !hovered.get() =>
                                {
                                    hovered.set(true);
                                    if let Some(sink) = sink {
                                        sink(tip);
                                    }
                                }
                                PointerEventKind::Exit => {
                                    hovered.set(false);
                                    if let Some(sink) = sink {
                                        sink("");
                                    }
                                }
                                _ => {}
                            }
                        }
                    })
                    .await;
            }),
        ButtonSpec::new().interaction_source(interaction),
        move || {
            if enabled {
                action(anchor.get());
            }
        },
        move || {
            // Icon-only controls stay a single child: toolbars and dense
            // inspector rows are bounded by their composed node count.
            if compact {
                Glyph(icon, 16.0, tint);
                return;
            }
            let label = label.clone();
            Row(
                Modifier::empty()
                    .height(28.0)
                    .padding_horizontal(if label.is_empty() { 6.0 } else { 8.0 }),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(5.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    if !icon.is_empty() {
                        Glyph(icon, 16.0, tint);
                    }
                    if !label.is_empty() {
                        Label(
                            label.clone(),
                            Modifier::empty(),
                            style(tint, 12.0, Some(FontWeight::MEDIUM)),
                        );
                    }
                    if chevron {
                        Glyph(
                            icons::CHEVRON,
                            14.0,
                            Color(tint.0, tint.1, tint.2, tint.3 * 0.6),
                        );
                    }
                },
            );
        },
    );
}

/// The one emphasized action in a region.
#[composable]
pub fn PrimaryButton(look: Look, icon: &'static str, label: String, action: impl Fn() + 'static) {
    let interaction = rememberMutableInteractionSource();
    let pressed = interaction.collectIsPressedAsState();
    let a = look.palette.accent;
    let shade = if pressed.value() { 0.86 } else { 1.0 };
    // Saturated IDE accents read best with white text, as native buttons do.
    let ink = if 0.2126 * a.0 + 0.7152 * a.1 + 0.0722 * a.2 > 0.62 {
        Color(0.0, 0.0, 0.0, 1.0)
    } else {
        Color(1.0, 1.0, 1.0, 1.0)
    };
    Button(
        Modifier::empty()
            .height(30.0)
            .rounded_corners(7.0)
            .background(Color(a.0 * shade, a.1 * shade, a.2 * shade, 1.0)),
        ButtonSpec::new().interaction_source(interaction),
        action,
        move || {
            let label = label.clone();
            Row(
                Modifier::empty().height(30.0).padding_horizontal(12.0),
                RowSpec::default()
                    .horizontal_arrangement(LinearArrangement::spaced_by(6.0))
                    .vertical_alignment(VerticalAlignment::CenterVertically),
                move || {
                    Glyph(icon, 16.0, ink);
                    Text(
                        label.clone(),
                        Modifier::empty(),
                        style(ink, 12.5, Some(FontWeight::SEMI_BOLD)),
                    );
                },
            );
        },
    );
}

/// A vertical hairline between toolbar groups.
#[composable]
pub fn Divider(look: Look) {
    UiBox(
        Modifier::empty()
            .width(1.0)
            .height(18.0)
            .background(look.line),
        BoxSpec::default(),
        || {},
    );
}

/// A small filled circle; a soft halo marks an active state. One node.
#[composable]
pub fn Dot(color: Color, halo: bool) {
    cranpose::Canvas(Modifier::empty().width(12.0).height(12.0), move |scope| {
        let center = cranpose::Point { x: 6.0, y: 6.0 };
        if halo {
            scope.draw_circle(
                cranpose::Brush::solid(Color(color.0, color.1, color.2, 0.22)),
                center,
                6.0,
            );
        }
        scope.draw_circle(cranpose::Brush::solid(color), center, 3.0);
    });
}

/// A quiet text action for secondary commands in dense rows.
#[composable]
pub fn Link(look: Look, label: &'static str, action: impl Fn() + 'static) {
    let hovered = rememberMutableStateOf(|| false);
    Text(
        label,
        Modifier::empty()
            .padding_symmetric(4.0, 2.0)
            .pointer_input(label, move |scope: PointerInputScope| async move {
                scope
                    .await_pointer_event_scope(|events| async move {
                        loop {
                            match events.await_pointer_event().await.kind {
                                PointerEventKind::Enter | PointerEventKind::Move => {
                                    hovered.set(true)
                                }
                                PointerEventKind::Exit => hovered.set(false),
                                _ => {}
                            }
                        }
                    })
                    .await;
            })
            .clickable(move |_| action()),
        style(
            if hovered.get() {
                look.palette.accent
            } else {
                look.palette.muted
            },
            11.0,
            Some(FontWeight::MEDIUM),
        ),
    );
}
