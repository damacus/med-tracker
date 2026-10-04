(() => {
  const instances = new WeakMap();
  const locks = new WeakMap();
  function init(root = document, options = {}) {
    if (instances.has(root)) return instances.get(root);
    const doc = root.ownerDocument || root;
    const win = doc.defaultView;
    const modalSelector = options.modalSelector || 'dialog[data-ui-modal]';
    const triggerAttribute = options.triggerAttribute || 'data-ui-open';
    const closeAttribute = options.closeAttribute || 'data-ui-close';
    const tabsSelector = options.tabsSelector || '[data-ui-tabs]';
    const openers = new Map();
    const managed = new Set();
    const find = id => {
      const element = doc.getElementById(id);
      return element && root.contains(element) ? element : null;
    };
    const modals = () => [...root.querySelectorAll(modalSelector)].filter(dialog => dialog.matches(':modal'));
    function syncScroll() {
      let state = locks.get(doc);
      if (!state) {
        state = { owners: new Set(), html: '', body: '' };
        locks.set(doc, state);
      }
      if (modals().length) {
        if (!state.owners.size) {
          state.html = doc.documentElement.style.overflow;
          state.body = doc.body.style.overflow;
        }
        state.owners.add(controller);
        doc.documentElement.style.overflow = 'hidden';
        doc.body.style.overflow = 'hidden';
      } else {
        const released = state.owners.delete(controller);
        if (released && !state.owners.size) {
          doc.documentElement.style.overflow = state.html;
          doc.body.style.overflow = state.body;
        }
      }
      if (options.scrollLockClass) doc.documentElement.classList.toggle(options.scrollLockClass, Boolean(state.owners.size));
    }
    function open(dialog, trigger) {
      if (!(dialog instanceof win.HTMLDialogElement) || !dialog.matches(modalSelector) || dialog.matches(':modal')) return;
      if (dialog.open) dialog.removeAttribute('open');
      if (trigger) openers.set(dialog, trigger);
      dialog.showModal();
      managed.add(dialog);
      syncScroll();
    }
    function closed(event) {
      const dialog = event.target;
      if (!(dialog instanceof win.HTMLDialogElement) || !dialog.matches(modalSelector)) return;
      managed.delete(dialog);
      syncScroll();
      const opener = openers.get(dialog);
      openers.delete(dialog);
      if (opener?.isConnected && !opener.disabled) opener.focus();
    }
    const enabledTabs = list => [...list.querySelectorAll('[role="tab"]')].filter(tab => tab.closest(tabsSelector) === list && !tab.disabled && tab.getAttribute('aria-disabled') !== 'true');
    function activate(tab, list) {
      if (options.navigation || list.dataset.uiMode === 'navigation') {
        if (!tab.href) return;
        try { win.sessionStorage.setItem(options.focusStorageKey || 'damacus-ui-tab-focus', tab.id); } catch {}
        win.location.assign(tab.href);
        return;
      }
      for (const item of list.querySelectorAll('[role="tab"]')) {
        if (item.closest(tabsSelector) !== list) continue;
        const selected = item === tab;
        item.setAttribute('aria-selected', String(selected));
        item.tabIndex = selected ? 0 : -1;
        const panel = find(item.getAttribute('aria-controls'));
        if (panel) panel.hidden = !selected;
      }
      tab.focus();
      list.dispatchEvent(new win.CustomEvent('ui:tab-change', { bubbles: true, detail: { id: tab.id } }));
    }
    function clicked(event) {
      if (!(event.target instanceof win.Element)) return;
      const trigger = event.target.closest(`[${triggerAttribute}]`);
      if (trigger && root.contains(trigger) && !trigger.disabled) {
        const dialog = find(trigger.getAttribute(triggerAttribute));
        if (dialog) open(dialog, trigger);
        return;
      }
      const close = event.target.closest(`[${closeAttribute}]`);
      if (close && root.contains(close) && !close.disabled) {
        const dialog = find(close.getAttribute(closeAttribute));
        if (dialog?.matches(modalSelector)) dialog.close();
        return;
      }
      const dialog = event.target;
      if (dialog instanceof win.HTMLDialogElement && dialog.matches(modalSelector) && dialog.matches(':modal')) {
        const box = dialog.getBoundingClientRect();
        if (event.clientX < box.left || event.clientX > box.right || event.clientY < box.top || event.clientY > box.bottom) dialog.close();
        return;
      }
      const tab = event.target.closest('[role="tab"]');
      const list = tab?.closest(tabsSelector);
      if (list && root.contains(list) && enabledTabs(list).includes(tab) && !(options.navigation || list.dataset.uiMode === 'navigation')) {
        event.preventDefault();
        activate(tab, list);
      }
    }
    function keyed(event) {
      if (!(event.target instanceof win.Element)) return;
      const tab = event.target.closest('[role="tab"]');
      const list = tab?.closest(tabsSelector);
      if (!list || !root.contains(list)) return;
      const tabs = enabledTabs(list);
      const index = tabs.indexOf(tab);
      if (index < 0 || !tabs.length) return;
      const vertical = list.getAttribute('aria-orientation') === 'vertical';
      let next;
      if (event.key === (vertical ? 'ArrowDown' : 'ArrowRight')) next = tabs[(index + 1) % tabs.length];
      if (event.key === (vertical ? 'ArrowUp' : 'ArrowLeft')) next = tabs[(index + tabs.length - 1) % tabs.length];
      if (event.key === 'Home') next = tabs[0];
      if (event.key === 'End') next = tabs[tabs.length - 1];
      if (!next) return;
      event.preventDefault();
      activate(next, list);
    }
    const controller = {
      dispose() {
        if (instances.get(root) !== controller) return;
        root.removeEventListener('click', clicked);
        root.removeEventListener('keydown', keyed);
        root.removeEventListener('close', closed, true);
        for (const dialog of managed) dialog.close();
        managed.clear();
        openers.clear();
        syncScroll();
        instances.delete(root);
      }
    };
    instances.set(root, controller);
    root.addEventListener('click', clicked);
    root.addEventListener('keydown', keyed);
    root.addEventListener('close', closed, true);
    for (const dialog of root.querySelectorAll(`${modalSelector}[open]`)) open(dialog);
    try {
      const key = options.focusStorageKey || 'damacus-ui-tab-focus';
      const id = win.sessionStorage.getItem(key);
      const tab = id && find(id);
      if (id) win.sessionStorage.removeItem(key);
      if (tab?.matches('[role="tab"][aria-selected="true"]')) {
        tab.focus();
      }
    } catch {}
    return controller;
  }
  window.DamacusUI = Object.freeze({ init });
})();
