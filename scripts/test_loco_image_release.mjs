import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { createHash } from "node:crypto";
import { chmodSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { runInNewContext } from "node:vm";
import test from "node:test";
import { releaseVersion } from "./loco_image_release.mjs";

const revision = "a".repeat(40);
const manifestMediaType = "application/vnd.oci.image.index.v1+json";
const manifest = JSON.stringify({
  schemaVersion: 2,
  mediaType: manifestMediaType,
  manifests: ["amd64", "arm64"].map((architecture) => ({ platform: { os: "linux", architecture } })),
});
const differentManifest = manifest + "\n";
const manifestDigest = `sha256:${createHash("sha256").update(manifest).digest("hex")}`;
const yaml = (path) => JSON.parse(execFileSync("ruby", ["-ryaml", "-rjson", "-e", "puts JSON.generate(YAML.parse_file(ARGV.fetch(0)).to_ruby)", path], { encoding: "utf8" }));
const workflow = (name) => yaml(`.github/workflows/${name}.yml`);

test("deployment defaults use the published release image", () => {
  const version = JSON.parse(readFileSync(".release-please-manifest.json", "utf8"))["."];
  const publish = workflow("loco-images").jobs.publish.steps.find((step) => step.id === "publish");
  const repository = publish.env.IMAGE.replace("${{ github.repository }}", "damacus/med-tracker");
  const compose = readFileSync("compose.production.yaml", "utf8");
  const defaults = Array.from(compose.matchAll(/image: \$\{LOCO_IMAGE:-([^}]+)\}/g), ([, image]) => image);
  assert.deepEqual(defaults, Array(3).fill(`${repository}:${version}`));
  const image = yaml("deploy/helm/values.yaml").controllers.web.containers.app.image;
  assert.equal(image.repository, repository);
  assert.equal(String(image.tag), version);
});

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

function publication(t, release = "0.6.0-rc.1", {
  corrupt = false, wrongArchitecture = false, existingSha, existingRelease, lookupStatus, lookupBody,
  lookupDigest, publishedRelease = manifest, rejectRelease = false,
} = {}) {
  const directory = mkdtempSync(join(tmpdir(), "loco-release-publish-"));
  t.after(() => rmSync(directory, { recursive: true }));
  const bin = join(directory, "bin");
  mkdirSync(bin);
  const registry = join(directory, "registry.json");
  const shaTag = `sha-${revision}`;
  writeFileSync(registry, JSON.stringify({ [shaTag]: existingSha, [release]: existingRelease }));
  const curl = join(bin, "curl");
  writeFileSync(curl, `#!${process.execPath}
import { createHash } from "node:crypto";
import { readFileSync, writeFileSync } from "node:fs";
const args = process.argv.slice(2);
const url = new URL(args.at(-1));
if (url.pathname === "/token") {
  console.log(JSON.stringify({ token: "fixture-read-token" }));
} else {
  const tag = url.pathname.split("/").at(-1);
  const contents = JSON.parse(readFileSync(process.env.REGISTRY, "utf8"))[tag];
  const status = process.env.LOOKUP_STATUS || (contents === undefined ? "404" : "200");
  const body = process.env.LOOKUP_BODY || contents || '{"errors":[{"code":"MANIFEST_UNKNOWN"}]}';
  const digest = process.env.LOOKUP_DIGEST || "sha256:" + createHash("sha256").update(body).digest("hex");
  const headers = status === "200"
    ? "content-type: ${manifestMediaType}\\r\\ndocker-content-digest: " + digest + "\\r\\n"
    : "";
  writeFileSync(args[args.indexOf("--dump-header") + 1], headers);
  writeFileSync(args[args.indexOf("--output") + 1], body);
  process.stdout.write(status);
}
`, { mode: 0o755 });
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
  const reference = args[3];
  if (reference.endsWith("-amd64")) console.log("sha256:" + "a".repeat(64));
  else if (reference.endsWith("-arm64")) console.log("sha256:" + "b".repeat(64));
  else console.log(process.env.MANIFEST_DIGEST);
} else if (args[0] === "buildx" && args[2] === "create") {
  const registry = JSON.parse(readFileSync(process.env.REGISTRY, "utf8"));
  for (const [index, arg] of args.entries()) {
    if (arg === "--tag") {
      const tag = args[index + 1].split(":").at(-1);
      if (tag === process.env.RELEASE_VERSION && process.env.REJECT_RELEASE === "1") process.exit(1);
      registry[tag] = tag === process.env.RELEASE_VERSION ? process.env.PUBLISHED_RELEASE : process.env.MANIFEST;
    }
  }
  writeFileSync(process.env.REGISTRY, JSON.stringify(registry));
} else if (!["image", "buildx"].includes(args[0])) {
  process.exit(1);
}
`);
  chmodSync(docker, 0o755);
  const script = workflow("loco-images").jobs.publish.steps.find((step) => step.name === "Publish tested images and assemble their manifest").run;
  const result = spawnSync("bash", ["-e", "-o", "pipefail", "-c", script], {
    cwd: directory,
    encoding: "utf8",
    env: {
      ...process.env,
      PATH: `${bin}:${process.env.PATH}`,
      IMAGE: "ghcr.io/damacus/med-tracker-loco",
      REVISION: revision,
      RELEASE_VERSION: release,
      GITHUB_ACTOR: "fixture",
      GHCR_TOKEN: "fixture-token",
      GITHUB_STEP_SUMMARY: join(directory, "summary"),
      GITHUB_OUTPUT: join(directory, "outputs"),
      RUNNER_TEMP: directory,
      CALLS: join(directory, "calls"),
      ARCH_FILE: join(directory, "arch"),
      WRONG_ARCH: wrongArchitecture ? "1" : "",
      REGISTRY: registry,
      LOOKUP_STATUS: lookupStatus ?? "",
      LOOKUP_BODY: lookupBody ?? "",
      LOOKUP_DIGEST: lookupDigest ?? "",
      MANIFEST: manifest,
      MANIFEST_DIGEST: manifestDigest,
      PUBLISHED_RELEASE: publishedRelease,
      REJECT_RELEASE: rejectRelease ? "1" : "",
    },
  });
  const calls = existsSync(join(directory, "calls")) ? readFileSync(join(directory, "calls"), "utf8").trim().split("\n").map((line) => JSON.parse(line)) : [];
  return { result, calls, directory };
}

test("RC publication loads checked images and tags their shared manifest without latest", (t) => {
  const { result, calls, directory } = publication(t);
  assert.equal(result.status, 0, result.stderr);
  assert.equal(calls.filter((args) => args[0] === "image" && args[1] === "push").length, 2);
  assert.deepEqual(calls.filter((args) => args[0] === "buildx" && args[2] === "create"), [
    ["buildx", "imagetools", "create", "--tag", `ghcr.io/damacus/med-tracker-loco:sha-${revision}`,
      `ghcr.io/damacus/med-tracker-loco@sha256:${"a".repeat(64)}`,
      `ghcr.io/damacus/med-tracker-loco@sha256:${"b".repeat(64)}`],
    ["buildx", "imagetools", "create", "--tag", "ghcr.io/damacus/med-tracker-loco:0.6.0-rc.1",
      `ghcr.io/damacus/med-tracker-loco@${manifestDigest}`],
  ]);
  assert.doesNotMatch(JSON.stringify(calls), /:latest|"build"/);
  assert.ok(readFileSync(join(directory, "summary"), "utf8").includes(`med-tracker-loco@${manifestDigest}`));
  assert.equal(readFileSync(join(directory, "outputs"), "utf8"), `digest=${manifestDigest}\n`);
});

test("a release tag pointing to a different digest fails publication verification", (t) => {
  const { result, directory } = publication(t, "0.6.0-rc.1", { publishedRelease: differentManifest });
  assert.notEqual(result.status, 0);
  assert.equal(existsSync(join(directory, "outputs")), false);
});

test("manual runs return the previously verified SHA digest without replacing it", (t) => {
  const { result, calls, directory } = publication(t, "", { existingSha: manifest });
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(calls, []);
  assert.equal(readFileSync(join(directory, "outputs"), "utf8"), `digest=${manifestDigest}\n`);
});

test("a release can tag the existing canonical SHA digest without replacing it", (t) => {
  const release = "0.6.0-rc.1";
  const { result, calls } = publication(t, release, { existingSha: manifest });
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(calls, [["buildx", "imagetools", "create", "--tag",
    `ghcr.io/damacus/med-tracker-loco:${release}`, `ghcr.io/damacus/med-tracker-loco@${manifestDigest}`]]);
});

test("a release alias write can be retried after its canonical SHA tag was published", (t) => {
  const release = "0.6.0-rc.1";
  const first = publication(t, release, { rejectRelease: true });
  assert.notEqual(first.result.status, 0);
  const registry = JSON.parse(readFileSync(join(first.directory, "registry.json"), "utf8"));
  assert.equal(registry[`sha-${revision}`], manifest);
  assert.equal(registry[release], undefined);

  const retry = publication(t, release, { existingSha: registry[`sha-${revision}`] });
  assert.equal(retry.result.status, 0, retry.result.stderr);
  assert.deepEqual(retry.calls, [["buildx", "imagetools", "create", "--tag",
    `ghcr.io/damacus/med-tracker-loco:${release}`, `ghcr.io/damacus/med-tracker-loco@${manifestDigest}`]]);
});

test("an existing release alias with a different digest fails without mutation", (t) => {
  const { result, calls } = publication(t, "0.6.0-rc.1", { existingSha: manifest, existingRelease: differentManifest });
  assert.notEqual(result.status, 0);
  assert.deepEqual(calls, []);
});

for (const [name, lookupStatus, lookupBody, lookupDigest] of [
  ["registry outage", "503", ""],
  ["unstructured not found", "404", "<html>not found</html>"],
  ["non-index SHA manifest", "200", '{"schemaVersion":2,"config":{}}'],
  ["digest header differs from the index", "200", manifest, `sha256:${"c".repeat(64)}`],
]) {
  test(`${name} fails without mutation`, (t) => {
    const { result, calls } = publication(t, "", { lookupStatus, lookupBody, lookupDigest });
    assert.notEqual(result.status, 0);
    assert.deepEqual(calls, []);
  });
}

test("manual publication creates only the SHA manifest", (t) => {
  const { result, calls } = publication(t, "");
  assert.equal(result.status, 0, result.stderr);
  const manifest = calls.find((args) => args[0] === "buildx" && args[2] === "create");
  assert.equal(manifest.filter((arg) => arg === "--tag").length, 1);
});

test("corrupted saved images fail before any registry mutation", (t) => {
  const { result, calls } = publication(t, "0.6.0-rc.1", { corrupt: true });
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /checksum/);
  assert.equal(calls.length, 0);
});

test("a saved image of the wrong architecture cannot be published", (t) => {
  const { result, calls } = publication(t, "0.6.0-rc.1", { wrongArchitecture: true });
  assert.notEqual(result.status, 0);
  assert.equal(calls.some((args) => args[1] === "push" || args[2] === "create"), false);
});

test("release-please versions Rust and publishes the checked Loco image", () => {
  const config = JSON.parse(readFileSync("release-please-config.json", "utf8")).packages["."];
  assert.equal(config["release-type"], "rust");
  assert.equal(config["release-as"], undefined);
  assert.equal(config.prerelease, false);
  assert.equal(config["version-file"], undefined);
  const release = workflow("release-please");
  assert.equal(release.jobs.docker.uses, "./.github/workflows/loco-images.yml");
  assert.equal(release.jobs.docker.if, "${{ needs.release-please.outputs.releases_created == 'true' }}");
  assert.equal(release.jobs.docker.with.publish, true);
  assert.equal(release.jobs.docker.with.tag_name, "${{ needs.release-please.outputs.tag_name }}");
  assert.equal(release.jobs.docker.with.beta, undefined);
});

test("release publication selects its repository without a checkout", (t) => {
  const directory = mkdtempSync(join(tmpdir(), "loco-release-edit-"));
  t.after(() => rmSync(directory, { recursive: true }));
  const publish = workflow("release-please").jobs["publish-release"];
  assert.deepEqual(publish.needs, ["release-please", "docker"]);
  const step = publish.steps.find((entry) => entry.name === "Publish the release after image verification");
  assert.equal(step.env.GH_REPO, "${{ github.repository }}");
  const gh = join(directory, "gh");
  writeFileSync(gh, `#!${process.execPath}
import assert from "node:assert/strict";
assert.equal(process.env.GH_REPO, "example/med-tracker");
assert.deepEqual(process.argv.slice(2), ["release", "edit", "v0.6.0", "--draft=false", "--verify-tag"]);
`);
  chmodSync(gh, 0o755);
  const result = spawnSync("bash", ["-e", "-o", "pipefail", "-c", step.run], {
    cwd: directory,
    encoding: "utf8",
    env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, GH_REPO: step.env.GH_REPO.replace("${{ github.repository }}", "example/med-tracker"), RELEASE_TAG: "v0.6.0" },
  });
  assert.equal(result.status, 0, result.stderr);
});

test("the workflow builds both PR architectures and only explicitly requested main images", () => {
  const image = workflow("loco-images");
  const events = image.on ?? image.true;
  assert.equal(events.push, undefined);
  assert.ok(events.pull_request.paths.includes("src/**"));
  assert.ok(events.workflow_call.inputs.tag_name.required);
  assert.equal(events.workflow_dispatch.inputs.publish.default, false);
  assert.equal(events.workflow_dispatch.inputs.beta.default, false);
});

test("the scheduler can start once AMD64 passes, while publication waits for all image checks", () => {
  const image = workflow("loco-images");
  for (const [arch, runner] of [["amd64", "ubuntu-24.04"], ["arm64", "ubuntu-24.04-arm"]]) {
    const job = image.jobs[`image_${arch}`];
    assert.equal(job["runs-on"], runner);
    assert.equal(job.env.ARCH, arch);
    assert.equal(job.needs, "tooling");
    assert.equal(job.steps.filter((step) => step.uses?.startsWith("docker/build-push-action@")).length, 1);
    assert.ok(job.steps.some((step) => step.run === "task image:test"));
    assert.ok(job.steps.some((step) => step.run === 'task image:test:upgrade PREVIOUS_IMAGE="$PREVIOUS_IMAGE"'));
  }
  assert.equal(image.jobs.scheduler_acceptance.needs, "image_amd64");
  assert.ok(image.jobs.scheduler_acceptance.steps.some((step) => step.run === "task test:scheduler:image"));
  assert.deepEqual(image.jobs.checked_images.needs, ["image_amd64", "image_arm64", "scheduler_acceptance"]);
  assert.ok(image.jobs.publish.needs.includes("checked_images"));
  assert.ok(image.jobs.publish.needs.includes("tooling"));
});

test("beta promotes only an opted-in dispatch's published digest", () => {
  const image = workflow("loco-images");
  assert.equal(image.jobs.publish.outputs.digest, "${{ steps.publish.outputs.digest }}");
  assert.equal(image.jobs.beta.needs, "publish");
  for (const event of ["push", "pull_request", "workflow_dispatch"]) {
    for (const beta of [false, true]) {
      assert.equal(runInNewContext(image.jobs.beta.if, { github: { event_name: event }, inputs: { beta } }),
        event === "workflow_dispatch" && beta);
    }
  }
  assert.equal(image.jobs.beta.steps.find((step) => step.name === "Promote current main beta").env.DIGEST, "${{ needs.publish.outputs.digest }}");
});

test("all checks use the exact source revision and publication cannot rebuild it", () => {
  const image = workflow("loco-images");
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

test("main and RC publishers for one commit share a serial publication group", () => {
  const concurrency = workflow("loco-images").jobs.publish.concurrency;
  assert.ok(concurrency, "same-commit publication must be serialized");
  assert.equal(concurrency["cancel-in-progress"], false);
  assert.equal(concurrency.queue, "max");
  const evaluate = (sha, runId, tag) => concurrency.group.replace(/\$\{\{ (.+?) \}\}/g, (_, expression) => runInNewContext(expression, {
    github: { sha, run_id: runId, event_name: "push", ref: "refs/heads/main" }, inputs: { tag_name: tag },
  }));
  assert.equal(evaluate(revision, 1, ""), evaluate(revision, 2, "v0.6.0-rc.1"));
  assert.notEqual(evaluate(revision, 1, ""), evaluate("b".repeat(40), 2, ""));
});
