# Inspector work and overlapping Restart — 0.6.6

Studio retains an inspector snapshot when its layout content is unchanged. Request IDs and capture durations no longer cause a replacement view on every 500 ms poll. A shared sequence gate still rejects replies older than the newest accepted request, even when Cranpose discards an equal state update. Reconnects start a new sequence; changes to text, bounds, modifiers, sources and truncation still update the UI. Removed selected and collapsed IDs are pruned with a set lookup.

All implementation and measurement code is Rust. Shared delivery and profiling code lives in the template, pinned by Studio. The application framework, application release configuration and hot-reload runner behavior are unchanged in this iteration. Inspection requests now send the decimal ID required by Cranpose v2. The previous JSON-object payload decoded as ID zero; request numbers now advance independently of whether the layout changes.

## Results

The visible inspector's median CPU fell from **6.15% to 0.60% of one core (90.2%)**.
All six runs verified the initial and changed inspector text. Settled output frames
were zero in both versions. Hidden phases recorded zero or one 10 ms CPU tick,
0–0.05% of one core; no hidden-work improvement is claimed.

| Pair | Visible baseline | Visible candidate | Hidden baseline | Hidden candidate |
|---|---:|---:|---:|---:|
| 1 | 6.55% | 0.65% | 0.05% | 0.05% |
| 2 | 6.15% | 0.60% | 0.05% | 0.05% |
| 3 | 5.50% | 0.45% | 0.00% | 0.00% |

[Raw measurements](inspection-measurements-0.6.6.json) include timings, clock
resolution, frame counts, compiler version and measurement binary checksums.
Earlier exploratory measurements preceded the wire-ID correction and are excluded
from this table. The fixture uses a 1000 × 800 logical-pixel UI surface.

## Measurement scope

Local release builds used Rust 1.98.1 on an Apple M5 running macOS 27. Baseline is Studio 0.6.5; candidate adds the inspector change and the template sequence gate. The final candidate resolves the shared SDK from commit 5df0b1b980030caed2fa1707a035c8b964233691 and includes the wire-ID correction. Both use the same release optimization settings and framework revision.

The Rust `inspection-profile` tool starts an isolated Studio UI through its embedded protocol. It sends a fixed 1,000-node layout, then updates only request ID and capture duration every 500 ms. Each visible and hidden phase settles for five seconds before a 20-second observation. It checks displayed inspector text before measurement and after a real content change. Three before/after pairs run sequentially, without local compilation during sampling. The IDE and desktop remain open.

CPU is cumulative process CPU divided by elapsed time, expressed as a percentage of one core. macOS `ps` has 10 ms resolution here. These results exclude the application producing snapshots, the measurement host, transport work in the IDE/JNI host and unrelated IDE processes. They are not a whole-IDE idle result. Hidden zero ticks means below this observation's resolution, not proof of zero work. Both versions already suppress unchanged output frames.

## Restart investigation

The SDK's `hot-smoke --startup-only --restart-rounds` now keeps the old preview alive while compiling its replacement, requests a fresh response from the old preview, and verifies its runner/compiler/application exit afterward. Each replacement's private path, startup phases, Cargo time and shutdown evidence are retained. It waits through the replacement's first matching snapshot; the IDE switches at connection, so it is a reproduction of overlapping process ownership, not an exact IDE-action latency measurement.

The actual IDE Restart control was also exercised: the old runner, compiler and application exited, and the replacement rendered the gallery. The repeated fixture measurements use the same shared Cargo target and two alternating leased private paths. Cargo still rebuilds the application, with both support crates fresh. Attribution of the remaining delay specifically to shared incremental artifacts needs a controlled experiment. No startup or live-edit speedup is claimed in this iteration.

Seven verified overlapping replacements took 3.16–7.51 seconds (median 3.39 s),
with Cargo reporting 1.32–4.49 seconds (median 1.53 s). One slow final sample is
retained; the desktop and IDE were not isolated. The predecessor answered a fresh
request in 0.47–1.08 ms each time. Both support crates stayed fresh, the private
paths alternated, and every recorded process exited. [Raw Restart evidence](restart-measurements-0.6.6.json)
includes all samples. This uses the existing 0.6.5 runner, whose behavior is unchanged.

## Repeat and validation

`cargo run -p xtask -- inspection-profile --binary /path/to/studio-ui --nodes 1000 --seconds 20 --settle-seconds 5 --require-quiet --log inspector.log --report inspector.json`

`cargo run -p xtask -- hot-smoke --runner /path/to/cranpose-dev-runner --workspace /path/to/counter-fixture --cache /path/to/cache --log restart.log --startup-only --restart-rounds 7 --profile-startup --build-diagnostics --report restart.json`

Rust tests cover unchanged-model equality and shared snapshot identity, stale replies after discarded equal updates, reconnect resets, truncation changes and removed nodes. The shared harness also rejects queued replies from earlier requests. CI runs the inspector probe and two overlapping replacements alongside its existing live-edit, error recovery, lifecycle, native-target, actual IDE and Plugin Verifier checks.
