# Cranpose 0.13.5

Picking a view and clicking its source link now opens the original source file
when the preview uses a generated launcher, including Showcase. The shared Rust
resolver understands Cargo's workspace-relative source paths and avoids
duplicating the generated package directory. Nested source mappings take
precedence over their enclosing workspace.

The running preview now follows Pick and source-link navigation across file tabs,
keeping the application state and selection. Pick works with Inspect closed so
the full preview remains available. Panel changes still wait for fresh geometry.

Live popups leave source clicks available for caret placement and stay clear of
the hovered source line. Byte-channel `Color::from_rgb_u8` and
`Color::from_rgba_u8`, plus `Color::rgb`, now have one color control and an inline
swatch. Edits retain the constructor, integer channels, comments and suffixes.

Application release builds are unchanged. This corrects source navigation; no
performance improvement is claimed.

## Included since 0.13.4

Pausing retains captured details while removing
bounds from the running preview. Reopening or resuming waits for a fresh snapshot
before restoring selection bounds. Delayed replies cannot restore an old highlight.

Uses the template's reusable Rust request fence. No new polling timer or
application release change is added. This is a selection correctness fix;
no speedup is claimed.

Includes the Cranpose 0.1.173 framework and shared SDK dependency update.

## Included since 0.13.3

Dense live-value marker replacement now uses the template's reusable Rust inlay
batch API. Literal-only edits retain their anchors; sparse files and markers at
any caret keep ordinary insertion. Unicode positions, selections, multiple
carets and callback cleanup have actual IDE regression coverage.

In three optimized headless Linux IDEA sessions, replacing 1,024 dense markers
took a pooled median 44.48 ms versus 52.99 ms with ordinary insertion (16.1% less).
This measures marker replacement only, excluding parsing, preview transport and
display. Small controls varied, and forced batching was slower on sparse files;
the production size guard avoids that path. See the
[measurement method and limits](https://github.com/samoylenkodmitry/cranpose-idea/blob/v0.13.3/docs/performance-0.13.3.md).

Uses Cranpose Build 0.1.4 with the same process SDK as the host. Application release
builds are unchanged. All six native targets, actual IDE integration, live-reload
suites and both Plugin Verifier targets passed before signing and submission.

## Included since 0.13.2

Application targets now use a compact selected-target summary. Open it to search
by target or package and choose from five results per page. Preview and build
controls stay near the top even in larger workspaces. Tab and Enter work in the
chooser, and selection returns focus to its summary.

Uses the reusable Rust/Cranpose choice control from the plugin template and
Cranpose 0.1.172. Application release builds are unchanged.

## Included since 0.13.1

Desktop Build & run now shows **Application running** after the OS creates the
process, instead of leaving **Launching application** visible until exit. Stop
remains available. Failed spawn, failed exit and cancellation keep their own
results; mobile deployment retains its device-handoff status. Running is process
status, not a first-paint or readiness guarantee.

Uses Cranpose Build 0.1.3 and the shared template's process-start notifications.
No polling timer or application release change is added.

## Included since 0.13.0

Build application packages locally for desktop and mobile from the Cranpose panel.

- Choose macOS, Linux, Windows, Android or iOS and development or release packaging.
- Check tools, set up pinned cross-compilation backends, inspect build plans, list devices and launch applications on compatible runtimes.
- Builds run in a cancellable background job with bounded output. Stop or closing the project terminates owned build processes.
- Uses the standalone Rust Cranpose Build library, also available from its public repository and native CLI releases.

Cross-compilation depends on the target SDKs and native libraries. Foreign desktop applications need their matching OS or VM to run; iOS packaging requires macOS and Xcode. Physical iOS deployment needs signing credentials. Application source and release profiles are unchanged.
