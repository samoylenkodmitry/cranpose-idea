package dev.cranpose.intellij

import com.google.gson.Gson
import com.intellij.execution.configurations.GeneralCommandLine
import com.intellij.execution.process.CapturingProcessHandler
import com.intellij.execution.process.KillableColoredProcessHandler
import com.intellij.execution.process.ProcessListener
import com.intellij.execution.process.ProcessEvent
import com.intellij.execution.ui.ConsoleViewContentType
import com.intellij.execution.filters.TextConsoleBuilderFactory
import com.intellij.execution.ui.RunContentDescriptor
import com.intellij.execution.ui.RunContentManager
import com.intellij.execution.executors.DefaultRunExecutor
import com.intellij.ide.impl.isTrusted
import com.intellij.openapi.Disposable
import com.intellij.openapi.application.ApplicationManager
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.service
import com.intellij.openapi.fileEditor.FileDocumentManager
import com.intellij.openapi.project.Project
import com.intellij.openapi.util.Disposer
import com.intellij.openapi.util.Key
import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.CopyOnWriteArrayList
import java.util.concurrent.atomic.AtomicBoolean

data class ProjectSnapshot(
    val status: String = "Open a Cargo project, then refresh.",
    val busy: Boolean = false,
    val targets: List<CargoTarget> = emptyList(),
    val selected: String = "",
    val root: String = "",
    val diagnostics: List<CargoDiagnostic> = emptyList(),
)

@Service(Service.Level.PROJECT)
class CranposeProjectService(private val project: Project) : Disposable {
    @Volatile var snapshot = ProjectSnapshot()
        private set
    private val listeners = CopyOnWriteArrayList<(ProjectSnapshot) -> Unit>()
    private val busy = AtomicBoolean()
    @Volatile private var process: com.intellij.execution.process.ProcessHandler? = null
    @Volatile private var disposed = false
    private var pendingPreview: String? = null
    private var activePreview: String? = null
    private var activeReceiver: ((CargoTarget, Path) -> Unit)? = null
    private var pendingReceiver: ((CargoTarget, Path) -> Unit)? = null
    private val gson = Gson()

    fun subscribe(parent: Disposable, listener: (ProjectSnapshot) -> Unit) {
        listeners.add(listener)
        Disposer.register(parent) { listeners.remove(listener) }
        listener(snapshot)
    }

    fun json(): String = gson.toJson(snapshot)

    fun select(id: String) {
        if (snapshot.targets.any { it.id == id }) publish(snapshot.copy(selected = id))
    }

    fun refresh() {
        if (!trusted() || !busy.compareAndSet(false, true)) return
        publish(snapshot.copy(status = "Reading Cargo workspace…", busy = true))
        ApplicationManager.getApplication().executeOnPooledThread {
            try {
                val base = project.basePath?.let(Path::of) ?: error("This project has no directory.")
                val manifest = base.resolve("Cargo.toml")
                require(Files.isRegularFile(manifest)) { "No Cargo.toml at the project root. Open the Cargo workspace directory." }
                val handler = CapturingProcessHandler(command(base, listOf("metadata", "--format-version", "1", "--no-deps", "--manifest-path", manifest.toString())))
                process = handler
                if (disposed) { handler.destroyProcess(); return@executeOnPooledThread }
                val output = handler.runProcess(60_000)
                check(!output.isTimeout) { "Cargo metadata timed out. Check the toolchain and refresh." }
                check(output.exitCode == 0) { output.stderr.ifBlank { "Cargo metadata failed (${output.exitCode})." } }
                val workspace = CargoWorkspace.parse(output.stdout)
                val selected = snapshot.selected.takeIf { id -> workspace.targets.any { it.id == id } }
                    ?: workspace.targets.firstOrNull()?.id.orEmpty()
                publish(ProjectSnapshot(
                    if (workspace.targets.isEmpty()) "No runnable Cranpose targets found. Add a binary or example depending on cranpose."
                    else "${workspace.targets.size} Cranpose targets · ${workspace.packages} workspace packages",
                    false, workspace.targets, selected, workspace.root))
            } catch (error: Exception) {
                publish(ProjectSnapshot(status = error.message ?: "Could not read Cargo workspace."))
            } finally {
                process = null
                busy.set(false)
            }
        }
    }

    fun execute(task: CargoTask, targetId: String = snapshot.selected, receiver: ((CargoTarget, Path) -> Unit)? = null) {
        later { executeOnEdt(task, targetId, receiver) }
    }

    private fun executeOnEdt(task: CargoTask, targetId: String, receiver: ((CargoTarget, Path) -> Unit)?) {
        if (!trusted()) return
        val target = snapshot.targets.firstOrNull { it.id == targetId } ?: return
        if (!busy.compareAndSet(false, true)) {
            if (task == CargoTask.PREVIEW) { pendingPreview = targetId; pendingReceiver = receiver }
            return
        }
        FileDocumentManager.getInstance().saveAllDocuments()
        activePreview = if (task == CargoTask.PREVIEW) targetId else null
        activeReceiver = receiver
        publish(snapshot.copy(status = "${task.verb.replaceFirstChar(Char::uppercase)}: ${target.label}", busy = true, diagnostics = emptyList()))
        try {
            val console = TextConsoleBuilderFactory.getInstance().createBuilder(project).console
            val handler = KillableColoredProcessHandler(command(Path.of(snapshot.root), CargoCommand.arguments(task, target)))
            process = handler
            val executable = java.util.concurrent.atomic.AtomicReference<Path?>()
            val lineBuffer = StringBuilder()
            val diagnostics = CopyOnWriteArrayList<CargoDiagnostic>()
            handler.addProcessListener(object : ProcessListener {
                override fun onTextAvailable(event: ProcessEvent, outputType: Key<*>) {
                    if (task != CargoTask.PREVIEW && task != CargoTask.CHECK) return
                    if (outputType != com.intellij.execution.process.ProcessOutputTypes.STDOUT) {
                        console.print(event.text, ConsoleViewContentType.NORMAL_OUTPUT)
                        return
                    }
                    synchronized(lineBuffer) {
                        lineBuffer.append(event.text)
                        while (true) {
                            val end = lineBuffer.indexOf("\n")
                            if (end < 0) break
                            val line = lineBuffer.substring(0, end)
                            CargoCommand.executable(line, target)?.let(executable::set)
                            CargoDiagnostic.parse(line)?.let { diagnostic ->
                                diagnostics.add(diagnostic)
                                console.print(diagnostic.rendered.ifBlank { diagnostic.message + "\n" },
                                    if (diagnostic.level == "error") ConsoleViewContentType.ERROR_OUTPUT else ConsoleViewContentType.NORMAL_OUTPUT)
                                if (diagnostic.file.isNotEmpty()) {
                                    val path = Path.of(snapshot.root).resolve(diagnostic.file).normalize()
                                    com.intellij.openapi.vfs.LocalFileSystem.getInstance().findFileByPath(path.toString())?.let { file ->
                                        console.printHyperlink("${diagnostic.file}:${diagnostic.line}:${diagnostic.column}\n",
                                            com.intellij.execution.filters.OpenFileHyperlinkInfo(project, file, diagnostic.line - 1, diagnostic.column - 1))
                                    }
                                }
                            }
                            lineBuffer.delete(0, end + 1)
                        }
                    }
                }

                override fun processTerminated(event: ProcessEvent) {
                    val binary = executable.get()
                    later {
                        process = null
                        busy.set(false)
                        val pending = pendingPreview
                        val pendingCallback = pendingReceiver
                        pendingPreview = null
                        pendingReceiver = null
                        val openPreview = task == CargoTask.PREVIEW && activePreview == targetId
                        val callback = activeReceiver
                        activePreview = null
                        activeReceiver = null
                        publish(snapshot.copy(busy = false, status = if (event.exitCode == 0)
                            "Finished: ${target.label}" else "Cargo failed (${event.exitCode}). See the Run console.", diagnostics = diagnostics.toList()))
                        if (pending != null) execute(CargoTask.PREVIEW, pending, pendingCallback)
                        else if (openPreview && event.exitCode == 0) {
                            if (binary != null) (callback ?: { t, b -> PreviewController.open(project, t, b) })(target, binary)
                            else publish(snapshot.copy(status = "Cargo produced no executable for ${target.label}."))
                        }
                    }
                }
            })
            if (task == CargoTask.RUN || task == CargoTask.TEST) console.attachToProcess(handler)
            console.print("cargo ${CargoCommand.arguments(task, target).joinToString(" ")}\n", ConsoleViewContentType.SYSTEM_OUTPUT)
            val descriptor = RunContentDescriptor(console, handler, console.component, "Cranpose: ${task.verb}")
            RunContentManager.getInstance(project).showRunContent(DefaultRunExecutor.getRunExecutorInstance(), descriptor)
            handler.startNotify()
        } catch (error: Exception) {
            process = null
            activePreview = null
            busy.set(false)
            publish(snapshot.copy(status = error.message ?: "Could not start Cargo.", busy = false))
        }
    }

    fun stop() { pendingPreview = null; activePreview = null; activeReceiver = null; pendingReceiver = null; process?.destroyProcess() }

    fun cancelPreview(targetId: String, receiver: ((CargoTarget, Path) -> Unit)? = null) {
        if (pendingPreview == targetId && pendingReceiver === receiver) { pendingPreview = null; pendingReceiver = null }
        if (activePreview == targetId && activeReceiver === receiver) {
            activePreview = null
            activeReceiver = null
            process?.destroyProcess()
        }
    }

    private fun trusted(): Boolean {
        if (project.isTrusted()) return true
        publish(snapshot.copy(status = "Trust this project in the IDE before running Cargo or previews."))
        return false
    }

    private fun publish(value: ProjectSnapshot) {
        if (disposed) return
        snapshot = value
        later { listeners.forEach { it(value) } }
    }

    private fun later(block: () -> Unit) {
        ApplicationManager.getApplication().invokeLater({ if (!disposed && !project.isDisposed) block() },
            com.intellij.openapi.application.ModalityState.nonModal(), project.disposed)
    }

    override fun dispose() {
        disposed = true
        process?.destroyProcess()
        listeners.clear()
    }

    companion object {
        fun get(project: Project): CranposeProjectService = project.service()
        fun command(directory: Path, arguments: List<String>): GeneralCommandLine {
            val executable = if (System.getProperty("os.name").startsWith("Windows")) "cargo.exe" else "cargo"
            val rustup = Path.of(System.getProperty("user.home"), ".cargo", "bin", executable)
            return GeneralCommandLine(listOf(if (Files.isExecutable(rustup)) rustup.toString() else executable) + arguments)
                .withWorkDirectory(directory.toFile()).withCharset(Charsets.UTF_8)
                .withParentEnvironmentType(GeneralCommandLine.ParentEnvironmentType.CONSOLE)
                .withEnvironment("CARGO_TERM_COLOR", "never")
        }
    }
}
