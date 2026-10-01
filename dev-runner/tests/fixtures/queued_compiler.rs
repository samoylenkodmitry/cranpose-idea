use std::{
    fs,
    io::{self, Write},
    thread,
    time::{Duration, Instant},
};

fn emit(message: &str) {
    println!("{message}");
    io::stdout().flush().expect("compiler output");
}

fn main() {
    if std::env::args().any(|arg| arg == "--version") {
        println!("dx 0.7.10");
        return;
    }
    let source = || fs::read_to_string("src/main.rs").expect("compiler input");
    let deadline = Instant::now() + Duration::from_secs(15);
    emit(r#"{"fake":"ready"}"#);
    while !source().contains("broken()") {
        assert!(Instant::now() < deadline, "missing broken source");
        thread::sleep(Duration::from_millis(10));
    }
    emit(r#"{"$message_type":"diagnostic","level":"error","message":"cannot find function broken"}"#);
    emit(r#"{"fake":"diagnostic"}"#);
    let finish = Instant::now() + Duration::from_millis(800);
    while Instant::now() < finish {
        if !source().contains("broken()") {
            emit(r#"{"fake":"lost"}"#);
            std::process::exit(1);
        }
        thread::sleep(Duration::from_millis(10));
    }
    emit(r#"{"level":"ERROR","message":"Build failed: compiler error"}"#);
    while !source().contains("latest()") {
        assert!(Instant::now() < deadline, "missing latest source");
        thread::sleep(Duration::from_millis(10));
    }
    emit(r#"{"level":"INFO","message":"Hot-patching: src/main.rs took 1ms"}"#);
    emit(r#"{"fake":"applied"}"#);
}
