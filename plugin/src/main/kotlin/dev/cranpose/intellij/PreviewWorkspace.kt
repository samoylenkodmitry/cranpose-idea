package dev.cranpose.intellij

import com.google.gson.Gson
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import com.intellij.ide.impl.isTrusted
import com.intellij.openapi.Disposable
import com.intellij.openapi.components.service
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.application.ModalityState
import com.intellij.openapi.application.PathManager
import com.intellij.openapi.fileChooser.FileChooserFactory
import com.intellij.openapi.fileChooser.FileSaverDescriptor
import com.intellij.openapi.fileEditor.OpenFileDescriptor
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import java.awt.BasicStroke
import java.awt.Color
import java.awt.Dimension
import java.awt.event.HierarchyEvent
import java.nio.file.Path
import javax.imageio.ImageIO
import javax.swing.JLayeredPane

/** IDE services and native surface placement for the Rust/Cranpose Studio. */
class PreviewWorkspace(private val project: Project, private val source: VirtualFile? = null) : JLayeredPane(), Disposable {
    private val service = CranposeProjectService.get(project)
    private val gson = Gson()
    private val cache = Path.of(PathManager.getSystemPath(), "cranpose-dev")
    private val binary: Path by lazy { UiBinary.override() ?: UiBinary.extractBundled(Path.of(PathManager.getSystemPath(), "cranpose-ui")) }
    private val studioPanel = CranposePanel({ listOf(binary.toString()) }, log = { com.intellij.openapi.diagnostic.Logger.getInstance(PreviewWorkspace::class.java).info("Cranpose Studio: $it") }, environment = { mapOf("CRANPOSE_STUDIO" to "1") })
    private val viewport = javax.swing.JPanel(null).apply { isOpaque = false; isVisible = false }
    private var ready = false
    private var closed = false
    private val pending = mutableListOf<String>()
    private var active: Child? = null
    private var candidate: Child? = null
    private var placement: JsonObject? = null

    private data class Child(val id: Long, val panel: CranposePanel, val lifetime: Disposable)

    init {
        minimumSize = Dimension(300, 240)
        add(studioPanel)
        setLayer(studioPanel, DEFAULT_LAYER)
        add(viewport)
        setLayer(viewport, PALETTE_LAYER)
        IdeBridge(project, studioPanel, this, ::connected, ::handle)
        studioPanel.onStopped = { ready = false; viewport.isVisible = false }
        service.subscribe(this) { if (ready) studioPanel.send("cranpose.project", service.json()) }
        addHierarchyListener { event ->
            if (event.changeFlags and HierarchyEvent.SHOWING_CHANGED.toLong() != 0L && isShowing && !ready && !closed) studioPanel.start()
        }
    }

    override fun doLayout() { studioPanel.setBounds(0, 0, width, height); applyPlacement() }

    fun showFunction(name: String) = command(mapOf("action" to "showFunction", "name" to name))
    fun showBuilt(target: CargoTarget, executable: Path) = command(mapOf("action" to "showBuilt", "target" to target))
    fun buildPreview() = command(mapOf("action" to "build"))
    fun previewTarget(target: CargoTarget) = command(mapOf("action" to "showTarget", "target" to target))

    private fun command(value: Map<String, Any>) {
        val payload = gson.toJson(value)
        if (ready) studioPanel.send("studio.command", payload) else pending.add(payload)
    }

    private fun connected() {
        if (closed) return
        ready = true
        studioPanel.send("studio.init", gson.toJson(mapOf("root" to service.snapshot.root.ifBlank { project.basePath.orEmpty() }, "cache" to cache.toString(), "source" to source?.path.orEmpty(), "settings" to StudioSettings.get(project))))
        studioPanel.send("cranpose.project", service.json())
        pending.forEach { studioPanel.send("studio.command", it) }; pending.clear()
        if (service.snapshot.targets.isEmpty() && !service.snapshot.busy) service.refresh()
    }

    private fun handle(channel: String, payload: String): Boolean {
        if (channel != "studio.host") return false
        if (closed) return true
        val request = runCatching { JsonParser.parseString(payload).asJsonObject }.getOrNull() ?: return true
        when (request.string("action")) {
            "start" -> startChild(request)
            "stop" -> { candidate?.let(::closeChild); candidate = null; active?.let(::closeChild); active = null; applyPlacement() }
            "layout" -> { placement = request; applyPlacement() }
            "message" -> active?.takeIf { it.id == request.get("session")?.asLong }?.panel?.send(request.string("channel"), request.string("payload"))
            "settings" -> runCatching { gson.fromJson(request.get("value"), StudioSettings.Data::class.java) }.getOrNull()?.let {
                project.service<StudioSettings>().loadState(it)
            }
            "navigate" -> {
                val file = LocalFileSystem.getInstance().findFileByPath(request.string("file"))
                file?.let { OpenFileDescriptor(project, it, ((request.get("line")?.asInt ?: 1) - 1).coerceAtLeast(0), 0).navigate(true) }
            }
            "configure" -> service.snapshot.targets.firstOrNull { it.id == request.string("target") }?.let { CranposeRunConfiguration.create(project, it) }
            "export" -> exportImage()
        }
        return true
    }

    private fun startChild(request: JsonObject) {
        val id = request.get("session")?.asLong ?: return
        if (!project.isTrusted()) { event(id, "stopped", mapOf("message" to "Trust this project before running its code")); return }
        val options = request.getAsJsonObject("options") ?: return
        val target = service.snapshot.targets.firstOrNull { it.packageName == options.string("package") && it.name == options.string("target") && it.kind == options.string("kind") }
        if (target == null) { event(id, "stopped", mapOf("message" to "Refresh the Cargo workspace to select this target")); return }
        options.addProperty("root", service.snapshot.root)
        options.addProperty("cache", cache.toString())
        candidate?.let(::closeChild)
        val lifetime = Disposer.newDisposable("Cranpose native preview $id")
        Disposer.register(this, lifetime)
        val preview = request.string("preview")
        val panel = CranposePanel(
            command = { listOf(binary.toString(), "--dev-run", options.toString()) },
            workingDirectory = Path.of(service.snapshot.root),
            log = { event(id, "log", mapOf("line" to it)) },
            environment = { if (preview.isBlank()) emptyMap() else mapOf("CRANPOSE_PREVIEW" to preview) },
            connectTimeoutMillis = 1_200_000,
        )
        val child = Child(id, panel, lifetime)
        candidate = child
        EditorOverlay(project, panel, lifetime)
        panel.onConnected = {
            if (!closed && candidate === child) {
                active?.let(::closeChild)
                active = child; candidate = null
                viewport.add(panel); applyPlacement(); revalidate(); repaint()
                event(id, "connected")
            } else panel.close()
        }
        panel.onAppMessage = { name, body -> event(id, "message", mapOf("channel" to name, "payload" to body)) }
        panel.onStopped = { message ->
            if (candidate === child) { candidate = null; closeChild(child); event(id, "failed", mapOf("message" to message, "fallbackSession" to (active?.id ?: 0))) }
            else if (active === child) { active = null; closeChild(child); event(id, "stopped", mapOf("message" to message)) }
        }
        panel.onPointerPress = { point ->
            val pick = placement?.get("pick")?.asBoolean == true
            if (pick) event(id, "pointer", mapOf("x" to point.x / panel.contentScale, "y" to point.y / panel.contentScale))
            !pick
        }
        panel.onScroll = { wheel ->
            if (wheel.isAltDown) {
                val delta = -wheel.preciseWheelRotation * 40.0
                event(id, "pan", mapOf("x" to if (wheel.isShiftDown) delta else 0.0, "y" to if (wheel.isShiftDown) 0.0 else delta))
                false
            } else true
        }
        panel.paintOverlay = { graphics ->
            placement?.get("selected")?.takeUnless { it.isJsonNull }?.asJsonObject?.let { rectangle ->
                val scale = panel.contentScale
                val bounds = java.awt.geom.Rectangle2D.Double(rectangle["x"].asDouble * scale, rectangle["y"].asDouble * scale, rectangle["width"].asDouble * scale, rectangle["height"].asDouble * scale)
                graphics.color = Color(0x36, 0xA2, 0xEB, 35); graphics.fill(bounds)
                graphics.color = Color(0x36, 0xA2, 0xEB); graphics.stroke = BasicStroke(1.5f); graphics.draw(bounds)
            }
        }
        panel.start()
    }

    private fun applyPlacement() {
        viewport.isVisible = active != null
        val request = placement ?: return
        val panel = active?.panel ?: return
        val scale = request["scale"]?.asDouble?.takeIf { it.isFinite() }?.coerceIn(0.05, 4.0) ?: 1.0
        if (panel.contentScale != scale) panel.contentScale = scale
        val clip = request.getAsJsonObject("viewport")
        viewport.setBounds(0, clip["y"].asDouble.toInt(), width, clip["height"].asDouble.toInt().coerceAtLeast(1))
        val bounds = java.awt.Rectangle(request["x"].asDouble.toInt(), request["y"].asDouble.toInt() - viewport.y, request["width"].asDouble.toInt().coerceAtLeast(1), request["height"].asDouble.toInt().coerceAtLeast(1))
        if (panel.bounds != bounds) panel.bounds = bounds
        panel.setTheme(request["dark"]?.asBoolean ?: false)
        panel.repaint()
    }

    private fun event(id: Long, event: String, values: Map<String, Any> = emptyMap()) {
        if (!closed) studioPanel.send("studio.child", gson.toJson(values + mapOf("session" to id, "event" to event)))
    }

    private fun closeChild(child: Child) {
        child.panel.onStopped = {}
        child.panel.close(); viewport.remove(child.panel)
        viewport.isVisible = viewport.componentCount > 0
        if (!Disposer.isDisposed(child.lifetime)) Disposer.dispose(child.lifetime)
        revalidate(); repaint()
    }

    private fun exportImage() {
        val image = active?.panel?.canvas?.snapshot() ?: return
        ApplicationManager.getApplication().invokeLater({
            if (!closed) {
                val file = FileChooserFactory.getInstance().createSaveFileDialog(FileSaverDescriptor("Export Cranpose preview", "Save the rendered preview", "png"), project).save(source?.parent, "cranpose-preview.png")?.file
                file?.let { ImageIO.write(image, "png", it) }
            }
        }, ModalityState.nonModal(), project.disposed)
    }

    override fun dispose() {
        closed = true; candidate?.let(::closeChild); candidate = null; active?.let(::closeChild); active = null; studioPanel.close(); pending.clear()
    }
}
