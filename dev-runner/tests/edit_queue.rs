#![cfg(unix)]
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    fs,
    io::{BufRead, BufReader},
    process::{Command, Stdio},
    sync::mpsc,
    time::{Duration, Instant},
};

#[test]
fn saves_during_a_failed_patch_coalesce_and_resume_without_another_save() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().join("app");
    fs::create_dir_all(root.join("src"))?;
    fs::create_dir_all(root.join("cranpose/src"))?;
    fs::write(
        root.join("Cargo.toml"),
        "[package]\nname='queue-fixture'\nversion='0.1.0'\nedition='2024'\n[dependencies]\ncranpose={path='cranpose'}\n",
    )?;
    fs::write(
        root.join("cranpose/Cargo.toml"),
        "[package]\nname='cranpose'\nversion='0.1.0'\nedition='2024'\n[features]\npreview=[]\nhot-reload=[]\n",
    )?;
    fs::write(root.join("cranpose/src/lib.rs"), "")?;
    let source = root.join("src/main.rs");
    let original = "fn screen() { initial(); }\nfn main() {}\n";
    fs::write(&source, original)?;
    let compiler = temp.path().join("dx");
    let compiler_source = temp.path().join("compiler.rs");
    fs::write(
        &compiler_source,
        include_str!("fixtures/queued_compiler.rs"),
    )?;
    let build = Command::new("rustc")
        .arg("--edition=2024")
        .arg(&compiler_source)
        .arg("-o")
        .arg(&compiler)
        .output()?;
    ensure!(
        build.status.success(),
        "Fixture compiler: {}",
        String::from_utf8_lossy(&build.stderr)
    );
    let options = json!({"root":root,"package":"queue-fixture","target":"queue-fixture","kind":"bin",
        "cache":temp.path().join("cache"),"hotReload":true,"watch":true});
    let mut command = Command::new(env!("CARGO_BIN_EXE_cranpose-dev-runner"));
    command
        .arg(options.to_string())
        .env("CRANPOSE_DX", compiler)
        .env("CARGO_NET_OFFLINE", "true")
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit());
    let mut runner = cranpose_plugin_process::Process::spawn_worker(command)?;
    let stdout = runner.stdout.take().context("runner stdout")?;
    let (sender, events) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in BufReader::new(stdout).lines().map_while(Result::ok) {
            let _ = sender.send(line);
        }
    });
    let wait = |expected: &str| -> Result<()> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let line = events
                .recv_timeout(Duration::from_secs(5))
                .context("compiler event")?;
            if let Ok(event) = serde_json::from_str::<Value>(&line) {
                ensure!(
                    event["fake"] != "lost",
                    "Runner submitted a save before the compiler finished"
                );
                if event["fake"] == expected {
                    return Ok(());
                }
            }
        }
        anyhow::bail!("Missing {expected}");
    };
    wait("ready")?;
    fs::write(&source, original.replace("initial()", "broken()"))?;
    wait("diagnostic")?;
    fs::write(&source, original.replace("initial()", "intermediate()"))?;
    std::thread::sleep(Duration::from_millis(150));
    fs::write(&source, original.replace("initial()", "latest()"))?;
    wait("applied")?;
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        if let Some(status) = runner.try_wait()? {
            ensure!(status.success(), "Runner failed: {status}");
            break;
        }
        ensure!(Instant::now() < deadline, "Runner did not stop");
        std::thread::sleep(Duration::from_millis(20));
    }
    reader.join().expect("output reader");
    Ok(())
}
