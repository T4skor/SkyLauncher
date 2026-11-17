import hashlib
import importlib.util
from pathlib import Path
import shutil
import subprocess
import sys
import tarfile
import tempfile
import unittest


ROOT = Path(__file__).parents[1]
builder = None
if sys.version_info >= (3, 11):
    import tomllib

    spec = importlib.util.spec_from_file_location("arch_builder", ROOT / "packaging/arch/build-local.py")
    builder = importlib.util.module_from_spec(spec)
    spec.loader.exec_module(builder)
    VERSION = tomllib.loads((ROOT / "Cargo.toml").read_text())["package"]["version"]


@unittest.skipIf(builder is None, "El empaquetado de Arch requiere Python 3.11+")
class PackagingTests(unittest.TestCase):
    def test_source_snapshot_is_reproducible_and_excludes_private_data(self):
        with tempfile.TemporaryDirectory() as directory:
            first, second = Path(directory) / "a.tar.gz", Path(directory) / "b.tar.gz"
            builder.snapshot(ROOT, first, VERSION, 0)
            builder.snapshot(ROOT, second, VERSION, 0)
            self.assertEqual(first.read_bytes(), second.read_bytes())
            with tarfile.open(first) as archive:
                names = archive.getnames()
                self.assertIn(f"skylauncher-{VERSION}/LICENSE", names)
                self.assertIn(f"skylauncher-{VERSION}/packaging/arch/skylauncher.desktop", names)
                forbidden = {".venv", "target", "dist", "__pycache__", ".git", "profiles.json", "rust-config.json"}
                for name in names:
                    self.assertTrue(name.startswith(f"skylauncher-{VERSION}/"))
                    self.assertFalse(Path(name).is_absolute())
                    self.assertNotIn("..", Path(name).parts)
                    self.assertFalse(forbidden.intersection(Path(name).parts), name)

    def test_source_snapshot_includes_windows_packaging_scripts(self):
        with tempfile.TemporaryDirectory() as directory:
            archive_path = Path(directory) / "sources.tar.gz"
            builder.snapshot(ROOT, archive_path, VERSION, 0)
            with tarfile.open(archive_path) as archive:
                names = set(archive.getnames())
                prefix = f"skylauncher-{VERSION}/"
                for relative in (
                    "packaging/windows/build-windows.sh",
                    "packaging/windows/installer.nsi",
                    "packaging/windows/LEEME-Windows.txt",
                    "docs/windows-build.md",
                ):
                    self.assertIn(prefix + relative, names)
                script = archive.getmember(prefix + "packaging/windows/build-windows.sh")
                self.assertEqual(script.mode, 0o755)

    def test_windows_installer_is_per_user_and_keeps_player_data(self):
        recipe = (ROOT / "packaging/windows/installer.nsi").read_text()
        self.assertIn("RequestExecutionLevel user", recipe)
        self.assertIn("$LOCALAPPDATA\\Programs\\SkyLauncher", recipe)
        self.assertNotIn(".mi-minecraft-launcher", recipe)

    @unittest.skipUnless(shutil.which("makepkg"), "makepkg solo está disponible en Arch")
    def test_prepared_recipe_has_real_checksums_and_matching_srcinfo(self):
        with tempfile.TemporaryDirectory() as directory:
            output = builder.prepare(ROOT, Path(directory) / "package", 0)
            source = output / f"skylauncher-{VERSION}.tar.gz"
            digest = hashlib.sha256(source.read_bytes()).hexdigest()
            recipe = (output / "PKGBUILD").read_text()
            info = (output / ".SRCINFO").read_text()
            self.assertIn(f"sha256sums=('{digest}')", recipe)
            self.assertIn(f"sha256sums = {digest}", info)
            self.assertIn("depends = python-minecraft-launcher-lib=8.0", info)
            self.assertNotIn("sha256sums = SKIP", info)
            with self.assertRaises(FileExistsError):
                builder.prepare(ROOT, output, 0)

    @unittest.skipUnless(shutil.which("bash"), "Requiere Bash")
    def test_recipe_shell_syntax(self):
        subprocess.run(["bash", "-n", str(ROOT / "packaging/arch/PKGBUILD")], check=True)

    @unittest.skipUnless(shutil.which("desktop-file-validate"), "Requiere desktop-file-utils")
    def test_desktop_entry(self):
        subprocess.run(["desktop-file-validate", str(ROOT / "packaging/arch/skylauncher.desktop")], check=True)


if __name__ == "__main__":
    unittest.main()
