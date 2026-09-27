# Live authoring dispatch — 0.10.2

Eligible editor changes no longer wait for the 100 ms project timer to start
parsing and then wait for another tick to collect the result. The shared Rust
host queues authoring after a document change and wakes the IDE when parsing
finishes. Notifications coalesce, only one parser runs per project, and stale
document versions are still rejected. Parsing stays off the UI thread. The
existing project timer remains a fallback; no additional idle timer was added.

The worker posts through a reusable JNI callback and detaches from the JVM on
return. Disposal disables queued callbacks and releases their captures. A Rust
concurrency test covers coalescing and permanent closure. An actual IDEA test
posts 1,000 notifications from a native worker, verifies one deferred EDT
delivery, and verifies that a retained callback and a late worker cannot execute
after closure.

Studio pins template SDK `02d9e861125e67f2ad8ebf798467022e4e8cdf6a`.
Application release sources, instrumentation and dependencies are unchanged.

## Local IDE comparison

Two alternating baseline/candidate pairs used the licensed RustRover 2026.2.3
sandbox on macOS ARM, the same Comfortable gallery preview, editor file, renderer
and instrumentation. Each session inserted `x` into `FIELD NOTES` and undid it
three times, yielding six measurements. The IDE and preview restarted between
sessions. The baseline was the verified 0.10.0 native host; 0.10.1 did not change
this authoring path. The candidate replaced only the native host with a local
release build of the new dispatch code. Other desktop and IDE work remained
active. The second pair used closer input spacing than the first.

| Pair | Baseline median | Candidate median |
| --- | ---: | ---: |
| 1, six edits each | 218.99 ms | 112.12 ms |
| 2, six edits each | 233.81 ms | 84.15 ms |
| All 12 edits per version | **225.98 ms** | **106.20 ms** |

The combined median fell by 119.78 ms (53.0%) in this local comparison. All
24 edits reached a matching Text snapshot with one request each. Baseline range
was 155.58–280.60 ms; candidate range was 69.68–195.68 ms. No new ERROR/SEVERE log
entries appeared during any of the four edit sequences. Original text was
restored and saved after every session.

The trace starts at the IDE document-change callback and ends after a matching
preview frame/snapshot has been received and the lightning message has been
queued. It includes source analysis, glyph placement and transport, but excludes
final Swing painting and physical display presentation. This is a small local
comparison of one warm gallery, not a general editor-to-display guarantee.
It does not establish compilation, startup, idle CPU or large-file throughput
improvements, or resolve the user's separate three-second project. Faster
dispatch can parse more intermediate versions during continuous typing; the
single in-flight parser and queued-notification gate bound concurrency.

## Repeating the observation

Use the same sandbox and fixture for both hosts. Enable
`-Dcranpose.trace.edits=true` only for the observation, restart the IDE and launch
the same preview. Once ready, insert one character in a visible eligible Text
literal and Undo it three times. Collect the six new `CRANPOSE_EDIT_PRESENTED`
JSON records from `idea.log`. Repeat with the other host, then alternate again.
Check the expected record count, snapshot request counts, unchanged fixture and
fresh IDE errors. Remove the VM option afterward. Raw records for this run are
in `measurements/dispatch-0.10.2/`.

RustRover analysis of the new wake module reported no errors. Existing
`ensure!` macro-resolution diagnostics in authoring/project code appeared on
both unchanged main and the candidate; Rust compilation and strict Clippy passed.
