use anyhow::{Context, Result, bail, ensure};
use cranpose_plugin_host::{
    protocol::{Event, Packet},
    session::{Options, Session, SessionEvent},
};
use serde_json::{Value, json};
use std::{
    collections::BTreeMap,
    fs,
    io::Write,
    path::Path,
    sync::mpsc::RecvTimeoutError,
    time::{Duration, Instant},
};

struct Preview {
    session: Session,
    runtime: Value,
    logs: Vec<String>,
    log: fs::File,
    request: u32,
    frames: usize,
}

impl Preview {
    fn event(&mut self) -> Result<Option<(String, Value)>> {
        match self.session.events.recv_timeout(Duration::from_millis(25)) {
            Ok(SessionEvent::Connected) => {
                self.session.send(
                    Packet::new(1)
                        .int(0)
                        .int(480)
                        .int(400)
                        .float(1.0)
                        .float(60.0),
                );
                self.session.send(Packet::new(13).int(0).byte(1));
            }
            Ok(SessionEvent::Data(Event::Frame(frame))) => {
                self.frames += 1;
                self.session
                    .send(Packet::new(11).int(frame.surface).int(frame.id));
            }
            Ok(SessionEvent::Data(Event::Message(channel, payload))) => {
                let payload: Value = serde_json::from_str(&payload)?;
                if channel == "cranpose.dev.applied" {
                    self.runtime = payload.clone();
                }
                return Ok(Some((channel, payload)));
            }
            Ok(SessionEvent::Log(line)) => {
                writeln!(self.log, "{line}")?;
                self.logs.push(line);
            }
            Ok(SessionEvent::Stopped(reason)) => bail!("Preview stopped: {reason}"),
            Err(RecvTimeoutError::Disconnected) => bail!("Preview event stream closed"),
            Err(RecvTimeoutError::Timeout) | Ok(_) => {}
        }
        Ok(None)
    }

    fn snapshot(&mut self, expected: &[&str], after: Option<u64>) -> Result<Value> {
        self.request += 1;
        let deadline = Instant::now() + Duration::from_secs(180);
        let mut next = Instant::now();
        let mut last = Vec::new();
        while Instant::now() < deadline {
            if !self.runtime.is_null() && Instant::now() >= next {
                self.session.send(Packet::message(
                    "cranpose.inspector.v2.request",
                    &self.request.to_string(),
                ));
                next = Instant::now() + Duration::from_millis(25);
            }
            if let Some((channel, payload)) = self.event()?
                && channel == "cranpose.inspector.v2.snapshot"
                && payload["requestId"] == self.request
            {
                let nodes = payload["nodes"].as_array().context("inspector nodes")?;
                last = nodes
                    .iter()
                    .filter_map(|node| node["text"].as_str().map(str::to_owned))
                    .collect();
                if !self.runtime.is_null()
                    && after.is_none_or(|old| {
                        self.runtime["generation"]
                            .as_u64()
                            .is_some_and(|new| new > old)
                    })
                    && expected
                        .iter()
                        .all(|text| last.iter().any(|actual| actual == text))
                {
                    return Ok(payload);
                }
            }
        }
        bail!(
            "Missing {expected:?}; visible: {last:?}; runtime: {}",
            self.runtime
        )
    }

    fn click(&self, snapshot: &Value, text: &str) -> Result<()> {
        let node = snapshot["nodes"]
            .as_array()
            .context("nodes")?
            .iter()
            .find(|node| node["text"] == text)
            .context("button label")?;
        let bounds = node;
        let x =
            bounds["x"].as_f64().context("x")? + bounds["width"].as_f64().context("width")? / 2.0;
        let y =
            bounds["y"].as_f64().context("y")? + bounds["height"].as_f64().context("height")? / 2.0;
        for kind in [2, 3, 4] {
            self.session
                .send(Packet::new(kind).int(0).float(x as f32).float(y as f32));
        }
        Ok(())
    }

    fn edit(
        &mut self,
        path: &Path,
        source: &str,
        expected: &[&str],
        samples: &mut Vec<Value>,
        name: &str,
    ) -> Result<Value> {
        let generation = self.runtime["generation"].as_u64().context("generation")?;
        let pid = self.runtime["pid"].clone();
        let started = Instant::now();
        fs::write(path, source)?;
        let snapshot = self.snapshot(expected, Some(generation))?;
        ensure!(self.runtime["pid"] == pid, "Edit restarted the app");
        samples
            .push(json!({"edit":name, "saveToSnapshotMs":started.elapsed().as_secs_f64()*1000.0}));
        Ok(snapshot)
    }
}

#[test]
#[ignore = "native desktop and Dioxus toolchain; run with --ignored --nocapture"]
fn native_rust_edits_keep_state_and_render_full_modifiers() -> Result<()> {
    let cache = std::env::var_os("CRANPOSE_NATIVE_SMOKE_CACHE")
        .map(std::path::PathBuf::from)
        .unwrap_or_else(|| std::env::temp_dir().join("cranpose-native-rust-smoke"));
    fs::create_dir_all(&cache)?;
    let lease =
        cranpose_plugin_cache::WorkspaceLease::acquire(&cache.join("fixtures"), b"native-rust-v1")?;
    let root = lease.path().join("workspace");
    fs::create_dir_all(root.join("src"))?;
    fs::write(
        root.join("Cargo.toml"),
        include_str!("fixtures/counter/Cargo.toml"),
    )?;
    let original = include_str!("fixtures/native.rs");
    let source = root.join("src/main.rs");
    fs::write(&source, original)?;
    let options = json!({"root":root, "package":"cranpose-hot-counter", "target":"cranpose-hot-counter",
        "kind":"bin", "cache":cache, "hotReload":true, "watch":true});
    let session = Session::start(Options {
        command: vec![
            env!("CARGO_BIN_EXE_cranpose-dev-runner").into(),
            options.to_string(),
        ],
        directory: Some(root),
        environment: BTreeMap::new(),
        timeout: Duration::from_secs(180),
    })?;
    let mut preview = Preview {
        session,
        runtime: Value::Null,
        logs: Vec::new(),
        log: fs::File::create(cache.join("native-rust.log"))?,
        request: 0,
        frames: 0,
    };
    let initial = preview.snapshot(&["Count: 0", "Increment", "Rust", "modifiers"], None)?;
    preview.click(&initial, "Increment")?;
    let before = preview.snapshot(&["Count: 1", "State retained"], None)?;
    let mut samples = Vec::new();

    let modifiers = original.replace(
        "Modifier::empty().card(8.0)",
        "{ let inset = if count.get() > 0 { 24.0 } else { 8.0 }; Modifier::empty().card(inset).height(128.0) }",
    ).replace("\"Rust\"", "\"Edited Rust\"");
    let after = preview.edit(
        &source,
        &modifiers,
        &["Count: 1", "Increment", "Edited Rust"],
        &mut samples,
        "modifier chain and conditional",
    )?;
    let left = |snapshot: &Value| -> Result<f64> {
        snapshot["nodes"]
            .as_array()
            .context("nodes")?
            .iter()
            .find(|node| node["text"] == "Count: 1")
            .context("count label")?["x"]
            .as_f64()
            .context("left")
    };
    ensure!(
        (left(&after)? - left(&before)? - 16.0).abs() < 1.0,
        "Modifier padding did not reach rendered layout: {} -> {}",
        left(&before)?,
        left(&after)?
    );

    let composable = modifiers.replace(
        "Text(caption(value), Modifier::empty(), TextStyle::default());",
        "Text(caption(value), Modifier::empty(), TextStyle::default());\n    Badge(value);",
    ) + "\n#[composable]\nfn Badge(value: i32) { Text(format!(\"Custom: {}\", value * 2), Modifier::empty(), TextStyle::default()); }\n";
    preview.edit(
        &source,
        &composable,
        &["Count: 1", "Custom: 2", "State retained"],
        &mut samples,
        "new composable",
    )?;
    let callback = composable.replace("move || count.set(count.get() + 1)",
        "move || { let step = if count.get() > 0 { 2 } else { 1 }; count.set(count.get() + step); }");
    let snapshot = preview.edit(
        &source,
        &callback,
        &["Count: 1", "Custom: 2"],
        &mut samples,
        "callback body",
    )?;
    preview.click(&snapshot, "Increment")?;
    preview.snapshot(&["Count: 3", "Custom: 6", "State retained"], None)?;

    let offset = preview.logs.len();
    fs::write(
        &source,
        callback.replace("caption(value)", "missing_function(value)"),
    )?;
    let deadline = Instant::now() + Duration::from_secs(60);
    while !preview.logs[offset..]
        .iter()
        .any(|line| line.contains("cannot find function"))
    {
        ensure!(Instant::now() < deadline, "Missing compiler diagnostic");
        preview.event()?;
    }
    preview.snapshot(&["Count: 3", "Custom: 6"], None)?;
    preview.edit(
        &source,
        &callback,
        &["Count: 3", "Custom: 6"],
        &mut samples,
        "compiler error recovery",
    )?;
    preview.edit(
        &source,
        original,
        &["Count: 3", "State retained"],
        &mut samples,
        "revert",
    )?;
    preview.edit(
        &source,
        &original.replace("\"Increment\"", "\"Live literal\""),
        &["Count: 3", "Live literal"],
        &mut samples,
        "literal after structural patches",
    )?;

    let offset = preview.logs.len();
    fs::write(&source, original.replace("0_i32", "0_i64"))?;
    let deadline = Instant::now() + Duration::from_secs(10);
    loop {
        preview.event()?;
        if preview.logs[offset..].iter().any(|line| {
            serde_json::from_str::<Value>(line)
                .is_ok_and(|event| event["cranposeDev"] == "rebuildRequired")
        }) {
            break;
        }
        ensure!(
            Instant::now() < deadline,
            "Incompatible state change did not request a rebuild"
        );
    }
    preview.snapshot(&["Count: 3"], None)?;

    let mut pids = vec![preview.runtime["pid"].as_u64().context("app pid")? as u32];
    for line in &preview.logs {
        if let Ok(event) = serde_json::from_str::<Value>(line)
            && event["cranposeDev"] == "compiler"
            && let Some(pid) = event["pid"].as_u64()
        {
            pids.push(pid as u32);
        }
    }
    preview.session.close();
    let deadline = Instant::now() + Duration::from_secs(6);
    while pids
        .iter()
        .any(|pid| cranpose_plugin_process::is_running(*pid))
    {
        ensure!(
            Instant::now() < deadline,
            "Preview processes survived shutdown: {pids:?}"
        );
        std::thread::sleep(Duration::from_millis(25));
    }
    let report = json!({"result":"passed", "snapshotPollMs":25, "patches":samples,
        "state":3, "frames":preview.frames, "stoppedPids":pids, "restartRequired":"state type changed"});
    fs::write(
        cache.join("native-rust.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    fs::write(source, original)?;
    lease.complete()?;
    Ok(())
}

impl Drop for Preview {
    fn drop(&mut self) {
        self.session.close();
        let deadline = Instant::now() + Duration::from_secs(6);
        while Instant::now() < deadline {
            match self.session.events.recv_timeout(Duration::from_millis(25)) {
                Ok(SessionEvent::Stopped(_)) | Err(RecvTimeoutError::Disconnected) => break,
                _ => {}
            }
        }
    }
}
