import importlib.util
import pathlib
import tempfile
import unittest
import io
import zipfile

spec = importlib.util.spec_from_file_location("release", pathlib.Path(__file__).parents[1] / "release.py")
release = importlib.util.module_from_spec(spec)
spec.loader.exec_module(release)


class ReleaseTest(unittest.TestCase):
    def test_requires_matching_release_version(self):
        with tempfile.TemporaryDirectory() as directory:
            properties = pathlib.Path(directory) / "gradle.properties"
            properties.write_text("pluginVersion = 0.1.0\n")
            self.assertEqual("0.1.0", release.validate_tag("v0.1.0", properties))
            with self.assertRaises(ValueError):
                release.validate_tag("v0.2.0", properties)

    def test_arranges_every_platform_and_rejects_missing_binaries(self):
        with tempfile.TemporaryDirectory() as directory:
            root = pathlib.Path(directory)
            with self.assertRaises(ValueError):
                release.arrange_native(root)
            for platform in release.PLATFORMS:
                folder = root / ("native-" + platform)
                folder.mkdir()
                (folder / ("cranpose-intellij-ui" + (".exe" if platform.startswith("windows") else ""))).write_bytes(b"binary")
            release.arrange_native(root)
            release.arrange_native(root)
            self.assertTrue(all((root / platform).is_dir() for platform in release.PLATFORMS))

    def test_archive_check_rejects_a_host_only_build(self):
        with tempfile.TemporaryDirectory() as directory:
            path = pathlib.Path(directory) / "plugin.zip"
            jar = io.BytesIO()
            with zipfile.ZipFile(jar, "w") as archive:
                archive.writestr("native/macos-aarch64/cranpose-intellij-ui", b"binary")
            with zipfile.ZipFile(path, "w") as archive:
                archive.writestr("plugin/lib/plugin.jar", jar.getvalue())
            with self.assertRaises(ValueError):
                release.check_zip(path)
