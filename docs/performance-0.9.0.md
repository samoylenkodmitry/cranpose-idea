# Hover controls and source analysis — 0.9.0

Controls now reuse one native renderer per project. The host starts it after
finding an eligible source catalog and opens popups without a hover timer.
The same renderer survives dismissal and changing between number and color
controls. Session identities prevent stale messages from editing a new value.
Keeping a renderer warm trades one resident process per project for avoiding
process startup on each open. No full-IDE memory or idle CPU improvement is claimed.

## Native control observations

One local release-build run of the shared Rust `authoring-smoke` harness tested
five reused sessions at each display scale:

| Scale | Process start to first control composition | Reused control composition, median | Reused range |
| --- | ---: | ---: | ---: |
| 1× | 201.3 ms | 41.4 ms | 40.5–44.4 ms |
| 2× | 60.0 ms | 45.1 ms | 43.7–73.2 ms |

These are different operations, not an A/B of complete IDE hover latency.
Composition is observed through 20 ms inspector polling with a 10 ms receive
loop. Times exclude Swing popup construction, display presentation, source
parsing and application preview updates. The run also checks fresh number
ranges and correct request IDs across reused sessions.

The editor overlay produced 36 source-arrival frames, then zero frames over a
3.12-second settled observation. The unfocused color control produced zero
settled frames over two seconds at each scale. Alpha was verified using opaque
and transparent swatch pixels. These observations cover isolated native UI,
not the IDE host or application. The sample used SDK `ad11773`; the later
arrival-opacity adjustment changes its appearance, not its duration.

## Additional parser work

Resolving variables adds an AST clone and a lexical visit. Three alternating
local release-build pairs ran seven parses per fixture. Below are medians of
the three per-run medians:

| Source fixture | Before | With reference analysis |
| --- | ---: | ---: |
| 4,118 bytes | 0.43 ms | 0.44 ms |
| 33,170 bytes | 3.38 ms | 3.61 ms |
| 133,034 bytes | 14.22 ms | 15.39 ms |
| 267,178 bytes / 4,096 literals | 30.60 ms | 32.85 ms |
| 138,154 bytes / Unicode | 14.36 ms | 17.01 ms |
| 213,942 bytes / single-line bodies | 26.31 ms | 29.54 ms |

The baseline is template main `48978d3`; the candidate uses the new resolver.
All six catalog hashes match, since these compatibility fixtures have no new
resolvable local bindings. Separate regressions cover variables, aliases,
fields, tuples, Unicode, shadowing and excluded contexts. This does not measure
worst-case reference density, IDE inlay placement or editor-to-display latency.

The recorded pairs are runs 7–9. Earlier exploratory runs overlapped native
builds or IDE startup and are excluded. RustRover and the isolated preview
remained open in the recorded runs; the observations are not a CPU-isolated
benchmark. Raw samples are in `docs/measurements/hover-0.9.0`.

Reproduce with `cargo run --release --locked -p cranpose-plugin-authoring
--example catalog_profile -- 7` in each template revision, alternating binaries
after both builds finish. Native controls use `authoring-smoke --binary <ui>
--log <path> --report <path>` from the shared Rust tools.
