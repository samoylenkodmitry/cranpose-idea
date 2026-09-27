# Live-edit lightning — 0.10.0

Editing visible text now sends a brief lightning arc from the source to the
updated Text view in the preview. The white core has a cyan/violet halo and a
small fork; an impact outline identifies the destination. Everything is drawn
in Cranpose by reusable Rust code in the template SDK.

The arc starts after the host receives a preview frame and confirms both the new
rendered text and its application source call in the layout snapshot. String
initializers, resolved local aliases and the text in `format!` are supported.
It does not guess at duplicate views, offscreen endpoints, different IDE windows
or incomplete source. Each new edit replaces pending work and cancels an older
effect. Numeric/color tuning retains its existing controls and animations.

The overlay passes mouse input through, renders only the area between its
endpoints, finishes in 850 ms, and hides after settling. It adds no idle snapshot
loop. Each pending edit permits one snapshot in flight, eight attempts, and a
ten-second lifetime. Source remains authoritative; application release builds,
instrumentation and dependencies are unchanged.

## Verification

The shared Rust native harness checks the source-to-view arc and destination
pixels at 1× and 2×, final transparency and zero settled frames. Unit tests cover
Unicode, aliases, format fields, source identity, stale replies, duplicate views
and request budgets. Actual IDE tests check Swing coordinate conversion and
pointer passthrough. Existing preview, inspector and hot-reload suites remain
required in CI.

The first local shader observation produced 39 frames at 1× and 42 at 2×,
then zero settled frames over 3.12 seconds. The native process used 0.10 and
0.16 CPU seconds in the respective 1.58/1.59-second observation windows. These
are transient feature costs from one local release run, not a before/after
performance comparison. They exclude the IDE and measurement process, including
PNG capture. Application updates and compilation were not timed by this harness.

For repeatable actual IDE observations, start a sandbox with
`-Dcranpose.trace.edits=true`. Successful matches write `CRANPOSE_EDIT_PRESENTED`
to the IDE log, including `editToMatchedFrameMs` and request count. That interval
includes the document callback, source analysis, frame receipt and snapshot
confirmation; it ends before final Swing/display presentation. The existing
100 ms authoring timer is unchanged, and no latency improvement is claimed.
