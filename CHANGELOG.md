# Changelog

## 0.14.4

- Value edits apply without compiling again; 0.14.2 and 0.14.3 compiled them
  whenever the file had a literal that had not run yet.
- The preview starts when the panel opens and shows what it is waiting for or
  building. A stopped preview stays stopped.
- No popups for strings and characters; they are edited in place.
- Plainer wording in the panel, popups and New Project wizard.

## 0.14.3

- Cranpose 0.1.176, plugin template and SDK 0.8.0, Cranpose Build 0.1.5 and
  the stability analyzer 0.2.0, all tagged releases.
- New projects use the Showcase 0.1.25 release on Cranpose 0.1.176.

## 0.14.2

- Cranpose 0.1.175 on crates.io offers the development-only `hot-reload`
  feature. The bundled Showcase starter and Create Cranpose App use it.
- Live values for a literal that has not run since its file was compiled are
  compiled instead of accepted unchecked, so values that do not fit report a
  compiler error.
- Studio, the plugin template and the stability analyzer use Cranpose 0.1.175.

## 0.14.1

- With a Cranpose revision that offers the development-only `hot-reload`
  feature, the preview runner enables it so structural hot patches keep
  remembered state around an edit. Release builds are unchanged.

## 0.14.0

- Structural edits inside function bodies hot-patch the running preview; edits
  that need a new process rebuild automatically with a specific reason.
- Redesigned Studio panel, tool window, platform builds and run configuration,
  with native IDE menus and tooltips.
- Editor glyphs, stability badges and call underlines are painted natively by
  the editor. Live-value controls apply edits as they are made.
- Dependency-aware stability analysis removes unexplained unknown badges.

## 0.13.3

- Rebuilding live-value markers in dense source files uses a guarded editor layout
  batch. Sparse files and markers at any caret keep ordinary insertion.
- Shared Rust IDE regressions cover Unicode positions, selections, multiple carets,
  callback cleanup and warmed comparisons across dense and sparse files.
- Cranpose Build 0.1.4 and Studio share the same process SDK revision.

## 0.6.1

- Live reload filters build and IDE noise before batching, coalesces repeated paths,
  and caps the wait during continuous changes. Incompatible edits stay pending;
  lost events and queue overflow require a restart instead of applying partial changes.
- Inspector state reads share immutable snapshots. Browsing the layout tree skips
  search-string construction, and unchanged viewport messages no longer trigger
  native layout and repaint work.
- The template owns reusable change batching, host-message deduplication and Rust
  reload timing tools. CI saves raw timing samples and exercises background build noise.


## 0.6.0 — Unreleased

- Searchable layout trees with ancestor context, branch folding, hierarchy navigation, pause and refresh.
- Responsive Layout/Details panels, selected-node contrast and clearer empty states.
- Rebuilt plugin UI reconnects to its existing application session.
- Shared Rust host, JVM bridge, build tools and UX models are owned by the original template.

## 0.1.0 — Unreleased

- Cranpose-powered workspace tool window with Cargo target discovery and build/run/test controls.
- Interactive target previews with rebuild on save, theme switching and layout inspection.
- Composable navigation, gutter markers, component completions and a starter app.
- Six-platform native packaging and a separate manual publishing workflow.
