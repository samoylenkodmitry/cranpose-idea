# Live feedback and keyboard editing — 0.11.0

Hover the diamond or color swatch to open a live control. Hovering the source
value leaves the editor clear for placing the caret and selecting text. Clicking
source text dismisses an open control; clicking its glyph focuses the control.

Eligible live edits now charge a small cyan/violet constellation beside the edit.
The host waits for exact revision acceptance, a frame received after acceptance
and composition confirmation. Frames and queued composition messages can arrive
in either order. A bounded search then finds the affected view. A confirmed match releases
lightning, sparks and an expanding outline. The transparent Cranpose overlay
passes pointer input through. Rejected or superseded edits cancel it, and a
ten-second deadline bounds pending animation and matching work.

Array text can now be followed through immutable bindings, tuple destructuring,
`for`, `iter`, `into_iter` and `enumerate`. New text and source provenance must
both match. Loop variables do not acquire an arbitrary single-element control.
Numbers, colors and booleans can target a unique component call through direct
arguments or supported immutable aliases. Repeated indistinguishable views,
unknown data flow, offscreen targets and truncated inspection trees are skipped.
Compiler-only edits retain the existing build behavior.

All source analysis, host integration, Cranpose effects and test tools are shared
Rust SDK code from the template. Studio adds exact update identities and a
composition side effect to its private debug preview runtime. Application release
sources and dependencies are unchanged. No new polling loop or timer is added.

## Validation

- Rust regressions cover array flow, lexical shadowing, aliases, typed values,
  duplicate views, stale revisions, rejected updates and bounded requests.
- Real IDE integration checks source hover, glyph hover, dismissal and stale
  popup callbacks.
- Native checks at 1× and 2× verify pending pixels, cancellation, arrival pixels,
  transparent completion and settled frame counts.
- Binary and library live-reload checks require both matching acknowledgment
  identity and composition generation, while retaining PID, remembered state,
  unsaved source behavior and compiler recovery.

These checks establish behavior, not a before/after latency or CPU improvement.
Frame receipt and composition acknowledgment are not physical display timing.

Windows authoring checks run on CI and can be repeated with its `windows-test-tools`
artifact. The shared Rust SDK reads Windows process CPU time through
`GetProcessTimes` with query-only handles. Counters use 100 ns units but OS
accounting granularity is coarser; zero observed frames does not imply zero CPU.
