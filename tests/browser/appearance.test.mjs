import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const source = () => readFileSync('assets/static/js/appearance.js', 'utf8');
const classes = new Set();
function boot(saved, dark = false, allowPalette = true, throws = false) {
  classes.clear();
  const listeners = {};
  const root = { dataset: { allowPalette: String(allowPalette) }, classList: { toggle(name, value) { value ? classes.add(name) : classes.delete(name); }, add(name) { classes.add(name); }, remove(name) { classes.delete(name); }, [Symbol.iterator]() { return classes.values(); } } };
  const media = { matches: dark, addEventListener(name, fn) { listeners[name] = fn; } };
  const storage = new Map(Object.entries(saved));
  const document = { documentElement: root, querySelector() { return null; }, querySelectorAll() { return []; }, addEventListener(name, fn) { listeners[name] = fn; } };
  const window = { matchMedia() { return media; } };
  runInNewContext(source(), { document, window, localStorage: { getItem(key) { if (throws) throw Error('blocked'); return storage.get(key) ?? null; }, setItem(key, value) { if (throws) throw Error('blocked'); storage.set(key, value); } } });
  return { root, media, listeners, storage, window };
}

test('saved palette and system appearance apply before document readiness', () => {
  const state = boot({ 'med-tracker-theme': 'warm-earth', 'med-tracker-appearance': 'system' }, true);
  assert.equal(state.root.dataset.theme, 'warm-earth-dark');
  assert.equal(state.root.dataset.appearance, 'system');
  state.media.matches = false;
  state.listeners.change();
  assert.equal(state.root.dataset.theme, 'warm-earth-light');
  assert.equal(state.storage.get('med-tracker-theme'), 'warm-earth');
});

test('signed-out pages keep the saved palette but use neutral surfaces', () => {
  const state = boot({ 'med-tracker-theme': 'warm-earth', 'med-tracker-appearance': 'dark' }, false, false);
  assert.equal(state.root.dataset.theme, 'neutral-dark');
  assert.equal(state.storage.get('med-tracker-theme'), 'warm-earth');
});

test('storage restrictions and unknown values fall back without losing rendering', () => {
  assert.equal(boot({}, false, true, true).root.dataset.theme, 'default-light');
  assert.equal(boot({ 'med-tracker-theme': '<script>', 'med-tracker-appearance': 'bad' }).root.dataset.theme, 'default-light');
  assert.equal(boot({ 'med-tracker-theme': 'minimalist-monochrome' }).root.dataset.theme, 'minimalist-monochrome-light');
});

test('daisyUI export preserves application border colour separately from thickness', () => {
  const css = readFileSync('assets/static/css/themes.css', 'utf8');
  assert.match(css, /@plugin "daisyui\/theme"/);
  assert.match(css, /--border-color:/);
  assert.match(css, /--border: 1px/);
  for (const palette of ['default', 'serene-sage', 'modern-clinical', 'warm-earth', 'deep-lavender', 'forest-care', 'sunset-support', 'tech-indigo', 'soft-rose', 'minty-fresh', 'minimalist-monochrome', 'neutral']) {
    for (const appearance of ['light', 'dark']) assert.ok(css.includes(`name: "${palette}-${appearance}"`));
  }
});

test('existing local fonts remain unchanged and carry their family licences', () => {
  for (const family of readdirSync('rails/app/assets/fonts')) {
    for (const name of readdirSync(`rails/app/assets/fonts/${family}`)) {
      assert.deepEqual(readFileSync(`assets/static/fonts/${family}/${name}`), readFileSync(`rails/app/assets/fonts/${family}/${name}`));
    }
    assert.match(readFileSync(`assets/static/fonts/${family}/OFL.txt`, 'utf8'), /SIL OPEN FONT LICENSE Version 1\.1/);
  }
  assert.match(readFileSync('assets/static/fonts/geist/OFL.txt', 'utf8'), /SIL OPEN FONT LICENSE Version 1\.1/);
});
