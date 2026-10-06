import { test as base, expect } from '@playwright/test';
import { execFile, spawn } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'node:net';
import { fileURLToPath } from 'node:url';
import { withOwnedDatabase } from '../../scripts/migration/foundation-database.mjs';
import { stopOwnedProcess } from '../../scripts/migration/process-cleanup.mjs';

const execute = promisify(execFile);
const root = fileURLToPath(new URL('../../', import.meta.url));

export const test = base.extend({
  careFixture: [async ({}, use) => withOwnedDatabase(async (databaseUrl, provision) => {
    await provision();
    const owner = new URL(databaseUrl);
    owner.pathname = '/medtracker_reference';
    if (owner.hostname !== '127.0.0.1') throw Error('Care fixture must use the owned loopback database');
    await execute('task', ['db:migrate', `MIGRATION_DATABASE_URL=${owner.href}`, 'LOCO_ENV=test'], { cwd: root, timeout: 60000 });
    const fixtureTask = async name => (await execute('task', [`browser-care:${name}`, `CARE_DATABASE_URL=${owner.href}`], { cwd: root, timeout: 60000 })).stdout.trim();
    await fixtureTask('seed');
    const runtime = new URL(owner);
    runtime.username = 'medtracker_browser_runtime';
    runtime.password = 'password';
    const reservation = createServer();
    await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
    const port = reservation.address().port;
    await new Promise(resolve => reservation.close(resolve));
    const origin = `http://127.0.0.1:${port}`;
    let server;
    let output = '';
    let exited = false;
    const start = async () => {
      output = '';
      exited = false;
      server = spawn('task', ['dev'], { cwd: root, detached: true, env: { ...process.env, LOCO_ENV: 'test', PORT: String(port), DATABASE_URL: runtime.href, MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'), MEDTRACKER_COOKIE_SECURE: 'false', RAILS_SECRET_KEY_BASE: 'synthetic-rails-secret-key-base-for-compatibility', RAILS_OLD_SECRET_KEY_BASE: 'synthetic-old-rails-secret-key-base' }, stdio: ['ignore', 'pipe', 'pipe'] });
      server.stdout.on('data', chunk => { output += chunk; });
      server.stderr.on('data', chunk => { output += chunk; });
      server.on('exit', () => { exited = true; });
      server.on('error', error => { exited = true; output += error.message; });
      const deadline = Date.now() + 60000;
      let ready = false;
      while (Date.now() < deadline) {
        if (exited) throw Error(`Owned care server stopped: ${output}`);
        try { ready = (await fetch(`${origin}/up`, { signal: AbortSignal.timeout(1000) })).ok; } catch {}
        if (ready) break;
        await new Promise(resolve => setTimeout(resolve, 250));
      }
      if (!ready) throw Error(`Owned care server did not start: ${output}`);
    };
    try {
      await start();
      await use({ origin, probe: async () => JSON.parse(await fixtureTask('probe')), revoke: () => fixtureTask('revoke'), reactivate: () => fixtureTask('reactivate'),
        seedOtp: () => fixtureTask('seed-otp'), otpProbe: async () => JSON.parse(await fixtureTask('otp-probe')), closeOtpAccount: () => fixtureTask('close-otp-account'),
        secondPerson: () => fixtureTask('second-person'),
        scheduledMedicine: () => fixtureTask('scheduled-medicine'),
        oauthProbe: async () => JSON.parse(await fixtureTask('oauth-probe')), ageAuthentication: () => fixtureTask('age-authentication'),
        failOAuthAudit: () => fixtureTask('fail-oauth-audit'), restoreOAuthAudit: () => fixtureTask('restore-oauth-audit'),
        restart: async () => { await stopOwnedProcess(server); await start(); },
        revokeSession: () => fixtureTask('revoke-session'), expireSession: () => fixtureTask('expire-session'), doseRequestId: () => fixtureTask('dose-request-id') });
    } finally {
      await stopOwnedProcess(server);
    }
  }), { timeout: 180000 }],
  baseURL: async ({ careFixture }, use) => use(careFixture.origin)
});
export { expect };
