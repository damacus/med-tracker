import { mountAuditDetail } from './audit-detail.js';
import { mountBulkMemberSelection } from './bulk-member-selection.js';
import { mountLookupPriority } from './lookup-priority.js';

export function administrationUrl(value, base, household = base.pathname.match(/^\/households\/([^/]+)\/admin(?:\/|$)/)?.[1]) {
  let url;
  try { url = new URL(value, base); } catch { return null; }
  if (url.origin !== base.origin || url.username || url.password) return null;
  const prefix = household ? `/households/${household}/admin` : null;
  return url.pathname === '/administration' || url.pathname.startsWith('/administration/') ||
    prefix && (url.pathname === prefix || url.pathname.startsWith(prefix + '/')) ? url : null;
}

export function mountAdministrationMenu(root) {
  const navigation = root.querySelector('.admin-navigation');
  if (!navigation) return;
  const controller = new AbortController();
  const details = document.createElement('details');
  details.className = 'dropdown admin-mobile-navigation';
  const summary = document.createElement('summary');
  summary.className = 'btn btn-outline admin-menu-trigger';
  const label = document.createElement('span');
  const context = document.createElement('span');
  context.className = 'admin-menu-context';
  context.textContent = navigation.dataset.title || 'Administration';
  const current = document.createElement('span');
  current.className = 'admin-menu-current';
  current.textContent = navigation.querySelector('[aria-current=page]')?.textContent.trim() || context.textContent;
  label.append(context, current);
  const chevron = document.createElement('span');
  chevron.className = 'admin-menu-chevron';
  chevron.setAttribute('aria-hidden', 'true');
  summary.append(label, chevron);
  const menu = document.createElement('ul');
  menu.className = 'menu dropdown-content admin-section-menu';
  for (const link of navigation.querySelectorAll('a[href]')) {
    const item = document.createElement('li');
    const choice = link.cloneNode(true);
    choice.className = link.hasAttribute('aria-current') ? 'menu-active' : '';
    item.append(choice);
    menu.append(item);
  }
  details.append(summary, menu);
  navigation.before(details);
  navigation.dataset.mobileMenu = 'true';
  document.addEventListener('pointerdown', event => {
    if (!details.contains(event.target)) details.open = false;
  }, { signal: controller.signal });
  details.addEventListener('keydown', event => {
    if (event.key === 'Escape' && details.open) {
      event.preventDefault();
      details.open = false;
      summary.focus({ preventScroll: true });
    }
  }, { signal: controller.signal });
  return () => { controller.abort(); details.remove(); delete navigation.dataset.mobileMenu; };
}

export function mountAdministration() {
  const main = document.querySelector('main');
  if (!main?.querySelector('.admin-navigation')) return;
  const base = new URL(location.href);
  const household = document.documentElement.dataset.navigationHousehold;
  const allowed = value => administrationUrl(value, base, household);
  const status = document.createElement('p');
  status.className = 'sr-only';
  status.setAttribute('role', 'status');
  status.setAttribute('aria-live', 'polite');
  main.before(status);
  let request;
  let sequence = 0;
  let cleanup;
  let sortable;
  const mount = () => {
    cleanup?.();
    const audit = mountAuditDetail(main, url => location.assign(url));
    const bulk = mountBulkMemberSelection(main);
    const lookup = mountLookupPriority(main);
    const menu = mountAdministrationMenu(main);
    cleanup = () => { audit?.(); bulk?.(); lookup?.(); menu?.(); };
    main.dataset.adminReady = 'true';
  };
  const prepare = async page => {
    if (page.querySelector('[data-lookup-priority]') && !window.Sortable) {
      sortable ||= new Promise((resolve, reject) => {
        const script = document.createElement('script');
        script.src = '/static/js/sortable.min.js';
        script.onload = resolve;
        script.onerror = () => { script.remove(); sortable = null; reject(Error('Reorder controls unavailable')); };
        document.head.append(script);
      });
      await sortable;
    }
  };
  async function navigate(url, pop = false) {
    const version = ++sequence;
    request?.abort();
    const controller = new AbortController();
    request = controller;
    main.setAttribute('aria-busy', 'true');
    status.className = 'sr-only';
    status.textContent = 'Loading…';
    try {
      const response = await fetch(url, { credentials: 'same-origin', cache: 'no-store', headers: { Accept: 'text/html' }, signal: AbortSignal.any([controller.signal, AbortSignal.timeout(15000)]) });
      if (version !== sequence) return;
      if (!allowed(response.url)) { location.assign(response.url); return; }
      if ([401, 403, 404].includes(response.status)) {
        cleanup?.();
        cleanup = null;
        main.replaceChildren();
        location.assign(response.url);
        return;
      }
      if (!response.ok || !response.headers.get('content-type')?.includes('text/html')) throw Error('Administration unavailable');
      const page = new DOMParser().parseFromString(await response.text(), 'text/html');
      const next = page.querySelector('main');
      if (!next?.querySelector('.admin-navigation')) throw Error('Missing administration content');
      if (page.documentElement.dataset.navigationHousehold !== document.documentElement.dataset.navigationHousehold) { location.assign(response.url); return; }
      await prepare(page);
      if (version !== sequence || controller.signal.aborted) return;
      const oldTabs = main.querySelector('.admin-navigation');
      const newTabs = next.querySelector('.admin-navigation');
      const focused = oldTabs?.contains(document.activeElement) || main.querySelector('.admin-mobile-navigation')?.contains(document.activeElement);
      cleanup?.();
      cleanup = null;
      main.querySelectorAll('dialog[open]').forEach(dialog => dialog.close());
      if (oldTabs) {
        oldTabs.replaceChildren(...newTabs.childNodes);
        newTabs.replaceWith(oldTabs);
      }
      const notice = main.querySelector('[data-canary-notice]');
      if (notice) next.querySelector('[data-canary-notice]')?.replaceWith(notice);
      next.querySelectorAll('script').forEach(script => script.remove());
      main.replaceChildren(...next.childNodes);
      main.className = next.className;
      document.title = page.title;
      document.documentElement.lang = page.documentElement.lang;
      if (!pop) history.pushState({ ...history.state, administration: true }, '', response.url);
      else history.replaceState(history.state, '', response.url);
      mount();
      if (focused) (matchMedia('(max-width: 767px)').matches ? main.querySelector('.admin-mobile-navigation summary') : main.querySelector('.admin-navigation a[aria-current=page]'))?.focus({ preventScroll: true });
      else main.focus({ preventScroll: true });
      window.scrollTo(0, 0);
      status.textContent = 'Loaded';
    } catch (error) {
      if (version !== sequence || controller.signal.aborted) return;
      status.className = 'alert alert-error';
      status.textContent = 'Administration could not be loaded. ';
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.className = 'btn btn-sm';
      retry.textContent = 'Retry';
      retry.addEventListener('click', () => { void navigate(url, pop); });
      status.append(retry);
    } finally {
      if (version === sequence) main.removeAttribute('aria-busy');
    }
  }
  main.addEventListener('click', event => {
    const link = event.target.closest('.admin-navigation a[href], .admin-mobile-navigation a[href], nav[aria-label="User scope"] a[href]');
    if (!link || event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey || link.hasAttribute('download') || link.target && link.target !== '_self') return;
    const url = allowed(link.href);
    if (!url) return;
    event.preventDefault();
    void navigate(url.href);
  });
  window.addEventListener('popstate', () => {
    if (!allowed(location.href)) { location.reload(); return; }
    void navigate(location.href, true);
  });
  window.addEventListener('pagehide', () => { ++sequence; request?.abort(); cleanup?.(); cleanup = null; });
  window.addEventListener('pageshow', event => { if (event.persisted) mount(); });
  void prepare(document).then(mount, mount);
}

if (typeof document !== 'undefined') mountAdministration();
