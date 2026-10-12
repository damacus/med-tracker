import { chmod, copyFile, mkdir, writeFile } from 'node:fs/promises';
import { execute } from './loco-process.mjs';

const directory = 'target/ci';
console.log('Compile application and integration executables for reuse');
await mkdir(directory, { recursive: true });
const output = await execute('cargo', ['test', '--locked', '--tests', '--no-run', '--message-format=json'], { capture: true });
const messages = output.split('\n').filter(Boolean).map(line => JSON.parse(line));
const artifacts = messages.filter(item => item.reason === 'compiler-artifact' && item.executable && item.target.kind.includes('test'));
const manifest = [];
for (const item of artifacts) {
  const path = `${directory}/${item.target.name}`;
  await copyFile(item.executable, path);
  await chmod(path, 0o755);
  manifest.push({ name: item.target.name, path });
}
if (!manifest.length) throw new Error('No integration test binaries were built');
const application = messages.find(item => item.executable && item.target.name === 'med-tracker' && !item.profile.test);
if (!application) throw new Error('Application binary missing');
await copyFile(application.executable, `${directory}/med-tracker`);
await chmod(`${directory}/med-tracker`, 0o755);
await writeFile(`${directory}/tests.json`, JSON.stringify(manifest, null, 2));
