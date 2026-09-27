# Live design controls — 0.8.0

Click a numeric diamond to open a seekbar. Set Min, Max and Step, then drag to
update source and the running preview. A gesture shares one Undo group. The
range belongs to the open control; larger integers retain exact text input.

Four normalized float channels in `Color(...)` and `Color::rgba(...)` now have
one swatch, including colors returned by ordinary palette functions. The
Cranpose popup has hex input, hue/saturation/brightness/opacity sliders, a
shader checkerboard and Reset. A color edit updates the four channels together
while retaining source comments, suffixes and line breaks. Constants, named
colors, computed channels and RGB/u8 constructors retain compiler behavior.

Text inside `format!` is live when the fields remain unchanged. For example,
`format!("{count:02} observations")` can become
`format!("{count:02} little discoveries")` without compilation. Changing
`{count:02}` to `{count:03}` requires compilation. Raw strings, escaped braces,
captured names, positional/named arguments and their evaluation order are
covered by Rust compiler regressions. Other macros remain compiled.

Source navigation now paints a brief shader sweep at the destination line.
Toolbar and popup actions share rounded surfaces and finite press feedback;
slider thumbs also animate. These transitions use Cranpose and settle after
the interaction. The native test observed 34 arrival frames followed by zero
settled overlay frames over three seconds, and zero unfocused color-control
frames over two seconds at both display scales. This is an isolated UI check,
not a full-IDE CPU measurement.

## Validation and timing limits

The reusable implementation is template SDK
`6abe2a686af53b51b662b9652b699eb883c6ea75`. Studio exports the shared format helper
through its private runtime, adopts the shared action controls, and expands
the binary/library fixtures. No application release source or dependency changes
are required.

A local binary fixture completed six alternating unsaved updates combining
ordinary text, format text and palette color. The counter state, PID and disk
source stayed unchanged. Protocol send to changed color framebuffer receipt
was 32.0–57.7 ms, median 42.3 ms. Runtime generation acknowledgment was
35.2–60.8 ms; matching inspector snapshots were 237.6–267.3 ms with 200 ms polling.

These are functional measurements of the preview protocol on one local machine.
Frame capture copies the test surface. They exclude IDE parsing, its dispatch
interval, source writes, Swing presentation and the physical display. No
before/after editor latency, startup or compilation speedup is claimed, and they
do not establish that another project's three-second edit path is solved.

Repeat with the shared Rust `authoring-smoke` command for pointer/animation
checks, and `hot-smoke --fixture counter --live-values-rounds 6` (or `--fixture
library --source src/lib.rs`) for actual runtime updates and compiler recovery.
