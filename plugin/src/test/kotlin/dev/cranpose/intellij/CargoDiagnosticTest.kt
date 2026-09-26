package dev.cranpose.intellij

import org.junit.Assert.*
import org.junit.Test
import com.intellij.execution.process.CapturingProcessHandler
import java.nio.file.Files

class CargoDiagnosticTest {
    @Test fun realCargoCheckRetainsLocationsForSourceLinks() {
        val root = Files.createTempDirectory("cranpose diagnostic ")
        try {
            Files.writeString(root.resolve("Cargo.toml"), "[package]\nname = \"diagnostic-test\"\nversion = \"0.1.0\"\nedition = \"2024\"\n[workspace]\n")
            val source = Files.createDirectories(root.resolve("src")).resolve("main.rs")
            Files.writeString(source, "fn main() { let _: u32 = \"wrong type\"; }\n")
            val target = CargoTarget("diagnostic-test", "diagnostic-test", "bin", root.resolve("Cargo.toml").toString(), source.toString(), emptyList(), null)
            val command = CranposeProjectService.command(root, CargoCommand.arguments(CargoTask.CHECK, target))
                .withEnvironment("CARGO_TARGET_DIR", root.resolve("target").toString())
            val output = CapturingProcessHandler(command).runProcess(30_000)
            assertFalse(output.isTimeout)
            assertEquals(101, output.exitCode)
            val diagnostic = output.stdout.lineSequence().mapNotNull(CargoDiagnostic::parse).firstOrNull { it.file.endsWith("main.rs") }
            assertNotNull("Cargo must provide a source span, not just rendered stderr: ${output.stderr}", diagnostic)
            assertEquals(1, diagnostic?.line)
            assertTrue(diagnostic?.rendered?.contains("mismatched types") == true)
        } finally {
            root.toFile().deleteRecursively()
        }
    }

    @Test fun choosesThePrimarySpanAndRetainsHumanReadableOutput() {
        val diagnostic = CargoDiagnostic.parse("""{"reason":"compiler-message","message":{"message":"expected String","level":"error","rendered":"error: expected String\n","spans":[{"is_primary":false,"file_name":"other.rs","line_start":1,"column_start":1},{"is_primary":true,"file_name":"src/main.rs","line_start":12,"column_start":8}]}}""")
        assertEquals("src/main.rs", diagnostic?.file)
        assertEquals(12, diagnostic?.line)
        assertEquals(8, diagnostic?.column)
        assertEquals("error: expected String\n", diagnostic?.rendered)
        assertNull(CargoDiagnostic.parse("Checking package"))
        assertNull(CargoDiagnostic.parse("""{"reason":"build-finished","success":true}"""))
    }
}
