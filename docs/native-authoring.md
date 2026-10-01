# Native Rust UI authoring

Studio's primary authoring path is the native compiler runner. Applications keep
ordinary Rust source, their normal Cranpose APIs, and ordinary `#[composable]`.

## Update contract

- Eligible literal edits use the existing value bridge without compilation.
- Compatible function-body edits, new functions and imports go through rustc and
  Subsecond. Modifier chains, user extension methods, custom composables, generic
  helpers, closures, branches and loops do not need runtime registrations.
- Edits saved during a native patch are coalesced by file. After that patch succeeds
  or reports a compiler error, the latest source is classified and submitted. A
  correction must not require another save to wake the compiler.
- Type layouts, signatures, statics, traits, macro definitions, modules, manifests
  and the entry point require a new process. Changing a remembered value's type
  also requests a rebuild. Incompatible state resets on rebuild.
- Invalid syntax and compilation errors retain the last successful preview.

The runner copies and instruments the workspace privately. Literal schemas belong
to the last source written to the compiler, not to the latest value-only edit.
They are cached lazily and replaced with each compiled source revision. The parsed
catalogue for a value edit is reused when sending the update.

This supports Rust's language and native APIs through the compiler; it does not
promise that every Rust change can patch a running process. Explicit and qualified
composable attributes receive hot-call boundaries. Attribute aliases and
macro-generated declarations are not instrumented. External path dependencies and
symlinked workspaces retain the limitations described in the README.

## Evidence

Local measurements on an Apple Silicon Mac, 2026-10-01, Dioxus/Subsecond 0.7.10:

| Measurement | Observed |
| --- | --- |
| Baseline counter: saved literal to runtime acknowledgement, 7 updates | 95–149 ms |
| Baseline counter: structural edit to inspector snapshot | 2,358 ms |
| Native fixture before runner optimization: modifier, composable and callback edits | 807–872 ms |
| Native fixture with queued saves and cached schemas: same three edit categories | 777–891 ms |
| Sample gallery: classification and catalogue preparation, 200 edits, debug runner | 14.335 → 4.819 ms/edit |

The fixtures differ: these are not before/after structural build comparisons.
The analysis measurement covers a 9,913-byte sample and excludes watching,
transport, compilation and rendering. Its first iteration includes the cache fill.
The baseline inspector polled every 200 ms; the native test polls every 25 ms.
Neither measures pixels presented on a physical display. Cold startup was 87.5 s
with dependency compilation and is separate from edit latency. No subsecond
guarantee follows from these samples.

The [final native report](measurements/native-authoring/native-rust.json) also
records compiler-error recovery at 644 ms and the subsequent literal edit at
160 ms, including snapshot polling. A regression using a deterministic compiler
fixture fails when the edit queue guard is disabled and passes when it is restored.

The native acceptance test checks rendered padding movement, adding a composable,
editing a callback, generic formatting, extension methods, branches and loops. It
clicks the application to verify state and callback behavior, recovers immediately
from a compiler error, reverts code, applies a literal edit after structural
patches, verifies rebuild requests for a state type change, and checks process
shutdown. The test runs on macOS and Linux in the hot-reload CI job.

Reproduce from this checkout:

~~~sh
CRANPOSE_NATIVE_SMOKE_CACHE=/tmp/cranpose-native-rust-smoke \
  cargo test --locked -p cranpose-dev-runner --test native_rust -- --ignored --nocapture
cargo test --locked -p cranpose-dev-runner --test edit_queue
cargo test --locked -p cranpose-dev-runner --lib profile_saved_values -- --ignored --nocapture
~~~

The native test writes `native-rust.log` and `native-rust.json` inside its cache.
Its renderer, compiler and runner are scoped to the test session.

## Further measurement

Measure the consuming application before further latency work, especially large
screens and slow machines. Build-profile or compilation-partition changes need
native state, callback and recovery tests alongside edit-to-preview measurements.
