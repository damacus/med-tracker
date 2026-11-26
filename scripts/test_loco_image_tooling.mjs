import assert from "node:assert/strict";
import { cpSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import test from "node:test";
import { runInNewContext } from "node:vm";
import { check } from "./check_loco_image_tooling.mjs";

test("image jobs restore only compatible Cargo caches and populate them on misses", () => {
  const workflow = readFileSync(new URL("../.github/workflows/loco-images.yml", import.meta.url), "utf8");
  const steps = workflow.split("    steps: &image_steps\n")[1].split("\n  image_arm64:")[0];
  assert.match(workflow, /image_arm64:[\s\S]*?ARCH: arm64\n    steps: \*image_steps/);
  assert.doesNotMatch(workflow, /pull_request_target:|^\s*cache-mode:/m);

  const cache = steps.match(/      - name: Cache Cargo build mounts\n([\s\S]+?)(?=\n      - )/)?.[1];
  assert.ok(cache, "image jobs restore a Cargo mount archive");
  const key = cache.match(/^          key: (.+)$/m)[1];
  const prefix = cache.match(/^          restore-keys: (.+)$/m)[1];
  assert.equal(prefix, "loco-cargo-v1-${{ runner.os }}-${{ env.ARCH }}-${{ hashFiles('Dockerfile') }}-");
  assert.equal(key, prefix + "${{ hashFiles('Cargo.toml', 'Cargo.lock', 'migration/Cargo.toml') }}");
  assert.match(cache, /^        continue-on-error: true$/m);

  const transfer = steps.match(/      - name: Transfer Cargo build mounts\n([\s\S]+?)(?=\n      - )/)?.[1];
  assert.ok(transfer, "image jobs transfer Cargo mounts through the archive");
  assert.match(transfer, /^        continue-on-error: true$/m);
  assert.match(transfer, /^          skip-extraction: \$\{\{ steps\.cargo_cache\.outputs\.cache-hit \}\}$/m);
  assert.ok(steps.indexOf("docker/setup-buildx-action@") < steps.indexOf(cache));
  assert.ok(steps.indexOf(cache) < steps.indexOf(transfer));
  assert.ok(steps.indexOf(transfer) < steps.indexOf("docker/build-push-action@"));

  const dockerfile = readFileSync(new URL("../Dockerfile", import.meta.url), "utf8");
  const expectedMounts = [...dockerfile.matchAll(/--mount=type=cache,id=(loco-\w+)-\$\{TARGETARCH\},target=([^,]+),sharing=locked/g)]
    .map(([, id, target]) => ({ id: `${id}-\${{ env.ARCH }}`, target, sharing: "locked" }));
  const cacheMap = JSON.parse(transfer.match(/          cache-map: \|\n([\s\S]+)/)[1]);
  const directory = cache.match(/^          path: (.+)$/m)[1];
  for (const path of Object.keys(cacheMap)) assert.ok(path.startsWith(`${directory}/`));
  assert.deepEqual(Object.values(cacheMap), expectedMounts);

  const prime = steps.match(/^            CARGO_CACHE_PRIME=\$\{\{ (.+) \}\}$/m)[1];
  const value = (cacheHit, outcome) => runInNewContext(prime, {
    steps: { cargo_cache: { outputs: { "cache-hit": cacheHit } }, cargo_mounts: { outcome } },
    github: { run_id: 42, run_attempt: 2 },
    format: (_, runId, attempt) => `${runId}-${attempt}`,
  });
  assert.equal(value("true", "success"), "");
  assert.equal(value("false", "success"), "42-2");
  assert.equal(value("", "success"), "42-2");
  assert.equal(value("true", "failure"), "42-2");
  assert.match(dockerfile, /ARG CARGO_CACHE_PRIME\nRUN --mount=type=cache,id=loco-registry/);
  assert.match(dockerfile, /install -Dm755 target\/release\/med-tracker \/rootfs\/app\/med-tracker \\\n    && cargo clean --frozen --release --workspace/);
});

test("publication allows tagged Release Please runs and opted-in main dispatches only", () => {
  const workflow = readFileSync(new URL("../.github/workflows/loco-images.yml", import.meta.url), "utf8");
  const condition = workflow.match(/^  publish:\n    if: (.+)$/m)[1];
  for (const repository of ["damacus/med-tracker", "fork/med-tracker"]) {
    for (const ref of ["refs/heads/main", "refs/heads/feature", "refs/pull/1/merge"]) {
      for (const event of ["push", "workflow_dispatch", "pull_request", "pull_request_target"]) {
        for (const publish of [false, true]) {
          for (const beta of [false, true]) {
            for (const tag_name of ["", "v0.7.0"]) {
              const inputs = { publish, beta, tag_name };
              const expected = repository === "damacus/med-tracker" && ref === "refs/heads/main"
                && ((event === "push" && publish && tag_name !== "") || (event === "workflow_dispatch" && (publish || beta)));
              assert.equal(runInNewContext(condition, {
                github: { repository, ref, event_name: event }, inputs,
              }), expected, JSON.stringify({ repository, ref, event, inputs }));
            }
          }
        }
      }
    }
  }
});

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
  const { root, rust } = fixture(t);
  assert.match(check(root, rust), /Image tools match app CI/);
});

test("new stable Rust requires an image update", (t) => {
  const { root } = fixture(t);
  assert.throws(() => check(root, "999.0.0"), /Rust/);
});

test("Node patch drift does not require an image update", (t) => {
  const { root, rust, node } = fixture(t);
  assert.match(check(root, rust, `v${node.split(".")[0]}.999.0`), /Image tools match app CI/);
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
