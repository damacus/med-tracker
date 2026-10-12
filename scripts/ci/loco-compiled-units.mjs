import { readFile } from 'node:fs/promises';
import { execute } from './loco-process.mjs';
import { isDatabaseFree } from './loco-inventory.mjs';

const manifest = JSON.parse(await readFile('target/ci/tests.json', 'utf8'));
for (const binary of manifest) {
  const listing = await execute(binary.path, ['--list', '--format=terse'], { capture: true });
  const names = listing.split('\n').filter(line => line.endsWith(': test')).map(line => line.slice(0, -6));
  if (!names.some(name => isDatabaseFree(binary.name, name))) continue;
  console.log(`Database-free contract unit tests: ${binary.name}`);
  const skipped = names.filter(name => !isDatabaseFree(binary.name, name));
  const env = { ...process.env };
  delete env.DATABASE_URL;
  delete env.MEDTRACKER_OWNED_DATABASE;
  await execute(binary.path, ['--exact', '--test-threads=2', ...skipped.flatMap(name => ['--skip', name])], { env });
}
