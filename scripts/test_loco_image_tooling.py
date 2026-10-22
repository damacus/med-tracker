"""Exercise release-image tool drift detection without building the application."""

from pathlib import Path
import re
import shutil
import tempfile
import unittest

from check_loco_image_tooling import check


class ImageToolingTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        source = Path(__file__).resolve().parents[1]
        for name in (
            "Dockerfile", "Cargo.toml", "mise.toml",
            ".github/workflows/ci.yml", ".github/workflows/loco-images.yml",
        ):
            target = self.root / name
            target.parent.mkdir(parents=True, exist_ok=True)
            shutil.copyfile(source / name, target)
        self.rust_version = re.search(
            r"FROM rust:([\d.]+)-", (self.root / "Dockerfile").read_text()
        ).group(1)

    def test_current_image_matches_stable_ci_and_app_tools(self):
        check(self.root, self.rust_version)

    def test_new_stable_requires_image_update(self):
        with self.assertRaisesRegex(ValueError, "Rust"):
            check(self.root, "999.0.0")

    def test_app_node_upgrade_requires_image_update(self):
        path = self.root / "mise.toml"
        source = path.read_text()
        node = re.search(r'^node = "(\d+)"', source, re.MULTILINE).group(1)
        path.write_text(source.replace(f'node = "{node}"', f'node = "{int(node) + 2}"'))
        with self.assertRaisesRegex(ValueError, "Node"):
            check(self.root, self.rust_version)

    def test_task_upgrade_requires_image_workflow_update(self):
        path = self.root / ".github/workflows/ci.yml"
        path.write_text(re.sub(r"version: [\d.]+", "version: 999.0.0", path.read_text()))
        with self.assertRaisesRegex(ValueError, "Task"):
            check(self.root, self.rust_version)


if __name__ == "__main__":
    unittest.main()
