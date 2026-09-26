package dev.cranpose.intellij

import com.intellij.execution.Executor
import com.intellij.execution.RunManager
import com.intellij.execution.configurations.*
import com.intellij.execution.process.KillableColoredProcessHandler
import com.intellij.execution.process.ProcessHandler
import com.intellij.execution.runners.ExecutionEnvironment
import com.intellij.openapi.options.SettingsEditor
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.ComboBox
import com.intellij.ui.components.*
import com.intellij.util.execution.ParametersListUtil
import org.jdom.Element
import java.awt.GridBagConstraints
import java.awt.GridBagLayout
import java.nio.file.Files
import java.nio.file.Path
import javax.swing.*

class CranposeConfigurationType : ConfigurationTypeBase("Cranpose", "Cranpose", "Run, check or test a Cranpose Cargo target", cranposeIcon) {
    init { addFactory(Factory(this)) }
    class Factory(type: ConfigurationType) : ConfigurationFactory(type) {
        override fun getId(): String = "Cranpose Cargo"
        override fun createTemplateConfiguration(project: Project): RunConfiguration = CranposeRunConfiguration(project, this, "Cranpose")
    }
}

class CranposeRunConfiguration(project: Project, factory: ConfigurationFactory, name: String) :
    RunConfigurationBase<RunConfigurationOptions>(project, factory, name) {
    var manifest = ""
    var packageName = ""
    var targetName = ""
    var targetKind = "bin"
    var command = "run"
    var arguments = ""
    var features = ""
    var directory = ""
    var environmentText = ""
    var noDefaultFeatures = false

    fun useTarget(target: CargoTarget) {
        manifest = target.manifest
        packageName = target.packageName
        targetName = target.name
        targetKind = target.kind
        features = target.features.joinToString(" ")
        directory = Path.of(target.manifest).parent.toString()
    }

    fun cargoArguments(): List<String> = buildList {
        add(command)
        addAll(listOf("--manifest-path", manifest, "--package", packageName))
        if (command != "test") addAll(listOf("--$targetKind", targetName))
        if (features.isNotBlank()) addAll(listOf("--features", features.trim().split(Regex("[,\\s]+")).joinToString(",")))
        if (noDefaultFeatures) add("--no-default-features")
        val extra = ParametersListUtil.parse(arguments)
        if (extra.isNotEmpty()) { add("--"); addAll(extra) }
    }

    fun environment(): Map<String, String> = environmentText.lines().filter { it.isNotBlank() }.associate { line ->
        val split = line.indexOf('=')
        require(split > 0 && Regex("[A-Za-z_][A-Za-z_0-9]*").matches(line.substring(0, split))) { "Use one NAME=value environment variable per line." }
        line.substring(0, split) to line.substring(split + 1)
    }

    override fun checkConfiguration() {
        if (manifest.isBlank() || !Files.isRegularFile(Path.of(manifest))) throw RuntimeConfigurationError("Select a Cargo.toml file.")
        if (packageName.isBlank() || targetName.isBlank()) throw RuntimeConfigurationError("Select a Cranpose target.")
        if (command !in setOf("run", "check", "test")) throw RuntimeConfigurationError("Choose run, check or test.")
        if (targetKind !in setOf("bin", "example")) throw RuntimeConfigurationError("Choose a binary or example target.")
        if (directory.isNotBlank() && !Files.isDirectory(Path.of(directory))) throw RuntimeConfigurationError("The working directory does not exist.")
        if (command == "check" && arguments.isNotBlank()) throw RuntimeConfigurationError("Cargo check does not take application arguments.")
        try { environment() } catch (error: IllegalArgumentException) { throw RuntimeConfigurationError(error.message ?: "Invalid environment") }
    }

    override fun getConfigurationEditor(): SettingsEditor<out RunConfiguration> = CranposeRunSettings(project)
    override fun getState(executor: Executor, environment: ExecutionEnvironment): RunProfileState =
        object : CommandLineState(environment) {
            override fun startProcess(): ProcessHandler {
                checkConfiguration()
                val cwd = directory.ifBlank { Path.of(manifest).parent.toString() }
                return KillableColoredProcessHandler(CranposeProjectService.command(Path.of(cwd), cargoArguments())
                    .withEnvironment(environment()))
            }
        }

    override fun readExternal(element: Element) {
        super.readExternal(element)
        element.getChild("cranpose")?.let {
            manifest = it.getAttributeValue("manifest").orEmpty()
            packageName = it.getAttributeValue("package").orEmpty()
            targetName = it.getAttributeValue("target").orEmpty()
            targetKind = it.getAttributeValue("kind") ?: "bin"
            command = it.getAttributeValue("command") ?: "run"
            arguments = it.getChildText("arguments").orEmpty()
            features = it.getChildText("features").orEmpty()
            directory = it.getChildText("directory").orEmpty()
            environmentText = it.getChildText("environment").orEmpty()
            noDefaultFeatures = it.getAttributeValue("noDefaultFeatures") == "true"
        }
    }

    override fun writeExternal(element: Element) {
        super.writeExternal(element)
        element.addContent(Element("cranpose").apply {
            setAttribute("manifest", manifest)
            setAttribute("package", packageName)
            setAttribute("target", targetName)
            setAttribute("kind", targetKind)
            setAttribute("command", command)
            setAttribute("noDefaultFeatures", noDefaultFeatures.toString())
            addContent(Element("arguments").setText(arguments))
            addContent(Element("features").setText(features))
            addContent(Element("directory").setText(directory))
            addContent(Element("environment").setText(environmentText))
        })
    }

    companion object {
        fun create(project: Project, target: CargoTarget) {
            val type = ConfigurationTypeUtil.findConfigurationType(CranposeConfigurationType::class.java)
            val manager = RunManager.getInstance(project)
            val settings = manager.createConfiguration(target.name, type.configurationFactories.single())
            (settings.configuration as CranposeRunConfiguration).useTarget(target)
            if (com.intellij.execution.impl.RunDialog.editConfiguration(project, settings, "Save Cranpose Configuration")) {
                manager.addConfiguration(settings)
                manager.selectedConfiguration = settings
            }
        }
    }
}

private class CranposeRunSettings(project: Project) : SettingsEditor<CranposeRunConfiguration>() {
    private val targets = ComboBox(CranposeProjectService.get(project).snapshot.targets.toTypedArray())
    private val manifest = JBTextField()
    private val pkg = JBTextField()
    private val name = JBTextField()
    private val kind = ComboBox(arrayOf("bin", "example"))
    private val command = ComboBox(arrayOf("run", "check", "test"))
    private val args = JBTextField()
    private val features = JBTextField()
    private val cwd = JBTextField()
    private val environment = JBTextArea(4, 40)
    private val noDefaults = JCheckBox("Disable default features")
    private val root = JPanel(GridBagLayout())
    init {
        targets.renderer = object : DefaultListCellRenderer() {
            override fun getListCellRendererComponent(list: JList<*>?, value: Any?, index: Int, selected: Boolean, focus: Boolean) =
                super.getListCellRendererComponent(list, (value as? CargoTarget)?.label ?: value, index, selected, focus)
        }
        val fields = listOf("Workspace target" to targets, "Cargo manifest" to manifest, "Package" to pkg,
            "Target" to name, "Target kind" to kind, "Command" to command, "Arguments" to args,
            "Features" to features, "Working directory" to cwd, "Environment" to JBScrollPane(environment), "" to noDefaults)
        fields.forEachIndexed { index, (label, component) ->
            root.add(JBLabel(label), GridBagConstraints().apply {
                gridx = 0; gridy = index; anchor = GridBagConstraints.WEST; insets = java.awt.Insets(5, 5, 5, 12)
            })
            root.add(component, GridBagConstraints().apply {
                gridx = 1; gridy = index; weightx = 1.0; fill = GridBagConstraints.HORIZONTAL; insets = java.awt.Insets(5, 0, 5, 5)
            })
        }
        targets.addActionListener {
            (targets.selectedItem as? CargoTarget)?.let {
                manifest.text = it.manifest; pkg.text = it.packageName; name.text = it.name; kind.selectedItem = it.kind
                features.text = it.features.joinToString(" "); cwd.text = Path.of(it.manifest).parent.toString()
            }
        }
        environment.toolTipText = "One NAME=value entry per line. Keep shared configurations free of credentials."
    }
    override fun resetEditorFrom(s: CranposeRunConfiguration) {
        manifest.text = s.manifest; pkg.text = s.packageName; name.text = s.targetName; kind.selectedItem = s.targetKind
        command.selectedItem = s.command; args.text = s.arguments; features.text = s.features; cwd.text = s.directory
        environment.text = s.environmentText; noDefaults.isSelected = s.noDefaultFeatures
    }
    override fun applyEditorTo(s: CranposeRunConfiguration) {
        s.manifest = manifest.text; s.packageName = pkg.text; s.targetName = name.text; s.targetKind = kind.selectedItem.toString()
        s.command = command.selectedItem.toString(); s.arguments = args.text; s.features = features.text; s.directory = cwd.text
        s.environmentText = environment.text; s.noDefaultFeatures = noDefaults.isSelected
    }
    override fun createEditor(): JComponent = root
}
