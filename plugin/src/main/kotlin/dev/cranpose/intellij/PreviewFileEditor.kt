package dev.cranpose.intellij

import com.intellij.openapi.fileEditor.*
import com.intellij.openapi.fileEditor.impl.text.TextEditorProvider
import com.intellij.openapi.project.DumbAware
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.UserDataHolderBase
import com.intellij.openapi.vfs.VirtualFile
import java.beans.PropertyChangeListener
import javax.swing.JComponent

class CranposePreviewEditorProvider : FileEditorProvider, DumbAware {
    override fun accept(project: Project, file: VirtualFile): Boolean {
        if (file.extension != "rs" || file.length > 2_000_000) return false
        return generateSequence(file.parent) { it.parent }.mapNotNull { it.findChild("Cargo.toml") }.any {
            it.length < 1_000_000 && com.intellij.openapi.vfs.VfsUtilCore.loadText(it).contains("cranpose")
        }
    }
    override fun createEditor(project: Project, file: VirtualFile): FileEditor {
        val text = TextEditorProvider.getInstance().createEditor(project, file) as TextEditor
        return CranposeSplitEditor(text, CranposePreviewEditor(project, file))
    }
    override fun getEditorTypeId(): String = "cranpose-studio"
    override fun getPolicy(): FileEditorPolicy = FileEditorPolicy.HIDE_DEFAULT_EDITOR
}

class CranposeSplitEditor(text: TextEditor, val studio: CranposePreviewEditor) :
    TextEditorWithPreview(text, studio, "Cranpose", Layout.SHOW_EDITOR) {
    override val splitterProportionKey: String get() = "cranpose.studio.split"
}

class CranposePreviewEditor(project: Project, private val source: VirtualFile) : UserDataHolderBase(), FileEditor {
    val workspace = PreviewWorkspace(project, source)
    override fun getComponent(): JComponent = workspace
    override fun getPreferredFocusedComponent(): JComponent = workspace
    override fun getName(): String = "Preview"
    override fun getFile(): VirtualFile = source
    override fun setState(state: FileEditorState) {}
    override fun isModified(): Boolean = false
    override fun isValid(): Boolean = source.isValid
    override fun addPropertyChangeListener(listener: PropertyChangeListener) {}
    override fun removePropertyChangeListener(listener: PropertyChangeListener) {}
    override fun dispose() = com.intellij.openapi.util.Disposer.dispose(workspace)
}
