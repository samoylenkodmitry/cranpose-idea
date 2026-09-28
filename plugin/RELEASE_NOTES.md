# Cranpose 0.13.1

Desktop Build & run now shows **Application running** after the OS creates the
process, instead of leaving **Launching application** visible until exit. Stop
remains available. Failed spawn, failed exit and cancellation keep their own
results; mobile deployment retains its device-handoff status. Running is process
status, not a first-paint or readiness guarantee.

Uses Cranpose Build 0.1.3 and the shared template's process-start notifications.
No polling timer or application release change is added.

## Included since 0.13.0

Build application packages locally for desktop and mobile from the Cranpose panel.

- Choose macOS, Linux, Windows, Android or iOS and development or release packaging.
- Check tools, set up pinned cross-compilation backends, inspect build plans, list devices and launch applications on compatible runtimes.
- Builds run in a cancellable background job with bounded output. Stop or closing the project terminates owned build processes.
- Uses the standalone Rust Cranpose Build library, also available from its public repository and native CLI releases.

Cross-compilation depends on the target SDKs and native libraries. Foreign desktop applications need their matching OS or VM to run; iOS packaging requires macOS and Xcode. Physical iOS deployment needs signing credentials. Application source and release profiles are unchanged.
