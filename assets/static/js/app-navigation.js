(() => {
  let installed = false;
  const authPaths = new Set(['/login', '/logout', '/recovery-login', '/auth/totp', '/auth/zitadel']);
  const transitionKey = 'medtracker.push.transition';
  function workerMessage(worker, type, id) {
    return new Promise((resolve, reject) => {
      const channel = new MessageChannel();
      const timeout = setTimeout(() => { channel.port1.close(); reject(Error('Notification transition timed out')); }, 5000);
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
  async function finishAuthTransition() {
    const id = sessionStorage.getItem(transitionKey);
    if (!id) return;
    const registration = await navigator.serviceWorker?.getRegistration('/');
    if (!registration?.active) { sessionStorage.removeItem(transitionKey); return; }
    const status = await workerMessage(registration.active, 'auth-transition-status');
    if (status.id !== id) { sessionStorage.removeItem(transitionKey); return; }
    await closeNotifications(registration);
    await workerMessage(registration.active, 'auth-transition-end', id);
    sessionStorage.removeItem(transitionKey);
  }
  const ready = finishAuthTransition().catch(() => {});
  async function beginAuthTransition() {
    await ready;
    const registration = await navigator.serviceWorker?.getRegistration('/');
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
        window.alert('Could not clear browser notifications. Retry, or disable notifications in browser settings before signing in or out.');
        return;
      }
      HTMLFormElement.prototype.submit.call(form);
    })();
  }, true);
  const element = (tag, text, className) => {
    const node = document.createElement(tag);
    if (text) node.textContent = text;
    if (className) node.className = className;
    return node;
  };
  document.addEventListener('app-navigation', event => {
    const data = event.detail;
    const header = document.querySelector('.shell-header');
    const template = document.getElementById('app-navigation-template');
    if (!header || !template || !data?.labels || installed) return;
    const labels = data.labels;
    const link = item => {
      const anchor = element('a', item.label, 'app-navigation-link');
      anchor.href = item.href;
      if (location.pathname === item.href || location.pathname.startsWith(`${item.href}/`)) anchor.setAttribute('aria-current', 'page');
      const svg = document.createElementNS('http://www.w3.org/2000/svg', 'svg');
      svg.setAttribute('width', '24');
      svg.setAttribute('height', '24');
      svg.setAttribute('aria-hidden', 'true');
      const use = document.createElementNS('http://www.w3.org/2000/svg', 'use');
      use.setAttribute('href', `/static/profile-icons.svg#${item.id}`);
      svg.append(use);
      anchor.prepend(svg);
      return anchor;
    };
    const logout = () => {
      const form = element('form');
      form.method = 'post';
      form.action = '/logout';
      const token = element('input');
      token.type = 'hidden';
      token.name = 'authenticity_token';
      token.value = data.authenticity_token;
      const button = element('button', labels.sidebar.sign_out, 'app-navigation-link');
      button.type = 'submit';
      form.append(token, button);
      return form;
    };
    const dialog = (id, title, trigger) => {
      const panel = element('dialog', null, `app-navigation-panel ${id}`);
      panel.id = id;
      panel.setAttribute('aria-label', title);
      const top = element('div', null, 'app-navigation-heading');
      top.append(element('h2', id === 'app-drawer' ? labels.mobile_menu.brand : title));
      const close = element('button', '×', 'btn btn-ghost');
      close.type = 'button';
      close.setAttribute('aria-label', labels.mobile_menu.close_menu);
      close.addEventListener('click', () => panel.close());
      top.append(close);
      panel.append(top);
      trigger.setAttribute('aria-label', title);
      trigger.addEventListener('click', () => { panel.showModal(); trigger.setAttribute('aria-expanded', 'true'); });
      panel.addEventListener('close', () => { trigger.setAttribute('aria-expanded', 'false'); trigger.focus(); });
      panel.addEventListener('click', event => {
        const bounds = panel.getBoundingClientRect();
        if (event.target === panel && (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom)) panel.close();
      });
      document.body.append(panel);
      return panel;
    };
    header.replaceChildren(template.content.cloneNode(true));
    const drawerTrigger = header.querySelector('[data-app-drawer]');
    const drawer = dialog('app-drawer', data.navigation_label, drawerTrigger);
    drawerTrigger.setAttribute('aria-label', labels.mobile_menu.open_menu);
    const navigation = element('nav');
    navigation.setAttribute('aria-label', labels.mobile_rail.primary_navigation);
    data.sidebar.forEach(item => navigation.append(link(item)));
    const profile = data.choices.find(item => item.id === 'profile');
    if (profile) navigation.append(link(profile));
    drawer.append(navigation, logout());
    const accountTrigger = header.querySelector('[data-app-account]');
    header.querySelector('[data-account-name]').textContent = data.name;
    const avatar = header.querySelector('[data-profile-avatar]');
    avatar.classList.add('relative');
    avatar.append(element('span', data.initials));
    if (data.avatar_attached || data.gravatar_url) {
      const image = element('img');
      image.src = data.avatar_attached ? `/households/${data.slug}/profile/avatar` : data.gravatar_url;
      image.alt = '';
      image.dataset.avatarImage = '';
      image.referrerPolicy = 'no-referrer';
      image.className = 'absolute inset-0';
      avatar.append(image);
    }
    const account = dialog('app-account', labels.profile_menu.my_account, accountTrigger);
    for (const id of ['dashboard', 'profile', 'administration']) {
      const item = data.choices.find(item => item.id === id);
      if (item) account.append(link({ ...item, label: labels.profile_menu[id] }));
    }
    account.append(logout());
    document.querySelector('[data-profile-navigation-fallback]')?.remove();
    installed = true;
  });
})();
