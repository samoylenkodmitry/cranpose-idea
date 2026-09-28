# Editor marker replacement in 0.13.3

Studio uses the template's Rust inlay batch wrapper when a structural source
change requires a large, dense set of live-value markers to be recreated.
The automatic path requires at least 256 glyphs, at most 64 UTF-16 document
units per glyph, and no glyph at any primary or secondary caret offset.
Literal-only changes continue to reuse their anchors.

## Measured result

Three fresh headless IDEA sessions on Linux compared ordinary insertion with
the automatic policy in the same optimized host. Pooled warmed medians:

| Glyphs | Source size, approximately | Ordinary | Automatic | Observation |
|---:|---:|---:|---:|---|
| 256 | 5.8 KB | 13.99 ms | 12.14 ms | 13.2% lower |
| 1,024 | 23.5 KB | 52.99 ms | 44.48 ms | 16.1% lower |
| 256 | 15.9 KB | 12.91 ms | 11.26 ms | 12.8% lower, near size boundary |
| 1,024 | 63.8 KB | 51.40 ms | 44.01 ms | 14.4% lower, near size boundary |
| 256 | 845.8 KB | 13.19 ms | 13.15 ms | Ordinary path; no gain claimed |
| 1,024 | 863.5 KB | 52.51 ms | 52.29 ms | Ordinary path; no gain claimed |
| 32 | 0.75 KB | 1.70 ms | 1.69 ms | Ordinary path; no gain claimed |
| 128 | 2.9 KB | 8.76 ms | 9.88 ms | Ordinary path; 1.12 ms higher observed |
| 32 | 840.8 KB | 1.84 ms | 2.06 ms | Ordinary path; 0.21 ms higher observed |
| 128 | 842.9 KB | 6.61 ms | 6.56 ms | Ordinary path; no gain claimed |

The dense 1,024-glyph session medians were 52.32 → 43.52 ms,
52.40 → 44.60 ms and 54.03 → 44.69 ms. Small controls varied despite both paths
selecting ordinary insertion. Mixed-mode ordering and JVM variation were not
isolated further; these observations do not establish their cause or promise
zero overhead for every file.

Forced batching of sparse 863.5 KB files took 72.29 ms versus 52.51 ms for
ordinary insertion. This demonstrated regression is why the document-size
guard is part of the implementation. A count-only policy was rejected.

## Method and limits

- Template production source: `8a3247b8cb21b74e7ac4a96f1ba53967310091fe`.
  Measurements used a local path override of that source in Studio 0.13.2's
  isolated Linux checkout. The final 0.13.3 dependency pins also share one process
  SDK with Cranpose Build 0.1.4.
- SSH host `samarch`; IDEA 2026.3 build 263.5701.42; Rust
  `1.96.0-nightly (b90dc1e59 2026-03-04)`; release optimization with LTO.
  The test process and its children were limited to logical CPUs 0 and 1.
  This is an experiment setting, not a shipped affinity policy.
- Ten fixtures, ordinary/forced/automatic/reuse modes interleaved, nine edits
  per fixture with the first two excluded. Each session ran three fixture rounds,
  reversing order in the middle round. Round zero is retained in the evidence
  but excluded from the warmed table. Each cell contains 42 observations:
  seven edits × two warmed rounds × three sessions.
- Unicode edits occur before each timed replacement. The interval includes
  marker disposal and insertion. Parsing, document writes, preview transport,
  application work, Swing painting and physical display are excluded. The
  benchmark forces replacements; it is not an ordinary keystroke latency test.
- The fresh original 0.13.2 baseline's 1,024-glyph median was 179.46 ms. Early
  results were affected by IDE warm-up and a different fixture sequence, so that
  number is excluded from the improvement calculation.
- No startup, compilation, idle CPU, normal literal-edit or exact editor-to-display
  speedup is claimed. This does not establish a fix for the separately reported
  three-second project.

All three final sessions passed the 12 actual IDE integration groups. The new
regressions verify UTF-16 offsets, glyph widths, retained anchors, primary and
secondary caret handling, selection positions, and callback/capture cleanup
after success, Rust errors and failed JNI lookup. They also assert that sparse
files avoid automatic batching.

## Reproduce

```sh
CARGO_BUILD_JOBS=2 taskset -c 0,1 cargo run --locked -p xtask -- \
  ide-test --profile --ide /path/to/idea
```

Omit `taskset` on other platforms. Use the reported evidence directory's
`authoring-placement.json` and `results.json`; do not compare checked-debug CI
timings directly with optimized timings. The template owns the Rust fixture
and [batching API documentation](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template/blob/8a3247b8cb21b74e7ac4a96f1ba53967310091fe/docs/editor-inlay-batching.md).
