package dev.cranpose.intellij

import org.junit.Assert.*
import org.junit.Test
import java.awt.event.MouseEvent
import java.util.concurrent.CountDownLatch
import java.util.concurrent.TimeUnit
import java.util.concurrent.atomic.AtomicInteger
import javax.swing.SwingUtilities

class PanelLifecycleTest {
    @Test fun cancellingAPendingCompilerLaunchDoesNotWaitForItsConnectionTimeout() {
        org.junit.Assume.assumeFalse(System.getProperty("os.name").startsWith("Windows"))
        val started = CountDownLatch(1)
        val cancelled = java.util.concurrent.atomic.AtomicBoolean()
        val pid = java.util.concurrent.atomic.AtomicLong()
        val executor = java.util.concurrent.Executors.newSingleThreadExecutor()
        try {
            val future = executor.submit<Boolean> {
                try {
                    CranposeSession.start(listOf("sh", "-c", "echo \$\$; sleep 60"), null, object : CranposeListener { override fun onFrame(frame: AppEvent.Frame) {} }, { line -> line.toLongOrNull()?.let { pid.set(it); started.countDown() } }, connectTimeoutMillis = 1_200_000, cancelled = cancelled::get).close()
                    false
                } catch (error: java.io.IOException) { error.message.orEmpty().contains("cancelled") }
            }
            assertTrue(started.await(5, TimeUnit.SECONDS))
            cancelled.set(true)
            assertTrue(future.get(3, TimeUnit.SECONDS))
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(2)
            while (ProcessHandle.of(pid.get()).map { it.isAlive }.orElse(false) && System.nanoTime() < deadline) Thread.sleep(20)
            assertFalse(ProcessHandle.of(pid.get()).map { it.isAlive }.orElse(false))
        } finally { cancelled.set(true); executor.shutdownNow() }
    }

    @Test fun aClosedUiProcessCanBeRestartedByClickingThePanel() {
        val binary = requireNotNull(System.getProperty(UiBinary.OVERRIDE_PROPERTY))
        val connections = AtomicInteger()
        val restarted = CountDownLatch(1)
        lateinit var panel: CranposePanel
        SwingUtilities.invokeAndWait {
            panel = CranposePanel({ listOf(binary) })
            panel.setSize(480, 640)
            panel.onConnected = {
                if (connections.incrementAndGet() == 1) panel.link.host?.close()
                else restarted.countDown()
            }
            panel.start()
        }
        try {
            val deadline = System.nanoTime() + TimeUnit.SECONDS.toNanos(20)
            while ((connections.get() == 0 || panel.link.host != null) && System.nanoTime() < deadline) Thread.sleep(25)
            assertEquals(1, connections.get())
            assertNull("the disconnected host should be cleared", panel.link.host)
            SwingUtilities.invokeAndWait {
                panel.dispatchEvent(MouseEvent(panel, MouseEvent.MOUSE_PRESSED, System.currentTimeMillis(), 0, 20, 20, 1, false, MouseEvent.BUTTON1))
            }
            assertTrue("click should restart the disconnected UI", restarted.await(10, TimeUnit.SECONDS))
        } finally {
            SwingUtilities.invokeAndWait { panel.close() }
        }
    }
}
