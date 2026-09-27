# Studio 0.6.7: compose the visible inspector rows

Large layouts now compose the visible rows plus two rows of overscan on each
side. Fixed 28-pixel rows use single-line ellipsis; the details panel retains the
full selected text. Spacers preserve the full scroll extent. Collapsing a tree
while scrolled now clamps its stored offset, so expanding it again shows the
first visible row instead of restoring a stale position.

The reusable Rust `viewport::RowWindow` and measurement tools live in template
commit `85ca6d831c6a5cb6fa6aece9eb3b8fae033d3bd1`. Studio pins that SDK revision.
No Cranpose framework or application source/build configuration changed.

## Measured update cost

Three local before/after pairs per size change every node label twice a second
for ten seconds. The CPU sample sends no UI-inspection requests; the final label
is verified afterward. Each sample produced 20 frames for 20 updates.

| Layout nodes | Studio 0.6.6 CPU | Studio 0.6.7 CPU | Reduction |
|---|---:|---:|---:|
| 1,000 | 7.60% | 1.30% | 82.9% |
| 5,000 | 27.17% | 1.80% | 93.4% |

CPU is the median percentage of one core used by the isolated Studio UI process.
It excludes the application producing snapshots, the IDE/JNI host and the
measurement process. macOS CPU accounting has 10 ms resolution. These are large
synthetic changing layouts, not whole-IDE idle or application reload results.

At 1,000 layout nodes the UI snapshot shrank from 4,059 nodes to 149. At 5,000,
the baseline UI snapshot hit its 10,000-node limit; the candidate still had 149
nodes and was not truncated. A 200-node budget is now enforced in CI at the
1000×800 logical viewport.

## Response timing and limits

The same probe also records initial and changed-layout acknowledgements through
the UI's own inspector. These include JSON serialization, transport, rendering,
UI snapshot capture and 20 ms acknowledgement polling. They are not frame times
or exact user-input latency. Each run changes all labels eight times; the table
uses the median of those per-run medians for changed layouts.

| Layout nodes | Initial acknowledgement before → after | Changed acknowledgement before → after |
|---|---:|---:|
| 1,000 | 113.2 → 67.7 ms | 80.0 → 69.9 ms |
| 5,000 | 489.2 → 147.1 ms | 311.2 → 131.8 ms |

The smaller UI tree also makes these measurement acknowledgements cheaper. Use
the separate streaming CPU interval above to assess normal update work without
that observer cost. Initial acknowledgement begins when the fixture sends its
first layout to an already connected UI; it is not plugin or preview startup.

Both versions emitted zero frames during the short unchanged visible/hidden
checks. No idle, build, startup or live-edit speedup is claimed here.

## Reproduction and validation

Apple M5, macOS 27.0, Rust 1.98.1. Both binaries use `--release
--no-default-features`, identical optimization settings and the same framework
revision. No local compilation ran during the measurements. The desktop, IDE
and existing demo remained open; this is not an isolated system. Pair order was
before/after, after/before, before/after for each size. No samples were excluded.
[Raw data and binary hashes](inspection-measurements-0.6.7.json) retain all runs.

```text
cargo run -p xtask -- inspection-profile --binary /path/to/studio-ui   --nodes 5000 --seconds 3 --settle-seconds 2 --require-quiet   --change-rounds 8 --change-seconds 10 --exercise-tree --max-ui-nodes 200   --log inspector.log --report inspector.json
```

Omit the last two flags for the 0.6.6 baseline: it exceeds the new node budget
and has the old collapse-at-bottom behavior. `--seconds` controls the short
unchanged CPU check; `--change-seconds` controls the changing-layout CPU sample.

Shared Rust tests cover partially visible rows, scroll boundaries, content
shrink, million-row ranges and invalid dimensions. All six candidate GUI probes
passed final-row reachability, collapse at the bottom, expansion, return to the
first row, filtering for the last node, selection details/source availability
and clearing the filter. Every probe terminated its process tree.

Local Studio tests (40), template workspace tests, formatting and Clippy passed.
RustRover reports no errors in the changed Studio UI or shared viewport helper.
Its existing `anyhow::ensure!` resolution errors in the profiling module remain;
the Rust compiler and Clippy accept that module. CI also runs the interaction
probe, native IDE integration, lifecycle, hot-reload and Plugin Verifier checks.
Application release builds remain unaffected. No Marketplace publication.
