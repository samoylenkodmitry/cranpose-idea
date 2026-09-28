use crate::{
    policy::{ReloadDecision, classify},
    toolchain,
    workspace::{DevWorkspace, Metadata},
};
use anyhow::{Context, Result, bail};
use cranpose_plugin_watch::{BatchPolicy, ChangeQueue};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    io::{BufRead, BufReader},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
        mpsc,
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

/// Quiet period after the last edit that needs a new process before a rebuild is requested.
pub const REBUILD_DELAY: Duration = Duration::from_millis(400);

/// A preview launch requested by the native Studio UI.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunOptions {
    pub root: PathBuf,
    pub package: String,
    pub target: String,
    pub kind: String,
    pub cache: PathBuf,
    #[serde(default)]
    pub features: Vec<String>,
    #[serde(default)]
    pub hot_reload: bool,
    #[serde(default = "watch_by_default")]
    pub watch: bool,
}
fn watch_by_default() -> bool {
    true
}

/// Runs an isolated debug preview and watches only compatible source edits.
pub fn run(options: RunOptions) -> Result<()> {
    if !matches!(options.kind.as_str(), "bin" | "example") {
        bail!("preview target must be a binary or example");
    }
    let started = Instant::now();
    let mut phase = started;
    let profile = std::env::var_os("CRANPOSE_PROFILE_STARTUP").is_some();
    let mut measured = |name: &str| {
        let now = Instant::now();
        if profile {
            println!(
                "{}",
                serde_json::json!({"cranposeDev":"startupPhase", "phase":name,
                "durationMs":(now-phase).as_secs_f64()*1000.0,
                "elapsedMs":(now-started).as_secs_f64()*1000.0})
            );
        }
        phase = now;
    };
    emit("preparing", "Preparing private development workspace");
    let metadata = Metadata::read(&options.root)?;
    measured("metadata");
    let package = metadata
        .packages
        .iter()
        .find(|package| package.name == options.package)
        .context("Cargo package no longer exists")?;
    let target = package
        .targets
        .iter()
        .find(|target| target.name == options.target && target.kind.contains(&options.kind))
        .context("Cargo target no longer exists")?;
    let dependency = package
        .dependencies
        .iter()
        .find(|dependency| dependency.name == "cranpose")
        .context("target has no direct Cranpose dependency")?;
    let alias = dependency.rename.as_deref().unwrap_or("cranpose");
    let mut features: BTreeSet<String> = options
        .features
        .iter()
        .chain(&target.required_features)
        .cloned()
        .collect();
    features.insert(format!("{alias}/preview"));
    let sequence = SystemTime::now().duration_since(UNIX_EPOCH)?.as_millis();
    let lease = if options.hot_reload {
        fs::create_dir_all(&options.cache)?;
        let cache = options.cache.canonicalize()?;
        let original = metadata.workspace_root.canonicalize()?;
        if cache.starts_with(&original) {
            bail!("development cache must be outside the application workspace");
        }
        let mut identity = b"cranpose-dev-workspace-v1\0".to_vec();
        identity.extend_from_slice(original.as_os_str().as_encoded_bytes());
        match cranpose_plugin_cache::WorkspaceLease::acquire(&cache.join("workspaces"), &identity) {
            Ok(lease) => Some(lease),
            Err(error) => {
                eprintln!("Reusable development workspace unavailable: {error}");
                None
            }
        }
    } else {
        None
    };
    if profile {
        println!(
            "{}",
            serde_json::json!({"cranposeDev":"workspaceLease", "reused":lease.as_ref().is_some_and(|lease| lease.reused())})
        );
    }
    let directory = lease
        .as_ref()
        .map(|lease| lease.path().to_owned())
        .unwrap_or_else(|| {
            options
                .cache
                .join(format!("session-{}-{sequence}", std::process::id()))
        });
    let mut workspace = DevWorkspace::prepare_with_mode(
        &metadata,
        &directory,
        &options.cache.join("support/dev-macros"),
        options.hot_reload,
    )?;
    let launcher = if options.hot_reload {
        crate::launcher::prepare(&mut workspace, package, target)?
    } else {
        None
    };
    let kind = if launcher.is_some() {
        "bin"
    } else {
        &options.kind
    };
    let launch_package = launcher.unwrap_or_else(|| options.package.clone());
    measured("workspace");
    let dependency_cache = if options.hot_reload {
        match crate::dependency_cache::DependencyCache::restore(
            &workspace.directory,
            &options.cache.join("dependency-locks"),
        ) {
            Ok(cache) => {
                if profile {
                    println!(
                        "{}",
                        serde_json::json!({"cranposeDev":"dependencyCache", "hit":cache.hit})
                    );
                }
                Some(cache)
            }
            Err(error) => {
                eprintln!("Development dependency cache unavailable: {error}");
                None
            }
        }
    } else {
        None
    };
    measured("dependencyCache");
    println!(
        "{}",
        serde_json::json!({"cranposeDev": "workspace", "private": workspace.directory, "original": workspace.original})
    );
    let mut command = if options.hot_reload {
        emit("toolchain", "Preparing hot-patch compiler");
        let dx = toolchain::ensure(&options.cache.join("tools"))?;
        // Dioxus uses its executable as Cargo's workspace wrapper. A path per
        // lease separates application artifacts while dependencies remain shared.
        let dx = match &lease {
            Some(lease) => lease.stage_executable(&dx)?,
            None => dx,
        };
        if profile {
            println!(
                "{}",
                serde_json::json!({"cranposeDev":"compilerAlias", "path":dx})
            );
        }
        let mut command = Command::new(dx);
        command.args([
            "serve",
            "--hot-patch",
            "--platform",
            "desktop",
            "--renderer",
            "native",
            "--interactive",
            "false",
            "--open",
            "false",
            "--json-output",
            "--raw-json-diagnostics",
        ]);
        let listener = std::net::TcpListener::bind("127.0.0.1:0")?;
        let port = listener.local_addr()?.port();
        command.args(["--addr", "127.0.0.1", "--port", &port.to_string()]);
        command.arg("--session-cache-dir").arg(
            options
                .cache
                .join(format!("compiler-{}-{sequence}", std::process::id())),
        );
        command
    } else {
        let mut command = Command::new("cargo");
        command.arg("run");
        command
    };
    measured("toolchain");
    command.args([
        "--package",
        &launch_package,
        &format!("--{kind}"),
        &options.target,
    ]);
    command.args([
        "--features",
        &features.into_iter().collect::<Vec<_>>().join(","),
    ]);
    command
        .current_dir(&workspace.directory)
        .env("CARGO_TARGET_DIR", options.cache.join("target"));
    let mut live_values = options
        .hot_reload
        .then(cranpose_plugin_authoring::transport::Bridge::new)
        .transpose()?;
    if let Some(bridge) = &live_values {
        command.envs(bridge.environment()?);
    }
    command
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());
    let mut child = cranpose_plugin_process::Process::spawn_worker(command)
        .context("start development compiler")?;
    measured("compilerSpawn");
    println!(
        "{}",
        serde_json::json!({"cranposeDev":"compiler", "pid":child.id()})
    );
    let (reports, compiler) = mpsc::channel();
    if let Some(stdout) = child.stdout.take() {
        pump(stdout, workspace.source_maps(), reports.clone());
    }
    if let Some(stderr) = child.stderr.take() {
        pump(stderr, workspace.source_maps(), reports);
    }
    let stop = Arc::new(AtomicBool::new(false));
    let signal = stop.clone();
    ctrlc::set_handler(move || {
        signal.store(true, Ordering::Release);
    })?;
    let changes = ChangeQueue::new(BatchPolicy {
        quiet: Duration::from_millis(60),
        max_delay: Duration::from_millis(240),
        capacity: 4096,
    });
    let pending = changes.clone();
    let root = workspace.original.clone();
    let target_directory = metadata.target_directory.clone();
    let mut watcher =
        notify::recommended_watcher(move |event: notify::Result<notify::Event>| match event {
            Ok(event) if event.need_rescan() => pending.invalidate(),
            Ok(event) if !matches!(event.kind, notify::EventKind::Access(_)) => {
                for path in event.paths {
                    if let Some(relative) = relevant_path(&root, &target_directory, path) {
                        pending.push(relative);
                    }
                }
            }
            Ok(_) => {}
            Err(_) => pending.invalidate(),
        })?;
    if options.watch {
        watcher.watch(&workspace.original, notify::RecursiveMode::Recursive)?;
        // Edits saved while the private copy was prepared precede the watcher.
        for (relative, source) in &workspace.sources {
            if fs::read_to_string(workspace.original.join(relative))
                .ok()
                .as_ref()
                != Some(source)
            {
                changes.push(relative.clone());
            }
        }
    }
    let mut dirty = BTreeSet::new();
    // Value updates change `workspace.sources` only; interfaces compare with compiled code.
    let mut compiled = workspace.sources.clone();
    let mut rebuild = Rebuild::default();
    // Once the running process cannot follow its sources, only a rebuild helps.
    let mut stale: Option<String> = None;
    let mut patches = Patches::default();
    loop {
        if stop.load(Ordering::Acquire) {
            break;
        }
        if let Some(status) = child.try_wait()? {
            if !status.success() {
                bail!("development compiler exited with {status}");
            }
            break;
        }
        while let Ok(report) = compiler.try_recv() {
            let Some(outcome) = patches.report(report) else {
                continue;
            };
            if stale.is_none() {
                let (Outcome::Unbuilt(reason) | Outcome::Behind(reason)) = &outcome;
                emit("restartRequired", reason);
                rebuild.request(reason, Instant::now() + REBUILD_DELAY);
            }
            if let (Outcome::Behind(reason), None) = (outcome, &stale) {
                stale = Some(reason);
            }
        }
        if let Some(reason) = rebuild.due(Instant::now()) {
            emit("rebuildRequired", &reason);
        }
        let Some(batch) = changes.take(Duration::from_millis(100)) else {
            continue;
        };
        dirty.extend(batch.items);
        if stale.is_none() && batch.invalidated {
            stale = Some("File watcher lost changes".into());
        }
        if stale.is_none() && dirty.len() > 4096 {
            stale = Some("Too many pending changes".into());
        }
        if let Some(reason) = &stale {
            dirty.clear();
            emit("restartRequired", reason);
            rebuild.request(reason, Instant::now() + REBUILD_DELAY);
            continue;
        }
        let decisions: Vec<_> = dirty
            .iter()
            .map(|relative| {
                let change = change(&workspace, &compiled, relative, options.hot_reload);
                (relative.clone(), change)
            })
            .collect();
        let mut invalid = false;
        for (_, change) in &decisions {
            if let Change::Invalid(error) = change {
                emit("error", error);
                invalid = true;
            }
        }
        if invalid {
            // Keep the syntax error visible; a rebuild waits for a parseable source.
            rebuild.cancel();
            emit("restartRequired", "Fix the Rust syntax to reload");
            continue;
        }
        if let Some(reason) = decisions.iter().find_map(|(_, change)| match change {
            Change::Restart(reason) => Some(reason),
            _ => None,
        }) {
            emit("restartRequired", reason);
            rebuild.request(reason, Instant::now() + REBUILD_DELAY);
            continue;
        }
        rebuild.cancel();
        dirty.clear();
        for (relative, change) in decisions {
            let (source, values) = match change {
                Change::Values(source) => (source, true),
                Change::Compile(source) => (source, false),
                _ => continue,
            };
            if values
                && let Some(bridge) = &mut live_values
                && let Ok(catalog) = cranpose_plugin_authoring::Catalog::parse(&source)
            {
                let request = cranpose_plugin_authoring::runtime::Update {
                    file: relative.to_string_lossy().replace('\\', "/"),
                    schema: catalog.schema,
                    revision: SystemTime::now().duration_since(UNIX_EPOCH)?.as_micros() as u64,
                    values: catalog
                        .literals
                        .into_iter()
                        .map(|v| cranpose_plugin_authoring::runtime::Value {
                            id: v.id,
                            kind: v.kind,
                            value: v.value,
                        })
                        .collect(),
                };
                if let Ok(reply) = bridge.exchange(
                    &serde_json::to_string(&request)?,
                    Duration::from_millis(500),
                ) && serde_json::from_str::<serde_json::Value>(&reply)
                    .is_ok_and(|v| v["accepted"] == true)
                {
                    workspace.sources.insert(relative, source);
                    emit("valuesApplied", "Live values updated without compilation");
                    continue;
                }
            }
            emit(
                "patching",
                "Compiling changes; the running preview remains interactive",
            );
            workspace.write_source(&relative, &source)?;
            workspace.sources.insert(relative.clone(), source.clone());
            compiled.insert(relative, source);
            patches.started();
        }
    }
    child
        .terminate(Duration::from_millis(500))
        .context("stop development compiler")?;
    if let Some(cache) = dependency_cache
        && let Err(error) = cache.save()
    {
        eprintln!("Development dependency cache not saved: {error}");
    }
    if let Some(lease) = lease {
        match child.wait_for_tree_exit(Duration::from_millis(200)) {
            Ok(true) => {
                if let Err(error) = lease.complete() {
                    eprintln!("Development workspace not reusable: {error}");
                }
            }
            Ok(false) => {
                eprintln!("Development workspace abandoned: processes have not finished exiting")
            }
            Err(error) => eprintln!("Development workspace exit could not be confirmed: {error}"),
        }
    }
    Ok(())
}

/// Reject build/IDE noise before it can allocate queue entries or reset debounce.
/// Keep lexical paths for removals and atomic renames; canonicalize only relevant files.
fn relevant_path(
    root: &std::path::Path,
    target: &std::path::Path,
    path: PathBuf,
) -> Option<PathBuf> {
    let relative = path.strip_prefix(root).ok()?;
    if path.starts_with(target) || crate::workspace::ignored_path(relative) {
        return None;
    }
    if !relative.extension().is_some_and(|extension| {
        matches!(
            extension.to_str(),
            Some(
                "rs" | "toml"
                    | "wgsl"
                    | "lock"
                    | "json"
                    | "ron"
                    | "png"
                    | "jpg"
                    | "jpeg"
                    | "svg"
                    | "ttf"
                    | "otf"
            )
        )
    }) {
        return None;
    }
    let path = path.canonicalize().unwrap_or(path);
    let relative = path.strip_prefix(root).ok()?;
    if path.starts_with(target) || crate::workspace::ignored_path(relative) {
        return None;
    }
    Some(relative.to_owned())
}

fn pump(
    reader: impl std::io::Read + Send + 'static,
    maps: Vec<(PathBuf, PathBuf)>,
    reports: mpsc::Sender<Compiler>,
) {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            println!("{}", map_output(&line, &maps));
            if let Some(report) = compiler_report(&line) {
                let _ = reports.send(report);
            }
        }
    });
}

/// Hot-patch outcomes reported by `dx serve --json-output`.
#[derive(Debug, PartialEq)]
pub(crate) enum Compiler {
    CompileError,
    /// Workspace dependency crates report their diagnostics inside this message.
    BuildFailed {
        compile_error: bool,
    },
    Patched,
    PatchFailed(String),
}

pub(crate) fn compiler_report(line: &str) -> Option<Compiler> {
    let value = serde_json::from_str::<serde_json::Value>(line).ok()?;
    if (value["$message_type"] == "diagnostic" && value["level"] == "error")
        || (value["reason"] == "compiler-message" && value["message"]["level"] == "error")
    {
        return Some(Compiler::CompileError);
    }
    let message = plain(value["message"].as_str()?);
    let failure = |detail: &str| {
        let detail = detail.lines().next().unwrap_or_default();
        Compiler::PatchFailed(format!(
            "Hot patch failed: {}",
            detail.chars().take(160).collect::<String>()
        ))
    };
    match value["level"].as_str()? {
        "INFO" if message.starts_with("Hot-patching:") => Some(Compiler::Patched),
        "INFO" if message.starts_with("Starting full rebuild") => Some(failure(
            message.trim_start_matches("Starting full rebuild: "),
        )),
        "ERROR" if message.contains("Build failed") => Some(Compiler::BuildFailed {
            compile_error: message.contains(r#""level":"error""#),
        }),
        "ERROR" if message.starts_with("Failed to hot-patch app") => Some(failure(
            message.trim_start_matches("Failed to hot-patch app: "),
        )),
        "WARN" if message.starts_with("No clients to hotreload") => {
            Some(failure("the preview is not connected to the compiler"))
        }
        // After a failed first build, the compiler has no base to patch and waits forever.
        "WARN" if message.starts_with("Ignoring patch rebuild") => {
            Some(Compiler::PatchFailed("Previous build failed".into()))
        }
        _ => None,
    }
}

fn plain(text: &str) -> String {
    let mut output = String::with_capacity(text.len());
    let mut characters = text.chars();
    while let Some(character) = characters.next() {
        if character == '\u{1b}' {
            characters.by_ref().find(|c| c.is_ascii_alphabetic());
        } else {
            output.push(character);
        }
    }
    output
}

/// Only failures of the patch mechanism, not compile errors in edited code, need a
/// rebuild: a failed patch leaves the running process behind its sources.
#[derive(Default)]
pub(crate) struct Patches {
    building: bool,
    compile_error: bool,
}
impl Patches {
    pub(crate) fn started(&mut self) {
        self.building = true;
    }
    pub(crate) fn report(&mut self, report: Compiler) -> Option<Outcome> {
        match report {
            Compiler::CompileError => {
                self.compile_error = true;
                None
            }
            Compiler::Patched => {
                *self = Self::default();
                None
            }
            Compiler::BuildFailed { compile_error } => {
                let failed = self.building && !self.compile_error && !compile_error;
                *self = Self::default();
                failed.then(|| Outcome::Unbuilt("Hot patch could not be built".into()))
            }
            Compiler::PatchFailed(reason) => Some(Outcome::Behind(reason)),
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Outcome {
    /// No patch was built; a later successful patch still catches up.
    Unbuilt(String),
    /// A built patch was not loaded; the process no longer follows its sources.
    Behind(String),
}

/// A rebuild request waits until edits needing a new process have been quiet.
#[derive(Default)]
pub(crate) struct Rebuild(Option<(String, Instant)>);
impl Rebuild {
    pub(crate) fn request(&mut self, reason: &str, at: Instant) {
        self.0 = Some((reason.to_owned(), at));
    }
    pub(crate) fn cancel(&mut self) {
        self.0 = None;
    }
    pub(crate) fn due(&mut self, now: Instant) -> Option<String> {
        if self.0.as_ref().is_some_and(|(_, at)| now >= *at) {
            self.0.take().map(|(reason, _)| reason)
        } else {
            None
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum Change {
    Unchanged,
    Values(String),
    Compile(String),
    Restart(String),
    Invalid(String),
}

/// Decides how one saved file reaches the running process. `compiled` holds the
/// sources last written to the private workspace; `workspace.sources` also
/// follows value updates, which never change the compiled program.
pub(crate) fn change(
    workspace: &DevWorkspace,
    compiled: &BTreeMap<PathBuf, String>,
    relative: &Path,
    hot_reload: bool,
) -> Change {
    let name = relative.to_string_lossy().replace('\\', "/");
    let path = workspace.original.join(relative);
    if relative
        .extension()
        .is_none_or(|extension| extension != "rs")
    {
        return match (fs::read(&path), workspace.files.get(relative)) {
            (Ok(bytes), Some(digest)) if Sha256::digest(&bytes).as_slice() == digest.as_slice() => {
                Change::Unchanged
            }
            (Err(_), None) if !workspace.directory.join(relative).exists() => Change::Unchanged,
            (Err(_), _) => Change::Restart(format!("`{name}` removed")),
            _ => Change::Restart(format!("`{name}` changed")),
        };
    }
    let applied = workspace.sources.get(relative);
    let Ok(next) = fs::read_to_string(&path) else {
        return match applied {
            Some(_) => Change::Restart(format!("`{name}` removed")),
            None => Change::Unchanged,
        };
    };
    if applied == Some(&next) {
        return Change::Unchanged;
    }
    if relative.file_name().is_some_and(|file| file == "build.rs") {
        return Change::Restart(format!("`{name}` changed"));
    }
    let Some(previous) = compiled.get(relative) else {
        // A new module is compiled only after a changed `mod` item rebuilds the preview.
        return if hot_reload {
            Change::Compile(next)
        } else {
            Change::Restart(format!("`{name}` added"))
        };
    };
    // Literal edits keep the compiled schema, even when they move later columns.
    if hot_reload
        && matches!(
            (cranpose_plugin_authoring::Catalog::parse(previous), cranpose_plugin_authoring::Catalog::parse(&next)),
            (Ok(old), Ok(new)) if old.schema == new.schema && !new.literals.is_empty()
        )
    {
        return Change::Values(next);
    }
    match classify(previous, &next) {
        ReloadDecision::Unchanged => Change::Unchanged,
        ReloadDecision::Patch if hot_reload => Change::Compile(next),
        ReloadDecision::Patch => Change::Restart(format!("`{name}` changed")),
        ReloadDecision::Restart(reason) => Change::Restart(reason),
        ReloadDecision::Invalid(error) => Change::Invalid(format!("{name}: {error}")),
    }
}

fn map_output(line: &str, maps: &[(PathBuf, PathBuf)]) -> String {
    fn replace(text: &str, maps: &[(PathBuf, PathBuf)]) -> String {
        maps.iter()
            .fold(text.to_owned(), |text, (private, original)| {
                text.replace(
                    private.to_string_lossy().as_ref(),
                    original.to_string_lossy().as_ref(),
                )
            })
    }
    fn visit(value: &mut serde_json::Value, maps: &[(PathBuf, PathBuf)]) {
        match value {
            serde_json::Value::String(text) => *text = replace(text, maps),
            serde_json::Value::Array(values) => {
                values.iter_mut().for_each(|value| visit(value, maps))
            }
            serde_json::Value::Object(values) => {
                values.values_mut().for_each(|value| visit(value, maps))
            }
            _ => {}
        }
    }
    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(line) {
        visit(&mut value, maps);
        value.to_string()
    } else {
        replace(line, maps)
    }
}

#[cfg(test)]
#[path = "../tests/unit/runner.rs"]
mod tests;

fn emit(kind: &str, message: &str) {
    println!(
        "{}",
        serde_json::json!({"cranposeDev": kind, "message": message})
    );
}
