import { createHash } from 'node:crypto';
import { createReadStream } from 'node:fs';
import { chmod, copyFile, mkdir, readFile, stat, writeFile } from 'node:fs/promises';
import { join, resolve } from 'node:path';
import { isDatabaseFree } from './loco-inventory.mjs';
import { parseShard, partition } from './loco-shards.mjs';
import { execute } from './loco-process.mjs';
import { sourceIdentity } from '../migration/run-slice.mjs';
import { withOwnedDatabase } from '../migration/foundation-database.mjs';

const assetFiles = ['assets/static/css/compiled.css', 'assets/static/js/html5-qrcode.min.js'];

async function digest(path) {
  const hash = createHash('sha256');
  for await (const chunk of createReadStream(path)) hash.update(chunk);
  return hash.digest('hex');
}

export function archiveArguments(path) {
  if (typeof path !== 'string' || !path.endsWith('.tar.zst')) throw new Error('Nextest archive path must end in .tar.zst');
  return ['nextest', 'archive', '--locked', '--workspace', '--archive-file', path];
}

export function parseNextestInventory(listing) {
  if (!listing || !Number.isSafeInteger(listing['test-count']) || listing['test-count'] <= 0 || !listing['rust-suites'] || typeof listing['rust-suites'] !== 'object') throw new Error('Missing nextest test inventory');
  const identities = new Set();
  const binaries = new Set();
  const inventory = [];
  let total = 0;
  for (const [key, suite] of Object.entries(listing['rust-suites'])) {
    if (!suite || key !== suite['binary-id'] || !['lib', 'bin', 'test'].includes(suite.kind) || suite.status !== 'listed' || !suite.testcases || typeof suite.testcases !== 'object') throw new Error(`Invalid nextest suite: ${key}`);
    if (binaries.has(suite['binary-id'])) throw new Error(`Duplicate nextest binary: ${key}`);
    binaries.add(key);
    for (const [name, details] of Object.entries(suite.testcases)) {
      if (!/^[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*$/.test(name) || typeof details?.ignored !== 'boolean') throw new Error(`Invalid nextest testcase: ${key}/${name}`);
      const identity = `${key}/${name}`;
      if (identities.has(identity)) throw new Error(`Duplicate nextest testcase: ${identity}`);
      identities.add(identity);
      total += 1;
      if (suite.kind === 'lib' || details.ignored) continue;
      const binaryName = suite['binary-name'] ?? key.split('::').at(-1);
      inventory.push({ binary: key, binaryName, kind: suite.kind, name, identity, databaseFree: isDatabaseFree(binaryName, name) });
    }
  }
  if (total !== listing['test-count'] || inventory.length === 0) throw new Error('Nextest inventory count mismatch or no runnable tests');
  return inventory;
}

function canonicalInventory(inventory) {
  return inventory.map(item => [item.identity, item.binary, item.binaryName, item.kind, item.name, item.databaseFree]).sort((a, b) => a[0].localeCompare(b[0]));
}

export function selectedTests(inventory, stage, shard = '1/1') {
  if (!['unit', 'integration'].includes(stage)) throw new Error('Unknown nextest stage');
  const matches = inventory.filter(item => item.databaseFree === (stage === 'unit'));
  if (stage === 'unit') return matches;
  const { index, total } = parseShard(shard);
  const selected = new Set(partition(matches.map(item => item.identity), total)[index]);
  if (!selected.size) throw new Error('Integration shard selected no tests');
  return matches.filter(item => selected.has(item.identity));
}

export function filterForSelection(binary, names) {
  if (typeof binary !== 'string' || !/^[A-Za-z0-9_-]+(?:::[A-Za-z0-9_-]+)*$/.test(binary) || !Array.isArray(names) || !names.length || new Set(names).size !== names.length || names.some(name => !/^[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)*$/.test(name))) throw new Error('Invalid exact nextest selection');
  return `binary_id(=${binary}) & (${names.map(name => `test(=${name})`).join(' | ')})`;
}

export async function buildArchiveArtifact({ directory = 'target/ci', root = process.cwd(), run = execute, identity = () => sourceIdentity(process.env) } = {}) {
  const source = await identity();
  const archive = join(directory, 'loco-tests.tar.zst');
  const application = join(directory, 'med-tracker');
  await mkdir(directory, { recursive: true });
  await run('cargo', archiveArguments(archive));
  const buildOutput = await run('cargo', ['build', '--locked', '--bin', 'med-tracker', '--message-format=json'], { capture: true });
  const artifacts = buildOutput.split('\n').filter(Boolean).map(line => JSON.parse(line)).filter(message => message.reason === 'compiler-artifact' && message.target?.name === 'med-tracker' && message.target?.kind?.includes('bin') && !message.profile?.test && message.executable);
  if (artifacts.length !== 1) throw new Error('Expected exactly one built Loco application executable');
  await copyFile(artifacts[0].executable, application);
  await chmod(application, 0o755);
  const listed = await run('cargo', ['nextest', 'list', '--archive-file', archive, '--workspace-remap', resolve(root), '--message-format', 'json'], { capture: true });
  const inventory = parseNextestInventory(JSON.parse(listed));
  const after = await identity();
  if (source.revision !== after.revision || source.sha256 !== after.sha256) throw new Error('Source changed while building the nextest archive');
  const assetsSha256 = Object.fromEntries(await Promise.all(assetFiles.map(async path => [path, await digest(join(root, path))])));
  const manifest = { version: 1, revision: source.revision, sourceSha256: source.sha256, archiveSha256: await digest(archive), applicationSha256: await digest(application), assetsSha256, inventory };
  await writeFile(join(directory, 'tests.json'), `${JSON.stringify(manifest, null, 2)}\n`);
  return manifest;
}

export async function validateArchiveArtifact(directory = 'target/ci', source, root = process.cwd()) {
  const manifest = JSON.parse(await readFile(join(directory, 'tests.json'), 'utf8'));
  if (manifest.version !== 1 || !Array.isArray(manifest.inventory) || !manifest.inventory.length || !/^[a-f0-9]{64}$/.test(manifest.archiveSha256) || !/^[a-f0-9]{64}$/.test(manifest.applicationSha256) || assetFiles.some(path => !/^[a-f0-9]{64}$/.test(manifest.assetsSha256?.[path]))) throw new Error('Invalid Loco archive manifest');
  if (manifest.revision !== source.revision || manifest.sourceSha256 !== source.sha256) throw new Error('Loco archive source identity differs from checkout');
  if (await digest(join(directory, 'loco-tests.tar.zst')) !== manifest.archiveSha256) throw new Error('Loco nextest archive checksum mismatch');
  if (await digest(join(directory, 'med-tracker')) !== manifest.applicationSha256 || ((await stat(join(directory, 'med-tracker'))).mode & 0o111) === 0) throw new Error('Loco application executable checksum or permissions mismatch');
  for (const path of assetFiles) if (await digest(join(root, path)) !== manifest.assetsSha256[path]) throw new Error(`Loco packaged asset checksum mismatch: ${path}`);
  const identities = manifest.inventory.map(item => item.identity);
  if (new Set(identities).size !== identities.length || manifest.inventory.some(item => !item.binary || !item.name || item.identity !== `${item.binary}/${item.name}`)) throw new Error('Invalid Loco archive inventory');
  return manifest;
}

export async function runArchiveStage({ stage, shard = '1/1', directory = 'target/ci', root = process.cwd(), run = execute, own = withOwnedDatabase, identity, environment = process.env, proofFactory } = {}) {
  const currentSource = identity ? await identity() : await sourceIdentity(environment, undefined, root);
  const manifest = await validateArchiveArtifact(directory, currentSource, root);
  const archive = resolve(directory, 'loco-tests.tar.zst');
  const listed = await run('cargo', ['nextest', 'list', '--archive-file', archive, '--workspace-remap', resolve(root), '--message-format', 'json'], { capture: true });
  const archivedInventory = parseNextestInventory(JSON.parse(listed));
  if (JSON.stringify(canonicalInventory(archivedInventory)) !== JSON.stringify(canonicalInventory(manifest.inventory))) throw new Error('Loco archive inventory differs from manifest');
  const selected = selectedTests(manifest.inventory, stage, shard);
  if (!selected.length) throw new Error('Nextest archive stage selected no tests');
  if (stage === 'integration') {
    const { index } = parseShard(shard);
    const selectionPath = join(root, 'test-results', `integration-selection-${index + 1}.json`);
    await mkdir(join(root, 'test-results'), { recursive: true });
    await writeFile(selectionPath, JSON.stringify(selected.map(item => item.identity)));
  }
  const application = resolve(directory, 'med-tracker');
  const binaries = [...new Set(selected.map(item => item.binary))];
  const filters = binaries.map(binary => filterForSelection(binary, selected.filter(item => item.binary === binary).map(item => item.name)));
  const filter = filters.map(expression => `(${expression})`).join(' | ');
  const createProof = proofFactory ?? (await import('../migration/nextest-proof.mjs')).createNextestProofReport;
  const proofReport = await createProof(resolve(root, environment.CARGO_TARGET_DIR ?? 'target'));
  const runSelected = async env => {
    await run('cargo', ['nextest', 'run', '--archive-file', archive, '--workspace-remap', resolve(root), '--no-tests=fail', '--test-threads', '2', '--retries', '0', '--config-file', proofReport.configPath, '--profile', 'contract-proof', '-E', filter], { env });
    const results = await proofReport.readSuites(binaries);
    for (const binary of binaries) {
      const expected = selected.filter(item => item.binary === binary).map(item => item.name).sort();
      const executed = [...results.get(binary).passed].sort();
      if (JSON.stringify(executed) !== JSON.stringify(expected)) throw new Error(`Nextest archive result did not execute exact selection: ${binary}`);
    }
  };
  try {
    if (stage === 'unit') {
      const env = { ...environment };
      delete env.DATABASE_URL;
      delete env.MEDTRACKER_OWNED_DATABASE;
      return await runSelected(env);
    }
    const needsStorage = selected.some(item => item.binaryName === 'care_api');
    return await own(async (url, provision, ownership) => {
      await provision();
      const storage = needsStorage ? await ownership.storage() : {};
      const env = { ...environment, ...ownership.storageEnvironment, ...storage, LOCO_TEST_APPLICATION: application, DATABASE_URL: url, MEDTRACKER_OWNED_DATABASE: '1', MEDTRACKER_SESSION_KEY: Buffer.alloc(64, 7).toString('base64'), RAILS_SECRET_KEY_BASE: 'synthetic-slice-rails-verification-secret' };
      for (const name of ['COMPOSE_FILE', 'COMPOSE_PROJECT_NAME', 'COMPOSE_PROFILES', 'DOCKER_HOST', 'DOCKER_CONTEXT', 'DOCKER_CONFIG', 'DOCKER_TLS_VERIFY', 'DOCKER_CERT_PATH']) delete env[name];
      await runSelected(env);
    }, undefined, environment, { storage: needsStorage });
  } finally {
    await proofReport.cleanup();
  }
}
