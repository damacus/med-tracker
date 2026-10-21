(() => {
  let installed = false;
  const authPaths = new Set(['/login', '/logout', '/recovery-login', '/auth/totp', '/auth/zitadel']);
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
  const element = (tag, text, className) => {
    const node = document.createElement(tag);
    if (text) node.textContent = text;
    if (className) node.className = className;
    return node;
  };
  document.addEventListener('app-navigation', event => {
    const data = event.detail;
    const header = document.querySelector('.shell-header');
    const sidebar = document.querySelector('.app-sidebar');
    const sidebarTrigger = document.querySelector('[data-sidebar-account]');
    const template = document.getElementById('app-navigation-template');
    if (!header || !template || !data?.labels || installed) return;
    const labels = data.labels;
    const searchTrigger = header?.querySelector('[data-global-search-trigger]');
    const appearanceTrigger = header?.querySelector('[data-dialog-open="appearance-dialog"]');
    const securityLink = header?.querySelector('a[href="/account/security"]');
    const selected = data.selected_section;
    const link = item => {
      const anchor = element('a', item.label, 'app-navigation-link');
      anchor.href = item.href;
      anchor.dataset.section = item.id;
      if (item.id === selected) anchor.setAttribute('aria-current', 'page');
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
    const dialog = (id, title, triggers) => {
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
      let activeTrigger;
      for (const trigger of Array.isArray(triggers) ? triggers : [triggers]) {
        trigger.setAttribute('aria-label', title);
        trigger.addEventListener('click', () => { activeTrigger = trigger; panel.showModal(); trigger.setAttribute('aria-expanded', 'true'); });
        trigger.hidden = false;
      }
      panel.addEventListener('close', () => { activeTrigger?.setAttribute('aria-expanded', 'false'); activeTrigger?.focus(); });
      panel.addEventListener('click', event => {
        const bounds = panel.getBoundingClientRect();
        if (event.target === panel && (event.clientX < bounds.left || event.clientX > bounds.right || event.clientY < bounds.top || event.clientY > bounds.bottom)) panel.close();
      });
      document.body.append(panel);
      return panel;
    };
    if (header) {
      header.replaceChildren(template.content.cloneNode(true));
      if (searchTrigger) header.querySelector('.app-header').insertBefore(searchTrigger, header.querySelector('[data-app-account]'));
      const drawerTrigger = header.querySelector('[data-app-drawer]');
      const drawer = dialog('app-drawer', data.navigation_label, drawerTrigger);
      drawerTrigger.setAttribute('aria-label', labels.mobile_menu.open_menu);
      const navigation = element('nav');
      navigation.setAttribute('aria-label', labels.mobile_rail.primary_navigation);
      data.sidebar.forEach(item => navigation.append(link(item)));
      const profile = data.choices.find(item => item.id === 'profile');
      if (profile) navigation.append(link(profile));
      if (sidebar) sidebar.querySelector('[data-sidebar-navigation]').replaceChildren(...[...data.sidebar, ...(profile ? [profile] : [])].map(link));
      drawer.append(navigation, logout());
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
    }
    if (sidebarTrigger) {
      sidebar.setAttribute("aria-label", labels.mobile_rail.primary_navigation);
      sidebar.querySelector("[data-sidebar-navigation]").setAttribute("aria-label", labels.mobile_rail.primary_navigation);
      const switcher = sidebar.querySelector('a[href="/households/choose?switch=1"]');
      const selectedHousehold = data.households?.find(household => String(household.id) === String(data.active_household_id) && typeof household.name === 'string');
      switcher.replaceChildren(element('span', labels.profile_menu.households));
      switcher.setAttribute('aria-label', labels.profile_menu.households);
      if (selectedHousehold) {
        switcher.append(element('span', selectedHousehold.name, 'text-xs shell-muted'));
        switcher.setAttribute('aria-label', `${labels.profile_menu.households} · ${selectedHousehold.name}`);
      }
      switcher.classList.add('app-household-switcher');
      sidebar.querySelector('[data-sidebar-name]').textContent = data.name;
      sidebar.querySelector('[data-sidebar-avatar]').replaceChildren(...[...header.querySelector('[data-profile-avatar]').childNodes].map(node => node.cloneNode(true)));
      sidebar.querySelector('[data-sidebar-profile]').hidden = true;
      const signOut = sidebar.querySelector('form[action="/logout"]');
      if (signOut) {
        signOut.querySelector("button").textContent = labels.sidebar.sign_out;
        signOut.querySelector('[name="authenticity_token"]').value = data.authenticity_token;
      } else sidebar.querySelector('.app-sidebar-account').append(logout());
    }
    const account = dialog('app-account', labels.profile_menu.my_account, [header.querySelector('[data-app-account]'), ...(sidebarTrigger ? [sidebarTrigger] : [])]);
    if (appearanceTrigger) {
      appearanceTrigger.className = 'app-navigation-link';
      appearanceTrigger.addEventListener('click', () => account.close());
      account.append(appearanceTrigger);
    }
    if (securityLink) {
      securityLink.className = 'app-navigation-link';
      account.append(securityLink);
    }
    for (const id of ['dashboard', 'profile', 'administration']) {
      const item = data.choices.find(item => item.id === id);
      if (item) account.append(link({ ...item, label: labels.profile_menu[id] }));
    }
    if (data.platform_url === '/platform/users') account.append(link({ id: 'administration', label: 'Platform administration', href: '/platform/users' }));
    if (Array.isArray(data.households) && data.households.length) {
      const section = element('section', null, 'border-t border-base-300 mt-2 pt-3');
      section.append(element('h3', labels.profile_menu.households, 'text-sm font-semibold px-4'));
      for (const household of data.households) {
        if (!Number.isSafeInteger(household.id) || household.id <= 0 || typeof household.name !== 'string') continue;
        const form = element('form');
        form.method = 'post';
        form.action = '/households/select';
        for (const [name, value] of [['authenticity_token', data.authenticity_token], ['household_id', String(household.id)]]) {
          const input = element('input');
          input.type = 'hidden';
          input.name = name;
          input.value = value;
          form.append(input);
        }
        const button = element('button', `${labels.profile_menu.switch_to} ${household.name}`, 'app-navigation-link');
        button.type = 'submit';
        if (String(household.id) === data.active_household_id) button.setAttribute('aria-current', 'true');
        form.append(button);
        section.append(form);
      }
      account.append(section);
    }
    account.append(logout());
    document.querySelector('[data-profile-navigation-fallback]')?.remove();
    document.body.dataset.navigationReady = 'true';
    installed = true;
  });
})();
