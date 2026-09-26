package dev.cranpose.intellij

import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.fileTypes.PlainTextFileType
import com.intellij.testFramework.fixtures.BasePlatformTestCase
import java.nio.file.Files

class StabilityPlatformTest : BasePlatformTestCase() {
    fun testNativeEngineAnalyzesUnsavedUnicodeParametersWithoutWritingTheProject() {
        val root = Files.createTempDirectory("cranpose-stability-")
        try {
            Files.createDirectories(root.resolve("src"))
            Files.writeString(root.resolve("Cargo.toml"), "[package]\nname='stability-test'\nversion='0.1.0'\n")
            val original = "#[cranpose::composable] fn Card(title: String) {}"
            Files.writeString(root.resolve("src/lib.rs"), original)
            val source = "const _: &str = \"😀\"; #[cranpose::composable] fn Card(名字: String, on_click: impl Fn()) {}"
            val result = NativeStability.analyze(root.toString(), listOf(StabilityOverlay("src/lib.rs", source)), listOf("src/lib.rs"))
            val badges = result.files.getValue("src/lib.rs")
            assertEquals(listOf("stable", "unstable"), badges.map { it.label })
            assertEquals(listOf("String", "impl Fn()"), badges.map { source.substring(it.start, it.end) })
            assertTrue(badges[1].detail.contains("every parent invocation"))
            assertEquals(original, Files.readString(root.resolve("src/lib.rs")))
            assertFalse(Files.exists(root.resolve("Cargo.lock")))
            assertFalse(Files.exists(root.resolve("target")))
            val fixed = NativeStability.analyze(root.toString(), listOf(StabilityOverlay("src/lib.rs", original)), listOf("src/lib.rs"))
            assertEquals(listOf("stable"), fixed.files.getValue("src/lib.rs").map { it.label })
        } finally { root.toFile().deleteRecursively() }
    }

    fun testBadgesAreRealInlineElementsAndReplacementRemovesOldFindings() {
        val source = "#[cranpose::composable]\nfn Card(title: String, count: i32) {}"
        myFixture.configureByText(PlainTextFileType.INSTANCE, source)
        val editor = myFixture.editor
        val start = source.indexOf("String")
        val stable = StabilityBadge(start, start + 6, "stable", "stable", "Compared by value")
        StabilityInlays.replace(editor, listOf(stable), true)
        val first = editor.inlayModel.getInlineElementsInRange(0, source.length, StabilityBadgeRenderer::class.java)
        assertEquals(1, first.size)
        assertEquals(start + 6, first.single().offset)
        assertEquals("stable", first.single().renderer.badge.label)
        WriteCommandAction.runWriteCommandAction(project) { editor.document.setText("fn Card() {}") }
        StabilityInlays.replace(editor, emptyList(), true)
        assertFalse(first.single().isValid)
        assertTrue(editor.inlayModel.getInlineElementsInRange(0, editor.document.textLength, StabilityBadgeRenderer::class.java).isEmpty())
    }

    fun testInvalidRangesCannotCreateMisplacedBadges() {
        myFixture.configureByText(PlainTextFileType.INSTANCE, "fn Card() {}")
        val editor = myFixture.editor
        StabilityInlays.replace(editor, listOf(
            StabilityBadge(-1, 2, "bad", "danger", "invalid"),
            StabilityBadge(2, 500, "bad", "danger", "invalid")), true)
        assertTrue(editor.inlayModel.getInlineElementsInRange(0, editor.document.textLength).isEmpty())
    }

    fun testSyntaxAndConfigurationFailuresStayVisible() {
        val result = NativeStability.parse("""{"schemaVersion":1,"files":[],"diagnostics":[{"rule":"CP000","path":"src/lib.rs","location":{"utf16Start":3},"message":"Unclosed function"}]}""")
        assertEquals("syntax error", result.files.getValue("src/lib.rs").single().label)
        try {
            NativeStability.parse("""{"schemaVersion":1,"error":"Invalid configuration"}""")
            fail("A failed analysis must not look like a clean report")
        } catch (expected: java.io.IOException) { assertTrue(expected.message!!.contains("Invalid configuration")) }
    }
}
