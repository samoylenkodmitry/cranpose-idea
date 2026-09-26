package dev.cranpose.intellij

import com.intellij.ide.IdeTooltip
import com.intellij.ide.IdeTooltipManager
import com.intellij.openapi.Disposable
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.actionSystem.ActionUpdateThread
import com.intellij.openapi.actionSystem.ToggleAction
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.application.ReadAction
import com.intellij.openapi.components.*
import com.intellij.openapi.editor.Document
import com.intellij.openapi.editor.EditorFactory
import com.intellij.openapi.editor.Inlay
import com.intellij.openapi.editor.event.*
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.startup.ProjectActivity
import com.intellij.openapi.util.text.StringUtil
import com.intellij.openapi.vfs.VirtualFileManager
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.VFileEvent
import com.intellij.util.Alarm
import java.nio.file.Path
import java.util.concurrent.atomic.AtomicLong
import javax.swing.JLabel

@Service(Service.Level.PROJECT)
@State(name = "CranposeStability", storages = [Storage(StoragePathMacros.WORKSPACE_FILE)])
class StabilitySettings : PersistentStateComponent<StabilitySettings.Data> {
    class Data { var enabled = true; var showStable = true }
    private var data = Data()
    override fun getState() = data
    override fun loadState(state: Data) { data = state }
}

class StabilityStartup : ProjectActivity {
    override suspend fun execute(project: Project) {
        if (!ApplicationManager.getApplication().isUnitTestMode) {
            ApplicationManager.getApplication().invokeLater {
                if (!project.isDisposed) project.service<StabilityService>().start()
            }
        }
    }
}

private data class StabilityDocument(val document: Document, val path: String, val stamp: Long, val source: String)
private data class StabilitySnapshot(val root: Path, val documents: List<StabilityDocument>, val overlays: List<StabilityOverlay>)

/** Schedules read-only native analysis and applies results only to the document versions it analyzed. */
@Service(Service.Level.PROJECT)
class StabilityService(private val project: Project) : Disposable {
    private val alarm = Alarm(Alarm.ThreadToUse.POOLED_THREAD, this)
    private val revision = AtomicLong()
    private val processLock = Any()
    @Volatile private var active: Process? = null
    @Volatile private var disposed = false
    private var started = false
    private var hovered: Inlay<*>? = null
    private var tooltip: IdeTooltip? = null

    fun start() {
        if (started || disposed) return
        started = true
        val factory = EditorFactory.getInstance()
        factory.eventMulticaster.addDocumentListener(object : DocumentListener {
            override fun documentChanged(event: DocumentEvent) {
                val file = FileDocumentManager.getInstance().getFile(event.document) ?: return
                val root = project.basePath ?: return
                if (file.path.startsWith("$root/") && (file.extension == "rs" || file.name == "cranpose-stability.toml" || file.name == "Cargo.toml")) schedule()
            }
        }, this)
        factory.addEditorFactoryListener(object : EditorFactoryListener {
            override fun editorCreated(event: EditorFactoryEvent) { if (event.editor.project == project) schedule() }
            override fun editorReleased(event: EditorFactoryEvent) {
                if (event.editor.project == project) { hideTooltip(); schedule() }
            }
        }, this)
        factory.eventMulticaster.addEditorMouseMotionListener(object : EditorMouseMotionListener {
            override fun mouseMoved(event: EditorMouseEvent) {
                if (event.editor.project != project) return
                val inlay = event.editor.inlayModel.getElementAt(event.mouseEvent.point, StabilityBadgeRenderer::class.java)
                if (hovered === inlay) return
                hideTooltip()
                if (inlay == null) return
                hovered = inlay
                val detail = StringUtil.escapeXmlEntities(inlay.renderer.badge.detail).replace("\n", "<br>")
                tooltip = IdeTooltip(event.editor.contentComponent, event.mouseEvent.point,
                    JLabel("<html><body style='width: 360px'>${detail}</body></html>")).also {
                    IdeTooltipManager.getInstance().show(it, false)
                }
            }
        }, this)
        project.messageBus.connect(this).subscribe(VirtualFileManager.VFS_CHANGES, object : BulkFileListener {
            override fun after(events: List<VFileEvent>) {
                val root = project.basePath ?: return
                if (events.any { event ->
                    event.path.startsWith("$root/") && !event.path.contains("/target/") &&
                        (event.path.endsWith(".rs") || event.path.endsWith("/Cargo.toml") || event.path.endsWith("/cranpose-stability.toml"))
                }) schedule()
            }
        })
        schedule()
    }

    fun schedule() {
        val expected = revision.incrementAndGet()
        synchronized(processLock) { active?.destroyForcibly(); active = null }
        alarm.cancelAllRequests()
        if (disposed || project.isDisposed) return
        if (!project.service<StabilitySettings>().state.enabled) {
            ApplicationManager.getApplication().invokeLater { clear() }
            return
        }
        alarm.addRequest({ scan(expected) }, 450)
    }

    private fun snapshot(): StabilitySnapshot? = ReadAction.compute<StabilitySnapshot?, RuntimeException> {
        if (project.isDisposed) return@compute null
        val root = project.basePath?.let { Path.of(it).toAbsolutePath().normalize() } ?: return@compute null
        val manager = FileDocumentManager.getInstance()
        val documents = EditorFactory.getInstance().allEditors.asSequence()
            .filter { it.project == project && !it.isDisposed }
            .map { it.document }.distinct()
            .mapNotNull { document ->
                val file = manager.getFile(document) ?: return@mapNotNull null
                val path = Path.of(file.path).toAbsolutePath().normalize()
                if (file.extension != "rs" || !path.startsWith(root)) return@mapNotNull null
                StabilityDocument(document, root.relativize(path).toString().replace('\\', '/'),
                    document.modificationStamp, document.text)
            }.toList()
        if (documents.isEmpty()) return@compute null
        val overlays = manager.unsavedDocuments.mapNotNull { document ->
            val file = manager.getFile(document) ?: return@mapNotNull null
            val path = Path.of(file.path).toAbsolutePath().normalize()
            if (file.extension != "rs" || !path.startsWith(root)) return@mapNotNull null
            StabilityOverlay(path.toString(), document.text)
        }.associateBy { it.path }.toMutableMap()
        // Some editor documents have not yet been registered as unsaved by the file manager.
        documents.forEach { overlays[root.resolve(it.path).toString()] = StabilityOverlay(root.resolve(it.path).toString(), it.source) }
        StabilitySnapshot(root, documents, overlays.values.toList())
    }

    private fun scan(expected: Long) {
        if (stale(expected)) return
        val snapshot = snapshot() ?: return
        val result = try {
            NativeStability.analyze(snapshot.root.toString(), snapshot.overlays, snapshot.documents.map { it.path },
                onProcess = { process -> synchronized(processLock) { if (stale(expected)) process.destroyForcibly() else active = process } })
        } catch (error: Exception) {
            if (stale(expected)) return
            StabilityAnalysis(snapshot.documents.associate { it.path to listOf(
                StabilityBadge(0, 0, "analysis unavailable", "muted", error.message ?: "Stability analysis failed")) })
        }
        if (stale(expected)) return
        ApplicationManager.getApplication().invokeLater {
            if (stale(expected)) return@invokeLater
            if (snapshot.documents.any { it.document.modificationStamp != it.stamp }) { schedule(); return@invokeLater }
            val settings = project.service<StabilitySettings>().state
            if (!settings.enabled) return@invokeLater
            hideTooltip()
            for (editor in EditorFactory.getInstance().allEditors) {
                if (editor.project != project || editor.isDisposed) continue
                val document = snapshot.documents.firstOrNull { it.document === editor.document } ?: continue
                StabilityInlays.replace(editor, result.files[document.path].orEmpty(), settings.showStable)
            }
        }
    }

    private fun stale(expected: Long) = disposed || project.isDisposed || revision.get() != expected
    private fun hideTooltip() { tooltip?.hide(); tooltip = null; hovered = null }
    private fun clear() {
        hideTooltip()
        EditorFactory.getInstance().allEditors.filter { it.project == project && !it.isDisposed }
            .forEach { StabilityInlays.clear(it) }
    }
    override fun dispose() {
        disposed = true
        revision.incrementAndGet()
        synchronized(processLock) { active?.destroyForcibly(); active = null }
        alarm.cancelAllRequests()
        if (ApplicationManager.getApplication().isDispatchThread) clear()
        else ApplicationManager.getApplication().invokeLater { clear() }
    }
}

class ToggleStabilityBadgesAction : ToggleAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.EDT
    override fun isSelected(event: AnActionEvent) = event.project?.service<StabilitySettings>()?.state?.enabled ?: false
    override fun setSelected(event: AnActionEvent, state: Boolean) {
        event.project?.let { it.service<StabilitySettings>().state.enabled = state; it.service<StabilityService>().schedule() }
    }
}
class ToggleStableBadgesAction : ToggleAction() {
    override fun getActionUpdateThread() = ActionUpdateThread.EDT
    override fun isSelected(event: AnActionEvent) = event.project?.service<StabilitySettings>()?.state?.showStable ?: false
    override fun setSelected(event: AnActionEvent, state: Boolean) {
        event.project?.let { it.service<StabilitySettings>().state.showStable = state; it.service<StabilityService>().schedule() }
    }
}
