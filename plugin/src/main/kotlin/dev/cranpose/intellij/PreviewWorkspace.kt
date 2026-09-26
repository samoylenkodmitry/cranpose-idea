package dev.cranpose.intellij

import com.intellij.openapi.Disposable
import com.intellij.icons.AllIcons
import com.intellij.openapi.actionSystem.*
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.fileChooser.FileChooserFactory
import com.intellij.openapi.fileChooser.FileSaverDescriptor
import com.intellij.openapi.fileEditor.OpenFileDescriptor
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.vfs.LocalFileSystem
import com.intellij.openapi.vfs.VirtualFile
import com.intellij.openapi.vfs.VirtualFileManager
import com.intellij.openapi.vfs.newvfs.BulkFileListener
import com.intellij.openapi.vfs.newvfs.events.VFileEvent
import com.intellij.ui.JBColor
import com.intellij.ui.components.*
import com.intellij.ui.treeStructure.Tree
import com.intellij.util.ui.JBUI
import java.awt.*
import java.awt.event.MouseAdapter
import java.awt.event.MouseEvent
import java.nio.file.Path
import javax.imageio.ImageIO
import javax.swing.*
import javax.swing.table.DefaultTableModel
import javax.swing.tree.DefaultMutableTreeNode
import javax.swing.tree.DefaultTreeModel
import javax.swing.tree.TreePath

class PreviewWorkspace(private val project: Project, private val source: VirtualFile? = null) :
    JPanel(BorderLayout()), Disposable {
    private val service = CranposeProjectService.get(project)
    private val saved = StudioSettings.get(project)
    private val targetBox = JComboBox<CargoTarget>()
    private val variants = JComboBox<Any>()
    private val status = JBLabel("Select a target and build a preview.")
    private val build = JButton("Build")
    private val pick = JToggleButton("Pick")
    private val inspect = JToggleButton("Inspect", saved.inspect)
    private val live = JToggleButton("Live", true)
    private val dark = JCheckBox("Dark", saved.dark)
    private val auto = JToggleButton("On save", saved.autoBuild)
    private val widthInput = JSpinner(SpinnerNumberModel(saved.width, 120, 4096, 1))
    private val heightInput = JSpinner(SpinnerNumberModel(saved.height, 120, 4096, 1))
    private val zoom = JComboBox(arrayOf("Fit", "25%", "50%", "75%", "100%", "125%", "150%", "200%"))
    private val stage = JPanel(GridBagLayout())
    private val viewport = JBScrollPane(stage)
    private val tree = Tree(DefaultMutableTreeNode("Layout"))
    private val propertyModel = object : DefaultTableModel(arrayOf("Property", "Value"), 0) {
        override fun isCellEditable(row: Int, column: Int): Boolean = false
    }
    private val properties = JTable(propertyModel)
    private val problemsModel = DefaultListModel<CargoDiagnostic>()
    private val problems = JBList(problemsModel)
    private val inspector = JSplitPane(JSplitPane.HORIZONTAL_SPLIT, JBScrollPane(tree), JBScrollPane(properties))
    private val content = JSplitPane(JSplitPane.VERTICAL_SPLIT, viewport, inspector)
    private val problemPane = JBScrollPane(problems)
    private val body = JPanel(BorderLayout())
    private var panel: CranposePanel? = null
    private var binary: Path? = null
    private var builtTarget: CargoTarget? = null
    private var snapshot: InspectionSnapshot? = null
    private var selectedNode: String? = null
    private var requestedFunction: String? = null
    private var requestedId = saved.preview
    private var request = 0L
    private var waitingSince = 0L
    private var updating = false
    private var closed = false
    private var connected = false
    private var sessionDisposable: Disposable? = null
    private var buildWhenReady = false
    private val receiveBinary: (CargoTarget, Path) -> Unit = { target, executable ->
        if (!closed) {
            binary = executable
            builtTarget = target
            startPreview()
        }
    }
    private val capture = Timer(500) {
        if (connected && isShowing && (inspect.isSelected || pick.isSelected) && live.isSelected) requestSnapshot()
    }
    private val rebuild = Timer(700) {
        if (auto.isSelected && binary != null && !closed) buildPreview()
    }.apply { isRepeats = false }

    init {
        minimumSize = Dimension(300, 240)
        targetBox.renderer = object : DefaultListCellRenderer() {
            override fun getListCellRendererComponent(list: JList<*>?, value: Any?, index: Int, selected: Boolean, focus: Boolean): Component =
                super.getListCellRendererComponent(list, (value as? CargoTarget)?.label ?: value, index, selected, focus)
        }
        val actions = JPanel(BorderLayout(6, 0)).apply {
            border = JBUI.Borders.empty(5, 8)
            add(targetBox, BorderLayout.CENTER)
            add(actionBar(
                action("Build preview", AllIcons.Actions.Execute, { buildPreview() }, { build.isEnabled }),
                action("Stop preview", AllIcons.Actions.Suspend, { stopPreview(); service.cancelPreview(target()?.id.orEmpty(), receiveBinary) }),
                action("Refresh Cargo workspace", AllIcons.Actions.Refresh, { service.refresh() }),
                toggle("Rebuild on save", AllIcons.Actions.Refresh, auto),
                action("Save run configuration", AllIcons.General.Settings, { target()?.let { CranposeRunConfiguration.create(project, it) } }),
            ), BorderLayout.EAST)
        }
        val controls = JPanel(BorderLayout(6, 0)).apply { border = JBUI.Borders.empty(3, 8) }
        variants.addItem("Application")
        controls.add(variants, BorderLayout.CENTER)
        val sizing = JPanel(FlowLayout(FlowLayout.TRAILING, 4, 0))
        widthInput.preferredSize = JBUI.size(76, 28)
        heightInput.preferredSize = JBUI.size(76, 28)
        sizing.add(widthInput)
        sizing.add(JBLabel("×"))
        sizing.add(heightInput)
        sizing.add(dark)
        controls.add(sizing, BorderLayout.EAST)
        val inspectionTools = JPanel(BorderLayout()).apply {
            border = JBUI.Borders.empty(2, 8)
            add(actionBar(
                toggle("Pick element", AllIcons.Actions.Find, pick),
                toggle("Layout inspector", AllIcons.Toolwindows.ToolWindowStructure, inspect),
                toggle("Live layout updates", AllIcons.Actions.Resume, live),
                action("Capture layout", AllIcons.Actions.Refresh, { requestSnapshot(true) }),
                action("Go to composable source", AllIcons.Actions.EditSource, { navigatePreview() }),
                action("Export preview as PNG", com.intellij.openapi.util.IconLoader.getIcon("/icons/export.svg", PreviewWorkspace::class.java), { exportImage() }),
            ), BorderLayout.CENTER)
            add(zoom, BorderLayout.EAST)
        }
        val top = JPanel().apply {
            layout = BoxLayout(this, BoxLayout.Y_AXIS)
            add(actions)
            add(controls)
            add(inspectionTools)
        }
        add(top, BorderLayout.NORTH)
        stage.border = JBUI.Borders.empty(24)
        stage.background = JBColor(0xE8EAED, 0x202124)
        stage.add(JBLabel("Build to render this application or a registered component."))
        viewport.border = JBUI.Borders.empty()
        inspector.resizeWeight = 0.45
        inspector.dividerSize = 1
        inspector.border = JBUI.Borders.empty()
        inspector.minimumSize = Dimension(0, 100)
        content.resizeWeight = 0.72
        content.border = JBUI.Borders.empty()
        content.dividerSize = 1
        content.bottomComponent = if (inspect.isSelected) inspector else null
        properties.rowHeight = JBUI.scale(24)
        properties.autoResizeMode = JTable.AUTO_RESIZE_LAST_COLUMN
        properties.fillsViewportHeight = true
        tree.isRootVisible = false
        tree.showsRootHandles = true
        problemPane.preferredSize = Dimension(0, 95)
        problemPane.isVisible = false
        problems.cellRenderer = object : DefaultListCellRenderer() {
            override fun getListCellRendererComponent(list: JList<*>?, value: Any?, index: Int, selected: Boolean, focus: Boolean): Component {
                val diagnostic = value as? CargoDiagnostic
                return super.getListCellRendererComponent(list, diagnostic?.let {
                    "${it.level}: ${it.message}  ·  ${it.file}:${it.line}"
                } ?: value, index, selected, focus)
            }
        }
        problems.addMouseListener(object : MouseAdapter() {
            override fun mouseClicked(e: MouseEvent) {
                if (e.clickCount == 2) problems.selectedValue?.let { navigate(it.file, it.line, it.column) }
            }
        })
        body.add(content, BorderLayout.CENTER)
        body.add(problemPane, BorderLayout.SOUTH)
        add(body, BorderLayout.CENTER)
        status.border = JBUI.Borders.empty(7, 10)
        add(status, BorderLayout.SOUTH)
        zoom.selectedItem = if (saved.fit) "Fit" else "${(saved.zoom * 100).toInt()}%"
        viewport.addComponentListener(object : java.awt.event.ComponentAdapter() {
            override fun componentResized(event: java.awt.event.ComponentEvent) { if (saved.fit) updateSize() }
        })
        build.addActionListener { buildPreview() }
        targetBox.addActionListener {
            if (!updating) target()?.let { saved.target = it.id; service.select(it.id) }
        }
        variants.addActionListener {
            if (!updating) {
                val selected = variants.selectedItem as? PreviewDescriptor
                requestedId = selected?.id.orEmpty()
                saved.preview = requestedId
                selected?.let {
                    updating = true
                    widthInput.value = it.width.coerceIn(120, 4096)
                    heightInput.value = it.height.coerceIn(120, 4096)
                    dark.isSelected = it.dark
                    updating = false
                }
                if (binary != null) startPreview()
            }
        }
        widthInput.addChangeListener { updateSize() }
        heightInput.addChangeListener { updateSize() }
        zoom.addActionListener { updateSize() }
        dark.addActionListener { saved.dark = dark.isSelected; panel?.setTheme(dark.isSelected) }
        auto.addActionListener { saved.autoBuild = auto.isSelected }
        inspect.addActionListener {
            saved.inspect = inspect.isSelected
            content.bottomComponent = if (inspect.isSelected) inspector else null
            if (inspect.isSelected) {
                content.setDividerLocation(0.72)
                inspector.setDividerLocation(0.45)
                requestSnapshot(true)
            }
        }
        pick.toolTipText = "Select an element without sending its click to the application"
        pick.addActionListener {
            panel?.cursor = Cursor.getPredefinedCursor(if (pick.isSelected) Cursor.CROSSHAIR_CURSOR else Cursor.DEFAULT_CURSOR)
            if (pick.isSelected && !inspect.isSelected) inspect.doClick()
            requestSnapshot(true)
        }
        live.toolTipText = "Refresh layout data every half second; turn off to inspect a captured frame"
        tree.addTreeSelectionListener {
            val node = (tree.lastSelectedPathComponent as? DefaultMutableTreeNode)?.userObject as? InspectionNode
            selectedNode = node?.id
            showProperties(node)
            panel?.repaint()
        }
        service.subscribe(this) { state ->
            updating = true
            val selected = target()?.id ?: saved.target.takeIf { it.isNotBlank() } ?: state.selected
            if ((0 until targetBox.itemCount).map { targetBox.getItemAt(it) } != state.targets) {
                targetBox.removeAllItems()
                state.targets.forEach(targetBox::addItem)
            }
            state.targets.firstOrNull { it.id == selected }?.let { targetBox.selectedItem = it }
            updating = false
            build.isEnabled = !state.busy && target() != null
            build.text = if (state.busy) "Building…" else if (binary == null) "Build" else "Rebuild"
            if (state.busy || binary == null || state.status.startsWith("Cargo failed")) status.text = state.status
            problemsModel.clear()
            state.diagnostics.forEach(problemsModel::addElement)
            problemPane.isVisible = state.diagnostics.isNotEmpty()
            if (state.status.startsWith("Cargo failed") && binary != null) status.text = "Build failed · showing the last successful preview"
            if (buildWhenReady && !state.busy && target() != null) {
                buildWhenReady = false
                later { buildPreview() }
            }
        }
        project.messageBus.connect(this).subscribe(VirtualFileManager.VFS_CHANGES, object : BulkFileListener {
            override fun after(events: List<VFileEvent>) {
                val root = service.snapshot.root.takeIf { it.isNotBlank() }?.let(Path::of) ?: return
                if (events.any {
                    val path = Path.of(it.path)
                    path.startsWith(root) && path.none { part -> part.toString() in setOf("target", ".git") } &&
                        (it.path.endsWith(".rs") || it.path.endsWith(".wgsl") || path.fileName.toString() == "Cargo.toml")
                }) rebuild.restart()
            }
        })
        capture.start()
        if (service.snapshot.targets.isEmpty() && !service.snapshot.busy) service.refresh()
    }

    fun showFunction(name: String) {
        requestedFunction = name
        if (binary == null) {
            if (target() == null) buildWhenReady = true else buildPreview()
        } else selectRequestedVariant()
    }

    fun showBuilt(target: CargoTarget, executable: Path) {
        updating = true
        targetBox.selectedItem = target
        updating = false
        receiveBinary(target, executable)
    }

    private fun actionBar(vararg actions: AnAction): JComponent =
        ActionManager.getInstance().createActionToolbar("Cranpose.Preview", DefaultActionGroup(*actions), true).apply {
            targetComponent = this@PreviewWorkspace
        }.component

    private fun action(label: String, icon: Icon, perform: () -> Unit, enabled: () -> Boolean = { true }) =
        object : AnAction(label, label, icon) {
            override fun getActionUpdateThread() = ActionUpdateThread.EDT
            override fun update(event: AnActionEvent) { event.presentation.isEnabled = enabled() }
            override fun actionPerformed(event: AnActionEvent) = perform()
        }

    private fun toggle(label: String, icon: Icon, button: AbstractButton) = object : ToggleAction(label, label, icon) {
        override fun getActionUpdateThread() = ActionUpdateThread.EDT
        override fun isSelected(event: AnActionEvent) = button.isSelected
        override fun setSelected(event: AnActionEvent, state: Boolean) { if (button.isSelected != state) button.doClick() }
    }

    private fun target(): CargoTarget? = targetBox.selectedItem as? CargoTarget

    fun buildPreview() {
        target()?.let { service.execute(CargoTask.PREVIEW, it.id, receiveBinary) }
    }

    private fun startPreview() {
        val executable = binary ?: return
        val target = builtTarget ?: return
        stopPreview()
        val chosen = requestedId
        val surface = CranposePanel({ listOf(executable.toString()) }, Path.of(target.manifest).parent,
            log = { line -> if (line.contains("panicked") || line.contains("error")) later { status.text = line.take(180) } },
            environment = { if (chosen.isBlank()) emptyMap() else mapOf("CRANPOSE_PREVIEW" to chosen) })
        panel = surface
        surface.onConnected = {
            if (panel === surface) {
                connected = true
                status.text = "Interactive preview · ${target.name}"
                surface.setTheme(dark.isSelected)
                if (inspect.isSelected) {
                    content.setDividerLocation(0.72)
                    inspector.setDividerLocation(0.45)
                }
                requestSnapshot(true)
            }
        }
        surface.onStopped = { message -> if (panel === surface) { connected = false; status.text = message } }
        surface.onPointerPress = { point ->
            if (pick.isSelected) {
                snapshot?.pick(point.x / saved.zoom, point.y / saved.zoom)?.let { selectNode(it.id) }
                false
            } else true
        }
        surface.paintOverlay = { graphics ->
            snapshot?.nodes?.firstOrNull { it.id == selectedNode }?.let { node ->
                val bounds = java.awt.geom.Rectangle2D.Double(node.x * saved.zoom, node.y * saved.zoom, node.width * saved.zoom, node.height * saved.zoom)
                graphics.color = Color(0x36, 0xA2, 0xEB, 35)
                graphics.fill(bounds)
                graphics.color = Color(0x36, 0xA2, 0xEB)
                graphics.stroke = BasicStroke(1.5f)
                graphics.draw(bounds)
            }
        }
        surface.onAppMessage = { channel, payload ->
            when (channel) {
                "cranpose.previews.v1" -> runCatching { PreviewDescriptor.parse(payload) }
                    .onSuccess { descriptors -> later { if (panel === surface) updateVariants(descriptors) } }
                    .onFailure { later { status.text = "Invalid preview registry: ${it.message}" } }
                "cranpose.inspector.v2.snapshot" -> runCatching { InspectionSnapshot.parse(payload) }
                    .onSuccess { value -> later { if (panel === surface && value.requestId >= (snapshot?.requestId ?: 0)) {
                        waitingSince = 0
                        updateTree(value)
                    } } }
                    .onFailure { later { waitingSince = 0; status.text = "Invalid layout snapshot: ${it.message}" } }
            }
        }
        val lifetime = Disposer.newDisposable("Cranpose preview session")
        sessionDisposable = lifetime
        Disposer.register(this, lifetime)
        EditorOverlay(project, surface, lifetime)
        surface.setTheme(dark.isSelected)
        stage.removeAll()
        stage.add(surface)
        updateSize()
        stage.revalidate()
        stage.repaint()
        surface.start()
    }

    private fun stopPreview() {
        connected = false
        sessionDisposable?.let(Disposer::dispose)
        sessionDisposable = null
        panel?.close()
        panel = null
        snapshot = null
        selectedNode = null
        waitingSince = 0
        tree.model = DefaultTreeModel(DefaultMutableTreeNode("Layout"))
        showProperties(null)
        stage.removeAll()
        stage.add(JBLabel("Preview stopped. Build to start again."))
        stage.revalidate()
        stage.repaint()
        status.text = "Preview stopped"
    }

    private fun updateSize() {
        if (updating) return
        saved.width = widthInput.value as Int
        saved.height = heightInput.value as Int
        saved.fit = zoom.selectedItem == "Fit"
        saved.zoom = if (saved.fit) minOf(
            (viewport.width - 52.0) / saved.width,
            (viewport.height - 52.0) / saved.height,
            1.0,
        ).coerceAtLeast(0.25) else zoom.selectedItem.toString().removeSuffix("%").toDouble() / 100
        saved.dark = dark.isSelected
        panel?.let {
            it.contentScale = saved.zoom
            it.preferredSize = Dimension((saved.width * saved.zoom).toInt(), (saved.height * saved.zoom).toInt())
            it.minimumSize = it.preferredSize
        }
        stage.revalidate()
        stage.repaint()
    }

    private fun updateVariants(descriptors: List<PreviewDescriptor>) {
        updating = true
        variants.removeAllItems()
        variants.addItem("Application")
        descriptors.forEach(variants::addItem)
        descriptors.firstOrNull { it.id == requestedId }?.let { variants.selectedItem = it }
        updating = false
        if (requestedFunction != null) selectRequestedVariant()
        else if (requestedId.isNotBlank() && descriptors.none { it.id == requestedId }) {
            requestedId = ""
            saved.preview = ""
            status.text = "The selected preview no longer exists. Showing the application."
            startPreview()
        }
    }

    private fun selectRequestedVariant() {
        val name = requestedFunction ?: return
        val matches = (0 until variants.itemCount).mapNotNull { variants.getItemAt(it) as? PreviewDescriptor }
            .filter { it.function == name }
        val matchingFile = matches.filter { descriptor ->
            source == null || Path.of(builtTarget?.manifest ?: return@filter false).parent.resolve(descriptor.file).normalize().toString() == source.path
        }
        val chosen = matchingFile.firstOrNull() ?: matches.singleOrNull()
        if (chosen != null) {
            requestedFunction = null
            variants.selectedItem = chosen
        } else status.text = "No registered preview for $name. Add #[cranpose::preview] to a parameterless fixture."
    }

    private fun requestSnapshot(force: Boolean = false) {
        if (!connected) return
        val now = System.currentTimeMillis()
        if (waitingSince != 0L && now - waitingSince < 3000 && !force) return
        if (waitingSince != 0L && now - waitingSince >= 3000) status.text = "Layout inspection needs a Cranpose build with inspection v2 support."
        waitingSince = now
        panel?.send("cranpose.inspector.v2.request", (++request).toString())
    }

    private fun updateTree(value: InspectionSnapshot) {
        val expanded = (0 until tree.rowCount).filter(tree::isExpanded).mapNotNull {
            ((tree.getPathForRow(it)?.lastPathComponent as? DefaultMutableTreeNode)?.userObject as? InspectionNode)?.id
        }.toSet()
        val first = snapshot == null
        snapshot = value
        val root = DefaultMutableTreeNode("Layout")
        val nodes = HashMap<String, DefaultMutableTreeNode>()
        value.nodes.forEach { node ->
            val item = DefaultMutableTreeNode(node)
            (nodes[node.parent] ?: root).add(item)
            nodes[node.id] = item
        }
        val selection = selectedNode
        tree.model = DefaultTreeModel(root)
        nodes.filterKeys { it in expanded }.values.forEach { tree.expandPath(TreePath(it.path)) }
        if (first) for (row in 0 until minOf(tree.rowCount + 3, 8)) tree.expandRow(row)
        selection?.let { id -> nodes[id]?.let { tree.selectionPath = TreePath(it.path) } }
        if (selection != null && selection !in nodes) { selectedNode = null; showProperties(null) }
        if (!service.snapshot.busy && !service.snapshot.status.startsWith("Cargo failed")) {
            status.text = "${value.nodes.size} nodes · ${value.captureMicros / 1000.0} ms capture${if (value.truncated) " · tree truncated" else ""}"
        }
        panel?.repaint()
    }

    private fun selectNode(id: String) {
        val root = tree.model.root as? DefaultMutableTreeNode ?: return
        val enumeration = root.depthFirstEnumeration()
        while (enumeration.hasMoreElements()) {
            val node = enumeration.nextElement() as DefaultMutableTreeNode
            if ((node.userObject as? InspectionNode)?.id == id) {
                val path = TreePath(node.path)
                tree.selectionPath = path
                tree.scrollPathToVisible(path)
                break
            }
        }
    }

    private fun showProperties(node: InspectionNode?) {
        propertyModel.rowCount = 0
        if (node == null) return
        fun row(name: String, value: Any) { propertyModel.addRow(arrayOf(name, value)) }
        row("Type", node.kind)
        row("Identity", node.id)
        row("Position", "%.1f, %.1f".format(node.x, node.y))
        row("Size", "%.1f × %.1f".format(node.width, node.height))
        node.text?.let { row("Text", it) }
        node.sources.forEach { row(it.name, "${it.file}:${it.line}") }
        node.modifiers.forEach { modifier ->
            if (modifier.properties.isEmpty()) row(modifier.name, "—")
            else modifier.properties.forEach { row("${modifier.name}.${it.name}", it.value) }
        }
    }

    private fun navigatePreview() {
        val origins = snapshot?.nodes?.firstOrNull { it.id == selectedNode }?.sources.orEmpty()
        for (origin in origins.asReversed()) {
            val root = Path.of(service.snapshot.root)
            val packageRoot = builtTarget?.manifest?.let { Path.of(it).parent } ?: root
            val candidates = listOf(Path.of(origin.manifestDir).resolve(origin.file), packageRoot.resolve(origin.file), root.resolve(origin.file))
            val path = candidates.firstOrNull { it.normalize().startsWith(root) && java.nio.file.Files.isRegularFile(it) }
            if (path != null) { navigate(path.toString(), origin.line, 1); return }
        }
        val selected = variants.selectedItem as? PreviewDescriptor
        if (selected != null) {
            val base = builtTarget?.manifest?.let { Path.of(it).parent } ?: return
            navigate(base.resolve(selected.file).normalize().toString(), selected.line, 1)
        } else (source?.path ?: builtTarget?.source)?.let { navigate(it, 1, 1) }
    }

    private fun navigate(path: String, line: Int, column: Int) {
        val absolute = Path.of(service.snapshot.root).resolve(path).normalize()
        LocalFileSystem.getInstance().findFileByPath(absolute.toString())?.let {
            OpenFileDescriptor(project, it, (line - 1).coerceAtLeast(0), (column - 1).coerceAtLeast(0)).navigate(true)
        }
    }

    private fun exportImage() {
        val image = panel?.canvas?.snapshot() ?: run { status.text = "Build a preview before exporting."; return }
        val file = FileChooserFactory.getInstance().createSaveFileDialog(
            FileSaverDescriptor("Export Cranpose Preview", "Save the rendered preview at its current pixel density.", "png"), project)
            .save(source?.parent, "cranpose-preview.png")?.file ?: return
        ApplicationManager.getApplication().executeOnPooledThread {
            runCatching { ImageIO.write(image, "png", file) }
                .onSuccess { later { status.text = "Exported ${file.name}" } }
                .onFailure { later { status.text = "Export failed: ${it.message}" } }
        }
    }

    private fun later(action: () -> Unit) {
        ApplicationManager.getApplication().invokeLater({ if (!closed && !project.isDisposed) action() }, project.disposed)
    }

    override fun dispose() {
        closed = true
        capture.stop()
        rebuild.stop()
        service.cancelPreview(target()?.id.orEmpty(), receiveBinary)
        stopPreview()
    }
}
