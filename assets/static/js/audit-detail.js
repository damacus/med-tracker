function formatTimes(root) {
  const language = document.documentElement.lang || 'en';
  const formatter = new Intl.DateTimeFormat(language === 'en' ? 'en-GB' : language, {
    dateStyle: 'medium', timeStyle: 'medium', timeZone: 'UTC'
  });
  for (const element of root.querySelectorAll('[data-audit-time]')) {
    const raw = element.textContent.trim();
    const date = new Date(raw.replace(' ', 'T').replace(' UTC', 'Z'));
    if (Number.isNaN(date.valueOf())) continue;
    element.dateTime = date.toISOString();
    element.textContent = `${formatter.format(date)} UTC`;
  }
}

export function mountAuditDetail(root, navigate) {
  formatTimes(root);
  const dialog = root.querySelector('#audit-detail-dialog');
  if (!dialog) return;
  const controller = new AbortController();
  let request;
  root.addEventListener('click', async event => {
    const link = event.target.closest('[data-audit-detail-link]');
    if (!link || !dialog || event.button !== 0 || event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) return;
    event.preventDefault();
    request?.abort();
    const current = new AbortController();
    request = current;
    try {
      const response = await fetch(link.href, { signal: AbortSignal.any([current.signal, AbortSignal.timeout(15000)]), credentials: 'same-origin', cache: 'no-store', headers: { Accept: 'text/html' } });
      if (!response.ok || response.redirected) throw new Error('Audit details unavailable');
      const page = new DOMParser().parseFromString(await response.text(), 'text/html');
      const title = page.querySelector('[data-audit-detail-title]');
      const fields = page.querySelector('[data-audit-detail-fields]');
      if (!title || !fields) throw new Error('Audit details unavailable');
      for (const duplicate of fields.querySelectorAll('[data-audit-list-duplicate]')) duplicate.remove();
      if (current.signal.aborted || request !== current || !dialog.isConnected) return;
      dialog.querySelector('#audit-detail-title').textContent = title.textContent;
      const content = dialog.querySelector('[data-audit-detail-content]');
      content.replaceChildren(fields);
      formatTimes(content);
      dialog.showModal();
      dialog.addEventListener('close', () => link.focus(), { once: true });
    } catch (error) {
      if (error.name !== 'AbortError') navigate(link.href);
    }
  }, { signal: controller.signal });
  return () => { controller.abort(); request?.abort(); if (dialog.open) dialog.close(); };
}
