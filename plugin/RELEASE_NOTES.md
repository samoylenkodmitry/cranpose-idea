# Cranpose 0.14.4

Value edits apply without compiling again. In 0.14.2 and 0.14.3, a change to a
number, color or text was compiled instead whenever its file had a literal that
had not run yet, such as an animation duration or a number in a click handler.
Most files have one, so almost every edit waited for the compiler.

The preview starts by itself when the panel opens. While Cargo reads the
project the panel says so, and while the preview builds it shows the crate
being compiled. If the project has no application to run, the panel says why.
A preview you stop stays stopped; automatic start can be turned off in the
panel's menu.

Strings and characters no longer open a popup. Edit them in place; the change
still applies without compiling.

The panel, popups and the New Project wizard use plainer wording.

Uses plugin template and SDK 0.8.1 and Cranpose Build 0.1.6.

## Included since 0.14.3

Studio now builds on released versions of everything it uses: Cranpose 0.1.176,
plugin template and SDK 0.8.0, Cranpose Build 0.1.5 and the stability analyzer
0.2.0. New projects start from the Showcase 0.1.25 release, which uses Cranpose
0.1.176, so structural hot patches keep remembered state from the first edit.

Cranpose 0.1.176 adds further text, glyph and frame-pacing performance work to
0.1.175's development-only `hot-reload` feature. Application release builds are
unchanged.

## Included since 0.14.2

Cranpose 0.1.175 is on crates.io with the development-only `hot-reload`
feature. New projects from **File → New Project → Cranpose** and **Create
Cranpose App** use it, so structural hot patches keep remembered state from the
first edit. Existing applications get the same behavior by moving to Cranpose
0.1.175. Application release builds are unchanged.

Live values now go through the compiler for literals that have not run yet.
Previously, after a hot patch, a click handler's number could be changed to a
value its type cannot hold: the preview silently kept the old number and no
error appeared. Now such values are compiled, so the compiler reports literals
that don't fit. Values that fit every type of their kind, and any value for a
literal that has already run, still apply without compilation.

Studio, the plugin template and the stability analyzer use Cranpose 0.1.175.

## Included since 0.14.1

Structural edits now keep remembered state. When an application uses a Cranpose
revision with the development-only `hot-reload` feature (Cranpose main from
this release's framework change, and released versions from 0.1.175), the
preview runner enables it in its private debug build. Composition is then
keyed by source structure rather than absolute lines, so adding a binding above
remembered state, or views before or after it or in another composable, is
hot-patched without resetting the count. Renaming a binding, changing a
condition or reordering same-named siblings starts those groups fresh.
Application release builds and the default feature set are unchanged.

In a local check on the counter fixture, four such edits reset the count to 0
with the previous Cranpose revision and kept it with the new one, in the same
process. CI now requires a structural edit to keep the fixture's count.

## Included since 0.14.0

Code edits no longer wait for a Restart. Changes inside function and method
bodies, including added, removed or reordered calls, statements, closures and
control flow, new functions and `use` changes, are hot-patched into the
running preview. Edits that need a new process (type definitions, signatures,
statics, `main`, modules or manifests) rebuild automatically after a short quiet
period and say why, for example "Rebuilding · Struct `Ink` fields changed".
The previous preview stays visible until the new one connects, and a failed
build recovers as soon as it is fixed. On a heavily loaded development machine,
adding a view to a composable reached the screen in about 0.75–1.2 s instead of
a 2.6–3.7 s manual restart. Adding a statement to a composable currently resets
that composable's remembered state; other composables keep theirs.

The Studio panel is redesigned: one compact toolbar, native IDE menus with
speed search and checkmarks, native tooltips, a framed device on a quiet
canvas, a status bar with live patch count and problems, a build-error drawer
and a cleaner inspector. The tool window, platform builds and Cargo run
configuration use the same visual language in light and dark themes.

Value knobs, color swatches, stability badges and composable-call underlines
are now painted by the editor itself, so they stay attached to the text while
scrolling and follow the editor font and theme. Live-value popups apply every
complete value as you edit: Apply, Reset, step and Toggle buttons are gone,
booleans use a two-state switch, and Undo in the editor restores earlier values.

Stability badges resolve primitives under glob imports, Cranpose and other
dependency types from their offline sources, and manual `PartialEq`
implementations. On five real Cranpose projects no parameter shows an
unexplained "unknown" badge any more; remaining unknowns name the type and the
reason. Tooltips show the resolved type.

## Included since 0.13.6

Clicking a live-value glyph now gives its already-open hover control keyboard
focus. Tab enters the field; Apply and Undo work without closing and reopening
the popup first. Hover itself leaves keyboard focus in the source editor, and
clicking the source still places the caret on the first click.

Uses the template's reusable Rust popup activation helper. No polling timer or
application release change is added. This is a focus correction, not a measured
performance improvement.

## Included since 0.13.5

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
