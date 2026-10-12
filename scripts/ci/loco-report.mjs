import { glob, readFile, appendFile, writeFile } from 'node:fs/promises';

const lines = ['## Loco execution timings', '', 'Aggregate command time; concurrent phases overlap.', '', '| Phase | Calls | Seconds |', '| --- | ---: | ---: |'];
const phases = new Map();
for await (const path of glob('test-results/**/timings/*.json')) {
  const item = JSON.parse(await readFile(path, 'utf8'));
  const label = item.command === 'task' ? item.args[0] : `${item.command} ${item.args[0] ?? ''}`;
  const phase = phases.get(label) ?? { count: 0, durationMs: 0 };
  phase.count++;
  phase.durationMs += item.durationMs;
  phases.set(label, phase);
}
for (const [label, phase] of [...phases].sort((a, b) => b[1].durationMs - a[1].durationMs)) lines.push(`| ${label} | ${phase.count} | ${(phase.durationMs / 1000).toFixed(2)} |`);
const weights = {};
for await (const path of glob('test-results/**/runtime-report.json')) {
  const browser = JSON.parse(await readFile(path, 'utf8'));
  const collect = suite => {
    for (const spec of suite.specs ?? []) for (const test of spec.tests ?? []) {
      if (test.results.length !== 1 || test.results[0].status !== test.expectedStatus) continue;
      const key = `${test.projectName}/${spec.file.split('/').at(-1)}`;
      weights[key] = (weights[key] ?? 0) + test.results[0].duration / 1000;
    }
    for (const child of suite.suites ?? []) collect(child);
  };
  for (const suite of browser.suites ?? []) collect(suite);
  lines.push('', `Browser results: ${browser.stats.expected} expected, ${browser.stats.unexpected} unexpected, ${browser.stats.flaky} flaky, ${browser.stats.skipped} skipped.`);
}
if (Object.keys(weights).length) await writeFile('test-results/browser-timings-proposed.json', `${JSON.stringify(weights, null, 2)}\n`);
const report = `${lines.join('\n')}\n`;
console.log(report);
if (process.env.GITHUB_STEP_SUMMARY) await appendFile(process.env.GITHUB_STEP_SUMMARY, report);
