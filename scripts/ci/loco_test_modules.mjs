import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { basename, dirname, join, relative, resolve } from 'node:path';

export function orphanTestFiles(root) {
  const tests = join(root, 'tests');
  if (!existsSync(tests)) return [];
  const all = [];
  const walk = dir => {
    for (const entry of readdirSync(dir, { withFileTypes: true })) {
      const path = join(dir, entry.name);
      if (entry.isDirectory()) walk(path);
      else if (entry.name.endsWith('.rs')) all.push(path);
    }
  };
  walk(tests);
  const reachable = new Set(all.filter(path => dirname(path) === tests));
  const cargo = join(root, 'Cargo.toml');
  if (existsSync(cargo)) {
    const manifest = readFileSync(cargo, 'utf8');
    for (const match of manifest.matchAll(/\[\[test\]\]\s*([^[]*?)path\s*=\s*"([^"]+)"/g)) {
      reachable.add(resolve(root, match[2]));
    }
  }
  const queue = [...reachable];
  while (queue.length) {
    const file = queue.shift();
    const stem = file.endsWith('mod.rs') ? dirname(file) : join(dirname(file), basename(file, '.rs'));
    let pending = null;
    for (const line of readFileSync(file, 'utf8').split('\n')) {
      const attribute = line.match(/#\[path\s*=\s*"([^"]+)"\]/);
      if (attribute) {
        pending = attribute[1];
        continue;
      }
      const declaration = line.match(/^\s*(?:pub(?:\s*\([^)]*\))?\s+)?mod\s+[A-Za-z_][A-Za-z0-9_]*\s*;/);
      if (!declaration) {
        if (!/^\s*(#|\/\/|$)/.test(line)) pending = null;
        continue;
      }
      const candidates = pending
        ? [resolve(dirname(file), pending)]
        : [join(stem, `${declaration[0].match(/mod\s+([A-Za-z_][A-Za-z0-9_]*)/)[1]}.rs`), join(stem, declaration[0].match(/mod\s+([A-Za-z_][A-Za-z0-9_]*)/)[1], 'mod.rs')];
      pending = null;
      for (const candidate of candidates) {
        if (existsSync(candidate) && !reachable.has(candidate)) {
          reachable.add(candidate);
          queue.push(candidate);
        }
      }
    }
  }
  return all.filter(path => !reachable.has(path)).map(path => relative(root, path));
}

const root = process.argv[2] ?? process.cwd();
const orphans = orphanTestFiles(resolve(root));
if (orphans.length) {
  console.error(`Unreachable Rust test files (add a mod declaration or remove):\n${orphans.map(path => `  ${path}`).join('\n')}`);
  process.exit(1);
}
