

https://github.com/user-attachments/assets/281c718c-80ce-4a84-80a2-404e3da7cad9






https://github.com/user-attachments/assets/aede3fd5-b144-4eed-b13a-2a8b9b86cefd

# Cranpose for IntelliJ IDEA

[JetBrains Marketplace](https://plugins.jetbrains.com/plugin/34594-cranpose) ·
[Signed releases](https://github.com/samoylenkodmitry/cranpose-idea/releases)

Build Cranpose applications in IntelliJ IDEA and RustRover, with interactive previews
beside Rust source. The Studio controls, inspector and tool window are written in Rust and rendered by Cranpose.

## Create and tune

Choose **File → New Project → Cranpose** to start from
[Cranpose Showcase](https://github.com/samoylenkodmitry/cranpose-showcase). The native
Cranpose wizard uses a pinned starter with Cranpose 0.1.176. Choose an empty
location; the starter is bundled and project creation works offline without Git.
Studio saves the desktop run configuration, opens source beside the preview,
and starts the first build automatically. Rust and downloaded Cargo dependencies
are needed to build and run. If Rust is missing, the preview offers setup and retry.

Inside explicit composables, **◆** marks supported string, character, boolean,
integer and float literals. Hover a diamond or color swatch for Cranpose controls;
source text remains available for ordinary cursor placement. Numbers have seekbars
with custom ranges and steps. Colors have hex, HSV and opacity controls. Supported
immutable variable references open the initializer's controls. Each gesture is
undoable in the source editor. Valid unsaved edits also update an active hot preview.

Eligible literal changes bypass compilation through a private development value
store. Text portions of `format!` are supported while its fields remain compiler-owned.
Const contexts, other macro tokens, remembered-state initializers, key identities
and structural/type changes retain the compiler/restart workflow. Callback values
take effect when the callback runs. This does not make arbitrary Rust compilation
instant. [Measurements and limits](docs/performance-0.7.0.md).

The editor adds Cranpose shader accents to component calls and stability badges.
Badges redraw when content, theme or geometry changes. Finite shader animations
mark source navigation and supported live updates without an idle animation loop.
Document-based preview gutters appear alongside Rust's macro-expansion markers.

Large, dense sets of live-value markers use fewer editor layout updates when a
structural source change rebuilds them. See the
[editor placement measurements and limits](docs/performance-0.13.3.md).

## Preview and inspect

Open a Rust source file and choose **Split**. The preview starts with the package's
default or main binary. Select another Cargo binary or example to run it instead.
Source tabs share the running preview, selected target and application state.
The preview runs your compiled application, including its state and event handlers.

- **Component previews:** register parameterless fixtures with `#[cranpose::preview]`.
  Choose named variants with their own dimensions and light or dark theme.
- **Layout inspector:** pick an element on the canvas to select its layout node.
  When elements overlap, choose from a popup beside the cursor showing their type,
  text, size and source location. A single hit jumps to source immediately.
  Inspect bounds, text, modifier values and composable origins, then jump to source.
  With a framework that reports recomposition counts, the tree and source stack
  show executions after each composable instance's initial composition.
  Live counters also appear above composable definitions in open code editors,
  even with the Inspector closed or paused. Multiple live instances are summed
  and show their instance count. Counters disappear when the preview stops.
  Tracking runs only in the plugin's debug preview process; release builds omit it.
  Search by text, component, source file or modifier. Matching nodes keep their ancestor
  context. Fold branches, follow the hierarchy, or pause and refresh a captured layout.
  Wide workspaces dock inspection beside the preview and use a compact toolbar.
  Narrow panels switch between Layout and Details; larger inspector panels show both.
- **Viewport controls:** change logical dimensions, Fit or fixed zoom, switch theme,
  and export the rendered surface as PNG.
- **Hot code reload:** eligible literals update without compilation; other supported
  edits compile on save. Both retain the process, remembered state and input connection. Compiler errors link
  to source and leave the previous preview interactive. Structural edits show
  **Restart required**. The Reload menu controls hot reload and file watching.
- **Editor integration:** Code / Split / Preview modes, composable gutter actions,
  source navigation and seven completions including `cppreview`.

### Add a component preview

Use a Cranpose revision containing [framework support in #861](https://github.com/samoylenkodmitry/Cranpose/pull/861).
The [sample](samples/counter) pins the tested revision:

```toml
[dependencies]
cranpose = { git = "https://github.com/samoylenkodmitry/Cranpose", rev = "283736c61e85486214ba0ef8b9ac813000b5055d", features = ["desktop", "preview"] }
```

```rust
#[cranpose::preview(name = "Compact", group = "Cards", width = 360, height = 180)]
#[cranpose::preview(name = "Evening", group = "Cards", width = 360, height = 180, dark = true)]
#[cranpose::composable]
fn CardPreview() {
    // Create fixture state here and call your component.
}
```

Keep the normal `AppLauncher::run(App)` entry point. The framework selects a
registered fixture when launched by the IDE. Functions taking parameters need a
parameterless fixture that supplies their inputs.

## Hot reload and release isolation

The plugin owns the compiler runner, source instrumentation, patch handling and UI.
It copies the Cargo workspace to the IDE cache and adds Subsecond only to that
private debug copy. Your sources, manifests, lockfile, profiles and release builds
remain unchanged. Cranpose itself has no new hot-reload dependency or runtime code.
The injected runtime refuses to compile without debug assertions.
Generated helper crates are cached by their contents. Restarting with unchanged
helpers reuses their Cargo artifacts, and running previews keep their original
helper version when another plugin build generates newer code.
The resolved development lockfile is also reused in later private sessions. Edits
to copied manifests, the original lockfile, toolchain or Cargo config invalidate
that seed. Cargo still validates and updates it normally. Cache failures fall back
to ordinary resolution; your application's lockfile remains the source input.
After a clean shutdown, the next hot preview can reuse its private source path.
Its contents are rebuilt from the current project; a stable path lets rustc reuse
incremental compilation work. Simultaneous previews receive separate directories.
Interrupted sessions are abandoned until cache cleanup, and ordinary previews
continue using fresh directories. Dioxus still performs the link required to start
a hot-reload session.

Stopping or replacing a preview also stops its compiler and application. The shared
Rust SDK owns their process group on Unix and their Job Object on Windows. Shutdown
signals immediately, allows bounded cleanup, and force-stops remaining descendants.
Connection failures, failed authentication and closed preview connections use the
same cleanup path. Cancellation runs off the IDE event thread.

The source watcher filters build and IDE output before batching. Compatible edits
settle for 60 ms, with a 240 ms maximum batch delay during continuous writes.
Duplicate paths share one queue entry. Lost events or too many pending files require
a restart; incompatible changes remain pending until corrected or rebuilt.

The first hot preview downloads the official Dioxus CLI 0.7.10 for your platform
and verifies its pinned SHA-256. It uses Subsecond 0.7.10 through the CLI's native
desktop compiler. A normal Rust toolchain and platform linker are required.
Set `CRANPOSE_DX` to an existing 0.7.10 CLI for offline development.

Compatible literal edits bypass compilation. Other compatible function-body
edits, new functions and imports use native code patches. Ordinary
`#[composable]`, native modifier chains and custom helper functions keep their
normal Rust syntax. Edits saved during compilation are coalesced and applied
after the current patch finishes.

Type layouts, function signatures, statics, traits, macro definitions, modules,
manifests and `main` require a rebuild, which resets live state. Invalid syntax
and compiler errors keep the last successful preview running. Changing the type
of a remembered value also requires a rebuild. See
[native reload behavior and measurements](docs/native-authoring.md).

Binary targets and binary-plus-library packages are supported. The latter get a
private binary launcher to work around the compiler's library/binary reload boundary.
Explicit and qualified `composable` attributes are instrumented; aliases and
macro-generated declarations are not. Symlinked workspaces currently require an
ordinary Cargo run. Dependencies outside the workspace are read from their real
paths and are not watched for patches.

Use **Fit** for the whole viewport, or a fixed zoom and **Alt+wheel** to pan
vertically (**Alt+Shift+wheel** horizontally). The size menu also accepts custom dimensions.

## Parameter stability

The editor shows inline badges beside composable parameter types. Hover a badge for
the comparison rule, its consequences, and suggested next steps. Analysis uses
unsaved Rust text and runs in the bundled native process.

- **stable:** a visible value-comparison contract.
- **unstable:** callbacks Cranpose marks changed on each parent invocation.
- **shared mutation:** retained parameters may alias newly mutated storage.
- **incompatible:** the type lacks a required parameter comparison trait.
- **unknown:** the contract needs compiler resolution or manual review.
- **no skip:** the function does not generate parameter comparison.

Use **Tools → Cranpose → Show Stability Badges** to toggle them, or **Include Stable
Parameters** to reduce visual detail. The same `cranpose-stability.toml` configuration
and reasoned suppressions work in the IDE and CI.

The engine is the separate Rust project [Cranpose Stability](https://github.com/samoylenkodmitry/cranpose-stability).
It supplies a standalone CLI, GitHub Action, JSON reports and SARIF output.
The plugin bundles the engine; installing another tool is unnecessary.

Cranpose uses `Clone + PartialEq` for ordinary parameters. The linter follows those
rules, including callback handling and shared mutation, rather than Kotlin collection
stability rules. This is source analysis, not measured recomposition cost. External,
generated and ambiguous contracts remain unknown. Full scope and rule documentation
are in the linter repository.

## Cargo workflow

The Cranpose tool window discovers workspace binaries and examples with a direct
Cranpose dependency, including renamed dependencies and required features.
It provides Preview, Run, Check and Test controls, compiler diagnostics, and the
current file's composable outline.

**Save run configuration** creates a persistent IDE configuration with a target,
command, features, arguments, working directory and environment. Configurations
can be edited, shared and run using the IDE's standard Run controls. Project
commands require a trusted project.

In an empty project directory, the tool window can create a runnable Cranpose
starter with its first component preview.

## Install

Requires IntelliJ IDEA 2026.1+ or RustRover 2026.1+, a Rust/Cargo toolchain, and a
Metal, Vulkan or DX12-capable system. Software Vulkan is supported.
Install JetBrains Rust support in IDEA for full Rust parsing, completion and
refactoring.

Find the plugin on [JetBrains Marketplace](https://plugins.jetbrains.com/plugin/34594-cranpose).
For versions awaiting Marketplace review, download the signed ZIP from
[GitHub Releases](https://github.com/samoylenkodmitry/cranpose-idea/releases).
In **Settings → Plugins → gear → Install Plugin from Disk**, select its ZIP.

1. Open the directory containing the workspace's `Cargo.toml`.
2. Open **View → Tool Windows → Cranpose** and select a target.
3. Open a Rust source file, choose **Split**, and build its preview.
4. Choose a registered component or the whole application in the preview selector.

Commands are also available through **Tools → Cranpose** and **Find Action**.
Version tags run the complete CI suite, sign the verified archive and attach it
to [GitHub Releases](https://github.com/samoylenkodmitry/cranpose-idea/releases).
After the first manual Marketplace listing, tag builds submit updates automatically.
See [Publishing](docs/publishing.md).

## Current boundaries

- Previews run local Cargo binaries and examples. Library components need a fixture
  reachable from one of those targets.
- Explicit restarts reset in-memory state; compatible hot patches preserve it.
- Inspection covers the primary surface. Selecting a node or using Pick reveals
  its application component invocation in source. The private preview build adds
  these call origins; enclosing composables are the fallback for other nodes.
  Individual modifier calls do not have separate source origins.
- Rust language analysis and debugging use the JetBrains Rust plugin. Cranpose's
  saved configurations provide Run, Check and Test.
- The embedded host's current IME, IDE shortcut and screen-reader limitations
  apply to rendered Cranpose surfaces.
- Attribute aliases and macro-generated declarations are left to Rust language
  support; the composable outline recognizes literal and qualified attributes.

## Develop

All authored implementation and tooling is Rust. Cranpose renders the Studio,
tool window and Cargo settings form. Rust generates the small JVM class adapters
required by IntelliJ extension points; there is no Kotlin/Java source or compiler
in the build. IntelliJ's own JVM runtime remains a platform dependency.

```sh
cargo fmt --all --check
cargo test --locked --no-default-features --workspace
cargo clippy --locked --no-default-features --workspace --all-targets --features cranpose-ide-host/ide-tests -- -D warnings
cargo run -p xtask -- release audit-source
cargo run -p xtask -- package
cargo run -p xtask -- ide-test --ide /path/to/IntelliJ-IDEA

```

### Measure startup, reload and inspector costs

```sh
cargo build --locked -p cranpose-dev-runner
cargo run --locked -p xtask -- hot-smoke --runner target/debug/cranpose-dev-runner --fixture counter --cache target/hot-cache --log reload.log --measure-rounds 6 --report reload.json
cargo run --locked -p xtask -- hot-smoke --runner target/debug/cranpose-dev-runner --fixture counter --cache target/hot-cache --log reload-noise.log --measure-rounds 6 --background-noise-ms 3000 --report reload-noise.json
cargo run --locked -p xtask -- hot-smoke --runner target/debug/cranpose-dev-runner --fixture counter --cache target/hot-cache --log restart.log --startup-only --profile-startup --require-cached-dependencies --build-diagnostics --require-cached-support --idle-seconds 20 --idle-settle-seconds 5 --report restart.json
cargo test --release --no-default-features -p cranpose-intellij-ui benchmark_inspector_model -- --ignored --nocapture
```

The Rust harness verifies each patch generation, PID and remembered counter state,
then tests recovery from compiler, syntax and state-type errors. Timings include raw
save-to-acknowledgement and save-to-snapshot samples. Snapshot polling adds up to
about 200 ms of observation delay. Compare warm runs on the same machine with
other builds stopped; startup and compilation caches can dominate early samples.
The inspector benchmark measures model operations, not whole-IDE frame rates.
CI stores the reload JSON alongside its logs without machine-dependent timing limits.
The restart check requires a previous launch with the same fixture and cache.
Startup profiling separates runner preparation from Dioxus timestamps and host
connection/snapshot observation. Idle phases each settle for five seconds, then
measure twenty seconds of process CPU with the preview visible, inspected every
500 ms, and hidden. CPU percentages describe one core. Keep generated fixture
caches outside Cargo's target directory when using `rust-cache` in CI.
See [0.6.4 measurements](docs/performance-0.6.4.md) and [0.6.5 warm restart measurements](docs/performance-0.6.5.md) for conditions and raw summaries.

Local packaging writes a ZIP for this machine under `target/plugin`. Rebuilding the
plugin UI reconnects its controls to the running preview without starting another
application. Session checkpoints stay in the IDE process and omit layout frames.
The native IDE suite starts a separate headless IDEA with isolated settings and
runs Rust assertions through the real plugin class loader. It checks frame updates,
native rendering, configuration persistence and stale-result handling for badges.
It also rebuilds the Studio UI and checks that its controller state reconnects.
Its logs, JSON results and rendered images are under `target/ide-tests`.
Use IDEA for the headless suite; RustRover's standalone launcher requires a license
in the isolated configuration. Both products are checked by Plugin Verifier.

On Linux, `cargo run -p xtask -- fetch-ide` downloads the pinned IDEA SDK and
checks its official SHA-256. `bridge-test --ide /path/to/IDE` also verifies the
generated adapters with full JVM verification and JNI checking.

CI builds native hosts and UIs for macOS, Linux and Windows on aarch64 and x86_64.
The live hot-reload harness is Rust as well. It tests binary and library applications,
state retention, compiler errors, invalid syntax, incompatible state changes and recovery.
One ZIP contains all six platforms. See [Publishing](docs/publishing.md).

## Architecture

- `ui`: Cranpose workspace controls, Studio, inspector, run settings and analyzer integration.
- `ide-host`: a small Rust entry point configuring the template's shared native host.
- `xtask`: a small Rust entry point configuring the template's shared build and release tools.
- [Template SDK](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template/tree/main/sdk): owns the JNI host, JVM classfile writer, packaging and IDE tests, live authoring, shaders, theme model and searchable trees. Its [shared UI and job APIs](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template/blob/main/docs/shared-ui-and-jobs.md) also provide Studio's compact actions, task output and safe document-save dispatch. This plugin pins those crates by commit; the template has no dependency back on this repository.
- `dev-runner` / `dev-macros`: private debug workspace, compiler provisioning, edit compatibility checks and hot-call instrumentation.
- [Cranpose Stability](https://github.com/samoylenkodmitry/cranpose-stability): the separate Rust analyzer and CI action, pinned by commit.
- Cranpose supplies component registration, layout metadata, source origins and rendering.

## Credits and license

Apache-2.0. Built from the
[Cranpose IntelliJ plugin template](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template)
and powered by [Cranpose](https://github.com/samoylenkodmitry/Cranpose).
The stability badge presentation was inspired by
[Compose Stability Analyzer](https://github.com/skydoves/compose-stability-analyzer).
The analysis engine and badge descriptions are Rust; the JVM adapter uses IntelliJ's
editor inlay API for placement, font scaling, theme colors and tooltips.

Hot reload uses [Dioxus Subsecond and the Dioxus CLI](https://github.com/DioxusLabs/dioxus),
dual-licensed under MIT and Apache-2.0.

The signing setup and publishing conventions follow Dmitry Samoylenko's
[Zeus Thunderbolt](https://github.com/samoylenkodmitry/Zeus-Thunderbolt-Idea-Plugin)
and [DiffTrack / Branch Lens](https://github.com/samoylenkodmitry/difftrack).
