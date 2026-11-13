(() => {
  const authPaths = new Set(['/login', '/logout', '/reset-password', '/auth/totp', '/auth/oidc']);
  const transitionKey = 'medtracker.push.transition';
  function workerMessage(worker, type, id, timeoutMs = 5000) {
    return new Promise((resolve, reject) => {
      const channel = new MessageChannel();
      const timeout = setTimeout(() => { channel.port1.close(); reject(Error('Notification transition timed out')); }, timeoutMs);
      channel.port1.onmessage = event => {
        clearTimeout(timeout);
        channel.port1.close();
        if (event.data?.ok) resolve(event.data);
        else reject(Error('Notification transition failed'));
      };
      worker.postMessage({ type, id }, [channel.port2]);
    });
  }
  async function closeNotifications(registration) {
    (await registration?.getNotifications?.())?.forEach(notification => notification.close());
  }
  async function compatibleRegistration() {
    let registration = await navigator.serviceWorker?.getRegistration('/');
    if (!registration?.active) return registration;
    try {
      const status = await workerMessage(registration.active, 'auth-transition-status', undefined, 250);
      if (status.protocol === 1 && registration.active.state === 'activated') return registration;
    } catch {}
    let timer;
    const deadline = Date.now() + 10000;
    try {
      registration = await Promise.race([
        navigator.serviceWorker.register('/profile-service-worker.js', { scope: '/', updateViaCache: 'none' }),
        new Promise((_, reject) => { timer = setTimeout(() => reject(Error('Notification worker upgrade timed out')), 10000); }),
      ]);
      while (registration.installing || registration.waiting || registration.active?.state !== 'activated') {
        if (Date.now() >= deadline) throw Error('Notification worker upgrade timed out');
        registration.waiting?.postMessage({ type: 'activate' });
        await new Promise(resolve => setTimeout(resolve, 50));
      }
      if (Date.now() >= deadline) throw Error('Notification worker upgrade timed out');
      const status = await workerMessage(registration.active, 'auth-transition-status', undefined, deadline - Date.now());
      if (status.protocol !== 1) throw Error('Notification worker is incompatible');
      return registration;
    } finally { clearTimeout(timer); }
  }
  async function finishAuthTransition() {
    const id = sessionStorage.getItem(transitionKey);
    if (!id) return;
    const registration = await compatibleRegistration();
    if (!registration?.active) { sessionStorage.removeItem(transitionKey); return; }
    const status = await workerMessage(registration.active, 'auth-transition-status');
    if (status.id !== id) { sessionStorage.removeItem(transitionKey); return; }
    await closeNotifications(registration);
    await workerMessage(registration.active, 'auth-transition-end', id);
    sessionStorage.removeItem(transitionKey);
  }
  const ready = compatibleRegistration().then(finishAuthTransition).catch(() => {});
  async function beginAuthTransition() {
    await ready;
    const registration = await compatibleRegistration();
    if (registration?.active) {
      const id = sessionStorage.getItem(transitionKey) || crypto.randomUUID();
      sessionStorage.setItem(transitionKey, id);
      await workerMessage(registration.active, 'auth-transition-begin', id);
    }
    await closeNotifications(registration);
  }
  window.medTrackerBeginPushTransition = beginAuthTransition;
  window.medTrackerEndPushTransition = finishAuthTransition;
  window.addEventListener('pageshow', event => {
    if (!event.persisted) return;
    for (const form of document.querySelectorAll('form[aria-busy="true"]')) {
      if (!authPaths.has(new URL(form.action).pathname)) continue;
      form.removeAttribute('aria-busy');
      form.querySelector('[data-auth-status]')?.remove();
      for (const submitter of form.querySelectorAll('[data-auth-pending]')) {
        submitter.disabled = false;
        submitter.removeAttribute('data-auth-pending');
      }
    }
  });
  document.addEventListener('submit', event => {
    const form = event.target;
    if (!(form instanceof HTMLFormElement) || !authPaths.has(new URL(form.action).pathname)) return;
    if (new URL(form.action).pathname === '/reset-password') {
      const enhanced = new CustomEvent('profile-flow-auth-submit', { bubbles: true, cancelable: true, detail: { submitter: event.submitter } });
      if (!form.dispatchEvent(enhanced)) { event.preventDefault(); return; }
    }
    event.preventDefault();
    if (form.getAttribute('aria-busy') === 'true') return;
    let status = form.querySelector('[data-auth-status]');
    if (!status) {
      status = document.createElement('p');
      status.dataset.authStatus = '';
      form.append(status);
    }
    status.setAttribute('role', 'status');
    status.textContent = 'Please wait…';
    form.setAttribute('aria-busy', 'true');
    if (event.submitter) {
      event.submitter.disabled = true;
      event.submitter.setAttribute('data-auth-pending', '');
    }
    void (async () => {
      try {
        await beginAuthTransition();
      } catch {
        status.setAttribute('role', 'alert');
        status.textContent = 'Could not sign in or out. Refresh this page and try again.';
        form.removeAttribute('aria-busy');
        if (event.submitter) {
          event.submitter.disabled = false;
          event.submitter.removeAttribute('data-auth-pending');
        }
        return;
      }
      if (!form.isConnected) document.body.append(form);
      let submitter;
      if (event.submitter?.name) {
        submitter = document.createElement('input');
        submitter.type = 'hidden';
        submitter.name = event.submitter.name;
        submitter.value = event.submitter.value;
        form.append(submitter);
      }
      try { HTMLFormElement.prototype.submit.call(form); }
      finally { submitter?.remove(); }
    })();
  }, true);
  const sidebar = document.querySelector("[data-sidebar-expanded]");
  if (sidebar) {
    sidebar.addEventListener("change", () => {
      document.body.dataset.sidebarCollapsed = String(!sidebar.checked);
      document.cookie = "medtracker.sidebar=" + (sidebar.checked ? "expanded" : "collapsed") + "; Path=/; Max-Age=31536000; SameSite=Lax";
    });
    requestAnimationFrame(() => document.body.classList.add("sidebar-interactive"));
  }
  for (const disclosure of document.querySelectorAll("[data-household-selector]")) {
    const summary = disclosure.querySelector("summary");
    const panel = disclosure.querySelector('[data-household-options]');
    const close = () => { disclosure.open = false; summary.focus({ preventScroll: true }); };
    panel.close = () => {
      disclosure.open = false;
      panel.removeAttribute('open');
      summary.setAttribute('aria-expanded', 'false');
      panel.dispatchEvent(new Event('close'));
    };
    summary.setAttribute('aria-controls', panel.id);
    summary.setAttribute('aria-expanded', String(disclosure.open));
    disclosure.addEventListener('toggle', () => {
      panel.toggleAttribute('open', disclosure.open);
      summary.setAttribute('aria-expanded', String(disclosure.open));
    });
    disclosure.addEventListener("keydown", event => {
      if (event.key === "Escape" && disclosure.open) { event.preventDefault(); event.stopPropagation(); close(); }
      if (event.key === 'ArrowDown' && event.target === summary) {
        event.preventDefault();
        disclosure.open = true;
        panel.querySelector('a[href], button')?.focus({ preventScroll: true });
      }
    });
    document.addEventListener("click", event => {
      if (disclosure.open && !disclosure.contains(event.target)) disclosure.open = false;
    });
    disclosure.addEventListener("focusout", event => {
      if (disclosure.open && event.relatedTarget && !disclosure.contains(event.relatedTarget)) disclosure.open = false;
    });
  }
  const accountMenu = document.querySelector('[data-account-menu]');
  if (accountMenu) {
    const summary = accountMenu.querySelector('summary');
    accountMenu.addEventListener('keydown', event => {
      if (event.key === 'Escape' && accountMenu.open) {
        event.preventDefault();
        accountMenu.open = false;
        summary.focus({ preventScroll: true });
      }
      if (event.key === 'ArrowDown' && event.target === summary) {
        event.preventDefault();
        accountMenu.open = true;
        accountMenu.querySelector('a[href]')?.focus({ preventScroll: true });
      }
    });
    document.addEventListener('click', event => {
      if (accountMenu.open && !accountMenu.contains(event.target)) accountMenu.open = false;
    });
    accountMenu.addEventListener('focusout', event => {
      if (accountMenu.open && event.relatedTarget && !accountMenu.contains(event.relatedTarget)) accountMenu.open = false;
    });
  }
  const drawer = document.querySelector("[data-mobile-drawer]");
  if (drawer && typeof drawer.querySelector('#app-drawer').showModal === 'function') {
    const summary = drawer.querySelector("summary");
    const panel = drawer.querySelector("#app-drawer");
    const closeButton = drawer.querySelector("[data-drawer-close]");
    summary.setAttribute('aria-controls', panel.id);
    summary.setAttribute('aria-expanded', String(drawer.open));
    drawer.addEventListener("toggle", () => {
      summary.setAttribute("aria-expanded", String(drawer.open));
      if (drawer.open && !panel.open) panel.showModal();
      else if (!drawer.open && panel.open) panel.close();
    });
    panel.addEventListener('close', () => { drawer.open = false; summary.setAttribute('aria-expanded', 'false'); });
    closeButton.addEventListener('click', () => panel.close());
    panel.addEventListener('click', event => {
      if (event.target !== panel) return;
      const rect = panel.getBoundingClientRect();
      if (event.clientX < rect.left || event.clientX > rect.right || event.clientY < rect.top || event.clientY > rect.bottom) panel.close();
    });
    window.matchMedia("(min-width: 1024px)").addEventListener("change", event => { if (event.matches && panel.open) panel.close(); });
    closeButton.hidden = false;
    drawer.classList.add('drawer-enhanced');
  }
})();
