(() => {
  const dialog = document.getElementById('profile-security-flow');
  if (!dialog) return;
  const content = dialog.querySelector('[data-security-flow-content]');
  const scripts = new Set(['/static/js/security-proof.js', '/static/js/add-passkey.js', '/static/js/recovery-result.js']);
  let opener;
  let pending = false;
  let credentialPending = false;
  let generation = 0;
  const isSecurity = url => url.origin === location.origin && (url.pathname === '/account/security' || url.pathname.startsWith('/account/security/'));
  function rememberReturn(minutes) {
    try { sessionStorage.setItem('medtracker.profile.security.return', JSON.stringify({ path: `${location.pathname}#security`, expires: Date.now() + minutes * 60 * 1000 })); } catch {}
  }
  function error() {
    let message = content.querySelector('[data-security-flow-error]');
    if (!message) {
      message = document.createElement('p');
      message.dataset.securityFlowError = '';
      message.setAttribute('role', 'alert');
      message.tabIndex = -1;
      content.prepend(message);
    }
    message.textContent = document.querySelector('[data-profile-status]').dataset.failureLabel;
    message.focus();
  }
  async function refresh() {
    const response = await fetch(location.pathname, { credentials: 'same-origin', cache: 'no-store' });
    if (!response.ok || new URL(response.url).pathname !== location.pathname) {
      location.reload();
      return;
    }
    const result = new DOMParser().parseFromString(await response.text(), 'text/html');
    const replacement = result.querySelector('[data-profile-security-content]');
    if (!replacement) throw Error('Missing security response');
    document.querySelector('[data-profile-security-content]').replaceWith(replacement);
  }
  async function load(url, options = {}) {
    if (pending || !isSecurity(new URL(url, location.href))) return;
    pending = true;
    const current = ++generation;
    dialog.setAttribute('aria-busy', 'true');
    try {
      const response = await fetch(url, { credentials: 'same-origin', cache: 'no-store', ...options });
      if (current !== generation) return;
      const destination = new URL(response.url);
      if (!isSecurity(destination)) {
        content.replaceChildren();
        location.assign(destination.href);
        return;
      }
      if (destination.pathname === '/account/security' && response.ok) {
        await refresh();
        dialog.close();
        return;
      }
      if (destination.pathname === '/account/security/email/pending' && response.ok) rememberReturn(30);
      const result = new DOMParser().parseFromString(await response.text(), 'text/html');
      const main = result.querySelector('main');
      if (!main) throw Error('Missing security form');
      const required = [...main.querySelectorAll('script[src]')].map(script => new URL(script.getAttribute('src'), location.origin).pathname).filter(path => scripts.has(path));
      main.querySelectorAll('script').forEach(script => script.remove());
      main.querySelectorAll('h1').forEach(heading => {
        const replacement = document.createElement('h3');
        for (const attribute of heading.attributes) replacement.setAttribute(attribute.name, attribute.value);
        replacement.textContent = heading.textContent;
        heading.replaceWith(replacement);
      });
      document.dispatchEvent(new Event('security-flow-unmount'));
      content.replaceChildren(...main.childNodes);
      for (const path of required) {
        await new Promise((resolve, reject) => {
          const script = document.createElement('script');
          script.src = path;
          script.onload = resolve;
          script.onerror = reject;
          content.append(script);
        });
      }
      const focus = content.querySelector('[role="alert"]:not([hidden]), h3');
      if (focus) { focus.tabIndex = -1; focus.focus(); }
    } catch {
      if (current === generation) error();
    } finally {
      pending = false;
      dialog.removeAttribute('aria-busy');
    }
  }
  function open(trigger) {
    if (dialog.open) return;
    opener = trigger;
    content.textContent = dialog.dataset.loadingLabel;
    dialog.showModal();
  }
  document.addEventListener('click', event => {
    const link = event.target.closest('a[data-security-flow], #profile-security-flow a[href]');
    if (!link || event.ctrlKey || event.metaKey || event.shiftKey || event.altKey) return;
    const url = new URL(link.href);
    if (!isSecurity(url)) return;
    event.preventDefault();
    open(link);
    void load(url);
  });
  document.addEventListener('submit', event => {
    const form = event.target.closest('form[data-security-flow], #profile-security-flow form');
    if (!form || form.matches('[data-add-passkey]')) return;
    const action = new URL(form.getAttribute('action'), location.href);
    if (action.origin === location.origin && action.pathname === '/auth/zitadel' && dialog.contains(form)) {
      rememberReturn(10);
      return;
    }
    if (!isSecurity(action)) return;
    event.preventDefault();
    const body = new URLSearchParams(new FormData(form));
    open(event.submitter || form);
    void load(action, { method: 'POST', body });
  });
  document.addEventListener('security-flow-navigate', event => {
    if (!dialog.open || !isSecurity(new URL(event.detail, location.href))) return;
    event.preventDefault();
    void load(event.detail);
  });
  document.addEventListener('security-flow-busy', event => {
    if (dialog.open) credentialPending = event.detail;
  });
  dialog.addEventListener('cancel', event => { if (pending || credentialPending) event.preventDefault(); });
  dialog.addEventListener('click', event => {
    if ((pending || credentialPending) && event.target.closest('[data-dialog-close]')) event.stopImmediatePropagation();
  });
  dialog.addEventListener('close', () => {
    generation += 1;
    document.dispatchEvent(new Event('security-flow-unmount'));
    content.replaceChildren();
    if (opener?.isConnected) opener.focus();
    else document.getElementById('profile-tab-security').focus();
  });
})();
