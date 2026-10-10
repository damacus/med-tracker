(() => {
  let installed = false;
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
    if (data.avatar_attached) {
      const image = element('img');
      image.src = `/households/${data.slug}/profile/avatar`;
      image.alt = '';
      avatar.append(image);
    } else avatar.textContent = data.initials;
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
