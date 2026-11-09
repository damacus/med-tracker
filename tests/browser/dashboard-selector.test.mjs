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
