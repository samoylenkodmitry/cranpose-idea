# Publishing

Marketplace publication is a separate, manual step. Ordinary pushes and tags
build and verify artifacts; they never publish this plugin.

## Before the first upload

1. Use a green Build run for the exact commit to be published. Its `plugin`
   artifact contains all six native binaries.
2. Install that ZIP in IDEA and RustRover and exercise the sample counter:
   discovery, check, run, preview, click, rebuild, theme switch and inspection.
3. Review the plugin name, vendor, description, icon, screenshots, license and
   privacy statement. No telemetry is sent; Cargo may access the registries and
   Git repositories configured by the opened project.
4. Set the desired version in `plugin/gradle.properties` and add its changelog entry.
5. Configure these repository secrets, using the same names as Zeus Thunderbolt
   and DiffTrack: `PUBLISH_TOKEN`, `CERTIFICATE_CHAIN`, `PRIVATE_KEY`,
   `PRIVATE_KEY_PASSWORD`. No credentials are stored in this repository.

JetBrains' [first-upload guide](https://plugins.jetbrains.com/docs/marketplace/uploading-a-new-plugin.html)
covers creating the Marketplace listing. Its
[signing guide](https://plugins.jetbrains.com/docs/intellij/plugin-signing.html)
describes certificates and token setup.

## Subsequent releases

Create a `vX.Y.Z` tag matching `pluginVersion` at a green source commit.
Run the **Publish** workflow manually for that tag. It requires the repository's
`marketplace` environment, downloads the native artifacts from the successful
Build run for that exact commit, validates all platforms, verifies compatibility,
signs and publishes the plugin. Configure required reviewers on that environment
if release approval is desired.

The workflow is ready for a configured Marketplace listing; it has not been run
as part of implementation.

## Local package

`cd plugin && ./gradlew buildPlugin` creates a ZIP for the current OS/CPU.
For a release, download the six `native-*` Build artifacts into `native/`, run
`python3 scripts/release.py arrange-native native`, then build with
`./gradlew buildPlugin -PcranposeNativeDir=/absolute/path/to/native`.
