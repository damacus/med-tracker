import assert from "node:assert/strict";
import test from "node:test";
import { cleanup, eventually } from "./loco_image_smoke.mjs";

test("readiness polling stops when the service is ready", async () => {
  let attempts = 0;
  await eventually(() => ++attempts === 2, 10, "not ready", async () => {});
  assert.equal(attempts, 2);
});

test("a service that never becomes ready fails the smoke check", async () => {
  let attempts = 0;
  await assert.rejects(eventually(() => { attempts += 1; return false; }, 3, "not ready", async () => {}), /not ready/);
  assert.equal(attempts, 3);
});

test("unexpected readiness errors fail immediately", async () => {
  await assert.rejects(eventually(() => { throw new Error("broken check"); }, 3, "not ready"), /broken check/);
});

test("cleanup attempts every container and the network after a removal failure", () => {
  const calls = [];
  assert.throws(() => cleanup(["database", "server", "worker"], "private-network", (...args) => {
    calls.push(args);
    if (args.at(-1) === "worker") throw new Error("cannot remove worker");
  }), AggregateError);
  assert.deepEqual(calls.map((args) => args.at(-1)), ["worker", "server", "database", "private-network"]);
});
