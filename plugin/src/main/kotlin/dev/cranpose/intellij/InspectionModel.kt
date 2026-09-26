package dev.cranpose.intellij

import com.google.gson.JsonParser

data class InspectionProperty(val name: String, val value: String)
data class InspectionSource(val name: String, val file: String, val line: Int, val manifestDir: String)
data class InspectionModifier(val name: String, val properties: List<InspectionProperty>)
data class InspectionNode(
    val id: String, val parent: String?, val kind: String, val text: String?,
    val x: Double, val y: Double, val width: Double, val height: Double,
    val modifiers: List<InspectionModifier>,
    val sources: List<InspectionSource> = emptyList(),
) {
    val label: String get() = text?.takeIf { it.isNotBlank() }?.let { "$kind · ${it.take(64)}" } ?: kind
    fun contains(px: Double, py: Double): Boolean =
        width > 0 && height > 0 && px >= x && py >= y && px < x + width && py < y + height
    override fun toString(): String = label
}

data class InspectionSnapshot(val requestId: Long, val captureMicros: Long, val truncated: Boolean, val nodes: List<InspectionNode>) {
    fun pick(x: Double, y: Double): InspectionNode? = nodes.asReversed().firstOrNull { it.contains(x, y) }
    companion object {
        fun parse(payload: String): InspectionSnapshot {
            val root = JsonParser.parseString(payload).asJsonObject
            require(root.get("schema").asInt == 2) { "Unsupported layout inspection schema" }
            val seen = HashSet<String>()
            val nodes = root.getAsJsonArray("nodes").map { item ->
                val n = item.asJsonObject
                val id = n.string("id")
                val parent = n.get("parent")?.takeUnless { it.isJsonNull }?.asString
                require(id.isNotEmpty() && seen.add(id)) { "Duplicate or missing layout identity" }
                require(parent == null || parent in seen && parent != id) { "Layout parent must precede its child" }
                val bounds = listOf("x", "y", "width", "height").map { n.get(it).asDouble }
                require(bounds.all { it.isFinite() } && bounds[2] >= 0 && bounds[3] >= 0) { "Invalid layout bounds" }
                InspectionNode(id, parent, n.string("kind"), n.get("text")?.takeUnless { it.isJsonNull }?.asString,
                    bounds[0], bounds[1], bounds[2], bounds[3],
                    n.getAsJsonArray("modifiers").map { modifier ->
                        val m = modifier.asJsonObject
                        InspectionModifier(m.string("name"), m.getAsJsonArray("properties").map { property ->
                            val p = property.asJsonObject
                            InspectionProperty(p.string("name"), p.string("value"))
                        })
                    }, n.getAsJsonArray("sources")?.map { source ->
                        val s = source.asJsonObject
                        InspectionSource(s.string("name"), s.string("file"), s.get("line").asInt, s.string("manifestDir"))
                    }.orEmpty())
            }
            return InspectionSnapshot(root.get("requestId").asLong, root.get("captureMicros").asLong,
                root.get("truncated").asBoolean, nodes)
        }
    }
}

data class PreviewDescriptor(
    val id: String, val name: String, val group: String, val function: String, val file: String,
    val line: Int, val width: Int, val height: Int, val dark: Boolean,
) {
    override fun toString(): String = if (group.isBlank()) name else "$group / $name"
    companion object {
        fun parse(payload: String): List<PreviewDescriptor> = JsonParser.parseString(payload).asJsonArray.map { item ->
            val p = item.asJsonObject
            PreviewDescriptor(p.string("id"), p.string("name"), p.string("group"), p.string("function"), p.string("file"),
                p.get("line").asInt, p.get("width").asInt.coerceIn(1, 8192), p.get("height").asInt.coerceIn(1, 8192), p.get("dark").asBoolean)
        }
    }
}
