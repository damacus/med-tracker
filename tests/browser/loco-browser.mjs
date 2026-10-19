import { pathToFileURL } from 'node:url';
import { withOwnedDatabase } from './foundation-database.mjs';
import { browserDatabasePool, execute } from './browser-database.mjs';

export async function runBrowser(args, own = withOwnedDatabase, run = execute) {
  const shard = process.env.LOCO_BROWSER_SHARD && process.env.LOCO_BROWSER_SHARD !== '1/1' ? [`--shard=${process.env.LOCO_BROWSER_SHARD}`] : [];
  console.log(`Desktop/mobile browser journeys (shard ${process.env.LOCO_BROWSER_SHARD ?? '1/1'})`);
  const arguments_ = ['playwright', 'test', '--config', 'playwright.config.mjs', ...shard, ...args];
  const listing = JSON.parse(await run('npx', [...arguments_, '--list', '--reporter=json'], { capture: true }));
  const files = [];
  let specs = 0;
  const collect = suites => {
    for (const suite of suites ?? []) {
      for (const spec of suite.specs ?? []) {
        specs += 1;
        const file = spec.file ?? spec.location?.file;
        if (file) files.push(file);
      }
      collect(suite.suites);
    }
  };
  collect(listing.suites);
  if (specs === 0) {
    console.log('No browser tests selected; skipping.');
    return;
  }
  const marker = files.length > 0 ? files.join('\n') : JSON.stringify(listing.suites);
  const avatar = marker.includes('profile-avatar.spec.mjs');
  const scanner = marker.includes('care-scanner.spec.mjs') || marker.includes('nhs-dmd-import.spec.mjs');
  await own(async (url, provision, ownership) => {
    await browserDatabasePool(url, provision, ownership, (name, variables) => run('task', [name, ...variables], { capture: true }));
    const scannerEnvironment = scanner ? await ownership.storage() : {};
    const descriptor = { url, project: ownership.project, templatePrepared: true, storageEnvironment: ownership.storageEnvironment, scannerEnvironment };
    const env = { ...process.env, LOCO_BROWSER_DATABASE: JSON.stringify(descriptor) };
    for (const name of ['DATABASE_URL', 'COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']) delete env[name];
    await run('npx', arguments_, { env });
  }, undefined, process.env, { storage: avatar });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  await runBrowser(process.argv.slice(2));
}
