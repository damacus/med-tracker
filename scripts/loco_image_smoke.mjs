import assert from "node:assert/strict";
import { execFileSync, spawnSync } from "node:child_process";
import { randomUUID } from "node:crypto";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { setTimeout } from "node:timers/promises";

export const image = process.env.LOCO_IMAGE ?? "medtracker-loco:local";
export const docker = (...args) => execFileSync("docker", args, { encoding: "utf8", timeout: 180_000, maxBuffer: 32 * 1024 * 1024 });

export async function eventually(check, attempts, message, delay = () => setTimeout(1000)) {
  for (let attempt = 0; attempt < attempts; attempt += 1) {
    if (await check()) return;
    if (attempt + 1 < attempts) await delay();
  }
  throw new Error(message);
}

export function cleanup(containers, network, run = docker) {
  const errors = [];
  for (const container of [...containers].reverse()) {
    try { run("rm", "--force", "--volumes", container); } catch (error) { errors.push(error); }
  }
  try { run("network", "rm", network); } catch (error) { errors.push(error); }
  if (errors.length) throw new AggregateError(errors, "Disposable image-test resource cleanup failed");
}

export function checkCommands() {
  for (const args of [["--help"], ["start", "--help"], ["db", "--help"], ["scheduler", "--help"]]) {
    assert.match(docker("run", "--rm", "--pull=never", "--network=none", "--read-only", "--cap-drop=ALL", "--security-opt=no-new-privileges=true", image, ...args), /Usage:/);
  }
}

export function checkAssets(t) {
  const metadata = JSON.parse(docker("image", "inspect", image))[0].Config;
  assert.equal(metadata.User, "65532:65532");
  assert.equal(metadata.WorkingDir, "/app");
  assert.deepEqual(metadata.Entrypoint, ["/app/med-tracker"]);
  assert.deepEqual(metadata.Cmd, ["start", "--environment", "production"]);
  const container = docker("create", "--pull=never", image).trim();
  t.after(() => docker("rm", container));
  const directory = mkdtempSync(join(tmpdir(), "loco-image-assets-"));
  t.after(() => rmSync(directory, { recursive: true }));
  const archive = join(directory, "rootfs.tar");
  docker("export", "--output", archive, container);
  const files = new Set(execFileSync("tar", ["-tf", archive], { encoding: "utf8", maxBuffer: 32 * 1024 * 1024 }).trim().split("\n"));
  for (const path of ["app/config/production.yaml", "app/assets/static/css/compiled.css", "app/assets/static/js/html5-qrcode.min.js", "app/assets/reports/fonts/NotoSans-Regular.ttf", "etc/ssl/certs/ca-certificates.crt", "usr/share/zoneinfo/Europe/London"]) {
    assert.ok(files.has(path), `Missing runtime asset: ${path}`);
    assert.ok(execFileSync("tar", ["-xOf", archive, path], { maxBuffer: 32 * 1024 * 1024 }).length > 0, `Empty runtime asset: ${path}`);
  }
  assert.ok([...files].some((path) => path.startsWith("app/assets/views/")));
  for (const path of ["bin/sh", "usr/bin/sh", "bin/bash", "usr/bin/bash", "bin/dash", "usr/bin/dash", "bin/ash", "usr/bin/ash", "bin/busybox", "usr/bin/busybox", "usr/bin/cargo", "usr/bin/node", "usr/local/cargo/bin/cargo", "usr/local/bin/node", "root/.cargo/bin/cargo", "app/.env", "app/.git/config", "app/rails/config/ai_medication_sources.yml"]) {
    assert.ok(!files.has(path), `Unexpected runtime file: ${path}`);
  }
}

export async function checkProductionBoot(t) {
  const prefix = `loco-image-${randomUUID()}`;
  const database = `${prefix}-db`;
  const server = `${prefix}-server`;
  const worker = `${prefix}-worker`;
  const scheduler = `${prefix}-scheduler`;
  const containers = [];
  const subnet = process.env.LOCO_IMAGE_SUBNET;
  docker("network", "create", ...(subnet ? ["--subnet", subnet] : []), prefix);
  t.after(() => cleanup(containers, prefix));
  containers.push(database);
  docker("run", "--detach", "--name", database, "--network", prefix, "--tmpfs", "/var/lib/postgresql", "--env", "POSTGRES_HOST_AUTH_METHOD=trust", "--env", "POSTGRES_DB=medtracker_image", "postgres:18-alpine@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5");
  await eventually(() => {
    const result = spawnSync("docker", ["exec", database, "pg_isready", "-h", "127.0.0.1", "-U", "postgres"], { timeout: 10_000 });
    if (result.error) throw result.error;
    return result.status === 0;
  }, 60, "Disposable PostgreSQL did not become ready");
  const common = ["--network", prefix, "--read-only", "--cap-drop=ALL", "--security-opt=no-new-privileges=true", "--tmpfs", "/tmp:uid=65532,gid=65532", "--env", "MEDTRACKER_PUBLIC_HOST=https://medtracker.invalid", "--env", `MEDTRACKER_SESSION_KEY=${"BwcH".repeat(21)}Bw==`, "--env", "AUTH_SESSION_SECRET=synthetic-image-test-secret-at-least-32-characters"];
  docker("run", "--rm", ...common, "--env", `DATABASE_URL=postgres://postgres@${database}/medtracker_image`, image, "db", "migrate", "--environment", "production");
  docker("exec", database, "createuser", "-U", "postgres", "--login", "--inherit", "--no-superuser", "--no-createdb", "--no-createrole", "--member-of=med_tracker_app", "image_runtime");
  const runtime = [...common, "--env", `DATABASE_URL=postgres://image_runtime@${database}/medtracker_image`];
  docker("run", "--rm", ...runtime, image, "scheduler", "--list", "--environment", "production");
  const directory = mkdtempSync(join(tmpdir(), "loco-scheduler-"));
  t.after(() => rmSync(directory, { recursive: true }));
  const config = join(directory, "scheduler.yaml");
  writeFileSync(config, 'jobs:\n  expiry:\n    run: support-access-expire\n    schedule: "0 0 0 1 1 *"\n    run_on_start: true\n', { mode: 0o644 });
  containers.push(scheduler);
  docker("run", "--detach", "--name", scheduler, ...runtime, "--mount", `type=bind,src=${config},dst=/tmp/scheduler.yaml,readonly`, image, "scheduler", "--environment", "production", "--config", "/tmp/scheduler.yaml");
  await eventually(() => {
    const logs = docker("logs", scheduler);
    assert.doesNotMatch(logs, /failed to execute scheduler job/);
    return logs.includes("Support expiry run finished");
  }, 30, "Native scheduled task did not execute");
  docker("kill", "--signal=SIGINT", scheduler);
  assert.equal(docker("wait", scheduler).trim(), "0");
  writeFileSync(config, 'jobs:\n  explicit_shell:\n    run: echo synthetic\n    shell: true\n    schedule: "0 0 0 1 1 *"\n    run_on_start: true\n');
  const shellScheduler = `${scheduler}-explicit-shell`;
  containers.push(shellScheduler);
  docker("run", "--detach", "--name", shellScheduler, ...runtime, "--mount", `type=bind,src=${config},dst=/tmp/scheduler.yaml,readonly`, image, "scheduler", "--environment", "production", "--config", "/tmp/scheduler.yaml");
  await eventually(() => {
    const logs = docker("logs", shellScheduler);
    return logs.includes("failed to execute scheduler job") && logs.includes("NotFound");
  }, 30, "Explicit shell job did not report the missing shell");
  docker("kill", "--signal=SIGINT", shellScheduler);
  assert.equal(docker("wait", shellScheduler).trim(), "0");
  for (const [name, args] of [[server, []], [worker, ["start", "--worker", "--environment", "production"]]]) {
    containers.push(name);
    docker("run", "--detach", "--name", name, ...runtime, ...(name === server ? ["--publish", "127.0.0.1::5150"] : []), image, ...args);
  }
  const port = docker("port", server, "5150").trim().split(":").at(-1);
  const origin = `http://127.0.0.1:${port}`;
  await eventually(async () => {
    try {
      const response = await fetch(`${origin}/health`, { signal: AbortSignal.timeout(2000), redirect: "error" });
      await response.arrayBuffer();
      return response.status === 200;
    } catch (error) {
      if (!(error instanceof TypeError) && error.name !== "TimeoutError") throw error;
      return false;
    }
  }, 60, "Production server did not become healthy");
  const css = await fetch(`${origin}/static/css/compiled.css`, { signal: AbortSignal.timeout(5000) });
  assert.equal(css.status, 200);
  assert.ok((await css.arrayBuffer()).byteLength > 1000);
  const login = await fetch(`${origin}/login`, { signal: AbortSignal.timeout(5000) });
  assert.equal(login.status, 200);
  assert.match(await login.text(), /<form/);
  for (const name of [server, worker]) {
    assert.equal(docker("inspect", "--format", "{{.State.Running}}", name).trim(), "true", docker("logs", name));
  }
}
