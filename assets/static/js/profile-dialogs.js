const assetQuery = new URL(import.meta.url).search;
const [{ mountAddPasskey }, { mountSecurityProof }] = await Promise.all([
  import('./add-passkey.js' + assetQuery), import('./security-proof.js' + assetQuery),
]);

const localPaths = /^(?:\/account\/security\/(?:email(?:\/pending)?|password(?:\/(?:confirm|email))?|totp(?:\/(?:finish|disable(?:\/confirm)?|backup-codes))?|passkey|keys(?:\/revoke)?|provider\/(?:link|confirm)|operation|session\/revoke)|\/account\/support(?:\/(?:approve|end))?|\/profile\/(?:history|schedules(?:\/[a-f0-9-]+)?)|\/reset-password(?:\/confirm)?)$/;

export function localFlowUrl(value, profile) {
  let url;
  try { url = new URL(value, profile); } catch { return null; }
  if (url.origin !== profile.origin || url.username || url.password) return null;
  if (url.pathname === profile.pathname || url.pathname === '/profile' || url.pathname === '/account/security') return url;
  return localPaths.test(url.pathname) ? url : null;
}

export class FlowRequests {
  begin() {
    this.cancel();
    this.active = new AbortController();
    return this.active;
  }
  current(request) { return this.active === request && !request.signal.aborted; }
  cancel() { this.active?.abort(); this.active = null; }
}

export function flowRoot(root, profile) {
  if (!root || root.dataset.flowProfile !== profile.pathname || root.querySelector('script') || !['email', 'password', 'reset', 'totp', 'passkey', 'keys', 'provider', 'operation', 'support', 'history', 'plans'].includes(root.dataset.flowKind) || !['continue', 'invalid', 'complete', 'secret-result'].includes(root.dataset.flowOutcome)) throw Error('Unexpected profile flow');
  return root;
}

export function formPayload(form, submitter, FormDataType = FormData) {
  const body = new FormDataType(form);
  if (submitter?.name) body.append(submitter.name, submitter.value);
  return body;
}

export function clearSecrets(root) {
  const secret = root?.dataset.flowSecret === 'true';
  root?.querySelectorAll(secret ? 'input, textarea' : 'input[type="password"], input[data-profile-secret], textarea[data-profile-secret]').forEach(input => { input.value = ''; input.removeAttribute('value'); });
  if (secret) root.replaceChildren();
}

export function mountProfileDialogs(document) {
  const host = document.querySelector('[data-profile-dialog-host]');
  if (!host || host.dataset.profileDialogsReady === 'true') return;
  const profile = new URL(host.dataset.profileUrl, location.origin);
  const labels = host.dataset;
  const requests = new FlowRequests();
  let ready = false;
  let root;
  let placeholder;
  let opener;
  let busy = false;
  let active = false;
  let kind;
  const dialog = document.querySelector('[data-profile-flow-dialog]');
  if (!dialog || typeof dialog.showModal !== 'function') return;
  const heading = dialog.querySelector('[data-profile-flow-heading]');
  const closeButton = dialog.querySelector('[data-profile-flow-close]');
  const status = dialog.querySelector('[data-profile-flow-status]');
  const content = dialog.querySelector('[data-profile-flow-content]');
  if (!heading || !closeButton || !status || !content) return;

  const setBusy = value => { busy = value; closeButton.disabled = value; dialog.setAttribute('aria-busy', String(value)); };
  const restoreFocus = target => {
    if (!target) return;
    if (!target.matches('a[href], button, input, select, textarea, [tabindex]')) target.tabIndex = -1;
    target.focus();
  };
  const teardown = () => {
    if (!active) return null;
    active = false;
    requests.cancel();
    clearSecrets(root);
    if (root && root.dataset.flowSecret !== 'true') {
      placeholder.querySelector('summary').textContent = heading.textContent;
      placeholder.append(root);
      placeholder.open = false;
    } else { root?.remove(); placeholder?.remove(); }
    root = null;
    placeholder = null;
    status.replaceChildren();
    setBusy(false);
    const target = opener?.isConnected ? opener : host.querySelector('[data-profile-tab][aria-selected="true"], h1');
    opener = null;
    return target;
  };
  const close = () => {
    if (busy || !dialog.open) return;
    const target = teardown();
    dialog.close();
    restoreFocus(target);
  };
  const fail = (mutation, fallback) => {
    status.textContent = mutation ? (labels.flowUncertain || 'The request may have completed. Check the current setting before trying again. A lost one-time key cannot be displayed again.') : (labels.flowFailed || 'The setting could not be loaded. Try again or open it inline.');
    if (mutation) {
      clearSecrets(root);
      root?.querySelectorAll('button[type="submit"], input[type="submit"]').forEach(button => { button.disabled = true; });
    } else if (fallback) {
      const link = document.createElement('a');
      link.href = fallback;
      link.textContent = labels.flowInline || 'Open setting inline';
      link.className = 'link';
      link.dataset.profileFlowFallback = '';
      status.append(document.createTextNode(' '), link);
    }
    status.tabIndex = -1;
    status.focus();
  };
  const mount = next => {
    flowRoot(next, profile);
    if (root && root !== next) { clearSecrets(root); root.remove(); }
    root = next;
    kind = root.dataset.flowKind;
    content.replaceChildren(root);
    heading.textContent = root.querySelector('h1, h2')?.textContent || labels.flowTitle || 'Profile setting';
    const refresh = root.querySelector('template[data-profile-security-refresh]')?.content.firstElementChild;
    if (refresh) host.querySelector('#profile-security-content')?.replaceWith(refresh);
    mountAddPasskey(root);
    mountSecurityProof(root);
    status.textContent = '';
    root.querySelector('[role="alert"], input:not([type="hidden"]):not([readonly]), button, a[href]')?.focus();
  };
  const slot = () => {
    const details = document.createElement('details');
    details.dataset.profileFlowInline = '';
    const summary = document.createElement('summary');
    summary.dataset.profileFlowReopen = '';
    summary.textContent = heading.textContent || labels.flowTitle || 'Profile setting';
    details.append(summary);
    return details;
  };
  const show = target => {
    if (!dialog.open) {
      teardown();
      dialog.showModal();
      active = true;
      opener = target;
      host.querySelectorAll('[data-profile-flow-root]').forEach(previous => { if (previous !== root) { clearSecrets(previous); (previous.closest('[data-profile-flow-inline]') || previous).remove(); } });
      if (!placeholder) { placeholder = slot(); host.append(placeholder); }
    }
  };
  const nativeFallback = () => {
    ready = false;
    active = false;
    delete host.dataset.profileDialogsReady;
    requests.cancel();
    if (root && placeholder) placeholder.replaceWith(root);
    else placeholder?.remove();
    placeholder = null;
    dialog.remove();
  };
  const destination = value => {
    const url = localFlowUrl(value, profile);
    if (!url) throw Error('Unexpected profile destination');
    if (url.pathname !== profile.pathname && !url.searchParams.has('profile_done')) {
      url.searchParams.set('profile_flow', kind || 'operation');
      const household = profile.pathname.match(/^\/households\/([^/]+)\/profile$/)?.[1];
      if (household) url.searchParams.set('profile_household', household);
    }
    return url;
  };
  const send = async (url, options = {}) => {
    const mutation = (options.method || 'GET').toUpperCase() !== 'GET';
    const request = requests.begin();
    setBusy(mutation);
    status.textContent = labels.flowLoading || 'Loading setting…';
    let sent = false;
    let guarded = false;
    let presented = false;
    try {
      if (mutation && url.pathname === '/reset-password') {
        await window.medTrackerBeginPushTransition();
        guarded = true;
        if (!requests.current(request)) return;
      }
      sent = true;
      const response = await fetch(url, { ...options, signal: request.signal, credentials: 'same-origin', cache: 'no-store', headers: { Accept: 'text/vnd.medtracker.profile-flow+html', 'Accept-Language': document.documentElement.lang, ...options.headers } });
      if (!requests.current(request)) return;
      const final = new URL(response.url);
      if (final.origin !== profile.origin) throw Error('Unexpected profile response');
      if (final.pathname === '/login') { clearSecrets(root); location.assign(final.href); return; }
      if (!localFlowUrl(final.href, profile) || !response.headers.get('content-type')?.includes('text/html')) throw Error('Unexpected profile response');
      const parsed = new DOMParser().parseFromString(await response.text(), 'text/html');
      const next = parsed.querySelector('[data-profile-flow-root]');
      if (!requests.current(request)) return;
      flowRoot(next, profile);
      mount(next);
      presented = true;
    } catch (error) {
      if (requests.current(request) && error.name !== 'AbortError') fail(mutation && sent, mutation && sent ? null : url);
    } finally {
      if (guarded) {
        try { await window.medTrackerEndPushTransition(); }
        catch { if (requests.current(request) && (!sent || presented)) fail(false, url); }
      }
      if (requests.current(request)) setBusy(false);
    }
  };
  try {
    closeButton.addEventListener('click', close);
    dialog.addEventListener('cancel', event => { event.preventDefault(); close(); });
    dialog.addEventListener('close', () => {
      if (!dialog.open) restoreFocus(teardown());
    });
    document.addEventListener('click', event => {
      if (!ready || event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
      const reopen = event.target.closest('[data-profile-flow-reopen]');
      if (reopen && host.contains(reopen)) {
        if (busy) { event.preventDefault(); return; }
        const inline = reopen.parentElement.querySelector('[data-profile-flow-root]');
        if (!inline) return;
        root = inline;
        placeholder = reopen.parentElement;
        try {
          show(reopen);
          mount(inline);
          event.preventDefault();
        } catch { nativeFallback(); }
        return;
      }
      const anchor = event.target.closest('a[href]');
      if (!anchor || anchor.hasAttribute('data-profile-flow-fallback') || anchor.target === '_blank') return;
      const inside = root?.contains(anchor);
      if (!inside && !anchor.hasAttribute('data-profile-flow-entry')) return;
      if (busy) { event.preventDefault(); return; }
      const url = localFlowUrl(anchor.href, profile);
      if (!url) return;
      if (anchor.hasAttribute('data-profile-flow-done') || inside && url.pathname === profile.pathname && !url.searchParams.has('panel')) { event.preventDefault(); close(); return; }
      kind = anchor.dataset.profileFlowEntry || kind;
      try {
        const target = destination(url.href);
        show(anchor);
        event.preventDefault();
        void send(target);
      } catch { nativeFallback(); }
    });
    const submit = (event, submitter = event.submitter) => {
      if (!ready || event.defaultPrevented) return;
      const form = event.target;
      if (form.hasAttribute('data-profile-external') || form.hasAttribute('data-add-passkey') || form.method === 'dialog') return;
      if (!root?.contains(form) && !form.hasAttribute('data-profile-flow-entry')) return;
      const url = localFlowUrl(form.action, profile);
      if (!url) return;
      if (url.pathname === '/reset-password' && typeof window.medTrackerBeginPushTransition !== 'function') return;
      if (busy) { event.preventDefault(); return; }
      kind = form.dataset.profileFlowEntry || kind;
      let target;
      let options;
      try {
        target = destination(url.href);
        show(submitter || form);
        const body = formPayload(form, submitter);
        options = { method: form.method.toUpperCase(), body: form.enctype === 'multipart/form-data' ? body : new URLSearchParams(body) };
      } catch { nativeFallback(); return; }
      event.preventDefault();
      clearSecrets(form);
      void send(target, options);
    };
    document.addEventListener('submit', submit);
    document.addEventListener('profile-flow-auth-submit', event => submit(event, event.detail?.submitter));
    document.addEventListener('security-flow-busy', event => { if (root && (event.target === document || root.contains(event.target))) setBusy(event.detail === true); });
    document.addEventListener('security-flow-navigate', event => {
      if (!root || event.target !== document && !root.contains(event.target)) return;
      const url = localFlowUrl(event.detail, profile);
      if (!url) return;
      event.preventDefault();
      setBusy(false);
      const destinationUrl = url.pathname === '/account/security' ? new URL(`${profile.pathname}?profile_done=${kind}`, profile) : destination(url.href);
      void send(destinationUrl);
    });
    document.addEventListener('security-flow-result', event => { if (root?.contains(event.target) || event.target === root) { setBusy(false); status.textContent = ''; } });
    window.addEventListener('pagehide', () => { requests.cancel(); clearSecrets(root); setBusy(false); });
    ready = true;
    host.dataset.profileDialogsReady = 'true';
    const inline = host.querySelector('[data-profile-flow-root]');
    if (inline) {
      root = inline;
      placeholder = slot();
      inline.replaceWith(placeholder);
      show(host.querySelector('h1'));
      mount(inline);
    }
  } catch {
    nativeFallback();
  }
}

if (typeof document !== 'undefined') mountProfileDialogs(document);
