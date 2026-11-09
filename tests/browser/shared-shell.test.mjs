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
  assert.match(read('assets/views/components/household-selector.html'), /household_id/);
  assert.match(read('assets/views/layouts/base.html'), /sidebar_collapsed/);
});

test('profile keeps its navigation fallback only without the complete shared shell', () => {
  const profile = read('assets/views/profile/show.html');
  assert.match(profile, /{% if not shell %}\s*{% include "profile\/navigation\.html" %}\s*{% endif %}/);
});

test('profile session dates retain semantic timestamps and System Information is always open', () => {
  const security = read('assets/views/profile/security.html');
  assert.match(security, /<time datetime="{{ session.created.datetime }}"/);
  assert.match(security, /<time datetime="{{ session.expires.datetime }}"/);
  const profile = read('assets/views/profile/show.html');
  assert.match(profile, /<section[^>]*class="[^"]*profile-system-info/);
  assert.doesNotMatch(profile, /<summary[^>]*>(?:(?!<\/summary>)[\s\S])*version_info\.title/);
});

test('shared profile exposes protected security controls without another index or profile-form interception', () => {
  const security = read('assets/views/profile/security.html');
  for (const action of ['unlink_provider', 'remove_password', 'remove_passkey', 'close_account']) {
    assert.ok(security.includes(`value="${action}"`), `${action} is absent from the profile`);
  }
  assert.match(security, /action="\/account\/security\/totp\/backup-codes\?profile_flow=totp/);
  assert.match(security, /data-passkey-id=/);
  assert.match(security, /{% if security.zitadel %}<p[^>]*>{{ labels.profiles.security.provider_linked }}<\/p>{% endif %}/);
  assert.doesNotMatch(security, /data-profile-form|href="\/account\/security"/);
});

test('header has a direct profile link and household choices submit through the existing CSRF endpoint', () => {
  const header = read('assets/views/components/app-navigation.html');
  assert.doesNotMatch(header, /data-account-menu|account-menu\.html/);
  assert.match(header, /href="\/households\/{{ slug }}\/profile"/);
  const selector = read('assets/views/components/household-selector.html');
  assert.match(selector, /action="\/households\/select"/);
  assert.match(selector, /name="authenticity_token"/);
  assert.match(selector, /name="household_id" value="{{ household.id }}"/);
  assert.match(selector, /aria-current="true"/);
  assert.doesNotMatch(selector, /<select|Show|Apply/);
});

test('mobile header names stay accessible in household and standalone profile links', () => {
  const mobileName = read('assets/static/css/app.css').match(/@media \(max-width: 640px\) \{ \[data-account-name\] \{([^}]+)\}/)?.[1];
  assert.ok(mobileName, 'mobile name styling is missing');
  assert.doesNotMatch(mobileName, /display:\s*none/);
  assert.match(mobileName, /clip:\s*rect\(0,\s*0,\s*0,\s*0\)/);
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
  assert.doesNotMatch(brand, /aria-label=/);
  assert.match(read('assets/views/components/app-sidebar.html'), /class="app-sidebar-brand-text">{{ shell.labels.navigation.brand }}/);
  assert.match(read('assets/static/css/shared-layout.css'), /\.app-sidebar-brand-text\s*\{[^}]*clip:rect\(0,0,0,0\)/);
});

test('mobile layout removes the desktop-only sidebar checkbox from keyboard focus', () => {
  const mobile = read('assets/static/css/shared-layout.css').split('@media (max-width:1023px) {')[1];
  assert.match(mobile, /\[data-sidebar-expanded\]\s*\{\s*display:none;\s*\}/);
});

test('icon rail hides shortcut hints by element even when search enhancement replaces classes', () => {
  const css = read('assets/static/css/shared-layout.css');
  assert.match(css, /:has\(\[data-sidebar-expanded\]:not\(:checked\)\) \.app-sidebar-search kbd\s*\{\s*display:none;\s*\}/);
});

function enhance({ sidebar = null, household = null, drawer = null, outside = [] } = {}) {
  const document = {
    body: { dataset: {}, classList: { add() {} } },
    querySelector(selector) { return selector === '[data-sidebar-expanded]' ? sidebar : selector === '[data-mobile-drawer]' ? drawer : null; },
    querySelectorAll(selector) { return selector === '[data-household-selector]' ? (household ? [household] : []) : outside; },
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

test('household selector keeps native forms and supports Escape and arrow entry', () => {
  const handlers = {};
  let focused = '';
  const summary = { setAttribute() {}, focus() { focused = 'summary'; } };
  const panel = { id: 'sidebar-households', querySelector() { return { focus() { focused = 'first-link'; } }; } };
  const household = {
    open: false,
    querySelector(selector) { return selector === 'summary' ? summary : panel; },
    addEventListener(type, fn) { handlers[type] = fn; },
  };
  enhance({ household });
  handlers.keydown({ key: 'ArrowDown', target: summary, preventDefault() {} });
  assert.equal(household.open, true);
  assert.equal(focused, 'first-link');
  focused = 'current-link';
  let prevented = false;
  handlers.keydown({ key: 'ArrowDown', target: panel.querySelector(), preventDefault() { prevented = true; } });
  assert.equal(focused, 'current-link');
  assert.equal(prevented, false);
  let stopped = false;
  handlers.keydown({ key: 'Escape', preventDefault() {}, stopPropagation() { stopped = true; } });
  assert.equal(household.open, false);
  assert.equal(stopped, true);
  assert.equal(focused, 'summary');
});
