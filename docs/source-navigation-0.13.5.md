# Source navigation from generated previews

On the signed 0.13.4 package, picking Mercury's description in Showcase selected
the correct Text and displayed `detail_screen.rs:59`, but neither Pick nor the
source button moved the editor. The problem was also observed before 0.13.4;
this investigation does not establish its first affected version.

The emitted source request contained a nonexistent project path under
`.cranpose-dev/launcher/src/screens/`. Cargo reported a workspace-relative
`file!()` alongside the generated package's `CARGO_MANIFEST_DIR`. Appending one
to the other duplicated the launcher directory before source mapping.

Studio now uses the template's shared Rust source-path resolver. It handles
workspace-relative, package-relative and absolute locations, with the most
specific directory mapping taking precedence. It does not scan by filename or
change application dependencies or release builds.

## Regression coverage

- The Studio model test uses the observed generated-launcher source shape and
  source line. It fails on 0.13.4 and passes with the resolver.
- The shared SDK compiles a real offline Cargo library and generated binary,
  then verifies both compiler locations resolve to editable files.
- Shared unit tests cover both relative forms, absolute paths, nested mapping
  order, ordinary workspace members and external sources.
- In the licensed RustRover sandbox, the candidate renderer opens
  `list_screen.rs:223` from the source button and `detail_screen.rs:59` from Pick.
  The source-arrival shader runs and the application keeps the same process.

Source tabs now share the workspace that initiated navigation. Moving its native
component into the destination tab preserves the application and controller.
Returning to a linked tab mounts that same workspace. Closing the origin keeps it
alive while another tab uses it; closing the last owner releases it. Existing
destination sessions remain owned by their original tab.

Pick and Inspect are independent. Closing Inspect expands the preview while Pick
stays enabled. A fresh snapshot is required after reflow; delayed replies remain
fenced. With both off, snapshot polling stops. The shared native fixture checks
these transitions, and actual IDEA integration checks mounting and disposal.

The same authoring correction disables IntelliJ's consuming outside-click popup
dismissal and leaves source mouse presses to normal caret handling. Popup bounds
avoid the source line. `Color::rgb`, `Color::from_rgb_u8` and
`Color::from_rgba_u8` use grouped color controls and inline swatches. Byte edits
round to valid channels and preserve numeric base, suffixes and comments. RGB
constructors remain opaque; RGBA exposes opacity. Compiled Rust regressions verify
generated development instrumentation and runtime updates.

This is a navigation correctness fix, with no performance claim.
