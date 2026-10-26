import test from 'node:test';
import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

function boot(platform, userAgentPlatform) {
  class Element {
    constructor() { this.attributes = {}; this.style = {}; this.listeners = {}; this.children = []; }
    setAttribute(name, value) { this.attributes[name] = value; }
    addEventListener(name, listener) { this.listeners[name] = listener; }
    append(child) { this.children.push(child); }
    querySelector(selector) { return selector === 'kbd' ? this.children.find(child => child.tag === 'kbd') : new Element(); }
    querySelectorAll() { return []; }
    replaceChildren() { this.children = []; }
    focus() { this.focused = true; }
  }
  const input = new Element();
  const panel = new Element();
  panel.hidden = true;
  panel.dataset = {};
  panel.querySelector = selector => selector === '#global_search_query' ? input : new Element();
  const triggers = [new Element(), new Element()];
  const document = {
    activeElement: triggers[0],
    listeners: {},
    querySelector() { return panel; },
    querySelectorAll(selector) { return selector === '[data-global-search-trigger]' ? triggers : []; },
    createElement(tag) { const element = new Element(); element.tag = tag; return element; },
    addEventListener(name, listener) { this.listeners[name] = listener; },
  };
  runInNewContext(readFileSync('assets/static/js/global-search.js', 'utf8'), {
    document, navigator: { platform, userAgentData: userAgentPlatform ? { platform: userAgentPlatform } : undefined },
    Element, clearTimeout, setTimeout,
  });
  return { triggers, document, panel, input };
}

for (const [platform, userAgentPlatform, hint, shortcut] of [
  ['MacIntel', undefined, '⌘K', 'Meta+K'],
  ['Win32', undefined, 'Ctrl+K', 'Control+K'],
  ['Linux x86_64', undefined, 'Ctrl+K', 'Control+K'],
  ['Linux x86_64', 'macOS', '⌘K', 'Meta+K'],
]) {
  test(`search hints and accessible shortcut agree on ${userAgentPlatform || platform}`, () => {
    const { triggers, document, panel, input } = boot(platform, userAgentPlatform);
    for (const trigger of triggers) {
      assert.equal(trigger.querySelector('kbd')?.textContent, hint);
      assert.equal(trigger.attributes['aria-keyshortcuts'], shortcut);
      assert.equal(trigger.querySelector('kbd').attributes['aria-hidden'], 'true');
    }
    for (const modifier of ['ctrlKey', 'metaKey']) {
      panel.hidden = true;
      document.listeners.keydown({ key: 'k', [modifier]: true, preventDefault() {} });
      assert.equal(panel.hidden, false);
      assert.equal(input.focused, true);
    }
  });
}
