import { existsSync, readFileSync, readdirSync } from 'node:fs';
import { basename, dirname, join, relative, resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

function stripComments(source) {
  let out = '';
  let i = 0;
  let block = 0;
  while (i < source.length) {
    const pair = source.slice(i, i + 2);
    if (block > 0) {
      if (pair === '/*') block += 1;
      else if (pair === '*/') block -= 1;
      else out += source[i] === '\n' ? '\n' : ' ';
      i += pair === '/*' || pair === '*/' ? 2 : 1;
      continue;
    }
    if (pair === '//') {
      while (i < source.length && source[i] !== '\n') {
        out += ' ';
        i += 1;
      }
      continue;
    }
    if (pair === '/*') {
      block = 1;
      out += '  ';
      i += 2;
      continue;
    }
    out += source[i];
    i += 1;
  }
  return out;
}

function blankLiterals(line) {
  return line
    .replace(/r(#*)"([^]*?)"\1/g, match => ' '.repeat(match.length))
    .replace(/"(\\.|[^"\\])*"?/g, match => ' '.repeat(match.length))
    .replace(/'(\\.|[^'\\])'/g, match => ' '.repeat(match.length));
}

const fileStem = file =>
  basename(file) === 'mod.rs' ? dirname(file) : join(dirname(file), basename(file, '.rs'));

function declarations(file, visit) {
  const stem = fileStem(file);
  const scope = [];
  let depth = 0;
  let pendingPath = null;
  let pendingDisabled = false;
  for (const line of stripComments(readFileSync(file, 'utf8')).split('\n')) {
    const events = [
      ...[...line.matchAll(/#\[path\s*=\s*"([^"]*)"\]/g)].map(match => ({ index: match.index, kind: 'path', value: match[1] })),
      ...[...line.matchAll(/#\[cfg\s*\(\s*any\s*\(\s*\)\s*\)\s*\]/g)].map(match => ({ index: match.index, kind: 'disabled' })),
      ...[...blankLiterals(line).matchAll(/\bmod\s+([A-Za-z_][A-Za-z0-9_]*)\s*([{;])|[{}]/g)].map(match => ({ index: match.index, kind: 'structure', name: match[1], opener: match[2] === '{', brace: match[0] })),
    ].sort((left, right) => left.index - right.index);
    for (const event of events) {
      if (event.kind === 'path') {
        pendingPath = event.value;
        continue;
      }
      if (event.kind === 'disabled') {
        pendingDisabled = true;
        continue;
      }
      if (event.brace === '{') {
        depth += 1;
      } else if (event.brace === '}') {
        depth -= 1;
        while (scope.length && scope[scope.length - 1].depth === depth) scope.pop();
      } else if (event.opener) {
        scope.push({ name: event.name, depth, disabled: pendingDisabled });
        depth += 1;
      } else {
        const base = pendingPath === null ? join(stem, ...scope.map(entry => entry.name)) : scope.length ? join(stem, ...scope.map(entry => entry.name)) : dirname(file);
        if (!pendingDisabled && !scope.some(entry => entry.disabled)) {
          visit(pendingPath === null ? [join(base, `${event.name}.rs`), join(base, event.name, 'mod.rs')] : [resolve(base, pendingPath)]);
        }
      }
      pendingPath = null;
      pendingDisabled = false;
    }
    if (!events.length && /\S/.test(line)) {
      pendingPath = null;
      pendingDisabled = false;
    }
  }
}

function manifestTestPaths(root) {
  const cargo = join(root, 'Cargo.toml');
  if (!existsSync(cargo)) return [];
  const paths = [];
  let inside = false;
  for (const line of readFileSync(cargo, 'utf8').split('\n')) {
    if (/^\s*\[/.test(line)) {
      inside = line.trim() === '[[test]]';
      continue;
    }
    const entry = inside && line.match(/^\s*path\s*=\s*"([^"]+)"/);
    if (entry) paths.push(entry[1]);
  }
  return paths;
}

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
  for (const path of manifestTestPaths(root)) reachable.add(resolve(root, path));
  const queue = [...reachable];
  while (queue.length) {
    const file = queue.shift();
    declarations(file, candidates => {
      for (const candidate of candidates) {
        if (existsSync(candidate) && !reachable.has(candidate)) {
          reachable.add(candidate);
          queue.push(candidate);
        }
      }
    });
  }
  return all.filter(path => !reachable.has(path)).map(path => relative(root, path));
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const root = resolve(process.argv[2] ?? process.cwd());
  const orphans = orphanTestFiles(root);
  if (orphans.length) {
    console.error(`Unreachable Rust test files (add a mod declaration or remove):\n${orphans.map(path => `  ${path}`).join('\n')}`);
    process.exit(1);
  }
}
