use cranpose::{Button, ButtonSpec, Column, ColumnSpec, Modifier, Text, TextStyle, composable, rememberMutableStateOf};

#[composable]
pub fn Counter() {
    let count = rememberMutableStateOf(|| 0_i32);
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
