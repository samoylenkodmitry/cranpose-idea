package dev.cranpose.intellij

import org.junit.Assert.*
import org.junit.Test
import java.awt.image.BufferedImage
import java.awt.image.DataBufferInt
import java.nio.ByteBuffer
import java.nio.ByteOrder
import java.nio.file.Files
import java.nio.file.Path
import java.util.concurrent.LinkedBlockingQueue
import java.util.concurrent.TimeUnit
import javax.imageio.ImageIO

class EmbeddedUiEndToEndTest {
    private val events = LinkedBlockingQueue<Any>()
    private var canvas = BufferedImage(1, 1, BufferedImage.TYPE_INT_ARGB_PRE)
    private class Exit(val error: Throwable?)

    @Test fun dashboardRendersTargetsAndSendsActionsOverTheRealTransport() {
        val binary = Path.of(System.getProperty("cranpose.idea.ui.binary"))
        assertTrue("native UI is required for the end-to-end test", Files.isExecutable(binary))
        val listener = object : CranposeListener {
            override fun onFrame(frame: AppEvent.Frame) { events.put(frame) }
            override fun onMessage(channel: String, payload: String) { events.put(AppEvent.Message(channel, payload)) }
            override fun onExit(error: Throwable?) { events.put(Exit(error)) }
        }
        CranposeSession.start(listOf(binary.toString()), null, listener, { println(it) }).use { session ->
            session.host.resize(0, 480, 1100, 1f, 60f)
            session.host.visibility(0, true)
            session.host.message("ide.theme", """{"dark":true,"background":"#2b2d30","surface":"#393b40","text":"#dfe1e5","muted":"#868a91","accent":"#3574f0"}""")
            session.host.message("cranpose.project", """{"status":"Ready","root":"/demo","busy":false,"selected":"/demo/Cargo.toml::bin::demo","targets":[{"id":"/demo/Cargo.toml::bin::demo","packageName":"demo-app","name":"demo","kind":"bin"}]}""")
            session.host.message("cranpose.editor", """{"path":"/demo/src/main.rs","name":"main.rs","composables":[{"name":"App","offset":42,"line":3}]}""")
            await(session) { it is AppEvent.Frame && canvas.width == 480 && canvas.height == 1100 && accentButton() != null }
            pump(session, 500)
            save("dashboard-dark.png")
            val (x, y) = requireNotNull(accentButton())
            session.host.pointerMove(0, x.toFloat(), y.toFloat())
            session.host.pointerDown(0, x.toFloat(), y.toFloat())
            pump(session, 40)
            session.host.pointerUp(0, x.toFloat(), y.toFloat())
            val message = await(session) { it is AppEvent.Message && it.channel == "cranpose.action" } as AppEvent.Message
            assertEquals("refresh", FlatJson.decodeStrings(message.payload)?.get("action"))

            session.host.message("ide.theme", """{"dark":false,"background":"#ffffff","surface":"#f4f4f4","text":"#202124","muted":"#666666","accent":"#3574f0"}""")
            await(session) { it is AppEvent.Frame && canvas.getRGB(1, 1) == -1 }
            save("dashboard-light.png")
            session.host.message("cranpose.project", "not json")
            session.host.resize(0, 360, 900, 1f, 60f)
            await(session) { it is AppEvent.Frame && canvas.width == 360 && canvas.height == 900 }
            save("dashboard-narrow.png")
        }
    }

    @Test fun ordinaryDesktopAppSupportsLivePreviewInputAndInspection() {
        val binary = Path.of(requireNotNull(System.getProperty("cranpose.test.sample")))
        assertTrue("sample executable is required", Files.isExecutable(binary))
        val listener = object : CranposeListener {
            override fun onFrame(frame: AppEvent.Frame) { events.put(frame) }
            override fun onMessage(channel: String, payload: String) { events.put(AppEvent.Message(channel, payload)) }
            override fun onExit(error: Throwable?) { events.put(Exit(error)) }
        }
        CranposeSession.start(listOf(binary.toString()), null, listener, { println(it) }).use { session ->
            session.host.resize(0, 480, 640, 1f, 60f)
            session.host.visibility(0, true)
            await(session) { it is AppEvent.Frame && canvas.width == 480 && canvas.height == 640 }
            pump(session, 300)
            save("preview-counter.png")
            session.host.message("cranpose.inspector.v1.request", "")
            val before = await(session) { it is AppEvent.Message && it.channel == "cranpose.inspector.v1.snapshot" } as AppEvent.Message
            assertTrue(before.payload, before.payload.contains("Count: 0"))
            session.host.pointerMove(0, 50f, 65f)
            session.host.pointerDown(0, 50f, 65f)
            pump(session, 40)
            session.host.pointerUp(0, 50f, 65f)
            pump(session, 300)
            session.host.message("cranpose.inspector.v1.request", "")
            val after = await(session) { it is AppEvent.Message && it.channel == "cranpose.inspector.v1.snapshot" } as AppEvent.Message
            assertTrue(after.payload, after.payload.contains("Count: 1"))
            save("preview-counter-clicked.png")
            System.getProperty("cranpose.test.output")?.let {
                Files.writeString(Path.of(it).resolve("preview-inspection.txt"), after.payload)
            }
        }
    }

    private fun accentButton(): Pair<Int, Int>? {
        for (y in 80 until canvas.height - 10) for (x in 12 until canvas.width - 40) {
            if (canvas.getRGB(x, y) == ACCENT && (0..30).all { canvas.getRGB(x + it, y) == ACCENT } &&
                (0..8).all { canvas.getRGB(x, y + it) == ACCENT }) return x + 5 to y + 5
        }
        return null
    }

    private fun await(session: CranposeSession, accept: (Any) -> Boolean): Any {
        val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(20)
        while (System.nanoTime() < deadline) {
            val event = next(session, 500) ?: continue
            if (accept(event)) return event
        }
        save("failure.png")
        error("No matching UI event within 20 seconds")
    }

    private fun pump(session: CranposeSession, millis: Long) {
        val deadline = System.nanoTime() + TimeUnit.MILLISECONDS.toNanos(millis)
        while (System.nanoTime() < deadline) next(session, 20)
    }

    private fun next(session: CranposeSession, millis: Long): Any? {
        val event = events.poll(millis, TimeUnit.MILLISECONDS)
        if (event is Exit) error("UI exited: ${event.error}")
        if (event is AppEvent.Frame) {
            if (canvas.width != event.bufferWidth || canvas.height != event.bufferHeight)
                canvas = BufferedImage(event.bufferWidth, event.bufferHeight, BufferedImage.TYPE_INT_ARGB_PRE)
            val pixels = (canvas.raster.dataBuffer as DataBufferInt).data
            val source = ByteBuffer.wrap(event.pixels).order(ByteOrder.LITTLE_ENDIAN).asIntBuffer()
            for (y in 0 until event.height) source.get(pixels, (event.y + y) * canvas.width + event.x, event.width)
            session.host.frameAck(event.surface, event.frameId)
        }
        return event
    }

    private fun save(name: String) {
        val root = System.getProperty("cranpose.test.output")?.let(Path::of) ?: return
        Files.createDirectories(root)
        ImageIO.write(canvas, "png", root.resolve(name).toFile())
    }

    companion object { private const val ACCENT = -13273872 }
}
