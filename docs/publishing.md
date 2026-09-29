# Publishing Cranpose

Pushing a version tag builds, verifies, signs and publishes the plugin. The first
Marketplace listing still requires a manual upload of the CI-built signed archive.

## Prepare

1. Set the semantic version in `plugin/VERSION` and update the change notes in
   `plugin/src/main/resources/META-INF/plugin.xml`.
2. Merge the change and wait for the Build workflow. It runs Rust tests, the real IDEA
   integration suite, hot-reload recovery checks, the stability action and Plugin
   Verifier against IDEA and RustRover.
3. Create and push the matching `vX.Y.Z` tag. The Build workflow runs all checks,
   then calls Publish after the verified all-platform package job succeeds.
4. Publish signs that exact artifact, attaches it to the matching GitHub release,
   and submits it to Marketplace when `MARKETPLACE_PLUGIN_ID` is configured.

The source audit rejects authored Kotlin, Java, Python and shell scripts. Workflow
YAML invokes Rust tools and third-party setup actions.

## First Marketplace upload

Create the Marketplace listing manually using the verified plugin ZIP. Set the
repository URL, Apache-2.0 license, description, screenshots and credits. Record the
numeric Marketplace plugin ID in the repository variable `MARKETPLACE_PLUGIN_ID`
for subsequent automatic updates. Leave it empty for the first tag: CI creates
the signed GitHub release archive for the manual listing upload. The listing's
screenshots and demo video are kept in [marketplace](marketplace/README.md).

Credits follow [Zeus Thunderbolt](https://github.com/samoylenkodmitry/Zeus-Thunderbolt-Idea-Plugin)
and [DiffTrack](https://github.com/samoylenkodmitry/difftrack). Retain the Cranpose,
template, Subsecond and Compose Stability Analyzer credits in the README.

## Signing and later updates

Configure repository secrets (or the `marketplace` GitHub environment) with:

- `CERTIFICATE_CHAIN`: PEM certificate chain.
- `PRIVATE_KEY`: PEM private key.
- `PRIVATE_KEY_PASSWORD`: optional key password.
- `PUBLISH_TOKEN`: JetBrains permanent upload token.

Version tags trigger publication automatically. **Publish** can also be retried
manually with the tag; it requires a successful Build for the exact tagged commit.
Rust tooling validates the tag, archive version and plugin ID, checks all six native
UIs and hosts, signs with JetBrains ZIP Signer 0.1.43, and uploads the signed file.
The stable Marketplace channel is represented by an empty channel name.
Keys are written only to temporary files and removed when the signer exits.

Local commands, for an already verified all-platform archive:

```sh
cargo run -p xtask -- release validate-tag v0.12.0
cargo run -p xtask -- release check-zip verified-plugin/cranpose-idea-0.12.0.zip
cargo run -p xtask -- release verify verified-plugin/cranpose-idea-0.12.0.zip --ide /path/to/IDE
cargo run -p xtask -- release sign verified-plugin/cranpose-idea-0.12.0.zip signed-plugin.zip
cargo run -p xtask -- release github-release signed-plugin.zip v0.12.0
cargo run -p xtask -- release publish signed-plugin.zip --plugin-id YOUR_MARKETPLACE_ID

```

The last two commands publish artifacts; run them only when ready to release.
The upload format follows [JetBrains' API](https://plugins.jetbrains.com/docs/marketplace/plugin-upload.html).
Signing follows [JetBrains' signing documentation](https://plugins.jetbrains.com/docs/intellij/plugin-signing.html).
