package dev.cranpose.intellij

import com.google.gson.JsonParser

data class CargoDiagnostic(val message: String, val level: String, val rendered: String,
                           val file: String, val line: Int, val column: Int) {
    companion object {
        fun parse(line: String): CargoDiagnostic? = runCatching {
            val value = JsonParser.parseString(line).asJsonObject
            if (value.string("reason") != "compiler-message") return null
            val message = value.getAsJsonObject("message")
            val spans = message.getAsJsonArray("spans").map { it.asJsonObject }
            val span = spans.firstOrNull { it.get("is_primary")?.asBoolean == true } ?: spans.firstOrNull()
            CargoDiagnostic(message.string("message"), message.string("level"), message.string("rendered"),
                span?.string("file_name").orEmpty(), span?.get("line_start")?.asInt ?: 1, span?.get("column_start")?.asInt ?: 1)
        }.getOrNull()
    }
}
