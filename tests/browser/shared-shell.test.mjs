import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import test from 'node:test';
import { runInNewContext } from 'node:vm';

const read = path => readFileSync(path, 'utf8');

test('household navigation is rendered before scripts run', () => {
  const sidebar = read('assets/views/components/shell-links.html');
  const header = read('assets/views/components/app-navigation.html');
  assert.match(sidebar, /shell\.sidebar/);
  assert.match(header, /shell\.name/);
  assert.match(read('assets/views/components/account-menu.html'), /household_id/);
  assert.match(read('assets/views/layouts/base.html'), /sidebar_collapsed/);
});

test('startup does not fetch or replace server navigation', () => {
  const shortcuts = read('assets/static/js/mobile-shortcuts.js');
  assert.doesNotMatch(shortcuts, /DOMContentLoaded/);
  assert.doesNotMatch(read('assets/static/js/app-navigation.js'), /replaceChildren|app-navigation['"]/);
});

test('shell context is request owned and loaded at the render boundary', () => {
  const source = read('src/controllers/shell.rs');
  assert.match(source, /impl FromRequestParts<AppContext>/);
  assert.match(source, /impl ViewRenderer for ShellView/);
  assert.doesNotMatch(source, /task_local!|OnceLock|LazyLock/);
});

test('collapsed sidebar retains the brand accessible name independently of hidden text', () => {
  const brand = read('assets/views/components/app-sidebar.html').match(/<a class="app-sidebar-brand"[^>]*>/)?.[0];
  assert.match(brand, /aria-label="{{ shell.labels.navigation.brand }}"/);
});

test('mobile layout removes the desktop-only sidebar checkbox from keyboard focus', () => {
  const mobile = read('assets/static/css/shared-layout.css').split('@media (max-width:1023px) {')[1];
  assert.match(mobile, /\[data-sidebar-expanded\]\s*\{\s*display:none;\s*\}/);
});

test('icon rail hides shortcut hints by element even when search enhancement replaces classes', () => {
  const css = read('assets/static/css/shared-layout.css');
  assert.match(css, /:has\(\[data-sidebar-expanded\]:not\(:checked\)\) \.app-sidebar-search kbd\s*\{\s*display:none;\s*\}/);
});

function enhance({ sidebar = null, account = null, drawer = null, outside = [] } = {}) {
  const document = {
    body: { dataset: {}, classList: { add() {} } },
    querySelector(selector) { return selector === '[data-sidebar-expanded]' ? sidebar : selector === '[data-mobile-drawer]' ? drawer : null; },
    querySelectorAll(selector) { return selector === '[data-account-menu]' ? (account ? [account] : []) : outside; },
    addEventListener() {},
  };
  runInNewContext(read('assets/static/js/app-navigation.js'), {
    document, navigator: {}, window: { matchMedia() { return { addEventListener() {} }; } },
    sessionStorage: { getItem() { return null; } }, requestAnimationFrame(fn) { fn(); },
    Event: class { constructor(type) { this.type = type; } },
  });
  return document;
}

test('rail enhancement preserves server state and saves only a deliberate toggle', () => {
  const handlers = {};
  const sidebar = { checked: false, addEventListener(type, fn) { handlers[type] = fn; } };
  const document = enhance({ sidebar });
  assert.equal(sidebar.checked, false);
  assert.equal(document.cookie, undefined);
  sidebar.checked = true;
  handlers.change();
  assert.equal(document.body.dataset.sidebarCollapsed, 'false');
  assert.match(document.cookie, /medtracker.sidebar=expanded; Path=\/; Max-Age=31536000; SameSite=Lax/);
  sidebar.checked = false;
  handlers.change();
  assert.match(document.cookie, /medtracker.sidebar=collapsed/);
});

test('account enhancement keeps native content and supports Escape and arrow entry', () => {
  const handlers = {};
  let focused = '';
  const summary = { setAttribute() {}, focus() { focused = 'summary'; } };
  const panel = { id: 'app-account', querySelector() { return { focus() { focused = 'first-link'; } }; } };
  const account = {
    open: false,
    querySelector(selector) { return selector === 'summary' ? summary : panel; },
    addEventListener(type, fn) { handlers[type] = fn; },
  };
  enhance({ account });
  handlers.keydown({ key: 'ArrowDown', target: summary, preventDefault() {} });
  assert.equal(account.open, true);
  assert.equal(focused, 'first-link');
  focused = 'current-link';
  let prevented = false;
  handlers.keydown({ key: 'ArrowDown', target: panel.querySelector(), preventDefault() { prevented = true; } });
  assert.equal(focused, 'current-link');
  assert.equal(prevented, false);
  handlers.keydown({ key: 'Escape', preventDefault() {} });
  assert.equal(account.open, false);
  assert.equal(focused, 'summary');
});
