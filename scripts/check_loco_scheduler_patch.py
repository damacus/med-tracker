"""Verify the published Loco archive, complete file inventory and reviewed diff."""

from pathlib import Path
import difflib
import hashlib
import tarfile


ARCHIVE_SHA256 = "5386baf405736a2620001f4561b00fa4874dae99eac3ccf01713fabc9ef13af4"
PACKAGE = "loco-rs-1.2.0"
PATCHED_FILES = {
    "Cargo.toml",
    "Cargo.toml.orig",
    "src/scheduler.rs",
    "src/snapshots/loco_rs__scheduler__tests__can_prepare_command_[shell].snap",
    "src/snapshots/loco_rs__scheduler__tests__can_prepare_command_[task].snap",
}


def check(root):
    vendor = root / "vendor"
    archive = vendor / "upstream" / f"{PACKAGE}.crate"
    if hashlib.sha256(archive.read_bytes()).hexdigest() != ARCHIVE_SHA256:
        raise ValueError("Published Loco archive checksum does not match")
    originals = {}
    with tarfile.open(archive) as source:
        for member in source:
            path = Path(member.name)
            if not member.isfile() or path.parts[0] != PACKAGE or ".." in path.parts:
                raise ValueError("Unexpected published Loco archive member")
            originals[str(Path(*path.parts[1:]))] = source.extractfile(member).read()
    directory = vendor / PACKAGE
    paths = [path for path in directory.rglob("*") if path.is_file() or path.is_symlink()]
    if any(path.is_symlink() for path in paths):
        raise ValueError("Vendored Loco file inventory contains a symlink")
    actuals = {str(path.relative_to(directory)): path.read_bytes() for path in paths}
    if actuals.keys() != originals.keys():
        raise ValueError("Vendored Loco file inventory differs from the published archive")
    differences = []
    for name, original in sorted(originals.items()):
        actual = actuals[name]
        if original == actual:
            continue
        if name not in PATCHED_FILES:
            raise ValueError(f"Vendored file changed outside the scheduler patch: {name}")
        differences.extend(difflib.unified_diff(
            original.decode().splitlines(keepends=True), actual.decode().splitlines(keepends=True),
            fromfile=f"a/{PACKAGE}/{name}", tofile=f"b/{PACKAGE}/{name}",
        ))
    if "".join(differences) != (vendor / "loco-scheduler.patch").read_text():
        raise ValueError("Vendored Loco differs from the reviewed patch")


if __name__ == "__main__":
    check(Path(__file__).resolve().parents[1])
    print("Published Loco archive and reviewed scheduler patch match")
