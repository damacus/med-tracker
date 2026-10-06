import { strict as assert } from 'node:assert';
import { spawnSync } from 'node:child_process';
import path from 'node:path';

const repositoryRoot = process.cwd();

const composeConfig = (label, composeFile) => {
  const result = spawnSync('docker', ['compose', 'config', '--quiet'], {
    cwd: repositoryRoot,
    env: {
      ...process.env,
      COMPOSE_FILE: composeFile,
      CONTRACT_STORAGE_ROOT: path.join(repositoryRoot, 'tmp'),
      CONTRACT_PROJECT: 'mtcontract-regression',
      CONTRACT_AUTH_SESSION_SECRET: 'a'.repeat(64),
      CONTRACT_APNS_PRIVATE_KEY: 'unused',
      CONTRACT_API_BUILD_CONTEXT: repositoryRoot,
      CONTRACT_BROWSER_BUILD_CONTEXT: path.join(repositoryRoot, 'rust', 'web'),
      CONTRACT_FIXTURE_DIR: path.join(repositoryRoot, 'tmp'),
      CONTRACT_DATABASE_URL: 'postgresql://medtracker:medtracker_password@db-test:5432/medtracker'
    },
    encoding: 'utf8'
  });
  assert.equal(result.error, undefined, `docker compose could not start: ${result.error?.message}`);
  assert.equal(
    result.status,
    0,
    `${label} compose configuration is invalid:\n${result.stdout}\n${result.stderr}`
  );
};

composeConfig(
  'legacy (existing Rust URL)',
  'rails/compose.yaml:rust/contract-tests/storage.compose.yaml'
);
composeConfig(
  'runner (self-provisioning)',
  'rails/compose.yaml:rust/contract-tests/storage.compose.yaml:rust/contract-tests/runner.compose.yaml'
);

console.log('Contract compose regression passed: legacy and runner configurations both resolve.');
