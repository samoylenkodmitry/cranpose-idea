# Live editor anchors — 0.10.3

Changing an eligible literal now retains its editor glyphs when the parsed source
structure, literal IDs, glyph count and current UTF-16 positions still match.
Previously each successful parse disposed and recreated all of them, up to
1,024 glyphs plus preview gutters. IntelliJ already tracks their positions as
text changes, so keeping valid anchors avoids repeated editor layout work and
native callback allocation.

The Rust host checks each anchor's validity and position. Structural changes,
missing anchors and unexpected offsets rebuild the markers using the existing
path. Invalid source clears them. The new catalog and Cranpose shader geometry
still update, so color swatches show the edited color and hover controls use the
current source. Disposal uses the same tested cleanup path for retained anchors.

The reusable implementation and actual IDEA regressions live in template SDK
`a31ebc73f65be3db0b6bdc067dff39a5b7276cb3`. All authored implementation and tests
are Rust. Application release sources, instrumentation and dependencies remain
unchanged. No new parser, timer or background polling was introduced.

## Validation and repeatable profiling

The actual IDEA suite verifies retained callback identities after edits that
alternate ASCII and supplementary Unicode, grouped color edits with references,
invalid-anchor recovery, structural preview-action changes and disposal.
It also checks each resulting offset and reserved glyph width.

The same Rust harness compares forced replacement (the previous algorithm) with
reuse at 32, 256 and 1,024 glyphs. It alternates order within a single IDE process,
uses two warmup pairs and records seven pairs per size. Timings are written to
`authoring-placement.json` beside the normal `results.json` evidence. CI checks
object reuse and correctness without requiring a fixed machine-dependent time.

To run with an optimized host, without the normal debug JNI checking overhead:

```sh
cargo run --locked -p xtask -- ide-test --profile --ide /path/to/idea
```

Omitting `--profile` retains the normal checked debug suite. Both modes use a
separate headless IDE profile, run the same integration assertions and create
test-only plugin packages. These packages must not be distributed.

The timed interval covers glyph validation or replacement after a document edit.
It excludes parsing, document writes, live transport, Cranpose rendering, Swing
painting and physical display. It does not establish whole-editor latency,
compilation, startup, idle CPU or continuous-typing throughput improvements.
Full replacement still applies to structural edits, and invalid intermediate
source still interrupts glyph reuse. The separate three-second project remains
unmeasured.
