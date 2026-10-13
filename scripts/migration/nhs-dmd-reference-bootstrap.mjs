import { join } from 'node:path';

export function assertCleanBenchmarkSource(status) {
  if (status.trim()) throw new Error('Benchmark requires a clean worktree, including untracked source files; commit the candidate and rebuild first');
}

export async function runReferenceImport({ url, provision, runCommand, locoCommand = 'cargo', locoArguments, locoEnvironment, railsArguments, environment }) {
  const referenceUrl = new URL(url);
  if (referenceUrl.hostname !== '127.0.0.1' || referenceUrl.pathname !== '/medtracker_loco' || !referenceUrl.port) throw new Error('Reference import requires an owned loopback database');
  referenceUrl.pathname = '/medtracker_reference';
  const utcEnvironment = { ...environment, TZ: 'UTC' };
  await provision();
  await runCommand('task', ['db:migrate', `MIGRATION_DATABASE_URL=${referenceUrl.href}`, 'LOCO_ENV=test'], utcEnvironment);
  await runCommand(locoCommand, locoArguments, { ...(locoEnvironment ?? environment), TZ: 'UTC' });
  await runCommand('docker', [railsArguments[0], '--env', 'TZ=UTC', ...railsArguments.slice(1)], utcEnvironment);
}

export function compareSnapshots(rails, loco) {
  const counts = ['status', 'created', 'updated', 'unchanged', 'skipped_expired', 'skipped_missing_name', 'skipped_invalid', 'trade_families', 'trade_family_groups', 'trade_memberships'];
  const countDifferences = Object.fromEntries(counts.filter(key => rails[key] !== loco[key]).map(key => [key, { rails: rails[key], loco: loco[key] }]));
  const rowDifferences = {};
  for (const key of ['barcodes', 'relationships', 'trade_family_group_rows', 'trade_family_rows', 'trade_membership_rows', 'supplementary_release_rows']) {
    const reference = new Map(rails[key].map(row => [row[0], row]));
    const candidate = new Map(loco[key].map(row => [row[0], row]));
    const keys = [...new Set([...reference.keys(), ...candidate.keys()])].sort();
    const mismatches = keys.filter(code => JSON.stringify(reference.get(code)) !== JSON.stringify(candidate.get(code)));
    rowDifferences[key] = { rails: rails[key].length, loco: loco[key].length, mismatches: mismatches.length, examples: mismatches.slice(0, 10).map(code => ({ code, rails: reference.get(code) ?? null, loco: candidate.get(code) ?? null })) };
  }
  return { countDifferences, rowDifferences };
}

export function comparisonInput(mode, release, root) {
  if (mode === 'synthetic') {
    if (release) throw new Error('NHS_DMD_RELEASE_PATH must be unset for synthetic comparison');
    return {
      release: join(root, 'tests/browser/fixtures/nhs-dmd-synthetic-supplementary.zip'),
      expectedChecksum: '5d83eef3d1377d3489bd0e43e3f79abfc779b7ee20ae97fad1c598efeb1a1646',
    };
  }
  if (mode !== 'official') throw new Error(`Unsupported comparison mode: ${mode}`);
  if (!release) throw new Error('NHS_DMD_RELEASE_PATH is required');
  return { release, expectedChecksum: '321db83dc17467430183e79ffdd98f1b0a1794c0d20f828d127682bc3f0edca6' };
}
