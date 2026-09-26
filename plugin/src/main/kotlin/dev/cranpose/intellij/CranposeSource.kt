package dev.cranpose.intellij

data class ComposableSymbol(val name: String, val offset: Int, val line: Int)

object CranposeSource {
    private val declaration = Regex("""#\s*\[\s*(?:[A-Za-z_][A-Za-z_0-9]*\s*::\s*)*composable\s*](?:\s|#\s*\[[^]]*])*?(?:pub(?:\s*\([^)]*\))?\s+)?(?:async\s+)?fn\s+(r#)?([A-Za-z_][A-Za-z_0-9]*)""")

    fun composables(source: String): List<ComposableSymbol> {
        val code = maskLiterals(source)
        return declaration.findAll(code).map { match ->
            val group = match.groups[2]!!
            val offset = match.groups[1]?.range?.first ?: group.range.first
            ComposableSymbol(group.value, offset, source.take(offset).count { it == '\n' } + 1)
        }.toList()
    }

    fun maskLiterals(source: String): String {
        val result = source.toCharArray()
        fun blank(from: Int, until: Int) {
            for (index in from until until.coerceAtMost(result.size)) if (result[index] != '\n') result[index] = ' '
        }
        var i = 0
        while (i < source.length) {
            val start = i
            when {
                source.startsWith("//", i) -> {
                    i = source.indexOf('\n', i).let { if (it < 0) source.length else it }
                    blank(start, i)
                }
                source.startsWith("/*", i) -> {
                    var depth = 1
                    i += 2
                    while (i < source.length && depth > 0) {
                        when {
                            source.startsWith("/*", i) -> { depth++; i += 2 }
                            source.startsWith("*/", i) -> { depth--; i += 2 }
                            else -> i++
                        }
                    }
                    blank(start, i)
                }
                source[i] == 'r' && (i == 0 || !source[i - 1].isLetterOrDigit()) -> {
                    var quote = i + 1
                    while (quote < source.length && source[quote] == '#') quote++
                    if (quote < source.length && source[quote] == '"') {
                        val ending = "\"" + "#".repeat(quote - i - 1)
                        val end = source.indexOf(ending, quote + 1)
                        i = if (end < 0) source.length else end + ending.length
                        blank(start, i)
                    } else i++
                }
                source[i] == '"' -> {
                    i++
                    while (i < source.length) {
                        if (source[i] == '\\') i = (i + 2).coerceAtMost(source.length)
                        else if (source[i++] == '"') break
                    }
                    blank(start, i)
                }
                source[i] == '\'' && i + 2 < source.length && (source[i + 2] == '\'' || source[i + 1] == '\\') -> {
                    i += 2
                    while (i < source.length && source[i] != '\'') i++
                    if (i < source.length) i++
                    blank(start, i)
                }
                else -> i++
            }
        }
        return String(result)
    }
}
