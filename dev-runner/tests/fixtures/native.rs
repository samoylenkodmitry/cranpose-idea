use cranpose::{
    composable, rememberMutableStateOf, AppLauncher, Button, ButtonSpec, Column, ColumnSpec,
    Color, Modifier, Text, TextStyle,
};
use std::fmt::Display;

trait CardStyle {
    fn card(self, inset: f32) -> Self;
}
impl CardStyle for Modifier {
    fn card(self, inset: f32) -> Self {
        self.fill_max_width().background(Color(0.125, 0.25, 0.375, 1.0)).padding(inset)
    }
}

fn caption<T: Display>(value: T) -> String {
    format!("Count: {value}")
}

#[composable]
fn Readout(value: i32, modifier: Modifier) {
    Column(modifier, ColumnSpec::default(), move || {
    Text(caption(value), Modifier::empty(), TextStyle::default());
    for label in ["Rust", "modifiers"] {
        Text(label, Modifier::empty(), TextStyle::default());
    }
    if value > 0 {
        Text("State retained", Modifier::empty(), TextStyle::default());
    }
    });
}

#[composable]
fn Counter() {
    let count = rememberMutableStateOf(|| 0_i32);
    Column(Modifier::empty().fill_max_size().padding(8.0), ColumnSpec::default(), move || {
        Readout(count.get(), Modifier::empty().card(8.0));
        Button(
            Modifier::empty().width(180.0).height(48.0),
            ButtonSpec::default(),
            move || count.set(count.get() + 1),
            || { Text("Increment", Modifier::empty(), TextStyle::default()); },
        );
    });
}

fn main() {
    let endpoint = cranpose::embed::EmbedEndpoint::from_env().expect("embedded acceptance host");
    AppLauncher::new().with_size(480, 400).run_embedded(endpoint, Counter);
}
