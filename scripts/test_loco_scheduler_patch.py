"""Check that only the reviewed scheduler patch can change vendored Loco."""

from pathlib import Path
import shutil
import tempfile
import unittest

from check_loco_scheduler_patch import check


class SchedulerPatchTest(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        source = Path(__file__).resolve().parents[1] / "vendor"
        shutil.copytree(source, self.root / "vendor")

    def test_reviewed_patch_matches_published_archive(self):
        check(self.root)

    def test_modified_upstream_archive_is_rejected(self):
        archive = self.root / "vendor/upstream/loco-rs-1.2.0.crate"
        archive.write_bytes(archive.read_bytes() + b"tampered")
        with self.assertRaisesRegex(ValueError, "archive checksum"):
            check(self.root)

    def test_unrelated_source_change_is_rejected(self):
        source = self.root / "vendor/loco-rs-1.2.0/src/app.rs"
        source.write_text(source.read_text() + "\nfn unexpected_change() {}\n")
        with self.assertRaisesRegex(ValueError, "outside the scheduler patch"):
            check(self.root)

    def test_unreviewed_scheduler_change_is_rejected(self):
        source = self.root / "vendor/loco-rs-1.2.0/src/scheduler.rs"
        source.write_text(source.read_text() + "\nfn unexpected_change() {}\n")
        with self.assertRaisesRegex(ValueError, "reviewed patch"):
            check(self.root)

    def test_extra_source_file_is_rejected(self):
        source = self.root / "vendor/loco-rs-1.2.0/src/unexpected.rs"
        source.write_text("fn unexpected_change() {}\n")
        with self.assertRaisesRegex(ValueError, "file inventory"):
            check(self.root)


if __name__ == "__main__":
    unittest.main()
