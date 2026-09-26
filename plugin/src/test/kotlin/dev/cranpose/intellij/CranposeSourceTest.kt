package dev.cranpose.intellij

import org.junit.Assert.*
import org.junit.Test

class CranposeSourceTest {
    @Test fun discoversQualifiedAttributesAndPreservesEditorOffsets() {
        val source = "// é🙂\n#[cranpose::composable]\n#[track_caller]\npub(crate) fn Screen<T>() {}\n#[composable] fn r#type() {}"
        val symbols = CranposeSource.composables(source)
        assertEquals(listOf("Screen", "type"), symbols.map { it.name })
        assertEquals(source.indexOf("Screen"), symbols[0].offset)
        assertEquals(source.indexOf("r#type"), symbols[1].offset)
        assertEquals(4, symbols[0].line)
    }

    @Test fun ignoresNestedCommentsAndStringContents() {
        val source = """
            /* outer /* #[composable] fn Wrong() {} */ still comment */
            // #[composable] fn AlsoWrong() {}
            const TEXT: &str = "#[composable] fn InString() {}";
            const RAW: &str = r##" "#[composable] fn InRaw() {} "##;
            #[composable]
            fn Real() {}
        """.trimIndent()
        assertEquals(listOf("Real"), CranposeSource.composables(source).map { it.name })
        assertEquals(source.length, CranposeSource.maskLiterals(source).length)
    }

    @Test fun handlesUnfinishedCodeWhileTyping() {
        assertTrue(CranposeSource.composables("/* incomplete").isEmpty())
        assertTrue(CranposeSource.composables("#[composable] fn").isEmpty())
        assertEquals(listOf("First"), CranposeSource.composables("#[composable] fn First() {}\n\"unfinished").map { it.name })
    }

    @Test fun starterUsesTheSameRootForDesktopAndPreview() {
        assertTrue(CranposeStarter.source.contains("#[cranpose::preview"))
        assertTrue(CranposeStarter.source.contains("app.run(App)"))
        assertTrue(CranposeStarter.manifest("demo").contains("features = [\"desktop\", \"preview\"]"))
        assertEquals(CranposeSnippets.all.size, CranposeSnippets.all.map { it.key }.distinct().size)
        assertTrue(CranposeSnippets.all.any { it.key == "cppreview" })
    }
}
