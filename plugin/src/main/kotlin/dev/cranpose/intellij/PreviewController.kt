package dev.cranpose.intellij

import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.TextEditorWithPreview
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.wm.ToolWindowManager
import com.intellij.ui.content.ContentFactory
import java.nio.file.Path

object PreviewController {
    fun showTarget(project: Project, target: CargoTarget) {
        val file = LocalFileSystem.getInstance().findFileByPath(target.source) ?: return
        showSource(project, file)?.previewTarget(target)
    }
    fun showSource(project: Project, file: VirtualFile, function: String? = null): PreviewWorkspace? {
        val manager = FileEditorManager.getInstance(project)
        val editor = manager.openFile(file, true).filterIsInstance<CranposeSplitEditor>().firstOrNull() ?: return null
        manager.setSelectedEditor(file, "cranpose-studio")
        editor.setLayout(TextEditorWithPreview.Layout.SHOW_EDITOR_AND_PREVIEW)
        function?.let(editor.studio.workspace::showFunction)
        return editor.studio.workspace
    }

    fun open(project: Project, target: CargoTarget, binary: Path) {
        val file = LocalFileSystem.getInstance().findFileByPath(target.source)
        val workspace = file?.let { showSource(project, it) }
        if (workspace != null) { workspace.showBuilt(target, binary); return }
        val window = ToolWindowManager.getInstance(project).getToolWindow("Cranpose") ?: return
        val preview = PreviewWorkspace(project, file)
        val content = ContentFactory.getInstance().createContent(preview, target.name, false)
        content.isCloseable = true
        window.contentManager.addContent(content)
        Disposer.register(content, preview)
        preview.showBuilt(target, binary)
        window.contentManager.setSelectedContent(content)
        window.show()
    }
}
