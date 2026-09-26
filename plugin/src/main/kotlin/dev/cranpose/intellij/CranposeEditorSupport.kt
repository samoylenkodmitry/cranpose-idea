package dev.cranpose.intellij

import com.intellij.codeInsight.completion.CompletionContributor
import com.intellij.codeInsight.completion.CompletionParameters
import com.intellij.codeInsight.completion.CompletionProvider
import com.intellij.codeInsight.completion.CompletionResultSet
import com.intellij.codeInsight.completion.CompletionType
import com.intellij.codeInsight.daemon.LineMarkerInfo
import com.intellij.codeInsight.daemon.LineMarkerProvider
import com.intellij.codeInsight.lookup.LookupElementBuilder
import com.intellij.openapi.editor.markup.GutterIconRenderer
import com.intellij.openapi.util.IconLoader
import com.intellij.patterns.PlatformPatterns
import com.intellij.psi.PsiElement
import com.intellij.psi.PsiFile
import com.intellij.psi.util.CachedValueProvider
import com.intellij.psi.util.CachedValuesManager
import com.intellij.util.ProcessingContext
import com.intellij.openapi.wm.ToolWindowManager
import javax.swing.Icon

internal val cranposeIcon: Icon get() = IconLoader.getIcon("/icons/cranpose.svg", CranposeLineMarker::class.java)

internal fun symbols(file: PsiFile): List<ComposableSymbol> = CachedValuesManager.getCachedValue(file) {
    CachedValueProvider.Result.create(CranposeSource.composables(file.text), file)
}

class CranposeLineMarker : LineMarkerProvider {
    override fun getLineMarkerInfo(element: PsiElement): LineMarkerInfo<*>? {
        val file = element.containingFile ?: return null
        if (file.virtualFile?.extension != "rs" || element.firstChild != null) return null
        val symbol = symbols(file).firstOrNull { it.offset == element.textRange.startOffset } ?: return null
        return LineMarkerInfo(element, element.textRange, cranposeIcon,
            { "Preview ${symbol.name} beside source" },
            { _, _ -> PreviewController.showSource(element.project, file.virtualFile, symbol.name) },
            GutterIconRenderer.Alignment.LEFT, { "Cranpose composable" })
    }
}

data class CranposeSnippet(val key: String, val title: String, val code: String)

object CranposeSnippets {
    val all = listOf(
        CranposeSnippet("cpcomposable", "Composable function", "#[cranpose::composable]\nfn MyComponent() {\n    \n}"),
        CranposeSnippet("cppreview", "Component preview", "#[cranpose::preview(name = \"Default\", width = 480, height = 640)]\n#[cranpose::composable]\nfn ComponentPreview() {\n    \n}"),
        CranposeSnippet("cpcolumn", "Column layout", "cranpose::Column(cranpose::Modifier::empty(), cranpose::ColumnSpec::default(), move || {\n    \n});"),
        CranposeSnippet("cprow", "Row layout", "cranpose::Row(cranpose::Modifier::empty(), cranpose::RowSpec::default(), move || {\n    \n});"),
        CranposeSnippet("cptext", "Text", "cranpose::Text(\"Hello\", cranpose::Modifier::empty(), cranpose::TextStyle::default());"),
        CranposeSnippet("cpstate", "Remembered mutable state", "let state = cranpose::rememberMutableStateOf(|| 0);"),
        CranposeSnippet("cpbutton", "Button with action", "cranpose::Button(cranpose::Modifier::empty(), cranpose::ButtonSpec::default(), move || {}, move || {\n    cranpose::Text(\"Button\", cranpose::Modifier::empty(), cranpose::TextStyle::default());\n});"),
    )
}

class CranposeCompletion : CompletionContributor() {
    init {
        extend(CompletionType.BASIC, PlatformPatterns.psiElement(), object : CompletionProvider<CompletionParameters>() {
            override fun addCompletions(parameters: CompletionParameters, context: ProcessingContext, result: CompletionResultSet) {
                if (parameters.originalFile.virtualFile?.extension != "rs") return
                val source = parameters.originalFile.text
                val offset = parameters.offset.coerceAtMost(source.length)
                if (offset > 0 && CranposeSource.maskLiterals(source)[offset - 1] == ' ' && source[offset - 1] != ' ') return
                for (snippet in CranposeSnippets.all) result.addElement(
                    LookupElementBuilder.create(snippet.key).withPresentableText(snippet.key)
                        .withTypeText("Cranpose").withTailText("  ${snippet.title}").withIcon(cranposeIcon)
                        .withInsertHandler { insertion, _ ->
                            insertion.document.replaceString(insertion.startOffset, insertion.tailOffset, snippet.code)
                            insertion.editor.caretModel.moveToOffset(insertion.startOffset + snippet.code.length)
                        })
            }
        })
    }
}
