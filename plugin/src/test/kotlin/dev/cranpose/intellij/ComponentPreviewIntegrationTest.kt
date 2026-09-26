package dev.cranpose.intellij

import org.junit.Assert.*
import org.junit.Test
import java.nio.file.Path
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit

class ComponentPreviewIntegrationTest {
    @Test fun compiledVariantsRenderIndependentlyAndExposeSourceOrigins() {
        val messages = LinkedBlockingQueue<AppEvent.Message>()
        val frames = LinkedBlockingQueue<AppEvent.Frame>()
        val listener = object : CranposeListener {
            override fun onFrame(frame: AppEvent.Frame) { frames.put(frame) }
            override fun onMessage(channel: String, payload: String) { messages.put(AppEvent.Message(channel, payload)) }
        }
        val binary = requireNotNull(System.getProperty("cranpose.test.sample"))
        var selected: PreviewDescriptor? = null
        CranposeSession.start(listOf(binary), null, listener, {}).use { session ->
            val registry = requireNotNull(messages.poll(15, TimeUnit.SECONDS))
            assertEquals("cranpose.previews.v1", registry.channel)
            val variants = PreviewDescriptor.parse(registry.payload)
            assertEquals(5, variants.size)
            assertEquals(5, variants.map { it.id }.distinct().size)
            assertTrue(variants.any { it.dark })
            selected = variants.single { it.name == "Status card" }
            session.host.resize(0, 480, 640, 1f, 60f)
            session.host.visibility(0, true)
            val frame = requireNotNull(frames.poll(20, TimeUnit.SECONDS))
            session.host.frameAck(frame.surface, frame.frameId)
        }
        frames.clear()
        messages.clear()
        val descriptor = requireNotNull(selected)
        CranposeSession.start(listOf(binary), null, listener, {}, environment = mapOf("CRANPOSE_PREVIEW" to descriptor.id)).use { session ->
            session.host.resize(0, descriptor.width, descriptor.height, 1f, 60f)
            session.host.visibility(0, true)
            val first = requireNotNull(frames.poll(20, TimeUnit.SECONDS))
            assertEquals(360, first.bufferWidth)
            assertEquals(180, first.bufferHeight)
            session.host.frameAck(first.surface, first.frameId)
            session.host.message("cranpose.inspector.v2.request", "73")
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(15)
            var snapshot: InspectionSnapshot? = null
            while (System.nanoTime() < deadline && snapshot == null) {
                val message = messages.poll(200, TimeUnit.MILLISECONDS)
                if (message?.channel == "cranpose.inspector.v2.snapshot") snapshot = InspectionSnapshot.parse(message.payload)
                while (true) { val frame = frames.poll() ?: break; session.host.frameAck(frame.surface, frame.frameId) }
            }
            val tree = requireNotNull(snapshot)
            assertEquals(73, tree.requestId)
            assertTrue(tree.nodes.any { it.text == "A good place to pause." })
            assertFalse(tree.nodes.any { it.text?.contains("Count:") == true })
            val label = tree.nodes.first { it.text == "A good place to pause." }
            assertTrue(label.sources.any { it.name == "StatusCard" && it.file.endsWith("gallery.rs") && it.line > 0 })
            assertTrue(tree.nodes.any { it.modifiers.any { m -> m.name.contains("padding", true) } })
            assertNotNull(tree.pick(label.x + 1, label.y + 1))
            System.getProperty("cranpose.test.output")?.let { path ->
                java.nio.file.Files.createDirectories(Path.of(path))
                java.nio.file.Files.writeString(Path.of(path, "component-tree.json"), com.google.gson.Gson().toJson(tree))
            }
        }
    }
}
