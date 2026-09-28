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
change source instrumentation, application dependencies or release builds.

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

The preview is still owned by its original source tab. Navigating to another
tab shows that tab's idle preview; returning to the original tab restores the
running preview. Keeping the active preview visible across source tabs is a
separate UX follow-up, not fixed in this version.

This is a navigation correctness fix, with no performance claim.
