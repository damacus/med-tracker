import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";

const helper = fileURLToPath(new URL("./check_whitespace.fish", import.meta.url));

function fixture(t) {
  const directory = mkdtempSync(join(tmpdir(), "medtracker whitespace-"));
  const temporary = join(directory, "temporary");
  const repo = join(directory, "repo");
  mkdirSync(temporary);
  mkdirSync(repo);
  t.after(() => rmSync(directory, { recursive: true, force: true }));
  const env = { ...process.env, TMPDIR: temporary, GIT_CONFIG_NOSYSTEM: "1", GIT_CONFIG_GLOBAL: "/dev/null" };
  for (const key of ["GIT_INDEX_FILE", "GIT_DIR", "GIT_WORK_TREE", "GIT_COMMON_DIR", "GIT_OBJECT_DIRECTORY", "GIT_ALTERNATE_OBJECT_DIRECTORIES"]) delete env[key];
  const git = (...args) => {
    const result = spawnSync("git", args, { cwd: repo, env, encoding: "utf8" });
    assert.equal(result.status, 0, result.stderr);
    return result.stdout.trim();
  };
  const write = (name, content) => writeFileSync(join(repo, name), content);
  git("init", "-b", "feature");
  git("config", "user.name", "Whitespace Test");
  git("config", "user.email", "whitespace@example.invalid");
  write("tracked.txt", "clean\n");
  write(".gitignore", "ignored*\n");
  git("add", ".");
  git("-c", "core.hooksPath=/dev/null", "-c", "commit.gpgsign=false", "commit", "-m", "fixture");
  const check = (...args) => {
    const index = join(repo, ".git", "index");
    const before = readFileSync(index);
    const objects = git("count-objects", "-v");
    const result = spawnSync("fish", ["--no-config", helper, ...args], { cwd: repo, env, encoding: "utf8" });
    assert.deepEqual(readFileSync(index), before, "preserve the real index byte for byte");
    assert.equal(git("count-objects", "-v"), objects, "leave the real object database unchanged");
    assert.deepEqual(readdirSync(temporary), [], "remove temporary indexes");
    return result;
  };
  return { repo, git, write, check, env };
}

for (const state of ["staged", "unstaged", "untracked", "staged-fixed-in-worktree"]) {
  test(`rejects whitespace in ${state} content without changing the index`, t => {
    const { write, git, check } = fixture(t);
    const name = state === "untracked" ? "new file.txt" : "tracked.txt";
    write(name, "bad \n\n");
    if (state.startsWith("staged")) git("add", name);
    if (state === "staged-fixed-in-worktree") write(name, "fixed\n");
    const result = check();
    assert.notEqual(result.status, 0);
    assert.match(result.stdout + result.stderr, /trailing whitespace|new blank line at EOF/);
    assert.ok((result.stdout + result.stderr).includes(name));
  });
}

test("accepts clean additions, deletions and ignored whitespace", t => {
  const { repo, write, git, check } = fixture(t);
  write("new file.txt", "clean\n");
  write("ignored.txt", "ignored \n\n");
  write("staged.txt", "clean\n");
  git("add", "staged.txt");
  rmSync(join(repo, "tracked.txt"));
  const result = check();
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

test("respects whitespace attributes", t => {
  const { write, check } = fixture(t);
  write(".gitattributes", "preserved.txt -whitespace\n");
  write("preserved.txt", "preserved \n\n");
  const result = check();
  assert.equal(result.status, 0, result.stdout + result.stderr);
});

test("leaves shared index files unchanged when split index is enabled", t => {
  const { repo, write, git, check } = fixture(t);
  git("config", "core.splitIndex", "true");
  git("update-index", "--split-index");
  const sharedIndexes = () => readdirSync(join(repo, ".git"))
    .filter(name => name.startsWith("sharedindex."))
    .sort()
    .map(name => [name, readFileSync(join(repo, ".git", name))]);
  const before = sharedIndexes();
  write("new file.txt", "clean\n");
  const result = check();
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.deepEqual(sharedIndexes(), before);
});

test("uses the merge base of an explicit non-main feature base", t => {
  const { write, git, check } = fixture(t);
  git("branch", "parent-feature");
  write("branch.txt", "branch \n");
  git("add", ".");
  git("-c", "commit.gpgsign=false", "commit", "-m", "branch change");
  git("checkout", "parent-feature");
  write("parent.txt", "parent\n");
  git("add", ".");
  git("-c", "commit.gpgsign=false", "commit", "-m", "parent change");
  git("checkout", "feature");
  assert.equal(check().status, 0, "default checks changes since HEAD");
  const result = check("parent-feature");
  assert.notEqual(result.status, 0);
  assert.match(result.stdout + result.stderr, /branch.txt/);
});

test("fails safely for a missing base", t => {
  const { check } = fixture(t);
  const result = check("missing-base");
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /missing-base/);
});

test("cleans up and preserves staging when Git cannot add a file", t => {
  const { write, git, check } = fixture(t);
  git("config", "filter.reject.clean", "false");
  git("config", "filter.reject.required", "true");
  write(".gitattributes", "rejected.txt filter=reject\n");
  write("rejected.txt", "clean\n");
  const result = check();
  assert.notEqual(result.status, 0);
  assert.match(result.stderr, /filter.*failed/);
});

test("cleans up and preserves staging when interrupted", t => {
  const { repo, env, check } = fixture(t);
  const bin = join(repo, ".git", "test-bin");
  mkdirSync(bin);
  const executable = join(bin, "git");
  writeFileSync(executable, `#!/usr/bin/env node
const { spawnSync } = require("node:child_process");
if (process.argv.slice(2).includes("add")) {
  process.kill(process.ppid, "SIGTERM");
  process.exit(0);
}
const env = { ...process.env, PATH: process.env.WHITESPACE_TEST_PATH };
const result = spawnSync("git", process.argv.slice(2), { env, stdio: "inherit" });
process.exit(result.status ?? 1);
`);
  chmodSync(executable, 0o755);
  env.WHITESPACE_TEST_PATH = env.PATH;
  env.PATH = `${bin}:${env.PATH}`;
  const result = check();
  assert.equal(result.status, 143, result.stdout + result.stderr);
});
