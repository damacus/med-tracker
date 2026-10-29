import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import test from "node:test";
import { runInNewContext } from "node:vm";

const workflow = readFileSync(new URL("../.github/workflows/loco-images.yml", import.meta.url), "utf8");
const revision = "a".repeat(40);
const digest = `sha256:${"b".repeat(64)}`;

test("queued main pushes have independent concurrency groups; PR updates share a group", () => {
  const expression = workflow.match(/^  group: loco-images-\$\{\{ (.+) \}\}$/m)[1];
  const group = (event, runId) => runInNewContext(expression, {
    github: { event_name: event, ref: "refs/heads/main", run_id: runId },
  });
  assert.notEqual(group("push", 1), group("push", 2));
  assert.notEqual(group("workflow_dispatch", 1), group("workflow_dispatch", 2));
  assert.equal(group("pull_request", 1), group("pull_request", 2));
});

function promote(t, main, lookupFails = false, registryDigest = digest) {
  const step = workflow.split("      - name: Promote current main beta\n")[1];
  assert.ok(step, "the guarded beta promotion step exists");
  const script = step.match(/        run: \|\n([\s\S]*)$/)[1].replace(/^          /gm, "");
  const directory = mkdtempSync(join(tmpdir(), "loco-publication-"));
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const log = join(directory, "docker.log");
  const summary = join(directory, "summary.md");
  writeFileSync(log, "");
  writeFileSync(summary, "");
  writeFileSync(join(directory, "gh"), `#!${process.execPath}
if (process.env.LOOKUP_FAILS === "true") process.exit(1);
console.log(process.env.TEST_MAIN);
`, { mode: 0o755 });
  writeFileSync(join(directory, "docker"), `#!${process.execPath}
require("node:fs").appendFileSync(process.env.TEST_LOG, JSON.stringify(process.argv.slice(2)) + "\\n");
if (process.argv.includes("inspect")) console.log(process.env.TEST_REGISTRY_DIGEST);
`, { mode: 0o755 });
  const path = join(directory, "promote.bash");
  writeFileSync(path, `#!/usr/bin/env bash\nset -euo pipefail\n${script}`);
  const result = spawnSync("bash", [path], {
    encoding: "utf8",
    env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, TEST_MAIN: main,
      LOOKUP_FAILS: String(lookupFails), TEST_LOG: log, IMAGE: "ghcr.io/damacus/med-tracker-loco",
      REVISION: revision, DIGEST: digest, TEST_REGISTRY_DIGEST: registryDigest,
      GITHUB_REPOSITORY: "damacus/med-tracker", GITHUB_STEP_SUMMARY: summary },
  });
  return { ...result, commands: readFileSync(log, "utf8").trim().split("\n").filter(Boolean).map(JSON.parse),
    summary: readFileSync(summary, "utf8") };
}

test("current main promotes the immutable digest and verifies beta", (t) => {
  const result = promote(t, revision);
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.commands[0], ["buildx", "imagetools", "create", "--tag",
    "ghcr.io/damacus/med-tracker-loco:beta", `ghcr.io/damacus/med-tracker-loco@${digest}`]);
  assert.equal(result.commands.length, 2);
  assert.match(result.summary, /Beta updated/);
});

test("an older rerun leaves beta untouched and reports its skipped promotion", (t) => {
  const result = promote(t, "c".repeat(40));
  assert.equal(result.status, 0, result.stderr);
  assert.deepEqual(result.commands, []);
  assert.match(result.summary, /Beta unchanged/);
});

test("a registry digest mismatch fails promotion verification", (t) => {
  const result = promote(t, revision, false, `sha256:${"c".repeat(64)}`);
  assert.notEqual(result.status, 0);
  assert.doesNotMatch(result.summary, /Beta updated/);
});

for (const [name, main, fails] of [["failed", "", true], ["malformed", "not-a-commit", false]]) {
  test(`${name} main-ref lookup fails closed without changing beta`, (t) => {
    const result = promote(t, main, fails);
    assert.notEqual(result.status, 0, JSON.stringify(result));
    assert.deepEqual(result.commands, []);
  });
}
