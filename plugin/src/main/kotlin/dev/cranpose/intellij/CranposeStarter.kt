package dev.cranpose.intellij

import com.intellij.openapi.command.WriteCommandAction
import com.intellij.openapi.project.Project
import com.intellij.openapi.ui.Messages
import com.intellij.openapi.vfs.LocalFileSystem

object CranposeStarter {
    fun create(project: Project) {
        val base = project.basePath?.let { LocalFileSystem.getInstance().findFileByPath(it) } ?: return
        if (base.findChild("Cargo.toml") != null || base.findChild("src") != null) {
            Messages.showInfoMessage(project, "Create a starter in an empty project directory. Cargo.toml or src already exists.", "Cranpose")
            return
        }
        val name = Messages.showInputDialog(project, "Cargo package name", "Create Cranpose App", null, "cranpose-app", null) ?: return
        if (!Regex("[a-z][a-z0-9_-]*").matches(name)) {
            Messages.showErrorDialog(project, "Use lowercase letters, digits, hyphens, or underscores, starting with a letter.", "Cranpose")
            return
        }
        WriteCommandAction.runWriteCommandAction(project, "Create Cranpose app", null, Runnable {
            val manifest = base.createChildData(this, "Cargo.toml")
            manifest.setBinaryContent(manifest(name).toByteArray())
            base.createChildDirectory(this, "src").createChildData(this, "main.rs").setBinaryContent(source.toByteArray())
        })
        CranposeProjectService.get(project).refresh()
    }

    fun manifest(name: String): String = """
        [package]
        name = "$name"
        version = "0.1.0"
        edition = "2024"

        [dependencies]
        cranpose = { git = "https://github.com/samoylenkodmitry/Cranpose", rev = "e177b19985c303a13fcf40d61decc7253fdbe057", features = ["desktop", "preview"] }
    """.trimIndent() + "\n"

    val source = """
        use cranpose::{
            AppLauncher, Button, ButtonSpec, Column, ColumnSpec, Modifier, Text, TextStyle,
            composable, rememberMutableStateOf,
        };

        #[cranpose::preview(name = "Default", width = 480, height = 640)]
        #[composable]
        fn App() {
            let count = rememberMutableStateOf(|| 0);
            Column(Modifier::empty().fill_max_size().padding(24.0), ColumnSpec::default(), move || {
                Text(format!("Count: {}", count.get()), Modifier::empty(), TextStyle::default());
                Button(Modifier::empty().padding(12.0), ButtonSpec::default(),
                    move || count.set(count.get() + 1),
                    IncrementLabel);
            });
        }

        #[composable]
        fn IncrementLabel() {
            Text("Increment", Modifier::empty(), TextStyle::default());
        }

        fn main() {
            let app = AppLauncher::new().with_title("Cranpose app").with_size(480, 640);
            app.run(App);
        }
    """.trimIndent() + "\n"
}
