import assert from "node:assert/strict";
import { chmodSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { spawnSync } from "node:child_process";
import test from "node:test";

test("local SMTP acceptance cleans only its own project and preserves failures", () => {
  const directory = mkdtempSync(join(tmpdir(), "invitation-mail-task-"));
  const log = join(directory, "calls.jsonl");
  const stub = `#!/usr/bin/env node
const fs = require("node:fs");
const args = process.argv.slice(2);
fs.appendFileSync(process.env.MAIL_TEST_LOG, JSON.stringify({args, database:process.env.DATABASE_URL, smtp:process.env.MEDTRACKER_SMTP_PORT, capture:process.env.MEDTRACKER_MAILPIT_URL, owned:process.env.MEDTRACKER_OWNED_DATABASE})+"\\n");
if (args.includes("up") && process.env.MAIL_TEST_FAILURE === "up") process.exit(41);
if (args.includes("port")) {
  if (process.env.MAIL_TEST_FAILURE === "port" && args.at(-1) === "1025") { console.log(":0"); process.exit(0); }
  const port = {"5432":19001,"9000":19002,"1025":19003,"8025":19004}[args.at(-1)];
  console.log("127.0.0.1:"+port);
}
if (args.includes("host:test:browser") && process.env.MAIL_TEST_FAILURE === "test") process.exit(42);
`;
  for (const command of ["docker", "task"]) {
    const executable = join(directory, command);
    writeFileSync(executable, stub);
    chmodSync(executable, 0o755);
  }
  try {
    for (const failure of ["none", "up", "port", "test"]) {
      writeFileSync(log, "");
      const result = spawnSync("fish", ["--no-config", "scripts/invitation_mail.fish"], {
        encoding: "utf8",
        env: { ...process.env, PATH: `${directory}:${process.env.PATH}`, MAIL_TEST_LOG: log, MAIL_TEST_FAILURE: failure },
      });
      assert.equal(result.status, { none: 0, up: 41, port: 2, test: 42 }[failure], result.stderr);
      const calls = readFileSync(log, "utf8").trim().split("\n").map(JSON.parse);
      const compose = calls.filter(call => call.args.includes("compose"));
      const projects = new Set(compose.map(call => call.args[call.args.indexOf("-p") + 1]));
      assert.equal(projects.size, 1);
      assert.match([...projects][0], /^mtloco-mail-[0-9a-f-]+$/);
      assert.ok(compose.at(-1).args.includes("down"));
      assert.ok(compose.at(-1).args.includes("--volumes"));
      const acceptance = calls.find(call => call.args.includes("host:test:browser"));
      if (failure === "up" || failure === "port") {
        assert.equal(acceptance, undefined);
      } else {
        assert.ok(acceptance.args.includes("TARGET=invitation_mail"));
        assert.equal(acceptance.database, "postgres://medtracker:medtracker_password@127.0.0.1:19001/medtracker_loco");
        assert.equal(acceptance.smtp, "19003");
        assert.equal(acceptance.capture, "http://127.0.0.1:19004");
        assert.equal(acceptance.owned, "1");
      }
    }
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test("Docker development passes local SMTP settings and uses Mailpit's container port", () => {
  for (const capture of ["false", "true"]) {
    const result = spawnSync("docker", ["compose", "--env-file", "/dev/null", "-p", "tooling-mail", "-f", "compose.yaml", "-f", "compose.local.yaml", "--profile", "dev", "--profile", "local-mail", "config", "--format", "json"], {
      encoding: "utf8",
      env: { ...process.env, MT_PROJECT: "tooling-mail", MT_TOOL_TARGET: "development", MT_TARGET_CACHE: "test-target", MT_CARGO_REGISTRY_CACHE: "test-registry", MT_CARGO_GIT_CACHE: "test-git", MT_BROWSER_CACHE: "test-browser", MEDTRACKER_CAPTURE_MAIL: capture, MEDTRACKER_SMTP_PORT: "31025", MEDTRACKER_INVITATION_MAIL_FROM: "Synthetic <invitation@example.test>" },
    });
    assert.equal(result.status, 0, result.stderr);
    const config = JSON.parse(result.stdout);
    const app = config.services["dev-app"].environment;
    assert.equal(app.MEDTRACKER_CAPTURE_MAIL, capture);
    assert.equal(app.MEDTRACKER_SMTP_HOST, "mailpit");
    assert.equal(app.MEDTRACKER_SMTP_PORT, "31025");
    assert.equal(app.MEDTRACKER_INVITATION_MAIL_FROM, "Synthetic <invitation@example.test>");
    assert.equal(config.services.mailpit.environment.MP_SMTP_BIND_ADDR, "0.0.0.0:31025");
    assert.ok(config.services.mailpit.ports.some(port => port.target === 31025 && port.host_ip === "127.0.0.1"));
  }
});

test("Docker development starts capture before the app only when opted in", () => {
  for (const capture of ["false", "true"]) {
    const result = spawnSync("task", ["--dry", "--verbose", "dev:up"], {
      encoding: "utf8",
      env: { ...process.env, PORTLESS: "0", PORT: "15150", MEDTRACKER_CAPTURE_MAIL: capture },
    });
    assert.equal(result.status, 0, result.stderr);
    const commands = result.stdout + result.stderr;
    const mail = commands.indexOf("--profile local-mail up -d --wait mailpit");
    if (capture === "true") {
      assert.notEqual(mail, -1, "opt-in starts Mailpit");
      assert.ok(mail < commands.indexOf("up -d dev-frontend dev-builder dev-app dev-scheduler"));
    } else {
      assert.equal(mail, -1, "default startup does not start Mailpit");
    }
  }
});

test("Docker development shutdown includes capture after the opt-in is removed", () => {
  const result = spawnSync("task", ["--dry", "--verbose", "dev:down"], {
    encoding: "utf8",
    env: { ...process.env, PORTLESS: "0", MEDTRACKER_CAPTURE_MAIL: "false" },
  });
  assert.equal(result.status, 0, result.stderr);
  assert.match(result.stdout + result.stderr, /--profile local-mail down/);
});
