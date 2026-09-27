# Offline project creation — 0.11.1

**File → New Project → Cranpose** creates the bundled Showcase without Git,
download utilities or network access. Previously it called `git fetch`, which
failed with “program not found” on Windows machines without Git.

The shared Rust SDK embeds the same pinned Showcase revision,
`dc439faf9fd019191ccbf1c6fa70f2c523ae1163`, using Cranpose 0.1.167. The full source
archive includes its license, assets, platform projects and executable scripts.
Generation checks its SHA-256, unpacks to a private staging directory, and installs
only into an empty project location. IDE metadata is allowed. Cancellation and
failures clean up newly created files; existing source files are never overwritten.
No template scripts run during generation.

Project creation needs no development tools. Building and running the generated
Rust app still requires a Rust toolchain, platform linker and downloaded Cargo
dependencies. The wizard states this distinction. Application release sources
and dependencies are unchanged by this plugin update.

## Regression checks

- Windows, Linux and macOS run generation with an empty `PATH` and unreachable
  HTTP proxies, and compare all 66 generated files against the pinned archive.
- Unix checks retain executable permissions. Other checks cover IDE metadata,
  cancellation, existing projects, path escapes, links and path collisions.
- Actual IDE tests exercise the wizard's asynchronous generation worker and
  verify application sources, assets, license and destination.
- `xtask starter-smoke --output <empty-directory>` runs the production generator
  from a prebuilt Rust executable, including on machines without Rust or Git.

The archive adds approximately 5.5 MiB to each native host before plugin ZIP
compression. Keeping the starter available offline is intentional. Updating it
requires a reviewed revision and checksum change in the shared SDK.
