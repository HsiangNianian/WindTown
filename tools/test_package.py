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
            for name in ["README.md", "README.zh-CN.md", "LICENSE.md", "CHANGELOG.md", "assets/people.png",
                         "assets/town.png", "assets/fonts/fusion-pixel.ttf", "assets/fonts/OFL.txt",
                         "assets/maps/town.tmj", "assets/maps/tackle-shop.tmj",
                         "assets/maps/harbor.tsj", "assets/maps/harbor.png",
                         "assets/fishing/items.png", "assets/fishing/frame.png",
                         "assets/fishing/slot.png", "assets/fishing/water.png",
                         "assets/ui/editor-icons.png"]:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_bytes(b"test fixture")
            for target, platform in TARGETS.items():
                binary = root / "target" / target / "release" / ("yapshire.exe" if "windows" in target else "yapshire")
                binary.parent.mkdir(parents=True)
                binary.write_bytes(b"native executable fixture")
                archive = package(root, target)
                prefix = f"yapshire-1.2.3-{platform}/"
                if archive.suffix == ".zip":
                    with zipfile.ZipFile(archive) as file:
                        names = file.namelist()
                        self.assertIn(prefix + "yapshire.exe", names)
                else:
                    with tarfile.open(archive) as file:
                        names = file.getnames()
                        executable = prefix + ("Yapshire.app/Contents/MacOS/" if "macos" in platform else "") + "yapshire"
                        self.assertEqual(file.getmember(executable).mode & 0o111, 0o111)
                        if "macos" in platform:
                            info = plistlib.load(file.extractfile(prefix + "Yapshire.app/Contents/Info.plist"))
                            self.assertEqual(info["CFBundleShortVersionString"], "1.2.3")
                            self.assertEqual(info["CFBundleExecutable"], "yapshire")
                            self.assertEqual(info["CFBundleName"], "Yapshire")
                            self.assertEqual(info["CFBundleIdentifier"], "io.github.hsiangnianian.yapshire")
                self.assertIn(prefix + "LICENSE.md", names)
                self.assertIn(prefix + "README.zh-CN.md", names)
                self.assertTrue(any(name.endswith("assets/fonts/fusion-pixel.ttf") for name in names))
                self.assertTrue(any(name.endswith("assets/fonts/OFL.txt") for name in names))
                self.assertTrue(any(name.endswith("assets/ui/editor-icons.png") for name in names))
                for asset in ["town.tmj", "tackle-shop.tmj", "harbor.tsj", "harbor.png"]:
                    self.assertTrue(any(name.endswith("assets/maps/" + asset) for name in names))
                for asset in ["items.png", "frame.png", "slot.png", "water.png"]:
                    self.assertTrue(any(name.endswith("assets/fishing/" + asset) for name in names))

    def test_missing_binary_is_rejected(self):
        with tempfile.TemporaryDirectory() as tmp:
            (Path(tmp) / "Cargo.toml").write_text('[package]\nversion = "1.2.3"\n')
            with self.assertRaises(ValueError):
                package(tmp, "x86_64-unknown-linux-gnu")
