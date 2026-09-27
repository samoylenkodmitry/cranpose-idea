# 0.7.1: faster authoring catalogs in larger files

Studio now consumes the template SDK's source index. The authoring parser converts
function, call and literal positions to source bytes and IDE UTF-16 offsets without
repeatedly scanning the beginning of the file. This benefits the catalog work used
by editor glyphs, floating controls and private preview instrumentation.

Three alternating local release-build pairs, seven parses per case per run:

| Synthetic source | Before median | After median |
|---|---:|---:|
| 4 KB, 64 literals | 1.014 ms | 1.000 ms |
| 33 KB, 512 literals | 37.09 ms | 6.38 ms |
| 133 KB, 2,048 literals | 495.70 ms | 22.43 ms |
| 267 KB, 4,096 literals | 1,852.03 ms | 44.06 ms |
| 138 KB with Unicode, 2,048 literals | 491.75 ms | 24.25 ms |
| 214 KB, long Unicode lines, 4,096 literals | 996.73 ms | 39.30 ms |

Apple M5, macOS, rustc 1.98.1. The largest ASCII fixture fell 97.6%; the tiny-file
case showed no meaningful gain. Unrelated Java and Rust builds were active, so
these measurements are not a quiet-machine or cross-machine guarantee.

This measures `Catalog::parse`, including syntax parsing and schema hashing. It
does not measure IDE decoration placement, editor-to-display latency, compilation,
startup or idle CPU. The [0.7.0 live-value results](performance-0.7.0.md) are separate.

All measured catalogs match the pre-change baseline, including every byte/UTF-16
range, schema and literal identity. The template's ordinary Rust CI tests retain
those fixtures and exercise Unicode character boundaries, CRLF and long lines.
The change introduces no application release dependency or instrumentation.

The implementation, repeatable Rust benchmark and raw samples are in the
[template report](https://github.com/samoylenkodmitry/cranpose-intellij-plugin-template/blob/582e3951fe80d0d8d2a261a682fec9417cd57983/docs/performance-0.4.1.md).
