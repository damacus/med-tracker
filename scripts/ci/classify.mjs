import { execFileSync } from 'node:child_process';
import { appendFileSync, existsSync, readFileSync } from 'node:fs';
import { pathToFileURL } from 'node:url';

export const policy = JSON.parse(readFileSync(new URL('./policy.json', import.meta.url), 'utf8'));

export function classify(paths, { androidAvailable = existsSync('mobile/android/Taskfile.yml') } = {}) {
  const selected = Object.fromEntries(Object.keys(policy.jobs).map(suite => [suite, false]));
  const reasons = [];
  const unmapped = [];
  for (const path of paths) {
    const suites = new Set();
    const markdown = /\.md$/i.test(path);
    let mapped = markdown;
    if (markdown) suites.add('markdown');
    for (const rule of policy.rules) {
      if ((!markdown || rule.prose) && new RegExp(rule.pattern).test(path)) {
        mapped = true;
        for (const suite of rule.suites) {
          if (suite === 'all') {
            for (const name of Object.keys(selected)) {
              if (name !== 'android' || androidAvailable) suites.add(name);
            }
          } else if (suite === 'android_if_present') {
            if (androidAvailable) suites.add('android');
          } else {
            if (!(suite in selected)) throw new Error(`Unknown suite: ${suite}`);
            suites.add(suite);
          }
        }
      }
    }
    if (!mapped) unmapped.push(path);
    for (const suite of suites) selected[suite] = true;
    reasons.push({ path, suites: [...suites] });
  }
  if (unmapped.length) throw new Error(`Unmapped paths; update scripts/ci/policy.json:\n${unmapped.join('\n')}`);
  return { selected, reasons };
}

export function changedPaths(base, head = 'HEAD', event = 'pull_request') {
  if (!base) throw new Error('CI_BASE is required');
  const git = args => execFileSync('git', args, { encoding: 'utf8', maxBuffer: 64 * 1024 * 1024 });
  const target = git(['rev-parse', '--verify', `${head}^{commit}`]).trim();
  if (/^0+$/.test(base)) return git(['ls-tree', '-rz', '--name-only', target]).split('\0').filter(Boolean);
  const source = git(['rev-parse', '--verify', `${base}^{commit}`]).trim();
  const range = `${source}${event === 'push' ? '..' : '...'}${target}`;
  return git(['diff', '--no-renames', '--name-only', '-z', range, '--']).split('\0').filter(Boolean);
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const result = classify(changedPaths(process.env.CI_BASE, process.env.CI_HEAD, process.env.CI_EVENT));
  if (process.env.GITHUB_OUTPUT) {
    appendFileSync(process.env.GITHUB_OUTPUT, Object.entries(result.selected).map(([key, value]) => `${key}=${value}\n`).join(''));
  }
  if (process.env.GITHUB_STEP_SUMMARY) {
    appendFileSync(process.env.GITHUB_STEP_SUMMARY, `## Selected CI suites\n\n\`\`\`json\n${JSON.stringify(result, null, 2)}\n\`\`\`\n`);
  }
  console.log(JSON.stringify(result, null, 2));
}
