"""Bundle a native release with its assets and notices; no external packaging tools."""
import argparse
from pathlib import Path
import plistlib
import shutil
import tarfile
import tomllib
import zipfile

TARGETS = {
    "x86_64-unknown-linux-gnu": "linux-x64",
    "x86_64-pc-windows-msvc": "windows-x64",
    "aarch64-apple-darwin": "macos-arm64",
    "x86_64-apple-darwin": "macos-x64",
}


def package(root, target):
    root = Path(root)
    platform = TARGETS[target]
    version = tomllib.loads((root / "Cargo.toml").read_text())["package"]["version"]
    name = f"wind-town-{version}-{platform}"
    windows, mac = "windows" in platform, "macos" in platform
    binary = root / "target" / target / "release" / ("wind-town.exe" if windows else "wind-town")
    if not binary.is_file() or not binary.stat().st_size:
        raise ValueError(f"Build {target} before packaging")
    stage = root / "dist" / name
    if stage.exists():
        shutil.rmtree(stage)
    stage.mkdir(parents=True)
    executable_dir = stage / "Wind Town.app/Contents/MacOS" if mac else stage
    executable_dir.mkdir(parents=True, exist_ok=True)
    shutil.copy2(binary, executable_dir / binary.name)
    (executable_dir / binary.name).chmod(0o755)
    shutil.copytree(root / "assets", executable_dir / "assets")
    for file in ["README.md", "LICENSE.md", "CHANGELOG.md"]:
        shutil.copy2(root / file, stage / file)
    if (root / "docs").is_dir():
        shutil.copytree(root / "docs", stage / "docs")
    for required in ["assets/people.png", "assets/town.png", "assets/fonts/fusion-pixel.ttf"]:
        if not (executable_dir / required).is_file():
            raise ValueError(f"Missing bundled asset: {required}")
    if mac:
        with (stage / "Wind Town.app/Contents/Info.plist").open("wb") as file:
            plistlib.dump({"CFBundleExecutable": "wind-town", "CFBundleName": "Wind Town",
                          "CFBundleIdentifier": "io.github.hsiangnianian.wind-town",
                          "CFBundlePackageType": "APPL", "CFBundleVersion": version,
                          "CFBundleShortVersionString": version, "NSHighResolutionCapable": True}, file)
    archive = stage.with_name(name + (".zip" if windows else ".tar.gz"))
    if windows:
        with zipfile.ZipFile(archive, "w", zipfile.ZIP_DEFLATED) as file:
            for path in sorted(stage.rglob("*")):
                if path.is_file():
                    file.write(path, path.relative_to(stage.parent))
    else:
        with tarfile.open(archive, "w:gz") as file:
            file.add(stage, arcname=name)
    shutil.rmtree(stage)
    return archive


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--target", required=True, choices=TARGETS)
    print(package(Path(__file__).resolve().parent.parent, parser.parse_args().target))
