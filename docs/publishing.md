# Publishing Cranpose

Marketplace publication is a separate, manual step. Build and test the exact tagged
commit before using this workflow.

## Prepare

1. Set the semantic version in `plugin/VERSION` and update the change notes in
   `plugin/src/main/resources/META-INF/plugin.xml`.
2. Merge the change and wait for the Build workflow. It runs Rust tests, the real IDEA
   integration suite, hot-reload recovery checks, the stability action and Plugin
   Verifier against IDEA and RustRover.
3. Create a matching `vX.Y.Z` tag. The Publish workflow requires a successful Build
   for that exact commit and signs its existing all-platform artifact.

The source audit rejects authored Kotlin, Java, Python and shell scripts. Workflow
YAML invokes Rust tools and third-party setup actions.

## First Marketplace upload

Create the Marketplace listing manually using the verified plugin ZIP. Set the
repository URL, Apache-2.0 license, description, screenshots and credits. Record the
numeric Marketplace plugin ID for later updates.

Credits follow [Zeus Thunderbolt](https://github.com/samoylenkodmitry/Zeus-Thunderbolt-Idea-Plugin)
and [DiffTrack](https://github.com/samoylenkodmitry/difftrack). Retain the Cranpose,
template, Subsecond and Compose Stability Analyzer credits in the README.

## Signing and later updates

Configure the `marketplace` GitHub environment with:

- `CERTIFICATE_CHAIN`: PEM certificate chain.
- `PRIVATE_KEY`: PEM private key.
- `PRIVATE_KEY_PASSWORD`: optional key password.
- `PUBLISH_TOKEN`: JetBrains permanent upload token.

Run **Publish** manually with the matching tag and Marketplace numeric ID.
Rust tooling validates the tag, locates the successful build, checks all six native
UIs and hosts, signs with JetBrains ZIP Signer 0.1.43, and uploads the signed file.
Keys are written only to temporary files and removed when the signer exits.

Local commands, for an already verified all-platform archive:

```sh
cargo run -p xtask -- release validate-tag v0.5.0
cargo run -p xtask -- release check-zip target/plugin/cranpose-idea-0.5.0.zip
cargo run -p xtask -- release verify target/plugin/cranpose-idea-0.5.0.zip --ide /path/to/IDE
cargo run -p xtask -- release sign target/plugin/cranpose-idea-0.5.0.zip signed-plugin.zip
cargo run -p xtask -- release publish signed-plugin.zip --plugin-id YOUR_MARKETPLACE_ID

```

The last command publishes an update; run it only when ready to submit to Marketplace.
The upload format follows [JetBrains' API](https://plugins.jetbrains.com/docs/marketplace/plugin-upload.html).
Signing follows [JetBrains' signing documentation](https://plugins.jetbrains.com/docs/intellij/plugin-signing.html).
