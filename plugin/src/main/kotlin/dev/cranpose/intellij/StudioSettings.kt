package dev.cranpose.intellij

import com.intellij.openapi.components.PersistentStateComponent
import com.intellij.openapi.components.Service
import com.intellij.openapi.components.State
import com.intellij.openapi.components.Storage
import com.intellij.openapi.components.StoragePathMacros
import com.intellij.openapi.components.service
import com.intellij.openapi.project.Project

@Service(Service.Level.PROJECT)
@State(name = "CranposeStudio", storages = [Storage(StoragePathMacros.WORKSPACE_FILE)])
class StudioSettings : PersistentStateComponent<StudioSettings.Data> {
    class Data {
        var target = ""
        var preview = ""
        var width = 480
        var height = 640
        var zoom = 1.0
        var dark = false
        var autoBuild = true
        var inspect = false
        var fit = true
    }
    private var data = Data()
    override fun getState(): Data = data
    override fun loadState(state: Data) {
        state.width = state.width.coerceIn(120, 4096)
        state.height = state.height.coerceIn(120, 4096)
        state.zoom = state.zoom.takeIf { it.isFinite() }?.coerceIn(0.25, 2.0) ?: 1.0
        data = state
    }
    companion object { fun get(project: Project): Data = project.service<StudioSettings>().state }
}
