import assert from 'node:assert/strict';
import test from 'node:test';
import { FlowEvent, flowHarness, settle } from './profile-flow-harness.mjs';

test('profile flow destinations are limited to local setting routes', async () => {
  const { localFlowUrl } = await import('../../assets/static/js/profile-dialogs.js');
  const base = new URL('https://example.test/households/one/profile');
  assert.equal(localFlowUrl('/account/security/email', base)?.pathname, '/account/security/email');
  assert.equal(localFlowUrl('https://other.test/account/security/email', base), null);
  assert.equal(localFlowUrl('/platform/users', base), null);
  assert.equal(localFlowUrl('/api/v1/households/1', base), null);
  assert.equal(localFlowUrl('/auth/zitadel', base), null);
});

test('flow requests reject late replies and never replay a mutation', async () => {
  const { FlowRequests } = await import('../../assets/static/js/profile-dialogs.js');
  const requests = new FlowRequests();
  const first = requests.begin();
  const second = requests.begin();
  assert.equal(first.signal.aborted, true);
  assert.equal(requests.current(first), false);
  assert.equal(requests.current(second), true);
  requests.cancel();
  assert.equal(second.signal.aborted, true);
  assert.equal(requests.current(second), false);
});

test('explicit invalid and one-time outcomes survive independently of HTTP status', async () => {
  const { flowRoot } = await import('../../assets/static/js/profile-dialogs.js');
  const profile = new URL('https://example.test/profile');
  const root = outcome => ({ dataset: { flowProfile: '/profile', flowKind: 'password', flowOutcome: outcome }, querySelector() { return null; } });
  assert.equal(flowRoot(root('invalid'), profile).dataset.flowOutcome, 'invalid');
  assert.equal(flowRoot(root('secret-result'), profile).dataset.flowOutcome, 'secret-result');
  assert.throws(() => flowRoot(root('unrecognised'), profile));
  const unknown = root('continue');
  unknown.dataset.flowKind = 'platform';
  assert.throws(() => flowRoot(unknown, profile));
  const foreign = root('continue');
  foreign.dataset.flowProfile = '/households/foreign/profile';
  assert.throws(() => flowRoot(foreign, profile));
  const executable = root('continue');
  executable.querySelector = () => ({});
  assert.throws(() => flowRoot(executable, profile));
});

test('plan action submitters and proof fields are sent exactly once', async () => {
  const { formPayload } = await import('../../assets/static/js/profile-dialogs.js');
  class Fields extends Map {
    constructor(form) { super(form.fields); }
    append(name, value) { assert.equal(this.has(name), false); this.set(name, value); }
  }
  const payload = formPayload({ fields: [['authenticity_token', 'synthetic'], ['clinical_revision', '7']] }, { name: 'action', value: 'pause' }, Fields);
  assert.equal(payload.get('action'), 'pause');
  assert.equal(payload.get('clinical_revision'), '7');
  assert.equal(payload.get('authenticity_token'), 'synthetic');
});

test('dismissal clears one-time material and password values without caching the body', async () => {
  const { clearSecrets } = await import('../../assets/static/js/profile-dialogs.js');
  const removed = [];
  const input = { value: 'synthetic-one-time-value', removeAttribute(name) { removed.push(name); } };
  let cleared = false;
  clearSecrets({ dataset: { flowSecret: 'true' }, querySelectorAll() { return [input]; }, replaceChildren() { cleared = true; } });
  assert.equal(input.value, '');
  assert.deepEqual(removed, ['value']);
  assert.equal(cleared, true);
});

test('a dialog initialisation failure restores the live inline form and leaves native entry links alone', async () => {
  const { mountProfileDialogs } = await import('../../assets/static/js/profile-dialogs.js');
  class Element {
    constructor(tag) { this.tag = tag; this.dataset = {}; this.children = []; }
    append(...nodes) { for (const node of nodes) { node.remove(); node.parent = this; this.children.push(node); } }
    replaceWith(node) { const parent = this.parent; const index = parent.children.indexOf(this); this.remove(); node.remove(); node.parent = parent; parent.children.splice(index, 0, node); }
    remove() { if (this.parent) { this.parent.children.splice(this.parent.children.indexOf(this), 1); this.parent = null; } }
    setAttribute() {}
    addEventListener() {}
    querySelectorAll(selector) { return this.children.flatMap(child => [...(selector === '[data-profile-flow-root]' && child.dataset.flowOutcome ? [child] : []), ...child.querySelectorAll(selector)]); }
    querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  }
  const host = new Element('main');
  host.dataset.profileUrl = '/profile';
  const inline = new Element('section');
  inline.dataset.flowOutcome = 'continue';
  inline.dataset.flowProfile = '/profile';
  host.append(inline);
  const body = new Element('body');
  body.append(host);
  const listeners = new Map();
  const document = {
    body,
    querySelector() { return host; },
    createElement(tag) { const element = new Element(tag); if (tag === 'dialog') element.showModal = () => { throw Error('Initialisation failed'); }; return element; },
    addEventListener(type, listener) { listeners.set(type, listener); },
  };
  const previous = { location: globalThis.location, window: globalThis.window };
  globalThis.location = { origin: 'https://example.test' };
  globalThis.window = { addEventListener() {} };
  try {
    mountProfileDialogs(document);
    assert.equal(host.querySelector('[data-profile-flow-root]'), inline);
    assert.notEqual(host.dataset.profileDialogsReady, 'true');
    let prevented = false;
    listeners.get('click')({ button: 0, preventDefault() { prevented = true; } });
    assert.equal(prevented, false);
  } finally {
    globalThis.location = previous.location;
    globalThis.window = previous.window;
  }
});

test('runtime opening failure retains native GET and password-bearing named POST actions', async () => {
  const { mountProfileDialogs } = await import('../../assets/static/js/profile-dialogs.js');
  for (const action of ['link', 'form']) {
    const state = flowHarness({ throwOpen: true });
    let fetched = 0;
    state.globals.fetch = async () => { fetched += 1; throw Error('Unexpected request'); };
    await state.environment(async () => {
      mountProfileDialogs(state.document);
      const element = state.document.createElement(action === 'link' ? 'a' : 'form');
      element.dataset.profileFlowEntry = 'operation';
      element.href = 'https://example.test/profile?panel=email';
      element.action = 'https://example.test/account/security/operation?profile_flow=operation';
      const password = state.field('password', 'synthetic-password', 'input', 'password');
      const submitter = Object.assign(state.document.createElement('button'), { name: 'action', value: 'remove_password', type: 'submit' });
      element.append(password, submitter);
      state.host.append(element);
      const event = new FlowEvent(action === 'link' ? 'click' : 'submit', { submitter });
      element.dispatchEvent(event);
      await settle();
      assert.equal(event.defaultPrevented, false, 'the original native action remains available');
      assert.equal(password.value, 'synthetic-password');
      assert.equal(submitter.name, 'action');
      assert.equal(submitter.value, 'remove_password');
      assert.equal(fetched, 0);
      assert.notEqual(state.host.dataset.profileDialogsReady, 'true');
    });
  }
});

test('plan notes and readonly clinical values survive close, reopen and an unrelated edit submission', async () => {
  const { mountProfileDialogs } = await import('../../assets/static/js/profile-dialogs.js');
  let form;
  let notes;
  let dose;
  let origin;
  const state = flowHarness({ inline: ({ document, root, field }) => {
    const plans = root('plans');
    form = document.createElement('form');
    form.action = 'https://example.test/profile/schedules/11111111-1111-1111-1111-111111111111?profile_flow=plans';
    notes = field('notes', 'Existing clinical notes must remain.', 'textarea');
    dose = field('dose_amount', '2');
    origin = field('origin_time_zone', 'Europe/London'); origin.readOnly = true;
    form.append(notes, dose, origin);
    plans.append(form);
    return plans;
  } });
  let posted;
  state.globals.fetch = async (url, options) => {
    posted = options.body;
    return { url: String(url), headers: new Headers({ 'content-type': 'text/html' }), text: async () => 'plans' };
  };
  state.globals.DOMParser = class { parseFromString() { return { querySelector: () => state.root('plans') }; } };
  await state.environment(async () => {
    mountProfileDialogs(state.document);
    state.document.querySelector('dialog').close();
    assert.equal(notes.value, 'Existing clinical notes must remain.');
    assert.equal(origin.value, 'Europe/London');
    state.host.querySelector('[data-profile-flow-reopen]').dispatchEvent(new FlowEvent('click'));
    dose.value = '3';
    form.dispatchEvent(new FlowEvent('submit'));
    await settle();
    assert.equal(posted.get('notes'), 'Existing clinical notes must remain.');
    assert.equal(posted.get('dose_amount'), '3');
    assert.equal(posted.get('origin_time_zone'), 'Europe/London');
  });
});

test('real navigation and profile modules guard one enhanced reset request and keep the document in place', async () => {
  const { mountProfileDialogs } = await import('../../assets/static/js/profile-dialogs.js');
  let form;
  const state = flowHarness({ inline: ({ document, root, field }) => {
    const reset = root('reset');
    form = document.createElement('form');
    form.action = 'https://example.test/reset-password?profile_flow=reset';
    form.append(field('email', 'synthetic@example.test'), field('authenticity_token', 'synthetic-csrf'));
    reset.append(form);
    return reset;
  } });
  let fetched = 0;
  const pending = state.root('reset');
  pending.append(state.document.createTextNode('Check your email'));
  state.globals.fetch = async (url, options) => {
    fetched += 1;
    assert.equal(state.messages.filter(message => message === 'auth-transition-begin').length, 1);
    assert.equal(options.body.get('email'), 'synthetic@example.test');
    assert.equal(options.body.get('authenticity_token'), 'synthetic-csrf');
    return { url: String(url), headers: new Headers({ 'content-type': 'text/html' }), text: async () => 'pending' };
  };
  state.globals.DOMParser = class { parseFromString() { return { querySelector: () => pending }; } };
  await state.environment(async () => {
    state.installNavigation();
    mountProfileDialogs(state.document);
    form.dispatchEvent(new FlowEvent('submit'));
    await settle();
    assert.equal(fetched, 1);
    assert.equal(state.native.length, 0);
    assert.equal(state.location.href, 'https://example.test/profile');
    assert.equal(state.document.querySelector('dialog').open, true);
    assert.equal(state.document.querySelector('dialog').contains(pending), true);
    assert.equal(state.messages.filter(message => message === 'auth-transition-end').length, 1);
    assert.ok(state.messages.includes('closed'));
  });
});

function webauthnHarness(mode) {
  const state = flowHarness();
  const root = state.root(mode === 'registration' ? 'passkey' : 'operation');
  const labels = state.document.createElement('span');
  labels.dataset.securityLabels = '';
  labels.dataset.labels = JSON.stringify({ operation_uncertain: 'The request may have completed. Check current security settings.', review_security: 'Review current security settings' });
  const form = state.document.createElement('form');
  const button = Object.assign(state.document.createElement('button'), { type: mode === 'registration' ? 'submit' : 'button' });
  const error = state.document.createElement('p'); error.hidden = true;
  if (mode === 'registration') { form.dataset.addPasskey = ''; error.dataset.registrationError = ''; }
  else { button.dataset.securityPasskey = ''; button.dataset.operation = 'synthetic-operation'; error.dataset.proofError = ''; }
  form.append(button);
  root.append(labels, form, error);
  state.host.append(root);
  state.globals.PublicKeyCredential = { parseCreationOptionsFromJSON: value => value, parseRequestOptionsFromJSON: value => value };
  state.navigator.credentials = { create: async () => ({ toJSON: () => ({ synthetic: true }) }), get: async () => ({ toJSON: () => ({ synthetic: true }) }) };
  return { state, root, form, button, error };
}

for (const mode of ['registration', 'proof']) {
  for (const failure of ['cancel', 'start', 'transport', 'decode', 'server', 'rejected', ...(mode === 'proof' ? ['shape'] : [])]) {
    test(`${mode} ${failure} distinguishes safe cancellation from an uncertain final operation`, async () => {
      const { mountAddPasskey } = await import('../../assets/static/js/add-passkey.js');
      const { mountSecurityProof } = await import('../../assets/static/js/security-proof.js');
      const { state, root, form, button, error } = webauthnHarness(mode);
      const final = [];
      const busy = [];
      root.addEventListener('security-flow-busy', event => busy.push(event.detail));
      const alternative = Object.assign(state.document.createElement('button'), { type: 'submit' });
      if (mode === 'proof') form.append(alternative);
      state.globals.fetch = async (url) => {
        if (String(url).includes('verify-registration') || String(url).endsWith('/passkey/confirm')) {
          final.push(url);
          if (failure === 'transport') throw Error('Lost final response');
          if (failure === 'server' || failure === 'rejected') return { ok: false, status: failure === 'server' ? 503 : 401 };
          if (failure === 'shape') return { ok: true, status: 200, json: async () => ({}) };
          return { ok: true, status: 200, json: async () => { throw SyntaxError('Unreadable final response'); } };
        }
        if (failure === 'start') throw Error('Challenge unavailable');
        return { ok: true, json: async () => ({ challenge: 'synthetic' }) };
      };
      if (failure === 'cancel') state.navigator.credentials[mode === 'registration' ? 'create' : 'get'] = async () => { throw new DOMException('Cancelled', 'NotAllowedError'); };
      await state.environment(async () => {
        if (mode === 'registration') mountAddPasskey(root); else mountSecurityProof(root);
        (mode === 'registration' ? form : button).dispatchEvent(new FlowEvent(mode === 'registration' ? 'submit' : 'click'));
        await settle();
        const uncertain = ['transport', 'decode', 'server', 'shape'].includes(failure);
        assert.equal(button.disabled, uncertain);
        assert.equal(final.length, ['cancel', 'start'].includes(failure) ? 0 : 1);
        assert.deepEqual(busy, [true, false]);
        assert.equal(error.hidden, false);
        if (uncertain) {
          if (mode === 'proof') assert.equal(alternative.disabled, true);
          assert.match(error.textContent, /may have completed/);
          const recovery = error.querySelector('a');
          assert.equal(recovery.href, '/profile#security');
          assert.equal(recovery.hasAttribute('data-profile-flow-fallback'), true);
        }
      });
    });
  }
}

test('reset notification-guard failure sends nothing and keeps the enhanced form visible', async () => {
  const { mountProfileDialogs } = await import('../../assets/static/js/profile-dialogs.js');
  let form;
  const state = flowHarness({ inline: ({ document, root, field }) => {
    const reset = root('reset');
    form = document.createElement('form');
    form.action = 'https://example.test/reset-password?profile_flow=reset';
    form.append(field('email', 'synthetic@example.test'));
    reset.append(form);
    return reset;
  } });
  let registrationCalls = 0;
  const registration = state.navigator.serviceWorker.getRegistration;
  state.navigator.serviceWorker.getRegistration = async () => { if (++registrationCalls > 1) throw Error('Worker unavailable'); return registration(); };
  let fetched = 0;
  state.globals.fetch = async () => { fetched += 1; };
  await state.environment(async () => {
    state.installNavigation();
    mountProfileDialogs(state.document);
    form.dispatchEvent(new FlowEvent('submit'));
    await settle();
    assert.equal(fetched, 0);
    assert.equal(state.native.length, 0);
    const dialog = state.document.querySelector('dialog');
    assert.equal(dialog.open, true);
    assert.equal(dialog.contains(form), true);
    assert.equal(dialog.getAttribute('aria-busy'), 'false');
    assert.ok(dialog.querySelector('[role=status]').textContent);
  });
});

for (const enhancement of ['unavailable', 'runtime-open-failure']) {
  test(`reset ${enhancement} keeps native notification guarding and its named submitter`, async () => {
    const { mountProfileDialogs } = await import('../../assets/static/js/profile-dialogs.js');
    const state = flowHarness({ throwOpen: true });
    const form = state.document.createElement('form');
    form.action = 'https://example.test/reset-password?profile_flow=reset';
    form.dataset.profileFlowEntry = 'reset';
    const email = state.field('email', 'synthetic@example.test');
    const submitter = Object.assign(state.document.createElement('button'), { name: 'intent', value: 'request_reset', type: 'submit' });
    form.append(email, submitter);
    state.host.append(form);
    let fetched = 0;
    state.globals.fetch = async () => { fetched += 1; };
    await state.environment(async () => {
      state.installNavigation();
      if (enhancement !== 'unavailable') mountProfileDialogs(state.document);
      form.dispatchEvent(new FlowEvent('submit', { submitter }));
      await settle();
      assert.equal(fetched, 0);
      assert.equal(state.native.length, 1);
      assert.equal(state.native[0].get('email'), 'synthetic@example.test');
      assert.deepEqual(state.native[0].getAll('intent'), ['request_reset']);
      assert.equal(email.value, 'synthetic@example.test');
      assert.equal(state.messages.filter(message => message === 'auth-transition-begin').length, 1);
      assert.ok(state.messages.includes('closed'));
    });
  });
}
