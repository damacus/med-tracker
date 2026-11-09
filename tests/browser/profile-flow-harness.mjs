import { readFileSync } from 'node:fs';
import { runInNewContext } from 'node:vm';

const camel = value => value.replace(/-([a-z])/g, (_, letter) => letter.toUpperCase());

export class FlowEvent {
  constructor(type, options = {}) { Object.assign(this, { type, bubbles: true, cancelable: true, button: 0, defaultPrevented: false }, options); }
  preventDefault() { if (this.cancelable) this.defaultPrevented = true; }
  stopPropagation() { this.stopped = true; }
}

class Element {
  constructor(tag, document) { this.tag = tag; this.document = document; this.dataset = {}; this.children = []; this.listeners = []; this.attributes = {}; this.text = ''; }
  get parentElement() { return this.parent; }
  get isConnected() { return this === this.document || !!this.parent?.isConnected; }
  get textContent() { return this.text + this.children.map(child => child.textContent).join(''); }
  set textContent(value) { this.replaceChildren(); this.text = value; }
  append(...nodes) { for (let node of nodes) { if (typeof node === 'string') node = this.document.createTextNode(node); node.remove(); node.parent = this; this.children.push(node); } }
  replaceChildren(...nodes) { this.children.slice().forEach(node => node.remove()); this.text = ''; this.append(...nodes); }
  replaceWith(node) { const parent = this.parent; const index = parent.children.indexOf(this); this.remove(); node.remove(); node.parent = parent; parent.children.splice(index, 0, node); }
  remove() { if (this.parent) { this.parent.children.splice(this.parent.children.indexOf(this), 1); this.parent = null; } }
  contains(node) { return this === node || this.children.some(child => child.contains(node)); }
  setAttribute(name, value) { this.attributes[name] = String(value); if (name.startsWith('data-')) this.dataset[camel(name.slice(5))] = String(value); }
  getAttribute(name) {
    if (name.startsWith('data-')) return this.dataset[camel(name.slice(5))] ?? null;
    if (name in this.attributes) return this.attributes[name];
    if (name === 'readonly') return this.readOnly ? '' : null;
    if (name === 'open') return this.open ? '' : null;
    return this[name] ?? null;
  }
  hasAttribute(name) { return this.getAttribute(name) !== null; }
  removeAttribute(name) { delete this.attributes[name]; if (name.startsWith('data-')) delete this.dataset[camel(name.slice(5))]; }
  matches(selector) {
    return selector.split(',').some(part => {
      const excluded = [...part.matchAll(/:not\(([^)]+)\)/g)].map(match => match[1]);
      part = part.trim().replace(/:not\([^)]+\)/g, '');
      if (excluded.some(value => this.matches(value))) return false;
      const tag = part.match(/^[a-z]+/)?.[0];
      if (tag && this.tag !== tag) return false;
      if ([...part.matchAll(/\.([\w-]+)/g)].some(match => !(this.className || '').split(' ').includes(match[1]))) return false;
      if ([...part.matchAll(/#([\w-]+)/g)].some(match => this.id !== match[1])) return false;
      return [...part.matchAll(/\[([^=\]]+)(?:=["']?([^"'\]]*)["']?)?\]/g)].every(match => match[2] === undefined ? this.hasAttribute(match[1]) : String(this.getAttribute(match[1])) === match[2]);
    });
  }
  closest(selector) { return this.matches(selector) ? this : this.parent?.closest(selector) || null; }
  querySelectorAll(selector) { return this.children.flatMap(child => [...(child.matches(selector) ? [child] : []), ...child.querySelectorAll(selector)]); }
  querySelector(selector) { return this.querySelectorAll(selector)[0] || null; }
  addEventListener(type, listener, capture = false) { this.listeners.push({ type, listener, capture: capture === true }); }
  dispatchEvent(event) {
    event.target = this;
    const path = [this];
    if (event.bubbles) while (path.at(-1).parent) path.push(path.at(-1).parent);
    for (const capture of [true, false]) for (const node of capture ? [...path].reverse() : path) {
      for (const entry of node.listeners) if (entry.type === event.type && entry.capture === capture) entry.listener(event);
      if (event.stopped) return !event.defaultPrevented;
    }
    return !event.defaultPrevented;
  }
  focus() { this.document.activeElement = this; }
}

export class Fields extends Array {
  constructor(form) {
    super();
    for (const input of form.querySelectorAll('input, textarea, select')) if (input.name && !input.disabled) this.push([input.name, input.value]);
  }
  append(name, value) { this.push([name, value]); }
  get(name) { return this.find(field => field[0] === name)?.[1]; }
}

export function flowHarness({ inline, throwOpen = false, queueClose = false, serverChrome = true } = {}) {
  const native = [];
  const closeEvents = [];
  let document;
  class Form extends Element {
    constructor() { super('form', document); this.method = 'post'; this.enctype = 'application/x-www-form-urlencoded'; }
    submit() { native.push(new URLSearchParams(new Fields(this))); }
  }
  document = new Element('#document');
  document.document = document;
  document.documentElement = { lang: 'en' };
  document.createElement = tag => {
    const element = tag === 'form' ? new Form() : new Element(tag, document);
    if (tag === 'dialog') {
      element.showModal = () => { if (throwOpen) throw Error('Opening failed'); element.open = true; };
      element.close = () => {
        element.open = false;
        const notify = () => element.dispatchEvent(new FlowEvent('close', { bubbles: false }));
        if (queueClose) closeEvents.push(notify); else notify();
      };
    }
    return element;
  };
  document.createTextNode = text => { const node = new Element('#text', document); node.text = text; return node; };
  document.body = document.createElement('body');
  document.append(document.body);
  const host = document.createElement('main');
  host.dataset.profileDialogHost = '';
  host.dataset.profileUrl = '/profile';
  document.body.append(host);
  if (serverChrome) {
    const dialog = document.createElement('dialog'); dialog.dataset.profileFlowDialog = '';
    const box = document.createElement('div'); box.className = 'profile-flow-box';
    const header = document.createElement('header');
    const heading = document.createElement('p'); heading.dataset.profileFlowHeading = '';
    const close = document.createElement('button'); close.dataset.profileFlowClose = ''; close.type = 'button';
    const status = document.createElement('p'); status.dataset.profileFlowStatus = ''; status.setAttribute('role', 'status');
    const content = document.createElement('div'); content.dataset.profileFlowContent = ''; content.className = 'profile-flow-content';
    header.append(heading, close); box.append(header, status, content); dialog.append(box);
    document.body.append(dialog);
  }
  const field = (name, value, tag = 'input', type = 'text') => Object.assign(document.createElement(tag), { name, value, type });
  const root = (kind, outcome = 'continue') => {
    const element = document.createElement('section');
    Object.assign(element.dataset, { profileFlowRoot: '', flowProfile: '/profile', flowKind: kind, flowOutcome: outcome, flowSecret: 'false' });
    const heading = document.createElement('h1'); heading.textContent = kind;
    element.append(heading);
    return element;
  };
  if (inline) host.append(inline({ document, root, field }));
  const window = { addEventListener() {}, alert() {} };
  const location = { origin: 'https://example.test', href: 'https://example.test/profile', assign(value) { this.href = value; } };
  const messages = [];
  const storage = new Map();
  const sessionStorage = { getItem: key => storage.get(key), setItem: (key, value) => storage.set(key, value), removeItem: key => storage.delete(key) };
  const active = { state: 'activated', postMessage(message, ports) {
    messages.push(message.type);
    ports[0].postMessage({ ok: true, protocol: 1, id: message.id || storage.get('medtracker.push.transition') });
  } };
  const navigator = { serviceWorker: { getRegistration: async () => ({ active, getNotifications: async () => [{ close() { messages.push('closed'); } }] }) } };
  class Channel {
    constructor() { this.port1 = { close() {} }; this.port2 = { postMessage: data => queueMicrotask(() => this.port1.onmessage({ data })) }; }
  }
  const globals = { document, window, location, navigator, FormData: Fields, CustomEvent: FlowEvent, HTMLFormElement: Form };
  const installNavigation = () => runInNewContext(readFileSync('assets/static/js/app-navigation.js', 'utf8'), { ...globals, URL, sessionStorage, MessageChannel: Channel, crypto: { randomUUID: () => 'synthetic-transition' }, setTimeout, clearTimeout, queueMicrotask });
  const environment = async callback => {
    const previous = Object.fromEntries(Object.keys(globals).map(key => [key, Object.getOwnPropertyDescriptor(globalThis, key)]));
    for (const [key, value] of Object.entries(globals)) Object.defineProperty(globalThis, key, { configurable: true, writable: true, value });
    try { return await callback(); } finally { for (const [key, descriptor] of Object.entries(previous)) if (descriptor) Object.defineProperty(globalThis, key, descriptor); else delete globalThis[key]; }
  };
  const flushCloseEvents = () => closeEvents.splice(0).forEach(notify => notify());
  return { document, window, location, host, field, root, native, messages, navigator, globals, environment, installNavigation, flushCloseEvents };
}

export const settle = () => new Promise(setImmediate);
