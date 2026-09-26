# Cranpose for IntelliJ IDEA

Build Cranpose applications in IntelliJ IDEA and RustRover, with interactive previews
beside Rust source and a tool window written in Cranpose.

## Preview and inspect

Open a Rust source file and choose **Split**. Select a Cargo binary or example, then
**Build preview**. The preview runs your compiled application, including its state
and event handlers.

- **Component previews:** register parameterless fixtures with `#[cranpose::preview]`.
  Choose named variants with their own dimensions and light or dark theme.
- **Layout inspector:** pick an element on the canvas to select its layout node.
  Inspect bounds, text, modifier values and composable origins, then jump to source.
  Live updates preserve the selected node; pause them to examine a captured layout.
- **Viewport controls:** change logical dimensions, Fit or fixed zoom, switch theme,
  and export the rendered surface as PNG.
- **Rebuild on save:** Rust, WGSL and manifest saves rebuild the active preview.
  Compiler errors link to source, while the last successful preview stays interactive.
- **Editor integration:** Code / Split / Preview modes, composable gutter actions,
  source navigation and seven completions including `cppreview`.

### Add a component preview

Use a Cranpose revision containing [framework support in #861](https://github.com/samoylenkodmitry/Cranpose/pull/861).
The [sample](samples/counter) pins the tested revision:

```toml
[dependencies]
cranpose = { git = "https://github.com/samoylenkodmitry/Cranpose", rev = "e177b19985c303a13fcf40d61decc7253fdbe057", features = ["desktop", "preview"] }
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

Download the `plugin` artifact from a successful [Build](https://github.com/samoylenkodmitry/cranpose-idea/actions/workflows/build.yml)
run. In **Settings → Plugins → gear → Install Plugin from Disk**, select its ZIP.

1. Open the directory containing the workspace's `Cargo.toml`.
2. Open **View → Tool Windows → Cranpose** and select a target.
3. Open a Rust source file, choose **Split**, and build its preview.
4. Choose a registered component or the whole application in the preview selector.

Commands are also available through **Tools → Cranpose** and **Find Action**.
Marketplace publication is a separate step.

## Current boundaries

- Previews run local Cargo binaries and examples. Library components need a fixture
  reachable from one of those targets.
- Rebuilds restart the process and reset its in-memory state.
- Inspection covers the primary surface. Source origins identify enclosing
  composable functions; they are not individual modifier call-site locations.
- Rust language analysis and debugging use the JetBrains Rust plugin. Cranpose's
  saved configurations provide Run, Check and Test.
- The embedded host's current IME, IDE shortcut and screen-reader limitations
  apply to rendered Cranpose surfaces.
- Attribute aliases and macro-generated declarations are left to Rust language
  support; the composable outline recognizes literal and qualified attributes.

## Develop

```sh
cargo fmt --all --check
cargo test --locked --no-default-features -p cranpose-intellij-ui
cargo clippy --locked --no-default-features -p cranpose-intellij-ui --all-targets -- -D warnings
python3 -m unittest discover -s scripts/tests
cd plugin
./gradlew test buildPlugin
./gradlew verifyPlugin
./gradlew runIde -PrunIdeProject=/path/to/cargo/project
```

Gradle downloads the configured IDEA and a Java 21 toolchain when needed.
Use `-PplatformLocalPath=/path/to/RustRover.app` for an installed IDE.
The native UI runs in a separate process. For UI hot reload, build it and pass
`-PcranposeUiBinary=/absolute/path/to/target/debug/cranpose-intellij-ui` to `runIde`.

CI builds native binaries for macOS, Linux and Windows on aarch64 and x86_64,
runs tests with software Vulkan, verifies IDEA/RustRover compatibility, and bundles
one plugin ZIP. Local `buildPlugin` bundles the current machine's binary.
See [Publishing](docs/publishing.md) for signing and the separate Marketplace step.

## Architecture

- `ui/src/dashboard.rs`: Cranpose-rendered workspace controls and source navigation.
- `CranposeProjectService.kt`: project-scoped Cargo processes and compiler diagnostics.
- `PreviewFileEditor.kt` / `PreviewWorkspace.kt`: source preview, process lifecycle and inspector.
- `CranposeRunConfiguration.kt`: persistent IDE run configurations.
- `CranposeEditorSupport.kt`: completions and gutter markers.
- The template's session and surface code provides authenticated native transport,
  scaling, input, lifecycle callbacks and pixel capture.
- Cranpose supplies compiled fixture registration, layout metadata and source origins.

## Credits and license

Apache-2.0. Built from the
[Cranpose IntelliJ plugin template](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template)
and powered by [Cranpose](https://github.com/samoylenkodmitry/Cranpose).

The signing setup and publishing conventions follow Dmitry Samoylenko's
[Zeus Thunderbolt](https://github.com/samoylenkodmitry/Zeus-Thunderbolt-Idea-Plugin)
and [DiffTrack / Branch Lens](https://github.com/samoylenkodmitry/difftrack).
