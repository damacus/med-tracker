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
  let currentUrl = location.href;
  let cleanup;
  let sortable;
  const mount = () => {
    const audit = mountAuditDetail(main, url => location.assign(url));
    const bulk = mountBulkMemberSelection(main);
    const lookup = mountLookupPriority(main);
    cleanup = () => { audit?.(); bulk?.(); lookup?.(); };
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
      const focused = oldTabs?.contains(document.activeElement);
      cleanup?.();
      main.querySelectorAll('dialog[open]').forEach(dialog => dialog.close());
      if (oldTabs) {
        oldTabs.replaceChildren(...newTabs.childNodes);
        newTabs.replaceWith(oldTabs);
      }
      next.querySelectorAll('script').forEach(script => script.remove());
      main.replaceChildren(...next.childNodes);
      main.className = next.className;
      document.title = page.title;
      document.documentElement.lang = page.documentElement.lang;
      if (!pop) history.pushState({ ...history.state, administration: true }, '', response.url);
      else history.replaceState(history.state, '', response.url);
      currentUrl = response.url;
      mount();
      if (focused) main.querySelector('.admin-navigation a[aria-current=page]')?.focus({ preventScroll: true });
      else main.focus({ preventScroll: true });
      window.scrollTo(0, 0);
      status.textContent = 'Loaded';
    } catch (error) {
      if (version !== sequence || controller.signal.aborted) return;
      if (pop) history.replaceState(history.state, '', currentUrl);
      status.className = 'alert alert-error';
      status.textContent = 'Administration could not be loaded. ';
      const retry = document.createElement('button');
      retry.type = 'button';
      retry.className = 'btn btn-sm';
      retry.textContent = 'Retry';
      retry.addEventListener('click', () => { void navigate(url); });
      status.append(retry);
    } finally {
      if (version === sequence) main.removeAttribute('aria-busy');
    }
  }
  main.addEventListener('click', event => {
    const link = event.target.closest('.admin-navigation a[href], nav[aria-label="User scope"] a[href]');
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
  window.addEventListener('pagehide', () => { ++sequence; request?.abort(); cleanup?.(); });
  window.addEventListener('pageshow', event => { if (event.persisted) mount(); });
  void prepare(document).then(mount, mount);
}

if (typeof document !== 'undefined') mountAdministration();
