package dev.cranpose.intellij

import org.junit.Assert.*
import org.junit.Test

class InspectionModelTest {
    private val root = InspectionNode("1:0", null, "Column", null, 0.0, 0.0, 200.0, 200.0, emptyList())
    private val child = InspectionNode("2:0", root.id, "Button", "Save", 12.0, 18.0, 80.0, 24.0, emptyList())

    @Test fun pickingPrefersTheFrontmostDeepestNodeAndExcludesOutsideBounds() {
        val snapshot = InspectionSnapshot(1, 4, false, listOf(root, child))
        assertEquals(child, snapshot.pick(20.0, 20.0))
        assertEquals(root, snapshot.pick(92.0, 20.0))
        assertNull(snapshot.pick(-1.0, 20.0))
        assertNull(snapshot.pick(200.0, 20.0))
    }

    @Test fun parsingRejectsOrphanedNodesAndInvalidGeometry() {
        val node = """{"id":"2:0","parent":"1:0","kind":"Text","text":"Hello","x":2,"y":3,"width":40,"height":20,"modifiers":[]}"""
        val json = """{"schema":2,"requestId":9,"captureMicros":3,"truncated":false,"nodes":[$node]}"""
        assertTrue(runCatching { InspectionSnapshot.parse(json) }.isFailure)
        val valid = json.replace("\"parent\":\"1:0\"", "\"parent\":null")
        assertEquals("Hello", InspectionSnapshot.parse(valid).nodes.single().text)
        assertTrue(runCatching { InspectionSnapshot.parse(valid.replace("\"width\":40", "\"width\":-1")) }.isFailure)
    }
}
