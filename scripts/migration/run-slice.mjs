import { execFileSync, spawn } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { withoutCoverageOverrides } from './coverage-environment.mjs';
import { withOwnedDatabase } from './foundation-database.mjs';
import { stopOwnedProcess } from './process-cleanup.mjs';

const root = fileURLToPath(new URL('../../', import.meta.url));

async function sourceIdentity(environment) {
  const git = args => execFileSync('git', args, { cwd: root, env: environment, maxBuffer: 64 * 1024 * 1024 });
  const revision = git(['rev-parse', 'HEAD']).toString().trim();
  const status = git(['status', '--porcelain', '--untracked-files=all', '-z']);
  const trackedDiff = git(['diff', '--binary', 'HEAD', '--']);
  const untracked = git(['ls-files', '--others', '--exclude-standard', '-z']).toString().split('\0').filter(Boolean).sort();
  const hash = createHash('sha256').update(revision).update(status).update(trackedDiff);
  for (const path of untracked) hash.update(path).update(await readFile(join(root, path)));
  return { revision, dirty: status.length > 0, sha256: hash.digest('hex') };
}

async function runCargo(args, databaseUrl, inheritedEnvironment, signal, coverageEnvironment) {
  signal.throwIfAborted();
  const environment = { ...(coverageEnvironment ? withoutCoverageOverrides(inheritedEnvironment) : inheritedEnvironment), ...coverageEnvironment, DATABASE_URL: databaseUrl, MEDTRACKER_OWNED_DATABASE: '1', MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'), RAILS_SECRET_KEY_BASE: 'synthetic-slice-rails-verification-secret' };
  for (const name of ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']) delete environment[name];
  const child = spawn('cargo', args, { cwd: root, env: environment, detached: true, stdio: 'inherit' });
  let timer;
  let interrupt;
  let failed = false;
  try {
    await new Promise((resolve, reject) => {
      interrupt = () => reject(signal.reason);
      signal.addEventListener('abort', interrupt, { once: true });
      timer = setTimeout(() => reject(new Error('Slice Cargo execution exceeded 30 minutes')), 1800000);
      child.once('error', reject);
      child.once('close', code => code === 0 ? resolve() : reject(new Error(`Slice Cargo execution exited ${code}`)));
    });
  } catch (error) {
    failed = true;
    throw error;
  } finally {
    clearTimeout(timer);
    signal.removeEventListener('abort', interrupt);
    try { await stopOwnedProcess(child); } catch (error) {
      if (!failed) throw error;
      process.stderr.write(`Slice child cleanup failed: ${error.message}\n`);
    }
  }
}

export async function runCoverage({ outputDir = 'target/coverage/reports' } = {}, inheritedEnvironment = process.env) {
  if (typeof outputDir !== 'string' || !outputDir.trim() || outputDir !== outputDir.trim() || outputDir.startsWith('-')) throw new Error('Coverage output directory must be a non-empty path without surrounding whitespace or a leading flag');
  const runId = randomUUID();
  const reportDirectory = join(resolve(root, outputDir), runId);
  const reportPath = join(reportDirectory, 'lcov.info');
  const manifestPath = join(reportDirectory, 'manifest.json');
  const instrumentedDirectory = join(root, 'target/coverage/instrumented', runId);
  const coverageEnvironment = {
    CARGO_TARGET_DIR: join(instrumentedDirectory, 'target'),
    CARGO_BUILD_BUILD_DIR: join(instrumentedDirectory, 'intermediates'),
  };
  const metadataEnvironment = withoutCoverageOverrides(inheritedEnvironment);
  for (const name of ['DATABASE_URL', 'COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH', 'MEDTRACKER_SESSION_KEY', 'RAILS_SECRET_KEY_BASE']) delete metadataEnvironment[name];
  const controller = new AbortController();
  const interrupted = signal => controller.abort(new Error(`Coverage execution interrupted by ${signal}`));
  const onTerm = () => interrupted('SIGTERM');
  const onInt = () => interrupted('SIGINT');
  process.on('SIGTERM', onTerm);
  process.on('SIGINT', onInt);
  let complete = false;
  try {
    const sourceAtStart = await sourceIdentity(metadataEnvironment);
    const rustc = execFileSync('rustc', ['--version'], { cwd: root, env: metadataEnvironment, encoding: 'utf8' }).trim();
    const cargoLlvmCov = execFileSync('cargo', ['llvm-cov', '--version'], { cwd: root, env: metadataEnvironment, encoding: 'utf8' }).trim();
    await mkdir(reportDirectory, { recursive: true });
    await withOwnedDatabase(async (url, provision, ownership) => {
      await provision();
      const scannerEnvironment = await ownership.storage();
      const environment = { ...inheritedEnvironment, ...ownership.storageEnvironment, ...scannerEnvironment };
      const run = args => runCargo(['llvm-cov', ...args], url, environment, controller.signal, coverageEnvironment);
      await run(['clean', '--workspace']);
      await run(['test', '--no-report', '--locked', '--lib']);
      await run(['test', '--no-report', '--no-clean', '--locked', '--test', 'requests', 'api_v1']);
      await run(['test', '--no-report', '--no-clean', '--locked', '--test', 'care_api', '--', '--test-threads=2']);
      await run(['report', '--lcov', '--output-path', reportPath]);
    }, undefined, inheritedEnvironment, { storage: true });
    if (!(await readFile(reportPath)).length) throw new Error('Rust coverage report is empty');
    const sourceAtEnd = await sourceIdentity(metadataEnvironment);
    if (sourceAtEnd.sha256 !== sourceAtStart.sha256) throw new Error('Source changed while measuring Rust coverage');
    const manifest = {
      status: 'complete',
      source_revision: sourceAtEnd.revision,
      source_state: { dirty: sourceAtEnd.dirty, sha256: sourceAtEnd.sha256 },
      rustc,
      cargo_llvm_cov: cargoLlvmCov,
      selectors: ['lib', 'requests:api_v1', 'care_api'],
      report: reportPath,
      generated_at: new Date().toISOString(),
    };
    await writeFile(manifestPath, `${JSON.stringify(manifest, null, 2)}\n`);
    complete = true;
    process.stdout.write(`Rust coverage LCOV: ${reportPath}\n`);
  } finally {
    if (!complete) await rm(reportDirectory, { recursive: true, force: true });
    await rm(instrumentedDirectory, { recursive: true, force: true });
    process.off('SIGTERM', onTerm);
    process.off('SIGINT', onInt);
  }
}

export async function runSlice({ all = false, target, filter = '', includeIgnored = false, noCapture = false }, inheritedEnvironment = process.env) {
  if (!all && !/^[A-Za-z0-9][A-Za-z0-9_-]*$/.test(target ?? '')) throw new Error('TARGET must name a root Rust test binary');
  if (filter && !/^[A-Za-z0-9_:]+$/.test(filter)) throw new Error('FILTER must name a Rust test');
  const args = ['test', '--locked'];
  if (!all) args.push('--test', target);
  if (filter) args.push(filter);
  if (includeIgnored || noCapture) args.push('--', ...(includeIgnored ? ['--include-ignored'] : []), ...(noCapture ? ['--nocapture'] : []));
  const controller = new AbortController();
  const interrupted = signal => controller.abort(new Error(`Slice execution interrupted by ${signal}`));
  const onTerm = () => interrupted('SIGTERM');
  const onInt = () => interrupted('SIGINT');
  process.on('SIGTERM', onTerm);
  process.on('SIGINT', onInt);
  try {
    await withOwnedDatabase(async (url, provision, ownership) => {
      if (all || ['persistence', 'tenant_access', 'care_doses', 'care_medications', 'care_api', 'identity_compatibility', 'identity_resource', 'oauth_server', 'queue_runtime', 'requests'].includes(target)) await provision();
      const needsScannerStorage = all || (target === 'care_api' && (!filter || filter.includes('nhs_dmd')));
      const scannerEnvironment = needsScannerStorage ? await ownership.storage() : {};
      await runCargo(args, url, { ...inheritedEnvironment, ...ownership.storageEnvironment, ...scannerEnvironment }, controller.signal);
    }, undefined, inheritedEnvironment, { storage: all || target === 'care_api' });
  } finally {
    process.off('SIGTERM', onTerm);
    process.off('SIGINT', onInt);
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const arguments_ = process.argv.slice(2);
    if (arguments_.some((argument, index) => !['--all', '--target', '--filter', '--coverage', '--output-dir'].includes(argument) && !['--target', '--filter', '--output-dir'].includes(arguments_[index - 1]))) throw new Error('Unsupported slice runner argument');
    const all = process.argv.includes('--all');
    const coverage = process.argv.includes('--coverage');
    const target = arguments_[arguments_.indexOf('--target') + 1];
    const filter = arguments_[arguments_.indexOf('--filter') + 1];
    const outputDir = arguments_.includes('--output-dir') ? arguments_[arguments_.indexOf('--output-dir') + 1] : undefined;
    if ((arguments_.includes('--target') && target === undefined) || (arguments_.includes('--filter') && filter === undefined) || (arguments_.includes('--output-dir') && outputDir === undefined)) throw new Error('Missing slice runner argument value');
    if (coverage && (all || arguments_.includes('--target') || arguments_.includes('--filter'))) throw new Error('Coverage cannot select slice test arguments');
    if (coverage) await runCoverage({ outputDir });
    else if (arguments_.includes('--output-dir')) throw new Error('Output directory requires coverage mode');
    else await runSlice({ all, target, filter: all ? '' : filter });
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
