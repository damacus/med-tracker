import { readFile } from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import { runSlice } from './run-slice.mjs';

async function creditedTests() {
  const text = await readFile(new URL('../../tests/requests/api_v1_inventory.jsonl', import.meta.url), 'utf8');
  const required = new Map();
  for (const line of text.trim().split('\n')) {
    const operation = JSON.parse(line);
    if (!['unverified', 'partial', 'complete'].includes(operation.coverage) || !Array.isArray(operation.evidence)) throw new Error(`Invalid contract evidence for ${operation.operation_id}`);
    if (operation.coverage === 'unverified' && operation.evidence.length) throw new Error(`Unverified contract has evidence: ${operation.operation_id}`);
    if (operation.coverage !== 'unverified' && !operation.evidence.length) throw new Error(`Missing contract evidence for ${operation.operation_id}`);
    const withinOperation = new Set();
    for (const evidence of operation.evidence) {
      if (!['requests', 'care_api', 'oauth_server'].includes(evidence.binary) || !/^[A-Za-z0-9_]+(?:::[A-Za-z0-9_]+)+$/.test(evidence.test) || !Array.isArray(evidence.checks) || !evidence.checks.length || !evidence.checks.every(check => typeof check === 'string' && check.length)) throw new Error(`Invalid contract evidence for ${operation.operation_id}`);
      const key = `${evidence.binary}\0${evidence.test}`;
      if (withinOperation.has(key)) throw new Error(`Duplicate contract evidence for ${operation.operation_id}`);
      withinOperation.add(key);
      if (!required.has(evidence.binary)) required.set(evidence.binary, new Set());
      required.get(evidence.binary).add(evidence.test);
    }
  }
  return required;
}

export async function runContracts({ group, filter = '' }) {
  if (!['api-v1', 'api-v1-inventory'].includes(group)) throw new Error(`Unknown contract group: ${group}`);
  if (group === 'api-v1' && filter) throw new Error('API v1 acceptance cannot select one test');
  if (group === 'api-v1') {
    const required = await creditedTests();
    for (const [binary, names] of required) {
      const proof = await runSlice({ target: binary, filter: binary === 'requests' ? 'api_v1' : '', executionProof: true });
      for (const name of names) if (!proof.passed.has(name)) throw new Error(`Missing executed contract evidence: ${binary}::${name}`);
    }
    const gate = await runSlice({ target: 'requests', filter: 'api_v1', includeIgnored: true, executionProof: true });
    if (!gate.passed.has('api_v1::acceptance')) throw new Error('Missing executed API v1 acceptance gate');
    return;
  }
  await runSlice({
    target: 'requests',
    filter: filter || 'api_v1',
    noCapture: true,
  });
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  try {
    const args = process.argv.slice(2);
    if (args.some((value, index) => !['--group', '--filter'].includes(value) && !['--group', '--filter'].includes(args[index - 1]))) throw new Error('Unsupported contract runner argument');
    const group = args[args.indexOf('--group') + 1];
    const filter = args.includes('--filter') ? args[args.indexOf('--filter') + 1] : '';
    if (group === undefined || filter === undefined) throw new Error('Missing contract runner argument value');
    await runContracts({ group, filter });
  } catch (error) {
    process.stderr.write(`${error.message}\n`);
    process.exitCode = 1;
  }
}
