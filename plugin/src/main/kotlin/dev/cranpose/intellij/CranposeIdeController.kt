package dev.cranpose.intellij

import com.google.gson.Gson
import com.intellij.ide.BrowserUtil
import com.intellij.openapi.Disposable
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.event.DocumentEvent
import com.intellij.openapi.editor.event.DocumentListener
import com.intellij.openapi.fileEditor.FileEditorManager
import com.intellij.openapi.fileEditor.FileEditorManagerEvent
import com.intellij.openapi.fileEditor.FileEditorManagerListener
import com.intellij.openapi.util.Disposer
import javax.swing.Timer

class CranposeIdeController(
    private val project: com.intellij.openapi.project.Project,
    private val panel: CranposePanel,
    parent: Disposable,
) {
    private val service = CranposeProjectService.get(project)
    private val gson = Gson()
    private val timer = Timer(250) { sendEditor() }.apply { isRepeats = false }
    init {
        service.subscribe(parent) { panel.send("cranpose.project", service.json()) }
        project.messageBus.connect(parent).subscribe(FileEditorManagerListener.FILE_EDITOR_MANAGER,
            object : FileEditorManagerListener {
                override fun selectionChanged(event: FileEditorManagerEvent) = sendEditor()
            })
        EditorFactory.getInstance().eventMulticaster.addDocumentListener(object : DocumentListener {
            override fun documentChanged(event: DocumentEvent) {
                if (event.document == FileEditorManager.getInstance(project).selectedTextEditor?.document) timer.restart()
            }
        }, parent)
        Disposer.register(parent) { timer.stop() }
    }

    fun connected() {
        panel.send("cranpose.project", service.json())
        sendEditor()
        if (service.snapshot.root.isEmpty()) service.refresh()
    }

    fun handle(channel: String, payload: String): Boolean {
        if (channel != "cranpose.action") return false
        val fields = FlatJson.decodeStrings(payload) ?: return true
        when (fields["action"]) {
            "refresh" -> service.refresh()
            "select" -> service.select(fields["value"].orEmpty())
            "check" -> service.execute(CargoTask.CHECK)
            "run" -> service.execute(CargoTask.RUN)
            "test" -> service.execute(CargoTask.TEST)
            "preview" -> service.execute(CargoTask.PREVIEW)
            "stop" -> service.stop()
            "create" -> CranposeStarter.create(project)
            "docs" -> BrowserUtil.browse("https://docs.rs/cranpose/latest/cranpose/")
            "configure" -> service.snapshot.targets.firstOrNull { it.id == service.snapshot.selected }?.let {
                CranposeRunConfiguration.create(project, it)
            }
            "component" -> FileEditorManager.getInstance(project).selectedFiles.firstOrNull()?.let {
                PreviewController.showSource(project, it, fields["value"])
            }
            "diagnostic" -> fields["value"]?.toIntOrNull()?.let { index ->
                service.snapshot.diagnostics.getOrNull(index)?.let { diagnostic ->
                    val path = java.nio.file.Path.of(service.snapshot.root).resolve(diagnostic.file).normalize()
                    com.intellij.openapi.vfs.LocalFileSystem.getInstance().findFileByPath(path.toString())?.let {
                        com.intellij.openapi.fileEditor.OpenFileDescriptor(project, it, diagnostic.line - 1, diagnostic.column - 1).navigate(true)
                    }
                }
            }
            "navigate" -> {
                val editor = FileEditorManager.getInstance(project).selectedTextEditor ?: return true
                val offset = fields["value"]?.toIntOrNull() ?: return true
                if (offset in 0..editor.document.textLength) {
                    editor.caretModel.moveToOffset(offset)
                    editor.scrollingModel.scrollToCaret(com.intellij.openapi.editor.ScrollType.CENTER)
                    editor.contentComponent.requestFocusInWindow()
                }
            }
        }
        return true
    }

    private fun sendEditor() {
        val manager = FileEditorManager.getInstance(project)
        val file = manager.selectedFiles.firstOrNull()
        val text = manager.selectedTextEditor?.document?.text.orEmpty()
        panel.send("cranpose.editor", gson.toJson(mapOf(
            "path" to file?.path.orEmpty(), "name" to file?.name.orEmpty(),
            "composables" to if (file?.extension == "rs") CranposeSource.composables(text) else emptyList()
        )))
    }
}
