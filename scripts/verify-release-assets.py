#!/usr/bin/env python3
"""Release-only orchestration. Cryptographic verification stays in RGT and gh."""
import argparse
import hashlib
from pathlib import Path
import re
import subprocess

TARGETS = [
    ("aarch64-apple-darwin", "tar.gz"),
    ("x86_64-apple-darwin", "tar.gz"),
    ("x86_64-unknown-linux-musl", "tar.gz"),
    ("aarch64-unknown-linux-gnu", "tar.gz"),
    ("x86_64-pc-windows-msvc", "zip"),
]


def validate_assets(root, tag):
    if not re.fullmatch(r"v\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?", tag):
        raise ValueError("expected a v-prefixed semantic release tag")
    files = {p.name: p for p in root.iterdir() if p.is_file()}
    archives = [f"rgt-{tag}-{target}.{suffix}" for target, suffix in TARGETS]
    for name in archives:
        if name not in files:
            raise ValueError(f"missing release archive: {name}")
    for suffix in (".deb", ".rpm"):
        packages = [name for name in files if name.endswith(suffix)]
        if len(packages) != 1:
            raise ValueError(f"expected exactly one {suffix} package")
        archives += packages
    for name in archives:
        if name + ".sigstore.json" not in files:
            raise ValueError(f"missing attestation bundle: {name}")
    allowed = set(archives) | {name + ".sigstore.json" for name in archives} | {"checksums.txt"}
    if set(files) != allowed:
        raise ValueError("unexpected release files: " + ", ".join(sorted(set(files) - allowed)))

    entries = {}
    for line in (root / "checksums.txt").read_text().splitlines():
        match = re.fullmatch(r"([0-9a-f]{64})  ([^/\\]+)", line)
        if not match or match[2] in entries or match[2] == "checksums.txt":
            raise ValueError("malformed, duplicate, or self-hashing checksum entry")
        entries[match[2]] = match[1]
    if set(entries) != set(files) - {"checksums.txt"}:
        raise ValueError("checksum manifest does not cover exactly all release files")
    for name, expected in entries.items():
        with files[name].open("rb") as file:
            actual = hashlib.file_digest(file, "sha256").hexdigest()
        if actual != expected:
            raise ValueError(f"checksum mismatch: {name}")
    return [files[name] for name in archives]


def verify_packaged_version(rgt, tag):
    version = subprocess.check_output([str(rgt.resolve()), "--version"], text=True).strip()
    if version != f"rgt {tag[1:]}":
        raise ValueError(f"packaged binary version {version!r} does not match {tag}")


def verify_attestations(archives, rgt):
    for archive in archives:
        bundle = Path(str(archive) + ".sigstore.json")
        subprocess.run([
            "gh", "attestation", "verify", str(archive), "--bundle", str(bundle),
            "--repo", "rafael-bianchi/rgt",
            "--signer-workflow", "rafael-bianchi/rgt/.github/workflows/release.yml",
        ], check=True)
        subprocess.run([
            str(rgt.resolve()), "update", "--verify-archive", str(archive),
            "--bundle", str(bundle),
        ], check=True)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--tag", required=True)
    parser.add_argument("--rgt", required=True, type=Path)
    args = parser.parse_args()
    archives = validate_assets(args.directory, args.tag)
    verify_packaged_version(args.rgt, args.tag)
    # Verify all archive digests with both independent gh and the same verifier
    # shipped in the release binary. The binary runs in offline, no-install mode.
    verify_attestations(archives, args.rgt)
    print("Release checksums and attestations verified; publication may proceed.")


if __name__ == "__main__":
    main()
