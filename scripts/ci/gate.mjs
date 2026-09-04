import { appendFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';
import { policy } from './classify.mjs';

export function evaluate(needs) {
  const errors = [];
  if (needs.changes?.result !== 'success') errors.push('Change classification must succeed');
  for (const [suite, jobs] of Object.entries(policy.jobs)) {
    const selected = needs.changes?.outputs?.[suite];
    if (selected !== 'true' && selected !== 'false') errors.push(`${suite}: missing or invalid selection`);
    for (const job of jobs) {
      const result = needs[job]?.result;
      if (selected === 'true' && result !== 'success') errors.push(`${suite}: ${job} must succeed (received ${result ?? 'missing'})`);
      if (selected === 'false' && !['success', 'skipped'].includes(result)) errors.push(`${suite}: ${job} unexpectedly ${result ?? 'missing'}`);
      if (selected === 'true') {
        for (const output of policy.workflow_results[job] || []) {
          const child = needs[job]?.outputs?.[output];
          if (child !== 'success') errors.push(`${suite}: ${output} must succeed (received ${child ?? 'missing'})`);
        }
      }
    }
  }
  return errors;
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const errors = evaluate(JSON.parse(process.env.CI_NEEDS));
  const report = errors.length ? errors.join('\n') : 'Every selected mandatory CI suite succeeded.';
  console.log(report);
  if (process.env.GITHUB_STEP_SUMMARY) appendFileSync(process.env.GITHUB_STEP_SUMMARY, `## CI gate\n\n${report}\n`);
  if (errors.length) process.exitCode = 1;
}
