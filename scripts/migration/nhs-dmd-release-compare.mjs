import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';
import { compareSnapshots, comparisonInput, runReferenceImport } from './nhs-dmd-reference-bootstrap.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const referenceScript = fileURLToPath(new URL('./nhs-dmd-reference-snapshot.rb', import.meta.url));
const mode = process.env.NHS_DMD_COMPARISON_MODE ?? 'official';
const { release, expectedChecksum } = comparisonInput(mode, process.env.NHS_DMD_RELEASE_PATH, root);

async function run(command, args, environment, timeout = 1800000) {
  const child = spawn(command, args, { cwd: root, env: environment, detached: true, stdio: 'inherit' });
  let timer;
  try {
    await new Promise((resolve, reject) => {
      timer = setTimeout(() => reject(new Error(`${command} timed out`)), timeout);
      child.on('error', reject);
      child.on('close', code => code === 0 ? resolve() : reject(new Error(`${command} exited ${code}`)));
    });
  } finally {
    clearTimeout(timer);
    await stopOwnedProcess(child);
  }
}

const checksum = createHash('sha256').update(await readFile(release)).digest('hex');
if (checksum !== expectedChecksum) throw new Error(`Release checksum mismatch: ${checksum}`);
const output = await mkdtemp(join(tmpdir(), 'nhs-dmd-compare-'));
const railsPath = join(output, 'rails.json');
const locoPath = join(output, 'loco.json');
try {
  await withOwnedDatabase(async (url, provision, ownership) => {
    const image = process.env.RAILS_ROLLBACK_IMAGE ?? 'med-tracker-web-test';
    const storageEnvironment = await ownership.storage();
    const locoEnvironment = {
      ...process.env,
      ...storageEnvironment,
      DATABASE_URL: url,
      MEDTRACKER_OWNED_DATABASE: '1',
      MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'),
      RAILS_SECRET_KEY_BASE: 'synthetic-slice-rails-verification-secret',
      NHS_DMD_RELEASE_PATH: release,
      NHS_DMD_COMPARISON_OUTPUT: locoPath,
    };
    await runReferenceImport({ url, provision, runCommand: run, environment: process.env, locoEnvironment,
      locoArguments: ['test', '--locked', '--test', 'care_api', 'nhs_dmd_official_release_comparison_snapshot', '--', '--ignored', '--nocapture'],
      railsArguments: [
      'run', '--rm', '--pull', 'never', '--network', `${ownership.project}_default`,
      '--env', 'RAILS_ENV=test', '--env', 'MEDTRACKER_OWNED_DATABASE=1',
      '--env', 'DATABASE_URL=postgresql://medtracker:medtracker_password@postgres:5432/medtracker_reference',
      '--env', 'NHS_DMD_RELEASE_PATH=/release.zip', '--env', 'NHS_DMD_COMPARISON_OUTPUT=/compare/rails.json',
      '--mount', `type=bind,source=${release},target=/release.zip,readonly`,
      '--mount', `type=bind,source=${referenceScript},target=/tmp/nhs-dmd-reference-snapshot.rb,readonly`,
      '--mount', `type=bind,source=${output},target=/compare`,
      image, 'bundle', 'exec', 'rails', 'runner', '/tmp/nhs-dmd-reference-snapshot.rb',
    ] });
  }, undefined, process.env, { storage: true });
  const rails = JSON.parse(await readFile(railsPath, 'utf8'));
  const loco = JSON.parse(await readFile(locoPath, 'utf8'));
  const comparison = { mode, releaseChecksum: checksum, ...compareSnapshots(rails, loco) };
  await writeFile(join(output, 'comparison.json'), JSON.stringify(comparison, null, 2));
  process.stdout.write(`${JSON.stringify({ output, ...comparison }, null, 2)}\n`);
  if (Object.keys(comparison.countDifferences).length || Object.values(comparison.rowDifferences).some(value => value.mismatches)) process.exitCode = 1;
} catch (error) {
  process.stderr.write(`Comparison snapshots retained at ${output}\n`);
  throw error;
}
