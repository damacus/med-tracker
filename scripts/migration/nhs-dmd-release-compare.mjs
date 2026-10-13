import { execFileSync, spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';
import { compareSnapshots, comparisonInput, runReferenceImport } from './nhs-dmd-reference-bootstrap.mjs';
import { BenchmarkMonitor } from './nhs-dmd-benchmark-monitor.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));
const referenceScript = fileURLToPath(new URL('./nhs-dmd-reference-snapshot.rb', import.meta.url));
const mode = process.env.NHS_DMD_COMPARISON_MODE ?? 'official';
const benchmark = process.env.NHS_DMD_BENCHMARK === '1';
const { release, expectedChecksum } = comparisonInput(mode, process.env.NHS_DMD_RELEASE_PATH, root);

async function runProcess(command, args, environment, timeout = 1800000) {
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
process.stdout.write(`Comparison evidence: ${output}\n`);
const monitor = benchmark ? new BenchmarkMonitor(output) : null;
async function run(command, args, environment) {
  const stage = command === 'docker' ? 'rails' : command === 'task' ? 'setup' : 'loco';
  const operation = () => runProcess(command, args, environment);
  return monitor ? monitor.measure(stage, operation, { quiet: stage !== 'setup' }) : operation();
}
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
      ...(benchmark ? { NHS_DMD_TIMING_OUTPUT: join(output, 'loco-timing.json') } : {}),
    };
    if (benchmark) {
      const binaries = {};
      for (const name of ['care_api', 'med-tracker']) {
        binaries[name] = createHash('sha256').update(await readFile(join(root, 'target/ci', name))).digest('hex');
      }
      await writeFile(join(output, 'benchmark-input.json'), JSON.stringify({
        observedAt: new Date().toISOString(), project: ownership.project,
        mode, releaseChecksum: checksum, railsImage: image, binaries,
        sourceHead: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
        sourceDiffChecksum: createHash('sha256').update(execFileSync('git', ['diff', 'HEAD'], { cwd: root })).digest('hex'),
        locoProfile: 'dev/test (unoptimised)', order: ['loco', 'rails'],
        quietGate: { minimumSeconds: 60, minimumIdlePercent: 80, maximumOneMinuteLoad: 'logical CPU count', memoryPressure: 'normal', freshSwapouts: 0 },
      }, null, 2));
    }
    await runReferenceImport({ url, provision, runCommand: run,
      environment: benchmark ? { ...process.env, MIGRATION_EXECUTABLE: join(root, 'target/ci/med-tracker') } : process.env,
      locoEnvironment,
      ...(benchmark ? { locoCommand: join(root, 'target/ci/care_api') } : {}),
      locoArguments: benchmark
        ? ['nhs_dmd_official_release_comparison_snapshot', '--ignored', '--nocapture']
        : ['test', '--locked', '--test', 'care_api', 'nhs_dmd_official_release_comparison_snapshot', '--', '--ignored', '--nocapture'],
      railsArguments: [
      'run', '--rm', '--pull', 'never', '--network', `${ownership.project}_default`,
      '--env', 'RAILS_ENV=test', '--env', 'MEDTRACKER_OWNED_DATABASE=1',
      '--env', 'DATABASE_URL=postgresql://medtracker:medtracker_password@postgres:5432/medtracker_reference',
      '--env', 'NHS_DMD_RELEASE_PATH=/release.zip', '--env', 'NHS_DMD_COMPARISON_OUTPUT=/compare/rails.json',
      ...(benchmark ? ['--env', 'NHS_DMD_TIMING_OUTPUT=/compare/rails-timing.json'] : []),
      '--mount', `type=bind,source=${release},target=/release.zip,readonly`,
      '--mount', `type=bind,source=${referenceScript},target=/tmp/nhs-dmd-reference-snapshot.rb,readonly`,
      '--mount', `type=bind,source=${output},target=/compare`,
      image, 'bundle', 'exec', 'rails', 'runner', '/tmp/nhs-dmd-reference-snapshot.rb',
    ] });
    if (benchmark) await writeFile(join(output, 'setup-timings.json'), JSON.stringify(ownership.timings, null, 2));
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
