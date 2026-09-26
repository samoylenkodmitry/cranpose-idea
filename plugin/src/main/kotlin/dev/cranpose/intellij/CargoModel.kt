package dev.cranpose.intellij

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import java.nio.file.Path

data class CargoTarget(
    val packageName: String,
    val name: String,
    val kind: String,
    val manifest: String,
    val source: String,
    val features: List<String>,
    val cranposeDependency: String?,
) {
    val id: String get() = "$manifest::$kind::$name"
    val label: String get() = "$packageName / $name ($kind)"
}

data class CargoWorkspace(val root: String, val targets: List<CargoTarget>, val packages: Int) {
    companion object {
        fun parse(json: String): CargoWorkspace {
            val value = JsonParser.parseString(json).asJsonObject
            val members = value.getAsJsonArray("workspace_members").map { it.asString }.toSet()
            val packages = value.getAsJsonArray("packages").map { it.asJsonObject }
                .filter { it.string("id") in members }
            val targets = packages.flatMap { pkg ->
                val dependencies = pkg.getAsJsonArray("dependencies").map { it.asJsonObject }
                val dependency = dependencies.firstOrNull { it.string("name") == "cranpose" }
                val isCranpose = dependency != null || pkg.string("name") == "cranpose"
                if (!isCranpose) emptyList() else pkg.getAsJsonArray("targets").mapNotNull { item ->
                    val target = item.asJsonObject
                    val kinds = target.getAsJsonArray("kind").map { it.asString }
                    val kind = when {
                        "bin" in kinds -> "bin"
                        "example" in kinds && target.getAsJsonArray("crate_types").any { it.asString == "bin" } -> "example"
                        else -> return@mapNotNull null
                    }
                    CargoTarget(pkg.string("name"), target.string("name"), kind, pkg.string("manifest_path"),
                        target.string("src_path"), target.getAsJsonArray("required-features")?.map { it.asString }.orEmpty(),
                        dependency?.get("rename")?.takeUnless { it.isJsonNull }?.asString ?: dependency?.string("name"))
                }
            }
            return CargoWorkspace(value.string("workspace_root"), targets, packages.size)
        }
    }
}

enum class CargoTask(val verb: String) { CHECK("check"), RUN("run"), TEST("test"), PREVIEW("build") }

object CargoCommand {
    fun arguments(task: CargoTask, target: CargoTarget): List<String> = buildList {
        add(task.verb)
        addAll(listOf("--manifest-path", target.manifest, "--package", target.packageName))
        if (task != CargoTask.TEST) addAll(listOf("--${target.kind}", target.name))
        val features = target.features + when {
            task != CargoTask.PREVIEW -> emptyList()
            target.cranposeDependency != null -> listOf("${target.cranposeDependency}/embed")
            target.packageName == "cranpose" -> listOf("embed")
            else -> emptyList()
        }
        if (features.isNotEmpty()) addAll(listOf("--features", features.distinct().joinToString(",")))
        if (task == CargoTask.PREVIEW || task == CargoTask.CHECK) add("--message-format=json")
    }

    fun executable(line: String, target: CargoTarget): Path? = runCatching {
        val value = JsonParser.parseString(line).asJsonObject
        if (value.string("reason") != "compiler-artifact") return null
        val built = value.getAsJsonObject("target")
        if (built.string("name") != target.name ||
            built.string("src_path") != target.source ||
            built.getAsJsonArray("kind").none { it.asString == target.kind }) return null
        value.get("executable")?.takeUnless { it.isJsonNull }?.asString?.let(Path::of)
    }.getOrNull()
}

internal fun JsonObject.string(name: String): String = get(name)?.takeUnless { it.isJsonNull }?.asString.orEmpty()
