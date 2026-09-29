use super::{ReloadDecision, classify};

const APP: &str = r#"use cranpose::{Column, Modifier, Text};

#[derive(Clone, Copy)]
struct Palette { accent: f32 }

static TITLE: &str = "Counter";

#[composable]
fn Card(label: String) {
    Text(label, Modifier::empty());
}

#[composable]
fn App() {
    let count = rememberMutableStateOf(|| 0_i32);
    Column(Modifier::empty().padding(8.0), move || {
        Text(format!("Count: {}", count.get()), Modifier::empty());
        Card("Hello".into());
    });
}

impl Palette {
    fn accent(&self) -> f32 { self.accent }
}

impl std::fmt::Display for Palette {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", self.accent) }
}

mod widgets {
    pub fn helper() -> u32 { 1 }
}

fn main() {
    launch(App);
}
"#;

fn edit(from: &str, to: &str) -> String {
    assert!(APP.contains(from), "{from}");
    APP.replacen(from, to, 1)
}

fn restart(next: &str) -> String {
    match classify(APP, next) {
        ReloadDecision::Restart(reason) => reason,
        other => panic!("expected restart, got {other:?} for\n{next}"),
    }
}

#[test]
fn hot_patches_function_body_edits_including_shifted_positions() {
    for next in [
        // Added call and statement shift every later call site.
        edit(
            "        Card(\"Hello\".into());\n",
            "        Card(\"Hello\".into());\n        Text(\"Added\", Modifier::empty());\n",
        ),
        edit(
            "    let count = rememberMutableStateOf(|| 0_i32);\n",
            "\n\n    let count = rememberMutableStateOf(|| 0_i32);\n    let doubled = count.get() * 2;\n",
        ),
        // Removed and reordered calls.
        edit("        Card(\"Hello\".into());\n", ""),
        edit(
            "        Text(format!(\"Count: {}\", count.get()), Modifier::empty());\n        Card(\"Hello\".into());\n",
            "        Card(\"Hello\".into());\n        Text(format!(\"Count: {}\", count.get()), Modifier::empty());\n",
        ),
        // Modifier chains, control flow, closures and captures.
        edit(".padding(8.0)", ".padding(8.0).fill_max_width()"),
        edit(
            "        Card(\"Hello\".into());\n",
            "        if count.get() > 2 { Card(\"Many\".into()); } else { for _ in 0..2 { Card(\"Few\".into()); } }\n",
        ),
        edit("move || {", "move || { let label = TITLE;"),
        // Non-composable function and method bodies.
        edit("{ self.accent }", "{ self.accent * 2.0 }"),
        edit(
            "write!(f, \"{}\", self.accent)",
            "write!(f, \"accent {}\", self.accent)",
        ),
        edit(
            "pub fn helper() -> u32 { 1 }",
            "pub fn helper() -> u32 { 1 + 1 }",
        ),
        // Whitespace-only edits move call sites without changing tokens.
        edit("fn App() {\n", "fn App()\n{\n"),
        // Literal values keep their types.
        edit("0_i32", "5_i32"),
        edit("8.0", "12.5"),
    ] {
        assert_eq!(classify(APP, &next), ReloadDecision::Patch, "{next}");
    }
}

#[test]
fn hot_patches_new_and_removed_functions_and_imports() {
    for next in [
        edit(
            "fn main() {",
            "#[composable]\nfn Badge(text: String) {\n    Text(text, Modifier::empty());\n}\n\nfn main() {",
        ),
        edit(
            "fn main() {",
            "/// Formats a label.\nfn helper(value: i32) -> String { value.to_string() }\n\nfn main() {",
        ),
        edit(
            "    fn accent(&self) -> f32 { self.accent }\n",
            "    fn accent(&self) -> f32 { self.accent }\n    fn doubled(&self) -> f32 { self.accent * 2.0 }\n",
        ),
        edit(
            "    pub fn helper() -> u32 { 1 }\n",
            "    pub fn helper() -> u32 { 1 }\n    pub fn other() -> u32 { 2 }\n",
        ),
        // Renaming is a removal plus an addition.
        edit("fn Card(label: String) {", "fn Tile(label: String) {")
            .replace("Card(\"Hello\"", "Tile(\"Hello\""),
        edit("    fn accent(&self) -> f32 { self.accent }\n", ""),
        edit(
            "use cranpose::{Column, Modifier, Text};",
            "use cranpose::{Column, Modifier, Row, Text};\nuse std::fmt::Write;",
        ),
        // Documentation and lint attributes never change compiled code.
        edit(
            "#[derive(Clone, Copy)]\nstruct Palette",
            "/// Colors.\n#[derive(Clone, Copy)]\n#[allow(dead_code)]\nstruct Palette",
        ),
        edit(
            "struct Palette { accent: f32 }",
            "struct Palette {\n    /// Accent channel.\n    accent: f32,\n}",
        ),
        // Tests are not part of the preview build.
        format!(
            "{APP}\n#[cfg(test)]\nmod tests {{\n    struct Probe {{ value: u8 }}\n    #[test]\n    fn probe() {{}}\n}}\n"
        ),
    ] {
        assert_eq!(classify(APP, &next), ReloadDecision::Patch, "{next}");
    }
}

#[test]
fn rebuilds_declarations_old_code_shares_with_new_code() {
    for (next, reason) in [
        (
            edit(
                "struct Palette { accent: f32 }",
                "struct Palette { accent: f32, muted: f32 }",
            ),
            "Struct `Palette` fields changed",
        ),
        (
            edit(
                "struct Palette { accent: f32 }",
                "struct Palette { accent: f64 }",
            ),
            "Struct `Palette` fields changed",
        ),
        (
            edit("#[derive(Clone, Copy)]", "#[derive(Clone, Copy, Debug)]"),
            "Attributes of struct `Palette` changed",
        ),
        (
            edit(
                "static TITLE: &str = \"Counter\";",
                "static TITLE: &str = \"Total\";",
            ),
            "Static `TITLE` changed",
        ),
        (
            edit(
                "static TITLE: &str = \"Counter\";",
                "static TITLE: &str = \"Counter\";\nconst LIMIT: u32 = 4;",
            ),
            "Constant `LIMIT` added",
        ),
        (
            edit(
                "static TITLE: &str = \"Counter\";",
                "static TITLE: &str = \"Counter\";\nenum Mode { Light }",
            ),
            "Enum `Mode` added",
        ),
        (
            edit("static TITLE: &str = \"Counter\";\n", ""),
            "Static `TITLE` removed",
        ),
        (
            edit("fn Card(label: String) {", "fn Card(label: &str) {"),
            "Signature of `Card` changed",
        ),
        (
            edit(
                "fn Card(label: String) {",
                "fn Card(label: String, size: f32) {",
            ),
            "Signature of `Card` changed",
        ),
        (
            edit("fn accent(&self) -> f32 {", "fn accent(&self) -> f64 {"),
            "Signature of `Palette::accent` changed",
        ),
        (
            edit("#[composable]\nfn Card", "#[composable(no_skip)]\nfn Card"),
            "Attributes of function `Card` changed",
        ),
        (
            edit("launch(App);", "launch(App);\n    println!(\"started\");"),
            "`main` changed",
        ),
        (edit("launch(App);", "launch(Card);"), "`main` changed"),
        (
            edit("impl Palette {", "impl Clone2 for Palette {"),
            "Impl `Clone2 for Palette` added",
        ),
        (
            edit(
                "impl std::fmt::Display for Palette {",
                "impl std::fmt::Debug for Palette {",
            ),
            "Impl `std::fmt::Debug for Palette` added",
        ),
        (
            edit(
                "fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, \"{}\", self.accent) }",
                "",
            ),
            "Function `Palette::fmt` removed",
        ),
        (edit("mod widgets {", "mod parts {"), "Module `parts` added"),
        (format!("{APP}\nmod extra;\n"), "Module `extra` added"),
        (
            format!("{APP}\nmacro_rules! twice {{ ($e:expr) => {{ $e * 2 }}; }}\n"),
            "Macro `twice` added",
        ),
        (
            format!("{APP}\ntrait Shape {{ fn area(&self) -> f32; }}\n"),
            "Trait `Shape` added",
        ),
        (
            format!(
                "{APP}\n#[composable]\n#[preview]\nfn CardPreview() {{ Card(\"x\".into()); }}\n"
            ),
            "Preview `CardPreview` added",
        ),
        (
            format!("{APP}\n#[no_mangle]\nfn exported() {{}}\n"),
            "Function `exported` added",
        ),
    ] {
        assert_eq!(restart(&next), reason, "{next}");
    }
}

#[test]
fn rebuilds_items_nested_in_function_bodies_and_literal_type_changes() {
    let local = edit(
        "    let count",
        "    struct Local { value: u8 }\n    let count",
    );
    assert_eq!(restart(&local), "Struct `Local` added");
    assert_eq!(
        classify(
            &local,
            &local.replace("let count", "let total = 1;\n    let count")
        ),
        ReloadDecision::Patch
    );
    assert_eq!(
        classify(&local, &local.replace("value: u8", "value: u16")),
        ReloadDecision::Restart("Struct `Local` fields changed".into())
    );
    let nested = edit(
        "    pub fn helper() -> u32 { 1 }",
        "    pub fn helper() -> u32 { fn inner(x: u8) -> u8 { x } inner(1) as u32 }",
    );
    assert_eq!(
        classify(&nested, &nested.replace("inner(x: u8)", "inner(x: u16)")),
        ReloadDecision::Restart("Signature of `inner` changed".into())
    );
    assert_eq!(
        classify(&nested, &nested.replace("{ x }", "{ x + 1 }")),
        ReloadDecision::Patch
    );
    // State types follow literal types; a state slot keeps its value only when unchanged.
    assert_eq!(
        restart(&edit("0_i32", "0_i64")),
        "Literal types changed in `App`"
    );
    assert_eq!(restart(&edit("8.0", "8")), "Literal types changed in `App`");
    let repeat = edit("    let count", "    let cells = [0u8; 4];\n    let count");
    assert!(matches!(
        classify(&repeat, &repeat.replace("; 4]", "; 5]")),
        ReloadDecision::Restart(_)
    ));
    // A structural edit together with a type change is compiled; slots reset safely.
    assert_eq!(
        classify(
            APP,
            &edit(
                "0_i32);",
                "0_i64);\n    Text(\"extra\", Modifier::empty());"
            )
        ),
        ReloadDecision::Patch
    );
}

#[test]
fn keeps_invalid_edits_out_of_the_running_workspace() {
    assert!(matches!(
        classify("fn App() {}", "fn App() {"),
        ReloadDecision::Invalid(_)
    ));
    assert!(matches!(
        classify("fn App() {}", "fn App() {}\nfn Other() { let }"),
        ReloadDecision::Invalid(message) if message.starts_with("line 2: ")
    ));
    assert_eq!(
        classify("fn App() {}", "fn App() {}"),
        ReloadDecision::Unchanged
    );
    assert_eq!(
        classify("fn App( {}", "fn App() {}"),
        ReloadDecision::Restart("Previous source had a syntax error".into())
    );
}

#[test]
fn patches_ui_literals_and_callback_amounts() {
    let old = "fn Card() { Text(\"Before\"); count.set(count.get() + 1); }";
    assert_eq!(
        classify(old, &old.replace("Before", "After!").replace("+ 1", "+ 2")),
        ReloadDecision::Patch
    );
}
