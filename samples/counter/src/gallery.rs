use cranpose::{
    Button, ButtonSpec, Color, Column, ColumnSpec, LinearArrangement, Modifier, Row, RowSpec,
    SpanStyle, Text, TextStyle, composable, isSystemInDarkTheme, rememberMutableStateOf,
    text::{FontWeight, TextUnit},
};

#[derive(Clone, Copy, PartialEq)]
struct Ink {
    page: Color,
    surface: Color,
    text: Color,
    muted: Color,
    accent: Color,
}

fn ink(dark: bool) -> Ink {
    if dark {
        Ink {
            page: Color(0.09, 0.11, 0.12, 1.0),
            surface: Color(0.14, 0.17, 0.18, 1.0),
            text: Color(0.92, 0.94, 0.91, 1.0),
            muted: Color(0.55, 0.63, 0.60, 1.0),
            accent: Color(0.58, 0.81, 0.67, 1.0),
        }
    } else {
        Ink {
            page: Color(0.96, 0.96, 0.93, 1.0),
            surface: Color(1.0, 1.0, 0.99, 1.0),
            text: Color(0.13, 0.20, 0.18, 1.0),
            muted: Color(0.43, 0.49, 0.45, 1.0),
            accent: Color(0.19, 0.42, 0.31, 1.0),
        }
    }
}

fn typography(color: Color, size: f32, bold: bool) -> TextStyle {
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

#[cranpose::preview(name = "Compact", group = "Field notes", width = 390, height = 720)]
#[cranpose::preview(name = "Comfortable", group = "Field notes", width = 560, height = 760)]
#[cranpose::preview(
    name = "Evening",
    group = "Field notes",
    width = 390,
    height = 720,
    dark = true
)]
#[composable]
pub fn FieldNotes() {
    let palette = ink(isSystemInDarkTheme());
    let saved = rememberMutableStateOf(|| false);
    let collected = rememberMutableStateOf(|| 3);
    let filter = rememberMutableStateOf(|| 0);
    Column(
        Modifier::empty()
            .fill_max_size()
            .background(palette.page)
            .padding(28.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(22.0)),
        move || {
            Row(
                Modifier::empty().fill_max_width(),
                RowSpec::default().horizontal_arrangement(LinearArrangement::SpaceBetween),
                move || {
                    Text(
                        "FIELD NOTES",
                        Modifier::empty(),
                        typography(palette.accent, 11.0, true),
                    );
                    Text(
                        "VOL. 04 / SEP",
                        Modifier::empty(),
                        typography(palette.muted, 10.0, false),
                    );
                },
            );
            Column(
                Modifier::empty(),
                ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(6.0)),
                move || {
                    Text(
                        "A little room\nto notice.",
                        Modifier::empty(),
                        typography(palette.text, 34.0, true),
                    );
                    Text(
                        "Observations from an ordinary day.",
                        Modifier::empty(),
                        typography(palette.muted, 13.0, false),
                    );
                },
            );
            Row(
                Modifier::empty(),
                RowSpec::default().horizontal_arrangement(LinearArrangement::spaced_by(8.0)),
                move || {
                    for (index, label) in ["All notes", "Saved"].into_iter().enumerate() {
                        let selected = filter.get() == index;
                        Button(
                            Modifier::empty()
                                .background(if selected {
                                    palette.accent
                                } else {
                                    palette.surface
                                })
                                .rounded_corners(18.0)
                                .padding(10.0),
                            ButtonSpec::default(),
                            move || filter.set(index),
                            move || {
                                Text(
                                    label,
                                    Modifier::empty(),
                                    typography(
                                        if selected {
                                            palette.page
                                        } else {
                                            palette.muted
                                        },
                                        12.0,
                                        true,
                                    ),
                                );
                            },
                        );
                    }
                },
            );
            if filter.get() == 0 || saved.get() {
                Column(
                    Modifier::empty()
                        .fill_max_width()
                        .background(palette.surface)
                        .rounded_corners(16.0)
                        .padding(22.0),
                    ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(15.0)),
                    move || {
                        Text(
                            "09:41  ·  OUTSIDE",
                            Modifier::empty(),
                            typography(palette.muted, 10.0, true),
                        );
                        Text(
                            "The quiet part\nof the morning",
                            Modifier::empty(),
                            typography(palette.text, 23.0, true),
                        );
                        Text(
                            "Light on the kitchen wall.\nCoffee getting cold.\nNothing that needs to happen yet.",
                            Modifier::empty(),
                            typography(palette.muted, 14.0, false),
                        );
                        Button(
                            Modifier::empty()
                                .background(palette.page)
                                .rounded_corners(8.0)
                                .padding(10.0),
                            ButtonSpec::default(),
                            move || saved.set(!saved.get()),
                            move || {
                                Text(
                                    if saved.get() {
                                        "Saved to collection"
                                    } else {
                                        "Save this note"
                                    },
                                    Modifier::empty(),
                                    typography(palette.accent, 12.0, true),
                                );
                            },
                        );
                    },
                );
            } else {
                Text(
                    "Save a note to keep it here.",
                    Modifier::empty().padding(22.0),
                    typography(palette.muted, 14.0, false),
                );
            }
            ObservationCounter(collected.get(), "A collection of small things", move || {
                collected.set(collected.get() + 1)
            });
        },
    );
}

#[cranpose::preview(name = "Status card", group = "Components", width = 360, height = 180)]
#[composable]
pub fn StatusCard() {
    let palette = ink(isSystemInDarkTheme());
    Column(
        Modifier::empty()
            .fill_max_size()
            .background(palette.page)
            .padding(24.0),
        ColumnSpec::default().vertical_arrangement(LinearArrangement::spaced_by(10.0)),
        move || {
            Text(
                "ALL CAUGHT UP",
                Modifier::empty(),
                typography(palette.accent, 11.0, true),
            );
            Text(
                "A good place to pause.",
                Modifier::empty(),
                typography(palette.text, 24.0, true),
            );
            Text(
                "Your observations are right here.",
                Modifier::empty(),
                typography(palette.muted, 13.0, false),
            );
        },
    );
}

#[composable]
fn ObservationCounter(count: i32, description: &'static str, on_add: impl Fn() + 'static) {
    let on_add = std::rc::Rc::new(on_add);
    let palette = ink(isSystemInDarkTheme());
    Row(
        Modifier::empty().fill_max_width(),
        RowSpec::default().horizontal_arrangement(LinearArrangement::SpaceBetween),
        move || {
            Column(Modifier::empty(), ColumnSpec::default(), move || {
                Text(
                    format!("{count:02} observations"),
                    Modifier::empty(),
                    typography(palette.text, 14.0, true),
                );
                Text(
                    description,
                    Modifier::empty(),
                    typography(palette.muted, 11.0, false),
                );
            });
            Button(
                Modifier::empty()
                    .background(palette.accent)
                    .rounded_corners(20.0)
                    .padding(12.0),
                ButtonSpec::default(),
                {
                    let on_add = on_add.clone();
                    move || on_add()
                },
                move || {
                    Text("+", Modifier::empty(), typography(palette.page, 18.0, true));
                },
            );
        },
    );
}
