use super::*;

#[test]
fn unicode_comments_and_nested_functions_keep_valid_spans() {
    let source = r#"/* é✓ */ #[composable] fn Title() { Text("héllo"); }
mod nested { /* λ */ #[cranpose::composable] fn Card() {} }
"#;
    let result = instrument(source).expect("instrument unicode source");
    assert!(syn::parse_file(&result).is_ok(), "{result}");
    assert_eq!(result.matches("#[cranpose_dev_macros::hot]").count(), 2);
    assert_eq!(source.lines().count(), result.lines().count());
}
