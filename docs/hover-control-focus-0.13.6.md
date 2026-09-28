# Hover control keyboard activation in 0.13.6

An explicit click on a live-value glyph now activates its already-open hover
popup for keyboard input. Tab enters the first field. Clicking source dismisses
the control and places the caret with that same click. Merely hovering still
does not request keyboard focus.

## Cause and correction

The previous path called `requestFocusInWindow` on the Cranpose panel. IntelliJ
had created the hover window with native focus disabled, so this did not activate
the window. The shared Rust SDK now sets the popup focus request and defers native
window activation until editor mouse dispatch completes. It restores the window
focus flags, brings the window forward and uses `IdeFocusManager` to focus the
existing panel. Disposed or hidden popups are ignored. The glyph click is consumed;
ordinary source clicks are left to the editor.

The renderer, edit session and one-command Undo behavior are retained. There is
no new timer or application instrumentation. Framework and application release
dependencies are unchanged; Build's process SDK pin is aligned with the host.

## Validation and limits

- On the signed 0.13.5 macOS RustRover sandbox, clicking a hovered string glyph
  left Left-arrow input in the source editor.
- The corrected host passed one glyph click followed by Tab, typing, Tab and
  Enter: Apply changed `Source` to `Sourcex`; one Undo restored `Source`.
- A direct click from the open popup into `Source` placed the caret. Typing
  immediately changed it to `Sourxce`, and one Undo restored it.
- The shared IDEA test invokes the generated mouse listener. It checks hover's
  no-focus policy, explicit click activation and consumption, retained popup and
  renderer identity, stale edit rejection and unconsumed source presses.
- Local shared-host tests, Clippy and all nine template IDEA groups passed.

The headless tests exercise the IDE event path, not native OS window activation.
The interactive observation is macOS-specific. No latency, CPU, build-speed or
cross-platform desktop focus improvement is established.
