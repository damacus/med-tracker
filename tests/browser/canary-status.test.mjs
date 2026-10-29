import test from "node:test";
import assert from "node:assert/strict";
import fs from "node:fs";
import vm from "node:vm";

const source = fs.readFileSync(new URL("../../assets/static/js/canary.js", import.meta.url), "utf8");
const flush = () => new Promise(resolve => setImmediate(resolve));
function fixture({missing = false, failed = false, storage = new Map(), blocked = false} = {}) {
  const fields = Object.fromEntries(["last", "next", "countdown"].map(key => [key, {textContent: "Loading"}]));
  let now = Date.parse("2026-10-24T03:00:00Z");
  const polls = [];
  const requests = [];
  const dismiss = {addEventListener: (_event, callback) => { dismiss.click = callback; }};
  const notice = {hidden: false, querySelector: selector => selector === '[data-canary-dismiss]' ? dismiss : fields[selector.match(/canary-(.*)\]/)[1]]};
  class Clock extends Date { static now() { return now; } }
  const context = {
    document: {querySelector: () => missing ? null : notice, getElementById: () => null},
    sessionStorage: {
      getItem(key) { if (blocked) throw Error('blocked'); return storage.get(key); },
      setItem(key, value) { if (blocked) throw Error('blocked'); storage.set(key, value); },
    },
    Date: Clock, Intl, Number,
    window: {
      setInterval: (callback, interval) => { polls.push({callback, interval}); return polls.length; },
      clearInterval: id => { polls[id - 1].cleared = true; },
    },
    fetch: async (url, options) => {
      requests.push({url, options});
      return {ok: !failed, json: async () => ({phase: "ready", last_success: "2026-10-24T03:00:00Z", next_reset: "2026-10-25T04:00:00Z"})};
    },
  };
  vm.runInNewContext(source, context);
  return {fields, polls, requests, notice, dismiss, storage, advance: value => {now = Date.parse(value);}};
}

test('dismissal hides the warning and stops polling for the tab session', async () => {
  const f = fixture(); await flush();
  assert.equal(f.notice.hidden, false);
  f.dismiss.click();
  assert.equal(f.notice.hidden, true);
  assert.equal(f.polls[0].cleared, true);
  const reload = fixture({storage: f.storage}); await flush();
  assert.equal(reload.notice.hidden, true);
  assert.equal(reload.requests.length, 0);
  assert.equal(reload.polls.length, 0);
  assert.equal(fixture().notice.hidden, false);
});

test('storage restrictions still allow dismissal on the current page', async () => {
  const f = fixture({blocked: true}); await flush();
  f.dismiss.click();
  assert.equal(f.notice.hidden, true);
  assert.equal(f.polls[0].cleared, true);
  assert.equal(fixture({blocked: true}).notice.hidden, false);
});

test("reset dates use London across the autumn clock change and countdown uses exact instants", async () => {
  const f = fixture(); await flush();
  assert.match(f.fields.last.textContent, /04:00:00 BST.*Europe\/London/);
  assert.match(f.fields.next.textContent, /04:00:00 GMT.*Europe\/London/);
  assert.equal(f.fields.countdown.textContent, "1 day, 1 hour");
  assert.equal(f.requests[0].url, "/canary/status");
  assert.equal(f.requests[0].options.cache, "no-store");
  assert.equal(f.polls[0].interval, 30000);
  f.advance("2026-10-25T04:00:01Z"); await f.polls[0].callback();
  assert.equal(f.fields.countdown.textContent, "Reset due");
});
test("unavailable status clears stale reset details", async () => {
  const f = fixture({failed: true}); await flush();
  assert.equal(f.fields.last.textContent, "Unavailable");
  assert.equal(f.fields.next.textContent, "Unavailable");
  assert.equal(f.fields.countdown.textContent, "Reset details unavailable");
});
test("ordinary pages do not request canary status", async () => {
  const f = fixture({missing: true}); await flush();
  assert.equal(f.requests.length, 0);
  assert.equal(f.polls.length, 0);
});
