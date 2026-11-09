(() => {
  const authPaths = new Set(['/login', '/logout', '/reset-password', '/auth/totp', '/auth/zitadel']);
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
    const registration = await navigator.serviceWorker?.getRegistration('/');
    if (!registration?.active) return registration;
    try {
      const status = await workerMessage(registration.active, 'auth-transition-status', undefined, 250);
      if (status.protocol === 1 && registration.active.state === 'activated') return registration;
    } catch {}
    let timer;
    const deadline = Date.now() + 10000;
    try {
      await Promise.race([
        registration.update(),
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
  document.addEventListener('submit', event => {
    const form = event.target;
    if (!(form instanceof HTMLFormElement) || !authPaths.has(new URL(form.action).pathname)) return;
    event.preventDefault();
    void (async () => {
      try {
        await beginAuthTransition();
      } catch {
        window.alert('Could not update or clear browser notifications. Check your connection and retry signing in or out.');
        return;
      }
      if (!form.isConnected) document.body.append(form);
      HTMLFormElement.prototype.submit.call(form);
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
  for (const disclosure of document.querySelectorAll("[data-account-menu]")) {
    const summary = disclosure.querySelector("summary");
    const panel = disclosure.querySelector('#app-account');
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
      if (event.key === "Escape" && disclosure.open) { event.preventDefault(); close(); }
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
  const drawer = document.querySelector("[data-mobile-drawer]");
  if (drawer) {
    const summary = drawer.querySelector("summary");
    const panel = drawer.querySelector("#app-drawer");
    const closeButton = drawer.querySelector("[data-drawer-close]");
    const overlay = drawer.querySelector('[data-drawer-overlay]');
    const close = () => { drawer.open = false; summary.focus({ preventScroll: true }); };
    panel.close = () => { drawer.open = false; panel.removeAttribute('open'); panel.dispatchEvent(new Event('close')); };
    summary.setAttribute('aria-controls', panel.id);
    summary.setAttribute('aria-expanded', String(drawer.open));
    const controls = () => [...panel.querySelectorAll('a[href], button:not([disabled]), [tabindex="0"]')].filter(node => node.getClientRects().length);
    const outside = new Map();
    drawer.addEventListener("toggle", () => {
      summary.setAttribute("aria-expanded", String(drawer.open));
      panel.toggleAttribute('open', drawer.open);
      if (drawer.open) {
        panel.setAttribute("role", "dialog");
        panel.setAttribute("aria-modal", "true");
        for (const node of document.querySelectorAll('.app-sidebar, .shell-main, .shell-footer, [data-mobile-shortcuts], .app-header > :not([data-mobile-drawer])')) {
          if (!outside.has(node)) outside.set(node, node.inert);
          node.inert = true;
        }
        controls()[0]?.focus({ preventScroll: true });
      } else {
        panel.removeAttribute("role");
        panel.removeAttribute("aria-modal");
        for (const [node, inert] of outside) node.inert = inert;
        outside.clear();
      }
    });
    closeButton.addEventListener("click", close);
    overlay.addEventListener('click', close);
    drawer.addEventListener("keydown", event => {
      if (!drawer.open) return;
      if (event.key === "Escape") { event.preventDefault(); close(); return; }
      if (event.key !== "Tab") return;
      const focusable = controls();
      const first = focusable[0];
      const last = focusable.at(-1);
      if (event.shiftKey && (document.activeElement === first || document.activeElement === summary)) { event.preventDefault(); last?.focus(); }
      else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
    });
    document.addEventListener("click", event => { if (drawer.open && !drawer.contains(event.target)) drawer.open = false; });
    window.matchMedia("(min-width: 1024px)").addEventListener("change", event => { if (event.matches) drawer.open = false; });
    closeButton.hidden = false;
    overlay.hidden = false;
    drawer.classList.add('drawer-enhanced');
  }
})();
