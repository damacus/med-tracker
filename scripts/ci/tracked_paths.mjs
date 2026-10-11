import { execFileSync } from 'node:child_process';
import { classify } from './classify.mjs';

const paths = execFileSync('git', ['ls-files', '--cached', '-z'], {
  encoding: 'utf8',
  maxBuffer: 64 * 1024 * 1024,
}).split('\0').filter(Boolean);

try {
  classify(paths);
  console.log(`All ${paths.length} tracked paths have a CI classification.`);
} catch (error) {
  console.error(error.message);
  process.exitCode = 1;
}
