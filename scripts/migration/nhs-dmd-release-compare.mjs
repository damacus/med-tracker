import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';
import { runReferenceImport } from './nhs-dmd-reference-bootstrap.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const referenceScript = fileURLToPath(new URL('./nhs-dmd-reference-snapshot.rb', import.meta.url));
const release = process.env.NHS_DMD_RELEASE_PATH;
const expectedChecksum = '321db83dc17467430183e79ffdd98f1b0a1794c0d20f828d127682bc3f0edca6';

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

function differences(rails, loco) {
  const counts = ['status', 'created', 'updated', 'unchanged', 'skipped_expired', 'skipped_missing_name', 'skipped_invalid', 'trade_families', 'trade_family_groups', 'trade_memberships'];
  const countDifferences = Object.fromEntries(counts.filter(key => rails[key] !== loco[key]).map(key => [key, { rails: rails[key], loco: loco[key] }]));
  const rowDifferences = {};
  for (const key of ['barcodes', 'relationships']) {
    const reference = new Map(rails[key].map(row => [row[0], row]));
    const candidate = new Map(loco[key].map(row => [row[0], row]));
    const keys = [...new Set([...reference.keys(), ...candidate.keys()])].sort();
    const mismatches = keys.filter(code => JSON.stringify(reference.get(code)) !== JSON.stringify(candidate.get(code)));
    rowDifferences[key] = { rails: rails[key].length, loco: loco[key].length, mismatches: mismatches.length, examples: mismatches.slice(0, 10).map(code => ({ code, rails: reference.get(code) ?? null, loco: candidate.get(code) ?? null })) };
  }
  return { countDifferences, rowDifferences };
}

if (!release) throw new Error('NHS_DMD_RELEASE_PATH is required');
const checksum = createHash('sha256').update(await readFile(release)).digest('hex');
if (checksum !== expectedChecksum) throw new Error(`Release checksum mismatch: ${checksum}`);
const output = await mkdtemp(join(tmpdir(), 'nhs-dmd-compare-'));
const railsPath = join(output, 'rails.json');
const locoPath = join(output, 'loco.json');
try {
  await withOwnedDatabase(async (url, provision, ownership) => {
    const image = process.env.RAILS_ROLLBACK_IMAGE ?? 'med-tracker-web-test';
    await runReferenceImport({ url, provision, runCommand: run, environment: process.env, railsArguments: [
      'run', '--rm', '--pull', 'never', '--network', `${ownership.project}_default`,
      '--env', 'RAILS_ENV=test', '--env', 'MEDTRACKER_OWNED_DATABASE=1',
      '--env', 'DATABASE_URL=postgresql://medtracker:medtracker_password@postgres:5432/medtracker_reference',
      '--env', 'NHS_DMD_RELEASE_PATH=/release.zip', '--env', 'NHS_DMD_COMPARISON_OUTPUT=/compare/rails.json',
      '--mount', `type=bind,source=${release},target=/release.zip,readonly`,
      '--mount', `type=bind,source=${referenceScript},target=/tmp/nhs-dmd-reference-snapshot.rb,readonly`,
      '--mount', `type=bind,source=${output},target=/compare`,
      image, 'bundle', 'exec', 'rails', 'runner', '/tmp/nhs-dmd-reference-snapshot.rb',
    ] });
    const storageEnvironment = await ownership.storage();
    await run('cargo', ['test', '--locked', '--test', 'care_api', 'nhs_dmd_official_release_comparison_snapshot', '--', '--ignored', '--nocapture'], {
      ...process.env,
      ...storageEnvironment,
      DATABASE_URL: url,
      MEDTRACKER_OWNED_DATABASE: '1',
      MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'),
      RAILS_SECRET_KEY_BASE: 'synthetic-slice-rails-verification-secret',
      NHS_DMD_COMPARISON_OUTPUT: locoPath,
    });
  }, undefined, process.env, { storage: true });
  const rails = JSON.parse(await readFile(railsPath, 'utf8'));
  const loco = JSON.parse(await readFile(locoPath, 'utf8'));
  const comparison = { releaseChecksum: checksum, ...differences(rails, loco) };
  await writeFile(join(output, 'comparison.json'), JSON.stringify(comparison, null, 2));
  process.stdout.write(`${JSON.stringify({ output, ...comparison }, null, 2)}\n`);
  if (Object.keys(comparison.countDifferences).length || Object.values(comparison.rowDifferences).some(value => value.mismatches)) process.exitCode = 1;
} catch (error) {
  process.stderr.write(`Comparison snapshots retained at ${output}\n`);
  throw error;
}
