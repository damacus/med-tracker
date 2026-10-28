import assert from "node:assert/strict";
import { execFile } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { randomUUID } from "node:crypto";
import { promisify } from "node:util";
import { setTimeout } from "node:timers/promises";
import test from "node:test";

const execute = promisify(execFile);
const image = process.env.LOCO_IMAGE || "medtracker-loco:local";

async function docker(...args) {
  const result = await execute("docker", args, { timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
  return args[0] === "logs" ? result.stdout + result.stderr : result.stdout;
}

test("production boot with restricted database identity", async (t) => {
  const prefix = `loco-image-${randomUUID()}`;
  const database = `${prefix}-db`;
  const server = `${prefix}-server`;
  const worker = `${prefix}-worker`;
  const scheduler = `${prefix}-scheduler`;
  const containers = [];
  const subnet = process.env.LOCO_IMAGE_SUBNET;
  await docker("network", "create", ...(subnet ? ["--subnet", subnet] : []), prefix);
  t.after(async () => {
    const results = await Promise.allSettled(containers.map((name) => docker("rm", "--force", "--volumes", name)));
    await docker("network", "rm", prefix);
    for (const result of results) if (result.status === "rejected") throw result.reason;
  });
  await docker("run", "--detach", "--name", database, "--network", prefix,
    "--tmpfs", "/var/lib/postgresql", "--env", "POSTGRES_HOST_AUTH_METHOD=trust",
    "--env", "POSTGRES_DB=medtracker_image",
    "postgres:18-alpine@sha256:898c60cb390d1085bc97bdd6a1ae968757421df394b071848b52b1c09c89d6a5");
  containers.push(database);
  let ready = false;
  for (let attempt = 0; attempt < 60; attempt++) {
    try {
      await execute("docker", ["exec", database, "pg_isready", "-h", "127.0.0.1", "-U", "postgres"], { timeout: 10_000 });
      ready = true;
      break;
    } catch {
      await setTimeout(1000);
    }
  }
  assert.ok(ready, "Disposable PostgreSQL did not become ready");
  const common = ["--network", prefix, "--read-only", "--cap-drop=ALL",
    "--security-opt=no-new-privileges=true", "--tmpfs", "/tmp:uid=65532,gid=65532",
    "--env", "MEDTRACKER_PUBLIC_HOST=https://medtracker.invalid",
    "--env", `MEDTRACKER_SESSION_KEY=${"BwcH".repeat(21)}Bw==`,
    "--env", "AUTH_SESSION_SECRET=synthetic-image-test-secret-at-least-32-characters"];
  await docker("run", "--rm", ...common, "--env", `DATABASE_URL=postgres://postgres@${database}/medtracker_image`,
    image, "db", "migrate", "--environment", "production");
  await docker("exec", database, "createuser", "-U", "postgres", "--login", "--inherit",
    "--no-superuser", "--no-createdb", "--no-createrole", "--member-of=med_tracker_app", "image_runtime");
  const runtime = [...common, "--env", `DATABASE_URL=postgres://image_runtime@${database}/medtracker_image`];
  await docker("run", "--rm", ...runtime, image, "scheduler", "--list", "--environment", "production");
  const directory = await mkdtemp(join(tmpdir(), "loco-scheduler-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const config = join(directory, "scheduler.yaml");
  await writeFile(config, 'jobs:\n  expiry:\n    run: support-access-expire\n    schedule: "0 0 0 1 1 *"\n    run_on_start: true\n');
  await docker("run", "--detach", "--name", scheduler, ...runtime,
    "--mount", `type=bind,src=${config},dst=/tmp/scheduler.yaml,readonly`,
    image, "scheduler", "--environment", "production", "--config", "/tmp/scheduler.yaml");
  containers.push(scheduler);
  let logs = "";
  let ran = false;
  for (let attempt = 0; attempt < 30; attempt++) {
    logs = await docker("logs", scheduler);
    assert.ok(!logs.includes("failed to execute scheduler job"), logs);
    if (logs.includes("Support expiry run finished")) { ran = true; break; }
    await setTimeout(1000);
  }
  assert.ok(ran, `Native scheduled task did not execute: ${logs}`);
  await docker("kill", "--signal=SIGINT", scheduler);
  assert.equal((await docker("wait", scheduler)).trim(), "0");
  await writeFile(config, 'jobs:\n  explicit_shell:\n    run: echo synthetic\n    shell: true\n    schedule: "0 0 0 1 1 *"\n    run_on_start: true\n');
  const shellScheduler = `${scheduler}-explicit-shell`;
  await docker("run", "--detach", "--name", shellScheduler, ...runtime,
    "--mount", `type=bind,src=${config},dst=/tmp/scheduler.yaml,readonly`,
    image, "scheduler", "--environment", "production", "--config", "/tmp/scheduler.yaml");
  containers.push(shellScheduler);
  let missingShell = false;
  for (let attempt = 0; attempt < 30; attempt++) {
    logs = await docker("logs", shellScheduler);
    if (logs.includes("failed to execute scheduler job") && logs.includes("NotFound")) { missingShell = true; break; }
    await setTimeout(1000);
  }
  assert.ok(missingShell, `Explicit shell job did not report the missing shell: ${logs}`);
  await docker("kill", "--signal=SIGINT", shellScheduler);
  assert.equal((await docker("wait", shellScheduler)).trim(), "0");
  for (const [name, args] of [[server, []], [worker, ["start", "--worker", "--environment", "production"]]]) {
    await docker("run", "--detach", "--name", name, ...runtime,
      ...(name === server ? ["--publish", "127.0.0.1::5150"] : []), image, ...args);
    containers.push(name);
  }
  const port = (await docker("port", server, "5150")).trim().split(":").at(-1);
  const origin = `http://127.0.0.1:${port}`;
  let healthy = false;
  for (let attempt = 0; attempt < 60; attempt++) {
    try {
      const response = await fetch(`${origin}/health`, { signal: AbortSignal.timeout(2000) });
      await response.body?.cancel();
      assert.equal(response.status, 200);
      healthy = true;
      break;
    } catch {
      await setTimeout(1000);
    }
  }
  assert.ok(healthy, `Production server did not become healthy: ${await docker("logs", server)}`);
  const css = await fetch(`${origin}/static/css/compiled.css`, { signal: AbortSignal.timeout(5000) });
  assert.equal(css.status, 200);
  assert.ok((await css.arrayBuffer()).byteLength > 1000);
  const login = await fetch(`${origin}/login`, { signal: AbortSignal.timeout(5000) });
  assert.equal(login.status, 200);
  assert.match(await login.text(), /<form/);
  for (const name of [server, worker]) {
    assert.equal((await docker("inspect", "--format", "{{.State.Running}}", name)).trim(), "true", await docker("logs", name));
  }
});

test("commands load shared libraries", async () => {
  for (const args of [["--help"], ["start", "--help"], ["db", "--help"], ["scheduler", "--help"]]) {
    const output = await docker("run", "--rm", "--pull=never", "--network=none", "--read-only",
      "--cap-drop=ALL", "--security-opt=no-new-privileges=true", image, ...args);
    assert.match(output, /Usage:/);
  }
});

test("unprivileged runtime and assets", async (t) => {
  const metadata = JSON.parse(await docker("image", "inspect", image))[0].Config;
  assert.equal(metadata.User, "65532:65532");
  assert.equal(metadata.WorkingDir, "/app");
  assert.deepEqual(metadata.Entrypoint, ["/app/med-tracker"]);
  assert.deepEqual(metadata.Cmd, ["start", "--environment", "production"]);
  const container = (await docker("create", "--pull=never", image)).trim();
  t.after(() => docker("rm", container));
  const directory = await mkdtemp(join(tmpdir(), "loco-image-assets-"));
  t.after(() => rm(directory, { recursive: true, force: true }));
  const archive = join(directory, "image.tar");
  await docker("export", "--output", archive, container);
  const listing = await execute("tar", ["-tf", archive], { timeout: 180_000, maxBuffer: 16 * 1024 * 1024 });
  const files = listing.stdout.trim().split("\n");
  for (const path of ["app/config/production.yaml", "app/assets/static/css/compiled.css",
    "app/assets/static/js/html5-qrcode.min.js", "app/assets/reports/fonts/NotoSans-Regular.ttf",
    "etc/ssl/certs/ca-certificates.crt", "usr/share/zoneinfo/Europe/London"]) {
    assert.ok(files.includes(path), `Missing image asset: ${path}`);
    const file = await execute("tar", ["-xOf", archive, path], { timeout: 180_000, encoding: "buffer", maxBuffer: 16 * 1024 * 1024 });
    assert.ok(file.stdout.length > 0, `Empty image asset: ${path}`);
  }
  assert.ok(files.some((path) => path.startsWith("app/assets/views/")));
  for (const path of ["bin/sh", "usr/bin/sh", "bin/bash", "usr/bin/bash", "bin/dash", "usr/bin/dash",
    "bin/ash", "usr/bin/ash", "bin/busybox", "usr/bin/busybox", "usr/bin/cargo", "usr/bin/node",
    "usr/local/cargo/bin/cargo", "usr/local/bin/node", "root/.cargo/bin/cargo", "app/.env", "app/.git/config",
    "app/rails/config/ai_medication_sources.yml"]) {
    assert.ok(!files.includes(path), `Forbidden file in release image: ${path}`);
  }
});
