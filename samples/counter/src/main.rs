use cranpose::{
    AppLauncher, Button, ButtonSpec, Column, ColumnSpec, Modifier, Text, TextStyle, composable,
    rememberMutableStateOf,
};

mod gallery;

#[cranpose::preview(name = "Counter", group = "Interaction", width = 480, height = 640)]
#[composable]
fn Counter() {
    let count = rememberMutableStateOf(|| 0);
    Column(
        Modifier::empty().fill_max_size().padding(24.0),
        ColumnSpec::default(),
        move || {
            Text(
                format!("Count: {}", count.get()),
                Modifier::empty(),
                TextStyle::default(),
            );
            Button(
                Modifier::empty().padding(12.0),
                ButtonSpec::default(),
                move || count.set(count.get() + 1),
                IncrementLabel,
            );
        },
    );
}

#[composable]
fn IncrementLabel() {
    Text("Increment", Modifier::empty(), TextStyle::default());
}

fn main() {
    AppLauncher::new()
        .with_title("Field notes")
        .with_size(480, 640)
        .run(gallery::FieldNotes);
}
