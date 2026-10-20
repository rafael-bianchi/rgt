"""Deterministic release gate regressions; no network or hosted runners."""
import importlib.util
from pathlib import Path
import tempfile
import unittest

SCRIPT = Path(__file__).resolve().parents[2] / "scripts/verify-release-assets.py"


class ReleaseAssetsTest(unittest.TestCase):
    def load_gate(self):
        spec = importlib.util.spec_from_file_location("release_gate", SCRIPT)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        return module

    def test_missing_archive_is_rejected(self):
        gate = self.load_gate()
        with tempfile.TemporaryDirectory() as directory:
            with self.assertRaisesRegex(ValueError, "missing release archive"):
                gate.validate_assets(Path(directory), "v0.8.1")

    def test_checksum_does_not_allow_missing_or_changed_assets(self):
        gate = self.load_gate()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.populate(root, gate)
            gate.validate_assets(root, "v0.8.1")
            archive = root / "rgt-v0.8.1-aarch64-apple-darwin.tar.gz"
            archive.write_bytes(b"tampered")
            with self.assertRaisesRegex(ValueError, "checksum mismatch"):
                gate.validate_assets(root, "v0.8.1")

    def test_missing_bundle_is_rejected(self):
        gate = self.load_gate()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.populate(root, gate)
            next(root.glob("*.sigstore.json")).unlink()
            with self.assertRaisesRegex(ValueError, "missing attestation bundle"):
                gate.validate_assets(root, "v0.8.1")

    def test_surplus_archive_is_rejected_even_with_matching_checksum(self):
        import hashlib
        gate = self.load_gate()
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.populate(root, gate)
            (root / "extra.tar.gz").write_bytes(b"unsigned")
            with (root / "checksums.txt").open("a") as manifest:
                manifest.write(f"{hashlib.sha256(b'unsigned').hexdigest()}  extra.tar.gz\n")
            with self.assertRaisesRegex(ValueError, "unexpected release files"):
                gate.validate_assets(root, "v0.8.1")

    def test_rc_version_mismatch_is_rejected(self):
        from unittest.mock import patch
        gate = self.load_gate()
        with patch.object(gate.subprocess, "check_output", return_value="rgt 0.8.1\n"):
            with self.assertRaisesRegex(ValueError, "does not match"):
                gate.verify_packaged_version(Path("rgt"), "v0.8.1-rc.1")
        with patch.object(gate.subprocess, "check_output", return_value="rgt 0.8.1-rc.1\n"):
            gate.verify_packaged_version(Path("rgt"), "v0.8.1-rc.1")

    def test_verifier_failure_stops_publication(self):
        import subprocess
        from unittest.mock import patch
        gate = self.load_gate()
        for failures in [
            [subprocess.CalledProcessError(1, "gh")],
            [None, subprocess.CalledProcessError(1, "rgt")],
        ]:
            with patch.object(gate.subprocess, "run", side_effect=failures):
                with self.assertRaises(subprocess.CalledProcessError):
                    gate.verify_attestations([Path("release/rgt.tar.gz")], Path("rgt"))

    def test_prerelease_source_versions_match_tag(self):
        script = SCRIPT.with_name("prepare-prerelease.py")
        spec = importlib.util.spec_from_file_location("prepare_rc", script)
        module = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(module)
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "Cargo.toml").write_text('[package]\nname = "rgt"\nversion = "0.8.0"\n')
            (root / "Cargo.lock").write_text('[[package]]\nname = "rgt"\nversion = "0.8.0"\n\n[[package]]\nname = "other"\nversion = "1.0.0"\n')
            module.prepare(root, "v0.8.1-rc.2")
            self.assertIn('version = "0.8.1-rc.2"', (root / "Cargo.toml").read_text())
            self.assertIn('version = "0.8.1-rc.2"', (root / "Cargo.lock").read_text())
            self.assertIn('name = "other"\nversion = "1.0.0"', (root / "Cargo.lock").read_text())

    def populate(self, root, gate):
        import hashlib
        archives = [f"rgt-v0.8.1-{target}.{suffix}" for target, suffix in gate.TARGETS]
        archives += ["rgt_0.8.1_amd64.deb", "rgt-0.8.1.x86_64.rpm"]
        for name in archives:
            (root / name).write_bytes(b"archive")
            (root / (name + ".sigstore.json")).write_text("{}")
        lines = [f"{hashlib.sha256(p.read_bytes()).hexdigest()}  {p.name}\n"
                 for p in root.iterdir()]
        (root / "checksums.txt").write_text("".join(lines))


if __name__ == "__main__":
    unittest.main()
