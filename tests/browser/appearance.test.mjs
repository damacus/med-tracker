import assert from 'node:assert/strict';
import { readFileSync, readdirSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const source = () => readFileSync('assets/static/js/appearance.js', 'utf8');
const classes = new Set();

function searchControls(platform, translated) {
  class Element {
    constructor(text = '') { this.text = text; this.attributes = {}; this.children = []; this.listeners = {}; }
    get textContent() { return this.text + this.children.map(child => child.textContent).join(''); }
    set textContent(text) { this.text = text; this.children = []; }
    setAttribute(name, value) { this.attributes[name] = value; }
    addEventListener(name, listener) { this.listeners[name] = listener; }
    append(...children) { this.children.push(...children); }
    querySelector(selector) { return this.children.find(child => child.tag === selector); }
  }
  const triggers = [new Element('Search'), new Element('Search')];
  for (const trigger of triggers) trigger.setAttribute('aria-label', 'Open global search');
  const fields = new Map();
  const panel = new Element();
  panel.dataset = { labelsReady: translated ? 'false' : 'true', household: 'synthetic' };
  panel.querySelector = selector => {
    if (!fields.has(selector)) fields.set(selector, new Element());
    return fields.get(selector);
  };
  panel.querySelectorAll = () => [];
  const document = {
    querySelector() { return panel; },
    querySelectorAll() { return triggers; },
    createElement(tag) { const node = new Element(); node.tag = tag; return node; },
    createTextNode(text) { return new Element(text); },
    addEventListener() {},
  };
  runInNewContext(readFileSync('assets/static/js/global-search.js', 'utf8'), {
    document, Element, navigator: { platform },
    fetch: async () => ({ ok: true, json: async () => ({ locale: 'es', labels: translated }) }),
  });
  return { triggers, panel };
}

for (const [platform, hint, shortcut] of [['MacIntel', '⌘K', 'Meta+K'], ['Win32', 'Ctrl+K', 'Control+K']]) {
  test(`search keeps descriptive names separate from ${platform} shortcut hints`, () => {
    for (const trigger of searchControls(platform).triggers) {
      assert.equal(trigger.attributes['aria-label'], 'Open global search');
      assert.equal(trigger.attributes['aria-keyshortcuts'], shortcut);
      assert.equal(trigger.querySelector('kbd').textContent, hint);
      assert.equal(trigger.querySelector('kbd').attributes['aria-hidden'], 'true');
    }
  });
}

test('search retains fetched descriptive names after shortcut hints refresh', async () => {
  const translated = { open: 'Abrir búsqueda global', open_short: 'Buscar', results: {}, types: {} };
  const { triggers, panel } = searchControls('MacIntel', translated);
  await new Promise(setImmediate);
  assert.equal(panel.dataset.labelsReady, 'true');
  for (const trigger of triggers) {
    assert.equal(trigger.attributes['aria-label'], translated.open);
    assert.equal(trigger.textContent, 'Buscar ⌘K');
    assert.equal(trigger.attributes['aria-keyshortcuts'], 'Meta+K');
    assert.equal(trigger.querySelector('kbd').attributes['aria-hidden'], 'true');
  }
});

test('navigation leaves standalone validation pages without a household drawer intact', async () => {
  const listeners = {};
  const header = { querySelector() { throw Error('standalone header must not be replaced'); } };
  const document = {
    querySelector(selector) { return selector === '.shell-header' ? header : null; },
    getElementById() { return {}; },
    addEventListener(name, listener) { listeners[name] = listener; },
  };
  runInNewContext(readFileSync('assets/static/js/app-navigation.js', 'utf8'), {
    document, window: {}, navigator: {}, sessionStorage: { getItem() { return null; } },
  });
  await Promise.resolve();
  assert.doesNotThrow(() => listeners['app-navigation']({ detail: { labels: {} } }));
});

test('login reuses the original Rails brand and benefit artwork', () => {
  const sprite = readFileSync('assets/static/auth-icons.svg', 'utf8');
  for (const file of [
    'rails/app/components/auth/mt_logo.rb',
    'rails/app/components/auth/benefit_icons/heart_check.rb',
    'rails/app/components/auth/benefit_icons/schedule_calendar.rb',
    'rails/app/components/auth/benefit_icons/progress_path_pin.rb',
    'rails/app/components/auth/benefit_icons/insights_search.rb',
  ]) {
    const reference = readFileSync(file, 'utf8');
    const paths = [...reference.matchAll(/\bd:\s*'([^']+)'/g)].map(match => match[1]);
    const constant = reference.match(/(?:HEART_PATH|PATH) = ([\s\S]+?)\n\n/);
    if (constant) paths.push([...constant[1].matchAll(/'([^']*)'/g)].map(match => match[1]).join(''));
    assert.ok(paths.length > 0, `reference paths missing: ${file}`);
    for (const path of paths) assert.ok(sprite.includes(`d="${path}"`), `original artwork changed: ${file}`);
    for (const match of reference.matchAll(/(?:fill|stop_color): '(#[A-Fa-f0-9]{6})'/g)) {
      assert.ok(sprite.includes(match[1]), `original artwork colour changed: ${file}`);
    }
  }
});

test('shared navigation follows the selected page language and otherwise keeps browser negotiation', async () => {
  for (const language of ['es', 'en', '', undefined]) {
    const listeners = {};
    let request;
    const document = {
      documentElement: { lang: language },
      addEventListener(name, listener) { listeners[name] = listener; },
    };
    runInNewContext(readFileSync('assets/static/js/mobile-shortcuts.js', 'utf8'), {
      document,
      location: { pathname: '/households/persistence-fixture/admin', origin: 'http://localhost' },
      navigator: { language: 'cy' },
      async fetch(url, options) { request = { url, options }; return { ok: false }; },
    });
    await listeners.DOMContentLoaded();
    assert.equal(request.url, '/households/persistence-fixture/profile/navigation');
    assert.equal(request.options.headers.Accept, 'application/json');
    assert.equal(request.options.cache, 'no-store');
    assert.equal(request.options.headers['Accept-Language'], language || undefined);
  }
});

test('dashboard reuses shared symbols without duplicating the profile sprite', () => {
  const symbols = file => [...readFileSync(file, 'utf8').matchAll(/<symbol id="([^"]+)"/g)].map(match => match[1]);
  const shared = new Set(symbols('assets/static/profile-icons.svg'));
  const duplicated = symbols('assets/static/dashboard-icons.svg').filter(id => shared.has(id));
  assert.deepEqual(duplicated, []);
});

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


test('browser template families share the application frame and keep document adapters separate', () => {
  const families = new Set(['administration', 'admin', 'people', 'person_carers', 'locations', 'dashboard', 'medications', 'dosage_options', 'treatments', 'health_events', 'dose_occurrences', 'reviews', 'reports', 'searches', 'profile', 'platform', 'auth', 'identity_onboarding', 'home', 'invitations', 'oauth']);
  const documents = new Set(['reports/pdf_base.html', 'oauth/form_post.html']);
  const seen = new Set();
  function visit(directory, prefix = '') {
    for (const entry of readdirSync(directory, { withFileTypes: true })) {
      const relative = `${prefix}${entry.name}`;
      if (entry.isDirectory()) { visit(`${directory}/${entry.name}`, `${relative}/`); continue; }
      const content = readFileSync(`${directory}/${entry.name}`, 'utf8');
      const parent = content.match(/\{% extends "([^"]+)" %\}/)?.[1];
      if (parent && !relative.startsWith('layouts/')) {
        assert.ok(families.has(relative.split('/')[0]), `unclassified browser family: ${relative}`);
        seen.add(relative.split('/')[0]);
        if (relative.startsWith('reports/') && relative !== 'reports/index.html') assert.equal(parent, 'reports/pdf_base.html');
        else assert.ok(['layouts/base.html', 'layouts/dashboard.html'].includes(parent), `screen bypasses shared frame: ${relative}`);
      }
      if (/<!doctype/i.test(content) && relative !== 'layouts/base.html') assert.ok(documents.has(relative), `unframed document: ${relative}`);
    }
  }
  visit('assets/views');
  assert.deepEqual([...seen].sort(), [...families].sort());
  assert.doesNotMatch(readFileSync('assets/views/reports/pdf_base.html', 'utf8'), /app-sidebar|app-navigation/);
  assert.doesNotMatch(readFileSync('assets/views/oauth/form_post.html', 'utf8'), /app-sidebar|app-navigation/);
});


test("shared route selection preserves chooser Inventory and the most specific authorised section", () => {
  const context = { document: { addEventListener() {} } };
  runInNewContext(readFileSync("assets/static/js/mobile-shortcuts.js", "utf8"), context);
  const root = "/households/synthetic";
  const choices = [
    { id: "dashboard", href: `${root}/dashboard` },
    { id: "inventory", href: `${root}/medications` },
    { id: "finder", href: `${root}/medications/finder` },
    { id: "people", href: `${root}/people` },
  ];
  for (const [path, expected] of [
    ["medications/1/edit", "inventory"],
    ["medications/finder", "finder"],
    ["add-medication", "inventory"],
    ["add-medication/step", "inventory"],
    ["dose-occurrences/1", "inventory"],
    ["people/1/treatments", "people"],
    ["add-medication-other", undefined],
  ]) assert.equal(context.navigationSection(`${root}/${path}`, choices), expected, path);
  assert.equal(context.navigationSection("/households/foreign/add-medication", choices), undefined);
  assert.equal(context.navigationSection(`${root}/add-medication`, choices.filter(item => item.id !== "inventory")), undefined);
});
