(() => {
  const section = document.querySelector('[data-regenerated-recovery]');
  if (!section) return;
  const list = section.querySelector('[data-result-codes]');
  const error = section.querySelector('[data-result-error]');
  const status = section.querySelector('[data-result-status]');
  const codes = () => Array.from(list.children, item => item.textContent).join('\n');
  const fail = message => { error.textContent = message; error.hidden = false; error.focus(); };
  document.addEventListener('recovery-generated', event => {
    list.replaceChildren(...event.detail.recoveryCodes.map(code => { const item = document.createElement('li'); item.textContent = code; return item; }));
    section.dataset.generation = event.detail.generation;
    document.querySelector('[data-password-operation]').hidden = true;
    section.hidden = false;
    section.querySelector('[data-result-heading]').focus();
  });
  section.querySelector('[data-result-copy]').addEventListener('click', async () => {
    try { await navigator.clipboard.writeText(codes()); status.textContent = 'Recovery codes copied.'; }
    catch { fail('Copy failed. Download your codes instead.'); }
  });
  section.querySelector('[data-result-download]').addEventListener('click', () => {
    const url = URL.createObjectURL(new Blob([`MedTracker recovery codes\n\n${codes()}\n`], { type: 'text/plain' }));
    const link = document.createElement('a'); link.href = url; link.download = 'medtracker-recovery-codes.txt'; link.click(); URL.revokeObjectURL(url);
    status.textContent = 'Recovery codes downloaded.';
  });
  section.querySelector('[data-result-continue]').addEventListener('click', async () => {
    if (!section.querySelector('[data-result-saved]').checked) { fail('Save your recovery codes before continuing.'); return; }
    try {
      const response = await fetch('/api/auth/security/recovery/acknowledge', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ generation: section.dataset.generation, saved: true }) });
      if (!response.ok) throw Error('This code set has expired or been replaced. Start regeneration again.');
      list.replaceChildren(); location.assign('/account/security');
    } catch (failure) { fail(failure.message); }
  });
  window.addEventListener('pagehide', () => list.replaceChildren());
})();
