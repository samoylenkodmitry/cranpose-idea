# Changelog

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
