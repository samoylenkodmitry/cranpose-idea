# Desktop run status

During the installed 0.13.0 check, Showcase was visible and interactive while
Build & run still said **Launching application**. The build library emitted its
next stage only after the application's process exited.

0.13.1 consumes Cranpose Build 0.1.3. The shared Rust process SDK now exposes
`execute_observed`, which emits `Started { pid }` after successful OS process
creation and before output callbacks. Desktop launch turns that event into
**Application running**. Stop stays available until the job ends. Successful
exit, cancellation and failure retain their own results.

This is process status, not an application-readiness or first-paint guarantee.
Mobile deployment still reports **Application launched** after successful device
handoff; starting adb or xcrun is not starting the application itself.

The notification adds no timer or polling loop. Existing output-only SDK callers
use the same execution and cancellation path. Application source, dependencies
and release configuration are unchanged. No speedup or idle CPU improvement is
claimed.

## Regression coverage

- Shared SDK: exactly one start before output, no start for missing executables
  or pre-cancellation, cancellation from the start callback, and confirmed process
  exit after cancellation. Existing descendant-lifecycle tests still apply.
- Build library: real compiled executables cover success, nonzero exit, Stop,
  pre-cancellation, missing executable, and ordered launch/run/exit stages.
- Studio: output cannot overwrite running status; Stop remains enabled while the
  job runs; success, cancellation and failures release the controls.

RustRover diagnostics, formatting and Clippy accompany these checks. The SDK and
Build suites run on macOS, Linux and Windows in CI.
