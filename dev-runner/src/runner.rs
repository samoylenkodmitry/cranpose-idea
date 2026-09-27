use crate::{
    policy::{ReloadDecision, classify},
    toolchain,
    workspace::{DevWorkspace, Metadata},
};
use anyhow::{Context, Result, bail};
use cranpose_plugin_watch::{BatchPolicy, ChangeQueue};
use notify::Watcher;
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    fs,
    io::{BufRead, BufReader},
    path::PathBuf,
    process::{Command, Stdio},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

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
    let directory = options
        .cache
        .join(format!("session-{}-{sequence}", std::process::id()));
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
    if let Some(stdout) = child.stdout.take() {
        pump(stdout, workspace.source_maps());
    }
    if let Some(stderr) = child.stderr.take() {
        pump(stderr, workspace.source_maps());
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
    }
    let mut watcher_invalidated = false;
    let mut dirty = BTreeSet::new();
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
        let Some(batch) = changes.take(Duration::from_millis(200)) else {
            continue;
        };
        if batch.invalidated {
            watcher_invalidated = true;
            emit(
                "restartRequired",
                "File watcher lost changes; restart the preview",
            );
            continue;
        }
        if watcher_invalidated {
            continue;
        }
        dirty.extend(batch.items);
        if dirty.len() > 4096 {
            dirty.clear();
            watcher_invalidated = true;
            emit(
                "restartRequired",
                "Too many pending changes; restart the preview",
            );
            continue;
        }
        let mut patches = Vec::new();
        let mut reason = None;
        for relative in &dirty {
            let Some(previous) = workspace.sources.get(relative) else {
                reason = Some("Project files changed".to_owned());
                break;
            };
            let Ok(next) = fs::read_to_string(workspace.original.join(relative)) else {
                reason = Some("Source file removed".to_owned());
                break;
            };
            match classify(previous, &next) {
                ReloadDecision::Unchanged => {}
                ReloadDecision::Patch if options.hot_reload => {
                    patches.push((relative.clone(), next))
                }
                ReloadDecision::Invalid(error) => {
                    emit("error", &error);
                    patches.clear();
                    reason = Some("Fix the Rust syntax to reload".into());
                    break;
                }
                ReloadDecision::Restart(message) => {
                    reason = Some(message);
                    break;
                }
                ReloadDecision::Patch => {
                    reason = Some("Source changed; rebuild the preview".into());
                    break;
                }
            }
        }
        if let Some(reason) = reason {
            emit("restartRequired", &reason);
            continue;
        }
        dirty.clear();
        if !patches.is_empty() {
            emit(
                "patching",
                "Compiling changes; the running preview remains interactive",
            );
            for (relative, source) in patches {
                workspace.write_source(&relative, &source)?;
                workspace.sources.insert(relative, source);
            }
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

fn pump(reader: impl std::io::Read + Send + 'static, maps: Vec<(PathBuf, PathBuf)>) {
    std::thread::spawn(move || {
        for line in BufReader::new(reader).lines().map_while(Result::ok) {
            println!("{}", map_output(&line, &maps));
        }
    });
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
