import { execFileSync } from 'node:child_process';
import { existsSync } from 'node:fs';
import { changedPaths } from './classify.mjs';

const base = process.env.CI_BASE;
const head = process.env.CI_HEAD || 'HEAD';
const files = changedPaths(base, head, process.env.CI_EVENT).filter(path => /\.md$/i.test(path) && existsSync(path));
const range = `${base}${process.env.CI_EVENT === 'push' ? '..' : '...'}${head}`;
if (!/^0+$/.test(base)) execFileSync('git', ['diff', '--check', range, '--'], { stdio: 'inherit' });
for (let offset = 0; offset < files.length; offset += 100) {
  execFileSync('npx', ['--yes', 'markdownlint-cli2@0.20.0', '--config', 'scripts/ci/markdown.markdownlint-cli2.jsonc', '--', ...files.slice(offset, offset + 100).map(path => `:${path}`)], { stdio: 'inherit' });
}
