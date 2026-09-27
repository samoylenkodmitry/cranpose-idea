# 0.7.0: live literal authoring

Eligible literals now update a private debug value store without invoking the
compiler. Valid unsaved document changes use the existing preview connection;
saved changes use a private authenticated runner channel. Application release
source and dependencies are unchanged.

## Local counter comparison

On the same Apple Silicon macOS host, two sessions per version, seven saved edits
per session. The baseline is the retained final 0.6.8 runner; the candidate is the
0.7.0 authoring runner. Both use the same counter source and separate Cargo caches.
Order: candidate, baseline, baseline, candidate. Edits happen after initial
compilation. IDE interaction occurred during the first pair; the second pair ran
without interaction. These are small local samples, not a cross-machine guarantee.

| Median across 14 saved edits | 0.6.8 | 0.7.0 |
|---|---:|---:|
| Save → runtime acknowledgment | 671.7 ms | 117.5 ms |
| Save → matching inspector snapshot | 877.7 ms | 321.6 ms |

Acknowledgment latency decreased 82.5%. The harness polls snapshots every 200 ms;
the second row includes that observation delay and is not a display frame time.
Every edit checks a fresh generation, the expected label, remembered count and
unchanged application PID. Both versions passed compiler-error, invalid-syntax
and state-type recovery, and every owned runner/compiler/application exited.

Twelve unsaved updates, sent directly over the IDE protocol, took 22.9–53.3 ms
to acknowledge (median 42.0 ms) and 229.4–258.7 ms to observe a matching snapshot
(median 247.1 ms). They varied string lengths and left disk source unchanged.
This excludes document parsing and the IDE's 100 ms dispatch timer. Actual editor
typing and the Cranpose popup were also exercised in RustRover, without claiming
precise end-to-end timings from UI automation.

The user's approximately three-second observation involved a different project.
This fixture does not establish that project's before/after timing. Structural
Rust changes still use compilation or Restart. No startup, cold-build, full-IDE
idle CPU or arbitrary-code reload speedup is claimed.

The prior 0.6.8 generation-2 acknowledgment without the expected label did not
recur in these runs. This is not proof that the intermittent compiler-path issue
is fixed; CI continues checking both acknowledgment and displayed contents.

## Editor shader rendering

The real native protocol fixture renders a diamond, call underline and stability
badge on a 640 × 400 transparent overlay: 2,576 painted pixels and zero frames
during a three-second settled observation. The local process CPU sample rounded
to zero at the platform clock's resolution. This small fixture excludes the IDE,
application preview, document parsing and large editor files.

Geometry is invalidated by source, viewport, inlay, folding and font events. The
shader itself is static and has no animation loop. Native IDE tests check that
transparent overlay painting preserves existing editor pixels.

## Reproduce

```sh
cargo build --locked -p cranpose-dev-runner
cargo run --locked -p xtask -- hot-smoke --runner target/debug/cranpose-dev-runner --fixture counter --reuse-fixture --cache /tmp/cranpose-authoring-cache --measure-rounds 6 --live-values-rounds 6 --log authoring.log --report authoring.json
cargo build --locked --no-default-features -p cranpose-intellij-ui
cargo run --locked -p xtask -- authoring-smoke --binary target/debug/cranpose-intellij-ui --log overlay.log --report overlay.json
```

The shared Rust harness and authoring SDK live in the template. CI retains these
checks, including binary/library recovery, unchanged disk source and process exit.
Raw samples are in [authoring-0.7.0](measurements/authoring-0.7.0/).
