# Hover authoring — 0.9.0

Hover a live expression, diamond or color swatch to open its Cranpose controls.
There is no hover timer, and opening the popup leaves keyboard focus in the
editor. Click the control to interact. Escape or an outside click dismisses it.
The renderer is warmed once per project and reused; it shuts down with the
project. Each popup gets a fresh editing session, range and Undo gesture group.
Messages from old sessions cannot edit the newly selected value.

Immutable variables can expose the same controls as their live initializer:

```rust
let spacing = 24.0;
let tint = Color(0.19, 0.42, 0.31, 0.65);
let gap = spacing;
Text("Hello", Modifier::empty().padding(gap).background(tint), style);
```

Hovering `gap` tunes `24.0`; hovering `tint` opens the color picker. Uses stay
as variables. The control header identifies the binding and initializer line.
Explicit struct fields, tuples and destructured bindings are supported too.
The resolver respects scopes and shadowing. Mutable bindings, computed values,
borrowed values, unknown imports and arbitrary function returns stay outside
this scope. It exposes existing live initializers, not every runtime variable.
In particular, a palette returned by a branching helper cannot be resolved to
one color statically; its source constructors remain editable.

Inline color chips now display actual alpha over a checkerboard and transition
between color edits. Source navigation has a 900 ms spectral sweep with bright
rails and a fading trail. Popups enter with a finite glass shader, and numeric
seekbars use a gradient accent. Press and thumb animations remain responsive.
All motion settles; no perpetual animation loop is added.

The implementation is shared Rust in the template SDK. The UI is Cranpose.
Application release source and dependencies are unchanged.

## Validation scope

The Rust parser tests cover aliases, fields, tuple bindings, Unicode, shadowing,
and compiler-owned contexts. The native harness checks real pointer controls
at 1× and 2×, alpha pixels, precise colors, fresh ranges and request IDs across
five reused sessions, and zero settled animation frames. IDE regressions use
real editor offsets and mouse events to verify alias glyphs, hover reuse,
dismissal and stale-session rejection. Headless IDE tests do not present a
desktop popup; desktop behavior is checked separately in the licensed sandbox.

Local timing observations and final CI/desktop results are recorded with the
delivery report. Native composition timings include inspection polling and
exclude Swing, display presentation and application live-edit latency.
