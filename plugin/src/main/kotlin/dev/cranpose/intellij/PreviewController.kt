package dev.cranpose.intellij

import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.wm.ToolWindowManager
import com.intellij.ui.JBColor
import com.intellij.ui.components.JBScrollPane
import com.intellij.ui.components.JBTextArea
import com.intellij.ui.content.ContentFactory
import java.awt.BorderLayout
import java.nio.file.Path
import javax.swing.JButton
import javax.swing.JPanel
import javax.swing.JToggleButton
import javax.swing.Timer

object PreviewController {
    private const val REQUEST = "cranpose.inspector.v1.request"
    private const val SNAPSHOT = "cranpose.inspector.v1.snapshot"

    fun open(project: Project, target: CargoTarget, binary: Path) {
        val window = ToolWindowManager.getInstance(project).getToolWindow("Cranpose") ?: return
        val panel = CranposePanel({ listOf(binary.toString()) }, Path.of(target.manifest).parent)
        val report = JBTextArea("Click Inspect to capture the preview's layout and runtime state.").apply {
            isEditable = false
            lineWrap = false
        }
        val root = JPanel(BorderLayout())
        val toolbar = JPanel()
        val dark = JToggleButton("Dark theme", !JBColor.isBright())
        val reload = JButton("Rebuild")
        val inspect = JButton("Inspect")
        val stop = JButton("Stop")
        val autoReload = JToggleButton("Rebuild on save", com.intellij.ide.util.PropertiesComponent.getInstance(project).getBoolean("cranpose.preview.autoReload", true))
        toolbar.add(reload)
        toolbar.add(dark)
        toolbar.add(inspect)
        toolbar.add(stop)
        toolbar.add(autoReload)
        root.add(toolbar, BorderLayout.NORTH)
        val split = javax.swing.JSplitPane(javax.swing.JSplitPane.VERTICAL_SPLIT, panel, JBScrollPane(report)).apply {
            resizeWeight = 0.75
        }
        root.add(split, BorderLayout.CENTER)
        val content = ContentFactory.getInstance().createContent(root, "Preview: ${target.label}", false)
        content.isCloseable = true
        window.contentManager.contents.filter { it.displayName == content.displayName }.forEach {
            window.contentManager.removeContent(it, true)
        }
        window.contentManager.addContent(content)
        Disposer.register(content) {
            CranposeProjectService.get(project).cancelPreview(target.id)
            panel.close()
        }
        panel.setTheme(dark.isSelected)
        dark.addActionListener { panel.setTheme(dark.isSelected) }
        reload.addActionListener {
            CranposeProjectService.get(project).execute(CargoTask.PREVIEW, target.id)
        }
        autoReload.addActionListener {
            com.intellij.ide.util.PropertiesComponent.getInstance(project).setValue("cranpose.preview.autoReload", autoReload.isSelected, true)
        }
        val rebuild = Timer(600) {
            if (autoReload.isSelected) CranposeProjectService.get(project).execute(CargoTask.PREVIEW, target.id)
        }.apply { isRepeats = false }
        Disposer.register(content) { rebuild.stop() }
        val sourceRoot = Path.of(CranposeProjectService.get(project).snapshot.root)
        project.messageBus.connect(content).subscribe(com.intellij.openapi.vfs.VirtualFileManager.VFS_CHANGES,
            object : com.intellij.openapi.vfs.newvfs.BulkFileListener {
                override fun after(events: List<com.intellij.openapi.vfs.newvfs.events.VFileEvent>) {
                    if (events.any { event ->
                        val path = Path.of(event.path)
                        path.startsWith(sourceRoot) && !path.any { it.toString() == "target" || it.toString() == ".git" } &&
                            (event.path.endsWith(".rs") || event.path.endsWith(".wgsl") || path.fileName.toString() == "Cargo.toml")
                    }) rebuild.restart()
                }
            })
        stop.addActionListener { window.contentManager.removeContent(content, true) }
        val timeout = Timer(5000) {
            report.text = "This preview did not return an inspection snapshot. Use Cranpose with IDE inspection support."
        }.apply { isRepeats = false }
        Disposer.register(content) { timeout.stop() }
        inspect.addActionListener {
            report.text = "Capturing layout…"
            panel.send(REQUEST, "")
            timeout.restart()
        }
        panel.onAppMessage = { channel, payload ->
            if (channel == SNAPSHOT) ApplicationManager.getApplication().invokeLater({
                timeout.stop()
                report.text = payload
                report.caretPosition = 0
            }, project.disposed)
        }
        panel.onConnected = { panel.setTheme(dark.isSelected) }
        EditorOverlay(project, panel, content)
        window.contentManager.setSelectedContent(content)
        window.show()
        panel.start()
    }
}
