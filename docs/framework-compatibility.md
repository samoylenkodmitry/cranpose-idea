# Cranpose-main compatibility checks

The scheduled workflow uses the template's shared Rust tools to test one
framework snapshot across the whole dependency graph. Earlier preparation
changed only Studio's direct UI dependencies; pinned shared SDK crates kept
the release revision and produced incompatible `Color` types.

`cargo run --locked -p xtask -- use-framework-main` now adds workspace-root
overrides for the discovered Cranpose and Coroflow packages. The overrides also
reach pinned SDK dependencies. Preparation checks the all-features Cargo graph
and rejects leftover framework revisions before running tests. It restores the
manifest and lockfile if preparation fails.

Use a disposable checkout. `--checkout /path/to/Cranpose` selects an existing
snapshot, and `--report framework-compatibility.json` records its revision and
resolved packages. The workflow retains this report with the generated root
manifest and lockfile, then runs workspace tests and the actual IDEA suite.
See the template's [usage and regression coverage](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template/blob/main/docs/framework-compatibility.md).

This change updates development tools only. Studio's runtime SDK pins, pinned
Cranpose revision, application sources and release artifacts are unchanged.
