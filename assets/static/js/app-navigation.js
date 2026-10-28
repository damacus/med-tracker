(() => {
  let installed = false;
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
    const layout = document.querySelector('[data-app-drawer-layout]');
    if (!header || !template || !layout || !data?.labels || installed) return;
    const labels = data.labels;
    const searchTrigger = header?.querySelector('[data-global-search-trigger]');
    const appearanceTrigger = header?.querySelector('[data-dialog-open="appearance-dialog"]');
    const securityLink = header?.querySelector('a[href="/account/security"]');
    const selected = data.selected_section;
    const menu = controls => {
      const list = element('ul', null, 'menu w-full');
      for (const control of controls) {
        const item = element('li');
        item.append(control);
        list.append(item);
      }
      return list;
    };
    const link = item => {
      const anchor = element('a', null, 'min-w-0 whitespace-normal wrap-anywhere');
      anchor.append(element('span', item.label, 'min-w-0 wrap-anywhere'));
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
      const button = element('button', labels.sidebar.sign_out, 'min-w-0 whitespace-normal wrap-anywhere');
      button.type = 'submit';
      form.append(token, menu([button]));
      return form;
    };
    const heading = (panel, title, closePanel) => {
      const top = element('div', null, 'app-navigation-heading');
      top.append(element('h2', title));
      const close = element('button', '×', 'btn btn-ghost');
      close.type = 'button';
      close.setAttribute('aria-label', labels.mobile_menu.close_menu);
      close.addEventListener('click', closePanel);
      top.append(close);
      panel.append(top);
    };
    const drawer = trigger => {
      const toggle = layout.querySelector('.drawer-toggle');
      const content = layout.querySelector('.drawer-content');
      const side = layout.querySelector('.drawer-side');
      const overlay = layout.querySelector('.drawer-overlay');
      const panel = layout.querySelector('#app-drawer');
      overlay.setAttribute('aria-label', labels.mobile_menu.close_menu);
      panel.setAttribute('role', 'dialog');
      panel.setAttribute('aria-modal', 'true');
      panel.setAttribute('aria-label', data.navigation_label);
      side.inert = true;
      let focusFrame;
      const focusWhenVisible = () => {
        if (!toggle.checked) return;
        const button = panel.querySelector('button');
        if (getComputedStyle(button).visibility === 'visible') button.focus({ preventScroll: true });
        else focusFrame = requestAnimationFrame(focusWhenVisible);
      };
      const sync = () => {
        cancelAnimationFrame(focusFrame);
        const open = toggle.checked;
        trigger.setAttribute('aria-expanded', String(open));
        panel.toggleAttribute('open', open);
        side.inert = !open;
        content.inert = open;
        if (open) focusFrame = requestAnimationFrame(focusWhenVisible);
        else if (panel.contains(document.activeElement) || document.activeElement === document.body) trigger.focus({ preventScroll: true });
      };
      panel.close = () => {
        toggle.checked = false;
        sync();
        panel.dispatchEvent(new Event('close'));
      };
      heading(panel, labels.mobile_menu.brand, panel.close);
      trigger.addEventListener('click', () => { toggle.checked = !toggle.checked; sync(); });
      toggle.addEventListener('change', sync);
      document.addEventListener('keydown', event => {
        if (!toggle.checked) return;
        if (event.key === 'Escape') { event.preventDefault(); panel.close(); }
        if (event.key !== 'Tab') return;
        const controls = [...panel.querySelectorAll('a[href], button:not([disabled]), [tabindex="0"]')].filter(node => node.getClientRects().length);
        const first = controls[0];
        const last = controls.at(-1);
        if (event.shiftKey && (document.activeElement === first || document.activeElement === panel)) { event.preventDefault(); last?.focus(); }
        else if (!event.shiftKey && document.activeElement === last) { event.preventDefault(); first?.focus(); }
      });
      return panel;
    };
    const dropdown = triggers => {
      const panel = element('nav', null, 'dropdown app-navigation-panel app-account');
      panel.id = 'app-account';
      panel.popover = 'auto';
      panel.setAttribute('aria-label', labels.profile_menu.my_account);
      let activeTrigger;
      const finishClose = () => {
        if (!panel.hasAttribute('open')) return;
        panel.removeAttribute('open');
        activeTrigger?.setAttribute('aria-expanded', 'false');
        panel.dispatchEvent(new Event('close'));
      };
      panel.close = () => {
        if (panel.matches(':popover-open')) panel.hidePopover();
        finishClose();
      };
      const anchorSupported = CSS.supports('position-anchor: --account-trigger') && CSS.supports('position-area: bottom');
      if (!anchorSupported) {
        panel.style.positionArea = 'none';
        panel.style.inset = 'auto';
        panel.style.margin = '0';
      }
      const position = () => {
        if (!activeTrigger) return;
        if (!activeTrigger.getClientRects().length) { panel.close(); return; }
        if (anchorSupported) return;
        const bounds = activeTrigger.getBoundingClientRect();
        const sidebar = activeTrigger === sidebarTrigger;
        panel.style.maxHeight = Math.max(0, sidebar ? bounds.top - 16 : innerHeight - bounds.bottom - 16) + 'px';
        panel.style.left = Math.max(8, Math.min(sidebar ? bounds.left : bounds.right - panel.offsetWidth, innerWidth - panel.offsetWidth - 8)) + 'px';
        panel.style.top = Math.max(8, sidebar ? bounds.top - panel.offsetHeight - 8 : bounds.bottom + 8) + 'px';
      };
      for (const [index, trigger] of triggers.entries()) {
        const anchor = '--account-trigger-' + index;
        trigger.style.anchorName = anchor;
        trigger.setAttribute('popovertarget', panel.id);
        trigger.removeAttribute('aria-haspopup');
        trigger.addEventListener('click', () => {
          activeTrigger = trigger;
          panel.style.positionAnchor = anchor;
          panel.classList.toggle('dropdown-top', trigger === sidebarTrigger);
          panel.classList.toggle('dropdown-start', trigger === sidebarTrigger);
          panel.classList.toggle('dropdown-bottom', trigger !== sidebarTrigger);
          panel.classList.toggle('dropdown-end', trigger !== sidebarTrigger);
        });
        trigger.addEventListener('keydown', event => {
          if (event.key !== 'ArrowDown') return;
          event.preventDefault();
          if (!panel.matches(':popover-open')) trigger.click();
          panel.querySelector('button, a[href]')?.focus({ preventScroll: true });
        });
        trigger.hidden = false;
      }
      panel.addEventListener('beforetoggle', event => {
        if (event.newState !== 'open') return;
        panel.setAttribute('open', '');
        activeTrigger?.setAttribute('aria-expanded', 'true');
      });
      panel.addEventListener('toggle', event => {
        if (event.newState === 'closed') finishClose();
        else position();
      });
      document.addEventListener('focusin', event => {
        if (panel.matches(':popover-open') && !panel.contains(event.target) && event.target !== activeTrigger) panel.close();
      });
      window.addEventListener('resize', position);
      document.addEventListener('scroll', position, true);
      document.body.append(panel);
      return panel;
    };
    if (header) {
      header.replaceChildren(template.content.cloneNode(true));
      if (searchTrigger) header.querySelector('.app-header').insertBefore(searchTrigger, header.querySelector('[data-app-account]'));
      const drawerTrigger = header.querySelector('[data-app-drawer]');
      const navigationDrawer = drawer(drawerTrigger);
      drawerTrigger.setAttribute('aria-label', labels.mobile_menu.open_menu);
      const navigation = element('nav');
      navigation.setAttribute('aria-label', labels.mobile_rail.primary_navigation);
      const profile = data.choices.find(item => item.id === 'profile');
      const destinations = [...data.sidebar, ...(profile ? [profile] : [])];
      navigation.append(menu(destinations.map(link)));
      if (sidebar) sidebar.querySelector('[data-sidebar-navigation]').replaceChildren(menu(destinations.map(link)));
      navigationDrawer.append(navigation, logout());
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
      const householdName = element('span', null, 'grid min-w-0 wrap-anywhere');
      householdName.append(element('span', labels.profile_menu.households));
      switcher.replaceChildren(householdName);
      if (selectedHousehold) {
        householdName.append(element('span', selectedHousehold.name, 'text-xs shell-muted'));
      }
      sidebar.querySelector('[data-sidebar-name]').textContent = data.name;
      sidebar.querySelector('[data-sidebar-avatar]').replaceChildren(...[...header.querySelector('[data-profile-avatar]').childNodes].map(node => node.cloneNode(true)));
      sidebar.querySelector('[data-sidebar-profile]').hidden = true;
      const signOut = sidebar.querySelector('form[action="/logout"]');
      if (signOut) {
        signOut.querySelector("button").textContent = labels.sidebar.sign_out;
        signOut.querySelector('[name="authenticity_token"]').value = data.authenticity_token;
      } else sidebar.querySelector('.app-sidebar-account').append(logout());
    }
    const accountTriggers = [header.querySelector('[data-app-account]'), ...(sidebarTrigger ? [sidebarTrigger] : [])];
    const account = dropdown(accountTriggers);
    const accountControls = [];
    for (const trigger of accountTriggers) trigger.setAttribute('aria-label', `${data.initials} ${data.name} ${labels.profile_menu.my_account}`);
    if (appearanceTrigger) {
      appearanceTrigger.className = 'min-w-0 whitespace-normal wrap-anywhere';
      appearanceTrigger.addEventListener('click', () => account.close());
      accountControls.push(appearanceTrigger);
    }
    if (securityLink) {
      securityLink.className = 'min-w-0 whitespace-normal wrap-anywhere';
      accountControls.push(securityLink);
    }
    for (const id of ['dashboard', 'profile', 'administration']) {
      const item = data.choices.find(item => item.id === id);
      if (item) accountControls.push(link({ ...item, label: labels.profile_menu[id] }));
    }
    if (data.platform_url === '/platform/users') accountControls.push(link({ id: 'administration', label: 'Platform administration', href: '/platform/users' }));
    account.append(menu(accountControls));
    if (Array.isArray(data.households) && data.households.length) {
      const section = element('section', null, 'border-t border-base-300 mt-2 pt-3');
      section.append(element('h3', labels.profile_menu.households, 'menu-title'));
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
        const button = element('button', `${labels.profile_menu.switch_to} ${household.name}`, 'min-w-0 whitespace-normal wrap-anywhere');
        button.type = 'submit';
        if (String(household.id) === data.active_household_id) button.setAttribute('aria-current', 'true');
        form.append(menu([button]));
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
