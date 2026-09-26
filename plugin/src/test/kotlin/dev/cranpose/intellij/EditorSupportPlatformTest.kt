package dev.cranpose.intellij

import com.intellij.testFramework.fixtures.BasePlatformTestCase

class EditorSupportPlatformTest : BasePlatformTestCase() {
    fun testCranposeCompletionsAreRegisteredForRustFiles() {
        myFixture.configureByText("screen.rs", "cpco<caret>")
        val variants = myFixture.completeBasic().orEmpty().map { it.lookupString }
        assertTrue("Cranpose component completions: $variants", variants.containsAll(listOf("cpcomposable", "cpcolumn")))
    }

    fun testCompletionsDoNotAppearInOtherFiles() {
        myFixture.configureByText("notes.txt", "cpco<caret>")
        val variants = myFixture.completeBasic().orEmpty().map { it.lookupString }
        assertFalse(variants.contains("cpcomposable"))
    }
}
