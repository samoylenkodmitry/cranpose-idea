package dev.cranpose.intellij

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import com.google.gson.JsonParser
import com.intellij.execution.process.CapturingProcessHandler
import com.intellij.openapi.application.PathManager
import java.io.IOException
import java.nio.charset.StandardCharsets
import java.nio.file.Path

data class StabilityBadge(
    val start: Int, val end: Int, val label: String, val tone: String, val detail: String,
    val rule: String? = null, val suppressed: Boolean = false,
)
data class StabilityOverlay(val path: String, val source: String)
data class StabilityAnalysis(val files: Map<String, List<StabilityBadge>>)

/** Transport to the bundled Rust analyzer. All stability rules and badge wording live in Rust. */
object NativeStability {
    fun analyze(root: String, overlays: List<StabilityOverlay>, only: List<String>,
                onProcess: (Process) -> Unit = {}, timeoutMillis: Int = 30_000): StabilityAnalysis {
        val request = JsonObject().apply {
            addProperty("root", root)
            add("overlays", JsonArray().apply {
                overlays.forEach { overlay -> add(JsonObject().apply {
                    addProperty("path", overlay.path); addProperty("source", overlay.source)
                }) }
            })
            add("only", JsonArray().apply { only.forEach { add(it) } })
        }
        val binary = UiBinary.override() ?: UiBinary.extractBundled(
            Path.of(PathManager.getSystemPath(), "cranpose", "native"))
        val process = ProcessBuilder(binary.toString(), "--stability").start()
        onProcess(process)
        try {
            val handler = CapturingProcessHandler(process, StandardCharsets.UTF_8, binary.toString())
            process.outputStream.bufferedWriter(StandardCharsets.UTF_8).use { it.write(request.toString()) }
            val output = handler.runProcess(timeoutMillis, true)
            if (output.isTimeout) throw IOException("Stability analysis timed out")
            if (output.exitCode != 0) throw IOException(output.stderr.take(2000).ifBlank { "Stability analyzer exited with ${output.exitCode}" })
            return parse(output.stdout)
        } finally {
            if (process.isAlive) process.destroyForcibly()
        }
    }

    fun parse(source: String): StabilityAnalysis {
        val result = JsonParser.parseString(source).asJsonObject
        if (result.get("schemaVersion")?.asInt != 1) throw IOException("Unsupported stability report version")
        result.get("error")?.takeUnless { it.isJsonNull }?.let { throw IOException(it.asString) }
        val files = linkedMapOf<String, MutableList<StabilityBadge>>()
        for (item in result.getAsJsonArray("files")) {
            val file = item.asJsonObject
            val badges = file.getAsJsonArray("badges").map { value ->
                val badge = value.asJsonObject
                StabilityBadge(badge.get("start").asInt, badge.get("end").asInt,
                    badge.get("label").asString, badge.get("tone").asString, badge.get("detail").asString,
                    badge.get("rule")?.takeUnless { it.isJsonNull }?.asString,
                    badge.get("suppressed")?.asBoolean ?: false)
            }
            files[file.get("path").asString] = badges.toMutableList()
        }
        // Syntax failures remain visible while a file is incomplete.
        for (item in result.getAsJsonArray("diagnostics")) {
            val diagnostic = item.asJsonObject
            if (diagnostic.get("rule").asString != "CP000") continue
            val location = diagnostic.getAsJsonObject("location")
            val offset = location.get("utf16Start").asInt
            files.getOrPut(diagnostic.get("path").asString) { mutableListOf() }.add(
                StabilityBadge(offset, offset, "syntax error", "danger",
                    diagnostic.get("message").asString, "CP000"))
        }
        return StabilityAnalysis(files)
    }
}
