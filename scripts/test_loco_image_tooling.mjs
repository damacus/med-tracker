import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { check } from "./check_loco_image_tooling.mjs";

function fixture(t) {
  const root = mkdtempSync(join(tmpdir(), "loco-image-tools-"));
  t.after(() => rmSync(root, { recursive: true }));
  const source = resolve(dirname(fileURLToPath(import.meta.url)), "..");
  for (const name of ["Dockerfile", "Cargo.toml", "mise.toml", ".github/workflows/ci.yml", ".github/workflows/loco-images.yml"]) {
    cpSync(join(source, name), join(root, name), { recursive: true });
  }
  const dockerfile = readFileSync(join(root, "Dockerfile"), "utf8");
  return { root, rust: dockerfile.match(/FROM rust:([\d.]+)-/)[1], node: dockerfile.match(/FROM node:([\d.]+)-/)[1] };
}

test("current image matches stable CI and application tools", (t) => {
  const { root, rust, node } = fixture(t);
  assert.match(check(root, rust, `v${node}`), /Image tools match app CI/);
});

test("new stable Rust requires an image update", (t) => {
  const { root } = fixture(t);
  assert.throws(() => check(root, "999.0.0"), /Rust/);
});

test("new Node patch requires an image update", (t) => {
  const { root, rust, node } = fixture(t);
  assert.throws(() => check(root, rust, `v${node.split(".")[0]}.999.0`), /Node/);
});

test("application Node upgrade requires an image update", (t) => {
  const { root, rust } = fixture(t);
  const path = join(root, "mise.toml");
  writeFileSync(path, readFileSync(path, "utf8").replace(/^node = "(\d+)"/m, (_, major) => `node = "${Number(major) + 2}"`));
  assert.throws(() => check(root, rust), /Node/);
});

test("Task upgrade requires an image workflow update", (t) => {
  const { root, rust } = fixture(t);
  const path = join(root, ".github/workflows/ci.yml");
  writeFileSync(path, readFileSync(path, "utf8").replace(/version: [\d.]+/g, "version: 999.0.0"));
  assert.throws(() => check(root, rust), /Task/);
});

test("Rust image must meet application MSRV", (t) => {
  const { root, rust } = fixture(t);
  const path = join(root, "Cargo.toml");
  writeFileSync(path, readFileSync(path, "utf8").replace(/rust-version = "[\d.]+"/, 'rust-version = "999.0"'));
  assert.throws(() => check(root, rust), /MSRV/);
});
