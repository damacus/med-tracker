import { spawnSync } from 'node:child_process';
import { readFileSync } from 'node:fs';
import { basename, dirname, isAbsolute, join, relative, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const checkoutRoot = resolve(fileURLToPath(new URL('../..', import.meta.url)));

function manifestsFrom(metadata, root) {
  if (!metadata || typeof metadata !== 'object' || metadata.workspace_root !== root || !Array.isArray(metadata.workspace_members) || metadata.workspace_members.length === 0 || !Array.isArray(metadata.packages)) {
    throw new Error('cargo metadata returned an invalid workspace');
  }
  const packages = new Map();
  for (const pkg of metadata.packages) {
    if (!pkg || typeof pkg.id !== 'string' || !pkg.id || typeof pkg.manifest_path !== 'string' || packages.has(pkg.id)) {
      throw new Error('cargo metadata returned invalid or duplicate package records');
    }
    packages.set(pkg.id, pkg);
  }
  const manifests = metadata.workspace_members.map(id => {
    const manifest = packages.get(id)?.manifest_path;
    if (typeof id !== 'string' || !isAbsolute(manifest ?? '') || basename(manifest) !== 'Cargo.toml') {
      throw new Error(`cargo metadata omitted workspace member ${String(id)}`);
    }
    const path = relative(root, dirname(manifest));
    if (path === '..' || path.startsWith(`..${process.platform === 'win32' ? '\\' : '/'}`) || isAbsolute(path)) {
      throw new Error(`cargo metadata returned an out-of-workspace manifest for ${id}`);
    }
    return manifest;
  });
  if (new Set(metadata.workspace_members).size !== manifests.length || new Set(manifests).size !== manifests.length || !manifests.includes(join(root, 'Cargo.toml'))) {
    throw new Error('cargo metadata returned duplicate or incomplete workspace members');
  }
  return manifests;
}

export function checkUnusedDeps({ root = checkoutRoot, run = spawnSync, write = text => process.stderr.write(text) } = {}) {
  const lockPath = join(root, 'Cargo.lock');
  let before;
  try {
    before = readFileSync(lockPath);
  } catch (error) {
    write(`Cannot read Cargo.lock: ${error.message}\n`);
    return 2;
  }
  let status = 2;
  try {
    const metadataResult = run('cargo', ['metadata', '--locked', '--no-deps', '--format-version', '1', '--manifest-path', join(root, 'Cargo.toml')], { cwd: root, encoding: 'utf8' });
    if (metadataResult.error || metadataResult.status !== 0) {
      write(metadataResult.stderr || String(metadataResult.error ?? 'cargo metadata failed\n'));
      status = metadataResult.status || 2;
    } else {
      const manifests = manifestsFrom(JSON.parse(metadataResult.stdout), root);
      const macheteResult = run('cargo-machete', ['--with-metadata', '--skip-target-dir', ...manifests], { cwd: root, encoding: 'utf8' });
      write(macheteResult.stdout ?? '');
      write(macheteResult.stderr ?? '');
      if (macheteResult.error) {
        write(`cargo-machete failed: ${macheteResult.error.message}\n`);
        status = 2;
      } else if (/error when handling\b/i.test(`${macheteResult.stdout ?? ''}\n${macheteResult.stderr ?? ''}`)) {
        write('cargo-machete reported incomplete analysis\n');
        status = macheteResult.status || 2;
      } else {
        status = macheteResult.status ?? 2;
      }
    }
  } catch (error) {
    write(`Unused-dependency check failed: ${error.message}\n`);
    status = 2;
  }
  try {
    if (!before.equals(readFileSync(lockPath))) {
      write('Unused-dependency check modified Cargo.lock\n');
      status = 2;
    }
  } catch (error) {
    write(`Cannot verify Cargo.lock: ${error.message}\n`);
    status = 2;
  }
  return status;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  process.exitCode = checkUnusedDeps();
}
