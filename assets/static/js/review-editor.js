(() => {
  const editor = document.querySelector('[data-review-editor]');
  const form = editor?.querySelector('[data-review-form]');
  if (!editor || !form) return;
  const status = editor.querySelector('[data-lease-status]');
  const warning = editor.querySelector('[data-lease-warning]');
  const lease = form.elements.namedItem('lease_token');
  const authenticity = form.elements.namedItem('authenticity_token');
  let expires = Date.parse(editor.dataset.expiresAt) || Date.now() + 300000;
  let lastRenewal = Date.now();
  let pending = false;
  const warn = () => {
    const remaining = expires - Date.now();
    warning.hidden = remaining > 60000 || remaining <= 0;
    if (remaining <= 0) status.textContent = editor.dataset.expiredMessage || 'Editing time expired. Your draft is still here. Reacquire editing before saving.';
  };
  const renew = async () => {
    if (pending || Date.now() >= expires) return;
    pending = true;
    const body = new URLSearchParams({ authenticity_token: authenticity.value, lease_token: lease.value });
    try {
      const response = await fetch(editor.dataset.renewUrl, { method: 'POST', body, credentials: 'same-origin', headers: { 'Accept': 'application/json' } });
      if (!response.ok) throw Error('lease');
      const result = await response.json();
      expires = Date.parse(result.data.expires_at);
      lastRenewal = Date.now();
      status.textContent = '';
      warning.hidden = true;
    } catch {
      status.textContent = editor.dataset.expiredMessage || 'Editing time expired. Your draft is still here. Reacquire editing before saving.';
    } finally { pending = false; }
  };
  form.addEventListener('input', () => { if (Date.now() - lastRenewal > 60000) void renew(); });
  form.addEventListener('keydown', () => { if (Date.now() - lastRenewal > 60000) void renew(); });
  editor.querySelector('[data-keep-editing]')?.addEventListener('click', renew);
  editor.querySelector('[data-cancel]')?.addEventListener('click', () => editor.querySelector('[data-cancel-form]').submit());
  editor.querySelector('[data-accept-version]')?.addEventListener('click', event => {
    form.elements.namedItem('etag').value = event.currentTarget.dataset.currentEtag;
    form.querySelector('[type=submit]').disabled = false;
    event.currentTarget.remove();
  });
  const reacquire = editor.querySelector('[data-reacquire]');
  reacquire?.addEventListener('submit', () => {
    for (const field of ['status', 'practitioner_role', 'practitioner_name', 'reviewed_on', 'review_note']) {
      reacquire.elements.namedItem(field).value = form.elements.namedItem(field).value;
    }
  });
  window.setInterval(warn, 1000);
})();
