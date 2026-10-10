(() => {
  const labels = JSON.parse(document.querySelector("[data-security-labels]")?.dataset.labels || "{}");
  const section = document.querySelector('[data-regenerated-recovery]');
  if (!section) return;
  const list = section.querySelector('[data-result-codes]');
  const error = section.querySelector('[data-result-error]');
  const status = section.querySelector('[data-result-status]');
  const lifecycle = new AbortController();
  document.addEventListener('security-flow-unmount', () => { list.replaceChildren(); lifecycle.abort(); }, { once: true, signal: lifecycle.signal });
  const codes = () => Array.from(list.children, item => item.textContent).join('\n');
  const fail = message => { error.textContent = message; error.hidden = false; error.focus(); };
  document.addEventListener('recovery-generated', event => {
    list.replaceChildren(...event.detail.recoveryCodes.map(code => { const item = document.createElement('li'); item.textContent = code; return item; }));
    section.dataset.generation = event.detail.generation;
    document.querySelector('[data-password-operation]').hidden = true;
    section.hidden = false;
    section.querySelector('[data-result-heading]').focus();
  }, { signal: lifecycle.signal });
  section.querySelector('[data-result-copy]').addEventListener('click', async () => {
    try { await navigator.clipboard.writeText(codes()); status.textContent = (labels.codes_copied || "Recovery codes copied."); }
    catch { fail((labels.codes_copy_failed || "Copy failed. Download your codes instead.")); }
  });
  section.querySelector('[data-result-download]').addEventListener('click', () => {
    const url = URL.createObjectURL(new Blob([`MedTracker recovery codes\n\n${codes()}\n`], { type: 'text/plain' }));
    const link = document.createElement('a'); link.href = url; link.download = 'medtracker-recovery-codes.txt'; link.click(); URL.revokeObjectURL(url);
    status.textContent = (labels.codes_downloaded || "Recovery codes downloaded.");
  });
  section.querySelector('[data-result-continue]').addEventListener('click', async () => {
    if (!section.querySelector('[data-result-saved]').checked) { fail((labels.codes_save_first || "Save your recovery codes before continuing.")); return; }
    const button = section.querySelector('[data-result-continue]');
    if (button.disabled) return;
    button.disabled = true;
    document.dispatchEvent(new CustomEvent('security-flow-busy', { detail: true }));
    try {
      const response = await fetch('/api/auth/security/recovery/acknowledge', { method: 'POST', credentials: 'same-origin', headers: { 'Content-Type': 'application/json' }, body: JSON.stringify({ generation: section.dataset.generation, saved: true }) });
      if (!response.ok) throw Error((labels.codes_expired || "This code set has expired or been replaced. Start regeneration again."));
      list.replaceChildren();
      const navigation = new CustomEvent('security-flow-navigate', { cancelable: true, detail: '/account/security' });
      if (document.dispatchEvent(navigation)) location.assign('/account/security');
    } catch (failure) { fail(failure.message); }
    finally {
      button.disabled = false;
      document.dispatchEvent(new CustomEvent('security-flow-busy', { detail: false }));
    }
  });
  window.addEventListener('pagehide', () => { list.replaceChildren(); lifecycle.abort(); }, { once: true, signal: lifecycle.signal });
})();
