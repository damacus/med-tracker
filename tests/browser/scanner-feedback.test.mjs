import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';
import test from 'node:test';

function scanner(found, canAdjust = true) {
  let decode;
  let sounds = 0;
  const controls = {};
  function element() {
    return { children: [], addEventListener(name, handler) { this[name] = handler; }, focus() {}, querySelector() { return element(); }, append(...items) { this.children.push(...items); }, replaceChildren() { this.children = []; } };
  }
  for (const key of ['start', 'stop', 'status', 'results', 'cameraStatus']) controls[key] = element();
  const form = element();
  form.elements = { q: { value: '' } };
  const root = {
    dataset: { household: 'test', canAdjust: String(canAdjust) },
    closest() { return null; },
    querySelector(selector) {
      return { '[data-barcode-form]': form, '[data-lookup-status]': controls.status,
        '[data-lookup-results]': controls.results, '[data-camera-start]': controls.start,
        '[data-camera-stop]': controls.stop, '[data-camera-status]': controls.cameraStatus }[selector];
    },
    querySelectorAll() { return []; },
  };
  const window = {
    addEventListener() {},
    AudioContext: class {
      constructor() { this.currentTime = 0; this.destination = {}; }
      resume() { return Promise.resolve(); }
      createOscillator() { return { connect() {}, start() { sounds++; }, stop() {}, frequency: { setValueAtTime() {} } }; }
      createGain() { return { connect() {}, gain: { setValueAtTime() {}, exponentialRampToValueAtTime() {} } }; }
    },
    __barcodeScannerTestLibrary: { Html5Qrcode: class {
      static async getCameras() { return [{}]; }
      async start(_camera, _options, callback) { decode = callback; }
      async stop() {}
      clear() {}
    } },
  };
  const document = { querySelector() { return root; }, createElement: element, addEventListener() {} };
  runInNewContext(readFileSync('assets/static/js/medication-finder.js', 'utf8'), {
    window, document, URLSearchParams, AbortController,
    fetch: async () => ({ ok: true, json: async () => found }),
  });
  return { controls, form, read: async value => { await controls.start.click(); await decode(value); }, sounds: () => sounds };
}

test('a camera read immediately plays a sound', async () => {
  const subject = scanner({ results: [], permissions: {} });
  await subject.read('1234567890123');
  assert.equal(subject.sounds(), 1);
});

test('a barcode matching stock offers stock rather than another medicine', async () => {
  const subject = scanner({ matches: [{ id: 1, name: 'Medicine', location: 'Home', current_supply: '5' }], results: [{ display: 'Medicine' }], permissions: { can_create: true } });
  await subject.read('1234567890123');
  const links = subject.controls.results.children.flatMap(card => card.children).filter(item => item.href);
  assert.deepEqual(links.map(item => item.textContent), ['Add stock']);
  assert.match(links[0].href, /\/1\?refill=true$/);
});

test('a viewer is directed to a manager instead of being offered stock changes', async () => {
  const subject = scanner({ matches: [{ id: 1, name: 'Medicine', location: 'Home' }], results: [], permissions: { can_create: false } }, false);
  await subject.read('1234567890123');
  const contents = subject.controls.results.children.flatMap(card => card.children);
  assert.equal(contents.some(item => item.href), false);
  assert.ok(contents.some(item => item.textContent === 'Ask a household manager to add stock.'));
});

test('an unsupported code explains how to continue', async () => {
  const subject = scanner({ results: [], permissions: {} });
  await subject.read('12345');
  assert.match(subject.controls.status.textContent, /13- or 14-digit/);
});
