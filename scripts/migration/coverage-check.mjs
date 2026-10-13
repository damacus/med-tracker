import { spawnSync } from 'node:child_process';
import { withoutCoverageOverrides } from './coverage-environment.mjs';

function output(command, args) {
  const result = spawnSync(command, args, { encoding: 'utf8', env: withoutCoverageOverrides(process.env) });
  if (result.error || result.status !== 0) return null;
  return result.stdout.trim();
}

const version = output('cargo', ['llvm-cov', '--version']);
if (version !== 'cargo-llvm-cov 0.9.1') {
  process.stderr.write('cargo-llvm-cov 0.9.1 is required; run task coverage:install explicitly.\n');
  process.exitCode = 1;
} else {
  const components = output('rustup', ['component', 'list', '--installed']);
  if (!components?.split('\n').some(line => line.startsWith('llvm-tools-'))) {
    process.stderr.write('The active Rust toolchain needs llvm-tools-preview; run task coverage:install explicitly.\n');
    process.exitCode = 1;
  } else {
    process.stdout.write(`${version}\n${components.split('\n').find(line => line.startsWith('llvm-tools-'))}\n`);
  }
}
