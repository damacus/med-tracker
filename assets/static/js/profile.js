const tablist = document.querySelector('[data-profile-tabs]');
const appearanceSummaries = document.querySelectorAll('[data-profile-appearance]');
if (appearanceSummaries.length) {
  const updateSummary = () => {
    for (const summary of appearanceSummaries) summary.textContent = summary.dataset[document.documentElement.dataset.appearance] || '';
    const palette = document.querySelector('[data-palette-choice][aria-pressed="true"]');
    for (const summary of document.querySelectorAll('[data-profile-palette]')) summary.textContent = palette?.textContent.trim() || '';
  };
  updateSummary();
  new MutationObserver(updateSummary).observe(document.documentElement, { attributes: true, attributeFilter: ['data-appearance', 'data-theme'] });
}
if (tablist) {
  const tabs = [...tablist.querySelectorAll('[data-profile-tab]')];
  const panels = [...document.querySelectorAll('[data-profile-panel]')];
  tablist.setAttribute('role', 'tablist');
  tabs.forEach(tab => {
    tab.setAttribute('role', 'tab');
    tab.setAttribute('aria-controls', tab.dataset.profileTab);
  });
  panels.forEach(panel => panel.setAttribute('role', 'tabpanel'));
  function selectTab(id, focus = false) {
    const selected = tabs.find(tab => tab.dataset.profileTab === id) || tabs[0];
    tabs.forEach(tab => {
      const active = tab === selected;
      tab.setAttribute('aria-selected', String(active));
      tab.tabIndex = active ? 0 : -1;
    });
    panels.forEach(panel => { panel.hidden = panel.id !== selected.dataset.profileTab; });
    if (focus) selected.focus();
  }
  tablist.addEventListener('click', event => {
    const tab = event.target.closest('[data-profile-tab]');
    if (!tab) return;
    event.preventDefault();
    history.replaceState(null, '', `#${tab.dataset.profileTab}`);
    selectTab(tab.dataset.profileTab);
  });
  tablist.addEventListener('keydown', event => {
    const current = tabs.indexOf(event.target);
    if (current < 0) return;
    let next;
    if (event.key === 'ArrowRight') next = (current + 1) % tabs.length;
    if (event.key === 'ArrowLeft') next = (current + tabs.length - 1) % tabs.length;
    if (event.key === 'Home') next = 0;
    if (event.key === 'End') next = tabs.length - 1;
    if (next === undefined) return;
    event.preventDefault();
    history.replaceState(null, '', `#${tabs[next].dataset.profileTab}`);
    selectTab(tabs[next].dataset.profileTab, true);
  });
  window.addEventListener('hashchange', () => selectTab(location.hash.slice(1)));
  selectTab(location.hash.slice(1));
}

document.addEventListener('submit', async event => {
  const form = event.target.closest('form[data-profile-form]');
  const dialog = form?.closest('dialog');
  if (!form) return;
  event.preventDefault();
  const region = form.closest('[data-profile-form-region]');
  const pendingRegion = region || form;
  if (pendingRegion.dataset.submitting === 'true') return;
  pendingRegion.dataset.submitting = 'true';
  const body = form.enctype === 'multipart/form-data' ? new FormData(form) : new URLSearchParams(new FormData(form));
  const controls = [...pendingRegion.querySelectorAll('button, input, select, textarea')].filter(control => !control.disabled);
  controls.forEach(control => { control.disabled = true; });
  try {
    const response = await fetch(form.action, { method: 'POST', body, headers: { Accept: 'text/html' }, cache: 'no-store' });
    const destination = new URL(response.url);
    if (destination.origin !== location.origin) throw new Error('Unexpected destination');
    if (destination.pathname === '/login') {
      location.assign(destination.href);
      return;
    }
    if ([401, 403, 404].includes(response.status)) {
      location.reload();
      return;
    }
    if (!response.ok && response.status !== 422) throw new Error('Setting unavailable');
    const documentResult = new DOMParser().parseFromString(await response.text(), 'text/html');
    const replacement = region ? documentResult.getElementById(region.id) : documentResult.getElementById(dialog?.id)?.querySelector('form');
    if (!replacement) throw new Error('Missing setting response');
    const expanded = [...(region || form).querySelectorAll('details[open][id]')].map(element => element.id);
    (region || form).replaceWith(replacement);
    for (const id of expanded) replacement.querySelector(`#${CSS.escape(id)}`)?.setAttribute('open', '');
    if (response.status === 422) {
      (dialog || replacement).querySelector('[data-dialog-error-focus]')?.focus();
      return;
    }
    const personal = documentResult.querySelector('[data-testid="profile-personal-info-card"]');
    for (const summary of documentResult.querySelectorAll('[data-profile-summary][id]')) document.getElementById(summary.id)?.replaceWith(summary);
    if (personal) document.querySelector('[data-testid="profile-personal-info-card"]')?.replaceWith(personal);
    const avatar = documentResult.querySelector('[data-profile-avatar]');
    if (avatar) for (const current of document.querySelectorAll('[data-profile-avatar]')) current.replaceChildren(...avatar.cloneNode(true).childNodes);
    if (dialog) dialog.close();
    else replacement.querySelector('[type="submit"]')?.focus();
    const status = document.querySelector('[data-profile-status]');
    if (status) status.textContent = status.dataset.savedLabel;
    document.dispatchEvent(new Event('profile-saved'));
  } catch {
    let error = form.querySelector('[data-profile-submit-error]');
    if (!error) {
      error = document.createElement('p');
      error.dataset.profileSubmitError = '';
      error.setAttribute('role', 'alert');
      error.tabIndex = -1;
      form.prepend(error);
    }
    error.textContent = document.querySelector('[data-profile-status]')?.dataset.failureLabel || '';
    error.focus();
  } finally {
    delete pendingRegion.dataset.submitting;
    controls.forEach(control => { control.disabled = false; });
  }
});
