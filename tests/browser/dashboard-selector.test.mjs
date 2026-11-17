import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

const source = readFileSync('assets/static/js/dashboard.js', 'utf8');
const template = readFileSync('assets/views/dashboard/show.html', 'utf8');

function picker() {
  const listeners = {};
  const selector = { open: true, dataset: { selectedPerson: '1' }, contains() { return true; } };
  const document = { querySelectorAll() { return [selector]; }, addEventListener(name, fn) { listeners[name] = fn; } };
  runInNewContext(source, { document });
  return { selector, listeners };
}

test('people and Everyone are native links with no Show button or script-only fallback', () => {
  assert.match(template, /href="\?dashboard_person_id={{ person.id }}&amp;dashboard_grouping={{ grouping }}"/);
  assert.match(template, /href="\?dashboard_person_id=all&amp;dashboard_grouping={{ grouping }}"/);
  assert.doesNotMatch(template, /data-selector-fallback|role="radio"|data-dashboard-selector[^>]*hidden/);
  assert.doesNotMatch(template, /person_selector\.show|<select/);
  assert.doesNotMatch(source, /requestSubmit|location\.(?:reload|assign)|hidden\s*=/);
});

test('selecting another person uses ordinary document navigation without interception', () => {
  const { selector, listeners } = picker();
  const option = { dataset: { personId: 'all' }, closest() { return selector; } };
  let prevented = false;
  listeners.click({ target: { closest() { return option; } }, preventDefault() { prevented = true; } });
  assert.equal(prevented, false);
});

test('selecting the rendered current person closes the menu without a reload', () => {
  const { selector, listeners } = picker();
  const option = { dataset: { personId: '1' }, closest() { return selector; } };
  let prevented = false;
  listeners.click({ target: { closest() { return option; } }, preventDefault() { prevented = true; } });
  assert.equal(prevented, true);
  assert.equal(selector.open, false);
});

test('modified current-person clicks retain native new-tab behaviour', () => {
  const { selector, listeners } = picker();
  const option = { dataset: { personId: '1' }, closest() { return selector; } };
  let prevented = false;
  listeners.click({ ctrlKey: true, target: { closest() { return option; } }, preventDefault() { prevented = true; } });
  assert.equal(prevented, false);
});

test('Escape restores focus to the picker summary', () => {
  const { selector, listeners } = picker();
  let focused = false;
  selector.querySelector = () => ({ focus() { focused = true; } });
  listeners.keydown({ key: 'Escape', target: { closest() { return selector; } }, preventDefault() {} });
  assert.equal(selector.open, false);
  assert.equal(focused, true);
});

test('arrow keys move focus between native links and leave Enter to the browser', () => {
  const { selector, listeners } = picker();
  let focused = false;
  const first = { closest() { return selector; } };
  const last = { focus() { focused = true; } };
  selector.querySelectorAll = () => [first, last];
  listeners.keydown({ key: 'ArrowDown', target: { closest(query) { return query.startsWith('details') ? selector : first; } }, preventDefault() {} });
  assert.equal(focused, true);
});

test('Escape closes a dose actions disclosure and restores its summary focus', () => {
  const listeners = {};
  let focused = false;
  let prevented = false;
  const menu = { open: true, querySelector() { return { focus() { focused = true; } }; } };
  const document = { addEventListener(name, fn) { listeners[name] = fn; } };
  runInNewContext(source, { document });
  listeners.keydown({
    key: 'Escape',
    target: { closest(query) { return query.includes('dashboard-row-menu') ? menu : null; } },
    preventDefault() { prevented = true; },
  });
  assert.equal(menu.open, false);
  assert.equal(focused, true);
  assert.equal(prevented, true);
});

test('leaving a dose actions disclosure closes it without moving focus', () => {
  const listeners = {};
  const menu = { open: true, contains() { return false; } };
  const document = { addEventListener(name, fn) { listeners[name] = fn; } };
  runInNewContext(source, { document });
  listeners.focusout({
    target: { closest(query) { return query.includes('dashboard-row-menu') ? menu : null; } },
    relatedTarget: {},
  });
  assert.equal(menu.open, false);
});

function trayLayout(observed = true) {
  const listeners = {};
  const values = {};
  let trayHeight = 156;
  let resized;
  let changed;
  let focusedInside = false;
  const panel = { open: true, contains() { return focusedInside; } };
  const media = { matches: false, addEventListener(_, callback) { changed = callback; } };
  const tray = { getBoundingClientRect() { return { height: trayHeight }; } };
  const dock = { getBoundingClientRect() { return { height: 64 }; } };
  const root = {
    querySelector() { return tray; },
    querySelectorAll() { return [panel]; },
    style: { setProperty(name, value) { values[name] = value; } },
  };
  const document = {
    addEventListener(name, callback) { listeners[name] = callback; },
    querySelector(selector) { return selector === '#dashboard' ? root : dock; },
  };
  const window = {
    matchMedia() { return media; },
    addEventListener(name, callback) { listeners[`window:${name}`] = callback; },
  };
  class Observer {
    constructor(callback) { resized = callback; }
    observe() {}
  }
  runInNewContext(source, { document, window, ResizeObserver: observed ? Observer : undefined });
  listeners.DOMContentLoaded?.();
  return { values, panel, media, listeners, grow() { trayHeight = 248; resized?.(); }, change() { changed?.(); }, focusPanel() { focusedInside = true; document.activeElement = {}; } };
}

test('tray clearance follows wrapped content while preserving a manually opened disclosure', () => {
  const layout = trayLayout();
  assert.equal(layout.values['--dashboard-tray-height'], '156px');
  assert.equal(layout.values['--dashboard-dock-height'], '64px');
  assert.equal(layout.panel.open, false);
  layout.panel.open = true;
  layout.grow();
  assert.equal(layout.values['--dashboard-tray-height'], '248px');
  assert.equal(layout.panel.open, true);
});

test('responsive supporting panels follow desktop transitions with a resize fallback', () => {
  const layout = trayLayout(false);
  assert.equal(typeof layout.listeners['window:resize'], 'function');
  layout.media.matches = true;
  layout.change();
  assert.equal(layout.panel.open, true);
  layout.media.matches = false;
  layout.change();
  assert.equal(layout.panel.open, false);
});

test('crossing to mobile keeps a supporting panel open while it contains focus', () => {
  const layout = trayLayout();
  layout.media.matches = true;
  layout.change();
  layout.focusPanel();
  layout.media.matches = false;
  layout.change();
  assert.equal(layout.panel.open, true);
});
