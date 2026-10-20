"""Version gate distinguishes pending source release from installed formula."""
from pathlib import Path
import shutil
import subprocess
import tempfile
import unittest


class VersionStateTest(unittest.TestCase):
    def check(self, formula_version):
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "scripts").mkdir()
            (root / "Formula").mkdir()
            source = Path(__file__).resolve().parents[2] / "scripts/check-version-state.sh"
            shutil.copy(source, root / "scripts/check-version-state.sh")
            (root / "Cargo.toml").write_text('version = "0.8.1"\n')
            (root / ".release-please-manifest.json").write_text('{".":"0.8.1"}')
            (root / "CHANGELOG.md").write_text('## [0.8.1]\n\n## [0.8.0]\n')
            (root / "Formula/rgt.rb").write_text(f'  version "{formula_version}"\n')
            return subprocess.run(["sh", str(root / "scripts/check-version-state.sh")],
                                  capture_output=True, text=True)

    def test_previous_release_formula_is_allowed_while_assets_are_pending(self):
        output = self.check("0.8.0")
        self.assertEqual(output.returncode, 0, output.stderr)
        self.assertIn("pending", output.stdout)

    def test_older_or_ahead_formula_is_rejected(self):
        for version in ["0.7.0", "0.9.0"]:
            self.assertNotEqual(self.check(version).returncode, 0)


if __name__ == "__main__":
    unittest.main()
