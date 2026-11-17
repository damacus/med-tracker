export function medicationUrl(value, medicine) {
  let url;
  try { url = new URL(value, medicine); } catch { return null; }
  if (url.origin !== medicine.origin || url.username || url.password) return null;
  const suffix = url.pathname.slice(medicine.pathname.length);
  if (url.pathname.startsWith(medicine.pathname) && /^(?:|\/edit|\/move|\/dosage_options(?:\/new|\/\d+(?:\/edit|\/destroy)?)?)$/.test(suffix)) return url;
  const household = medicine.pathname.slice(0, medicine.pathname.lastIndexOf('/medications/'));
  return [household + '/locations/new', household + '/locations'].includes(url.pathname) ? url : null;
}

export function mountMedicationManagement(document) {
  const host = document.querySelector('[data-medication-management]');
  if (!host) return;
  const medicine = new URL(host.dataset.medicationManagement, location.origin);
  const inline = host.querySelector('[data-medication-inline]');
  const options = host.querySelector('[data-medication-options]');
  const optionsSummary = host.querySelector('[data-dose-options-summary]');
  const optionsEntry = host.querySelector('[data-medication-panel="options"]');
  const move = host.querySelector('[data-medication-move-dialog]');
  const status = host.querySelector('[data-medication-status]');
  let target;
  let opener;
  let request;
  let busy = false;

  const focus = root => {
    const element = root.querySelector('[role="alert"]') || root.querySelector('h1, h2, input:not([type="hidden"]), select, a[href], button');
    if (element) {
      if (!element.matches('a[href], button, input, select, textarea, [tabindex]')) element.tabIndex = -1;
      element.focus();
    }
  };
  const close = () => {
    if (busy) return;
    request?.abort();
    if (target && move.contains(target)) move.close();
    else if (target) { target.replaceChildren(); target.hidden = true; }
    if (target === options) { optionsSummary.hidden = false; optionsEntry.hidden = false; }
    opener?.setAttribute('aria-expanded', 'false');
    opener?.focus();
    target = null;
  };
  const fail = mutation => {
    const message = document.createElement('p');
    message.className = 'alert mt-4';
    message.setAttribute('role', 'alert');
    message.tabIndex = -1;
    message.textContent = mutation ? host.dataset.uncertain : host.dataset.failed;
    target.append(message);
    if (!move.contains(target) && !target.querySelector('[data-medication-cancel], [data-medication-close]')) {
      const cancel = document.createElement('button');
      cancel.type = 'button';
      cancel.className = 'btn btn-ghost min-h-11';
      cancel.dataset.medicationClose = '';
      cancel.textContent = host.dataset.cancel;
      target.append(cancel);
    }
    message.focus();
  };
  const send = async (url, init = {}) => {
    request?.abort();
    const current = new AbortController();
    request = current;
    const mutation = init.method === 'POST';
    busy = mutation;
    status.hidden = true;
    target.setAttribute('aria-busy', 'true');
    if (mutation) target.querySelectorAll('button[type="submit"], [data-medication-cancel]').forEach(button => { button.disabled = true; });
    try {
      const response = await fetch(url, { ...init, signal: current.signal, credentials: 'same-origin', cache: 'no-store', headers: { Accept: 'text/html', 'Accept-Language': document.documentElement.lang } });
      if (request !== current || current.signal.aborted) return;
      const final = medicationUrl(response.url, medicine);
      if (!final || !response.headers.get('content-type')?.includes('text/html')) throw Error('Unexpected medicine response');
      if (mutation && response.ok && final.pathname === medicine.pathname) { location.replace(final.href); return; }
      if (mutation && response.ok && response.redirected && target === options) {
        const refreshed = new URL(location.href);
        refreshed.searchParams.delete('refill');
        refreshed.searchParams.set('manage_options', 'true');
        location.replace(refreshed.href);
        return;
      }
      const parsed = new DOMParser().parseFromString(await response.text(), 'text/html');
      const root = parsed.querySelector('[data-medication-fragment]');
      if (!root || root.querySelector('script') || ![200, 409, 422].includes(response.status)) throw Error('Medicine form unavailable');
      if (request !== current || current.signal.aborted) return;
      if (target === options && root.hasAttribute('data-dose-options-list')) {
        root.querySelector('[data-dose-options-list-heading]')?.remove();
        root.querySelector('[data-dose-options-list-context]')?.remove();
      }
      root.querySelectorAll('h1').forEach(heading => { const h2 = document.createElement('h2'); h2.className = heading.className; h2.textContent = heading.textContent; heading.replaceWith(h2); });
      root.querySelectorAll('[data-medication-cancel]').forEach(link => { link.textContent = host.dataset.cancel; });
      if (move.contains(target)) root.querySelectorAll('[data-medication-cancel]').forEach(link => { link.remove(); });
      target.replaceChildren(root);
      if (target === inline) {
        const remove = host.querySelector('[data-medication-delete]');
        if (remove) root.append(remove.content.cloneNode(true));
      }
      focus(root);
    } catch (error) {
      if (request === current && error.name !== 'AbortError') fail(mutation);
    } finally {
      if (request === current) { busy = false; target?.removeAttribute('aria-busy'); }
    }
  };
  move?.addEventListener('cancel', event => { event.preventDefault(); close(); });
  host.addEventListener('click', event => {
    if (event.defaultPrevented || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    if (event.target.closest('[data-medication-close]')) { event.preventDefault(); close(); return; }
    const link = event.target.closest('a[href]');
    if (!link) return;
    const entry = link.dataset.medicationPanel;
    if (!entry && !target?.contains(link)) return;
    const url = medicationUrl(link.href, medicine);
    if (!url) return;
    event.preventDefault();
    if (busy) return;
    if (link.hasAttribute('data-medication-cancel') || url.pathname === medicine.pathname) { close(); return; }
    if (entry) {
      const stockOrigin = link.closest('[data-stock-dialog]');
      close();
      host.querySelector('[data-stock-dialog]')?.close();
      target = entry === 'edit' ? inline : entry === 'options' ? options : move.querySelector('[data-medication-move-content]');
      opener = stockOrigin ? host.querySelector('[data-dialog-open="refill-inventory-dialog"]') : link;
      link.setAttribute('aria-expanded', 'true');
      target.hidden = false;
      if (target === options) { optionsSummary.hidden = true; optionsEntry.hidden = true; }
      if (move.contains(target)) move.showModal();
    }
    void send(url);
  });
  host.addEventListener('submit', event => {
    const form = event.target;
    if (!target?.contains(form) || form.method !== 'post') return;
    const url = medicationUrl(form.action, medicine);
    if (!url) return;
    event.preventDefault();
    if (busy) return;
    const body = new FormData(form);
    if (event.submitter?.name) body.append(event.submitter.name, event.submitter.value);
    void send(url, { method: 'POST', body: new URLSearchParams(body) });
  });
  const stock = host.querySelector('[data-stock-dialog]');
  if (stock) {
    const mode = stock.querySelector('[data-stock-mode]');
    const choose = () => {
      stock.querySelectorAll('[data-stock-operation]').forEach(form => {
        const active = form.dataset.stockOperation === mode.value;
        form.hidden = !active;
        form.querySelectorAll('input, select, textarea, button').forEach(field => { field.disabled = !active; });
      });
      stock.querySelectorAll('[data-stock-submit]').forEach(button => { button.hidden = button.dataset.stockSubmit !== mode.value; });
    };
    mode.addEventListener('change', choose);
    choose();
  }
  window.addEventListener('pagehide', () => { request?.abort(); });
  const current = new URL(location.href);
  if (current.searchParams.get('manage_options') === 'true') {
    current.searchParams.delete('manage_options');
    history.replaceState(history.state, '', current);
    host.querySelector('[data-medication-panel="options"]')?.click();
  }
}

if (typeof document !== 'undefined') mountMedicationManagement(document);
