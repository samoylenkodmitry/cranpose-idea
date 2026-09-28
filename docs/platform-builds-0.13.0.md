# Local platform builds — 0.13.0

The Cranpose panel's **Build for a platform** section calls the same Rust library
as [Cranpose Build](https://github.com/samoylenkodmitry/cranpose-build). Application
compilation runs locally. No GitHub Actions job or remote builder is invoked.

Choose a platform and development/release profile, then **Build package** or
**Build & run**. **Tools & devices** exposes prerequisite checks, pinned backend
setup, the actual build plan and device discovery. Android and iOS accept a device
ID. The job streams bounded output and can be stopped. Closing the project also
cancels its owned process tree. Builds require a trusted project and save editor
documents before compilation.

Desktop targets cover ARM64 and x86-64 macOS, Linux and Windows. Mobile paths use
an existing Android Gradle project or a Rust iOS executable plus Xcode. The bundled
Showcase starter now includes CranposeBuild.toml with its iOS entry point/resources
and Android ARM64 property. Studio and the starter use Cranpose 0.1.171.

The tool produces an application directory, ZIP, SHA256 and portable artifact
manifest. Moving the complete build directory to a matching machine preserves
its launch paths. Cross-compilation requires target SDKs and native libraries;
iOS packaging requires macOS/Xcode and physical devices need signing credentials.
Foreign desktop execution requires the target OS or VM. Packages are not
implicitly notarized, static or store-ready. Existing application sources,
release profiles and signing settings are unchanged.

## Validation and limits

The build library compiled the actual Showcase application locally on one Mac for
macOS ARM64, Linux x64, Windows x64, Android ARM64 and the iOS ARM64 simulator.
Observed individual build/package times were 28.6, 79.9, 44.1, 71.1 and 42.8 seconds
respectively. Caches and concurrent work varied; these are correctness runs,
not benchmarks or speedup claims. Development runs disabled debug info and
incremental compilation through the child environment.

The macOS package passed interactive planet/detail navigation. Android installed
and launched its resolved main activity with `Status: ok`; iOS installed and
launched in the simulator. A Windows diagnostic build initialized its DirectX 12
renderer and shaders over SSH, but no interactive Windows window inspection was
available. Linux SSH was unreachable, so no Linux application runtime claim is
made. Physical iOS signing/device deployment remains untested.

Library tests compile, package, relocate and launch an executable, inspect archive
metadata/assets, reject failed builds and invalid configuration, and validate
Android launcher resolution. Shared SDK tests cover bounded streaming, cancellation
and descendant exit on three operating systems. Real IDEA tests cover exclusive
background jobs, cancellation and project disposal. Studio tests cover bounded
output, status transitions, missing-tool details and clearing stale artifacts.

Local Clippy, workspace tests and RustRover diagnostics passed. Release CI additionally
checks six native hosts/renderers, actual IDEA integration, preview recovery and
both Plugin Verifier targets. All new implementation and tests are Rust; controls
are Cranpose.
