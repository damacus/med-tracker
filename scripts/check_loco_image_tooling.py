"""Fail when pinned image tools drift from the application's CI tool selection."""

from pathlib import Path
import os
import re
import subprocess


def check(root, rust_version, node_version=None):
    dockerfile = (root / "Dockerfile").read_text()
    mise = (root / "mise.toml").read_text()
    ci = (root / ".github/workflows/ci.yml").read_text()
    workflow = (root / ".github/workflows/loco-images.yml").read_text()
    cargo = (root / "Cargo.toml").read_text()
    rust = re.search(r"FROM rust:([\d.]+)-", dockerfile).group(1)
    if rust != rust_version or set(re.findall(r"toolchain: (\S+)", ci)) != {"stable"}:
        raise ValueError(f"Rust image {rust} must match app CI stable {rust_version}")
    minimum = re.search(r'rust-version = "([\d.]+)"', cargo).group(1)
    if tuple(map(int, rust.split("."))) < tuple(map(int, minimum.split("."))):
        raise ValueError(f"Rust image {rust} is older than application MSRV {minimum}")
    node = re.search(r"FROM node:([\d.]+)-", dockerfile).group(1)
    node_major = node.split(".")[0]
    app_node = re.search(r'^node = "(\d+)"', mise, re.MULTILINE).group(1)
    if node_major != app_node or set(re.findall(r'node-version: "(\d+)"', ci)) != {node_major}:
        raise ValueError(f"Node image {node} must match mise {app_node} and app CI")
    if node_version is not None and node != node_version.removeprefix("v"):
        raise ValueError(f"Node image {node} must match app CI resolved version {node_version}")
    task_pattern = r"uses: arduino/setup-task@[^\n]+\n\s+with:\n\s+version: ([\d.]+)"
    app_tasks = set(re.findall(task_pattern, ci))
    image_tasks = set(re.findall(task_pattern, workflow))
    if not app_tasks or image_tasks != app_tasks:
        raise ValueError(f"Task image workflow {image_tasks} must match app CI {app_tasks}")
    print(f"Image tools match app CI: Rust {rust}, Node {node}, Task {','.join(app_tasks)}")


if __name__ == "__main__":
    compiler = subprocess.check_output(["rustc", "--version"], text=True).split()[1]
    check(Path(__file__).resolve().parents[1], compiler, os.environ.get("IMAGE_CI_NODE_VERSION"))
