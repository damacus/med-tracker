import { readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { withOwnedDatabase } from '../migration/foundation-database.mjs';
import { execute } from './loco-process.mjs';
import { parseShard, partition } from './loco-shards.mjs';
import { isDatabaseFree, isDedicatedTemplateTest } from './loco-inventory.mjs';

const { index, total } = parseShard(process.argv[2]);
const manifest = JSON.parse(await readFile('target/ci/tests.json', 'utf8'));
const names = new Map();
for (const binary of manifest) {
  const listing = await execute(binary.path, ['--list', '--format=terse'], { capture: true });
  names.set(binary.name, listing.split('\n').filter(line => line.endsWith(': test')).map(line => line.slice(0, -6)));
}
const identities = [...names].flatMap(([binary, tests]) => tests.filter(name => !isDatabaseFree(binary, name) && !isDedicatedTemplateTest(binary, name)).map(name => `${binary}/${name}`));
const selected = new Set(partition(identities, total)[index]);
if (!selected.size) throw new Error('Integration shard selected no tests');
await writeFile(`test-results/integration-selection-${index + 1}.json`, JSON.stringify([...selected]));
await withOwnedDatabase(async (url, provision, ownership) => {
  await provision();
  const storage = [...selected].some(name => name.startsWith('care_api/')) ? await ownership.storage() : {};
  const env = { ...process.env, ...ownership.storageEnvironment, ...storage, LOCO_TEST_APPLICATION: resolve('target/ci/med-tracker'), DATABASE_URL: url, MEDTRACKER_OWNED_DATABASE: '1', MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'), RAILS_SECRET_KEY_BASE: 'synthetic-slice-rails-verification-secret' };
  for (const name of ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']) delete env[name];
  for (const binary of manifest) {
    const tests = names.get(binary.name);
    if (!tests.some(name => selected.has(`${binary.name}/${name}`))) continue;
    const skipped = tests.filter(name => !selected.has(`${binary.name}/${name}`));
    console.log(`API/security/database integration: ${binary.name} (shard ${index + 1}/${total})`);
    await execute(binary.path, ['--exact', '--test-threads=2', ...skipped.flatMap(name => ['--skip', name])], { env });
  }
}, undefined, process.env, { storage: [...selected].some(name => name.startsWith('care_api/')) });
