import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

function fixture({ native = true, enhanced = true } = {}) {
  let document;
  const queued = [];
  class Element {
    constructor() { this.attributes = new Map(); this.listeners = new Map(); this.textContent = ''; }
    setAttribute(name, value) { this.attributes.set(name, String(value)); }
    getAttribute(name) { return this.attributes.get(name) ?? null; }
    hasAttribute(name) { return this.attributes.has(name); }
    removeAttribute(name) { this.attributes.delete(name); }
    addEventListener(name, listener) { this.listeners.set(name, [...(this.listeners.get(name) || []), listener]); }
    dispatch(name, extra = {}) {
      for (const listener of this.listeners.get(name) || []) listener({ preventDefault() {}, currentTarget: this, ...extra });
    }
    focus(options) { document.activeElement = this; this.focusOptions = options; }
    getClientRects() { return [{}]; }
    querySelector() { return null; }
    querySelectorAll() { return []; }
    closest() { return null; }
    append() {}
    replaceChildren() {}
  }
  const search = new Element();
  search.textContent = 'Search';
  search.setAttribute('aria-label', 'Open global search');
  const hint = new Element();
  search.querySelector = selector => selector === 'kbd' ? hint : null;
  const summary = new Element();
  summary.setAttribute('aria-expanded', 'false');
  const disclosure = new Element();
  disclosure.open = false;
  disclosure.querySelector = selector => selector.includes('summary') ? summary : null;
  summary.click = () => { disclosure.open = !disclosure.open; };
  const navigation = new Element();
  navigation.id = 'sidebar-households';
  navigation.closest = selector => native && selector === 'details' ? disclosure : null;
  let navigationCloses = 0;
  if (enhanced || !native) navigation.close = () => {
    navigationCloses += 1;
    disclosure.open = false;
    navigation.removeAttribute('open');
    navigation.dispatch('close');
  };
  const panel = new Element();
  panel.dataset = { labelsReady: 'true', household: 'synthetic-household' };
  panel.open = false;
  panel.showModal = () => { panel.open = true; };
  panel.close = () => { panel.open = false; queued.push(() => panel.dispatch('close')); };
  const input = new Element();
  const results = new Element();
  const status = new Element();
  const form = new Element();
  const close = new Element();
  panel.querySelector = selector => ({ '#global_search_query': input, '[data-global-search-results]': results, '[data-global-search-status]': status, '[data-global-search-form]': form })[selector];
  panel.querySelectorAll = () => [close];
  const events = new Element();
  document = {
    addEventListener: events.addEventListener.bind(events),
    dispatch(name, extra) { events.dispatch(name, { ...extra, currentTarget: document }); },
  };
  document.querySelector = () => panel;
  document.querySelectorAll = selector => {
    if (selector === '[data-global-search-trigger]') return [search];
    if (selector === '.app-navigation-panel') return [navigation];
    if (selector === '.app-navigation-panel[open]') return navigation.hasAttribute('open') ? [navigation] : [];
    if (selector.startsWith('[aria-controls=')) return selector.includes('aria-expanded="true"') && summary.getAttribute('aria-expanded') !== 'true' ? [] : [summary];
    return [];
  };
  runInNewContext(readFileSync('assets/static/js/global-search.js', 'utf8'), {
    document, Element, navigator: { platform: 'Linux' }, setTimeout, clearTimeout,
  });
  return {
    search, summary, disclosure, navigation, panel, input, document,
    openSearch() { document.dispatch('keydown', { key: 'k', ctrlKey: true, currentTarget: document }); },
    escapeSearch() { panel.dispatch('cancel'); },
    deliverQueuedClose() { const callback = queued.shift(); assert.ok(callback, 'search close event was not queued'); callback(); },
    navigationCloses() { return navigationCloses; },
  };
}

test('Ctrl+K closes a native household disclosure before its queued toggle updates mirrored state', () => {
  const ui = fixture();
  ui.summary.click();
  assert.equal(ui.navigation.hasAttribute('open'), false);
  assert.equal(ui.summary.getAttribute('aria-expanded'), 'false');
  ui.openSearch();
  assert.equal(ui.disclosure.open, false);
  assert.equal(ui.navigationCloses(), 1);
  assert.equal(ui.panel.open, true);
  assert.equal(ui.document.activeElement, ui.input);
  ui.escapeSearch();
  assert.equal(ui.document.activeElement, ui.summary);
  assert.equal(ui.summary.focusOptions.preventScroll, true);
});

test('a queued search close cannot close reopened search or move focus away from its input', () => {
  const ui = fixture();
  ui.search.focus();
  ui.openSearch();
  ui.escapeSearch();
  assert.equal(ui.panel.open, false);
  assert.equal(ui.document.activeElement, ui.search);
  ui.summary.click();
  ui.openSearch();
  assert.equal(ui.disclosure.open, false);
  ui.deliverQueuedClose();
  assert.equal(ui.panel.open, true);
  assert.equal(ui.document.activeElement, ui.input);
  ui.escapeSearch();
  assert.equal(ui.document.activeElement, ui.summary);
});

test('a closed native disclosure ignores a stale mirrored open marker', () => {
  const ui = fixture();
  ui.navigation.setAttribute('open', '');
  ui.summary.setAttribute('aria-expanded', 'true');
  ui.search.focus();
  ui.openSearch();
  assert.equal(ui.navigationCloses(), 0);
  ui.escapeSearch();
  assert.equal(ui.document.activeElement, ui.search);
});

test('legacy open navigation panels retain their close and return-focus behaviour', () => {
  const ui = fixture({ native: false });
  ui.navigation.setAttribute('open', '');
  ui.summary.setAttribute('aria-expanded', 'true');
  ui.openSearch();
  assert.equal(ui.navigationCloses(), 1);
  ui.escapeSearch();
  assert.equal(ui.document.activeElement, ui.summary);
});

test('search closes native navigation even when the optional navigation enhancement is unavailable', () => {
  const ui = fixture({ enhanced: false });
  ui.summary.click();
  ui.openSearch();
  assert.equal(ui.disclosure.open, false);
  assert.equal(ui.panel.open, true);
  ui.escapeSearch();
  assert.equal(ui.document.activeElement, ui.summary);
});
