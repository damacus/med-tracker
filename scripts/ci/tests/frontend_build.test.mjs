import assert from 'node:assert/strict';
import { chmodSync, copyFileSync, mkdirSync, mkdtempSync, readFileSync, rmSync, unlinkSync, writeFileSync } from 'node:fs';
import { spawnSync } from 'node:child_process';
import { tmpdir } from 'node:os';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

const taskfile = fileURLToPath(new URL('../../../Taskfiles/frontend.yml', import.meta.url));

test('frontend build copies changed scanner source and recreates either missing output', () => {
  const cwd = mkdtempSync(join(tmpdir(), 'medtracker-frontend-build-'));
  const createFile = (name, contents) => {
    const path = join(cwd, name);
    mkdirSync(dirname(path), { recursive: true });
    writeFileSync(path, contents);
    return path;
  };
  const build = () => {
    const result = spawnSync('task', ['--taskfile', 'Taskfile.yml', 'build'], { cwd, encoding: 'utf8' });
    assert.equal(result.status, 0, result.stdout + result.stderr);
  };
  try {
    copyFileSync(taskfile, join(cwd, 'Taskfile.yml'));
    createFile('package.json', '{"scripts":{"build":"tailwindcss -i assets/static/css/app.css -o assets/static/css/compiled.css --minify"}}');
    for (const name of ['package-lock.json', 'assets/static/css/app.css', 'assets/static/css/themes.css', 'assets/static/css/fonts.css', 'assets/static/css/platform.css', 'assets/static/js/app.js', 'assets/views/index.html']) createFile(name, 'input');
    const executable = createFile('node_modules/.bin/tailwindcss', '#!/usr/bin/env node\nrequire("node:fs").writeFileSync("assets/static/css/compiled.css", "compiled")\n');
    chmodSync(executable, 0o755);
    const source = createFile('node_modules/html5-qrcode/html5-qrcode.min.js', 'scanner-one');
    const scanner = join(cwd, 'assets/static/js/html5-qrcode.min.js');
    const css = join(cwd, 'assets/static/css/compiled.css');
    build();
    assert.equal(readFileSync(scanner, 'utf8'), 'scanner-one');
    assert.equal(readFileSync(css, 'utf8'), 'compiled');
    writeFileSync(source, 'scanner-two');
    build();
    assert.equal(readFileSync(scanner, 'utf8'), 'scanner-two');
    unlinkSync(scanner);
    build();
    assert.equal(readFileSync(scanner, 'utf8'), 'scanner-two');
    unlinkSync(css);
    build();
    assert.equal(readFileSync(css, 'utf8'), 'compiled');
  } finally {
    rmSync(cwd, { recursive: true, force: true });
  }
});
