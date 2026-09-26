use cranpose_dev_macros::hot;

mod __cranpose_dev {
    pub fn observe_patch() {}
    pub fn call<A, R>(arguments: A, mut body: impl FnMut(A) -> R) -> R {
        body(arguments)
    }
}

#[hot]
fn consume(text: String) -> usize {
    text.into_bytes().len()
}
#[hot]
fn destructure((left, right): (i32, i32)) -> i32 {
    left + right
}
#[hot]
fn callback(mut call: impl FnMut() -> i32) -> i32 {
    call() + call()
}
#[hot]
fn borrowed(text: &str) -> &str {
    text
}

#[test]
fn hot_boundary_preserves_parameter_types_ownership_patterns_and_returns() {
    assert_eq!(consume("owned".into()), 5);
    assert_eq!(destructure((2, 3)), 5);
    let mut value = 0;
    assert_eq!(
        callback(|| {
            value += 1;
            value
        }),
        3
    );
    assert_eq!(borrowed("borrowed"), "borrowed");
}
