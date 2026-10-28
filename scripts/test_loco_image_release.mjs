import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { releaseVersion } from "./loco_image_release.mjs";

const revision = "a".repeat(40);
const workflow = (name) => JSON.parse(execFileSync("ruby", ["-ryaml", "-rjson", "-e", "puts JSON.generate(YAML.load_file(ARGV.fetch(0)))", `.github/workflows/${name}.yml`], { encoding: "utf8" }));

test("RC tags preserve the full prerelease version", () => {
  assert.equal(releaseVersion("v0.6.0-rc.1", revision, revision, "0.6.0-rc.1"), "0.6.0-rc.1");
});

test("manual builds retain SHA publication without a release version", () => {
  assert.equal(releaseVersion("", revision, revision, "0.1.0"), "");
});

test("a release must use the tagged commit and matching Cargo version", () => {
  assert.throws(() => releaseVersion("v0.6.0-rc.1", revision, "b".repeat(40), "0.6.0-rc.1"), /commit/);
  assert.throws(() => releaseVersion("v0.6.0-rc.1", revision, revision, "0.1.0"), /Cargo/);
});

test("invalid release versions fail before publication", () => {
  for (const tag of ["latest", "v0.6.0.rc-1", "v0.6.0-rc.01", "v00.6.0", "v0.6.0\nlatest", "v0.6.0\n", "v0.6.0+metadata", "v0.6.0-", "v0.6.0;echo bad"]) {
    assert.throws(() => releaseVersion(tag, revision, revision, tag.slice(1)), /tag/);
  }
});

function publication(t, release = "0.6.0-rc.1", corrupt = false, wrongArchitecture = false) {
  const directory = mkdtempSync(join(tmpdir(), "loco-release-publish-"));
  t.after(() => rmSync(directory, { recursive: true }));
  const bin = join(directory, "bin");
  mkdirSync(bin);
  for (const arch of ["amd64", "arm64"]) {
    const saved = join(directory, "checked", `checked-loco-${arch}`);
    mkdirSync(saved, { recursive: true });
    const files = { "image.tar.gz": `saved-${arch}`, "image.id": `sha256:${arch}\n` };
    const sums = Object.entries(files).map(([name, value]) => {
      writeFileSync(join(saved, name), value);
      return `${createHash("sha256").update(value).digest("hex")}  ${name}\n`;
    }).join("");
    writeFileSync(join(saved, "SHA256SUMS"), sums);
    if (corrupt && arch === "amd64") writeFileSync(join(saved, "image.tar.gz"), "corrupt");
  }
  const docker = join(bin, "docker");
  writeFileSync(docker, `#!${process.execPath}
import { appendFileSync, readFileSync, writeFileSync } from "node:fs";
const args = process.argv.slice(2);
appendFileSync(process.env.CALLS, JSON.stringify(args) + "\\n");
if (args[0] === "image" && args[1] === "load") {
  writeFileSync(process.env.ARCH_FILE, args.at(-1).includes("checked-loco-arm64") ? "arm64" : "amd64");
} else if (args[0] === "image" && args[1] === "inspect") {
  const arch = readFileSync(process.env.ARCH_FILE, "utf8");
  const format = args.at(-1);
  console.log(format.includes(".Id") ? "sha256:" + arch : format.includes(".Os") ? "linux" : process.env.WRONG_ARCH ? "invalid" : arch);
} else if (args[0] === "buildx" && args[2] === "inspect") {
  console.log("sha256:" + "d".repeat(64));
} else if (!["image", "buildx"].includes(args[0])) {
  process.exit(1);
}
`);
  chmodSync(docker, 0o755);
  const script = workflow("loco-images").jobs.publish.steps.find((step) => step.name === "Publish tested images and assemble their manifest").run;
  const result = spawnSync("bash", ["-e", "-o", "pipefail", "-c", script], {
    cwd: directory,
    encoding: "utf8",
    env: { ...process.env, PATH: `${bin}:${process.env.PATH}`, IMAGE: "ghcr.io/damacus/med-tracker-loco", REVISION: revision, RELEASE_VERSION: release, GITHUB_STEP_SUMMARY: join(directory, "summary"), CALLS: join(directory, "calls"), ARCH_FILE: join(directory, "arch"), WRONG_ARCH: wrongArchitecture ? "1" : "" },
  });
  const calls = existsSync(join(directory, "calls")) ? readFileSync(join(directory, "calls"), "utf8").trim().split("\n").map((line) => JSON.parse(line)) : [];
  return { result, calls, directory };
}

test("RC publication loads checked images and tags their shared manifest without latest", (t) => {
  const { result, calls, directory } = publication(t);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(calls.filter((args) => args[0] === "image" && args[1] === "push").length, 2);
  const manifest = calls.find((args) => args[0] === "buildx" && args[2] === "create");
  assert.ok(manifest.includes(`ghcr.io/damacus/med-tracker-loco:sha-${revision}`));
  assert.ok(manifest.includes("ghcr.io/damacus/med-tracker-loco:0.6.0-rc.1"));
  assert.equal(manifest.filter((arg) => arg.includes("@sha256:")).length, 2);
  assert.doesNotMatch(JSON.stringify(calls), /:latest|"build"/);
  assert.match(readFileSync(join(directory, "summary"), "utf8"), /med-tracker-loco@sha256:d{64}/);
});

test("manual publication creates only the SHA manifest", (t) => {
  const { result, calls } = publication(t, "");
  assert.equal(result.status, 0, result.stderr);
  const manifest = calls.find((args) => args[0] === "buildx" && args[2] === "create");
  assert.equal(manifest.filter((arg) => arg === "--tag").length, 1);
});

test("corrupted saved images fail before any registry mutation", (t) => {
  const { result, calls } = publication(t, "0.6.0-rc.1", true);
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /checksum/);
  assert.equal(calls.length, 0);
});

test("a saved image of the wrong architecture cannot be published", (t) => {
  const { result, calls } = publication(t, "0.6.0-rc.1", false, true);
  assert.notEqual(result.status, 0);
  assert.equal(calls.some((args) => args[1] === "push" || args[2] === "create"), false);
});

test("release-please versions Rust and publishes the checked Loco image", () => {
  const config = JSON.parse(readFileSync("release-please-config.json", "utf8")).packages["."];
  assert.equal(config["release-type"], "rust");
  assert.equal(config["release-as"], "0.6.0-rc.1");
  assert.equal(config.prerelease, true);
  assert.equal(config["version-file"], undefined);
  const release = workflow("release-please");
  assert.equal(release.jobs.docker.uses, "./.github/workflows/loco-images.yml");
  assert.equal(release.jobs.docker.with.publish, true);
  assert.equal(release.jobs.docker.with.tag_name, "${{ needs.release-please.outputs.tag_name }}");
});

test("publishing retains architecture and scheduler checks and exact source checkout", () => {
  const image = workflow("loco-images");
  assert.ok((image.on ?? image.true).workflow_call.inputs.tag_name);
  assert.deepEqual(image.jobs.checked_images.needs, ["image", "scheduler_acceptance"]);
  assert.ok(image.jobs.publish.needs.includes("checked_images"));
  assert.ok(image.jobs.publish.needs.includes("tooling"));
  for (const job of Object.values(image.jobs)) {
    for (const step of job.steps ?? []) {
      if (step.uses?.startsWith("actions/checkout@")) {
        assert.equal(step.with.ref, "${{ inputs.tag_name || github.sha }}");
      }
    }
  }
  const publish = image.jobs.publish.steps.find((step) => step.name === "Publish tested images and assemble their manifest");
  assert.match(publish.run, /RELEASE_VERSION/);
  assert.doesNotMatch(publish.run, /:latest|docker build(?: |$)/);
  assert.equal(image.jobs.publish.steps.some((step) => step.uses?.startsWith("actions/checkout@")), false);
});
