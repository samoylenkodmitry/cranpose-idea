# Compact target selection — 0.13.2

The Application card now displays its selected Cargo target in a compact summary.
Opening the summary reveals a search field and five choices per page. Search
matches target and package names. Selecting a result sends the same opaque Cargo
target ID to the existing host handler, closes the chooser and returns keyboard
focus to the summary. Tab/Shift-Tab and Enter work with the controls.

The shared implementation, standalone fixture and Rust native regression tool
live in the plugin template. Studio pins the UI crates at
`693bfbb6e346a9356577f635ba3cf4e487afa479` and the test tools at
`28de318352bfe8dc84bfdbddf4cc82de6f8e2ba6`. The host and Cranpose Build retain their
common process SDK revision from the Cranpose 0.1.172 upgrade.

## Validation and limits

- Native pointer and keyboard tests at 1x/2x verify duplicate-name identities,
  filter clearing, paging, removal of a selected target and an empty catalog.
- A 10,001-item fixture composes 36 UI nodes with five visible results. The
  collapsed control retains the same height as the seven-item fixture. CI caps
  the fixture at 65 nodes.
- The shared `choice-smoke --dashboard` mode records Preview and Build & run
  positions in an isolated 440 × 1200 logical-pixel dashboard with seven targets.
  The installed 0.13.1 baseline places them at y=622 and y=883 respectively.
  The candidate places them at y=304 and y=565: both are 318 logical pixels
  higher. The dashboard composes 51 nodes instead of 80.
- Installed RustRover testing passed search for `planet` and Tab/Enter selection
  of `robot-planet-gallery`, with the host acknowledging the selected target.
- That check exposed incomplete background coverage below short content. A
  viewport-sized background container now surrounds the scrollable dashboard.
  The rendered-pixel regression fails against the first candidate (top RGB
  30/31/34, bottom 18/18/24) and passes with the fix (30/31/34 at both locations).
  CI runs this check on macOS, Linux and Windows.

These are layout and interaction observations. They do not establish an idle CPU,
startup, compilation, or live-edit speed improvement. The framework also advances
to Cranpose 0.1.172 in this release. Application release sources and profiles are
unchanged, and the chooser adds no timer.
