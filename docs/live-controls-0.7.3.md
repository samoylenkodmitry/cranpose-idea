# Live-value pointer controls in 0.7.3

The floating text-selection menu could cover Apply and Reset. A real IDE check
reproduced the problem, and a Rust protocol regression failed against the exact
0.7.2 UI: clicking Apply with the menu visible submitted no source edit.

The shared Cranpose control now keeps its actions in a footer below that menu.
The popup is 340 × 240 logical pixels, and Apply uses the IDE accent color.
Text selection and clipboard actions remain available above the footer.

The template's `authoring-smoke` command verifies 16 action submissions across
1× and 2× scaling: string Apply/Reset, integer increment/Reset, float decrement/
Reset, and boolean Toggle/Reset. It opens the actual selection menu and clicks
controls using inspected bounds. The same check runs in template and Studio CI.
Geometry-aware UI probe helpers are shared with the inspector regression tools.

This iteration changes plugin UI and shared Rust checks. Application source,
framework revisions and release dependencies are unchanged. There is no startup,
live-edit latency or CPU improvement claim. The existing shader idle check still
requires zero settled frames; it does not measure the floating control or IDE.

Emoji font fallback remains an open UI issue. This change does not address the
separate missing-glyph observation.
