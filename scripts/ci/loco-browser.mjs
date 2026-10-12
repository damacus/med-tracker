import { pathToFileURL } from 'node:url';
import { withOwnedDatabase } from '../migration/foundation-database.mjs';
import { browserDatabasePool } from './browser-database.mjs';
import { execute } from './loco-process.mjs';

export async function runBrowser(args, own = withOwnedDatabase, run = execute) {
  console.log(`Desktop/mobile browser journeys (shard ${process.env.LOCO_BROWSER_SHARD ?? '1/1'})`);
  const arguments_ = ['playwright', 'test', '--config', 'playwright.config.mjs', ...args];
  const listing = JSON.parse(await run('npx', [...arguments_, '--list', '--reporter=json'], { capture: true }));
  const suites = JSON.stringify(listing.suites);
  const avatar = suites.includes('profile-avatar.spec.mjs');
  const scanner = suites.includes('care-scanner.spec.mjs');
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
