#!/usr/bin/env python3
"""Validate and arrange native release artifacts without executing them."""
import argparse
import pathlib
import re
import zipfile

PLATFORMS = (
    "macos-aarch64", "macos-x86_64", "linux-aarch64",
    "linux-x86_64", "windows-aarch64", "windows-x86_64",
)


def arrange_native(root):
    root = pathlib.Path(root)
    for platform in PLATFORMS:
        source = root / ("native-" + platform)
        destination = root / platform
        if source.exists():
            if destination.exists():
                raise ValueError(f"Both {source} and {destination} exist")
            source.rename(destination)
        executable = "cranpose-intellij-ui" + (".exe" if platform.startswith("windows") else "")
        binary = destination / executable
        if not binary.is_file() or binary.stat().st_size == 0:
            raise ValueError(f"Missing native binary: {binary}")
    return root


def validate_tag(tag, properties):
    version = re.search(r"^pluginVersion\s*=\s*(\S+)\s*$", pathlib.Path(properties).read_text(), re.M)
    if version is None or tag != "v" + version[1] or not re.fullmatch(r"v\d+\.\d+\.\d+(?:-[\w.]+)?", tag):
        raise ValueError("Release tag must match pluginVersion")
    return version[1]


def check_zip(path):
    with zipfile.ZipFile(path) as archive:
        jars = [name for name in archive.namelist() if name.endswith(".jar")]
        import io
        resources = set()
        for jar in jars:
            with zipfile.ZipFile(io.BytesIO(archive.read(jar))) as content:
                resources.update(content.namelist())
        for platform in PLATFORMS:
            executable = "cranpose-intellij-ui" + (".exe" if platform.startswith("windows") else "")
            if f"native/{platform}/{executable}" not in resources:
                raise ValueError(f"Plugin ZIP is missing {platform}")
    return path


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("command", choices=("arrange-native", "validate-tag", "check-zip"))
    parser.add_argument("value")
    args = parser.parse_args()
    if args.command == "arrange-native":
        print(arrange_native(args.value))
    elif args.command == "validate-tag":
        print(validate_tag(args.value, "plugin/gradle.properties"))
    else:
        print(check_zip(args.value))
