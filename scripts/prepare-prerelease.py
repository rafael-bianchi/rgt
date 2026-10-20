#!/usr/bin/env python3
"""Prepare a tag-only RC commit whose binary version matches its tag."""
import argparse
from pathlib import Path
import re


def prepare(root, tag):
    if not re.fullmatch(r"v\d+\.\d+\.\d+-rc\.\d+", tag):
        raise ValueError("expected vX.Y.Z-rc.N")
    version = tag[1:]
    manifest = root / "Cargo.toml"
    lock = root / "Cargo.lock"
    new_manifest, manifest_count = re.subn(
        r'(?ms)(^\[package\]\n(?:(?!^\[).)*?^version = ")[^"]+("\s*$)',
        lambda m: m[1] + version + m[2], manifest.read_text(), count=1,
    )
    new_lock, lock_count = re.subn(
        r'(\[\[package\]\]\nname = "rgt"\nversion = ")[^"]+("\n)',
        lambda m: m[1] + version + m[2], lock.read_text(),
    )
    if manifest_count != 1 or lock_count != 1:
        raise ValueError("expected exactly one root RGT version in manifest and lockfile")
    manifest.write_text(new_manifest)
    lock.write_text(new_lock)


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("tag")
    args = parser.parse_args()
    prepare(Path.cwd(), args.tag)
