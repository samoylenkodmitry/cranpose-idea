# New Project opens the first preview — 0.11.2

Creating a Cranpose project now saves and selects its desktop run configuration,
opens `main.rs` beside the preview, and starts the development build automatically.
The preview shows build progress while dependencies download and compile. There
is no need to find an entry point or run Cargo manually first.

The shared Rust SDK knows the bundled Showcase's desktop target before Cargo
discovery. Tests check that target against the pinned archive. Mobile targets and
robot binaries cannot become the initial selection. The saved configuration uses
the normal application target; preview instrumentation stays in the private
development build. Application release sources and dependencies are unchanged.

If Rust is missing, the Cranpose preview offers **Set up Rust** and **Retry preview**.
The setup button opens the official Rust installation instructions, including the
Windows C++ build tools prerequisite. After a standard rustup installation, Retry
finds Cargo without restarting the IDE. Creating files still needs neither Git
nor Rust; compiling requires Rust, its platform linker and downloaded dependencies.

Automatic launch happens only after successful New Project generation. Normal
project reopen and workspace refresh do not start applications. IDE trust checks
still apply. Stop and project closure use the existing bounded process cleanup.

Regression checks cover the pinned desktop manifest, Cargo discovery outside PATH,
persistent IntelliJ configuration selection, reuse without overwriting edited run
options, one initial preview request, missing-Rust retry and stale session replies.
