use cranpose::{AppLauncher, Button, ButtonSpec, Column, ColumnSpec, Modifier, Text, TextStyle, composable, rememberMutableStateOf};

#[composable]
fn Counter() {
    let count = rememberMutableStateOf(|| 0_i32);
    // Never runs: live edits elsewhere must not wait for this literal's type.
    if count.get() < 0 {
        count.set(1000);
    }
    Column(Modifier::empty().fill_max_size().background(palette()).padding(24.0), ColumnSpec::default(), move || {
        Text(format!("Count: {}", count.get()), Modifier::empty(), TextStyle::default());
        Button(Modifier::empty().width(160.0).height(48.0), ButtonSpec::default(), move || count.set(count.get() + 1), Label);
    });
}

fn palette() -> cranpose::Color { cranpose::Color(0.125, 0.25, 0.375, 1.0) }

#[composable]
fn Label() {
    Text("Increment", Modifier::empty(), TextStyle::default());
}

fn main() {
    let endpoint = cranpose::embed::EmbedEndpoint::from_env().expect("embedded acceptance host");
    AppLauncher::new().with_size(320, 240).run_embedded(endpoint, Counter);
}
