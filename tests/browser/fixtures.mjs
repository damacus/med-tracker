import { test as base, expect } from '@playwright/test';
import { spawn } from 'node:child_process';
import { createServer } from 'node:net';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';
import { sharedBrowserDatabase } from './browser-database.mjs';

export const test = base.extend({
  ownedOrigin: [async ({ }, use) => {
    const database = process.env.LOCO_BROWSER_DATABASE ? await sharedBrowserDatabase(process.env.LOCO_BROWSER_DATABASE) : withOwnedDatabase;
    return database(async (databaseUrl, _provision, ownership) => {
      if (ownership.runtimeRole) {
        const runtime = new URL(databaseUrl);
        runtime.username = ownership.runtimeRole;
        runtime.password = 'password';
        databaseUrl = runtime.href;
      }
      const reservation = createServer();
      await new Promise(resolve => reservation.listen(0, '127.0.0.1', resolve));
      const port = reservation.address().port;
      await new Promise(resolve => reservation.close(resolve));
      const origin = `http://127.0.0.1:${port}`;
      const server = spawn('task', ['browser-care:serve'], { detached: true, env: { ...process.env, LOCO_ENV: 'test', PORT: String(port), DATABASE_URL: databaseUrl }, stdio: ['ignore', 'pipe', 'pipe'] });
      let output = '';
      let exited = false;
      server.stdout.on('data', chunk => { output += chunk; });
      server.stderr.on('data', chunk => { output += chunk; });
      server.on('exit', () => { exited = true; });
      server.on('error', error => { exited = true; output += error.message; });
      try {
        const deadline = Date.now() + 60000;
        let ready = false;
        while (Date.now() < deadline) {
          if (exited) throw Error(`Owned Loco server stopped: ${output}`);
          try { ready = (await fetch(`${origin}/up`, { signal: AbortSignal.timeout(1000) })).ok; } catch { }
          if (ready) break;
          await new Promise(resolve => setTimeout(resolve, 250));
        }
        if (!ready) throw Error(`Owned Loco server did not start: ${output}`);
        await use(origin);
      } finally {
        await stopOwnedProcess(server);
      }
    });
  }, { scope: 'worker', timeout: 120000 }],
  baseURL: async ({ ownedOrigin }, use) => use(ownedOrigin)
});
export { expect };
