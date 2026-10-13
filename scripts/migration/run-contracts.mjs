import { pathToFileURL } from 'node:url';
import { runSlice } from './run-slice.mjs';

export async function runContracts({ group, filter = '' }) {
  if (!['api-v1', 'api-v1-inventory'].includes(group)) throw new Error(`Unknown contract group: ${group}`);
  if (group === 'api-v1' && filter) throw new Error('API v1 acceptance cannot select one test');
  await runSlice({
    target: 'requests',
    filter: filter || 'api_v1',
    includeIgnored: group === 'api-v1',
    noCapture: group === 'api-v1-inventory',
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
