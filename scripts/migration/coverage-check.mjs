import { spawnSync } from 'node:child_process';
import { withoutCoverageOverrides } from './coverage-environment.mjs';

function output(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', env: withoutCoverageOverrides(process.env) });
  if (result.error || result.status !== 0) return null;
  return result.stdout.trim();
}

const version = output('cargo', ['llvm-cov', '--version']);
if (output('python3', ['-I', '-c', 'import fcntl']) === null) {
  process.stderr.write('Python 3 with fcntl is required for the coverage build lock.\n');
  process.exitCode = 1;
} else if (version !== 'cargo-llvm-cov 0.9.1') {
  process.stderr.write('cargo-llvm-cov 0.9.1 is required; run task coverage:install explicitly.\n');
  process.exitCode = 1;
} else {
  const nextest = output('cargo', ['nextest', '--version']);
  if (!/^cargo-nextest 0\.9\.148(?:\s|$)/.test(nextest ?? '')) {
    process.stderr.write('cargo-nextest 0.9.148 is required for Rust coverage.\n');
    process.exitCode = 1;
  } else {
    const components = output('rustup', ['component', 'list', '--installed']);
    if (!components?.split('\n').some(line => line.startsWith('llvm-tools-'))) {
      process.stderr.write('The active Rust toolchain needs llvm-tools-preview; run task coverage:install explicitly.\n');
      process.exitCode = 1;
    } else {
      process.stdout.write(`${version}\n${nextest}\n${components.split('\n').find(line => line.startsWith('llvm-tools-'))}\n`);
    }
  }
}
