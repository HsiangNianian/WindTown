"""Verify distributable layouts, executable permissions, and required assets."""
from pathlib import Path
import plistlib
import tarfile
import tempfile
import unittest
import zipfile
from package import TARGETS, package


class PackagingTests(unittest.TestCase):
    def test_all_platforms_include_assets_and_notices(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            (root / "Cargo.toml").write_text('[package]\nversion = "1.2.3"\n')
            for name in ["README.md", "LICENSE.md", "CHANGELOG.md", "assets/people.png",
                         "assets/town.png", "assets/fonts/fusion-pixel.ttf", "assets/fonts/OFL.txt"]:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"test fixture")
            for target, platform in TARGETS.items():
                binary = root / "target" / target / "release" / ("wind-town.exe" if "windows" in target else "wind-town")
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"native executable fixture")
                archive = package(root, target)
                prefix = f"wind-town-1.2.3-{platform}/"
                if archive.suffix == ".zip":
                    with zipfile.ZipFile(archive) as file:
                        names = file.namelist()
                        self.assertIn(prefix + "wind-town.exe", names)
                else:
                    with tarfile.open(archive) as file:
                        names = file.getnames()
                        executable = prefix + ("Wind Town.app/Contents/MacOS/" if "macos" in platform else "") + "wind-town"
                        self.assertEqual(file.getmember(executable).mode & 0o111, 0o111)
                        if "macos" in platform:
                            info = plistlib.load(file.extractfile(prefix + "Wind Town.app/Contents/Info.plist"))
                            self.assertEqual(info["CFBundleShortVersionString"], "1.2.3")
                self.assertIn(prefix + "LICENSE.md", names)
                self.assertTrue(any(name.endswith("assets/fonts/fusion-pixel.ttf") for name in names))
                self.assertTrue(any(name.endswith("assets/fonts/OFL.txt") for name in names))

    def test_missing_binary_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "Cargo.toml").write_text('[package]\nversion = "1.2.3"\n')
            with self.assertRaises(ValueError):
                package(tmp, "x86_64-unknown-linux-gnu")
