import assert from "node:assert/strict";
import { readFileSync, existsSync, mkdtempSync, writeFileSync, chmodSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";
import test from "node:test";
const read = path => readFileSync(new URL(`../${path}`, import.meta.url), "utf8");
test("public Compose tasks run from the repository root", () => {
  const directory = mkdtempSync(join(tmpdir(), "medtracker-task-cwd-"));
  const executable = join(directory, "docker");
  const log = join(directory, "calls.jsonl");
  writeFileSync(executable, '#!/usr/bin/env node\nconst fs = require("node:fs"); fs.appendFileSync(process.env.COMPOSE_CALL_LOG, JSON.stringify({cwd:process.cwd(),args:process.argv.slice(2)})+"\\n"); if(process.argv[2]==="info")console.log("arm64");\n');
  chmodSync(executable, 0o755);
  try {
    for (const environment of ["dev", "test", "prod"]) {
      const result = spawnSync("task", [`${environment}:build`], {encoding:"utf8", env:{...process.env, PATH:`${directory}:${process.env.PATH}`, COMPOSE_CALL_LOG:log}});
      assert.equal(result.status, 0, result.stderr);
    }
    const calls = readFileSync(log, "utf8").trim().split("\n").map(JSON.parse);
    assert.equal(calls.filter(call => call.args[0] === "compose").length, 3);
    for (const call of calls.filter(call => call.args[0] === "compose")) assert.equal(call.cwd, resolve(fileURLToPath(new URL("..", import.meta.url))), JSON.stringify(call));
  } finally {
    rmSync(directory, {recursive:true, force:true});
  }
});
test("containers use native tools without a Task installation or wrapper", () => {
  assert.doesNotMatch(read("Dockerfile"), /task_linux|COPY Taskfiles/);
  assert.doesNotMatch(read("compose.local.yaml"), /task --dir|container.yml/);
  assert.equal(existsSync(new URL("compose_workflows.py", import.meta.url)), false);
});
test("host checks have an explicit independent entrypoint", () => {
  assert.match(read("Taskfiles/host.yml"), /cargo clippy/);
  assert.doesNotMatch(read("Taskfile.yml"), /eq \(.CI|compose_workflows/);
  assert.match(read(".github/workflows/ci.yml"), /task host:test:unit/);
});
test("seed acceptance invokes the actual guarded executable", () => {
  const source = read("tests/persistence.rs");
  assert.doesNotMatch(source, /Command::new\("task"\)/);
  assert.match(source, /Command::new\(env!\("CARGO_BIN_EXE_med-tracker"\)\)/);
});
test("focused Docker tests retain target/filter and unit needs no services", () => {
  const result = spawnSync("task", ["--dry", "--verbose", "test:unit", "TARGET=med-tracker", "FILTER=public_origin"], {encoding:"utf8"});
  assert.equal(result.status, 0, result.stderr);
  const output = result.stdout + result.stderr;
  assert.match(output, /nextest.*--package.*med-tracker.*public_origin/s);
  assert.doesNotMatch(output, /up.*postgres|up.*rustfs/);
});
test("up origin variables cannot recurse through the old port lookup", () => {
  const source = read("Taskfiles/compose.yml");
  const compose = source.slice(source.indexOf("  COMPOSE:"), source.indexOf("  APP:"));
  assert.doesNotMatch(compose, /HTTP_PORT|ORIGIN/);
  assert.match(source, /test "{{.ENV}}" = prod \|\| printf "http:\/\/127.0.0.1:/);
  assert.match(source, /MT_HTTP_PORT={{.HTTP_PORT.*MT_PUBLIC_ORIGIN={{.ORIGIN/);
  assert.doesNotMatch(source, /node -e|task .*internal:/);
});
test("native integration and browser commands preserve focused arguments", () => {
  for (const kind of ["integration", "browser"]) {
    const result = spawnSync("task", ["--dry", "--verbose", `test:${kind}`, "TARGET=requests", "FILTER=two words"], {encoding:"utf8"});
    assert.equal(result.status, 0, result.stderr);
    const output = result.stdout + result.stderr;
    assert.match(output, /run --rm --no-deps tool cargo nextest.*--test requests.*'two words'/);
    assert.match(output, /up -d --wait postgres rustfs/);
    assert.doesNotMatch(output, /\] task test:run/);
  }
});
test("resolved Compose retains cache sharing, isolated data and native builder", () => {
  const configs = [["dev", "3"], ["test", "3"], ["test", undefined]].map(([environment, threads]) => {
    const result = spawnSync("docker", ["compose", "--env-file", "/dev/null", "-p", `tooling-${environment}`, "-f", "compose.yaml", "-f", "compose.local.yaml", "--profile", environment, "config", "--format", "json"], {
      encoding:"utf8", env:{...process.env, NEXTEST_TEST_THREADS:threads, MT_PROJECT:`tooling-${environment}`, MT_TOOL_TARGET:environment === "dev" ? "development" : "test", MT_TARGET_CACHE:"checkout-target-linux-arm64", MT_CARGO_REGISTRY_CACHE:"mt-cargo-registry-v1", MT_CARGO_GIT_CACHE:"mt-cargo-git-v1", MT_BROWSER_CACHE:"mt-browser-linux-arm64-ubuntu26-v1"}
    });
    assert.equal(result.status, 0, result.stderr);
    return JSON.parse(result.stdout);
  });
  for (const cache of ["target", "cargo-registry", "cargo-git", "browser-cache"]) {
    assert.equal(configs[0].volumes[cache].external, true);
    assert.equal(configs[0].volumes[cache].name, configs[1].volumes[cache].name);
  }
  assert.notEqual(configs[0].volumes.database.name, configs[1].volumes.database.name);
  for (const config of configs.slice(0, 2)) assert.equal(config.services.tool.environment.NEXTEST_TEST_THREADS, "3");
  assert.equal(configs[2].services.tool.environment.NEXTEST_TEST_THREADS ?? null, null);
  assert.match(configs[0].services["dev-builder"].command.join(" "), /cargo install/);
  assert.doesNotMatch(configs[0].services["dev-builder"].command.join(" "), /task /);
  assert.equal(configs[1].services.tool.volumes.some(volume => volume.type === "bind"), false);
  assert.match(read("Dockerfile"), /FROM scratch AS runtime[\s\S]*USER 65532:65532/);
});
test("reset guards remove only named data volumes", () => {
  const source = read("Taskfiles/compose.yml");
  assert.match(source, /Reset requires CONFIRM={{.PROJECT}}/);
  assert.match(source, /docker volume rm {{.PROJECT}}_database {{.PROJECT}}_scanner-data {{.PROJECT}}_scanner-logs/);
  assert.doesNotMatch(source, /down --volumes|docker volume prune/);
  assert.doesNotMatch(read("Taskfiles/prod.yml"), /  seed:|  reset:/);
});
test("approved provisioning retains restricted role and existing-role refusal", () => {
  const source = read("Taskfiles/compose.yml");
  const provision = source.slice(source.indexOf("  provision:"), source.indexOf("  data-check:"));
  assert.match(provision, /set -eu/);
  assert.match(provision, /if \[ -z "\$exists" \]; then[\s\S]*createuser -U medtracker --login --inherit --no-superuser --no-createdb --no-createrole --member-of=med_tracker_app medtracker_runtime/);
  assert.match(provision, /ALTER ROLE medtracker_runtime NOBYPASSRLS PASSWORD 'synthetic_local_runtime_password'/);
  assert.match(provision, /rolcanlogin AND rolinherit AND NOT rolsuper AND NOT rolcreatedb AND NOT rolcreaterole AND NOT rolbypassrls/);
  assert.match(provision, /pg_has_role\('medtracker_runtime', 'med_tracker_app', 'MEMBER'\)/);
  assert.match(provision, /test "\$restricted" = t \|\|.*refusing to alter an existing role/);
});
test("production app and scheduler receive both synthetic storage operators", () => {
  const result = spawnSync("docker", ["compose", "--env-file", "/dev/null", "-p", "tooling-prod", "-f", "compose.yaml", "-f", "compose.local.yaml", "--profile", "prod", "config", "--format", "json"], {
    encoding: "utf8", env: {...process.env, MT_PROJECT:"tooling-prod", MT_TOOL_TARGET:"development", MT_TARGET_CACHE:"test-target", MT_CARGO_REGISTRY_CACHE:"test-registry", MT_CARGO_GIT_CACHE:"test-git", MT_BROWSER_CACHE:"test-browser"}
  });
  assert.equal(result.status, 0, result.stderr);
  const config = JSON.parse(result.stdout);
  for (const service of ["app", "scheduler"]) {
    const env = config.services[service].environment;
    assert.equal(env.ACTIVE_STORAGE_S3_ENDPOINT, "http://rustfs:9000");
    assert.equal(env.ACTIVE_STORAGE_S3_BUCKET, "medtracker-fixture");
    assert.equal(env.ACTIVE_STORAGE_S3_REGION, "us-east-1");
    assert.equal(env.ACTIVE_STORAGE_S3_ACCESS_KEY_ID, "storage-smoke-access");
    assert.equal(env.ACTIVE_STORAGE_S3_SECRET_ACCESS_KEY, "storage-smoke-secret");
    assert.equal(env.ACTIVE_STORAGE_S3_FORCE_PATH_STYLE, "true");
    assert.equal(env.S3_ENDPOINT, "http://rustfs:9000");
    assert.equal(env.S3_BUCKET, "scanner-fixture");
    assert.equal(env.AWS_REGION, "us-east-1");
    assert.equal(env.AWS_ACCESS_KEY_ID, "storage-smoke-access");
    assert.equal(env.AWS_SECRET_ACCESS_KEY, "storage-smoke-secret");
  }
});
