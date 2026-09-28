# Compact target selection — 0.13.2

The Application card now displays its selected Cargo target in a compact summary.
Opening the summary reveals a search field and five choices per page. Search
matches target and package names. Selecting a result sends the same opaque Cargo
target ID to the existing host handler, closes the chooser and returns keyboard
focus to the summary. Tab/Shift-Tab and Enter work with the controls.

The shared implementation, standalone fixture and Rust native regression tool
live in the plugin template. Studio pins the UI and tool crates at
`693bfbb6e346a9356577f635ba3cf4e487afa479`. The host and Cranpose Build retain their
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

These are layout and interaction observations. They do not establish an idle CPU,
startup, compilation, or live-edit speed improvement. The framework also advances
to Cranpose 0.1.172 in this release. Application release sources and profiles are
unchanged, and the chooser adds no timer.
