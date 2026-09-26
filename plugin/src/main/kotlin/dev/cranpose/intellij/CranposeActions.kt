package dev.cranpose.intellij

import com.intellij.ide.BrowserUtil
import com.intellij.openapi.actionSystem.AnActionEvent
import com.intellij.openapi.project.DumbAwareAction

class RefreshCranposeAction : DumbAwareAction() {
    override fun actionPerformed(e: AnActionEvent) { e.project?.let { CranposeProjectService.get(it).refresh() } }
}
class CheckCranposeAction : DumbAwareAction() {
    override fun actionPerformed(e: AnActionEvent) { e.project?.let { CranposeProjectService.get(it).execute(CargoTask.CHECK) } }
}
class RunCranposeAction : DumbAwareAction() {
    override fun actionPerformed(e: AnActionEvent) { e.project?.let { CranposeProjectService.get(it).execute(CargoTask.RUN) } }
}
class PreviewCranposeAction : DumbAwareAction() {
    override fun actionPerformed(e: AnActionEvent) { e.project?.let { CranposeProjectService.get(it).execute(CargoTask.PREVIEW) } }
}
class CranposeDocsAction : DumbAwareAction() {
    override fun actionPerformed(e: AnActionEvent) { BrowserUtil.browse("https://docs.rs/cranpose/latest/cranpose/") }
}
