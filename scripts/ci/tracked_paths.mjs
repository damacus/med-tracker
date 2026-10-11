import { execFileSync } from 'node:child_process';
import { classify } from './classify.mjs';

const list = (args) => execFileSync('git', ['ls-files', ...args, '-z'], {
  encoding: 'utf8',
  maxBuffer: 64 * 1024 * 1024,
}).split('\0').filter(Boolean);
const paths = [...new Set([...list(['--cached']), ...list(['--others', '--exclude-standard'])])];

try {
  classify(paths);
  console.log(`All ${paths.length} tracked and nonignored new paths have a CI classification.`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
