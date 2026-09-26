package dev.cranpose.intellij

import com.intellij.testFramework.fixtures.BasePlatformTestCase
import org.jdom.Element

class StudioPlatformTest : BasePlatformTestCase() {
    fun testRunConfigurationRoundTripPreservesQuotedArgumentsAndEnvironment() {
        val factory = CranposeConfigurationType().configurationFactories.single()
        val config = CranposeRunConfiguration(project, factory, "Journal")
        config.useTarget(CargoTarget("journal", "desktop", "bin", "/a b/Cargo.toml", "/a b/main.rs", listOf("desktop"), "cranpose"))
        config.arguments = """--title "A quiet morning" --count 2"""
        config.environmentText = "RUST_LOG=debug\nNOTE=one=two"
        config.noDefaultFeatures = true
        val xml = Element("configuration")
        config.writeExternal(xml)
        val restored = CranposeRunConfiguration(project, factory, "Restored")
        restored.readExternal(xml)
        assertEquals(config.cargoArguments(), restored.cargoArguments())
        assertEquals(listOf("--title", "A quiet morning", "--count", "2"), restored.cargoArguments().takeLast(4))
        assertEquals(mapOf("RUST_LOG" to "debug", "NOTE" to "one=two"), restored.environment())
        assertEquals("/a b", restored.directory)
        assertTrue(restored.noDefaultFeatures)
    }

    fun testPreviewProviderOnlyClaimsCranposeCargoSources() {
        val plain = myFixture.addFileToProject("plain/Cargo.toml", "[package]\nname='ordinary'").virtualFile
        val rust = myFixture.addFileToProject("plain/src/main.rs", "fn main() {}").virtualFile
        val provider = CranposePreviewEditorProvider()
        assertFalse(provider.accept(project, rust))
        val manifest = myFixture.addFileToProject("ui/Cargo.toml", "[dependencies]\ncranpose='0.1'").virtualFile
        val component = myFixture.addFileToProject("ui/src/card.rs", "#[cranpose::composable]\nfn Card() {}").virtualFile
        assertTrue(provider.accept(project, component))
        assertFalse(provider.accept(project, manifest))
        assertFalse(provider.accept(project, plain))
    }

    fun testSettingsClampInvalidPersistedViewport() {
        val settings = StudioSettings()
        settings.loadState(StudioSettings.Data().apply { width = -2; height = Int.MAX_VALUE; zoom = Double.NaN })
        assertEquals(120, settings.state.width)
        assertEquals(4096, settings.state.height)
        assertEquals(1.0, settings.state.zoom)
    }

    fun testPreviewEditorCreatesItsToolbarsAndDisposesWithTheEditor() {
        val file = myFixture.addFileToProject("studio/main.rs", "fn main() {}").virtualFile
        val editor = CranposePreviewEditor(project, file)
        try {
            assertSame(file, editor.file)
            assertTrue(editor.component.componentCount > 0)
        } finally {
            editor.dispose()
        }
        assertTrue(com.intellij.openapi.util.Disposer.isDisposed(editor.workspace))
    }
}
