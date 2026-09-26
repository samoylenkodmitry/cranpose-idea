package dev.cranpose.intellij

import org.junit.Assert.*
import org.junit.Test

class CargoModelTest {
    private val metadata = """
        {"workspace_root":"/work/a b","workspace_members":["app-id","other-id"],"packages":[
          {"id":"app-id","name":"app","manifest_path":"/work/a b/Cargo.toml",
           "dependencies":[{"name":"cranpose","rename":"ui"}],"targets":[
             {"name":"app","kind":["bin"],"crate_types":["bin"],"src_path":"/work/a b/src/main.rs","required-features":["desktop"]},
             {"name":"card","kind":["example"],"crate_types":["bin"],"src_path":"/work/a b/examples/card.rs"},
             {"name":"lib","kind":["lib"],"crate_types":["lib"],"src_path":"/work/a b/src/lib.rs"}]},
          {"id":"other-id","name":"other","manifest_path":"/work/other/Cargo.toml","dependencies":[],"targets":[]},
          {"id":"external","name":"dep","manifest_path":"/registry/Cargo.toml","dependencies":[{"name":"cranpose"}],"targets":[]}
        ]}
    """.trimIndent()

    @Test fun readsOnlyWorkspaceCranposeRunnableTargets() {
        val workspace = CargoWorkspace.parse(metadata)
        assertEquals(2, workspace.packages)
        assertEquals(listOf("app", "card"), workspace.targets.map { it.name })
        assertEquals("ui", workspace.targets.first().cranposeDependency)
    }

    @Test fun previewPreservesRequiredFeaturesAndDependencyAlias() {
        val target = CargoWorkspace.parse(metadata).targets.first()
        assertEquals(listOf("build", "--manifest-path", "/work/a b/Cargo.toml", "--package", "app", "--bin", "app",
            "--features", "desktop,ui/preview", "--message-format=json"), CargoCommand.arguments(CargoTask.PREVIEW, target))
        assertFalse(CargoCommand.arguments(CargoTask.TEST, target).contains("--bin"))
        assertFalse(CargoCommand.arguments(CargoTask.RUN, target).joinToString().contains("/embed"))
    }

    @Test fun frameworkExamplesEnableTheirOwnEmbedFeature() {
        val target = CargoWorkspace.parse(metadata).targets.first().copy(packageName = "cranpose", cranposeDependency = null)
        val arguments = CargoCommand.arguments(CargoTask.PREVIEW, target)
        assertEquals("desktop,preview", arguments[arguments.indexOf("--features") + 1])
    }

    @Test fun readsExecutableOnlyFromSelectedTargetArtifact() {
        val target = CargoWorkspace.parse(metadata).targets.first()
        val artifact = """{"reason":"compiler-artifact","target":{"name":"app","kind":["bin"],"src_path":"/work/a b/src/main.rs"},"executable":"/custom/target/app"}"""
        assertEquals("/custom/target/app", CargoCommand.executable(artifact, target).toString())
        assertNull(CargoCommand.executable("compiler warning", target))
        assertNull(CargoCommand.executable(artifact.replace("\"bin\"", "\"example\""), target))
        assertNull(CargoCommand.executable(artifact.replace("src/main.rs", "other/main.rs"), target))
    }
}
