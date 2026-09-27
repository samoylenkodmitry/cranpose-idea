# Color precision — 0.8.1

Opening a color control and clicking Apply now preserves its original float
channels. The hex label no longer feeds rounded 8-bit values back into source.
Opacity changes retain RGB, and hue/saturation/brightness changes retain alpha.
Reset restores the original channels and precision; Apply after Reset does too.

Newly generated channels use at most six decimal places, with a maximum rounding
error of 0.0000005 per normalized channel. This includes explicit new hex input;
each 8-bit channel still converts back to the same byte. Untouched channels keep
their original precision. HSV state is retained throughout a gesture, including
hue while the color is gray or black.

## Reproduction and validation

The new native unchanged-Apply regression fails against the exact verified
0.8.0 binary. Its input `0.19,0.42,0.31,0.123456789` becomes
`0.18823529411764706,0.4196078431372549,0.30980392156862746,0.12156862745098039`.
The fixed control submits the original input unchanged.

The same Rust harness checks opacity, hue, Apply after each gesture, Reset,
Apply after Reset, invalid partial hex and explicit hex at 1× and 2× scale.
For example, moving hue to 75% generates `0.305,0.19,0.42` while retaining alpha.
Model tests repeat 1,000 close/reopen cycles and check every 8-bit channel's
conversion. These are correctness checks, not edit-to-display latency results.

The local fixed native UI emitted zero settled frames from the unfocused color
control over two seconds and the source-arrival overlay over three seconds.
Text-field focus is explicitly released to exclude caret blinking. This small
fixture excludes the IDE and application; no CPU or latency improvement is claimed.

The implementation and regression tools are shared Rust/Cranpose SDK revision
`bb0ad53bab83c77aece9712a7848d9b9e2b4d0bc` from the plugin template. Application
release source, instrumentation and dependencies are unchanged.

Repeat with `cargo run --locked -p xtask -- authoring-smoke --binary
target/debug/cranpose-intellij-ui --log authoring.log --report authoring.json`.
