import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import test from 'node:test';
import { stopOwnedProcess } from './process-cleanup.mjs';
import { withOwnedDatabase } from './foundation-database.mjs';

test('the root Loco listener exposes health and an initialized Tera view', { timeout: 210000 }, async () => withOwnedDatabase(async databaseUrl => {
  const reservation = createServer();
  await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
  const port = reservation.address().port;
  await new Promise(resolve => reservation.close(resolve));
  const origin = `http://127.0.0.1:${port}`;
  const child = spawn('task', ['dev'], { detached: true, env: { ...process.env, LOCO_ENV: 'test', PORT: String(port), DATABASE_URL: databaseUrl }, stdio: ['ignore', 'pipe', 'pipe'] });
  let output = '';
  child.stdout.on('data', chunk => { output += chunk; });
  child.stderr.on('data', chunk => { output += chunk; });
  let exited = false;
  child.on('exit', () => { exited = true; });
  child.on('error', error => { exited = true; output += error.message; });
  let failed = false;
  try {
    let response;
    const startupDeadline = Date.now() + 60000;
    while (Date.now() < startupDeadline) {
      if (exited) assert.fail(`Loco failed to boot: ${output}`);
      try { response = await fetch(`${origin}/up`, { signal: AbortSignal.timeout(1000) }); } catch {}
      if (response) break;
      await new Promise(resolve => setTimeout(resolve, 500));
    }
    assert.ok(response, `No root listener: ${output}`);
    assert.ok(!exited, `Owned Loco process exited: ${output}`);
    assert.equal(response.status, 200);
    assert.deepEqual(await response.json(), { status: 'ok', application: 'med-tracker' });
    const readiness = await fetch(`${origin}/_health`, { signal: AbortSignal.timeout(5000) });
    assert.equal(readiness.status, 200);
    const page = await fetch(`${origin}/`, { signal: AbortSignal.timeout(5000) });
    assert.equal(page.status, 200);
    assert.match(page.headers.get('content-type'), /text\/html/);
    assert.match(await page.text(), /MedTracker migration foundation/);
    const unknown = await fetch(`${origin}/api/v1/medications`, { signal: AbortSignal.timeout(5000) });
    assert.equal(unknown.status, 404);
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    try { await stopOwnedProcess(child); } catch (error) {
      if (!failed) throw error;
      process.stderr.write(`Owned process cleanup failed: ${error.message}\n`);
    }
  }
}));
