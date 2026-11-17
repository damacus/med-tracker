import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

function openDialog(url, links, id = 'record-dose-dialog', opener = null, beforeClose = () => {}) {
  const events = {};
  const closed = {};
  class Dialog {
    constructor() { this.id = id; }
    showModal() {}
    addEventListener(name, handler) { closed[name] = handler; }
    querySelector() { return null; }
  }
  const dialog = new Dialog();
  const state = { retained: true };
  const window = {
    location: new URL(url),
    history: {
      state,
      replaceState(value, _, href) {
        assert.equal(value, state);
        window.location = new URL(href, window.location);
      },
    },
  };
  const document = {
    addEventListener(name, handler) { events[name] = handler; },
    getElementById() { return null; },
    querySelector() { return opener; },
    querySelectorAll(selector) {
      return selector === 'dialog[data-dialog-auto-open]' ? [dialog] : links.map(link => ({
        getClientRects() { return [{}]; },
        closest() { return null; },
        ...link,
      }));
    },
  };
  runInNewContext(readFileSync('assets/static/js/interaction.js', 'utf8'), {
    document, window, URL, URLSearchParams, HTMLDialogElement: Dialog,
  });
  events.DOMContentLoaded();
  beforeClose();
  closed.close?.();
  return window.location;
}

const recording = 'record_medication_id=74001&record_source_type=person_medication&record_source_id=75001&record_person_id=73001';
const base = 'https://example.test/households/synthetic/dashboard';

test('dashboard cancellation chooses the visible occurrence action instead of a hidden desktop copy', () => {
  let focused = '';
  const links = [
    { href: `${base}?${recording}`, getClientRects() { return []; }, focus() { focused = 'hidden'; }, scrollIntoView() {} },
    { href: `${base}?${recording}`, focus() { focused = 'tray'; }, scrollIntoView() {} },
  ];
  openDialog(`${base}?${recording}`, links);
  assert.equal(focused, 'tray');
});

test('dashboard cancellation resolves the visible opener after a viewport change', () => {
  let mobile = true;
  let focused = '';
  const links = [
    { href: `${base}?${recording}`, getClientRects() { return mobile ? [] : [{}]; }, focus() { focused = 'desktop'; }, scrollIntoView() {} },
    { href: `${base}?${recording}`, getClientRects() { return mobile ? [{}] : []; }, focus() { focused = 'tray'; }, scrollIntoView() {} },
  ];
  openDialog(`${base}?${recording}`, links, 'record-dose-dialog', null, () => { mobile = false; });
  assert.equal(focused, 'desktop');
});

test('returning focus to the fixed dose tray does not scroll the timeline', () => {
  let scrolled = false;
  const link = { href: `${base}?${recording}`, closest() { return {}; }, focus() {}, scrollIntoView() { scrolled = true; } };
  openDialog(`${base}?${recording}`, [link]);
  assert.equal(scrolled, false);
});

test('a visible tray opener avoids layout reads for preceding timeline links', () => {
  let unrelatedReads = 0;
  let trayReads = 0;
  const unrelated = { href: `${base}?${recording.replace('75001', '75002')}`, getClientRects() { unrelatedReads++; return [{}]; }, focus() {}, scrollIntoView() {} };
  const tray = { href: `${base}?${recording}`, getClientRects() { trayReads++; return [{}]; }, closest() { return {}; }, focus() {}, scrollIntoView() {} };
  openDialog(`${base}?${recording}`, [unrelated, tray], 'record-dose-dialog', tray);
  assert.equal(unrelatedReads, 0);
  assert.equal(trayReads, 1);
});

test('dashboard cancellation restores the selected occurrence among doses from the same treatment', () => {
  let focused = '';
  const links = ['first', 'second'].map(key => ({
    href: `${base}?${recording}&record_occurrence_key=${key}&record_occurrence_etag=current`,
    focus() { focused = key; },
    scrollIntoView() {},
  }));
  const result = openDialog(`${base}?${recording}&record_occurrence_key=second&record_occurrence_etag=previous&dashboard_person_id=73001&tag=a&tag=b#routine`, links);
  assert.equal(focused, 'second');
  assert.equal(result.href, `${base}?dashboard_person_id=73001&tag=a&tag=b#routine`);
});

test('an as-needed action does not restore focus to a scheduled occurrence of the same treatment', () => {
  let focused = '';
  const links = [
    { href: `${base}?${recording}&record_occurrence_key=scheduled`, focus() { focused = 'scheduled'; }, scrollIntoView() {} },
    { href: `${base}?${recording}`, focus() { focused = 'as-needed'; }, scrollIntoView() {} },
  ];
  openDialog(`${base}?${recording}`, links);
  assert.equal(focused, 'as-needed');
});

test('dashboard cancellation focuses the matching action and preserves other URL state', () => {
  let focused = '';
  let scrolled = false;
  const links = [
    { href: `${base}?${recording.replace('75001', '75002')}`, focus() { focused = 'wrong'; } },
    { href: `${base}?${recording}`, focus() { focused = 'matching'; }, scrollIntoView(options) { scrolled = options.block === 'center'; } },
  ];
  const result = openDialog(`${base}?${recording}&dashboard_person_id=73001&dashboard_grouping=person&lang=es&tag=a&tag=b#routine`, links);
  assert.equal(focused, 'matching');
  assert.equal(scrolled, true);
  assert.equal(result.href, `${base}?dashboard_person_id=73001&dashboard_grouping=person&lang=es&tag=a&tag=b#routine`);
});

test('dashboard cancellation clears recording state even if the matching action disappeared', () => {
  const result = openDialog(`${base}?${recording}`, []);
  assert.equal(result.href, base);
});

test('other automatic dialogs retain their URL state', () => {
  const url = `${base}?${recording}&refill=true`;
  assert.equal(openDialog(url, [], 'refill-inventory-dialog').href, url);
});

test('same-page automatic dose dialogs still return focus to their opener', () => {
  let focused = false;
  const opener = { focus() { focused = true; } };
  const url = 'https://example.test/households/synthetic/medications/74001?lang=es';
  assert.equal(openDialog(url, [], 'record-dose-dialog', opener).href, url);
  assert.equal(focused, true);
});
