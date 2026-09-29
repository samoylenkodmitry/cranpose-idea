# Live editing, native decorations and Studio redesign — 0.14.0

## Structural edits

The development runner compares each changed file item by item, with function
bodies erased, against the source last compiled into the running preview.

- Hot-patched in place: any function or method body edit, new or removed free
  functions and inherent methods, and `use` changes. Doc comments, lint
  attributes and test-only items are ignored.
- Rebuilt automatically, with a specific reason: struct, enum and union
  definitions, type aliases, statics, consts, traits, `macro_rules!`, impl
  headers, trait-impl method sets, function signatures and attributes, `main`,
  `const fn`, modules, manifests and build scripts. A body edit that only
  changes a literal's type (for example `0_i32` to `0_i64` in remembered state)
  also rebuilds.

The runner requests a rebuild after 400 ms without further edits; a syntax
error or reverting the edit withdraws the request. Studio starts the
replacement session when reload on save is on and keeps the previous preview
visible until it connects. dx patch failures, and a first build that failed and
was then fixed, also trigger a rebuild instead of leaving the preview stale.

Measured on a development machine with load averages of 12–41, save to new
content rendered:

| Case | Manual restart before | Now |
| --- | --- | --- |
| Fixture, add a `Text` call | 2.6–3.6 s | 0.75–0.77 s, same process |
| `samples/counter`, add a `Text` | 3.2–3.7 s | 1.12–1.17 s, same process |
| `samples/counter`, add a struct field | 3.2–3.8 s | 3.6–3.7 s, automatic |
| Failed struct change, then fixed | stuck until Restart | 4.3 s after the fix |

The manual numbers exclude the time to notice the message and press Restart.
With Cranpose 0.1.173, adding a statement or call inside a composable resets
that composable's remembered state: branch group keys are numbered through the
whole function, so the body's group key moves. Expression-only edits keep
state, as do other composables. This is memory-safe; the slot table replaces
mismatched payloads.

## Editor decorations

Value knobs, color swatches and stability badges are IntelliJ inlay renderers
that paint with Java2D; composable-call underlines are range highlighters. They
move with the text in the same frame. The transparent Cranpose overlay now only
draws transient effects (source arrival and lightning). IDE tests paint the real
renderers into an offscreen image and save `editor-decorations.png`.

Live-value popups apply each complete value as it is entered. Typing shares one
Undo group per popup, each drag has its own, and boolean choices are separate
steps. Incomplete input is held with an inline reason.

## Studio panel

Menus are native IDE list popups opened at the toolbar control, so they can
overlap the running application; icon controls set native tooltips. The IDE
suite covers the popup step, checkmarks, separators and the reported choice.

## Verification

- Studio: workspace fmt, clippy with CI flags and tests; hot-smoke counter,
  restart isolation and library scenarios; authoring, choice, dashboard and
  inspection-profile smokes against the redesigned UI.
- Template: workspace clippy with CI flags, unit tests and all IDE integration
  groups on Android Studio (IntelliJ Platform 261), including native glyph,
  badge and menu checks.
- Stability analyzer: 67 tests, and before/after counts on five real projects.

These are correctness checks and single-machine observations, not controlled
performance comparisons.
